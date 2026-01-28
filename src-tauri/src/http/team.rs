//! Team management HTTP handlers

use axum::{
    extract::{Path, State, ConnectInfo},
    http::HeaderMap,
    Json,
};
use std::net::SocketAddr;
use serde::Deserialize;
use crate::models::TeamMemberWithUser;
use super::{AppState, websocket::WsEvent};
use crate::http::auth::extract_ip;

// Helper to extract token from headers
fn extract_token(headers: &HeaderMap) -> Result<String, crate::error::AppError> {
    headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or_else(|| crate::error::AppError::Unauthorized)
}

/// List all team members
pub async fn list_team_members(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamMemberWithUser>>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    
    let tenant_id = claims.tenant_id
        .ok_or_else(|| crate::error::AppError::Validation("No tenant ID in token".to_string()))?;
    
    let members = state.team_service.list_members(&tenant_id).await?;
    Ok(Json(members))
}

#[derive(Deserialize)]
pub struct AddMemberDto {
    email: String,
    name: String,
    role_id: String,
    password: Option<String>,
}

/// Add a new team member
pub async fn add_team_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<AddMemberDto>,
) -> Result<Json<TeamMemberWithUser>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);
    
    let tenant_id = claims.tenant_id
        .ok_or_else(|| crate::error::AppError::Validation("No tenant ID in token".to_string()))?;
    
    state.auth_service.check_permission(&claims.sub, &tenant_id, "team", "create").await?;
    
    // Check Role Level
    let requester_level = state.team_service.get_user_role_level(&claims.sub, &tenant_id).await
        .map_err(|e| crate::error::AppError::Internal(e))?;
    let new_role_level = state.team_service.get_role_level_by_id(&payload.role_id).await
        .map_err(|e| crate::error::AppError::Internal(e))?;

    if requester_level < new_role_level {
         return Err(crate::error::AppError::Forbidden("Insufficient permissions: Cannot assign role higher than your own".to_string()));
    }

    let member = state.team_service
        .add_member(&tenant_id, &payload.email, &payload.name, &payload.role_id, payload.password, Some(&claims.sub), Some(&ip))
        .await
        .map_err(|e| crate::error::AppError::Internal(e))?;
    
    // Broadcast member added event
    state.ws_hub.broadcast(WsEvent::MemberUpdated { user_id: member.user_id.clone() });
    
    Ok(Json(member))
}

#[derive(Deserialize)]
pub struct UpdateMemberDto {
    role_id: String,
}

/// Update a team member's role
pub async fn update_team_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateMemberDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);
    
    let tenant_id = claims.tenant_id
        .ok_or_else(|| crate::error::AppError::Validation("No tenant ID in token".to_string()))?;
    
    state.auth_service.check_permission(&claims.sub, &tenant_id, "team", "update").await?;
    
    state.team_service
        .update_member(&tenant_id, &id, &payload.role_id, Some(&claims.sub), Some(&ip))
        .await
        .map_err(|e| crate::error::AppError::Internal(e))?;
    
    // Broadcast member updated event - permissions may have changed
    state.ws_hub.broadcast(WsEvent::PermissionsChanged);
    
    Ok(Json(serde_json::json!({"success": true})))
}

/// Remove a team member
pub async fn remove_team_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);
    
    let tenant_id = claims.tenant_id
        .ok_or_else(|| crate::error::AppError::Validation("No tenant ID in token".to_string()))?;
    
    state.auth_service.check_permission(&claims.sub, &tenant_id, "team", "delete").await?;
    
    state.team_service.remove_member(&tenant_id, &id, Some(&claims.sub), Some(&ip)).await
        .map_err(|e| crate::error::AppError::Internal(e))?;
    
    // Broadcast member removed event
    state.ws_hub.broadcast(WsEvent::PermissionsChanged);
    
    Ok(Json(serde_json::json!({"success": true})))
}
