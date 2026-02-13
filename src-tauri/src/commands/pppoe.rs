
use crate::models::{
    CreatePppoeAccountRequest, PaginatedResponse, PppoeAccountPublic, PppoeImportCandidate,
    PppoeImportFromRouterRequest, PppoeImportResult, UpdatePppoeAccountRequest,
};
use crate::services::{AuthService, PppoeService};
use tauri::State;

#[tauri::command]
pub async fn list_pppoe_accounts(
    token: String,
    customer_id: Option<String>,
    location_id: Option<String>,
    router_id: Option<String>,
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    auth: State<'_, AuthService>,
    pppoe: State<'_, PppoeService>,
) -> Result<PaginatedResponse<PppoeAccountPublic>, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims.tenant_id.ok_or_else(|| "No tenant ID in token".to_string())?;

    pppoe
        .list_accounts(
            &claims.sub,
            &tenant_id,
            customer_id,
            location_id,
            router_id,
            q,
            page.unwrap_or(1),
            per_page.unwrap_or(25),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_pppoe_account(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    pppoe: State<'_, PppoeService>,
) -> Result<PppoeAccountPublic, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims.tenant_id.ok_or_else(|| "No tenant ID in token".to_string())?;

    pppoe
        .get_account(&claims.sub, &tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_pppoe_account(
    token: String,
    router_id: String,
    customer_id: String,
    location_id: String,
    username: String,
    password: String,
    profile_id: Option<String>,
    router_profile_name: Option<String>,
    remote_address: Option<String>,
    address_pool: Option<String>,
    disabled: Option<bool>,
    comment: Option<String>,
    auth: State<'_, AuthService>,
    pppoe: State<'_, PppoeService>,
) -> Result<PppoeAccountPublic, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims.tenant_id.ok_or_else(|| "No tenant ID in token".to_string())?;

    let dto = CreatePppoeAccountRequest {
        router_id,
        customer_id,
        location_id,
        username,
        password,
        profile_id,
        router_profile_name,
        remote_address,
        address_pool,
        disabled,
        comment,
    };

    pppoe
        .create_account(&claims.sub, &tenant_id, dto, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_pppoe_account(
    token: String,
    id: String,
    username: Option<String>,
    password: Option<String>,
    profile_id: Option<String>,
    router_profile_name: Option<String>,
    remote_address: Option<String>,
    address_pool: Option<String>,
    disabled: Option<bool>,
    comment: Option<String>,
    auth: State<'_, AuthService>,
    pppoe: State<'_, PppoeService>,
) -> Result<PppoeAccountPublic, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims.tenant_id.ok_or_else(|| "No tenant ID in token".to_string())?;

    let dto = UpdatePppoeAccountRequest {
        username,
        password,
        profile_id,
        router_profile_name,
        remote_address,
        address_pool,
        disabled,
        comment,
    };

    pppoe
        .update_account(&claims.sub, &tenant_id, &id, dto, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_pppoe_account(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    pppoe: State<'_, PppoeService>,
) -> Result<(), String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims.tenant_id.ok_or_else(|| "No tenant ID in token".to_string())?;

    pppoe
        .delete_account(&claims.sub, &tenant_id, &id, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn apply_pppoe_account(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    pppoe: State<'_, PppoeService>,
) -> Result<PppoeAccountPublic, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims.tenant_id.ok_or_else(|| "No tenant ID in token".to_string())?;

    pppoe
        .apply_account(&claims.sub, &tenant_id, &id, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reconcile_pppoe_router(
    token: String,
    router_id: String,
    auth: State<'_, AuthService>,
    pppoe: State<'_, PppoeService>,
) -> Result<serde_json::Value, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims.tenant_id.ok_or_else(|| "No tenant ID in token".to_string())?;

    pppoe
        .reconcile_router(&claims.sub, &tenant_id, &router_id, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn preview_pppoe_import_from_router(
    token: String,
    router_id: String,
    include_disabled: Option<bool>,
    auth: State<'_, AuthService>,
    pppoe: State<'_, PppoeService>,
) -> Result<Vec<PppoeImportCandidate>, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims.tenant_id.ok_or_else(|| "No tenant ID in token".to_string())?;

    pppoe
        .preview_import_from_router(&claims.sub, &tenant_id, &router_id, include_disabled.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_pppoe_from_router(
    token: String,
    router_id: String,
    usernames: Vec<String>,
    customer_id: Option<String>,
    location_id: Option<String>,
    auth: State<'_, AuthService>,
    pppoe: State<'_, PppoeService>,
) -> Result<PppoeImportResult, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims.tenant_id.ok_or_else(|| "No tenant ID in token".to_string())?;

    let req = PppoeImportFromRouterRequest {
        usernames,
        customer_id,
        location_id,
    };

    pppoe
        .import_from_router(&claims.sub, &tenant_id, &router_id, req, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}
