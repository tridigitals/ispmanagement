use crate::error::{AppError, AppResult};
use crate::http::AppState;
use crate::models::{
    CreateNetworkLinkRequest, CreateNetworkNodeRequest, CreateServiceZoneRequest,
    CreateZoneNodeBindingRequest, CreateZoneOfferRequest, CoverageCheckRequest, ComputePathRequest,
    PaginatedResponse, ResolveZoneRequest, UpdateNetworkLinkRequest, UpdateNetworkNodeRequest,
    UpdateServiceZoneRequest, UpdateZoneOfferRequest,
};
use crate::services::network_mapping_service::ListQuery;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/nodes", get(list_nodes).post(create_node))
        .route("/nodes/{id}", patch(update_node).delete(delete_node))
        .route("/links", get(list_links).post(create_link))
        .route("/links/{id}", patch(update_link).delete(delete_link))
        .route("/zones", get(list_zones).post(create_zone))
        .route("/zones/{id}", patch(update_zone).delete(delete_zone))
        .route("/zones/resolve", post(resolve_zone))
        .route("/paths/compute", post(compute_path))
        .route("/coverage/check", post(check_coverage))
        .route("/zone-offers", get(list_zone_offers).post(create_zone_offer))
        .route(
            "/zone-offers/{id}",
            patch(update_zone_offer).delete(delete_zone_offer),
        )
        .route(
            "/zone-node-bindings",
            get(list_zone_node_bindings).post(create_zone_node_binding),
        )
        .route("/zone-node-bindings/{id}", delete(delete_zone_node_binding))
}

#[derive(Debug, Deserialize)]
struct ListParams {
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    status: Option<String>,
    kind: Option<String>,
    bbox: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListZoneBindingsParams {
    zone_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListZoneOffersParams {
    zone_id: Option<String>,
    package_id: Option<String>,
    active_only: Option<bool>,
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

fn parse_bbox(raw: Option<String>) -> AppResult<Option<(f64, f64, f64, f64)>> {
    let Some(raw) = raw else { return Ok(None) };
    let parts: Vec<&str> = raw.split(',').collect();
    if parts.len() != 4 {
        return Err(AppError::Validation(
            "bbox must be minLng,minLat,maxLng,maxLat".into(),
        ));
    }
    let min_lng = parts[0]
        .trim()
        .parse::<f64>()
        .map_err(|_| AppError::Validation("bbox minLng invalid".into()))?;
    let min_lat = parts[1]
        .trim()
        .parse::<f64>()
        .map_err(|_| AppError::Validation("bbox minLat invalid".into()))?;
    let max_lng = parts[2]
        .trim()
        .parse::<f64>()
        .map_err(|_| AppError::Validation("bbox maxLng invalid".into()))?;
    let max_lat = parts[3]
        .trim()
        .parse::<f64>()
        .map_err(|_| AppError::Validation("bbox maxLat invalid".into()))?;
    Ok(Some((min_lng, min_lat, max_lng, max_lat)))
}

fn to_list_query(q: ListParams) -> AppResult<ListQuery> {
    Ok(ListQuery {
        q: q.q,
        page: q.page.unwrap_or(1),
        per_page: q.per_page.unwrap_or(50),
        status: q.status,
        kind: q.kind,
        bbox: parse_bbox(q.bbox)?,
    })
}

async fn list_nodes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListParams>,
) -> AppResult<Json<PaginatedResponse<crate::models::NetworkNode>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .list_nodes(&claims.sub, &tenant_id, to_list_query(q)?)
        .await?;
    Ok(Json(out))
}

async fn create_node(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<CreateNetworkNodeRequest>,
) -> AppResult<Json<crate::models::NetworkNode>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .create_node(&claims.sub, &tenant_id, dto)
        .await?;
    Ok(Json(out))
}

async fn update_node(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(dto): Json<UpdateNetworkNodeRequest>,
) -> AppResult<Json<crate::models::NetworkNode>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .update_node(&claims.sub, &tenant_id, &id, dto)
        .await?;
    Ok(Json(out))
}

async fn delete_node(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .network_mapping_service
        .delete_node(&claims.sub, &tenant_id, &id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn list_links(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListParams>,
) -> AppResult<Json<PaginatedResponse<crate::models::NetworkLink>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .list_links(&claims.sub, &tenant_id, to_list_query(q)?)
        .await?;
    Ok(Json(out))
}

async fn create_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<CreateNetworkLinkRequest>,
) -> AppResult<Json<crate::models::NetworkLink>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .create_link(&claims.sub, &tenant_id, dto)
        .await?;
    Ok(Json(out))
}

async fn update_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(dto): Json<UpdateNetworkLinkRequest>,
) -> AppResult<Json<crate::models::NetworkLink>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .update_link(&claims.sub, &tenant_id, &id, dto)
        .await?;
    Ok(Json(out))
}

async fn delete_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .network_mapping_service
        .delete_link(&claims.sub, &tenant_id, &id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn list_zones(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListParams>,
) -> AppResult<Json<PaginatedResponse<crate::models::ServiceZone>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .list_zones(&claims.sub, &tenant_id, to_list_query(q)?)
        .await?;
    Ok(Json(out))
}

async fn create_zone(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<CreateServiceZoneRequest>,
) -> AppResult<Json<crate::models::ServiceZone>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .create_zone(&claims.sub, &tenant_id, dto)
        .await?;
    Ok(Json(out))
}

async fn update_zone(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(dto): Json<UpdateServiceZoneRequest>,
) -> AppResult<Json<crate::models::ServiceZone>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .update_zone(&claims.sub, &tenant_id, &id, dto)
        .await?;
    Ok(Json(out))
}

async fn delete_zone(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .network_mapping_service
        .delete_zone(&claims.sub, &tenant_id, &id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn resolve_zone(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<ResolveZoneRequest>,
) -> AppResult<Json<crate::models::ResolvedZoneResponse>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .resolve_zone(&claims.sub, &tenant_id, dto)
        .await?;
    Ok(Json(out))
}

async fn check_coverage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<CoverageCheckRequest>,
) -> AppResult<Json<crate::models::CoverageCheckResponse>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .coverage_check(&claims.sub, &tenant_id, dto)
        .await?;
    Ok(Json(out))
}

async fn compute_path(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<ComputePathRequest>,
) -> AppResult<Json<crate::models::ComputePathResponse>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .compute_path(&claims.sub, &tenant_id, dto)
        .await?;
    Ok(Json(out))
}

async fn list_zone_offers(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListZoneOffersParams>,
) -> AppResult<Json<Vec<crate::models::ZoneOffer>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .list_zone_offers(
            &claims.sub,
            &tenant_id,
            q.zone_id,
            q.package_id,
            q.active_only.unwrap_or(false),
        )
        .await?;
    Ok(Json(out))
}

async fn create_zone_offer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<CreateZoneOfferRequest>,
) -> AppResult<Json<crate::models::ZoneOffer>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .create_zone_offer(&claims.sub, &tenant_id, dto)
        .await?;
    Ok(Json(out))
}

async fn update_zone_offer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(dto): Json<UpdateZoneOfferRequest>,
) -> AppResult<Json<crate::models::ZoneOffer>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .update_zone_offer(&claims.sub, &tenant_id, &id, dto)
        .await?;
    Ok(Json(out))
}

async fn delete_zone_offer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .network_mapping_service
        .delete_zone_offer(&claims.sub, &tenant_id, &id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn list_zone_node_bindings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListZoneBindingsParams>,
) -> AppResult<Json<Vec<crate::models::ZoneNodeBinding>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .list_zone_bindings(&claims.sub, &tenant_id, q.zone_id)
        .await?;
    Ok(Json(out))
}

async fn create_zone_node_binding(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<CreateZoneNodeBindingRequest>,
) -> AppResult<Json<crate::models::ZoneNodeBinding>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let out = state
        .network_mapping_service
        .create_zone_binding(&claims.sub, &tenant_id, dto)
        .await?;
    Ok(Json(out))
}

async fn delete_zone_node_binding(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    state
        .network_mapping_service
        .delete_zone_binding(&claims.sub, &tenant_id, &id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
