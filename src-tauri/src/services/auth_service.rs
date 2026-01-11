//! Authentication Service

use crate::db::connection::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{LoginDto, RegisterDto, User, UserResponse, Tenant, TenantMember};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use crate::services::{EmailService, AuditService};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,     // user_id
    pub email: String,
    pub role: String,
    pub tenant_id: Option<String>,
    pub is_super_admin: bool,
    pub exp: usize,      // expiration timestamp
    pub iat: usize,      // issued at
}

/// Authentication response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub tenant: Option<crate::models::tenant::Tenant>,
    pub token: Option<String>,
    pub expires_at: Option<String>,
    pub message: Option<String>,
}

/// Password validation result
#[derive(Debug, Serialize)]
pub struct PasswordValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

/// Auth settings from database
#[derive(Debug, Clone, Serialize)]
pub struct AuthSettings {
    pub jwt_expiry_hours: i64,
    pub password_min_length: usize,
    pub password_require_uppercase: bool,
    pub password_require_number: bool,
    pub password_require_special: bool,
    pub max_login_attempts: i32,
    pub lockout_duration_minutes: i64,
    pub allow_registration: bool,
    pub logout_all_on_password_change: bool,
    pub require_email_verification: bool,
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self {
            jwt_expiry_hours: 24,
            password_min_length: 8,
            password_require_uppercase: true,
            password_require_number: true,
            password_require_special: false,
            max_login_attempts: 5,
            lockout_duration_minutes: 15,
            allow_registration: true,
            logout_all_on_password_change: true,
            require_email_verification: false,
        }
    }
}

/// Auth service for handling authentication
#[derive(Clone)]
pub struct AuthService {
    pub pool: DbPool,
    jwt_secret: Arc<RwLock<String>>,
    email_service: EmailService,
    audit_service: AuditService,
}

impl AuthService {
    pub fn new(pool: DbPool, jwt_secret: String, email_service: EmailService, audit_service: AuditService) -> Self {
        Self {
            pool,
            jwt_secret: Arc::new(RwLock::new(jwt_secret)),
            email_service,
            audit_service,
        }
    }

    /// Get auth settings from database
    pub async fn get_auth_settings(&self) -> AuthSettings {
        let mut settings = AuthSettings::default();
        
        // Helper to get setting value
        async fn get_setting(pool: &DbPool, key: &str) -> Option<String> {
            sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = $1")
                .bind(key)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
        }

        if let Some(val) = get_setting(&self.pool, "auth_jwt_expiry_hours").await {
            settings.jwt_expiry_hours = val.parse().unwrap_or(24);
        }
        if let Some(val) = get_setting(&self.pool, "auth_password_min_length").await {
            settings.password_min_length = val.parse().unwrap_or(8);
        }
        if let Some(val) = get_setting(&self.pool, "auth_password_require_uppercase").await {
            settings.password_require_uppercase = val == "true";
        }
        if let Some(val) = get_setting(&self.pool, "auth_password_require_number").await {
            settings.password_require_number = val == "true";
        }
        if let Some(val) = get_setting(&self.pool, "auth_password_require_special").await {
            settings.password_require_special = val == "true";
        }
        if let Some(val) = get_setting(&self.pool, "auth_max_login_attempts").await {
            settings.max_login_attempts = val.parse().unwrap_or(5);
        }
        if let Some(val) = get_setting(&self.pool, "auth_lockout_duration_minutes").await {
            settings.lockout_duration_minutes = val.parse().unwrap_or(15);
        }
        if let Some(val) = get_setting(&self.pool, "auth_allow_registration").await {
            settings.allow_registration = val == "true";
        }
        if let Some(val) = get_setting(&self.pool, "auth_logout_all_on_password_change").await {
            settings.logout_all_on_password_change = val == "true";
        }
        if let Some(val) = get_setting(&self.pool, "auth_require_email_verification").await {
            settings.require_email_verification = val == "true";
        }

        settings
    }

    /// Validate password against policy
    pub fn validate_password(&self, password: &str, settings: &AuthSettings) -> PasswordValidationResult {
        let mut errors = Vec::new();

        if password.len() < settings.password_min_length {
            errors.push(format!("Password must be at least {} characters", settings.password_min_length));
        }

        if settings.password_require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            errors.push("Password must contain at least one uppercase letter".to_string());
        }

        if settings.password_require_number && !password.chars().any(|c| c.is_numeric()) {
            errors.push("Password must contain at least one number".to_string());
        }

        if settings.password_require_special {
            let special_chars = "!@#$%^&*()_+-=[]{}|;:',.<>?/`~";
            if !password.chars().any(|c| special_chars.contains(c)) {
                errors.push("Password must contain at least one special character".to_string());
            }
        }

        PasswordValidationResult {
            valid: errors.is_empty(),
            errors,
        }
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
    async fn generate_token(&self, user: &User, tenant_id: Option<String>) -> AppResult<(String, String)> {
        let secret = self.jwt_secret.read().await;
        let settings = self.get_auth_settings().await;
        let expires_at = Utc::now() + Duration::hours(settings.jwt_expiry_hours);
        
        let claims = Claims {
            sub: user.id.clone(),
            email: user.email.clone(),
            role: user.role.clone(),
            tenant_id: tenant_id.clone(),
            is_super_admin: user.is_super_admin,
            exp: expires_at.timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(format!("Token generation failed: {}", e)))?;

        // Store session in database
        let session_id = uuid::Uuid::new_v4().to_string();
        let query = sqlx::query(
            "INSERT INTO sessions (id, user_id, tenant_id, token, expires_at, created_at) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(&session_id)
        .bind(&user.id)
        .bind(&tenant_id)
        .bind(&token);

        #[cfg(feature = "postgres")]
        let query = query
            .bind(expires_at)
            .bind(Utc::now());

        #[cfg(not(feature = "postgres"))]
        let query = query
            .bind(expires_at.to_rfc3339())
            .bind(Utc::now().to_rfc3339());

        query.execute(&self.pool).await?;

        Ok((token, expires_at.to_rfc3339()))
    }

    /// Validate JWT token and return claims
    pub async fn validate_token(&self, token: &str) -> AppResult<Claims> {
        // Check if session exists and is valid in database
        let session_exists: bool = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM sessions WHERE token = $1")
            .bind(token)
            .fetch_one(&self.pool)
            .await
            .map(|count| count > 0)
            .unwrap_or(false);

        if !session_exists {
            return Err(AppError::InvalidToken);
        }

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

    /// Check if user has admin role
    /// Check if user has admin role
    pub async fn check_admin(&self, token: &str) -> AppResult<Claims> {
        let claims = self.validate_token(token).await?;
        if claims.role != "admin" {
            return Err(AppError::Unauthorized);
        }
        Ok(claims)
    }

    /// Logout (revoke current session)
    pub async fn logout(&self, token: &str, ip_address: Option<String>) -> AppResult<()> {
        // Try to decode token to get user_id before it's deleted
        if let Ok(claims) = self.validate_token(token).await {
            self.audit_service.log(
                Some(&claims.sub),
                claims.tenant_id.as_deref(),
                "USER_LOGOUT",
                "auth",
                None,
                None,
                ip_address.as_deref()
            ).await;
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
    async fn increment_failed_attempts(&self, user_id: &str, settings: &AuthSettings) -> AppResult<()> {
        // Get current attempts
        let current: (i32,) = sqlx::query_as("SELECT failed_login_attempts FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        
        let new_attempts = current.0 + 1;
        
        if new_attempts >= settings.max_login_attempts {
            // Lock the account
            let locked_until = Utc::now() + Duration::minutes(settings.lockout_duration_minutes);
            warn!("Account {} locked until {} after {} failed attempts", user_id, locked_until, new_attempts);
            
            let query = sqlx::query(
                "UPDATE users SET failed_login_attempts = $1, locked_until = $2, updated_at = $3 WHERE id = $4"
            )
            .bind(new_attempts);

            #[cfg(feature = "postgres")]
            let query = query
                .bind(locked_until)
                .bind(Utc::now());

            #[cfg(not(feature = "postgres"))]
            let query = query
                .bind(locked_until.to_rfc3339())
                .bind(Utc::now().to_rfc3339());

            query.bind(user_id)
            .execute(&self.pool)
            .await?;
        } else {
            let query = sqlx::query(
                "UPDATE users SET failed_login_attempts = $1, updated_at = $2 WHERE id = $3"
            )
            .bind(new_attempts);

            #[cfg(feature = "postgres")]
            let query = query.bind(Utc::now());

            #[cfg(not(feature = "postgres"))]
            let query = query.bind(Utc::now().to_rfc3339());

            query.bind(user_id)
            .execute(&self.pool)
            .await?;
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

        query.bind(user_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    /// Register a new user
    pub async fn register(&self, dto: RegisterDto, ip_address: Option<String>) -> AppResult<AuthResponse> {
        let settings = self.get_auth_settings().await;
        
        // Check if registration is allowed
        if !settings.allow_registration {
            return Err(AppError::Validation("Public registration is currently disabled".to_string()));
        }

        // Validate password against policy
        let validation = self.validate_password(&dto.password, &settings);
        if !validation.valid {
            return Err(AppError::Validation(validation.errors.join(", ")));
        }

        // Check if user exists
        let existing: Option<User> = sqlx::query_as(
            "SELECT * FROM users WHERE email = $1"
        )
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
        
        // Handle email verification
        if settings.require_email_verification {
            let token = uuid::Uuid::new_v4().to_string();
            user.verification_token = Some(token.clone());
        } else {
             user.email_verified_at = Some(Utc::now());
        }

        let query = sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, name, role, is_active, failed_login_attempts, created_at, updated_at, verification_token, email_verified_at)
            VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, $10)
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
            .bind(user.email_verified_at);

        #[cfg(not(feature = "postgres"))]
        let query = query
            .bind(user.created_at.to_rfc3339())
            .bind(user.updated_at.to_rfc3339())
            .bind(&user.verification_token)
            .bind(user.email_verified_at.map(|t| t.to_rfc3339()));

        query.execute(&self.pool).await?;

        info!("New user registered: {}", user.email);

        if settings.require_email_verification {
            // ... existing email code ...
            // Send verification email
            if let Some(token) = &user.verification_token {
                let link = format!("/auth/verify-email?token={}", token);
                
                let body = format!(
                    "Welcome, {}!\n\nPlease verify your email by clicking the link below:\n{}\n\nIf you cannot click the link, use this code: {}",
                    user.name, link, token
                );
                
                if let Err(e) = self.email_service.send_email(&user.email, "Verify your email", &body).await {
                    warn!("Failed to send verification email: {}", e);
                }
            }
            
            self.audit_service.log(
                Some(&user.id),
                None,
                "USER_REGISTER",
                "auth",
                Some(&user.id),
                Some(&format!("Registered via email {}", user.email)),
                ip_address.as_deref()
            ).await;

            Ok(AuthResponse {
                user: user.into(),
                tenant: None,
                token: None,
                expires_at: None,
                message: Some("Registration successful. Please check your email to verify your account.".to_string()),
            })
        } else {
            // Generate token (no tenant for now on direct registration)
            let (token, expires_at) = self.generate_token(&user, None).await?;

            self.audit_service.log(
                Some(&user.id),
                None,
                "USER_REGISTER",
                "auth",
                Some(&user.id),
                Some(&format!("Registered via email {}", user.email)),
                ip_address.as_deref()
            ).await;

            Ok(AuthResponse {
                user: user.into(),
                tenant: None,
                token: Some(token),
                expires_at: Some(expires_at),
                message: None,
                
            })
        }
    }

    /// Verify email with token
    pub async fn verify_email(&self, token: &str) -> AppResult<AuthResponse> {
        // Find user by verification token
        let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE verification_token = $1")
            .bind(token)
            .fetch_optional(&self.pool)
            .await?;

        let user = match user {
            Some(u) => u,
            None => return Err(AppError::Validation("Invalid verification token".to_string())),
        };

        if user.email_verified_at.is_some() {
             return Err(AppError::Validation("Email already verified".to_string()));
        }

        // Update user
        let now = Utc::now();
        let query = sqlx::query("UPDATE users SET email_verified_at = $1, verification_token = NULL, updated_at = $2 WHERE id = $3");

        #[cfg(feature = "postgres")]
        let query = query
            .bind(now)
            .bind(now);

        #[cfg(not(feature = "postgres"))]
        let query = query
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339());

        query.bind(&user.id)
            .execute(&self.pool)
            .await?;

        // Get user's primary tenant
        let tenant: Option<crate::models::tenant::Tenant> = sqlx::query_as(
            r#"
            SELECT t.* FROM tenants t
            JOIN tenant_members tm ON t.id = tm.tenant_id
            WHERE tm.user_id = $1
            LIMIT 1
            "#
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
            let query = query
                .bind(expires_at)
                .bind(Utc::now());

            #[cfg(not(feature = "postgres"))]
            let query = query
                .bind(expires_at.to_rfc3339())
                .bind(Utc::now().to_rfc3339());

            query.bind(&user.id)
                .execute(&self.pool)
                .await?;

            // Send email
            let link = format!("/forgot-password/reset?token={}", token);
            let body = format!(
                "Hello {},\n\nYou requested a password reset. Click the link below to reset your password:\n{}\n\nThis link expires in 1 hour.\n\nIf you did not request this, please ignore this email.",
                user.name, link
            );

            if let Err(e) = self.email_service.send_email(&user.email, "Reset your password", &body).await {
                warn!("Failed to send reset email: {}", e);
            }
        }

        // Always return OK to prevent email enumeration
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
            None => return Err(AppError::Validation("Invalid or expired reset token".to_string())),
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

        query.bind(&user.id)
            .execute(&self.pool)
            .await?;
            
        // Optional: Logout all sessions
        if settings.logout_all_on_password_change {
            self.logout_all(&user.id).await?;
        }

        Ok(())
    }

    /// Login user
    pub async fn login(&self, dto: LoginDto, ip_address: Option<String>) -> AppResult<AuthResponse> {
        let settings = self.get_auth_settings().await;
        
        // Find user by email
        let user: Option<User> = sqlx::query_as(
            "SELECT * FROM users WHERE email = $1"
        )
        .bind(&dto.email)
        .fetch_optional(&self.pool)
        .await?;

        let user = match user {
            Some(u) => u,
            None => {
                // Log failed (user not found - privacy concern? generic message)
                self.audit_service.log(
                    None, None, "USER_LOGIN_FAILED", "auth", None, 
                    Some(&format!("Failed login for {}: User not found", dto.email)), 
                    ip_address.as_deref()
                ).await;
                return Err(AppError::InvalidCredentials)
            },
        };

        // Check if account is locked
        if user.is_locked() {
             self.audit_service.log(
                Some(&user.id), None, "USER_LOGIN_LOCKED", "auth", None, 
                Some("Attempted login on locked account"), 
                ip_address.as_deref()
            ).await;

            let remaining = user.locked_until
                .map(|t| (t - Utc::now()).num_minutes())
                .unwrap_or(0);
            return Err(AppError::Validation(
                format!("Account is locked. Try again in {} minutes", remaining.max(1))
            ));
        }

        // Check if account is active
        if !user.is_active {
            return Err(AppError::Validation("Account is deactivated".to_string()));
        }

        // Check email verification
        if settings.require_email_verification && user.email_verified_at.is_none() {
             return Err(AppError::Validation("Please verify your email address before logging in.".to_string()));
        }

        // Verify password
        if !Self::verify_password(&dto.password, &user.password_hash)? {
            // Increment failed attempts
            self.increment_failed_attempts(&user.id, &settings).await?;
            
            self.audit_service.log(
                Some(&user.id), None, "USER_LOGIN_FAILED", "auth", None, 
                Some("Invalid password"), 
                ip_address.as_deref()
            ).await;

            let remaining = settings.max_login_attempts - user.failed_login_attempts - 1;
            if remaining > 0 {
                return Err(AppError::Validation(
                    format!("Invalid credentials. {} attempts remaining", remaining)
                ));
            } else {
                return Err(AppError::Validation(
                    format!("Account locked for {} minutes", settings.lockout_duration_minutes)
                ));
            }
        }

        // Reset failed attempts on successful login
        self.reset_failed_attempts(&user.id).await?;
        
        // Log Success
        self.audit_service.log(
            Some(&user.id),
            None, 
            "USER_LOGIN",
            "auth",
            None,
            Some("Login successful"),
            ip_address.as_deref()
        ).await;

        info!("User logged in: {}", user.email);

        // Get user's primary tenant
        let tenant: Option<crate::models::tenant::Tenant> = sqlx::query_as(
            r#"
            SELECT t.* FROM tenants t
            JOIN tenant_members tm ON t.id = tm.tenant_id
            WHERE tm.user_id = $1
            LIMIT 1
            "#
        )
        .bind(&user.id)
        .fetch_optional(&self.pool)
        .await?;

        // Self-healing: Create default tenant if none exists
        let mut tenant = tenant;
        if tenant.is_none() {
            info!("User {} has no tenant. Creating default tenant.", user.id);
            let tenant_name = format!("{}'s Team", user.name);
            let slug = uuid::Uuid::new_v4().to_string(); // Simple slug
            let new_tenant = Tenant::new(tenant_name, slug);
            
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
            let owner_role: Option<(String,)> = sqlx::query_as("SELECT id FROM roles WHERE name = 'Owner' AND tenant_id IS NULL")
                .fetch_optional(&self.pool)
                .await?;
            
            #[cfg(feature = "sqlite")]
            let owner_role: Option<(String,)> = sqlx::query_as("SELECT id FROM roles WHERE name = 'Owner' AND tenant_id IS NULL")
                .fetch_optional(&self.pool)
                .await?;

            let role_id = owner_role.map(|r| r.0);
            
            // Create Member
            let member = TenantMember::new(new_tenant.id.clone(), user.id.clone(), "Owner".to_string(), role_id);
            
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

        // Override role with tenant role if available
        if let Some(tid) = &tenant_id {
            if let Ok(Some(tenant_role)) = self.get_tenant_role_name(&user_response.id, tid).await {
                user_response.role = tenant_role;
            }
        }

        Ok(AuthResponse {
            user: user_response,
            tenant,
            token: Some(token),
            expires_at: Some(expires_at),
            message: None,
        })
    }

    /// Get user's role name in a tenant
    pub async fn get_tenant_role_name(&self, user_id: &str, tenant_id: &str) -> AppResult<Option<String>> {
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

        Ok(role_name)
    }

    /// Get all permissions for a user in a tenant
    pub async fn get_user_permissions(&self, user_id: &str, tenant_id: &str) -> AppResult<Vec<String>> {
        // If owner, get all permissions? Or just rely on seeded.
        // Let's simplified: get all distinct permissions assigned to user's role.
        // Also check if user is Owner via role name, if so, maybe grant *all* or specific set.
        // For now, we rely on the seeded permissions which gave Owner all perms.
        
        // Check if user is Owner
        #[cfg(feature = "postgres")]
        let is_owner: bool = sqlx::query_scalar(r#"
            SELECT COUNT(*) > 0 FROM tenant_members tm
            JOIN roles r ON tm.role_id = r.id
            WHERE tm.user_id = $1 AND tm.tenant_id = $2 AND r.name = 'Owner'
        "#)
        .bind(user_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(false);

        #[cfg(feature = "sqlite")]
        let is_owner: bool = sqlx::query_scalar(r#"
            SELECT COUNT(*) > 0 FROM tenant_members tm
            JOIN roles r ON tm.role_id = r.id
            WHERE tm.user_id = ? AND tm.tenant_id = ? AND r.name = 'Owner'
        "#)
        .bind(user_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(false);

        if is_owner {
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
        sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::UserNotFound)
    }

    /// Get enriched user response (with tenant role and permissions)
    pub async fn get_enriched_user(&self, user_id: &str, tenant_id: Option<String>) -> AppResult<crate::models::UserResponse> {
        let user = self.get_user_by_id(user_id).await?;
        let mut user_response: crate::models::UserResponse = user.into();

        if let Some(tid) = tenant_id {
            // Get permissions
            let permissions = self.get_user_permissions(user_id, &tid).await?;
            user_response.permissions = permissions;

            // Get tenant role
            if let Ok(Some(tenant_role)) = self.get_tenant_role_name(user_id, &tid).await {
                user_response.role = tenant_role;
            }

            // Get tenant slug
            #[cfg(feature = "postgres")]
            let slug_query = "SELECT slug FROM tenants WHERE id = $1";
            #[cfg(feature = "sqlite")]
            let slug_query = "SELECT slug FROM tenants WHERE id = ?";

            let slug: Option<String> = sqlx::query_scalar(slug_query)
                .bind(&tid)
                .fetch_optional(&self.pool)
                .await
                .unwrap_or(None);
            
            user_response.tenant_slug = slug;
        }

        Ok(user_response)
    }

    /// Change password
    pub async fn change_password(&self, user_id: &str, old_password: &str, new_password: &str) -> AppResult<()> {
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
        let query = sqlx::query("UPDATE users SET password_hash = $1, updated_at = $2 WHERE id = $3")
            .bind(new_hash);

        #[cfg(feature = "postgres")]
        let query = query.bind(Utc::now());

        #[cfg(not(feature = "postgres"))]
        let query = query.bind(Utc::now().to_rfc3339());

        query.bind(user_id)
            .execute(&self.pool)
            .await?;

        // Logout all sessions if setting is enabled
        if settings.logout_all_on_password_change {
            self.logout_all(user_id).await?;
        }

        Ok(())
    }


    /// Check if a user has a specific permission
    pub async fn has_permission(&self, user_id: &str, tenant_id: &str, resource: &str, action: &str) -> AppResult<bool> {
        let perm_id = format!("{}:{}", resource, action);

        // Check 1: Is user Owner? Owners generally bypass or have all permissions.
        // We can check if they have the 'Owner' role name directly for speed/fallback,
        // or rely on the seeded permissions. Let's rely on seeded permissions + explicit role check for safety.
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

        Ok(count > 0)
    }

    /// Enforce permission check (returns Error if denied)
    pub async fn check_permission(&self, user_id: &str, tenant_id: &str, resource: &str, action: &str) -> AppResult<()> {
        if self.has_permission(user_id, tenant_id, resource, action).await? {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!("Permission denied: {}:{}", resource, action)))
        }
    }
}
