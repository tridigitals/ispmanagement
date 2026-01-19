//! Plan Management HTTP Endpoints

use axum::{
    extract::{Path, State, Query},
    http::{StatusCode, HeaderMap},
    Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::http::AppState;
use crate::models::{
    Plan, PlanWithFeatures, FeatureDefinition, TenantSubscription, FeatureAccess,
    CreatePlanRequest, UpdatePlanRequest, CreateFeatureRequest, TenantSubscriptionDetails,
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

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

// Helper to extract and validate token from headers
async fn authenticate(state: &AppState, headers: &HeaderMap) -> Result<Claims, (StatusCode, Json<ErrorResponse>)> {
    let auth_header = headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            error: "Missing authorization header".to_string()
        })))?;

    state.auth_service.validate_token(auth_header).await
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            error: e.to_string()
        })))
}

fn require_superadmin(claims: &Claims) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if !claims.is_super_admin {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse {
            error: "Superadmin access required".to_string()
        })));
    }
    Ok(())
}

// ==================== PLANS ====================

async fn list_plans(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Plan>>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    
    let plans = if claims.is_super_admin {
        state.plan_service.list_plans().await
    } else {
        state.plan_service.list_active_plans().await
    };

    plans
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn get_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Option<PlanWithFeatures>>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state.plan_service.get_plan_with_features(&id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn create_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreatePlanRequest>,
) -> Result<Json<Plan>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state.plan_service.create_plan(req).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn update_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdatePlanRequest>,
) -> Result<Json<Plan>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state.plan_service.update_plan(&id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn delete_plan_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state.plan_service.delete_plan(&id).await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

// ==================== FEATURES ====================

async fn list_features(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<FeatureDefinition>>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state.plan_service.list_feature_definitions().await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn create_feature(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateFeatureRequest>,
) -> Result<Json<FeatureDefinition>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state.plan_service.create_feature(req).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn delete_feature(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state.plan_service.delete_feature(&id).await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

// ==================== PLAN FEATURES ====================

#[derive(Deserialize)]
struct SetPlanFeatureBody {
    feature_id: String,
    value: String,
}

async fn set_plan_feature(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(plan_id): Path<String>,
    Json(body): Json<SetPlanFeatureBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state.plan_service.set_plan_feature(&plan_id, &body.feature_id, &body.value).await
        .map(|_| StatusCode::OK)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

// ==================== SUBSCRIPTIONS ====================

#[derive(Deserialize)]
struct SubscriptionDetailsParams {
    tenant_id: Option<String>,
}

async fn get_subscription_details(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<SubscriptionDetailsParams>,
) -> Result<Json<TenantSubscriptionDetails>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    
    // Determine target tenant_id
    let target_tenant_id = match params.tenant_id {
        Some(ref tid) => {
             // If specifying a tenant, must be superadmin or own tenant
             if !claims.is_super_admin && claims.tenant_id.as_deref() != Some(tid) {
                 return Err((StatusCode::FORBIDDEN, Json(ErrorResponse {
                     error: "Unauthorized".to_string()
                 })));
             }
             tid.clone()
        },
        None => {
            // Default to own tenant
            claims.tenant_id.ok_or_else(|| (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                error: "Tenant ID required".to_string()
            })))?
        }
    };

    state.plan_service.get_tenant_subscription_details(&target_tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn get_subscription(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<String>,
) -> Result<Json<Option<TenantSubscription>>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    
    // Allow superadmin or own tenant
    if !claims.is_super_admin && claims.tenant_id.as_deref() != Some(tenant_id.as_str()) {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse {
            error: "Unauthorized".to_string()
        })));
    }

    state.plan_service.get_tenant_subscription(&tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

#[derive(Deserialize)]
struct AssignPlanBody {
    plan_id: String,
}

async fn assign_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<String>,
    Json(body): Json<AssignPlanBody>,
) -> Result<Json<TenantSubscription>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state.plan_service.assign_plan_to_tenant(&tenant_id, &body.plan_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

// ==================== FEATURE ACCESS ====================

async fn check_access(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, feature_code)): Path<(String, String)>,
) -> Result<Json<FeatureAccess>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    
    // Allow superadmin or own tenant
    if !claims.is_super_admin && claims.tenant_id.as_deref() != Some(tenant_id.as_str()) {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse {
            error: "Unauthorized".to_string()
        })));
    }

    state.plan_service.check_feature_access(&tenant_id, &feature_code).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}
