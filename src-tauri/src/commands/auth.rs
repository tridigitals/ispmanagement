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
    let dto = RegisterDto { email, password, name };
    
    if let Err(e) = dto.validate() {
        return Err(format!("Validation error: {}", e));
    }

    auth_service.register(dto).await.map_err(|e| e.to_string())
}

/// Login user
#[tauri::command]
pub async fn login(
    email: String,
    password: String,
    auth_service: State<'_, AuthService>,
) -> Result<AuthResponse, String> {
    let dto = LoginDto { email, password };

    if let Err(e) = dto.validate() {
        return Err(format!("Validation error: {}", e));
    }

    auth_service.login(dto).await.map_err(|e| e.to_string())
}

/// Get current user from token
#[tauri::command]
pub async fn get_current_user(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<UserResponse, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    let user = auth_service.get_user_by_id(&claims.sub).await.map_err(|e| e.to_string())?;
    Ok(user.into())
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
    auth_service.logout(&token).await.map_err(|e| e.to_string())
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
