//! System Health Tauri Commands

use crate::services::{AuthService, SystemService};
use crate::services::system_service::SystemHealth;
use tauri::State;

#[tauri::command]
pub async fn get_system_health(
    token: String,
    auth_service: State<'_, AuthService>,
    system_service: State<'_, SystemService>,
) -> Result<SystemHealth, String> {
    // Validate token and check super admin
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    
    if !claims.is_super_admin {
        return Err("Unauthorized: Super Admin access required".to_string());
    }

    system_service.get_system_health().await.map_err(|e| e.to_string())
}
