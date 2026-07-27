//! Authentication Service

mod core;
mod dto;
mod integration;
mod mapper;
mod repository;
mod validation;

pub use dto::{AuthResponse, AuthSettings, Claims, PasswordValidationResult};

use crate::db::connection::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{LoginDto, RegisterDto, TrustedDevice, User};
use crate::services::{AuditService, EmailService, SettingsService};
use crate::services::whatsapp_gateway_service::normalize_phone;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use std::sync::Arc;
use tokio::sync::RwLock;
use totp_rs::{Algorithm, Secret, TOTP};
use tracing::{info, warn};
use uuid::Uuid;

/// Auth service for handling authentication
#[derive(Clone)]
pub struct AuthService {
    pub pool: DbPool,
    jwt_secret: Arc<RwLock<String>>,
    email_service: EmailService,
    audit_service: AuditService,
    settings_service: SettingsService,
    /// Cached auth settings with TTL (60 seconds)
    auth_settings_cache: Arc<crate::services::cache::SingleValueCache<AuthSettings>>,
}

impl AuthService {
    pub fn new(
        pool: DbPool,
        jwt_secret: String,
        email_service: EmailService,
        audit_service: AuditService,
        settings_service: SettingsService,
    ) -> Self {
        Self {
            pool,
            jwt_secret: Arc::new(RwLock::new(jwt_secret)),
            email_service,
            audit_service,
            settings_service,
            // Initialize cache with 60 second TTL
            auth_settings_cache: Arc::new(crate::services::cache::SingleValueCache::new(60)),
        }
    }

    /// Get auth settings from database (with caching)
    pub async fn get_auth_settings(&self) -> AuthSettings {
        // Check cache first
        if let Some(cached) = self.auth_settings_cache.get() {
            return cached;
        }

        // Fetch from database with a single batch query
        let settings = self.fetch_auth_settings_from_db().await;

        // Cache the result
        self.auth_settings_cache.set(settings.clone());

        settings
    }

    /// Fetch auth settings from database (internal helper)
    async fn fetch_auth_settings_from_db(&self) -> AuthSettings {
        repository::fetch_auth_settings_from_db(&self.pool).await
    }

    /// Invalidate auth settings cache (call when settings are updated)
    pub fn invalidate_auth_settings_cache(&self) {
        self.auth_settings_cache.invalidate();
    }

    /// Resolve effective email verification requirement.
    ///
    /// Priority:
    /// 1. Tenant-level `auth_require_email_verification` (if set to true/false)
    /// 2. Global auth setting fallback
    pub async fn get_effective_require_email_verification(&self, tenant_id: Option<&str>) -> bool {
        if let Some(tid) = tenant_id {
            if let Ok(Some(value)) = self
                .settings_service
                .get_value(Some(tid), "auth_require_email_verification")
                .await
            {
                let normalized = value.trim().to_ascii_lowercase();
                if normalized == "true" {
                    return true;
                }
                if normalized == "false" {
                    return false;
                }
            }
        }

        self.get_auth_settings().await.require_email_verification
    }

    /// Validate password against policy
    pub fn validate_password(
        &self,
        password: &str,
        settings: &AuthSettings,
    ) -> PasswordValidationResult {
        validation::validate_password(password, settings)
    }

    /// Hash password using Argon2
    pub fn hash_password(password: &str) -> AppResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| AppError::Internal(format!("Password hashing failed: {}", e)))
    }

    /// Verify password against hash
    pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| AppError::Internal(format!("Invalid password hash: {}", e)))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Generate JWT token
    async fn generate_token(
        &self,
        user: &User,
        tenant_id: Option<String>,
    ) -> AppResult<(String, String)> {
        let secret = self.jwt_secret.read().await;
        let settings = self.get_auth_settings().await;
        let now = Utc::now();
        let jwt_expiry_hours = settings.jwt_expiry_hours.max(1);
        let session_timeout_minutes = settings.session_timeout_minutes.max(1);
        let jwt_expires_at = now + Duration::hours(jwt_expiry_hours);
        let session_expires_at = now + Duration::minutes(session_timeout_minutes);

        let claims = core::build_login_claims(user, tenant_id.clone(), now, jwt_expires_at);
        let token = integration::encode_jwt(&claims, secret.as_str())?;

        let session_id = Uuid::new_v4().to_string();
        repository::insert_session(
            &self.pool,
            &session_id,
            &user.id,
            tenant_id.as_deref(),
            &token,
            session_expires_at,
            now,
        )
        .await?;

        Ok((token, session_expires_at.to_rfc3339()))
    }

    /// Validate JWT token and return claims
    pub async fn validate_token(&self, token: &str) -> AppResult<Claims> {
        // Validate against active session first (sliding inactivity timeout).
        let now = Utc::now();
        let settings = self.get_auth_settings().await;
        let session_timeout_minutes = settings.session_timeout_minutes.max(1);

        #[cfg(feature = "postgres")]
        let session_expires_at: Option<DateTime<Utc>> =
            sqlx::query_scalar("SELECT expires_at FROM sessions WHERE token = $1")
                .bind(token)
                .fetch_optional(&self.pool)
                .await
                .unwrap_or(None);

        #[cfg(feature = "sqlite")]
        let session_expires_at: Option<DateTime<Utc>> = {
            let raw: Option<String> =
                sqlx::query_scalar("SELECT expires_at FROM sessions WHERE token = $1")
                    .bind(token)
                    .fetch_optional(&self.pool)
                    .await
                    .unwrap_or(None);

            raw.and_then(|v| {
                chrono::DateTime::parse_from_rfc3339(&v)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            })
        };

        let session_expires_at = if let Some(v) = session_expires_at {
            v
        } else {
            return Err(AppError::InvalidToken);
        };

        if session_expires_at <= now {
            let _ = sqlx::query("DELETE FROM sessions WHERE token = $1")
                .bind(token)
                .execute(&self.pool)
                .await;
            return Err(AppError::TokenExpired);
        }

        let secret = self.jwt_secret.read().await;
        // JWT `exp` is treated as informational; active session timeout is enforced by DB session.
        let mut validation = Validation::default();
        validation.validate_exp = false;

        let claims = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|e| match e.kind() {
            _ => AppError::InvalidToken,
        })?;

        // Extend session expiry when user is actively using the app.
        let refresh_threshold =
            now + Duration::minutes(std::cmp::max(1, session_timeout_minutes / 2));
        if session_expires_at <= refresh_threshold {
            let new_expires_at = now + Duration::minutes(session_timeout_minutes);
            let query = sqlx::query("UPDATE sessions SET expires_at = $1 WHERE token = $2");

            #[cfg(feature = "postgres")]
            let query = query.bind(new_expires_at).bind(token);

            #[cfg(not(feature = "postgres"))]
            let query = query.bind(new_expires_at.to_rfc3339()).bind(token);

            let _ = query.execute(&self.pool).await;
        }

        // Enforce tenant suspension for non-superadmin sessions.
        if !claims.is_super_admin {
            if let Some(ref tenant_id) = claims.tenant_id {
                #[cfg(feature = "postgres")]
                let is_active: Option<bool> =
                    sqlx::query_scalar("SELECT is_active FROM tenants WHERE id = $1")
                        .bind(tenant_id)
                        .fetch_optional(&self.pool)
                        .await
                        .unwrap_or(None);

                #[cfg(feature = "sqlite")]
                let is_active: Option<i64> =
                    sqlx::query_scalar("SELECT is_active FROM tenants WHERE id = $1")
                        .bind(tenant_id)
                        .fetch_optional(&self.pool)
                        .await
                        .unwrap_or(None);

                #[cfg(feature = "postgres")]
                let tenant_is_active = is_active.unwrap_or(false);

                #[cfg(feature = "sqlite")]
                let tenant_is_active = is_active.unwrap_or(0) != 0;

                if !tenant_is_active {
                    return Err(AppError::Forbidden("Tenant is suspended".to_string()));
                }
            }
        }

        Ok(claims)
    }

    /// Validate 2FA temp token (does not check sessions table)
    /// This is used for temporary tokens during 2FA verification flow
    pub async fn validate_2fa_token(&self, token: &str) -> AppResult<Claims> {
        let secret = self.jwt_secret.read().await;

        decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
            _ => AppError::InvalidToken,
        })
    }

    /// Apply RLS context to an active transaction (PostgreSQL).
    ///
    /// Uses `set_config(..., true)` so values are transaction-local and cannot leak
    /// across pooled connections.
    #[cfg(feature = "postgres")]
    pub async fn apply_rls_context_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        claims: &Claims,
    ) -> AppResult<()> {
        self.apply_rls_context_tx_values(
            tx,
            claims.tenant_id.as_deref(),
            Some(claims.sub.as_str()),
            claims.is_super_admin,
        )
        .await
    }

    /// SQLite no-op counterpart so call sites can stay uniform.
    #[cfg(feature = "sqlite")]
    pub async fn apply_rls_context_tx(
        &self,
        _tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        _claims: &Claims,
    ) -> AppResult<()> {
        Ok(())
    }

    /// Apply RLS context using raw values. Useful in service-layer transactions that
    /// only know `tenant_id` and privilege level.
    #[cfg(feature = "postgres")]
    pub async fn apply_rls_context_tx_values(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        tenant_id: Option<&str>,
        user_id: Option<&str>,
        is_super_admin: bool,
    ) -> AppResult<()> {
        let tenant_id = tenant_id.unwrap_or("").to_string();
        let user_id = user_id.unwrap_or("").to_string();
        let is_superadmin = if is_super_admin { "true" } else { "false" };

        sqlx::query(
            "SELECT set_config('app.current_tenant_id', $1, true), set_config('app.current_user_id', $2, true), set_config('app.current_is_superadmin', $3, true)",
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(is_superadmin)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn apply_rls_context_tx_values(
        &self,
        _tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        _tenant_id: Option<&str>,
        _user_id: Option<&str>,
        _is_super_admin: bool,
    ) -> AppResult<()> {
        Ok(())
    }

    /// Logout (revoke current session)
    pub async fn logout(&self, token: &str, ip_address: Option<String>) -> AppResult<()> {
        // Try to decode token to get user_id before it's deleted
        if let Ok(claims) = self.validate_token(token).await {
            self.audit_service
                .log(
                    Some(&claims.sub),
                    claims.tenant_id.as_deref(),
                    "USER_LOGOUT",
                    "auth",
                    None,
                    None,
                    ip_address.as_deref(),
                )
                .await;
        }

        sqlx::query("DELETE FROM sessions WHERE token = $1")
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Logout from all devices (revoke all sessions for user)
    pub async fn logout_all(&self, user_id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Increment failed login attempts
    async fn increment_failed_attempts(
        &self,
        user_id: &str,
        settings: &AuthSettings,
    ) -> AppResult<()> {
        // Get current attempts
        let current: (i32,) =
            sqlx::query_as("SELECT failed_login_attempts FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;

        let new_attempts = current.0 + 1;

        if new_attempts >= settings.max_login_attempts {
            // Lock the account
            let locked_until = Utc::now() + Duration::minutes(settings.lockout_duration_minutes);
            warn!(
                "Account {} locked until {} after {} failed attempts",
                user_id, locked_until, new_attempts
            );

            let query = sqlx::query(
                "UPDATE users SET failed_login_attempts = $1, locked_until = $2, updated_at = $3 WHERE id = $4"
            )
            .bind(new_attempts);

            #[cfg(feature = "postgres")]
            let query = query.bind(locked_until).bind(Utc::now());

            #[cfg(not(feature = "postgres"))]
            let query = query
                .bind(locked_until.to_rfc3339())
                .bind(Utc::now().to_rfc3339());

            query.bind(user_id).execute(&self.pool).await?;
        } else {
            let query = sqlx::query(
                "UPDATE users SET failed_login_attempts = $1, updated_at = $2 WHERE id = $3",
            )
            .bind(new_attempts);

            #[cfg(feature = "postgres")]
            let query = query.bind(Utc::now());

            #[cfg(not(feature = "postgres"))]
            let query = query.bind(Utc::now().to_rfc3339());

            query.bind(user_id).execute(&self.pool).await?;
        }

        Ok(())
    }

    /// Reset failed login attempts on successful login
    async fn reset_failed_attempts(&self, user_id: &str) -> AppResult<()> {
        let query = sqlx::query(
            "UPDATE users SET failed_login_attempts = 0, locked_until = NULL, updated_at = $1 WHERE id = $2"
        );

        #[cfg(feature = "postgres")]
        let query = query.bind(Utc::now());

        #[cfg(not(feature = "postgres"))]
        let query = query.bind(Utc::now().to_rfc3339());

        query.bind(user_id).execute(&self.pool).await?;

        Ok(())
    }

    async fn get_primary_active_tenant_id(&self, user_id: &str) -> Option<String> {
        #[cfg(feature = "postgres")]
        let tenant_id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT t.id
            FROM tenants t
            JOIN tenant_members tm ON t.id = tm.tenant_id
            WHERE tm.user_id = $1
              AND t.is_active = true
            ORDER BY tm.created_at ASC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        #[cfg(feature = "sqlite")]
        let tenant_id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT t.id
            FROM tenants t
            JOIN tenant_members tm ON t.id = tm.tenant_id
            WHERE tm.user_id = ?1
              AND t.is_active = 1
            ORDER BY tm.created_at ASC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        tenant_id
    }

    /// Register a new user
    pub async fn register(
        &self,
        dto: RegisterDto,
        ip_address: Option<String>,
    ) -> AppResult<AuthResponse> {
        self.register_with_email_verification_policy(dto, ip_address, None, None)
            .await
    }

    /// Register a new user with optional email verification policy override
    /// and optional invite token context.
    ///
    /// `invite_token` only informs whether the registration is invited
    /// (auto-active for users + customer) versus pending admin approval. The
    /// token itself must be consumed by the caller before this method.
    pub async fn register_with_email_verification_policy(
        &self,
        dto: RegisterDto,
        ip_address: Option<String>,
        require_email_verification_override: Option<bool>,
        invite_token: Option<&str>,
    ) -> AppResult<AuthResponse> {
        let settings = self.get_auth_settings().await;
        let require_email_verification =
            require_email_verification_override.unwrap_or(settings.require_email_verification);

        // Check if registration is allowed
        if !settings.allow_registration {
            return Err(AppError::Validation(
                "Public registration is currently disabled".to_string(),
            ));
        }

        // Validate password against policy
        let validation = self.validate_password(&dto.password, &settings);
        if !validation.valid {
            return Err(AppError::Validation(validation.errors.join(", ")));
        }

        // Check if user exists
        let existing: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = $1")
            .bind(&dto.email)
            .fetch_optional(&self.pool)
            .await?;

        if existing.is_some() {
            return Err(AppError::UserAlreadyExists);
        }

        // Hash password
        let password_hash = Self::hash_password(&dto.password)?;

        // Create user
        let mut user = User::new(dto.email, password_hash, dto.name);

        // Hybrid flow: all public registrations are pending
        // Approval gating:
        // - invite_token → trusted (admin/owner issued link) → auto-active
        // - manual        → pending admin approval before login
        let via_invite = invite_token
            .map(|t| !t.trim().is_empty())
            .unwrap_or(false);
        if via_invite {
            user.registration_status = "active".to_string();
            user.is_active = true;
        } else {
            user.registration_status = "pending".to_string();
            user.is_active = false; // Pending users are NOT active until approved
        }

        // Handle email verification
        if require_email_verification {
            let token = uuid::Uuid::new_v4().to_string();
            user.verification_token = Some(token.clone());
        } else {
            user.email_verified_at = Some(Utc::now());
        }

        let query = sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, name, role, is_active, failed_login_attempts, created_at, updated_at, verification_token, email_verified_at, registration_status)
            VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.name)
        .bind(&user.role)
        .bind(user.is_active);

        #[cfg(feature = "postgres")]
        let query = query
            .bind(user.created_at)
            .bind(user.updated_at)
            .bind(&user.verification_token)
            .bind(user.email_verified_at)
            .bind(&user.registration_status);

        #[cfg(not(feature = "postgres"))]
        let query = query
            .bind(user.created_at.to_rfc3339())
            .bind(user.updated_at.to_rfc3339())
            .bind(&user.verification_token)
            .bind(user.email_verified_at.map(|t| t.to_rfc3339()))
            .bind(&user.registration_status);

        query.execute(&self.pool).await?;

        info!("New user registered (pending approval): {}", user.email);

        // Send verification email if required
        if require_email_verification {
            if let Some(token) = &user.verification_token {
                let link = format!("/auth/verify-email?token={}", token);

                let body = format!(
                    "Welcome, {}!\n\nPlease verify your email by clicking the link below:\n{}\n\nIf you cannot click the link, use this code: {}",
                    user.name, link, token
                );

                if let Err(e) = self
                    .email_service
                    .send_email(&user.email, "Verify your email", &body)
                    .await
                {
                    warn!("Failed to send verification email: {}", e);
                }
            }
        }

        // Notify superadmin(s) about pending registration
        let superadmin_emails: Vec<String> = sqlx::query_scalar(
            "SELECT email FROM users WHERE is_super_admin = true AND is_active = true",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for admin_email in superadmin_emails {
            let subject = format!("New user registration pending approval: {}", user.email);
            let body = format!(
                "A new user has registered and is awaiting your approval.\n\n\
                 Name: {}\nEmail: {}\nRegistered at: {}\n\n\
                 Please review and approve or reject this registration in the admin panel.",
                user.name,
                user.email,
                user.created_at.to_rfc3339()
            );
            if let Err(e) = self
                .email_service
                .send_email(&admin_email, &subject, &body)
                .await
            {
                warn!(
                    "Failed to send pending registration notification to {}: {}",
                    admin_email, e
                );
            }
        }

        self.audit_service
            .log(
                Some(&user.id),
                None,
                "USER_REGISTER",
                "auth",
                Some(&user.id),
                Some(&format!(
                    "Registered via email {} (pending approval)",
                    user.email
                )),
                ip_address.as_deref(),
            )
            .await;

        // Always return pending status — no token issued
        Ok(AuthResponse {
            user: user.into(),
            tenant: None,
            token: None,
            expires_at: None,
            message: Some(
                "Registration successful. Your account is pending approval by an administrator. You will be able to login once approved."
                    .to_string(),
            ),
            requires_2fa: None,
            requires_2fa_setup: None,
            temp_token: None,
            available_2fa_methods: None,
        })
    }

    /// Verify email with token
    pub async fn verify_email(&self, token: &str) -> AppResult<AuthResponse> {
        // Find user by verification token
        let user: Option<User> =
            sqlx::query_as("SELECT * FROM users WHERE verification_token = $1")
                .bind(token)
                .fetch_optional(&self.pool)
                .await?;

        let user = match user {
            Some(u) => u,
            None => {
                return Err(AppError::Validation(
                    "Invalid verification token".to_string(),
                ))
            }
        };

        if user.email_verified_at.is_some() {
            return Err(AppError::Validation("Email already verified".to_string()));
        }

        // Update user
        let now = Utc::now();
        let query = sqlx::query("UPDATE users SET email_verified_at = $1, verification_token = NULL, updated_at = $2 WHERE id = $3");

        #[cfg(feature = "postgres")]
        let query = query.bind(now).bind(now);

        #[cfg(not(feature = "postgres"))]
        let query = query.bind(now.to_rfc3339()).bind(now.to_rfc3339());

        query.bind(&user.id).execute(&self.pool).await?;

        // Get user's primary tenant
        let tenant: Option<crate::models::tenant::Tenant> = sqlx::query_as(
            r#"
            SELECT t.* FROM tenants t
            JOIN tenant_members tm ON t.id = tm.tenant_id
            WHERE tm.user_id = $1
            LIMIT 1
            "#,
        )
        .bind(&user.id)
        .fetch_optional(&self.pool)
        .await?;

        let tenant_id = tenant.as_ref().map(|t| t.id.clone());

        // Login user
        let (jwt, expires_at) = self.generate_token(&user, tenant_id).await?;

        // Refresh user data
        let updated_user = self.get_user_by_id(&user.id).await?;

        Ok(AuthResponse {
            user: updated_user.into(),
            tenant,
            token: Some(jwt),
            expires_at: Some(expires_at),
            message: Some("Email verified successfully".to_string()),
            requires_2fa: None,
            temp_token: None,
            available_2fa_methods: None,
            requires_2fa_setup: None,
        })
    }

    /// Request password reset
    pub async fn forgot_password(&self, email: &str) -> AppResult<()> {
        let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(user) = user {
            // Generate reset token
            let token = uuid::Uuid::new_v4().to_string();
            let expires_at = Utc::now() + Duration::hours(1);

            let query = sqlx::query("UPDATE users SET reset_token = $1, reset_token_expires = $2, updated_at = $3 WHERE id = $4")
                .bind(&token);

            #[cfg(feature = "postgres")]
            let query = query.bind(expires_at).bind(Utc::now());

            #[cfg(not(feature = "postgres"))]
            let query = query
                .bind(expires_at.to_rfc3339())
                .bind(Utc::now().to_rfc3339());

            query.bind(&user.id).execute(&self.pool).await?;

            // Send email per-tenant
            let tenant_id = self.get_primary_active_tenant_id(&user.id).await;
            let tenant_id_str = tenant_id.as_deref();
            let link = format!("/forgot-password/reset?token={}", token);
            let body = format!(
                "Hello {},\n\nYou requested a password reset. Click the link below to reset your password:\n{}\n\nThis link expires in 1 hour.\n\nIf you did not request this, please ignore this email.",
                user.name, link
            );
            let subject = "Reset your password";

            let send_result = self
                .email_service
                .send_email_for_tenant(tenant_id_str, &user.email, subject, &body)
                .await;

            match &send_result {
                Err(AppError::Validation(msg)) if msg.starts_with("Unknown email provider") => {
                    return Err(AppError::Configuration(format!(
                        "SMTP_NOT_CONFIGURED: email provider for tenant {:?}",
                        tenant_id
                    )));
                }
                Err(e) => {
                    warn!("Failed to send reset email: {}", e);
                }
                Ok(()) => { /* email sent */ }
            }
        }

        // Always return OK unless SMTP is not configured (prevent email enumeration)
        Ok(())
    }

    /// Reset password with token
    pub async fn reset_password(&self, token: &str, new_password: &str) -> AppResult<()> {
        let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE reset_token = $1")
            .bind(token)
            .fetch_optional(&self.pool)
            .await?;

        let user = match user {
            Some(u) => u,
            None => {
                return Err(AppError::Validation(
                    "Invalid or expired reset token".to_string(),
                ))
            }
        };

        // Check expiration
        if let Some(expires) = user.reset_token_expires {
            if Utc::now() > expires {
                return Err(AppError::Validation("Reset token has expired".to_string()));
            }
        } else {
            return Err(AppError::Validation("Invalid token state".to_string()));
        }

        // Validate password
        let settings = self.get_auth_settings().await;
        let validation = self.validate_password(new_password, &settings);
        if !validation.valid {
            return Err(AppError::Validation(validation.errors.join(", ")));
        }

        // Update password and clear token
        let new_hash = Self::hash_password(new_password)?;

        let query = sqlx::query("UPDATE users SET password_hash = $1, reset_token = NULL, reset_token_expires = NULL, updated_at = $2 WHERE id = $3")
            .bind(new_hash);

        #[cfg(feature = "postgres")]
        let query = query.bind(Utc::now());

        #[cfg(not(feature = "postgres"))]
        let query = query.bind(Utc::now().to_rfc3339());

        query.bind(&user.id).execute(&self.pool).await?;

        // Optional: Logout all sessions
        if settings.logout_all_on_password_change {
            self.logout_all(&user.id).await?;
        }

        Ok(())
    }

    /// Login user
    pub async fn login(
        &self,
        dto: LoginDto,
        ip_address: Option<String>,
        device_fingerprint: Option<String>,
    ) -> AppResult<AuthResponse> {
        let settings = self.get_auth_settings().await;

        // Normalize identifier: if it looks like a phone number, normalize it
        // Otherwise treat as email (lowercase, trimmed)
        let id = dto.get_identifier();
        let identifier = id.trim();
        let (query_email, query_phone) = if identifier.contains('@') {
            // Looks like email
            (Some(identifier.to_lowercase()), None)
        } else {
            // Treat as phone number - normalize
            let normalized = normalize_phone(identifier);
            (None, Some(normalized))
        };

        // Find user by email OR phone
        let user: Option<User> = if let Some(email_val) = &query_email {
            sqlx::query_as("SELECT * FROM users WHERE email = $1")
                .bind(email_val)
                .fetch_optional(&self.pool)
                .await?
        } else if let Some(phone_val) = &query_phone {
            // Try both normalized (62...) and original (08...) values
            let original = identifier.to_string();
            sqlx::query_as(
                "SELECT * FROM users WHERE phone = $1 OR phone = $2",
            )
            .bind(phone_val)
            .bind(original)
            .fetch_optional(&self.pool)
            .await?
        } else {
            None
        };

        let user = match user {
            Some(u) => u,
            None => {
                // Audit (best-effort). Avoid leaking sensitive info.
                let details = serde_json::json!({
                    "identifier": identifier,
                    "reason": "user_not_found"
                })
                .to_string();
                self.audit_service
                    .log(
                        None,
                        None,
                        "login_failed",
                        "auth",
                        None,
                        Some(details.as_str()),
                        ip_address.as_deref(),
                    )
                    .await;
                return Err(AppError::InvalidCredentials);
            }
        };

        // Check if account is locked
        if user.is_locked() {
            let details = serde_json::json!({
                "email": user.email,
                "reason": "account_locked"
            })
            .to_string();
            self.audit_service
                .log(
                    Some(&user.id),
                    None,
                    "login_locked",
                    "auth",
                    None,
                    Some(details.as_str()),
                    ip_address.as_deref(),
                )
                .await;

            let remaining = user
                .locked_until
                .map(|t| (t - Utc::now()).num_minutes())
                .unwrap_or(0);
            return Err(AppError::RateLimited(format!(
                "Account is locked. Try again in {} minutes",
                remaining.max(1)
            )));
        }

        // Check registration status — pending users cannot login
        // Must be checked BEFORE is_active since pending users have is_active=false
        if user.registration_status == "pending" {
            let details = serde_json::json!({
                "email": user.email,
                "reason": "account_pending_approval"
            })
            .to_string();
            self.audit_service
                .log(
                    Some(&user.id),
                    None,
                    "login_pending_approval",
                    "auth",
                    None,
                    Some(details.as_str()),
                    ip_address.as_deref(),
                )
                .await;
            return Err(AppError::AccountPendingApproval);
        }

        if user.registration_status == "rejected" {
            let details = serde_json::json!({
                "email": user.email,
                "reason": "account_rejected"
            })
            .to_string();
            self.audit_service
                .log(
                    Some(&user.id),
                    None,
                    "login_rejected",
                    "auth",
                    None,
                    Some(details.as_str()),
                    ip_address.as_deref(),
                )
                .await;
            return Err(AppError::Validation(
                "Your registration has been rejected. Please contact support.".to_string(),
            ));
        }

        // Check if account is active (after registration_status so pending/rejected are caught first)
        if !user.is_active {
            let details = serde_json::json!({
                "email": user.email,
                "reason": "account_deactivated"
            })
            .to_string();
            self.audit_service
                .log(
                    Some(&user.id),
                    None,
                    "login_failed",
                    "auth",
                    None,
                    Some(details.as_str()),
                    ip_address.as_deref(),
                )
                .await;
            return Err(AppError::Validation("Account is deactivated".to_string()));
        }

        // Check email verification using effective scope (tenant override -> global fallback).
        let tenant_id_for_email_policy = if user.is_super_admin {
            None
        } else {
            self.get_primary_active_tenant_id(&user.id).await
        };
        let require_email_verification = self
            .get_effective_require_email_verification(tenant_id_for_email_policy.as_deref())
            .await;

        if require_email_verification && user.email_verified_at.is_none() {
            let details = serde_json::json!({
                "email": user.email,
                "reason": "email_unverified"
            })
            .to_string();
            self.audit_service
                .log(
                    Some(&user.id),
                    None,
                    "login_failed",
                    "auth",
                    None,
                    Some(details.as_str()),
                    ip_address.as_deref(),
                )
                .await;
            return Err(AppError::Validation(
                "Please verify your email address before logging in.".to_string(),
            ));
        }

        // Verify password
        if !Self::verify_password(&dto.password, &user.password_hash)? {
            // Increment failed attempts
            self.increment_failed_attempts(&user.id, &settings).await?;

            let details = serde_json::json!({
                "email": user.email,
                "reason": "invalid_password"
            })
            .to_string();
            self.audit_service
                .log(
                    Some(&user.id),
                    None,
                    "login_failed",
                    "auth",
                    None,
                    Some(details.as_str()),
                    ip_address.as_deref(),
                )
                .await;

            let remaining = settings.max_login_attempts - user.failed_login_attempts - 1;
            if remaining > 0 {
                return Err(AppError::Validation(format!(
                    "Invalid credentials. {} attempts remaining",
                    remaining
                )));
            } else {
                return Err(AppError::RateLimited(format!(
                    "Account locked for {} minutes",
                    settings.lockout_duration_minutes
                )));
            }
        }

        // Reset failed attempts on successful login
        self.reset_failed_attempts(&user.id).await?;

        info!("User logged in: {}", user.email);

        // Check 2FA
        if user.two_factor_enabled {
            // Check if device is trusted - skip 2FA if trusted
            if let Some(ref fingerprint) = device_fingerprint {
                if self
                    .is_device_trusted(&user.id, fingerprint)
                    .await
                    .unwrap_or(false)
                {
                    info!("Device is trusted, skipping 2FA for user: {}", user.email);
                    return self.complete_login(user).await;
                }
            }

            // Audit: password ok but 2FA is required (no session yet)
            let details = serde_json::json!({
                "email": user.email,
                "reason": "2fa_required"
            })
            .to_string();
            self.audit_service
                .log(
                    Some(&user.id),
                    None,
                    "login_2fa_required",
                    "auth",
                    None,
                    Some(details.as_str()),
                    ip_address.as_deref(),
                )
                .await;

            let (token, expires_at) = self.generate_2fa_token(&user).await?;

            // Get available methods based on user's enabled methods AND global settings
            let global_methods = self.get_available_2fa_methods().await;
            let mut available_methods = vec![];

            // If user has TOTP enabled and it's allowed globally
            if user.totp_enabled
                && user.two_factor_secret.is_some()
                && global_methods.contains(&"totp".to_string())
            {
                available_methods.push("totp".to_string());
            }

            // If user has Email 2FA enabled and it's allowed globally
            if user.email_2fa_enabled && global_methods.contains(&"email".to_string()) {
                available_methods.push("email".to_string());
            }

            // Legacy fallback: if two_factor_enabled but no specific methods set, use TOTP if secret exists
            if available_methods.is_empty() && user.two_factor_secret.is_some() {
                available_methods.push("totp".to_string());
            }

            return Ok(AuthResponse {
                user: user.into(), // Minimal info
                tenant: None,
                token: None,
                expires_at: Some(expires_at),
                message: Some("2FA verification required".to_string()),
                requires_2fa: Some(true),
                temp_token: Some(token),
                available_2fa_methods: Some(available_methods),
                requires_2fa_setup: None,
            });
        }

        self.complete_login(user).await
    }

    /// Get user's role name in a tenant
    pub async fn get_tenant_role_name(
        &self,
        user_id: &str,
        tenant_id: &str,
    ) -> AppResult<Option<String>> {
        // Primary: JOIN through role_id
        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT r.name 
            FROM tenant_members tm
            JOIN roles r ON tm.role_id = r.id
            WHERE tm.user_id = $1 AND tm.tenant_id = $2
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT r.name 
            FROM tenant_members tm
            JOIN roles r ON tm.role_id = r.id
            WHERE tm.user_id = ? AND tm.tenant_id = ?
        "#;

        let role_name: Option<String> = sqlx::query_scalar(query)
            .bind(user_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;

        if role_name.is_some() {
            return Ok(role_name);
        }

        // Fallback: NULL role_id but tenant_members.role = 'Owner'
        // (old tenants created before role_id migration)
        #[cfg(feature = "postgres")]
        let fallback: Option<String> = sqlx::query_scalar(
            r#"
            SELECT tm.role
            FROM tenant_members tm
            WHERE tm.user_id = $1 AND tm.tenant_id = $2
              AND tm.role_id IS NULL
              AND tm.role IS NOT NULL
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let fallback: Option<String> = sqlx::query_scalar(
            r#"
            SELECT tm.role
            FROM tenant_members tm
            WHERE tm.user_id = ? AND tm.tenant_id = ?
              AND tm.role_id IS NULL
              AND tm.role IS NOT NULL
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(fallback)
    }

    /// Get all permissions for a user in a tenant
    pub async fn get_user_permissions(
        &self,
        user_id: &str,
        tenant_id: &str,
    ) -> AppResult<Vec<String>> {
        // If owner, get all permissions? Or just rely on seeded.
        // Let's simplified: get all distinct permissions assigned to user's role.
        // Also check if user is Owner via role name, if so, maybe grant *all* or specific set.
        // For now, we rely on the seeded permissions which gave Owner all perms.

        // Check if user is Owner
        #[cfg(feature = "postgres")]
        let is_owner: bool = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) > 0 FROM tenant_members tm
            JOIN roles r ON tm.role_id = r.id
            WHERE tm.user_id = $1 AND tm.tenant_id = $2 AND r.name = 'Owner'
        "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(false);

        #[cfg(feature = "sqlite")]
        let is_owner: bool = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) > 0 FROM tenant_members tm
            JOIN roles r ON tm.role_id = r.id
            WHERE tm.user_id = ? AND tm.tenant_id = ? AND r.name = 'Owner'
        "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(false);

        if is_owner {
            return Ok(vec!["*".to_string()]);
        }

        // Fallback: check if user has NULL role_id but is the tenant owner
        // (old tenants created before create_tenant role_id fix).
        // A global Owner role (tenant_id IS NULL) exists — if the user is
        // a tenant_member with NULL role_id, they're the original owner.
        // Guard: also verify tm.role column says 'Owner'/'admin' to prevent
        // privilege escalation from bugs setting role_id=NULL on non-owners.
        #[cfg(feature = "postgres")]
        let fallback_owner: bool = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) > 0
            FROM tenant_members tm
            CROSS JOIN roles r
            WHERE tm.user_id = $1 AND tm.tenant_id = $2
              AND r.name = 'Owner' AND r.tenant_id IS NULL
              AND tm.role_id IS NULL
              AND tm.role IN ('Owner', 'admin')
        "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(false);

        #[cfg(feature = "sqlite")]
        let fallback_owner: bool = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) > 0
            FROM tenant_members tm
            CROSS JOIN roles r
            WHERE tm.user_id = ? AND tm.tenant_id = ?
              AND r.name = 'Owner' AND r.tenant_id IS NULL
              AND tm.role_id IS NULL
              AND tm.role IN ('Owner', 'admin')
        "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(false);

        if fallback_owner {
            return Ok(vec!["*".to_string()]);
        }

        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT DISTINCT rp.permission_id
            FROM tenant_members tm
            JOIN roles r ON tm.role_id = r.id
            JOIN role_permissions rp ON r.id = rp.role_id
            WHERE tm.user_id = $1 AND tm.tenant_id = $2
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT DISTINCT rp.permission_id
            FROM tenant_members tm
            JOIN roles r ON tm.role_id = r.id
            JOIN role_permissions rp ON r.id = rp.role_id
            WHERE tm.user_id = ? AND tm.tenant_id = ?
        "#;

        let permissions: Vec<String> = sqlx::query_scalar(query)
            .bind(user_id)
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(permissions)
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: &str) -> AppResult<User> {
        repository::get_user_by_id(&self.pool, user_id).await
    }

    /// Get enriched user response (with tenant role and permissions)
    pub async fn get_enriched_user(
        &self,
        user_id: &str,
        tenant_id: Option<String>,
    ) -> AppResult<crate::models::UserResponse> {
        let user = self.get_user_by_id(user_id).await?;
        let mut user_response: crate::models::UserResponse = user.into();

        if let Some(tid) = tenant_id {
            // Get permissions
            let permissions = self.get_user_permissions(user_id, &tid).await?;
            user_response.permissions = permissions.clone();

            // Get tenant role
            if let Ok(Some(tenant_role)) = self.get_tenant_role_name(user_id, &tid).await {
                user_response.role = tenant_role;
            } else if permissions.contains(&"*".to_string()) {
                // Fallback: user has wildcard permission → treat as Owner
                user_response.role = "Owner".to_string();
            }

            // Get tenant slug, custom_domain, and enforce_2fa
            #[cfg(feature = "postgres")]
            let tenant_info: Option<(String, Option<String>, Option<String>, bool)> = sqlx::query_as(
                "SELECT slug, custom_domain, custom_domain_status, enforce_2fa FROM tenants WHERE id = $1",
            )
            .bind(&tid)
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);

            #[cfg(feature = "sqlite")]
            let tenant_info: Option<(String, Option<String>, Option<String>, bool)> = sqlx::query_as(
                "SELECT slug, custom_domain, custom_domain_status, enforce_2fa FROM tenants WHERE id = ?",
            )
            .bind(&tid)
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);

            mapper::apply_tenant_info(&mut user_response, tenant_info);
        }

        Ok(user_response)
    }

    /// Change password
    pub async fn change_password(
        &self,
        user_id: &str,
        old_password: &str,
        new_password: &str,
    ) -> AppResult<()> {
        let settings = self.get_auth_settings().await;

        // Get user to check current password
        let user = self.get_user_by_id(user_id).await?;

        // Verify old password
        if !Self::verify_password(old_password, &user.password_hash)? {
            return Err(AppError::Validation("Invalid current password".to_string()));
        }

        // Validate new password policy
        let validation = self.validate_password(new_password, &settings);
        if !validation.valid {
            return Err(AppError::Validation(validation.errors.join(", ")));
        }

        // Hash new password
        let new_hash = Self::hash_password(new_password)?;

        // Update password
        // Update password
        let query =
            sqlx::query("UPDATE users SET password_hash = $1, updated_at = $2 WHERE id = $3")
                .bind(new_hash);

        #[cfg(feature = "postgres")]
        let query = query.bind(Utc::now());

        #[cfg(not(feature = "postgres"))]
        let query = query.bind(Utc::now().to_rfc3339());

        query.bind(user_id).execute(&self.pool).await?;

        // Logout all sessions if setting is enabled
        if settings.logout_all_on_password_change {
            self.logout_all(user_id).await?;
        }

        Ok(())
    }

    /// Check if a user has a specific permission
    pub async fn has_permission(
        &self,
        user_id: &str,
        tenant_id: &str,
        resource: &str,
        action: &str,
    ) -> AppResult<bool> {
        let perm_id = format!("{}:{}", resource, action);

        // Check 1: Is user Owner? Owners generally bypass or have all permissions.
        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT COUNT(*) FROM tenant_members tm
            JOIN roles r ON tm.role_id = r.id
            LEFT JOIN role_permissions rp ON r.id = rp.role_id
            WHERE tm.user_id = $1 AND tm.tenant_id = $2
            AND (
                r.name = 'Owner'
                OR rp.permission_id = $3
            )
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT COUNT(*) FROM tenant_members tm
            JOIN roles r ON tm.role_id = r.id
            LEFT JOIN role_permissions rp ON r.id = rp.role_id
            WHERE tm.user_id = ? AND tm.tenant_id = ?
            AND (
                r.name = 'Owner'
                OR rp.permission_id = ?
            )
        "#;

        let count: i64 = sqlx::query_scalar(query)
            .bind(user_id)
            .bind(tenant_id)
            .bind(&perm_id)
            .fetch_one(&self.pool)
            .await?;

        if count > 0 {
            return Ok(true);
        }

        // Fallback: check if user has NULL role_id but is the tenant owner
        // (old tenants created before create_tenant role_id fix).
        // Cross-join with global Owner role to detect this case.
        // Guard: also verify tm.role column says 'Owner'/'admin' to prevent
        // privilege escalation from bugs setting role_id=NULL on non-owners.
        #[cfg(feature = "postgres")]
        let fallback_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM tenant_members tm
            CROSS JOIN roles r
            WHERE tm.user_id = $1 AND tm.tenant_id = $2
              AND r.name = 'Owner' AND r.tenant_id IS NULL
              AND tm.role_id IS NULL
              AND tm.role IN ('Owner', 'admin')
        "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let fallback_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM tenant_members tm
            CROSS JOIN roles r
            WHERE tm.user_id = ? AND tm.tenant_id = ?
              AND r.name = 'Owner' AND r.tenant_id IS NULL
              AND tm.role_id IS NULL
              AND tm.role IN ('Owner', 'admin')
        "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(fallback_count > 0)
    }

    /// Enforce permission check (returns Error if denied)
    pub async fn check_permission(
        &self,
        user_id: &str,
        tenant_id: &str,
        resource: &str,
        action: &str,
    ) -> AppResult<()> {
        if self
            .has_permission(user_id, tenant_id, resource, action)
            .await?
        {
            Ok(())
        } else if resource == "customers" && action == "read_own" {
            // Backward compatibility for legacy tenants:
            // Some existing databases may have Customer role assignment without
            // the newer explicit `customers:read_own` role_permission mapping.
            // Allow this specific portal permission when role name is Customer.
            #[cfg(feature = "postgres")]
            let is_customer_role: bool = sqlx::query_scalar(
                r#"
                SELECT EXISTS(
                    SELECT 1
                    FROM tenant_members tm
                    JOIN roles r ON tm.role_id = r.id
                    WHERE tm.user_id = $1
                      AND tm.tenant_id = $2
                      AND lower(r.name) = 'customer'
                )
                "#,
            )
            .bind(user_id)
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;

            #[cfg(feature = "sqlite")]
            let is_customer_role: bool = {
                let raw: i64 = sqlx::query_scalar(
                    r#"
                    SELECT EXISTS(
                        SELECT 1
                        FROM tenant_members tm
                        JOIN roles r ON tm.role_id = r.id
                        WHERE tm.user_id = ?
                          AND tm.tenant_id = ?
                          AND lower(r.name) = 'customer'
                    )
                    "#,
                )
                .bind(user_id)
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await?;
                raw != 0
            };

            if is_customer_role {
                Ok(())
            } else {
                Err(AppError::Forbidden(format!(
                    "Permission denied: {}:{}",
                    resource, action
                )))
            }
        } else {
            Err(AppError::Forbidden(format!(
                "Permission denied: {}:{}",
                resource, action
            )))
        }
    }

    /// Complete Login Flow (Tenant resolution, Token generation)
    pub async fn complete_login(&self, user: crate::models::user::User) -> AppResult<AuthResponse> {
        // Get user's primary ACTIVE tenant (oldest one they joined).
        // If user belongs only to suspended tenants, block login (except superadmin).
        let tenant: Option<crate::models::tenant::Tenant> = sqlx::query_as(
            r#"
            SELECT t.* FROM tenants t
            JOIN tenant_members tm ON t.id = tm.tenant_id
            WHERE tm.user_id = $1
              AND t.is_active = true
            ORDER BY tm.created_at ASC
            LIMIT 1
            "#,
        )
        .bind(&user.id)
        .fetch_optional(&self.pool)
        .await?;

        // If no ACTIVE tenant is found but membership exists, do not self-heal into a new tenant.
        // This prevents bypassing tenant suspension by auto-creating a new tenant.
        if tenant.is_none() && !user.is_super_admin {
            let has_any_membership: bool = sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM tenant_members WHERE user_id = $1",
            )
            .bind(&user.id)
            .fetch_one(&self.pool)
            .await
            .map(|count| count > 0)
            .unwrap_or(false);

            if has_any_membership {
                let details = serde_json::json!({
                    "email": user.email,
                    "reason": "tenant_suspended"
                })
                .to_string();
                self.audit_service
                    .log(
                        Some(&user.id),
                        None,
                        "login_failed",
                        "auth",
                        None,
                        Some(details.as_str()),
                        None,
                    )
                    .await;
                return Err(AppError::Validation(
                    "Tenant is suspended. Please contact support.".to_string(),
                ));
            }
        }

        // Self-healing: Create default tenant if none exists
        let mut tenant = tenant;
        let mut created_tenant = false;
        if tenant.is_none() {
            // Superadmins can login without an active tenant.
            if user.is_super_admin {
                let tenant_id = None;
                let permissions = vec![];
                let (token, expires_at) = self.generate_token(&user, tenant_id.clone()).await?;

                let mut user_response: crate::models::user::UserResponse = user.into();
                user_response.permissions = permissions;
                user_response.tenant_slug = None;
                user_response.tenant_custom_domain = None;

                let details = serde_json::json!({
                    "email": user_response.email,
                    "tenant_id": null,
                    "is_super_admin": true
                })
                .to_string();
                self.audit_service
                    .log(
                        Some(&user_response.id),
                        None,
                        "login",
                        "auth",
                        None,
                        Some(details.as_str()),
                        None,
                    )
                    .await;

                return Ok(AuthResponse {
                    user: user_response,
                    tenant: None,
                    token: Some(token),
                    expires_at: Some(expires_at),
                    message: None,
                    requires_2fa: None,
                    temp_token: None,
                    available_2fa_methods: None,
                    requires_2fa_setup: None,
                });
            }

            info!("User {} has no tenant. Creating default tenant.", user.id);
            created_tenant = true;
            let tenant_name = format!("{}'s Team", user.name);
            let slug = uuid::Uuid::new_v4().to_string(); // Simple slug
            let new_tenant = crate::models::tenant::Tenant::new(tenant_name, slug);

            // Insert Tenant
            #[cfg(feature = "postgres")]
            sqlx::query("INSERT INTO tenants (id, name, slug, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)")
                .bind(&new_tenant.id)
                .bind(&new_tenant.name)
                .bind(&new_tenant.slug)
                .bind(new_tenant.is_active)
                .bind(new_tenant.created_at)
                .bind(new_tenant.updated_at)
                .execute(&self.pool)
                .await?;

            #[cfg(feature = "sqlite")]
            sqlx::query("INSERT INTO tenants (id, name, slug, is_active, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
                .bind(&new_tenant.id)
                .bind(&new_tenant.name)
                .bind(&new_tenant.slug)
                .bind(new_tenant.is_active)
                .bind(new_tenant.created_at.to_rfc3339())
                .bind(new_tenant.updated_at.to_rfc3339())
                .execute(&self.pool)
                .await?;

            // Get Owner Role ID
            #[cfg(feature = "postgres")]
            let owner_role: Option<(String,)> =
                sqlx::query_as("SELECT id FROM roles WHERE name = 'Owner' AND tenant_id IS NULL")
                    .fetch_optional(&self.pool)
                    .await?;

            #[cfg(feature = "sqlite")]
            let owner_role: Option<(String,)> =
                sqlx::query_as("SELECT id FROM roles WHERE name = 'Owner' AND tenant_id IS NULL")
                    .fetch_optional(&self.pool)
                    .await?;

            let role_id = owner_role.map(|r| r.0);

            // Create Member
            let member = crate::models::tenant::TenantMember::new(
                new_tenant.id.clone(),
                user.id.clone(),
                "Owner".to_string(),
                role_id,
            );

            // Insert Member
            #[cfg(feature = "postgres")]
            sqlx::query("INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)")
                .bind(&member.id)
                .bind(&member.tenant_id)
                .bind(&member.user_id)
                .bind(&member.role)
                .bind(&member.role_id)
                .bind(member.created_at)
                .execute(&self.pool)
                .await?;

            #[cfg(feature = "sqlite")]
            sqlx::query("INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES (?, ?, ?, ?, ?, ?)")
                .bind(&member.id)
                .bind(&member.tenant_id)
                .bind(&member.user_id)
                .bind(&member.role)
                .bind(&member.role_id)
                .bind(member.created_at.to_rfc3339())
                .execute(&self.pool)
                .await?;

            tenant = Some(new_tenant);
        }

        let tenant_id = tenant.as_ref().map(|t| t.id.clone());

        // Enforce 2FA if tenant requires it and user doesn't have it enabled
        if let Some(ref t) = tenant {
            if t.enforce_2fa && !user.two_factor_enabled {
                // Generate temp token
                let (token, expires_at) = self.generate_2fa_token(&user).await?;

                // Get minimal user info
                let user_response: crate::models::user::UserResponse = user.into();

                let details = serde_json::json!({
                    "email": user_response.email,
                    "tenant_id": t.id,
                    "reason": "2fa_setup_required"
                })
                .to_string();
                self.audit_service
                    .log(
                        Some(&user_response.id),
                        Some(&t.id),
                        "login_2fa_setup_required",
                        "auth",
                        None,
                        Some(details.as_str()),
                        None,
                    )
                    .await;

                return Ok(AuthResponse {
                    user: user_response,
                    tenant: None,
                    token: None,
                    expires_at: Some(expires_at),
                    message: Some("2FA setup is required by your organization".to_string()),
                    requires_2fa: None,
                    requires_2fa_setup: Some(true),
                    temp_token: Some(token),
                    available_2fa_methods: None,
                });
            }
        }

        // Get permissions if tenant exists
        let permissions = if let Some(tid) = &tenant_id {
            self.get_user_permissions(&user.id, tid).await?
        } else {
            vec![]
        };

        // Generate token
        let (token, expires_at) = self.generate_token(&user, tenant_id.clone()).await?;

        let mut user_response: crate::models::user::UserResponse = user.into();
        user_response.permissions = permissions;
        user_response.tenant_slug = tenant.as_ref().map(|t| t.slug.clone());
        user_response.enforce_2fa =
            tenant.as_ref().map(|t| t.enforce_2fa).unwrap_or(false);
        user_response.tenant_custom_domain = tenant.as_ref().and_then(|t| {
            if t.custom_domain_status.as_deref()
                == Some(crate::models::tenant::CUSTOM_DOMAIN_STATUS_ACTIVE)
            {
                t.custom_domain.clone()
            } else {
                None
            }
        });

        // Override role with tenant role if available
        if let Some(tid) = &tenant_id {
            if let Ok(Some(tenant_role)) = self.get_tenant_role_name(&user_response.id, tid).await {
                user_response.role = tenant_role;
            }
        }

        let details = serde_json::json!({
            "email": user_response.email,
            "tenant_id": tenant_id,
            "tenant_created": created_tenant,
            "is_super_admin": user_response.is_super_admin
        })
        .to_string();
        self.audit_service
            .log(
                Some(&user_response.id),
                tenant_id.as_deref(),
                "login",
                "auth",
                None,
                Some(details.as_str()),
                None,
            )
            .await;

        Ok(AuthResponse {
            user: user_response,
            tenant,
            token: Some(token),
            expires_at: Some(expires_at),
            message: None,
            requires_2fa: None,
            requires_2fa_setup: None,
            temp_token: None,
            available_2fa_methods: None,
        })
    }

    /// Generate temporary 2FA pending token
    async fn generate_2fa_token(
        &self,
        user: &crate::models::user::User,
    ) -> AppResult<(String, String)> {
        let secret = self.jwt_secret.read().await;
        // Settings not needed for this check
        // Short expiry for 2FA challenge (e.g. 5 minutes)
        let expires_at = Utc::now() + Duration::minutes(5);

        let claims = Claims {
            sub: user.id.clone(),
            email: user.email.clone(),
            role: "2fa_pending".to_string(), // Restricted role
            tenant_id: None,
            is_super_admin: false,
            exp: expires_at.timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(format!("Token generation failed: {}", e)))?;

        Ok((token, expires_at.to_rfc3339()))
    }

    /// Enable 2FA: Generate Secret & QR Code
    pub async fn enable_2fa(&self, user_id: &str) -> AppResult<(String, String)> {
        let user = self.get_user_by_id(user_id).await?;

        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            Secret::generate_secret().to_bytes().unwrap(),
            Some(format!("SaaS ({})", user.email)), // Issuer
            user.email.clone(),                     // Account name
        )
        .unwrap();

        let secret = totp.get_secret_base32();
        let qr = totp.get_qr_base64().unwrap();

        Ok((secret, qr))
    }

    /// Verify 2FA Setup
    pub async fn verify_2fa_setup(
        &self,
        user_id: &str,
        secret: &str,
        code: &str,
    ) -> AppResult<Vec<String>> {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            Secret::Encoded(secret.to_string()).to_bytes().unwrap(),
            None,
            "".to_string(),
        )
        .unwrap();

        if !totp.check_current(code).unwrap_or(false) {
            return Err(AppError::Validation("Invalid OTP code".to_string()));
        }

        // Generate recovery codes
        let recovery_codes: Vec<String> = (0..8)
            .map(|_| uuid::Uuid::new_v4().to_string().replace("-", "")[0..10].to_uppercase())
            .collect();
        let recovery_codes_str = serde_json::to_string(&recovery_codes).unwrap();

        // Implement DB Update
        #[cfg(feature = "postgres")]
        sqlx::query("UPDATE users SET two_factor_enabled = true, totp_enabled = true, two_factor_secret = $1, two_factor_recovery_codes = $2, updated_at = $3 WHERE id = $4")
            .bind(secret)
            .bind(&recovery_codes_str)
            .bind(Utc::now())
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE users SET two_factor_enabled = true, totp_enabled = true, two_factor_secret = ?, two_factor_recovery_codes = ?, updated_at = ? WHERE id = ?")
            .bind(secret)
            .bind(&recovery_codes_str)
            .bind(Utc::now().to_rfc3339())
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        self.audit_service
            .log(
                Some(user_id),
                None,
                "USER_2FA_ENABLED",
                "auth",
                None,
                None,
                None,
            )
            .await;

        Ok(recovery_codes)
    }

    /// Disable 2FA
    pub async fn disable_2fa(&self, user_id: &str, code: &str) -> AppResult<()> {
        let user = self.get_user_by_id(user_id).await?;

        if !user.two_factor_enabled {
            return Err(AppError::Validation("2FA is not enabled".to_string()));
        }

        let mut verified = false;

        // 1. Try TOTP if secret exists
        if let Some(secret) = &user.two_factor_secret {
            let totp = TOTP::new(
                Algorithm::SHA1,
                6,
                1,
                30,
                Secret::Encoded(secret.clone()).to_bytes().unwrap(),
                None,
                "".to_string(),
            )
            .unwrap();

            if totp.check_current(code).unwrap_or(false) {
                verified = true;
            }
        }

        // 2. Try Email OTP if not verified yet
        if !verified {
            if let Some(stored_code) = &user.email_otp_code {
                if let Some(expires) = &user.email_otp_expires {
                    if Utc::now() <= *expires && stored_code == code {
                        verified = true;
                    }
                }
            }
        }

        // 3. Try Recovery Codes if not verified yet
        if !verified {
            let recovery_codes: Vec<String> = user
                .two_factor_recovery_codes
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            if recovery_codes.iter().any(|r| r == code) {
                verified = true;
            }
        }

        if !verified {
            return Err(AppError::Validation(
                "Invalid authentication code".to_string(),
            ));
        }

        // DB Update: Clear all 2FA related fields
        #[cfg(feature = "postgres")]
        let query = "UPDATE users SET two_factor_enabled = false, totp_enabled = false, email_2fa_enabled = false, two_factor_secret = NULL, two_factor_recovery_codes = NULL, email_otp_code = NULL, email_otp_expires = NULL, preferred_2fa_method = 'totp', updated_at = $1 WHERE id = $2";

        #[cfg(feature = "postgres")]
        sqlx::query(query)
            .bind(Utc::now())
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE users SET two_factor_enabled = false, totp_enabled = false, email_2fa_enabled = false, two_factor_secret = NULL, two_factor_recovery_codes = NULL, email_otp_code = NULL, email_otp_expires = NULL, preferred_2fa_method = 'totp', updated_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        self.audit_service
            .log(
                Some(user_id),
                None,
                "USER_2FA_DISABLED",
                "auth",
                None,
                None,
                None,
            )
            .await;

        Ok(())
    }

    /// Verify Login 2FA
    pub async fn verify_login_2fa(&self, temp_token: &str, code: &str) -> AppResult<AuthResponse> {
        // 1. Decode temp token (use 2FA token validation - no session lookup)
        let claims = self.validate_2fa_token(temp_token).await?;
        if claims.role != "2fa_pending" {
            return Err(AppError::InvalidToken);
        }

        let user_id = claims.sub;
        let user = self.get_user_by_id(&user_id).await?;

        // 2. Verify Code
        if user.two_factor_enabled {
            if let Some(secret) = &user.two_factor_secret {
                let totp = TOTP::new(
                    Algorithm::SHA1,
                    6,
                    1,
                    30,
                    Secret::Encoded(secret.clone()).to_bytes().unwrap(),
                    None,
                    "".to_string(),
                )
                .unwrap();

                // Check standard TOTP
                if !totp.check_current(code).unwrap_or(false) {
                    // Check recovery codes
                    let mut recovery_codes: Vec<String> = user
                        .two_factor_recovery_codes
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default();

                    if let Some(pos) = recovery_codes.iter().position(|r| r == code) {
                        // Used a recovery code! Remove it.
                        recovery_codes.remove(pos);
                        let new_recovery_str = serde_json::to_string(&recovery_codes).unwrap();

                        // Update DB
                        #[cfg(feature = "postgres")]
                        sqlx::query(
                            "UPDATE users SET two_factor_recovery_codes = $1 WHERE id = $2",
                        )
                        .bind(new_recovery_str)
                        .bind(&user.id)
                        .execute(&self.pool)
                        .await?;

                        #[cfg(feature = "sqlite")]
                        sqlx::query("UPDATE users SET two_factor_recovery_codes = ? WHERE id = ?")
                            .bind(new_recovery_str)
                            .bind(&user.id)
                            .execute(&self.pool)
                            .await?;

                        info!("User {} used a recovery code", user.id);
                    } else {
                        return Err(AppError::Validation("Invalid OTP code".to_string()));
                    }
                }
            } else {
                return Err(AppError::Internal(
                    "2FA enabled but no secret found".to_string(),
                ));
            }
        }

        // 3. Complete Login
        self.complete_login(user).await
    }

    /// Get available 2FA methods from global settings
    pub async fn get_available_2fa_methods(&self) -> Vec<String> {
        let methods_str = sqlx::query_scalar::<_, String>(
            "SELECT value FROM settings WHERE key = '2fa_methods' AND tenant_id IS NULL",
        )
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "totp".to_string());

        methods_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    }

    /// Check if 2FA is globally enabled
    #[allow(dead_code)]
    pub async fn is_2fa_enabled(&self) -> bool {
        sqlx::query_scalar::<_, String>(
            "SELECT value FROM settings WHERE key = '2fa_enabled' AND tenant_id IS NULL",
        )
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true) // Default enabled
    }

    /// Generate Email OTP and send via email
    pub async fn generate_email_otp(&self, user_id: &str) -> AppResult<()> {
        let user = self.get_user_by_id(user_id).await?;

        // Generate 6-digit code
        use rand::Rng;
        let code: u32 = rand::thread_rng().gen_range(100000..999999);
        let code_str = code.to_string();

        // Get expiry from settings (default 5 minutes)
        let expiry_minutes: i64 = sqlx::query_scalar::<_, String>(
            "SELECT value FROM settings WHERE key = '2fa_email_otp_expiry_minutes' AND tenant_id IS NULL"
        )
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);

        let expires_at = Utc::now() + Duration::minutes(expiry_minutes);

        // Store in database
        #[cfg(feature = "postgres")]
        sqlx::query("UPDATE users SET email_otp_code = $1, email_otp_expires = $2 WHERE id = $3")
            .bind(&code_str)
            .bind(expires_at)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE users SET email_otp_code = ?, email_otp_expires = ? WHERE id = ?")
            .bind(&code_str)
            .bind(expires_at.to_rfc3339())
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        // Send email
        let subject = "Your Login Verification Code";
        let body = format!(
            "Hello {},\n\nYour verification code is: {}\n\nThis code will expire in {} minutes.\n\nIf you did not request this code, please ignore this email.",
            user.name, code_str, expiry_minutes
        );

        self.email_service
            .send_email(&user.email, subject, &body)
            .await?;

        info!("Email OTP sent to user {}", user_id);
        Ok(())
    }

    /// Verify Email OTP and complete login
    pub async fn verify_email_otp(&self, temp_token: &str, code: &str) -> AppResult<AuthResponse> {
        // 1. Validate temp token (use 2FA token validation - no session lookup)
        let claims = self.validate_2fa_token(temp_token).await?;

        // 2. Get user
        let user = self.get_user_by_id(&claims.sub).await?;

        // 3. Check code
        if let Some(stored_code) = &user.email_otp_code {
            if let Some(expires) = &user.email_otp_expires {
                if Utc::now() > *expires {
                    return Err(AppError::Validation("OTP code has expired".to_string()));
                }
                if stored_code != code {
                    return Err(AppError::Validation("Invalid OTP code".to_string()));
                }
            } else {
                return Err(AppError::Validation("No OTP code pending".to_string()));
            }
        } else {
            return Err(AppError::Validation("No OTP code pending".to_string()));
        }

        // 4. Clear OTP
        #[cfg(feature = "postgres")]
        sqlx::query(
            "UPDATE users SET email_otp_code = NULL, email_otp_expires = NULL WHERE id = $1",
        )
        .bind(&user.id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            "UPDATE users SET email_otp_code = NULL, email_otp_expires = NULL WHERE id = ?",
        )
        .bind(&user.id)
        .execute(&self.pool)
        .await?;

        // 5. Complete login
        self.complete_login(user).await
    }
    /// Set 2FA Preference (totp or email)
    pub async fn set_2fa_preference(&self, user_id: &str, method: &str) -> AppResult<()> {
        if method != "totp" && method != "email" {
            return Err(AppError::Validation("Invalid 2FA method".to_string()));
        }

        let user = self.get_user_by_id(user_id).await?;

        // Validations
        if method == "totp" && user.two_factor_secret.is_none() {
            return Err(AppError::Validation(
                "Cannot set TOTP as preferred method: TOTP not set up".to_string(),
            ));
        }

        // Update DB
        #[cfg(feature = "postgres")]
        let query = "UPDATE users SET preferred_2fa_method = $1, updated_at = $2 WHERE id = $3";
        #[cfg(feature = "postgres")]
        sqlx::query(query)
            .bind(method)
            .bind(Utc::now())
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE users SET preferred_2fa_method = ?, updated_at = ? WHERE id = ?")
            .bind(method)
            .bind(Utc::now().to_rfc3339())
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    /// Request Email 2FA Setup (Send OTP)
    pub async fn request_email_2fa_setup(&self, user_id: &str) -> AppResult<()> {
        let user = self.get_user_by_id(user_id).await?;
        if user.two_factor_enabled {
            return Err(AppError::Validation("2FA is already enabled".to_string()));
        }
        self.generate_email_otp(user_id).await
    }

    /// Verify and Enable Email 2FA
    pub async fn verify_email_2fa_setup(&self, user_id: &str, code: &str) -> AppResult<()> {
        let user = self.get_user_by_id(user_id).await?;

        // Check code (Reusing logic from verify_email_otp but without token requirement)
        if let Some(stored_code) = &user.email_otp_code {
            if let Some(expires) = &user.email_otp_expires {
                if Utc::now() > *expires {
                    return Err(AppError::Validation("OTP code has expired".to_string()));
                }
                if stored_code != code {
                    return Err(AppError::Validation("Invalid OTP code".to_string()));
                }
            } else {
                return Err(AppError::Validation("No OTP code pending".to_string()));
            }
        } else {
            return Err(AppError::Validation("No OTP code pending".to_string()));
        }

        // Clear OTP & Enable 2FA
        #[cfg(feature = "postgres")]
        let query = "UPDATE users SET email_otp_code = NULL, email_otp_expires = NULL, two_factor_enabled = true, email_2fa_enabled = true, preferred_2fa_method = 'email', updated_at = $1 WHERE id = $2";

        #[cfg(feature = "postgres")]
        sqlx::query(query)
            .bind(Utc::now())
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE users SET email_otp_code = NULL, email_otp_expires = NULL, two_factor_enabled = true, email_2fa_enabled = true, preferred_2fa_method = 'email', updated_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        self.audit_service
            .log(
                Some(user_id),
                None,
                "USER_2FA_ENABLED_EMAIL",
                "auth",
                None,
                None,
                None,
            )
            .await;

        Ok(())
    }

    // ==============================================
    // Trusted Device Methods for 2FA
    // ==============================================

    /// Generate a device fingerprint from user agent and IP address
    pub fn generate_device_fingerprint(
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> String {
        core::generate_device_fingerprint(user_agent, ip_address)
    }

    /// Check if device is trusted for the user
    pub async fn is_device_trusted(
        &self,
        user_id: &str,
        device_fingerprint: &str,
    ) -> AppResult<bool> {
        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT COUNT(*) as count FROM trusted_devices 
            WHERE user_id = $1 
            AND device_fingerprint = $2 
            AND expires_at > NOW()
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT COUNT(*) as count FROM trusted_devices 
            WHERE user_id = ? 
            AND device_fingerprint = ? 
            AND expires_at > datetime('now')
        "#;

        let count: i64 = sqlx::query_scalar(query)
            .bind(user_id)
            .bind(device_fingerprint)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        // Update last_used_at if trusted
        if count > 0 {
            #[cfg(feature = "postgres")]
            let update_query = r#"
                UPDATE trusted_devices SET last_used_at = NOW() 
                WHERE user_id = $1 AND device_fingerprint = $2
            "#;

            #[cfg(feature = "sqlite")]
            let update_query = r#"
                UPDATE trusted_devices SET last_used_at = datetime('now') 
                WHERE user_id = ? AND device_fingerprint = ?
            "#;

            let _ = sqlx::query(update_query)
                .bind(user_id)
                .bind(device_fingerprint)
                .execute(&self.pool)
                .await;
        }

        Ok(count > 0)
    }

    /// Trust a device for the user (default 30 days)
    pub async fn trust_device(
        &self,
        user_id: &str,
        device_fingerprint: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> AppResult<()> {
        let trust_days: i64 = self
            .settings_service
            .get_value(None, "auth_2fa_trust_device_days")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(30);

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::days(trust_days);

        // Use "upsert" pattern - insert or update if exists
        #[cfg(feature = "postgres")]
        let query = r#"
            INSERT INTO trusted_devices (id, user_id, device_fingerprint, ip_address, user_agent, trusted_at, expires_at, last_used_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $6)
            ON CONFLICT (user_id, device_fingerprint) 
            DO UPDATE SET expires_at = $7, last_used_at = $6, ip_address = $4
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            INSERT OR REPLACE INTO trusted_devices (id, user_id, device_fingerprint, ip_address, user_agent, trusted_at, expires_at, last_used_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        #[cfg(feature = "postgres")]
        sqlx::query(query)
            .bind(&id)
            .bind(user_id)
            .bind(device_fingerprint)
            .bind(ip_address)
            .bind(user_agent)
            .bind(now)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(query)
            .bind(&id)
            .bind(user_id)
            .bind(device_fingerprint)
            .bind(ip_address)
            .bind(user_agent)
            .bind(now.to_rfc3339())
            .bind(expires_at.to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(&self.pool)
            .await?;

        info!(
            "Device trusted for user {} (expires in {} days)",
            user_id, trust_days
        );
        Ok(())
    }

    /// Reset 2FA for a user (Admin/Superadmin only)
    pub async fn reset_2fa(
        &self,
        user_id: &str,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        #[cfg(feature = "postgres")]
        let query = "UPDATE users SET two_factor_enabled = false, two_factor_secret = NULL, two_factor_recovery_codes = NULL, email_otp_code = NULL, email_otp_expires = NULL, preferred_2fa_method = 'totp', updated_at = $1 WHERE id = $2";

        #[cfg(feature = "postgres")]
        let rows_affected = sqlx::query(query)
            .bind(Utc::now())
            .bind(user_id)
            .execute(&self.pool)
            .await?
            .rows_affected();

        #[cfg(feature = "sqlite")]
        let rows_affected = sqlx::query("UPDATE users SET two_factor_enabled = false, two_factor_secret = NULL, two_factor_recovery_codes = NULL, email_otp_code = NULL, email_otp_expires = NULL, preferred_2fa_method = 'totp', updated_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(user_id)
            .execute(&self.pool)
            .await?
            .rows_affected();

        if rows_affected == 0 {
            return Err(AppError::NotFound("User not found".to_string()));
        }

        self.audit_service
            .log(
                actor_id,
                None,
                "USER_2FA_RESET_BY_ADMIN",
                "auth",
                Some(user_id),
                Some("2FA reset by administrator"),
                ip_address,
            )
            .await;

        Ok(())
    }

    /// Remove all trusted devices for a user (useful on password change)
    #[allow(dead_code)]
    pub async fn revoke_trusted_devices(&self, user_id: &str) -> AppResult<()> {
        #[cfg(feature = "postgres")]
        let query = "DELETE FROM trusted_devices WHERE user_id = $1";

        #[cfg(feature = "sqlite")]
        let query = "DELETE FROM trusted_devices WHERE user_id = ?";

        sqlx::query(query).bind(user_id).execute(&self.pool).await?;

        Ok(())
    }

    /// List all trusted devices for a user
    pub async fn list_trusted_devices(&self, user_id: &str) -> AppResult<Vec<TrustedDevice>> {
        let devices = sqlx::query_as::<_, TrustedDevice>(
            "SELECT * FROM trusted_devices WHERE user_id = $1 ORDER BY last_used_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(devices)
    }

    /// Revoke a specific trusted device for a user
    pub async fn revoke_trusted_device(&self, user_id: &str, device_id: &str) -> AppResult<()> {
        let rows_affected =
            sqlx::query("DELETE FROM trusted_devices WHERE id = $1 AND user_id = $2")
                .bind(device_id)
                .bind(user_id)
                .execute(&self.pool)
                .await?
                .rows_affected();

        if rows_affected == 0 {
            return Err(AppError::NotFound(
                "Device not found or permission denied".to_string(),
            ));
        }

        Ok(())
    }

    /// Approve a pending user: set registration_status='active', attach to tenant with role.
    pub async fn approve_pending_user(
        &self,
        actor_user_id: &str,
        target_user_id: &str,
        tenant_id: &str,
        role_id: &str,
    ) -> AppResult<()> {
        let now = Utc::now();

        // Optimistic lock: only approve if still pending
        #[cfg(feature = "postgres")]
        let affected = sqlx::query(
            r#"UPDATE users
               SET registration_status = 'active',
                   is_active = true,
                   approved_at = $1,
                   approved_by_user_id = $2,
                   updated_at = $3
               WHERE id = $4 AND registration_status = 'pending'"#,
        )
        .bind(now)
        .bind(actor_user_id)
        .bind(now)
        .bind(target_user_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        #[cfg(not(feature = "postgres"))]
        let affected = {
            let now_str = now.to_rfc3339();
            sqlx::query(
                r#"UPDATE users
                   SET registration_status = 'active',
                       is_active = 1,
                       approved_at = ?,
                       approved_by_user_id = ?,
                       updated_at = ?
                   WHERE id = ? AND registration_status = 'pending'"#,
            )
            .bind(&now_str)
            .bind(actor_user_id)
            .bind(&now_str)
            .bind(target_user_id)
            .execute(&self.pool)
            .await?
            .rows_affected()
        };

        if affected == 0 {
            return Err(AppError::NotFound(
                "User not found or not in pending state".to_string(),
            ));
        }

        // Activate any customer record bound to this user (created via public
        // registration) so that downstream billing flows (subscriptions, invoices,
        // PPPoE accounts) become reachable. Mirror on sqlite using bool-as-int.
        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"UPDATE customers
               SET is_active = true, updated_at = $1
               WHERE tenant_id = $2 AND id IN (
                   SELECT customer_id FROM customer_users
                   WHERE tenant_id = $2 AND user_id = $3
               )"#,
        )
        .bind(now)
        .bind(tenant_id)
        .bind(target_user_id)
        .execute(&self.pool)
        .await?;

        #[cfg(not(feature = "postgres"))]
        {
            let now_str = now.to_rfc3339();
            sqlx::query(
                r#"UPDATE customers
                   SET is_active = 1, updated_at = ? WHERE id IN (
                       SELECT customer_id FROM customer_users
                       WHERE tenant_id = ? AND user_id = ?
                   )"#,
            )
            .bind(&now_str)
            .bind(tenant_id)
            .bind(target_user_id)
            .execute(&self.pool)
            .await?;
        }

        // Attach user to tenant via tenant_members
        let member_id = Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at)
               VALUES ($1, $2, $3, 'Member', $4, $5)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(&member_id)
        .bind(tenant_id)
        .bind(target_user_id)
        .bind(role_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(not(feature = "postgres"))]
        {
            let now_str = now.to_rfc3339();
            sqlx::query(
                r#"INSERT OR IGNORE INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at)
                   VALUES (?, ?, ?, 'Member', ?, ?)"#
            )
            .bind(&member_id)
            .bind(tenant_id)
            .bind(target_user_id)
            .bind(role_id)
            .bind(&now_str)
            .execute(&self.pool)
            .await?;
        }

        // Audit log
        self.audit_service
            .log(
                Some(actor_user_id),
                Some(tenant_id),
                "user.registration_approved",
                "users",
                Some(target_user_id),
                Some(&format!(
                    "Approved pending user and assigned to tenant with role_id={}",
                    role_id
                )),
                None,
            )
            .await;

        Ok(())
    }

    /// Reject a pending user: mark as rejected (soft, not deleted for audit trail).
    pub async fn reject_pending_user(
        &self,
        actor_user_id: &str,
        target_user_id: &str,
        reason: &str,
    ) -> AppResult<()> {
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let affected = sqlx::query(
            r#"UPDATE users
               SET registration_status = 'rejected',
                   rejected_at = $1,
                   rejected_by_user_id = $2,
                   rejected_reason = $3,
                   updated_at = $4
               WHERE id = $5 AND registration_status = 'pending'"#,
        )
        .bind(now)
        .bind(actor_user_id)
        .bind(reason)
        .bind(now)
        .bind(target_user_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        #[cfg(not(feature = "postgres"))]
        let affected = {
            let now_str = now.to_rfc3339();
            sqlx::query(
                r#"UPDATE users
                   SET registration_status = 'rejected',
                       rejected_at = ?,
                       rejected_by_user_id = ?,
                       rejected_reason = ?,
                       updated_at = ?
                   WHERE id = ? AND registration_status = 'pending'"#,
            )
            .bind(&now_str)
            .bind(actor_user_id)
            .bind(reason)
            .bind(&now_str)
            .bind(target_user_id)
            .execute(&self.pool)
            .await?
            .rows_affected()
        };

        if affected == 0 {
            return Err(AppError::NotFound(
                "User not found or not in pending state".to_string(),
            ));
        }

        self.audit_service
            .log(
                Some(actor_user_id),
                None,
                "user.registration_rejected",
                "users",
                Some(target_user_id),
                Some(&format!("Rejected with reason: {}", reason)),
                None,
            )
            .await;

        Ok(())
    }

    /// List all users with registration_status='pending'.
    pub async fn list_pending_users(&self) -> AppResult<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE registration_status = 'pending' ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }
}

#[cfg(test)]
mod tests;
