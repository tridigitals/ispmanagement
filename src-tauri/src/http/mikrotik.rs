use crate::error::{AppError, AppResult};
use crate::http::AppState;
use crate::models::{
    CreateMikrotikRouterRequest, MikrotikInterfaceMetric, MikrotikRouter, MikrotikRouterMetric,
    MikrotikTestResult, UpdateMikrotikRouterRequest,
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/routers", get(list_routers).post(create_router))
        .route(
            "/routers/{id}",
            get(get_router).put(update_router).delete(delete_router),
        )
        .route("/routers/{id}/test", post(test_router))
        .route("/routers/{id}/metrics", get(list_metrics))
        .route("/routers/{id}/interfaces/metrics", get(list_interface_metrics))
        .route("/routers/{id}/interfaces/latest", get(list_interface_latest))
        .route("/routers/{id}/snapshot", get(get_snapshot))
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

// GET /api/admin/mikrotik/routers/{id}
async fn get_router(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<MikrotikRouter>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let router = state
        .mikrotik_service
        .get_router(&tenant_id, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Router not found".to_string()))?;

    Ok(Json(router))
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

    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "create",
            "mikrotik_router",
            Some(&router.id),
            Some(&format!("Created router '{}' ({})", router.name, router.host)),
            None,
        )
        .await;
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

    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "update",
            "mikrotik_router",
            Some(&router.id),
            Some(&format!("Updated router '{}' ({})", router.name, router.host)),
            None,
        )
        .await;
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

    let existing = state.mikrotik_service.get_router(&tenant_id, &id).await?;
    state.mikrotik_service.delete_router(&tenant_id, &id).await?;

    let details = existing
        .as_ref()
        .map(|r| format!("Deleted router '{}' ({})", r.name, r.host));
    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "delete",
            "mikrotik_router",
            Some(&id),
            details.as_deref(),
            None,
        )
        .await;
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

    let details = if res.ok {
        format!(
            "Tested router connection: ok identity={:?} version={:?} latency_ms={:?}",
            res.identity, res.ros_version, res.latency_ms
        )
    } else {
        format!("Tested router connection: failed error={:?}", res.error)
    };
    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "test_connection",
            "mikrotik_router",
            Some(&id),
            Some(&details),
            None,
        )
        .await;
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

#[derive(Deserialize)]
pub struct InterfaceMetricsQuery {
    pub interface: Option<String>,
    pub limit: Option<u32>,
}

// GET /api/admin/mikrotik/routers/{id}/interfaces/metrics?interface=ether1&limit=120
async fn list_interface_metrics(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<InterfaceMetricsQuery>,
) -> AppResult<Json<Vec<MikrotikInterfaceMetric>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let rows = state
        .mikrotik_service
        .list_interface_metrics(&tenant_id, &id, q.interface.as_deref(), q.limit.unwrap_or(120))
        .await?;
    Ok(Json(rows))
}

// GET /api/admin/mikrotik/routers/{id}/interfaces/latest
async fn list_interface_latest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<MikrotikInterfaceMetric>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let rows = state
        .mikrotik_service
        .list_latest_interface_metrics(&tenant_id, &id)
        .await?;
    Ok(Json(rows))
}

// GET /api/admin/mikrotik/routers/{id}/snapshot
async fn get_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<crate::models::MikrotikRouterSnapshot>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let snap = state.mikrotik_service.get_snapshot(&tenant_id, &id).await?;
    Ok(Json(snap))
}
