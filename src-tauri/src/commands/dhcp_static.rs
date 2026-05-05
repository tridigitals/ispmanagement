use crate::models::{
    CreateDhcpStaticServiceRequest, DhcpStaticQueueMode, DhcpStaticServicePublic,
    PaginatedResponse, UpdateDhcpStaticServiceRequest,
};
use crate::services::{AuthService, DhcpStaticServiceManager};
use tauri::State;

#[tauri::command]
pub async fn list_dhcp_static_services(
    token: String,
    customer_id: Option<String>,
    location_id: Option<String>,
    router_id: Option<String>,
    dhcp_server_name: Option<String>,
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    auth: State<'_, AuthService>,
    dhcp: State<'_, DhcpStaticServiceManager>,
) -> Result<PaginatedResponse<DhcpStaticServicePublic>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    dhcp.list_services(
        &claims.sub,
        &tenant_id,
        customer_id,
        location_id,
        router_id,
        dhcp_server_name,
        q,
        page.unwrap_or(1),
        per_page.unwrap_or(25),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_dhcp_static_service(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    dhcp: State<'_, DhcpStaticServiceManager>,
) -> Result<DhcpStaticServicePublic, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    dhcp.get_service(&claims.sub, &tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_dhcp_static_service(
    token: String,
    subscription_id: String,
    router_id: String,
    customer_id: String,
    location_id: String,
    package_id: String,
    dhcp_server_name: String,
    mac_address: String,
    ip_address: String,
    comment: Option<String>,
    disabled: Option<bool>,
    queue_mode: Option<DhcpStaticQueueMode>,
    queue_rate_limit: Option<String>,
    work_order_id: Option<String>,
    auth: State<'_, AuthService>,
    dhcp: State<'_, DhcpStaticServiceManager>,
) -> Result<DhcpStaticServicePublic, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    dhcp.create_service(
        &claims.sub,
        &tenant_id,
        CreateDhcpStaticServiceRequest {
            subscription_id,
            router_id,
            customer_id,
            location_id,
            package_id,
            dhcp_server_name,
            mac_address,
            ip_address,
            comment,
            disabled,
            queue_mode,
            queue_rate_limit,
            work_order_id,
        },
        Some("127.0.0.1"),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_dhcp_static_service(
    token: String,
    id: String,
    router_id: Option<String>,
    package_id: Option<String>,
    dhcp_server_name: Option<String>,
    mac_address: Option<String>,
    ip_address: Option<String>,
    comment: Option<String>,
    disabled: Option<bool>,
    queue_mode: Option<DhcpStaticQueueMode>,
    queue_rate_limit: Option<String>,
    work_order_id: Option<String>,
    auth: State<'_, AuthService>,
    dhcp: State<'_, DhcpStaticServiceManager>,
) -> Result<DhcpStaticServicePublic, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    dhcp.update_service(
        &claims.sub,
        &tenant_id,
        &id,
        UpdateDhcpStaticServiceRequest {
            router_id,
            package_id,
            dhcp_server_name,
            mac_address,
            ip_address,
            comment,
            disabled,
            queue_mode,
            queue_rate_limit,
            work_order_id,
        },
        Some("127.0.0.1"),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_dhcp_static_service(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    dhcp: State<'_, DhcpStaticServiceManager>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    dhcp.delete_service(&claims.sub, &tenant_id, &id, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn apply_dhcp_static_service(
    token: String,
    id: String,
    work_order_id: Option<String>,
    auth: State<'_, AuthService>,
    dhcp: State<'_, DhcpStaticServiceManager>,
) -> Result<DhcpStaticServicePublic, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    dhcp.apply_service(
        &claims.sub,
        &tenant_id,
        &id,
        work_order_id.as_deref(),
        Some("127.0.0.1"),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reconcile_dhcp_static_router(
    token: String,
    router_id: String,
    auth: State<'_, AuthService>,
    dhcp: State<'_, DhcpStaticServiceManager>,
) -> Result<serde_json::Value, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    dhcp.reconcile_router(&claims.sub, &tenant_id, &router_id, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}
