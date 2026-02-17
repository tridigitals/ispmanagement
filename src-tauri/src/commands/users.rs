//! User Management Commands

use crate::models::{
    CreateUserAddressDto, CreateUserDto, PaginatedResponse, UpdateUserAddressDto, UpdateUserDto,
    UserAddress, UserResponse,
};
use crate::security::access_rules;
use crate::services::{AuthService, UserService};

use tauri::State;
use validator::Validate;

/// List users with pagination (Super Admin Only)
#[tauri::command]
pub async fn list_users(
    token: String,
    page: Option<u32>,
    per_page: Option<u32>,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<PaginatedResponse<UserResponse>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    // Improved Security: Only Super Admin can list all global users
    if !access_rules::can_access_global_user_management(claims.is_super_admin) {
        return Err("Unauthorized".to_string());
    }

    let page = page.unwrap_or(1);
    let per_page = per_page.unwrap_or(10);

    let (users, total) = user_service
        .list(page, per_page)
        .await
        .map_err(|e| e.to_string())?;

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
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    // Improved Security: Only Super Admin can get arbitrary user details via this API
    if !access_rules::can_access_global_user_management(claims.is_super_admin) {
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
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    // Improved Security: Only Super Admin can create global users directly
    if !access_rules::can_access_global_user_management(claims.is_super_admin) {
        return Err("Unauthorized".to_string());
    }

    let dto = CreateUserDto {
        email,
        password,
        name,
    };

    if let Err(e) = dto.validate() {
        return Err(format!("Validation error: {}", e));
    }

    user_service
        .create(dto, Some(&claims.sub), Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

/// Update user (Super Admin OR Self)
#[tauri::command]
#[allow(clippy::too_many_arguments)]
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
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let attempts_privileged_change = role.is_some() || is_active.is_some();
    if !access_rules::can_update_user(
        claims.is_super_admin,
        &claims.sub,
        &id,
        attempts_privileged_change,
    ) {
        return Err("Unauthorized".to_string());
    }

    let dto = UpdateUserDto {
        email,
        name,
        role,
        is_active,
        is_super_admin: None,
    };

    if let Err(e) = dto.validate() {
        return Err(format!("Validation error: {}", e));
    }

    user_service
        .update(&id, dto, Some(&claims.sub), Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

/// Delete user (Super Admin Only)
#[tauri::command]
pub async fn delete_user(
    token: String,
    id: String,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    // Improved Security: Only Super Admin can delete global users
    if !access_rules::can_access_global_user_management(claims.is_super_admin) {
        return Err("Unauthorized".to_string());
    }

    user_service
        .delete(&id, Some(&claims.sub), Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

/// List current user's addresses
#[tauri::command]
pub async fn list_my_addresses(
    token: String,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<Vec<UserAddress>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    user_service
        .list_addresses(&claims.sub)
        .await
        .map_err(|e| e.to_string())
}

/// Create a new address for current user
#[tauri::command]
pub async fn create_my_address(
    token: String,
    dto: CreateUserAddressDto,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<UserAddress, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    user_service
        .create_address(&claims.sub, dto, Some(&claims.sub), Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

/// Update an existing address for current user
#[tauri::command]
pub async fn update_my_address(
    token: String,
    address_id: String,
    dto: UpdateUserAddressDto,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<UserAddress, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    user_service
        .update_address(
            &claims.sub,
            &address_id,
            dto,
            Some(&claims.sub),
            Some("127.0.0.1"),
        )
        .await
        .map_err(|e| e.to_string())
}

/// Delete an address for current user
#[tauri::command]
pub async fn delete_my_address(
    token: String,
    address_id: String,
    user_service: State<'_, UserService>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    user_service
        .delete_address(
            &claims.sub,
            &address_id,
            Some(&claims.sub),
            Some("127.0.0.1"),
        )
        .await
        .map_err(|e| e.to_string())
}
