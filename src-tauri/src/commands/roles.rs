//! RBAC (Roles and Permissions) commands

use crate::http::{websocket::WsEvent, WsHub};
use crate::models::{CreateRoleDto, Permission, RoleWithPermissions, UpdateRoleDto};
use crate::services::{AuthService, RoleService};
use std::sync::Arc;
use tauri::State;

/// List all roles (global + tenant-specific)
#[tauri::command]
pub async fn get_roles(
    token: String,
    auth: State<'_, AuthService>,
    role_service: State<'_, RoleService>,
) -> Result<Vec<RoleWithPermissions>, String> {
    // Validate token and get tenant_id
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims.tenant_id.as_deref();

    role_service
        .list_roles(tenant_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all available permissions
#[tauri::command]
pub async fn get_permissions(
    token: String,
    auth: State<'_, AuthService>,
    role_service: State<'_, RoleService>,
) -> Result<Vec<Permission>, String> {
    auth.validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    role_service
        .list_permissions()
        .await
        .map_err(|e| e.to_string())
}

/// Get a single role by ID
#[tauri::command]
pub async fn get_role(
    token: String,
    role_id: String,
    auth: State<'_, AuthService>,
    role_service: State<'_, RoleService>,
) -> Result<Option<RoleWithPermissions>, String> {
    auth.validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    role_service
        .get_role_by_id(&role_id)
        .await
        .map_err(|e| e.to_string())
}

/// Create a new role
#[tauri::command]
pub async fn create_new_role(
    token: String,
    name: String,
    description: Option<String>,
    level: i32,
    permissions: Vec<String>,
    auth: State<'_, AuthService>,
    role_service: State<'_, RoleService>,
    ws_hub: State<'_, Arc<WsHub>>,
) -> Result<RoleWithPermissions, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims.tenant_id.as_deref();

    // Permission Check
    if let Some(tid) = tenant_id {
        auth.check_permission(&claims.sub, tid, "roles", "create")
            .await
            .map_err(|e| e.to_string())?;
    } else if !claims.is_super_admin {
        return Err("Unauthorized: Only Super Admin can create global roles".to_string());
    }

    let dto = CreateRoleDto {
        name,
        description,
        level: Some(level),
        permissions,
    };

    let role = role_service
        .create_role(tenant_id, dto, Some(&claims.sub), Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())?;

    // Broadcast role created event
    ws_hub.broadcast(WsEvent::RoleCreated {
        role_id: role.id.clone(),
    });

    Ok(role)
}

/// Update an existing role
#[tauri::command]
pub async fn update_existing_role(
    token: String,
    role_id: String,
    name: Option<String>,
    description: Option<String>,
    level: Option<i32>,
    permissions: Option<Vec<String>>,
    auth: State<'_, AuthService>,
    role_service: State<'_, RoleService>,
    ws_hub: State<'_, Arc<WsHub>>,
) -> Result<RoleWithPermissions, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims.tenant_id.as_deref();

    // Permission Check
    if let Some(tid) = tenant_id {
        auth.check_permission(&claims.sub, tid, "roles", "update")
            .await
            .map_err(|e| e.to_string())?;
    } else if !claims.is_super_admin {
        return Err("Unauthorized: Only Super Admin can update global roles".to_string());
    }

    let dto = UpdateRoleDto {
        name,
        description,
        level,
        permissions,
    };

    let role = role_service
        .update_role(
            &role_id,
            dto,
            claims.is_super_admin,
            Some(&claims.sub),
            Some("127.0.0.1"),
        )
        .await
        .map_err(|e| e.to_string())?;

    // Broadcast role updated event
    ws_hub.broadcast(WsEvent::RoleUpdated {
        role_id: role.id.clone(),
    });

    Ok(role)
}

/// Delete a role
#[tauri::command]
pub async fn delete_existing_role(
    token: String,
    role_id: String,
    auth: State<'_, AuthService>,
    role_service: State<'_, RoleService>,
    ws_hub: State<'_, Arc<WsHub>>,
) -> Result<bool, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims.tenant_id.as_deref();

    // Permission Check
    if let Some(tid) = tenant_id {
        auth.check_permission(&claims.sub, tid, "roles", "delete")
            .await
            .map_err(|e| e.to_string())?;
    } else if !claims.is_super_admin {
        return Err("Unauthorized: Only Super Admin can delete global roles".to_string());
    }

    let deleted = role_service
        .delete_role(
            &role_id,
            claims.is_super_admin,
            Some(&claims.sub),
            Some("127.0.0.1"),
        )
        .await
        .map_err(|e| e.to_string())?;

    // Broadcast role deleted event
    if deleted {
        ws_hub.broadcast(WsEvent::RoleDeleted { role_id });
    }

    Ok(deleted)
}
