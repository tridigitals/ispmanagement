//! Roles and permissions HTTP handlers

use super::{websocket::WsEvent, AppState};
use crate::http::auth::extract_ip;
use crate::models::{CreateRoleDto, Permission, RoleWithPermissions, UpdateRoleDto};
use axum::{
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use std::net::SocketAddr;

// Helper to extract token from headers
fn extract_token(headers: &HeaderMap) -> Result<String, crate::error::AppError> {
    headers
        .get("Authorization")
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

    if let Some(tid) = claims.tenant_id.as_deref() {
        state
            .auth_service
            .check_permission(&claims.sub, tid, "roles", "read")
            .await?;
    } else if !claims.is_super_admin {
        return Err(crate::error::AppError::Forbidden("Forbidden".to_string()));
    }

    let tenant_id = claims.tenant_id.as_deref();
    let roles = state.role_service.list_roles(tenant_id).await?;

    Ok(Json(roles))
}

/// Get all available permissions
pub async fn get_permissions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Permission>>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    if let Some(tid) = claims.tenant_id.as_deref() {
        state
            .auth_service
            .check_permission(&claims.sub, tid, "roles", "read")
            .await?;
    } else if !claims.is_super_admin {
        return Err(crate::error::AppError::Forbidden("Forbidden".to_string()));
    }

    let permissions = state.role_service.list_permissions().await?;
    Ok(Json(permissions))
}

/// Get a single role by ID
pub async fn get_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Option<RoleWithPermissions>>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    if let Some(tid) = claims.tenant_id.as_deref() {
        state
            .auth_service
            .check_permission(&claims.sub, tid, "roles", "read")
            .await?;
    } else if !claims.is_super_admin {
        return Err(crate::error::AppError::Forbidden("Forbidden".to_string()));
    }

    let role = state.role_service.get_role_by_id(&id).await?;
    Ok(Json(role))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRolePayload {
    name: String,
    description: Option<String>,
    level: Option<i32>,
    permissions: Vec<String>,
}

/// Create a new role
pub async fn create_new_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<CreateRolePayload>,
) -> Result<Json<RoleWithPermissions>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);

    let tenant_id = claims.tenant_id.as_deref();

    // Check permission
    if let Some(tid) = &claims.tenant_id {
        state
            .auth_service
            .check_permission(&claims.sub, tid, "roles", "create")
            .await?;
    }

    let dto = CreateRoleDto {
        name: payload.name,
        description: payload.description,
        level: payload.level,
        permissions: payload.permissions,
    };

    let role = state
        .role_service
        .create_role(tenant_id, dto, Some(&claims.sub), Some(&ip))
        .await?;

    // Broadcast role created event to all connected clients
    state.ws_hub.broadcast(WsEvent::RoleCreated {
        role_id: role.id.clone(),
    });

    Ok(Json(role))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateRolePayload {
    name: Option<String>,
    description: Option<String>,
    level: Option<i32>,
    permissions: Option<Vec<String>>,
}

/// Update an existing role
pub async fn update_existing_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateRolePayload>,
) -> Result<Json<RoleWithPermissions>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);

    // Check permission
    if let Some(tid) = &claims.tenant_id {
        state
            .auth_service
            .check_permission(&claims.sub, tid, "roles", "update")
            .await?;
    }

    let dto = UpdateRoleDto {
        name: payload.name,
        description: payload.description,
        level: payload.level,
        permissions: payload.permissions,
    };

    let role = state
        .role_service
        .update_role(
            &id,
            dto,
            claims.is_super_admin,
            Some(&claims.sub),
            Some(&ip),
        )
        .await?;

    // Broadcast role updated event to all connected clients
    state.ws_hub.broadcast(WsEvent::RoleUpdated {
        role_id: role.id.clone(),
    });

    Ok(Json(role))
}

/// Delete a role
pub async fn delete_existing_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);

    // Check permission
    if let Some(tid) = &claims.tenant_id {
        state
            .auth_service
            .check_permission(&claims.sub, tid, "roles", "delete")
            .await?;
    }

    let deleted = state
        .role_service
        .delete_role(&id, claims.is_super_admin, Some(&claims.sub), Some(&ip))
        .await?;

    // Broadcast role deleted event to all connected clients
    if deleted {
        state.ws_hub.broadcast(WsEvent::RoleDeleted { role_id: id });
    }

    Ok(Json(serde_json::json!({"success": deleted})))
}
