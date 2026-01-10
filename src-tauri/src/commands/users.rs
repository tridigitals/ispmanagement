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


/// List users with pagination (Super Admin Only)
#[tauri::command]
pub async fn list_users(
    token: String,
    page: Option<u32>,
    per_page: Option<u32>,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<PaginatedResponse<UserResponse>, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;

    // Improved Security: Only Super Admin can list all global users
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

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

/// Get user by ID (Super Admin Only)
#[tauri::command]
pub async fn get_user(
    token: String,
    id: String,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<UserResponse, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;

    // Improved Security: Only Super Admin can get arbitrary user details via this API
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    user_service.get_by_id(&id).await.map_err(|e| e.to_string())
}

/// Create new user (Super Admin Only)
#[tauri::command]
pub async fn create_user(
    token: String,
    email: String,
    password: String,
    name: String,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<UserResponse, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;

    // Improved Security: Only Super Admin can create global users directly
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let dto = CreateUserDto { email, password, name };

    if let Err(e) = dto.validate() {
        return Err(format!("Validation error: {}", e));
    }

    user_service.create(dto).await.map_err(|e| e.to_string())
}

/// Update user (Super Admin OR Self)
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
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    
    // Security Fix: Prevent Tenant Admins (role=admin) from updating arbitrary global users.
    // Allow update if:
    // 1. User is Super Admin
    // 2. User is updating themselves
    let is_self = claims.sub == id;
    
    if !claims.is_super_admin && !is_self {
        return Err("Unauthorized".to_string());
    }
    
    // Additional restriction: Non-superadmins cannot change role or active status
    if !claims.is_super_admin && (role.is_some() || is_active.is_some()) {
         return Err("Unauthorized: Cannot change role or status".to_string());
    }

    let dto = UpdateUserDto { email, name, role, is_active, is_super_admin: None };

    if let Err(e) = dto.validate() {
        return Err(format!("Validation error: {}", e));
    }

    user_service.update(&id, dto).await.map_err(|e| e.to_string())
}

/// Delete user (Super Admin Only)
#[tauri::command]
pub async fn delete_user(
    token: String,
    id: String,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;

    // Improved Security: Only Super Admin can delete global users
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    user_service.delete(&id).await.map_err(|e| e.to_string())
}
