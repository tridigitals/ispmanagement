//! User Management Commands

use crate::models::{CreateUserDto, UpdateUserDto, UserResponse};
use crate::services::{AuthService, UserService};
use serde::Serialize;
use tauri::State;
use validator::Validate;

/// Paginated response wrapper
#[derive(Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}

/// List users with pagination
#[tauri::command]
pub async fn list_users(
    token: String,
    page: Option<u32>,
    per_page: Option<u32>,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<PaginatedResponse<UserResponse>, String> {
    auth_service.check_admin(&token).await.map_err(|e| e.to_string())?;

    let page = page.unwrap_or(1);
    let per_page = per_page.unwrap_or(10);

    let (users, total) = user_service.list(page, per_page).await.map_err(|e| e.to_string())?;

    Ok(PaginatedResponse {
        data: users,
        total,
        page,
        per_page,
    })
}

/// Get user by ID
#[tauri::command]
pub async fn get_user(
    token: String,
    id: String,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<UserResponse, String> {
    auth_service.check_admin(&token).await.map_err(|e| e.to_string())?;
    user_service.get_by_id(&id).await.map_err(|e| e.to_string())
}

/// Create new user
#[tauri::command]
pub async fn create_user(
    token: String,
    email: String,
    password: String,
    name: String,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<UserResponse, String> {
    auth_service.check_admin(&token).await.map_err(|e| e.to_string())?;

    let dto = CreateUserDto { email, password, name };

    if let Err(e) = dto.validate() {
        return Err(format!("Validation error: {}", e));
    }

    user_service.create(dto).await.map_err(|e| e.to_string())
}

/// Update user
#[tauri::command]
pub async fn update_user(
    token: String,
    id: String,
    email: Option<String>,
    name: Option<String>,
    role: Option<String>,
    is_active: Option<bool>,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<UserResponse, String> {
    // Note: This is strictly for Admin usage. 
    // Self-update should use a different command or logic if needed, but 'update_user' implies admin control (e.g. changing roles).
    // If a user wants to update their own profile, we might need a separate 'update_profile' command or check if id == current_user.id.
    // For now, locking this to Admin is safe for the "Admin Page" requirement.
    // However, the "Profile" page uses 'update_user'. We need to handle that!
    
    // Check if admin OR if updating self
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    
    if claims.role != "admin" && claims.sub != id {
        return Err("Unauthorized".to_string());
    }
    
    // Prevent non-admins from changing their own role or active status
    if claims.role != "admin" && (role.is_some() || is_active.is_some()) {
         return Err("Unauthorized: Cannot change role or status".to_string());
    }

    let dto = UpdateUserDto { email, name, role, is_active };

    if let Err(e) = dto.validate() {
        return Err(format!("Validation error: {}", e));
    }

    user_service.update(&id, dto).await.map_err(|e| e.to_string())
}

/// Delete user
#[tauri::command]
pub async fn delete_user(
    token: String,
    id: String,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    auth_service.check_admin(&token).await.map_err(|e| e.to_string())?;
    user_service.delete(&id).await.map_err(|e| e.to_string())
}
