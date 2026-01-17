//! System Health HTTP Endpoints

use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use super::AppState;
use crate::services::system_service::SystemHealth;

// Helper to check super admin permission
async fn check_super_admin(state: &AppState, headers: &HeaderMap) -> Result<(), crate::error::AppError> {
    let token = headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(token).await?;
    
    if !claims.is_super_admin {
        return Err(crate::error::AppError::Unauthorized);
    }
    
    Ok(())
}

pub async fn get_system_health(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SystemHealth>, crate::error::AppError> {
    check_super_admin(&state, &headers).await?;

    let health = state.system_service.get_system_health().await?;
    
    Ok(Json(health))
}
