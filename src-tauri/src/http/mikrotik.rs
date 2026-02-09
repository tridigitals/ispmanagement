use crate::error::{AppError, AppResult};
use crate::http::AppState;
use crate::models::{
    CreateMikrotikRouterRequest, MikrotikRouter, MikrotikRouterMetric, MikrotikTestResult,
    UpdateMikrotikRouterRequest,
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/routers", get(list_routers).post(create_router))
        .route(
            "/routers/{id}",
            put(update_router).delete(delete_router),
        )
        .route("/routers/{id}/test", post(test_router))
        .route("/routers/{id}/metrics", get(list_metrics))
}

fn bearer_token(headers: &HeaderMap) -> AppResult<String> {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or(AppError::Unauthorized)
}

async fn tenant_and_claims(state: &AppState, headers: &HeaderMap) -> AppResult<(String, crate::services::auth_service::Claims)> {
    let token = bearer_token(headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims.tenant_id.clone().ok_or(AppError::Unauthorized)?;
    Ok((tenant_id, claims))
}

// GET /api/admin/mikrotik/routers
async fn list_routers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<MikrotikRouter>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let rows = state.mikrotik_service.list_routers(&tenant_id).await?;
    Ok(Json(rows))
}

// POST /api/admin/mikrotik/routers
async fn create_router(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateMikrotikRouterRequest>,
) -> AppResult<Json<MikrotikRouter>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let router = state
        .mikrotik_service
        .create_router(&tenant_id, payload)
        .await?;
    Ok(Json(router))
}

// PUT /api/admin/mikrotik/routers/{id}
async fn update_router(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateMikrotikRouterRequest>,
) -> AppResult<Json<MikrotikRouter>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let router = state
        .mikrotik_service
        .update_router(&tenant_id, &id, payload)
        .await?;
    Ok(Json(router))
}

// DELETE /api/admin/mikrotik/routers/{id}
async fn delete_router(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<()> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    state.mikrotik_service.delete_router(&tenant_id, &id).await?;
    Ok(())
}

// POST /api/admin/mikrotik/routers/{id}/test
async fn test_router(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<MikrotikTestResult>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let res = state.mikrotik_service.test_connection(&tenant_id, &id).await?;
    Ok(Json(res))
}

#[derive(Deserialize)]
pub struct MetricsQuery {
    pub limit: Option<u32>,
}

// GET /api/admin/mikrotik/routers/{id}/metrics?limit=120
async fn list_metrics(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<MetricsQuery>,
) -> AppResult<Json<Vec<MikrotikRouterMetric>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let rows = state
        .mikrotik_service
        .list_metrics(&tenant_id, &id, q.limit.unwrap_or(120))
        .await?;
    Ok(Json(rows))
}
