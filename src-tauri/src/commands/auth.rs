//! Authentication Commands

use crate::models::{LoginDto, RegisterDto, UserResponse};
use crate::services::{AuthResponse, AuthService, AuditService};
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
    let dto = RegisterDto { email, password, name };
    
    if let Err(e) = dto.validate() {
        return Err(format!("Validation error: {}", e));
    }

    // IP is None for Desktop
    auth_service.register(dto, None).await.map_err(|e| e.to_string())
}

/// Login user
#[tauri::command]
pub async fn login(
    email: String,
    password: String,
    auth_service: State<'_, AuthService>,
) -> Result<AuthResponse, String> {
    let dto = LoginDto { email: email.clone(), password };

    if let Err(e) = dto.validate() {
        return Err(format!("Validation error: {}", e));
    }

    // IP is None for Desktop
    auth_service.login(dto, None).await.map_err(|e| e.to_string())
}

/// Get current user from token
#[tauri::command]
pub async fn get_current_user(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<UserResponse, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    // Pass tenant_id from claims to get enriched data (role, permissions)
    let user_response = auth_service.get_enriched_user(&claims.sub, claims.tenant_id).await.map_err(|e| e.to_string())?;
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
pub async fn logout(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    auth_service.logout(&token, None).await.map_err(|e| e.to_string())
}

/// Change password
#[tauri::command]
pub async fn change_password(
    token: String,
    old_password: String,
    new_password: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    auth_service.change_password(&claims.sub, &old_password, &new_password).await.map_err(|e| e.to_string())
}

/// Verify email
#[tauri::command]
pub async fn verify_email(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<AuthResponse, String> {
    auth_service.verify_email(&token).await.map_err(|e| e.to_string())
}

/// Request password reset (Forgot Password)
#[tauri::command]
pub async fn forgot_password(
    email: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    auth_service.forgot_password(&email).await.map_err(|e| e.to_string())
}

/// Reset password
#[tauri::command]
pub async fn reset_password(
    token: String,
    password: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    auth_service.reset_password(&token, &password).await.map_err(|e| e.to_string())
}
