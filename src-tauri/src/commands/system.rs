//! System Health Tauri Commands

use crate::services::metrics_service::MetricsService;
use crate::services::system_service::SystemHealth;
use crate::services::{AuthService, SystemService};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_system_health(
    token: String,
    auth_service: State<'_, AuthService>,
    system_service: State<'_, SystemService>,
    metrics_service: State<'_, Arc<MetricsService>>,
) -> Result<SystemHealth, String> {
    // Validate token and check super admin
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized: Super Admin access required".to_string());
    }

    let mut health = system_service
        .get_system_health()
        .await
        .map_err(|e| e.to_string())?;

    // Inject request metrics
    health.request_metrics = Some(metrics_service.get_metrics());

    Ok(health)
}
