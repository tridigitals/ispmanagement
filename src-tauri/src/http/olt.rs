//! OLT (Optical Line Terminal) HTTP handlers — tenant-scoped CRUD, monitoring, and control
//!
//! All endpoints require Bearer token + RBAC permission check.
//! Mounted at `/api/admin/olts`.

use crate::error::{AppError, AppResult};
use crate::http::AppState;
use crate::models::olt::{
    CreateOltRequest, OltTestConnectionRequest, RebootOnuRequest, UpdateOltRequest,
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;

// ── Helpers ──────────────────────────────────────────────────

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
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(AppError::Unauthorized)?;
    Ok((tenant_id, claims))
}

// ── Query Params ─────────────────────────────────────────────

#[derive(Deserialize)]
struct OltStatsQuery {
    #[serde(default)]
    force_refresh: bool,
}

#[derive(Deserialize)]
struct OnuHistoryQuery {
    #[serde(default = "default_history_limit")]
    limit: i64,
}

fn default_history_limit() -> i64 {
    200
}

// ── CRUD Handlers ────────────────────────────────────────────

/// GET /api/admin/olts
async fn list_olts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt", "read")
        .await?;

    let olts = state.olt_service.list_olts(&tenant).await?;
    Ok(Json(serde_json::json!({ "status": "success", "data": olts })))
}

/// GET /api/admin/olts/{id}
async fn get_olt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt", "read")
        .await?;

    let olt = state.olt_service.get_olt(&id, &tenant).await?;
    Ok(Json(serde_json::json!({ "status": "success", "data": olt })))
}

/// POST /api/admin/olts
async fn create_olt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateOltRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt", "manage")
        .await?;

    let olt = state.olt_service.create_olt(&claims.sub, &tenant, payload).await?;
    Ok(Json(serde_json::json!({ "status": "success", "data": olt })))
}

/// PUT /api/admin/olts/{id}
async fn update_olt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateOltRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt", "manage")
        .await?;

    let olt = state.olt_service.update_olt(&id, &tenant, payload).await?;
    Ok(Json(serde_json::json!({ "status": "success", "data": olt })))
}

/// DELETE /api/admin/olts/{id}
async fn delete_olt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt", "manage")
        .await?;

    state.olt_service.delete_olt(&id, &tenant).await?;
    Ok(Json(serde_json::json!({ "status": "success" })))
}

// ── Monitoring Handlers ──────────────────────────────────────

/// GET /api/admin/olts/{id}/stats?force_refresh=true
async fn get_olt_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<OltStatsQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt", "read")
        .await?;

    let stats = state
        .olt_service
        .get_olt_stats(&id, &tenant, query.force_refresh)
        .await?;
    Ok(Json(
        serde_json::to_value(stats)
            .map_err(|e| AppError::Internal(e.to_string()))?,
    ))
}

/// GET /api/admin/olts/{id}/details
async fn get_olt_all_details(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt", "read")
        .await?;

    let details = state.olt_service.get_olt_all_details(&id, &tenant).await?;
    Ok(Json(
        serde_json::to_value(details)
            .map_err(|e| AppError::Internal(e.to_string()))?,
    ))
}

/// GET /api/admin/olts/all-onus
async fn get_all_onus(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt", "read")
        .await?;

    let onus = state.olt_service.get_all_onus(&tenant).await?;
    Ok(Json(
        serde_json::to_value(onus)
            .map_err(|e| AppError::Internal(e.to_string()))?,
    ))
}

// ── Control Handlers ─────────────────────────────────────────

/// POST /api/admin/olts/{id}/reboot-onu
async fn reboot_onu(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<RebootOnuRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt", "manage")
        .await?;

    let result = state.olt_service.reboot_onu(&id, &tenant, payload).await?;
    Ok(Json(result))
}

/// POST /api/admin/olts/test
async fn test_connection(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OltTestConnectionRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt", "manage")
        .await?;

    let result = state
        .olt_service
        .test_connection(
            &tenant,
            payload.id.as_deref(),
            &payload.host,
            payload.port,
            &payload.username,
            &payload.password,
            &payload.olt_type,
        )
        .await?;
    Ok(Json(
        serde_json::to_value(result)
            .map_err(|e| AppError::Internal(e.to_string()))?,
    ))
}

// ── History Handlers ─────────────────────────────────────────

/// GET /api/admin/olts/{id}/onu-history?limit=200
async fn get_onu_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(olt_id): Path<String>,
    Query(query): Query<OnuHistoryQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt_onu_history", "read")
        .await?;

    let history = state
        .olt_service
        .get_onu_history(&olt_id, &tenant, query.limit)
        .await?;
    Ok(Json(serde_json::json!({ "status": "success", "data": history })))
}

// ── Token Management Handlers ────────────────────────────────

/// GET /api/admin/olts/{id}/public-tokens
async fn list_public_tokens(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt", "manage")
        .await?;

    let tokens = state.olt_service.list_public_tokens(&id, &tenant).await?;
    Ok(Json(serde_json::json!({ "status": "success", "data": tokens })))
}

/// POST /api/admin/olts/{id}/public-tokens
async fn create_public_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<crate::models::CreatePublicTokenRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt", "manage")
        .await?;

    let token = state.olt_service.create_public_token(&id, &tenant, payload).await?;
    Ok(Json(serde_json::json!({ "status": "success", "data": token })))
}

/// DELETE /api/admin/olts/{id}/public-tokens/{token_id}
async fn delete_public_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, token_id)): Path<(String, String)>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant, "olt", "manage")
        .await?;

    state.olt_service.delete_public_token(&token_id, &tenant).await?;
    Ok(Json(serde_json::json!({ "status": "success" })))
}

// ── Router ───────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_olts).post(create_olt))
        .route("/all-onus", get(get_all_onus))
        .route("/test", post(test_connection))
        .route(
            "/{id}",
            get(get_olt).put(update_olt).delete(delete_olt),
        )
        .route("/{id}/stats", get(get_olt_stats))
        .route("/{id}/details", get(get_olt_all_details))
        .route("/{id}/reboot-onu", post(reboot_onu))
        .route("/{id}/onu-history", get(get_onu_history))
        .route(
            "/{id}/public-tokens",
            get(list_public_tokens).post(create_public_token),
        )
        .route(
            "/{id}/public-tokens/{token_id}",
            delete(delete_public_token),
        )
}
