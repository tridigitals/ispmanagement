//! Authentication Commands

use crate::models::{LoginDto, RegisterDto, UserResponse};
use crate::services::{AuthResponse, AuthService};
use tauri::State;
use validator::Validate;

/// Register a new user
#[tauri::command]
pub async fn register(
    email: String,
    password: String,
    name: String,
    auth_service: State<'_, AuthService>,
) -> Result<AuthResponse, String> {
    let dto = RegisterDto {
        email,
        password,
        name,
    };

    if let Err(e) = dto.validate() {
        return Err(format!("Validation error: {}", e));
    }

    // IP is None for Desktop
    auth_service
        .register(dto, None)
        .await
        .map_err(|e| e.to_string())
}

/// Login user
#[tauri::command]
pub async fn login(
    email: String,
    password: String,
    auth_service: State<'_, AuthService>,
) -> Result<AuthResponse, String> {
    let dto = LoginDto {
        email: email.clone(),
        password,
    };

    if let Err(e) = dto.validate() {
        return Err(format!("Validation error: {}", e));
    }

    // IP is None for Desktop
    let fingerprint = AuthService::generate_device_fingerprint(Some("Desktop App"), None);
    auth_service
        .login(dto, None, Some(fingerprint))
        .await
        .map_err(|e| e.to_string())
}

/// Get current user from token
#[tauri::command]
pub async fn get_current_user(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<UserResponse, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    // Pass tenant_id from claims to get enriched data (role, permissions)
    let user_response = auth_service
        .get_enriched_user(&claims.sub, claims.tenant_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(user_response)
}

/// Validate token (check if still valid)
#[tauri::command]
pub async fn validate_token(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<bool, String> {
    match auth_service.validate_token(&token).await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Logout user
#[tauri::command]
pub async fn logout(token: String, auth_service: State<'_, AuthService>) -> Result<(), String> {
    auth_service
        .logout(&token, None)
        .await
        .map_err(|e| e.to_string())
}

/// Change password
#[tauri::command]
pub async fn change_password(
    token: String,
    old_password: String,
    new_password: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    auth_service
        .change_password(&claims.sub, &old_password, &new_password)
        .await
        .map_err(|e| e.to_string())
}

/// Verify email
#[tauri::command]
pub async fn verify_email(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<AuthResponse, String> {
    auth_service
        .verify_email(&token)
        .await
        .map_err(|e| e.to_string())
}

/// Request password reset (Forgot Password)
#[tauri::command]
pub async fn forgot_password(
    email: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    auth_service
        .forgot_password(&email)
        .await
        .map_err(|e| e.to_string())
}

/// Reset password
#[tauri::command]
pub async fn reset_password(
    token: String,
    password: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    auth_service
        .reset_password(&token, &password)
        .await
        .map_err(|e| e.to_string())
}

/// Enable 2FA: Returns secret and QR code (base64)
#[tauri::command]
pub async fn enable_2fa(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<(String, String), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    auth_service
        .enable_2fa(&claims.sub)
        .await
        .map_err(|e| e.to_string())
}

/// Verify 2FA Setup: Returns recovery codes
#[tauri::command]
pub async fn verify_2fa_setup(
    token: String,
    secret: String,
    code: String,
    auth_service: State<'_, AuthService>,
) -> Result<Vec<String>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    auth_service
        .verify_2fa_setup(&claims.sub, &secret, &code)
        .await
        .map_err(|e| e.to_string())
}

/// Disable 2FA
#[tauri::command]
pub async fn disable_2fa(
    token: String,
    code: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    auth_service
        .disable_2fa(&claims.sub, &code)
        .await
        .map_err(|e| e.to_string())
}

/// Verify Login 2FA: Exchange temp token + code for full session
#[tauri::command]
pub async fn verify_login_2fa(
    temp_token: String,
    code: String,
    trust_device: Option<bool>,
    auth_service: State<'_, AuthService>,
) -> Result<AuthResponse, String> {
    let response = auth_service
        .verify_login_2fa(&temp_token, &code)
        .await
        .map_err(|e| e.to_string())?;

    if trust_device.unwrap_or(false) {
        let fingerprint = AuthService::generate_device_fingerprint(Some("Desktop App"), None);
        let _ = auth_service
            .trust_device(&response.user.id, &fingerprint, None, Some("Desktop App"))
            .await;
    }

    Ok(response)
}

/// Request Email OTP: Send verification code via email
#[tauri::command]
pub async fn request_email_otp(
    temp_token: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_2fa_token(&temp_token)
        .await
        .map_err(|e| e.to_string())?;
    auth_service
        .generate_email_otp(&claims.sub)
        .await
        .map_err(|e| e.to_string())
}

/// Verify Email OTP: Exchange temp token + email code for full session
#[tauri::command]
pub async fn verify_email_otp(
    temp_token: String,
    code: String,
    trust_device: Option<bool>,
    auth_service: State<'_, AuthService>,
) -> Result<AuthResponse, String> {
    let response = auth_service
        .verify_email_otp(&temp_token, &code)
        .await
        .map_err(|e| e.to_string())?;

    if trust_device.unwrap_or(false) {
        let fingerprint = AuthService::generate_device_fingerprint(Some("Desktop App"), None);
        let _ = auth_service
            .trust_device(&response.user.id, &fingerprint, None, Some("Desktop App"))
            .await;
    }

    Ok(response)
}

/// Get available 2FA methods
#[tauri::command]
pub async fn get_2fa_methods(auth_service: State<'_, AuthService>) -> Result<Vec<String>, String> {
    Ok(auth_service.get_available_2fa_methods().await)
}
/// Request Email 2FA Setup (Send OTP)
#[tauri::command]
pub async fn request_email_2fa_setup(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    auth_service
        .request_email_2fa_setup(&claims.sub)
        .await
        .map_err(|e| e.to_string())
}

/// Verify Email 2FA Setup
#[tauri::command]
pub async fn verify_email_2fa_setup(
    token: String,
    code: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    auth_service
        .verify_email_2fa_setup(&claims.sub, &code)
        .await
        .map_err(|e| e.to_string())
}

/// Set 2FA Preference (totp or email)
#[tauri::command]
pub async fn set_2fa_preference(
    token: String,
    method: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    auth_service
        .set_2fa_preference(&claims.sub, &method)
        .await
        .map_err(|e| e.to_string())
}
