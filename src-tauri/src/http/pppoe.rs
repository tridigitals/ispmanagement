use crate::error::{AppError, AppResult};
use crate::http::auth::extract_ip;
use crate::http::AppState;
use crate::models::{
    CreatePppoeAccountRequest, PaginatedResponse, PppoeAccountPublic, PppoeImportCandidate,
    PppoeImportFromRouterRequest, PppoeImportResult, UpdatePppoeAccountRequest,
};
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::net::SocketAddr;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/accounts", get(list_accounts).post(create_account))
        .route(
            "/accounts/{id}",
            get(get_account).put(update_account).delete(delete_account),
        )
        .route("/accounts/{id}/apply", post(apply_account))
        .route("/routers/{router_id}/reconcile", post(reconcile_router))
        .route("/routers/{router_id}/import/preview", get(preview_import))
        .route("/routers/{router_id}/import", post(run_import))
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
struct ListQuery {
    customer_id: Option<String>,
    location_id: Option<String>,
    router_id: Option<String>,
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

// GET /api/admin/pppoe/accounts
async fn list_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<PaginatedResponse<PppoeAccountPublic>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let rows = state
        .pppoe_service
        .list_accounts(
            &claims.sub,
            &tenant_id,
            q.customer_id,
            q.location_id,
            q.router_id,
            q.q,
            q.page.unwrap_or(1),
            q.per_page.unwrap_or(25),
        )
        .await?;
    Ok(Json(rows))
}

// GET /api/admin/pppoe/accounts/{id}
async fn get_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<PppoeAccountPublic>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let row = state
        .pppoe_service
        .get_account(&claims.sub, &tenant_id, &id)
        .await?;
    Ok(Json(row))
}

// POST /api/admin/pppoe/accounts
async fn create_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<CreatePppoeAccountRequest>,
) -> AppResult<Json<PppoeAccountPublic>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .pppoe_service
        .create_account(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

// PUT /api/admin/pppoe/accounts/{id}
async fn update_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(dto): Json<UpdatePppoeAccountRequest>,
) -> AppResult<Json<PppoeAccountPublic>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .pppoe_service
        .update_account(&claims.sub, &tenant_id, &id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

// DELETE /api/admin/pppoe/accounts/{id}
async fn delete_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    state
        .pppoe_service
        .delete_account(&claims.sub, &tenant_id, &id, Some(&ip))
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// POST /api/admin/pppoe/accounts/{id}/apply
async fn apply_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
) -> AppResult<Json<PppoeAccountPublic>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .pppoe_service
        .apply_account(&claims.sub, &tenant_id, &id, Some(&ip))
        .await?;
    Ok(Json(row))
}

// POST /api/admin/pppoe/routers/{router_id}/reconcile
async fn reconcile_router(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(router_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .pppoe_service
        .reconcile_router(&claims.sub, &tenant_id, &router_id, Some(&ip))
        .await?;
    Ok(Json(row))
}

#[derive(Debug, Deserialize)]
struct PreviewQuery {
    include_disabled: Option<bool>,
}

// GET /api/admin/pppoe/routers/{router_id}/import/preview
async fn preview_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(router_id): Path<String>,
    Query(q): Query<PreviewQuery>,
) -> AppResult<Json<Vec<PppoeImportCandidate>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let rows = state
        .pppoe_service
        .preview_import_from_router(&claims.sub, &tenant_id, &router_id, q.include_disabled.unwrap_or(false))
        .await?;
    Ok(Json(rows))
}

// POST /api/admin/pppoe/routers/{router_id}/import
async fn run_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(router_id): Path<String>,
    Json(dto): Json<PppoeImportFromRouterRequest>,
) -> AppResult<Json<PppoeImportResult>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .pppoe_service
        .import_from_router(&claims.sub, &tenant_id, &router_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}
