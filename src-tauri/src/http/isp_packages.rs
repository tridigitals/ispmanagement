use crate::error::{AppError, AppResult};
use crate::http::AppState;
use crate::models::{
    CreateIspPackageRequest, IspPackage, IspPackageRouterMapping, IspPackageRouterMappingView,
    PaginatedResponse, UpdateIspPackageRequest, UpsertIspPackageRouterMappingRequest,
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/packages", get(list_packages).post(create_package))
        .route("/packages/{id}", put(update_package).delete(delete_package))
        .route(
            "/router-mappings",
            get(list_router_mappings).post(upsert_router_mapping),
        )
}

fn bearer_token(headers: &HeaderMap) -> AppResult<String> {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or(AppError::Unauthorized)
}

async fn tenant_and_claims(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<(String, crate::services::auth_service::Claims)> {
    let token = bearer_token(headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims.tenant_id.clone().ok_or(AppError::Unauthorized)?;
    Ok((tenant_id, claims))
}

#[derive(Debug, Deserialize)]
struct ListPackagesQuery {
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

// GET /api/admin/isp-packages/packages
async fn list_packages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListPackagesQuery>,
) -> AppResult<Json<PaginatedResponse<IspPackage>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .isp_package_service
        .list_packages(
            &claims.sub,
            &tenant_id,
            q.q,
            q.page.unwrap_or(1),
            q.per_page.unwrap_or(25),
        )
        .await?;
    Ok(Json(out))
}

// POST /api/admin/isp-packages/packages
async fn create_package(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<CreateIspPackageRequest>,
) -> AppResult<Json<IspPackage>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .isp_package_service
        .create_package(&claims.sub, &tenant_id, dto, None)
        .await?;
    Ok(Json(out))
}

// PUT /api/admin/isp-packages/packages/{id}
async fn update_package(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(dto): Json<UpdateIspPackageRequest>,
) -> AppResult<Json<IspPackage>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .isp_package_service
        .update_package(&claims.sub, &tenant_id, &id, dto, None)
        .await?;
    Ok(Json(out))
}

// DELETE /api/admin/isp-packages/packages/{id}
async fn delete_package(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .isp_package_service
        .delete_package(&claims.sub, &tenant_id, &id, None)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
struct ListMappingsQuery {
    router_id: Option<String>,
}

// GET /api/admin/isp-packages/router-mappings
async fn list_router_mappings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListMappingsQuery>,
) -> AppResult<Json<Vec<IspPackageRouterMappingView>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .isp_package_service
        .list_router_mappings(&claims.sub, &tenant_id, q.router_id)
        .await?;
    Ok(Json(out))
}

// POST /api/admin/isp-packages/router-mappings
async fn upsert_router_mapping(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<UpsertIspPackageRouterMappingRequest>,
) -> AppResult<Json<IspPackageRouterMapping>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .isp_package_service
        .upsert_router_mapping(&claims.sub, &tenant_id, dto, None)
        .await?;
    Ok(Json(out))
}
