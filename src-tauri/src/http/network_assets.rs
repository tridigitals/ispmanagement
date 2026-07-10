use crate::error::{AppError, AppResult};
use crate::http::AppState;
use crate::models::{
    CreateNetworkAssetRequest, ListNetworkAssetsParams, NetworkAsset, NetworkAssetListItem,
    PaginatedResponse, UpdateNetworkAssetRequest,
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
        .route("/", get(list_assets).post(create_asset))
        .route(
            "/{id}",
            get(get_asset).patch(update_asset).delete(delete_asset),
        )
        .route("/{id}/customer", post(assign_customer))
        .route("/{id}/location", post(assign_location))
        .route("/{id}/work-order", post(assign_work_order))
        .route("/{id}/parent", post(link_parent_asset))
        .route("/customer/{customer_id}", get(list_customer_assets))
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
    q: Option<String>,
    asset_type: Option<String>,
    status: Option<String>,
    customer_id: Option<String>,
    location_id: Option<String>,
    parent_asset_id: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RelationRequest {
    customer_id: Option<String>,
    location_id: Option<String>,
    work_order_id: Option<String>,
    parent_asset_id: Option<String>,
}

async fn list_assets(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<PaginatedResponse<NetworkAssetListItem>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_asset_service
        .list_assets(
            &claims.sub,
            &tenant_id,
            ListNetworkAssetsParams {
                q: q.q,
                asset_type: q.asset_type,
                status: q.status,
                customer_id: q.customer_id,
                location_id: q.location_id,
                parent_asset_id: q.parent_asset_id,
                page: q.page,
                per_page: q.per_page,
            },
        )
        .await?;
    Ok(Json(out))
}

async fn get_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<NetworkAsset>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_asset_service
        .get_asset(&claims.sub, &tenant_id, &id)
        .await?;
    Ok(Json(out))
}

async fn create_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<CreateNetworkAssetRequest>,
) -> AppResult<Json<NetworkAsset>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_asset_service
        .create_asset(&claims.sub, &tenant_id, dto)
        .await?;
    Ok(Json(out))
}

async fn update_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(dto): Json<UpdateNetworkAssetRequest>,
) -> AppResult<Json<NetworkAsset>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_asset_service
        .update_asset(&claims.sub, &tenant_id, &id, dto)
        .await?;
    Ok(Json(out))
}

async fn delete_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .network_asset_service
        .delete_asset(&claims.sub, &tenant_id, &id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn assign_customer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<RelationRequest>,
) -> AppResult<Json<NetworkAsset>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_asset_service
        .assign_customer(&claims.sub, &tenant_id, &id, body.customer_id.as_deref())
        .await?;
    Ok(Json(out))
}

async fn assign_location(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<RelationRequest>,
) -> AppResult<Json<NetworkAsset>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_asset_service
        .assign_location(&claims.sub, &tenant_id, &id, body.location_id.as_deref())
        .await?;
    Ok(Json(out))
}

async fn assign_work_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<RelationRequest>,
) -> AppResult<Json<NetworkAsset>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_asset_service
        .assign_work_order(&claims.sub, &tenant_id, &id, body.work_order_id.as_deref())
        .await?;
    Ok(Json(out))
}

async fn link_parent_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<RelationRequest>,
) -> AppResult<Json<NetworkAsset>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_asset_service
        .link_parent_asset(
            &claims.sub,
            &tenant_id,
            &id,
            body.parent_asset_id.as_deref(),
        )
        .await?;
    Ok(Json(out))
}

async fn list_customer_assets(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(customer_id): Path<String>,
) -> AppResult<Json<Vec<NetworkAssetListItem>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_asset_service
        .list_customer_assets(&claims.sub, &tenant_id, &customer_id)
        .await?;
    Ok(Json(out))
}
