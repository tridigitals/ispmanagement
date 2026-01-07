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
    pub token: String,
    pub expires_at: String,
}

/// Password validation result
#[derive(Debug, Serialize)]
pub struct PasswordValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

/// Auth settings from database
#[derive(Debug, Clone)]
pub struct AuthSettings {
    pub jwt_expiry_hours: i64,
    pub password_min_length: usize,
    pub password_require_uppercase: bool,
    pub password_require_number: bool,
    pub password_require_special: bool,
    pub max_login_attempts: i32,
    pub lockout_duration_minutes: i64,
    pub allow_registration: bool,
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
        }
    }
}

/// Auth service for handling authentication
pub struct AuthService {
    pool: Pool<Sqlite>,
    jwt_secret: Arc<RwLock<String>>,
}

impl AuthService {
    pub fn new(pool: Pool<Sqlite>, jwt_secret: String) -> Self {
        Self {
            pool,
            jwt_secret: Arc::new(RwLock::new(jwt_secret)),
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
        let user = User::new(dto.email, password_hash, dto.name);

        sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, name, role, is_active, failed_login_attempts, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?)
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
        .execute(&self.pool)
        .await?;

        info!("New user registered: {}", user.email);

        // Generate token
        let (token, expires_at) = self.generate_token(&user).await?;

        Ok(AuthResponse {
            user: user.into(),
            token,
            expires_at,
        })
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
            token,
            expires_at,
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

        Ok(())
    }
}
