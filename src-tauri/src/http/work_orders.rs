use crate::error::{AppError, AppResult};
use crate::http::auth::extract_ip;
use crate::http::AppState;
use crate::models::{
    AssignInstallationWorkOrderRequest, InstallationWorkOrder, InstallationWorkOrderView,
    UpdateInstallationWorkOrderStatusRequest,
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
        .route("/", get(list_work_orders))
        .route("/{id}/assign", post(assign_work_order))
        .route("/{id}/start", post(start_work_order))
        .route("/{id}/complete", post(complete_work_order))
        .route("/{id}/cancel", post(cancel_work_order))
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
struct ListWorkOrderQuery {
    status: Option<String>,
    assigned_to: Option<String>,
    include_closed: Option<bool>,
    limit: Option<u32>,
}

async fn list_work_orders(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListWorkOrderQuery>,
) -> AppResult<Json<Vec<InstallationWorkOrderView>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let rows = state
        .customer_service
        .list_installation_work_orders(
            &claims.sub,
            &tenant_id,
            q.status,
            q.assigned_to,
            q.include_closed.unwrap_or(false),
            q.limit.unwrap_or(200),
        )
        .await?;
    Ok(Json(rows))
}

async fn assign_work_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(dto): Json<AssignInstallationWorkOrderRequest>,
) -> AppResult<Json<InstallationWorkOrder>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .assign_installation_work_order(
            &claims.sub,
            &tenant_id,
            &id,
            &dto.assigned_to,
            dto.scheduled_at,
            dto.notes,
            Some(&ip),
        )
        .await?;
    Ok(Json(row))
}

async fn start_work_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateInstallationWorkOrderStatusRequest>,
) -> AppResult<Json<InstallationWorkOrder>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .start_installation_work_order(&claims.sub, &tenant_id, &id, dto.notes, Some(&ip))
        .await?;
    Ok(Json(row))
}

async fn complete_work_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateInstallationWorkOrderStatusRequest>,
) -> AppResult<Json<InstallationWorkOrder>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .complete_installation_work_order(&claims.sub, &tenant_id, &id, dto.notes, Some(&ip))
        .await?;
    Ok(Json(row))
}

async fn cancel_work_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateInstallationWorkOrderStatusRequest>,
) -> AppResult<Json<InstallationWorkOrder>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .cancel_installation_work_order(&claims.sub, &tenant_id, &id, dto.notes, Some(&ip))
        .await?;
    Ok(Json(row))
}
