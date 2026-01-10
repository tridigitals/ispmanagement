//! RBAC (Roles and Permissions) commands

use tauri::State;
use std::sync::Arc;
use crate::services::{AuthService, list_roles, list_permissions, create_role, update_role, delete_role, get_role_by_id};
use crate::models::{RoleWithPermissions, Permission, CreateRoleDto, UpdateRoleDto};
use crate::http::{WsHub, websocket::WsEvent};

/// List all roles (global + tenant-specific)
#[tauri::command]
pub async fn get_roles(
    token: String,
    auth: State<'_, AuthService>,
) -> Result<Vec<RoleWithPermissions>, String> {
    // Validate token and get tenant_id
    let claims = auth.validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    
    let tenant_id = claims.tenant_id.as_deref();
    
    list_roles(&auth.pool, tenant_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all available permissions
#[tauri::command]
pub async fn get_permissions(
    token: String,
    auth: State<'_, AuthService>,
) -> Result<Vec<Permission>, String> {
    auth.validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    
    list_permissions(&auth.pool)
        .await
        .map_err(|e| e.to_string())
}

/// Get a single role by ID
#[tauri::command]
pub async fn get_role(
    token: String,
    role_id: String,
    auth: State<'_, AuthService>,
) -> Result<Option<RoleWithPermissions>, String> {
    auth.validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    
    get_role_by_id(&auth.pool, &role_id)
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
    ws_hub: State<'_, Arc<WsHub>>,
) -> Result<RoleWithPermissions, String> {
    let claims = auth.validate_token(&token)
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
    
    let role = create_role(&auth.pool, tenant_id, dto)
        .await
        .map_err(|e| e.to_string())?;
    
    // Broadcast role created event
    ws_hub.broadcast(WsEvent::RoleCreated { role_id: role.id.clone() });
    
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
    ws_hub: State<'_, Arc<WsHub>>,
) -> Result<RoleWithPermissions, String> {
    let claims = auth.validate_token(&token)
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
    
    let role = update_role(&auth.pool, &role_id, dto)
        .await
        .map_err(|e| e.to_string())?;
    
    // Broadcast role updated event
    ws_hub.broadcast(WsEvent::RoleUpdated { role_id: role.id.clone() });
    
    Ok(role)
}

/// Delete a role
#[tauri::command]
pub async fn delete_existing_role(
    token: String,
    role_id: String,
    auth: State<'_, AuthService>,
    ws_hub: State<'_, Arc<WsHub>>,
) -> Result<bool, String> {
    let claims = auth.validate_token(&token)
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
    
    let deleted = delete_role(&auth.pool, &role_id)
        .await
        .map_err(|e| e.to_string())?;
    
    // Broadcast role deleted event
    if deleted {
        ws_hub.broadcast(WsEvent::RoleDeleted { role_id });
    }
    
    Ok(deleted)
}
