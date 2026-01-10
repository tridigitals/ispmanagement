//! Roles and permissions HTTP handlers

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use crate::models::{RoleWithPermissions, Permission, CreateRoleDto, UpdateRoleDto};
use crate::services::{list_roles, list_permissions, create_role, update_role, delete_role, get_role_by_id};
use super::{AppState, websocket::WsEvent};
use tracing::info;

// Helper to extract token from headers
fn extract_token(headers: &HeaderMap) -> Result<String, crate::error::AppError> {
    headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or_else(|| crate::error::AppError::Unauthorized)
}

/// List all roles
pub async fn get_roles(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<RoleWithPermissions>>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    
    let tenant_id = claims.tenant_id.as_deref();
    let roles = list_roles(&state.auth_service.pool, tenant_id).await?;
    
    Ok(Json(roles))
}

/// Get all available permissions
pub async fn get_permissions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Permission>>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    state.auth_service.validate_token(&token).await?;
    
    let permissions = list_permissions(&state.auth_service.pool).await?;
    Ok(Json(permissions))
}

/// Get a single role by ID
pub async fn get_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Option<RoleWithPermissions>>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    state.auth_service.validate_token(&token).await?;
    
    let role = get_role_by_id(&state.auth_service.pool, &id).await?;
    Ok(Json(role))
}

#[derive(Deserialize)]
pub struct CreateRolePayload {
    name: String,
    description: Option<String>,
    permissions: Vec<String>,
}

/// Create a new role
pub async fn create_new_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateRolePayload>,
) -> Result<Json<RoleWithPermissions>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    
    let tenant_id = claims.tenant_id.as_deref();
    
    // Check permission
    if let Some(tid) = &claims.tenant_id {
        state.auth_service.check_permission(&claims.sub, tid, "roles", "create").await?;
    }
    
    let dto = CreateRoleDto {
        name: payload.name,
        description: payload.description,
        permissions: payload.permissions,
    };
    
    let role = create_role(&state.auth_service.pool, tenant_id, dto).await?;
    
    // Broadcast role created event to all connected clients
    state.ws_hub.broadcast(WsEvent::RoleCreated { role_id: role.id.clone() });
    
    Ok(Json(role))
}

#[derive(Deserialize)]
pub struct UpdateRolePayload {
    name: Option<String>,
    description: Option<String>,
    permissions: Option<Vec<String>>,
}

/// Update an existing role
pub async fn update_existing_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateRolePayload>,
) -> Result<Json<RoleWithPermissions>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    
    // Check permission
    if let Some(tid) = &claims.tenant_id {
        state.auth_service.check_permission(&claims.sub, tid, "roles", "update").await?;
    }
    
    let dto = UpdateRoleDto {
        name: payload.name,
        description: payload.description,
        permissions: payload.permissions,
    };
    
    let role = update_role(&state.auth_service.pool, &id, dto).await?;
    
    // Broadcast role updated event to all connected clients
    state.ws_hub.broadcast(WsEvent::RoleUpdated { role_id: role.id.clone() });
    
    Ok(Json(role))
}

/// Delete a role
pub async fn delete_existing_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    
    // Check permission
    if let Some(tid) = &claims.tenant_id {
        state.auth_service.check_permission(&claims.sub, tid, "roles", "delete").await?;
    }
    
    let deleted = delete_role(&state.auth_service.pool, &id).await?;
    
    // Broadcast role deleted event to all connected clients
    if deleted {
        state.ws_hub.broadcast(WsEvent::RoleDeleted { role_id: id });
    }
    
    Ok(Json(serde_json::json!({"success": deleted})))
}
