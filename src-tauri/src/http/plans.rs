//! Plan Management HTTP Endpoints

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::http::AppState;
use crate::models::{
    CreateFeatureRequest, CreatePlanRequest, FeatureAccess, FeatureDefinition, Plan,
    PlanWithFeatures, TenantSubscription, TenantSubscriptionDetails, UpdatePlanRequest,
};
use crate::services::Claims;

pub fn plan_routes() -> Router<AppState> {
    Router::new()
        // Plans
        .route("/", get(list_plans))
        .route("/", post(create_plan))
        .route("/{id}", get(get_plan))
        .route("/{id}", put(update_plan))
        .route("/{id}", delete(delete_plan_handler))
        // Features
        .route("/features", get(list_features))
        .route("/features", post(create_feature))
        .route("/features/{id}", delete(delete_feature))
        // Plan Features
        .route("/{plan_id}/features", post(set_plan_feature))
        // Subscriptions
        .route("/subscriptions/details", get(get_subscription_details))
        .route("/subscriptions/{tenant_id}", get(get_subscription))
        .route("/subscriptions/{tenant_id}/assign", post(assign_plan))
        // Feature access check
        .route("/access/{tenant_id}/{feature_code}", get(check_access))
}

// Helper to extract and validate token from headers
async fn authenticate(state: &AppState, headers: &HeaderMap) -> AppResult<Claims> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized)?;

    state
        .auth_service
        .validate_token(auth_header)
        .await
        .map_err(|_| AppError::Unauthorized)
}

fn require_superadmin(claims: &Claims) -> AppResult<()> {
    if !claims.is_super_admin {
        return Err(AppError::Forbidden("Superadmin access required".to_string()));
    }
    Ok(())
}

async fn require_plan_read_access(state: &AppState, claims: &Claims) -> AppResult<()> {
    if claims.is_super_admin {
        return Ok(());
    }
    let tenant_id = claims
        .tenant_id
        .as_deref()
        .ok_or_else(|| AppError::Forbidden("Tenant context required".to_string()))?;
    state
        .auth_service
        .check_permission(&claims.sub, tenant_id, "billing", "read")
        .await
        .map_err(|_| AppError::Forbidden("Billing read access required".to_string()))?;
    Ok(())
}

// ==================== PLANS ====================

async fn list_plans(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<Plan>>> {
    let claims = authenticate(&state, &headers).await?;
    require_plan_read_access(&state, &claims).await?;

    let plans = if claims.is_super_admin {
        state.plan_service.list_plans().await?
    } else {
        state.plan_service.list_active_plans().await?
    };

    Ok(Json(plans))
}

async fn get_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<Option<PlanWithFeatures>>> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    Ok(Json(
state
           .plan_service
           .get_plan_with_features(&id)
           .await?,
    ))
}

async fn create_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreatePlanRequest>,
) -> AppResult<Json<Plan>> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    // Service membalas AppResult: Validation -> 400, Conflict (slug duplikat)
    // -> 409, bukan 500 mentah.
    Ok(Json(state.plan_service.create_plan(req).await?))
}

async fn update_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdatePlanRequest>,
) -> AppResult<Json<Plan>> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    Ok(Json(state.plan_service.update_plan(&id, req).await?))
}

async fn delete_plan_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    // 404 jujur untuk id tak dikenal; 409 berisi daftar tenant pemakai
    // kalau paket masih dipakai (FK NO ACTION).
    state.plan_service.delete_plan(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ==================== FEATURES ====================

async fn list_features(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<FeatureDefinition>>> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    Ok(Json(
state
           .plan_service
           .list_feature_definitions()
           .await?,
    ))
}

async fn create_feature(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateFeatureRequest>,
) -> AppResult<Json<FeatureDefinition>> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    Ok(Json(
state
           .plan_service
           .create_feature(req)
           .await?,
    ))
}

async fn delete_feature(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state
           .plan_service
           .delete_feature(&id)
           .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ==================== PLAN FEATURES ====================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct SetPlanFeatureBody {
    #[serde(alias = "feature_id")]
    feature_id: String,
    value: String,
}

async fn set_plan_feature(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(plan_id): Path<String>,
    Json(body): Json<SetPlanFeatureBody>,
) -> AppResult<StatusCode> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state
           .plan_service
           .set_plan_feature(&plan_id, &body.feature_id, &body.value)
           .await?;
    Ok(StatusCode::OK)
}

// ==================== SUBSCRIPTIONS ====================

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SubscriptionDetailsParams {
    tenant_id: Option<String>,
}

async fn get_subscription_details(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<SubscriptionDetailsParams>,
) -> AppResult<Json<TenantSubscriptionDetails>> {
    let claims = authenticate(&state, &headers).await?;
    require_plan_read_access(&state, &claims).await?;

    // Determine target tenant_id
    let target_tenant_id = match params.tenant_id {
        Some(ref tid) => {
            // If specifying a tenant, must be superadmin or own tenant
            if !claims.is_super_admin && claims.tenant_id.as_deref() != Some(tid) {
                return Err(AppError::Forbidden("Unauthorized".to_string()));
            }
            tid.clone()
        }
        None => {
            // Default to own tenant
            claims
                .tenant_id
                .ok_or_else(|| AppError::Validation("Tenant ID required".to_string()))?
        }
    };

    Ok(Json(
        state
            .plan_service
            .get_tenant_subscription_details(&target_tenant_id)
            .await?,
    ))
}

async fn get_subscription(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<String>,
) -> AppResult<Json<Option<TenantSubscription>>> {
    let claims = authenticate(&state, &headers).await?;
    require_plan_read_access(&state, &claims).await?;

    // Allow superadmin or own tenant
    if !claims.is_super_admin && claims.tenant_id.as_deref() != Some(tenant_id.as_str()) {
        return Err(AppError::Forbidden("Unauthorized".to_string()));
    }

    Ok(Json(
state
           .plan_service
           .get_tenant_subscription(&tenant_id)
           .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct AssignPlanBody {
    #[serde(alias = "plan_id")]
    plan_id: String,
}

async fn assign_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<String>,
    Json(body): Json<AssignPlanBody>,
) -> AppResult<Json<TenantSubscription>> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    Ok(Json(
        state
            .plan_service
            .assign_plan_to_tenant(&tenant_id, &body.plan_id)
            .await?,
    ))
}

// ==================== FEATURE ACCESS ====================

async fn check_access(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, feature_code)): Path<(String, String)>,
) -> AppResult<Json<FeatureAccess>> {
    let claims = authenticate(&state, &headers).await?;
    require_plan_read_access(&state, &claims).await?;

    // Allow superadmin or own tenant
    if !claims.is_super_admin && claims.tenant_id.as_deref() != Some(tenant_id.as_str()) {
        return Err(AppError::Forbidden("Unauthorized".to_string()));
    }

    Ok(Json(
state
           .plan_service
           .check_feature_access(&tenant_id, &feature_code)
           .await?,
    ))
}
