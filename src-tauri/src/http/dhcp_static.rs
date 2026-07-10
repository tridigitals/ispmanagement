use crate::error::{AppError, AppResult};
use crate::http::auth::extract_ip;
use crate::http::AppState;
use crate::models::{
    CreateDhcpStaticServiceRequest, DhcpStaticServicePublic, PaginatedResponse,
    UpdateDhcpStaticServiceRequest,
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
        .route("/services", get(list_services).post(create_service))
        .route(
            "/services/{id}",
            get(get_service).put(update_service).delete(delete_service),
        )
        .route("/services/{id}/apply", post(apply_service))
        .route("/routers/{router_id}/reconcile", post(reconcile_router))
}

fn bearer_token(headers: &HeaderMap) -> AppResult<String> {
    crate::http::extract_token(headers)
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
    dhcp_server_name: Option<String>,
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn list_services(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<PaginatedResponse<DhcpStaticServicePublic>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let rows = state
        .dhcp_static_service
        .list_services(
            &claims.sub,
            &tenant_id,
            q.customer_id,
            q.location_id,
            q.router_id,
            q.dhcp_server_name,
            q.q,
            q.page.unwrap_or(1),
            q.per_page.unwrap_or(25),
        )
        .await?;
    Ok(Json(rows))
}

async fn get_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<DhcpStaticServicePublic>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let row = state
        .dhcp_static_service
        .get_service(&claims.sub, &tenant_id, &id)
        .await?;
    Ok(Json(row))
}

async fn create_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<CreateDhcpStaticServiceRequest>,
) -> AppResult<Json<DhcpStaticServicePublic>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .dhcp_static_service
        .create_service(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

async fn update_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateDhcpStaticServiceRequest>,
) -> AppResult<Json<DhcpStaticServicePublic>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .dhcp_static_service
        .update_service(&claims.sub, &tenant_id, &id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

async fn delete_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    state
        .dhcp_static_service
        .delete_service(&claims.sub, &tenant_id, &id, Some(&ip))
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
struct ApplyRequest {
    work_order_id: Option<String>,
}

async fn apply_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(body): Json<ApplyRequest>,
) -> AppResult<Json<DhcpStaticServicePublic>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .dhcp_static_service
        .apply_service(
            &claims.sub,
            &tenant_id,
            &id,
            body.work_order_id.as_deref(),
            Some(&ip),
        )
        .await?;
    Ok(Json(row))
}

async fn reconcile_router(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(router_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .dhcp_static_service
        .reconcile_router(&claims.sub, &tenant_id, &router_id, Some(&ip))
        .await?;
    Ok(Json(row))
}
