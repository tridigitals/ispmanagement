//! Authentication Service

use crate::error::{AppError, AppResult};
use crate::models::{LoginDto, RegisterDto, User, UserResponse};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use crate::services::EmailService;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,     // user_id
    pub email: String,
    pub role: String,
    pub exp: usize,      // expiration timestamp
    pub iat: usize,      // issued at
}

/// Authentication response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
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
pub struct AuthService {
    pool: Pool<Sqlite>,
    jwt_secret: Arc<RwLock<String>>,
    email_service: EmailService,
}

impl AuthService {
    pub fn new(pool: Pool<Sqlite>, jwt_secret: String, email_service: EmailService) -> Self {
        Self {
            pool,
            jwt_secret: Arc::new(RwLock::new(jwt_secret)),
            email_service,
        }
    }

    /// Get auth settings from database
    pub async fn get_auth_settings(&self) -> AuthSettings {
        let mut settings = AuthSettings::default();
        
        // Helper to get setting value
        async fn get_setting(pool: &Pool<Sqlite>, key: &str) -> Option<String> {
            sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
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
    async fn generate_token(&self, user: &User) -> AppResult<(String, String)> {
        let secret = self.jwt_secret.read().await;
        let settings = self.get_auth_settings().await;
        let expires_at = Utc::now() + Duration::hours(settings.jwt_expiry_hours);
        
        let claims = Claims {
            sub: user.id.clone(),
            email: user.email.clone(),
            role: user.role.clone(),
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
        sqlx::query(
            "INSERT INTO sessions (id, user_id, token, expires_at, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&session_id)
        .bind(&user.id)
        .bind(&token)
        .bind(expires_at.to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok((token, expires_at.to_rfc3339()))
    }

    /// Validate JWT token and return claims
    pub async fn validate_token(&self, token: &str) -> AppResult<Claims> {
        // Check if session exists and is valid in database
        let session_exists: bool = sqlx::query_scalar("SELECT count(*) FROM sessions WHERE token = ?")
            .bind(token)
            .fetch_one(&self.pool)
            .await
            .map(|count: i64| count > 0)
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
    pub async fn check_admin(&self, token: &str) -> AppResult<()> {
        let claims = self.validate_token(token).await?;
        if claims.role != "admin" {
            return Err(AppError::Unauthorized);
        }
        Ok(())
    }

    /// Logout (revoke current session)
    pub async fn logout(&self, token: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Logout from all devices (revoke all sessions for user)
    pub async fn logout_all(&self, user_id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Increment failed login attempts
    async fn increment_failed_attempts(&self, user_id: &str, settings: &AuthSettings) -> AppResult<()> {
        // Get current attempts
        let current: (i32,) = sqlx::query_as("SELECT failed_login_attempts FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        
        let new_attempts = current.0 + 1;
        
        if new_attempts >= settings.max_login_attempts {
            // Lock the account
            let locked_until = Utc::now() + Duration::minutes(settings.lockout_duration_minutes);
            warn!("Account {} locked until {} after {} failed attempts", user_id, locked_until, new_attempts);
            
            sqlx::query(
                "UPDATE users SET failed_login_attempts = ?, locked_until = ?, updated_at = datetime('now') WHERE id = ?"
            )
            .bind(new_attempts)
            .bind(locked_until.to_rfc3339())
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE users SET failed_login_attempts = ?, updated_at = datetime('now') WHERE id = ?"
            )
            .bind(new_attempts)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        }
        
        Ok(())
    }

    /// Reset failed login attempts on successful login
    async fn reset_failed_attempts(&self, user_id: &str) -> AppResult<()> {
        sqlx::query(
            "UPDATE users SET failed_login_attempts = 0, locked_until = NULL, updated_at = datetime('now') WHERE id = ?"
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    /// Register a new user
    pub async fn register(&self, dto: RegisterDto) -> AppResult<AuthResponse> {
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
            "SELECT * FROM users WHERE email = ?"
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
            // is_active could be false until verified, but usually we let them login with restrictions or block login
            // For now, let's keep is_active=true but check email_verified_at on login if strict mode
            // Actually, best practice is prevent login if not verified.
            // Let's set is_active = true, but use email_verified_at as gate.
        } else {
             user.email_verified_at = Some(Utc::now());
        }

        sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, name, role, is_active, failed_login_attempts, created_at, updated_at, verification_token, email_verified_at)
            VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, ?)
            "#,
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.name)
        .bind(&user.role)
        .bind(user.is_active)
        .bind(user.created_at.to_rfc3339())
        .bind(user.updated_at.to_rfc3339())
        .bind(&user.verification_token)
        .bind(user.email_verified_at.map(|t| t.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        info!("New user registered: {}", user.email);

        if settings.require_email_verification {
            // Send verification email
            if let Some(token) = &user.verification_token {
                // Link: /auth/verify?token=...
                // Assuming client runs on localhost or deployed domain.
                // Since this is a Tauri app, we might need a deep link or a frontend route.
                // We'll use a relative path for the frontend to handle.
                // Or better, just send the token code.
                let link = format!("/auth/verify-email?token={}", token);
                // In a real app, verify_url should be from settings
                
                let body = format!(
                    "Welcome, {}!\n\nPlease verify your email by clicking the link below:\n{}\n\nIf you cannot click the link, use this code: {}",
                    user.name, link, token
                );
                
                // Fire and forget email to not block registration? Or await?
                // Await for now to ensure it works.
                if let Err(e) = self.email_service.send_email(&user.email, "Verify your email", &body).await {
                    warn!("Failed to send verification email: {}", e);
                }
            }

            Ok(AuthResponse {
                user: user.into(),
                token: None,
                expires_at: None,
                message: Some("Registration successful. Please check your email to verify your account.".to_string()),
            })
        } else {
            // Generate token
            let (token, expires_at) = self.generate_token(&user).await?;

            Ok(AuthResponse {
                user: user.into(),
                token: Some(token),
                expires_at: Some(expires_at),
                message: None,
            })
        }
    }

    /// Verify email with token
    pub async fn verify_email(&self, token: &str) -> AppResult<AuthResponse> {
        // Find user by verification token
        let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE verification_token = ?")
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
        sqlx::query("UPDATE users SET email_verified_at = datetime('now'), verification_token = NULL, updated_at = datetime('now') WHERE id = ?")
            .bind(&user.id)
            .execute(&self.pool)
            .await?;

        // Login user
        let (jwt, expires_at) = self.generate_token(&user).await?;
        
        // Refresh user data
        let updated_user = self.get_user_by_id(&user.id).await?;

        Ok(AuthResponse {
            user: updated_user.into(),
            token: Some(jwt),
            expires_at: Some(expires_at),
            message: Some("Email verified successfully".to_string()),
        })
    }

    /// Request password reset
    pub async fn forgot_password(&self, email: &str) -> AppResult<()> {
        let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = ?")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(user) = user {
            // Generate reset token
            let token = uuid::Uuid::new_v4().to_string();
            let expires_at = Utc::now() + Duration::hours(1);

            sqlx::query("UPDATE users SET reset_token = ?, reset_token_expires = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(&token)
                .bind(expires_at.to_rfc3339())
                .bind(&user.id)
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
                // Don't fail the request to prevent enumeration? 
                // Actually returning error is helpful for legitimate users. 
                // Security-wise, generic message is better.
            }
        }

        // Always return OK to prevent email enumeration
        Ok(())
    }

    /// Reset password with token
    pub async fn reset_password(&self, token: &str, new_password: &str) -> AppResult<()> {
        let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE reset_token = ?")
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
        
        sqlx::query("UPDATE users SET password_hash = ?, reset_token = NULL, reset_token_expires = NULL, updated_at = datetime('now') WHERE id = ?")
            .bind(new_hash)
            .bind(&user.id)
            .execute(&self.pool)
            .await?;
            
        // Optional: Logout all sessions
        if settings.logout_all_on_password_change {
            self.logout_all(&user.id).await?;
        }

        Ok(())
    }

    /// Login user
    pub async fn login(&self, dto: LoginDto) -> AppResult<AuthResponse> {
        let settings = self.get_auth_settings().await;
        
        // Find user by email (get all users including inactive for proper error handling)
        let user: Option<User> = sqlx::query_as(
            "SELECT * FROM users WHERE email = ?"
        )
        .bind(&dto.email)
        .fetch_optional(&self.pool)
        .await?;

        let user = match user {
            Some(u) => u,
            None => return Err(AppError::InvalidCredentials),
        };

        // Check if account is locked
        if user.is_locked() {
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

        info!("User logged in: {}", user.email);

        // Generate token
        let (token, expires_at) = self.generate_token(&user).await?;

        Ok(AuthResponse {
            user: user.into(),
            token: Some(token),
            expires_at: Some(expires_at),
            message: None,
        })
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: &str) -> AppResult<User> {
        sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::UserNotFound)
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
        sqlx::query("UPDATE users SET password_hash = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(new_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        // Logout all sessions if setting is enabled
        if settings.logout_all_on_password_change {
            self.logout_all(user_id).await?;
        }

        Ok(())
    }
}
