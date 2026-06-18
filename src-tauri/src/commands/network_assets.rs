use crate::models::{
    CreateNetworkAssetRequest, ListNetworkAssetsParams, NetworkAsset, NetworkAssetListItem,
    PaginatedResponse, UpdateNetworkAssetRequest,
};
use crate::services::{AuthService, NetworkAssetService};
use tauri::State;

async fn tenant_context(auth: &AuthService, token: &str) -> Result<(String, String), String> {
    let claims = auth
        .validate_token(token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    Ok((claims.sub, tenant_id))
}

#[tauri::command]
pub async fn list_network_assets(
    token: String,
    q: Option<String>,
    asset_type: Option<String>,
    status: Option<String>,
    customer_id: Option<String>,
    location_id: Option<String>,
    parent_asset_id: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    auth: State<'_, AuthService>,
    network_assets: State<'_, NetworkAssetService>,
) -> Result<PaginatedResponse<NetworkAssetListItem>, String> {
    let (actor_id, tenant_id) = tenant_context(&auth, &token).await?;
    network_assets
        .list_assets(
            &actor_id,
            &tenant_id,
            ListNetworkAssetsParams {
                q,
                asset_type,
                status,
                customer_id,
                location_id,
                parent_asset_id,
                page,
                per_page,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_network_asset(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    network_assets: State<'_, NetworkAssetService>,
) -> Result<NetworkAsset, String> {
    let (actor_id, tenant_id) = tenant_context(&auth, &token).await?;
    network_assets
        .get_asset(&actor_id, &tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_network_asset(
    token: String,
    asset_type: String,
    name: String,
    code: Option<String>,
    vendor: Option<String>,
    model: Option<String>,
    serial_number: Option<String>,
    status: Option<String>,
    customer_id: Option<String>,
    location_id: Option<String>,
    work_order_id: Option<String>,
    parent_asset_id: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    notes: Option<String>,
    metadata: Option<serde_json::Value>,
    auth: State<'_, AuthService>,
    network_assets: State<'_, NetworkAssetService>,
) -> Result<NetworkAsset, String> {
    let (actor_id, tenant_id) = tenant_context(&auth, &token).await?;
    network_assets
        .create_asset(
            &actor_id,
            &tenant_id,
            CreateNetworkAssetRequest {
                asset_type,
                name,
                code,
                vendor,
                model,
                serial_number,
                status,
                customer_id,
                location_id,
                work_order_id,
                parent_asset_id,
                olt_id: None,
                pon_port: None,
                latitude,
                longitude,
                notes,
                metadata,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_network_asset(
    token: String,
    id: String,
    asset_type: Option<String>,
    name: Option<String>,
    code: Option<String>,
    vendor: Option<String>,
    model: Option<String>,
    serial_number: Option<String>,
    status: Option<String>,
    customer_id: Option<String>,
    location_id: Option<String>,
    work_order_id: Option<String>,
    parent_asset_id: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    notes: Option<String>,
    metadata: Option<serde_json::Value>,
    auth: State<'_, AuthService>,
    network_assets: State<'_, NetworkAssetService>,
) -> Result<NetworkAsset, String> {
    let (actor_id, tenant_id) = tenant_context(&auth, &token).await?;
    network_assets
        .update_asset(
            &actor_id,
            &tenant_id,
            &id,
            UpdateNetworkAssetRequest {
                asset_type,
                name,
                code,
                vendor,
                model,
                serial_number,
                status,
                customer_id,
                location_id,
                work_order_id,
                parent_asset_id,
                olt_id: None,
                pon_port: None,
                latitude,
                longitude,
                notes,
                metadata,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_network_asset(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    network_assets: State<'_, NetworkAssetService>,
) -> Result<(), String> {
    let (actor_id, tenant_id) = tenant_context(&auth, &token).await?;
    network_assets
        .delete_asset(&actor_id, &tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn assign_network_asset_customer(
    token: String,
    id: String,
    customer_id: Option<String>,
    auth: State<'_, AuthService>,
    network_assets: State<'_, NetworkAssetService>,
) -> Result<NetworkAsset, String> {
    let (actor_id, tenant_id) = tenant_context(&auth, &token).await?;
    network_assets
        .assign_customer(&actor_id, &tenant_id, &id, customer_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn assign_network_asset_location(
    token: String,
    id: String,
    location_id: Option<String>,
    auth: State<'_, AuthService>,
    network_assets: State<'_, NetworkAssetService>,
) -> Result<NetworkAsset, String> {
    let (actor_id, tenant_id) = tenant_context(&auth, &token).await?;
    network_assets
        .assign_location(&actor_id, &tenant_id, &id, location_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn assign_network_asset_work_order(
    token: String,
    id: String,
    work_order_id: Option<String>,
    auth: State<'_, AuthService>,
    network_assets: State<'_, NetworkAssetService>,
) -> Result<NetworkAsset, String> {
    let (actor_id, tenant_id) = tenant_context(&auth, &token).await?;
    network_assets
        .assign_work_order(&actor_id, &tenant_id, &id, work_order_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn link_network_asset_parent(
    token: String,
    id: String,
    parent_asset_id: Option<String>,
    auth: State<'_, AuthService>,
    network_assets: State<'_, NetworkAssetService>,
) -> Result<NetworkAsset, String> {
    let (actor_id, tenant_id) = tenant_context(&auth, &token).await?;
    network_assets
        .link_parent_asset(&actor_id, &tenant_id, &id, parent_asset_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_customer_network_assets(
    token: String,
    customer_id: String,
    auth: State<'_, AuthService>,
    network_assets: State<'_, NetworkAssetService>,
) -> Result<Vec<NetworkAssetListItem>, String> {
    let (actor_id, tenant_id) = tenant_context(&auth, &token).await?;
    network_assets
        .list_customer_assets(&actor_id, &tenant_id, &customer_id)
        .await
        .map_err(|e| e.to_string())
}
