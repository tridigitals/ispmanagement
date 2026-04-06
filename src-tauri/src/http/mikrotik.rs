use crate::error::{AppError, AppResult};
use crate::http::AppState;
use crate::models::{
    CreateMikrotikIpPoolRequest, CreateMikrotikPppProfileRequest, CreateMikrotikRouterRequest,
    ManagedRadiusRouterSetup, MikrotikAlert, MikrotikIncident, MikrotikInterfaceCounter,
    MikrotikInterfaceMetric, MikrotikIpPool, MikrotikIpPoolDeleteResult,
    MikrotikIpPoolDependencyStatus, MikrotikLogClearResult, MikrotikLogEntry,
    MikrotikLogRetentionSettings, MikrotikLogSyncResult, MikrotikPppProfile,
    MikrotikPppProfileDeleteResult, MikrotikPppProfileDependencyStatus, MikrotikRouter,
    MikrotikRouterMetric, MikrotikTestResult, PaginatedResponse, SimulateMikrotikIncidentRequest,
    UpdateMikrotikIncidentRequest, UpdateMikrotikIpPoolRequest, UpdateMikrotikPppProfileRequest,
    UpdateMikrotikRouterRequest,
};
use crate::services::mikrotik_service::{
    MIKROTIK_LOGS_DEFAULT_INCLUDE_TOTAL, MIKROTIK_LOGS_DEFAULT_PAGE, MIKROTIK_LOGS_DEFAULT_PER_PAGE,
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/noc", get(get_noc))
        .route("/alerts", get(list_alerts))
        .route("/alerts/{id}/ack", post(ack_alert))
        .route("/alerts/{id}/resolve", post(resolve_alert))
        .route("/incidents", get(list_incidents))
        .route("/incidents/simulate", post(simulate_incident))
        .route(
            "/incidents/escalate-now",
            post(run_incident_auto_escalation),
        )
        .route("/incidents/{id}", put(update_incident))
        .route("/incidents/{id}/ack", post(ack_incident))
        .route("/incidents/{id}/resolve", post(resolve_incident))
        .route("/logs", get(list_logs))
        .route("/routers", get(list_routers).post(create_router))
        .route(
            "/routers/{id}",
            get(get_router).put(update_router).delete(delete_router),
        )
        .route("/routers/{id}/logs/sync", post(sync_logs))
        .route("/routers/{id}/logs", delete(clear_logs))
        .route(
            "/routers/{id}/logs/retention",
            get(get_log_retention).put(update_log_retention),
        )
        .route("/routers/{id}/ppp-profiles", get(list_ppp_profiles).post(create_ppp_profile))
        .route(
            "/routers/{id}/ppp-profiles/{profile_id}",
            put(update_ppp_profile).delete(delete_ppp_profile),
        )
        .route(
            "/routers/{id}/ppp-profiles/{profile_id}/dependencies",
            get(get_ppp_profile_dependencies),
        )
        .route("/routers/{id}/ppp-profiles/sync", post(sync_ppp_profiles))
        .route("/routers/{id}/ip-pools", get(list_ip_pools).post(create_ip_pool))
        .route(
            "/routers/{id}/ip-pools/{pool_id}",
            put(update_ip_pool).delete(delete_ip_pool),
        )
        .route(
            "/routers/{id}/ip-pools/{pool_id}/dependencies",
            get(get_ip_pool_dependencies),
        )
        .route("/routers/{id}/ip-pools/sync", post(sync_ip_pools))
        .route(
            "/routers/{id}/managed-radius-setup",
            get(get_managed_radius_setup),
        )
        .route(
            "/routers/{id}/managed-radius-setup/assign-default",
            post(assign_managed_radius_default),
        )
        .route(
            "/routers/{id}/managed-radius-setup/create-mapping",
            post(create_managed_radius_mapping),
        )
        .route("/routers/{id}/test", post(test_router))
        .route("/routers/{id}/metrics", get(list_metrics))
        .route(
            "/routers/{id}/interfaces/metrics",
            get(list_interface_metrics),
        )
        .route(
            "/routers/{id}/interfaces/latest",
            get(list_interface_latest),
        )
        .route("/routers/{id}/interfaces/live", get(get_interface_live))
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
struct AlertsQuery {
    active_only: Option<bool>,
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct LogsQuery {
    router_id: Option<String>,
    level: Option<String>,
    topic: Option<String>,
    q: Option<String>,
    month: Option<u32>,
    year: Option<i32>,
    page: Option<u32>,
    per_page: Option<u32>,
    include_total: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SyncLogsBody {
    fetch_limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct UpdateRetentionBody {
    retention_days: Option<i64>,
}

// GET /api/admin/mikrotik/alerts?active_only=true&limit=200
async fn list_alerts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<AlertsQuery>,
) -> AppResult<Json<Vec<MikrotikAlert>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let rows = state
        .mikrotik_service
        .list_alerts(
            &tenant_id,
            q.active_only.unwrap_or(true),
            q.limit.unwrap_or(200),
        )
        .await?;
    Ok(Json(rows))
}

// POST /api/admin/mikrotik/alerts/{id}/ack
async fn ack_alert(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    state
        .mikrotik_service
        .ack_alert(&tenant_id, &id, &claims.sub)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

// POST /api/admin/mikrotik/alerts/{id}/resolve
async fn resolve_alert(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    state
        .mikrotik_service
        .resolve_alert_by_id(&tenant_id, &id, &claims.sub)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

// GET /api/admin/mikrotik/incidents?active_only=true&limit=200
async fn list_incidents(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<AlertsQuery>,
) -> AppResult<Json<Vec<MikrotikIncident>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let rows = state
        .mikrotik_service
        .list_incidents(
            &tenant_id,
            q.active_only.unwrap_or(true),
            q.limit.unwrap_or(200),
        )
        .await?;
    Ok(Json(rows))
}

// PUT /api/admin/mikrotik/incidents/{id}
async fn update_incident(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateMikrotikIncidentRequest>,
) -> AppResult<Json<MikrotikIncident>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let row = state
        .mikrotik_service
        .update_incident(&tenant_id, &id, req.owner_user_id, req.notes, &claims.sub)
        .await?;

    Ok(Json(row))
}

// POST /api/admin/mikrotik/incidents/simulate
async fn simulate_incident(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SimulateMikrotikIncidentRequest>,
) -> AppResult<Json<MikrotikIncident>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let row = state
        .mikrotik_service
        .simulate_incident(
            &tenant_id,
            &claims.sub,
            &req.router_id,
            &req.incident_type,
            req.severity,
            req.interface_name,
            req.message,
        )
        .await?;
    Ok(Json(row))
}

// POST /api/admin/mikrotik/incidents/escalate-now
async fn run_incident_auto_escalation(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let escalated = state
        .mikrotik_service
        .trigger_auto_escalation_now(&tenant_id, &claims.sub)
        .await?;
    Ok(Json(
        serde_json::json!({ "ok": true, "escalated": escalated }),
    ))
}

// POST /api/admin/mikrotik/incidents/{id}/ack
async fn ack_incident(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    state
        .mikrotik_service
        .ack_incident(&tenant_id, &id, &claims.sub)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

// POST /api/admin/mikrotik/incidents/{id}/resolve
async fn resolve_incident(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    state
        .mikrotik_service
        .resolve_incident_by_id(&tenant_id, &id, &claims.sub)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
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

// GET /api/admin/mikrotik/noc
async fn get_noc(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<crate::models::MikrotikRouterNocRow>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let rows = state.mikrotik_service.list_noc(&tenant_id).await?;
    Ok(Json(rows))
}

// GET /api/admin/mikrotik/logs
async fn list_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<LogsQuery>,
) -> AppResult<Json<PaginatedResponse<MikrotikLogEntry>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let rows = state
        .mikrotik_service
        .list_logs(
            &tenant_id,
            q.router_id,
            q.level,
            q.topic,
            q.q,
            q.month,
            q.year,
            q.page.unwrap_or(MIKROTIK_LOGS_DEFAULT_PAGE),
            q.per_page.unwrap_or(MIKROTIK_LOGS_DEFAULT_PER_PAGE),
            q.include_total
                .unwrap_or(MIKROTIK_LOGS_DEFAULT_INCLUDE_TOTAL),
        )
        .await?;
    Ok(Json(rows))
}

// POST /api/admin/mikrotik/routers/{id}/logs/sync
async fn sync_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Option<Json<SyncLogsBody>>,
) -> AppResult<Json<MikrotikLogSyncResult>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let fetch_limit = body
        .and_then(|b| b.fetch_limit)
        .unwrap_or(500)
        .clamp(50, 50_000);

    let rows = state
        .mikrotik_service
        .sync_logs_for_router(&tenant_id, &id, fetch_limit)
        .await?;
    Ok(Json(rows))
}

async fn clear_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<MikrotikLogClearResult>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let result = state
        .mikrotik_service
        .clear_logs_for_router(&tenant_id, &id)
        .await?;

    Ok(Json(result))
}

async fn get_log_retention(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<MikrotikLogRetentionSettings>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let result = state
        .mikrotik_service
        .get_router_log_retention(&tenant_id, &id)
        .await?;

    Ok(Json(result))
}

async fn update_log_retention(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Json<UpdateRetentionBody>,
) -> AppResult<Json<MikrotikLogRetentionSettings>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let result = state
        .mikrotik_service
        .update_router_log_retention_days(&tenant_id, &id, body.retention_days)
        .await?;

    Ok(Json(result))
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

// GET /api/admin/mikrotik/routers/{id}/ppp-profiles
async fn list_ppp_profiles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<MikrotikPppProfile>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let rows = state
        .mikrotik_service
        .list_ppp_profiles(&tenant_id, &id)
        .await?;
    Ok(Json(rows))
}

// POST /api/admin/mikrotik/routers/{id}/ppp-profiles
async fn create_ppp_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<CreateMikrotikPppProfileRequest>,
) -> AppResult<Json<MikrotikPppProfile>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let row = state
        .mikrotik_service
        .create_ppp_profile(&tenant_id, &id, payload)
        .await?;
    Ok(Json(row))
}

// PUT /api/admin/mikrotik/routers/{id}/ppp-profiles/{profile_id}
async fn update_ppp_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, profile_id)): Path<(String, String)>,
    Json(payload): Json<UpdateMikrotikPppProfileRequest>,
) -> AppResult<Json<MikrotikPppProfile>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let row = state
        .mikrotik_service
        .update_ppp_profile(&tenant_id, &id, &profile_id, payload)
        .await?;
    Ok(Json(row))
}

// DELETE /api/admin/mikrotik/routers/{id}/ppp-profiles/{profile_id}
async fn delete_ppp_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, profile_id)): Path<(String, String)>,
) -> AppResult<Json<MikrotikPppProfileDeleteResult>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let result = state
        .mikrotik_service
        .delete_ppp_profile(&tenant_id, &id, &profile_id)
        .await?;
    Ok(Json(result))
}

// GET /api/admin/mikrotik/routers/{id}/ppp-profiles/{profile_id}/dependencies
async fn get_ppp_profile_dependencies(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, profile_id)): Path<(String, String)>,
) -> AppResult<Json<MikrotikPppProfileDependencyStatus>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let result = state
        .mikrotik_service
        .get_ppp_profile_dependencies(&tenant_id, &id, &profile_id)
        .await?;
    Ok(Json(result))
}

// POST /api/admin/mikrotik/routers/{id}/ppp-profiles/sync
async fn sync_ppp_profiles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<MikrotikPppProfile>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let rows = state
        .mikrotik_service
        .sync_ppp_profiles(&tenant_id, &id)
        .await?;
    Ok(Json(rows))
}

// GET /api/admin/mikrotik/routers/{id}/ip-pools
async fn list_ip_pools(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<MikrotikIpPool>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let rows = state
        .mikrotik_service
        .list_ip_pools(&tenant_id, &id)
        .await?;
    Ok(Json(rows))
}

// POST /api/admin/mikrotik/routers/{id}/ip-pools
async fn create_ip_pool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<CreateMikrotikIpPoolRequest>,
) -> AppResult<Json<MikrotikIpPool>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let row = state
        .mikrotik_service
        .create_ip_pool(&tenant_id, &id, payload)
        .await?;
    Ok(Json(row))
}

// PUT /api/admin/mikrotik/routers/{id}/ip-pools/{pool_id}
async fn update_ip_pool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, pool_id)): Path<(String, String)>,
    Json(payload): Json<UpdateMikrotikIpPoolRequest>,
) -> AppResult<Json<MikrotikIpPool>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let row = state
        .mikrotik_service
        .update_ip_pool(&tenant_id, &id, &pool_id, payload)
        .await?;
    Ok(Json(row))
}

// DELETE /api/admin/mikrotik/routers/{id}/ip-pools/{pool_id}
async fn delete_ip_pool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, pool_id)): Path<(String, String)>,
) -> AppResult<Json<MikrotikIpPoolDeleteResult>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let result = state
        .mikrotik_service
        .delete_ip_pool(&tenant_id, &id, &pool_id)
        .await?;
    Ok(Json(result))
}

// GET /api/admin/mikrotik/routers/{id}/ip-pools/{pool_id}/dependencies
async fn get_ip_pool_dependencies(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, pool_id)): Path<(String, String)>,
) -> AppResult<Json<MikrotikIpPoolDependencyStatus>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let result = state
        .mikrotik_service
        .get_ip_pool_dependencies(&tenant_id, &id, &pool_id)
        .await?;
    Ok(Json(result))
}

// GET /api/admin/mikrotik/routers/{id}/managed-radius-setup
async fn get_managed_radius_setup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<ManagedRadiusRouterSetup>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let router = state
        .mikrotik_service
        .get_router(&tenant_id, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Router not found".into()))?;

    let plan_allows_managed_radius = state
        .plan_service
        .check_feature_access(&tenant_id, "managed_radius")
        .await
        .map(|access| access.has_access)
        .unwrap_or(false);

    let mut setup = state
        .managed_radius_service
        .get_router_setup(&tenant_id, &router, plan_allows_managed_radius)
        .await?;

    let can_reveal_secret = state
        .auth_service
        .check_permission(
            &claims.sub,
            &tenant_id,
            "network_routers",
            "manage_radius_secret",
        )
        .await
        .is_ok();

    if !can_reveal_secret {
        setup.shared_secret = None;
    }

    Ok(Json(setup))
}

async fn assign_managed_radius_default(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<ManagedRadiusRouterSetup>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let router = state
        .mikrotik_service
        .get_router(&tenant_id, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Router not found".into()))?;

    let plan_allows_managed_radius = state
        .plan_service
        .check_feature_access(&tenant_id, "managed_radius")
        .await
        .map(|access| access.has_access)
        .unwrap_or(false);

    if !plan_allows_managed_radius {
        return Err(AppError::Validation(
            "Managed RADIUS is not included in the current plan".into(),
        ));
    }

    if state
        .managed_radius_service
        .get_active_assignment_for_tenant_optional(&tenant_id)
        .await?
        .is_some()
    {
        return Err(AppError::Validation(
            "Managed RADIUS assignment is already active for this tenant".into(),
        ));
    }

    let assigned = state
        .managed_radius_service
        .auto_assign_default_server_for_tenant(&tenant_id)
        .await?;

    if assigned.is_none() {
        return Err(AppError::Configuration(
            "No default Managed RADIUS server is configured".into(),
        ));
    }

    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "MIKROTIK_ROUTER_MANAGED_RADIUS_ASSIGNED_DEFAULT",
            "mikrotik_router",
            Some(&router.id),
            Some(&format!(
                "Assigned tenant {} to the default Managed RADIUS server from router {}",
                tenant_id, router.name
            )),
            None,
        )
        .await;

    let mut setup = state
        .managed_radius_service
        .get_router_setup(&tenant_id, &router, true)
        .await?;

    let can_reveal_secret = state
        .auth_service
        .check_permission(
            &claims.sub,
            &tenant_id,
            "network_routers",
            "manage_radius_secret",
        )
        .await
        .is_ok();

    if !can_reveal_secret {
        setup.shared_secret = None;
    }

    Ok(Json(setup))
}

async fn create_managed_radius_mapping(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<ManagedRadiusRouterSetup>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let router = state
        .mikrotik_service
        .get_router(&tenant_id, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Router not found".into()))?;

    let plan_allows_managed_radius = state
        .plan_service
        .check_feature_access(&tenant_id, "managed_radius")
        .await
        .map(|access| access.has_access)
        .unwrap_or(false);

    if !plan_allows_managed_radius {
        return Err(AppError::Validation(
            "Managed RADIUS is not included in the current plan".into(),
        ));
    }

    state
        .managed_radius_service
        .auto_create_mapping_for_router(&tenant_id, &router)
        .await?;

    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "MIKROTIK_ROUTER_MANAGED_RADIUS_MAPPING_CREATED",
            "mikrotik_router",
            Some(&router.id),
            Some(&format!(
                "Created Managed RADIUS NAS mapping automatically for router {}",
                router.name
            )),
            None,
        )
        .await;

    let mut setup = state
        .managed_radius_service
        .get_router_setup(&tenant_id, &router, true)
        .await?;

    let can_reveal_secret = state
        .auth_service
        .check_permission(
            &claims.sub,
            &tenant_id,
            "network_routers",
            "manage_radius_secret",
        )
        .await
        .is_ok();

    if !can_reveal_secret {
        setup.shared_secret = None;
    }

    Ok(Json(setup))
}

// POST /api/admin/mikrotik/routers/{id}/ip-pools/sync
async fn sync_ip_pools(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<MikrotikIpPool>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await?;

    let rows = state
        .mikrotik_service
        .sync_ip_pools(&tenant_id, &id)
        .await?;
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

    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "create",
            "mikrotik_router",
            Some(&router.id),
            Some(&format!(
                "Created router '{}' ({})",
                router.name, router.host
            )),
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
            Some(&format!(
                "Updated router '{}' ({})",
                router.name, router.host
            )),
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
    state
        .mikrotik_service
        .delete_router(&tenant_id, &id)
        .await?;

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

    let res = state
        .mikrotik_service
        .test_connection(&tenant_id, &id)
        .await?;

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
        .list_interface_metrics(
            &tenant_id,
            &id,
            q.interface.as_deref(),
            q.limit.unwrap_or(120),
        )
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

#[derive(Debug, Deserialize)]
struct LiveQuery {
    names: String, // comma-separated
}

// GET /api/admin/mikrotik/routers/{id}/interfaces/live?names=ether1,ether2
async fn get_interface_live(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<LiveQuery>,
) -> AppResult<Json<Vec<MikrotikInterfaceCounter>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await?;

    let names: Vec<String> = q
        .names
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let rows = state
        .mikrotik_service
        .get_live_interface_counters(&tenant_id, &id, names)
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
