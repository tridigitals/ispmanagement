use crate::models::{CreateUserDto, UserResponse, UpdateUserDto};
use crate::services::UserService;
use tauri::State;

#[tauri::command]
pub async fn is_installed(user_service: State<'_, UserService>) -> Result<bool, String> {
    // Check if any users exist using the count method
    let count = user_service.count().await.map_err(|e| e.to_string())?;
    Ok(count > 0)
}

#[tauri::command]
pub async fn install_app(
    admin_name: String,
    admin_email: String,
    admin_password: String,
    user_service: State<'_, UserService>,
) -> Result<UserResponse, String> {
    // Double check if installed
    let count = user_service.count().await.map_err(|e| e.to_string())?;
    if count > 0 {
        return Err("Application already installed".to_string());
    }

    let dto = CreateUserDto {
        email: admin_email,
        password: admin_password,
        name: admin_name,
    };

    // Create the user (defaults to role="user")
    let user_res = user_service.create(dto, None, Some("127.0.0.1")).await.map_err(|e| e.to_string())?;
    
    // Update role to admin
    let update_dto = UpdateUserDto {
        email: None,
        name: None,
        role: Some("admin".to_string()),
        is_super_admin: Some(true),
        is_active: Some(true),
    };
    
    user_service.update(&user_res.id, update_dto, None, Some("127.0.0.1")).await.map_err(|e| e.to_string())
}
