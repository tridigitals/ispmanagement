use crate::models::{
    ComputePathRequest, ComputePathResponse, ConnectNodeToLinkRequest, ConnectNodeToLinkResponse,
    CoverageCheckRequest, CoverageCheckResponse, CreateNetworkLinkRequest,
    CreateNetworkNodeRequest, CreateServiceZoneRequest, CreateZoneNodeBindingRequest,
    CreateZoneOfferRequest, NetworkImpactResponse, NetworkLink, NetworkNode, PaginatedResponse,
    RankCandidateNodesRequest, RankCandidateNodesResponse, ResolveZoneRequest,
    ResolvedZoneResponse, ServiceZone, SyncTopologyAssetsResponse, UpdateNetworkLinkRequest,
    UpdateNetworkNodeRequest, UpdateServiceZoneRequest, UpdateZoneOfferRequest, ZoneNodeBinding,
    ZoneOffer,
};
use crate::services::auth_service::Claims;
use crate::services::network_mapping_service::ListQuery;
use crate::services::{AuthService, NetworkMappingService};
use tauri::State;

async fn tenant_context(auth: &AuthService, token: &str) -> Result<(Claims, String), String> {
    let claims = auth
        .validate_token(token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    Ok((claims, tenant_id))
}

fn parse_bbox(raw: Option<String>) -> Result<Option<(f64, f64, f64, f64)>, String> {
    let Some(raw) = raw else { return Ok(None) };
    let parts: Vec<&str> = raw.split(',').collect();
    if parts.len() != 4 {
        return Err("bbox must be minLng,minLat,maxLng,maxLat".to_string());
    }

    let min_lng = parts[0]
        .trim()
        .parse::<f64>()
        .map_err(|_| "bbox minLng invalid".to_string())?;
    let min_lat = parts[1]
        .trim()
        .parse::<f64>()
        .map_err(|_| "bbox minLat invalid".to_string())?;
    let max_lng = parts[2]
        .trim()
        .parse::<f64>()
        .map_err(|_| "bbox maxLng invalid".to_string())?;
    let max_lat = parts[3]
        .trim()
        .parse::<f64>()
        .map_err(|_| "bbox maxLat invalid".to_string())?;

    Ok(Some((min_lng, min_lat, max_lng, max_lat)))
}

fn list_query(
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    status: Option<String>,
    kind: Option<String>,
    bbox: Option<String>,
    include_legacy_ftth: Option<bool>,
) -> Result<ListQuery, String> {
    Ok(ListQuery {
        q,
        page: page.unwrap_or(1),
        per_page: per_page.unwrap_or(50),
        status,
        kind,
        bbox: parse_bbox(bbox)?,
        include_legacy_ftth: include_legacy_ftth.unwrap_or(false),
    })
}

#[tauri::command]
pub async fn list_network_nodes(
    token: String,
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    status: Option<String>,
    kind: Option<String>,
    bbox: Option<String>,
    include_legacy_ftth: Option<bool>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<PaginatedResponse<NetworkNode>, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .list_nodes(
            &claims.sub,
            &tenant_id,
            list_query(q, page, per_page, status, kind, bbox, include_legacy_ftth)?,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_network_node(
    token: String,
    name: String,
    node_type: String,
    status: Option<String>,
    lat: f64,
    lng: f64,
    capacity_json: Option<serde_json::Value>,
    health_json: Option<serde_json::Value>,
    metadata: Option<serde_json::Value>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<NetworkNode, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .create_node(
            &claims.sub,
            &tenant_id,
            CreateNetworkNodeRequest {
                name,
                node_type,
                status,
                lat,
                lng,
                capacity_json,
                health_json,
                metadata,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_network_node(
    token: String,
    id: String,
    name: Option<String>,
    node_type: Option<String>,
    status: Option<String>,
    lat: Option<f64>,
    lng: Option<f64>,
    capacity_json: Option<serde_json::Value>,
    health_json: Option<serde_json::Value>,
    metadata: Option<serde_json::Value>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<NetworkNode, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .update_node(
            &claims.sub,
            &tenant_id,
            &id,
            UpdateNetworkNodeRequest {
                name,
                node_type,
                status,
                lat,
                lng,
                capacity_json,
                health_json,
                metadata,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_network_node(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<(), String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .delete_node(&claims.sub, &tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_network_links(
    token: String,
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    status: Option<String>,
    kind: Option<String>,
    bbox: Option<String>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<PaginatedResponse<NetworkLink>, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .list_links(
            &claims.sub,
            &tenant_id,
            list_query(q, page, per_page, status, kind, bbox, None)?,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_network_link(
    token: String,
    from_node_id: String,
    to_node_id: String,
    name: String,
    link_type: String,
    status: Option<String>,
    priority: Option<i32>,
    capacity_mbps: Option<f64>,
    utilization_pct: Option<f64>,
    loss_db: Option<f64>,
    latency_ms: Option<f64>,
    geometry: serde_json::Value,
    metadata: Option<serde_json::Value>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<NetworkLink, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .create_link(
            &claims.sub,
            &tenant_id,
            CreateNetworkLinkRequest {
                from_node_id,
                to_node_id,
                name,
                link_type,
                status,
                priority,
                capacity_mbps,
                utilization_pct,
                loss_db,
                latency_ms,
                geometry,
                metadata,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn connect_network_node_to_link(
    token: String,
    source_node_id: String,
    target_link_id: String,
    name: String,
    link_type: String,
    status: Option<String>,
    priority: Option<i32>,
    capacity_mbps: Option<f64>,
    utilization_pct: Option<f64>,
    loss_db: Option<f64>,
    latency_ms: Option<f64>,
    geometry: serde_json::Value,
    split_lat: f64,
    split_lng: f64,
    junction_name: Option<String>,
    junction_node_type: Option<String>,
    metadata: Option<serde_json::Value>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<ConnectNodeToLinkResponse, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .connect_node_to_link(
            &claims.sub,
            &tenant_id,
            ConnectNodeToLinkRequest {
                source_node_id,
                target_link_id,
                name,
                link_type,
                status,
                priority,
                capacity_mbps,
                utilization_pct,
                loss_db,
                latency_ms,
                geometry,
                split_lat,
                split_lng,
                junction_name,
                junction_node_type,
                metadata,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_network_link(
    token: String,
    id: String,
    from_node_id: Option<String>,
    to_node_id: Option<String>,
    name: Option<String>,
    link_type: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
    capacity_mbps: Option<f64>,
    utilization_pct: Option<f64>,
    loss_db: Option<f64>,
    latency_ms: Option<f64>,
    geometry: Option<serde_json::Value>,
    metadata: Option<serde_json::Value>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<NetworkLink, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .update_link(
            &claims.sub,
            &tenant_id,
            &id,
            UpdateNetworkLinkRequest {
                from_node_id,
                to_node_id,
                name,
                link_type,
                status,
                priority,
                capacity_mbps,
                utilization_pct,
                loss_db,
                latency_ms,
                geometry,
                metadata,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_network_link(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<(), String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .delete_link(&claims.sub, &tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn compute_network_path(
    token: String,
    source_node_id: String,
    target_node_id: String,
    max_hops: Option<u32>,
    max_utilization_pct: Option<f64>,
    allowed_link_types: Option<Vec<String>>,
    allowed_statuses: Option<Vec<String>>,
    exclude_link_ids: Option<Vec<String>>,
    require_active_nodes: Option<bool>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<ComputePathResponse, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .compute_path(
            &claims.sub,
            &tenant_id,
            ComputePathRequest {
                source_node_id,
                target_node_id,
                max_hops,
                max_utilization_pct,
                allowed_link_types,
                allowed_statuses,
                exclude_link_ids,
                require_active_nodes,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_network_mapping_assets(
    token: String,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<SyncTopologyAssetsResponse, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .sync_topology_asset_nodes(&claims.sub, &tenant_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rank_candidate_network_nodes(
    token: String,
    lat: Option<f64>,
    lng: Option<f64>,
    zone_id: Option<String>,
    node_types: Option<Vec<String>>,
    limit: Option<u32>,
    require_active_nodes: Option<bool>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<RankCandidateNodesResponse, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .rank_candidate_nodes(
            &claims.sub,
            &tenant_id,
            RankCandidateNodesRequest {
                lat,
                lng,
                zone_id,
                node_types,
                limit,
                require_active_nodes,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_service_zones(
    token: String,
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    status: Option<String>,
    kind: Option<String>,
    bbox: Option<String>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<PaginatedResponse<ServiceZone>, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .list_zones(
            &claims.sub,
            &tenant_id,
            list_query(q, page, per_page, status, kind, bbox, None)?,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_service_zone(
    token: String,
    name: String,
    zone_type: String,
    priority: Option<i32>,
    status: Option<String>,
    geometry: serde_json::Value,
    metadata: Option<serde_json::Value>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<ServiceZone, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .create_zone(
            &claims.sub,
            &tenant_id,
            CreateServiceZoneRequest {
                name,
                zone_type,
                priority,
                status,
                geometry,
                metadata,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_service_zone(
    token: String,
    id: String,
    name: Option<String>,
    zone_type: Option<String>,
    priority: Option<i32>,
    status: Option<String>,
    geometry: Option<serde_json::Value>,
    metadata: Option<serde_json::Value>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<ServiceZone, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .update_zone(
            &claims.sub,
            &tenant_id,
            &id,
            UpdateServiceZoneRequest {
                name,
                zone_type,
                priority,
                status,
                geometry,
                metadata,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_service_zone(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<(), String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .delete_zone(&claims.sub, &tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resolve_service_zone(
    token: String,
    lat: f64,
    lng: f64,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<ResolvedZoneResponse, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .resolve_zone(&claims.sub, &tenant_id, ResolveZoneRequest { lat, lng })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_network_coverage(
    token: String,
    lat: f64,
    lng: f64,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<CoverageCheckResponse, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .coverage_check(&claims.sub, &tenant_id, CoverageCheckRequest { lat, lng })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_zone_offers(
    token: String,
    zone_id: Option<String>,
    package_id: Option<String>,
    active_only: Option<bool>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<Vec<ZoneOffer>, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .list_zone_offers(
            &claims.sub,
            &tenant_id,
            zone_id,
            package_id,
            active_only.unwrap_or(false),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_zone_offer(
    token: String,
    zone_id: String,
    package_id: String,
    price_monthly: Option<f64>,
    price_yearly: Option<f64>,
    is_active: Option<bool>,
    metadata: Option<serde_json::Value>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<ZoneOffer, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .create_zone_offer(
            &claims.sub,
            &tenant_id,
            CreateZoneOfferRequest {
                zone_id,
                package_id,
                price_monthly,
                price_yearly,
                is_active,
                metadata,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_zone_offer(
    token: String,
    id: String,
    zone_id: Option<String>,
    package_id: Option<String>,
    price_monthly: Option<f64>,
    price_yearly: Option<f64>,
    is_active: Option<bool>,
    metadata: Option<serde_json::Value>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<ZoneOffer, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .update_zone_offer(
            &claims.sub,
            &tenant_id,
            &id,
            UpdateZoneOfferRequest {
                zone_id,
                package_id,
                price_monthly,
                price_yearly,
                is_active,
                metadata,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_zone_offer(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<(), String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .delete_zone_offer(&claims.sub, &tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_zone_node_bindings(
    token: String,
    zone_id: Option<String>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<Vec<ZoneNodeBinding>, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .list_zone_bindings(&claims.sub, &tenant_id, zone_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_zone_node_binding(
    token: String,
    zone_id: String,
    node_id: String,
    is_primary: Option<bool>,
    weight: Option<i32>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<ZoneNodeBinding, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .create_zone_binding(
            &claims.sub,
            &tenant_id,
            CreateZoneNodeBindingRequest {
                zone_id,
                node_id,
                is_primary,
                weight,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_zone_node_binding(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<(), String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .delete_zone_binding(&claims.sub, &tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_network_impacted_customers(
    token: String,
    node_id: Option<String>,
    link_id: Option<String>,
    router_id: Option<String>,
    auth: State<'_, AuthService>,
    network_mapping: State<'_, NetworkMappingService>,
) -> Result<NetworkImpactResponse, String> {
    let (claims, tenant_id) = tenant_context(&auth, &token).await?;
    network_mapping
        .list_impacted_customers(&claims.sub, &tenant_id, node_id, link_id, router_id)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn network_mapping_commands_are_defined() {
        let source = include_str!("network_mapping.rs");

        assert!(source.contains("pub async fn list_network_nodes"));
        assert!(source.contains("pub async fn create_network_node"));
        assert!(source.contains("pub async fn list_network_links"));
        assert!(source.contains("pub async fn connect_network_node_to_link"));
        assert!(source.contains("pub async fn compute_network_path"));
        assert!(source.contains("pub async fn sync_network_mapping_assets"));
        assert!(source.contains("pub async fn list_service_zones"));
        assert!(source.contains("pub async fn check_network_coverage"));
        assert!(source.contains("pub async fn list_zone_offers"));
        assert!(source.contains("pub async fn list_zone_node_bindings"));
        assert!(source.contains("pub async fn list_network_impacted_customers"));
    }
}
