use crate::models::UserResponse;
use serde::{Deserialize, Serialize};

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub email: String,
    pub role: String,
    pub tenant_id: Option<String>,
    pub is_super_admin: bool,
    pub exp: usize, // expiration timestamp
    pub iat: usize, // issued at
}

/// Authentication response
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AuthResponse {
    pub user: UserResponse,
    pub tenant: Option<crate::models::tenant::Tenant>,
    pub token: Option<String>,
    pub expires_at: Option<String>,
    pub message: Option<String>,
    pub requires_2fa: Option<bool>,
    pub requires_2fa_setup: Option<bool>,
    pub temp_token: Option<String>,
    pub available_2fa_methods: Option<Vec<String>>,
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
    pub session_timeout_minutes: i64,
    pub password_min_length: usize,
    pub password_require_uppercase: bool,
    pub password_require_number: bool,
    pub password_require_special: bool,
    pub max_login_attempts: i32,
    pub lockout_duration_minutes: i64,
    pub allow_registration: bool,
    pub logout_all_on_password_change: bool,
    pub require_email_verification: bool,
    pub main_domain: Option<String>,
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self {
            jwt_expiry_hours: 24,
            session_timeout_minutes: 60,
            password_min_length: 8,
            password_require_uppercase: true,
            password_require_number: true,
            password_require_special: false,
            max_login_attempts: 5,
            lockout_duration_minutes: 15,
            allow_registration: false,
            logout_all_on_password_change: true,
            require_email_verification: false,
            main_domain: std::env::var("APP_MAIN_DOMAIN").ok(),
        }
    }
}
