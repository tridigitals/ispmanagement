use crate::models::{
    CreateIspPackageRequest, IspPackage, IspPackageRouterMapping, IspPackageRouterMappingView,
    PaginatedResponse, UpdateIspPackageRequest, UpsertIspPackageRouterMappingRequest,
};
use crate::services::{AuthService, IspPackageService};
use tauri::State;

async fn require_isp_package_permission(
    auth: &AuthService,
    claims: &crate::services::auth_service::Claims,
    tenant_id: &str,
    action: &str,
) -> Result<(), String> {
    auth.check_permission(&claims.sub, tenant_id, "isp_packages", action)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_isp_packages(
    token: String,
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    auth: State<'_, AuthService>,
    svc: State<'_, IspPackageService>,
) -> Result<PaginatedResponse<IspPackage>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    require_isp_package_permission(&auth, &claims, &tenant_id, "read").await?;

    svc.list_packages(
        &claims.sub,
        &tenant_id,
        q,
        page.unwrap_or(1),
        per_page.unwrap_or(25),
        sort_by,
        sort_dir,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_isp_package(
    token: String,
    service_type: Option<String>,
    name: String,
    description: Option<String>,
    features: Option<Vec<String>>,
    is_active: Option<bool>,
    price_monthly: Option<f64>,
    price_yearly: Option<f64>,
    auth: State<'_, AuthService>,
    svc: State<'_, IspPackageService>,
) -> Result<IspPackage, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    require_isp_package_permission(&auth, &claims, &tenant_id, "manage").await?;

    let dto = CreateIspPackageRequest {
        service_type,
        name,
        description,
        features,
        is_active,
        price_monthly,
        price_yearly,
    };
    svc.create_package(&claims.sub, &tenant_id, dto, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_isp_package(
    token: String,
    id: String,
    service_type: Option<String>,
    name: Option<String>,
    description: Option<String>,
    features: Option<Vec<String>>,
    is_active: Option<bool>,
    price_monthly: Option<f64>,
    price_yearly: Option<f64>,
    auth: State<'_, AuthService>,
    svc: State<'_, IspPackageService>,
) -> Result<IspPackage, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    require_isp_package_permission(&auth, &claims, &tenant_id, "manage").await?;

    let dto = UpdateIspPackageRequest {
        service_type,
        name,
        description,
        features,
        is_active,
        price_monthly,
        price_yearly,
    };

    svc.update_package(&claims.sub, &tenant_id, &id, dto, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_isp_package(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    svc: State<'_, IspPackageService>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    require_isp_package_permission(&auth, &claims, &tenant_id, "manage").await?;

    svc.delete_package(&claims.sub, &tenant_id, &id, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_isp_package_router_mappings(
    token: String,
    router_id: Option<String>,
    auth: State<'_, AuthService>,
    svc: State<'_, IspPackageService>,
) -> Result<Vec<IspPackageRouterMappingView>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    require_isp_package_permission(&auth, &claims, &tenant_id, "read").await?;

    svc.list_router_mappings(&claims.sub, &tenant_id, router_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_isp_package_router_mapping(
    token: String,
    router_id: String,
    package_id: String,
    router_profile_name: String,
    address_pool: Option<String>,
    auth: State<'_, AuthService>,
    svc: State<'_, IspPackageService>,
) -> Result<IspPackageRouterMapping, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    require_isp_package_permission(&auth, &claims, &tenant_id, "manage").await?;

    let dto = UpsertIspPackageRouterMappingRequest {
        router_id,
        package_id,
        router_profile_name,
        address_pool,
    };

    svc.upsert_router_mapping(&claims.sub, &tenant_id, dto, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}
