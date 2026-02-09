//! MikroTik router inventory + monitoring commands (tenant admin).

use crate::models::{
    CreateMikrotikRouterRequest, MikrotikRouter, MikrotikRouterMetric, MikrotikTestResult,
    UpdateMikrotikRouterRequest,
};
use crate::services::{AuthService, MikrotikService};
use tauri::State;

#[tauri::command]
pub async fn list_mikrotik_routers(
    token: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<Vec<MikrotikRouter>, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .list_routers(&tenant_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_mikrotik_router(
    token: String,
    router: CreateMikrotikRouterRequest,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikRouter, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .create_router(&tenant_id, router)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_mikrotik_router(
    token: String,
    id: String,
    router: UpdateMikrotikRouterRequest,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikRouter, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .update_router(&tenant_id, &id, router)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_mikrotik_router(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<(), String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .delete_router(&tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_mikrotik_router(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikTestResult, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .test_connection(&tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mikrotik_router_metrics(
    token: String,
    router_id: String,
    limit: Option<u32>,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<Vec<MikrotikRouterMetric>, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .list_metrics(&tenant_id, &router_id, limit.unwrap_or(120))
        .await
        .map_err(|e| e.to_string())
}

