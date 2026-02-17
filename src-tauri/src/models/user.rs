//! User Model with UUID primary key

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// User entity representing a registered user
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String, // UUID as string for SQLite compatibility
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub name: String,
    pub role: String,
    pub is_super_admin: bool,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub verification_token: Option<String>,
    pub reset_token: Option<String>,
    pub reset_token_expires: Option<DateTime<Utc>>,
    // Lockout fields
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // 2FA
    #[serde(default)]
    pub two_factor_enabled: bool,
    #[serde(skip_serializing)]
    pub two_factor_secret: Option<String>,
    #[serde(skip_serializing)]
    pub two_factor_recovery_codes: Option<String>,
    // Email OTP
    #[serde(skip_serializing)]
    pub email_otp_code: Option<String>,
    #[serde(skip_serializing)]
    pub email_otp_expires: Option<DateTime<Utc>>,
    pub preferred_2fa_method: Option<String>,
    // Multi-method 2FA flags
    #[serde(default)]
    pub totp_enabled: bool,
    #[serde(default)]
    pub email_2fa_enabled: bool,
}

impl User {
    /// Create a new user with UUID
    pub fn new(email: String, password_hash: String, name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            email,
            password_hash,
            name,
            role: "user".to_string(),
            is_super_admin: false,
            avatar_url: None,
            is_active: true,
            email_verified_at: None,
            verification_token: None,
            reset_token: None,
            reset_token_expires: None,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: now,
            updated_at: now,
            two_factor_enabled: false,
            two_factor_secret: None,
            two_factor_recovery_codes: None,
            email_otp_code: None,
            email_otp_expires: None,
            preferred_2fa_method: Some("totp".to_string()),
            totp_enabled: false,
            email_2fa_enabled: false,
        }
    }

    /// Check if account is currently locked
    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            Utc::now() < locked_until
        } else {
            false
        }
    }
}

/// User response without sensitive fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub is_super_admin: bool,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub two_factor_enabled: bool,
    pub preferred_2fa_method: Option<String>,
    pub totp_enabled: bool,
    pub email_2fa_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub permissions: Vec<String>,
    pub tenant_slug: Option<String>,
    pub tenant_role: Option<String>,
    pub tenant_custom_domain: Option<String>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            role: user.role,
            is_super_admin: user.is_super_admin,
            avatar_url: user.avatar_url,
            is_active: user.is_active,
            two_factor_enabled: user.two_factor_enabled,
            preferred_2fa_method: user.preferred_2fa_method,
            totp_enabled: user.totp_enabled,
            email_2fa_enabled: user.email_2fa_enabled,
            created_at: user.created_at,
            permissions: vec![],        // Populated by service
            tenant_slug: None,          // Populated by service
            tenant_role: None,          // Populated by service
            tenant_custom_domain: None, // Populated by service
        }
    }
}

/// DTO for creating a new user
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserDto {
    #[validate(
        email(message = "Invalid email format"),
        length(max = 255, message = "Email too long")
    )]
    pub email: String,
    #[validate(length(min = 8, max = 128, message = "Password must be 8-128 characters"))]
    pub password: String,
    #[validate(length(min = 2, max = 100, message = "Name must be 2-100 characters"))]
    pub name: String,
}

/// DTO for updating a user
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserDto {
    #[validate(
        email(message = "Invalid email format"),
        length(max = 255, message = "Email too long")
    )]
    pub email: Option<String>,
    #[validate(length(min = 2, max = 100, message = "Name must be 2-100 characters"))]
    pub name: Option<String>,
    #[validate(length(max = 50, message = "Role too long"))]
    pub role: Option<String>,
    pub is_super_admin: Option<bool>,
    pub is_active: Option<bool>,
}

/// DTO for user login
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct LoginDto {
    #[validate(
        email(message = "Invalid email format"),
        length(max = 255, message = "Email too long")
    )]
    pub email: String,
    #[validate(length(min = 1, max = 128, message = "Password must be 1-128 characters"))]
    pub password: String,
}

/// DTO for user registration
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct RegisterDto {
    #[validate(
        email(message = "Invalid email format"),
        length(max = 255, message = "Email too long")
    )]
    pub email: String,
    #[validate(length(min = 8, max = 128, message = "Password must be 8-128 characters"))]
    pub password: String,
    #[validate(length(min = 2, max = 100, message = "Name must be 2-100 characters"))]
    pub name: String,
}
