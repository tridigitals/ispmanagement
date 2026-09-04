//! Team management HTTP handlers

use super::{websocket::WsEvent, AppState};
use crate::error::AppError;
use crate::http::auth::extract_ip;
use crate::models::TeamMemberWithUser;
use axum::{
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use std::net::SocketAddr;

fn enforce_member_role_change_permissions(
    requester_level: i32,
    target_level: i32,
    new_role_level: i32,
) -> Result<(), String> {
    if requester_level <= target_level {
        return Err(
            "Insufficient permissions: Cannot edit member with equal or higher role".to_string(),
        );
    }

    if requester_level < new_role_level {
        return Err(
            "Insufficient permissions: Cannot assign role higher than your own".to_string(),
        );
    }

    Ok(())
}

fn map_team_service_error(msg: String) -> crate::error::AppError {
    if msg.to_lowercase().contains("not found") {
        crate::error::AppError::NotFound(msg)
    } else {
        crate::error::AppError::Internal(msg)
    }
}

// Helper to extract token from headers
fn extract_token(headers: &HeaderMap) -> Result<String, crate::error::AppError> {
    crate::http::extract_token(headers)
}

/// List all team members
pub async fn list_team_members(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamMemberWithUser>>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| crate::error::AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "read")
        .await?;

    let members = state.team_service.list_members(&tenant_id).await?;
    Ok(Json(members))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddMemberDto {
    email: String,
    name: String,
    #[serde(rename = "roleId", alias = "role_id")]
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

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| crate::error::AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "create")
        .await?;

    // Check Role Level
    let requester_level = state
        .team_service
        .get_user_role_level(&claims.sub, &tenant_id)
        .await
        .map_err(crate::error::AppError::Internal)?;
    let new_role_level = state
        .team_service
        .get_role_level_by_id(&payload.role_id)
        .await
        .map_err(crate::error::AppError::Internal)?;

    if requester_level < new_role_level {
        return Err(crate::error::AppError::Forbidden(
            "Insufficient permissions: Cannot assign role higher than your own".to_string(),
        ));
    }

    // Block assigning Customer role via team management.
    // Customer accounts must be created from the Customers module.
    let role_name = state
        .team_service
        .get_role_name_by_id(&payload.role_id)
        .await
        .map_err(crate::error::AppError::Internal)?;

    if role_name.as_deref() == Some("Customer") {
        return Err(crate::error::AppError::Validation(
            "Cannot assign Customer role via team management. Create customer accounts from the Customers module instead.".to_string(),
        ));
    }

    let member = state
        .team_service
        .add_member(
            &tenant_id,
            &payload.email,
            &payload.name,
            &payload.role_id,
            payload.password,
            Some(&claims.sub),
            Some(&ip),
        )
        .await
        .map_err(crate::error::AppError::Internal)?;

    // Broadcast member added event
    state.ws_hub.broadcast(WsEvent::MemberUpdated {
        user_id: member.user_id.clone(),
    });

    Ok(Json(member))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateMemberDto {
    #[serde(rename = "roleId", alias = "role_id")]
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

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| crate::error::AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "update")
        .await?;

    let requester_level = state
        .team_service
        .get_user_role_level(&claims.sub, &tenant_id)
        .await
        .map_err(crate::error::AppError::Internal)?;
    let target_level = state
        .team_service
        .get_member_role_level(&id)
        .await
        .map_err(crate::error::AppError::Internal)?;
    let new_role_level = state
        .team_service
        .get_role_level_by_id(&payload.role_id)
        .await
        .map_err(crate::error::AppError::Internal)?;

    enforce_member_role_change_permissions(requester_level, target_level, new_role_level)
        .map_err(crate::error::AppError::Forbidden)?;

    // Block assigning Customer role via team management.
    let role_name = state
        .team_service
        .get_role_name_by_id(&payload.role_id)
        .await
        .map_err(crate::error::AppError::Internal)?;

    if role_name.as_deref() == Some("Customer") {
        return Err(crate::error::AppError::Validation(
            "Cannot assign Customer role via team management. Create customer accounts from the Customers module instead.".to_string(),
        ));
    }

    state
        .team_service
        .update_member(
            &tenant_id,
            &id,
            &payload.role_id,
            Some(&claims.sub),
            Some(&ip),
        )
        .await
        .map_err(map_team_service_error)?;

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

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| crate::error::AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "delete")
        .await?;

    let requester_level = state
        .team_service
        .get_user_role_level(&claims.sub, &tenant_id)
        .await
        .map_err(crate::error::AppError::Internal)?;
    let target_level = state
        .team_service
        .get_member_role_level(&id)
        .await
        .map_err(crate::error::AppError::Internal)?;

    if requester_level <= target_level {
        return Err(crate::error::AppError::Forbidden(
            "Insufficient permissions: Cannot remove member with equal or higher role".to_string(),
        ));
    }

    state
        .team_service
        .remove_member(&tenant_id, &id, Some(&claims.sub), Some(&ip))
        .await
        .map_err(map_team_service_error)?;

    // Broadcast member removed event
    state.ws_hub.broadcast(WsEvent::PermissionsChanged);

    Ok(Json(serde_json::json!({"success": true})))
}

/// List soft-deleted team members
pub async fn list_deleted_members(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamMemberWithUser>>, AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "read")
        .await?;

    let members = state
        .team_service
        .list_deleted_members(&tenant_id)
        .await
        .map_err(|e| map_team_service_error(e.to_string()))?;

    Ok(Json(members))
}

/// Restore a soft-deleted team member
pub async fn restore_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<serde_json::Value>, AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "update")
        .await?;

    let ip = extract_ip(&headers, addr);
    state
        .team_service
        .restore_member(&tenant_id, &id, Some(&claims.sub), Some(&ip))
        .await
        .map_err(map_team_service_error)?;

    state.ws_hub.broadcast(WsEvent::PermissionsChanged);

    Ok(Json(serde_json::json!({"success": true})))
}

/// Permanently delete a soft-deleted team member
pub async fn hard_delete_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<serde_json::Value>, AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "delete")
        .await?;

    let ip = extract_ip(&headers, addr);
    state
        .team_service
        .hard_delete_member(&tenant_id, &id, Some(&claims.sub), Some(&ip))
        .await
        .map_err(map_team_service_error)?;

    state.ws_hub.broadcast(WsEvent::PermissionsChanged);

    Ok(Json(serde_json::json!({"success": true})))
}
