use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    ComputePathRequest, ComputePathResponse, ComputedPathHop, ConnectNodeToLinkRequest,
    ConnectNodeToLinkResponse, CoverageCheckRequest, CoverageCheckResponse,
    CreateNetworkLinkRequest, CreateNetworkNodeRequest, CreateServiceZoneRequest,
    CreateZoneNodeBindingRequest, CreateZoneOfferRequest, NetworkImpactCustomer,
    NetworkImpactResponse, NetworkLink, NetworkNode, PaginatedResponse, RankCandidateNodesRequest,
    RankCandidateNodesResponse, RankedCandidateNode, ResolveZoneRequest, ResolvedZone,
    ResolvedZoneResponse, ServiceZone, SyncTopologyAssetsResponse, UpdateNetworkLinkRequest,
    UpdateNetworkNodeRequest, UpdateServiceZoneRequest, UpdateZoneOfferRequest, ZoneNodeBinding,
    ZoneOffer,
};
use crate::services::AuthService;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Clone)]
pub struct NetworkMappingService {
    pool: DbPool,
    auth_service: AuthService,
}

#[derive(Debug, Clone)]
pub struct ListQuery {
    pub q: Option<String>,
    pub page: u32,
    pub per_page: u32,
    pub status: Option<String>,
    pub kind: Option<String>,
    pub bbox: Option<(f64, f64, f64, f64)>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct NodeStatusRow {
    id: String,
    status: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct PathLinkRow {
    id: String,
    from_node_id: String,
    to_node_id: String,
    name: String,
    link_type: String,
    status: String,
    distance_m: f64,
    utilization_pct: Option<f64>,
    loss_db: Option<f64>,
    latency_ms: Option<f64>,
}

#[derive(Debug, Clone)]
struct PathEdge {
    link_id: String,
    from_node_id: String,
    to_node_id: String,
    name: String,
    link_type: String,
    status: String,
    distance_m: f64,
    cost: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct CandidateNodeRow {
    node_id: String,
    name: String,
    node_type: String,
    status: String,
    capacity_json: serde_json::Value,
    health_json: serde_json::Value,
    distance_m: Option<f64>,
    avg_link_utilization_pct: Option<f64>,
    down_links: i64,
    link_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct UuidTextRow {
    id: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct SyncRouterRow {
    id: String,
    name: String,
    enabled: bool,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct SyncCustomerLocationRow {
    location_id: String,
    customer_id: String,
    customer_name: String,
    label: String,
    subscription_status: String,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Clone)]
struct SnappedPolylinePoint {
    lng: f64,
    lat: f64,
    segment_index: usize,
    t: f64,
    distance_sq: f64,
}

impl NetworkMappingService {
    pub fn new(pool: DbPool, auth_service: AuthService) -> Self {
        Self { pool, auth_service }
    }

    async fn check_permission_any(
        &self,
        actor_id: &str,
        tenant_id: &str,
        permissions: &[(&str, &str)],
    ) -> AppResult<()> {
        let mut last_err: Option<AppError> = None;
        for (resource, action) in permissions {
            match self
                .auth_service
                .check_permission(actor_id, tenant_id, resource, action)
                .await
            {
                Ok(()) => return Ok(()),
                Err(err) => last_err = Some(err),
            }
        }
        Err(last_err.unwrap_or_else(|| AppError::Forbidden("permission check failed".into())))
    }

    async fn require_read(&self, actor_id: &str, tenant_id: &str) -> AppResult<()> {
        self.check_permission_any(
            actor_id,
            tenant_id,
            &[("network_topology", "read"), ("network_routers", "read")],
        )
        .await
    }

    async fn require_manage(&self, actor_id: &str, tenant_id: &str) -> AppResult<()> {
        self.check_permission_any(
            actor_id,
            tenant_id,
            &[
                ("network_topology", "manage"),
                ("network_routers", "manage"),
            ],
        )
        .await
    }

    async fn require_installation_read(&self, actor_id: &str, tenant_id: &str) -> AppResult<()> {
        self.check_permission_any(
            actor_id,
            tenant_id,
            &[
                ("network_topology", "read"),
                ("network_routers", "read"),
                ("work_orders", "read"),
                ("work_orders", "manage"),
            ],
        )
        .await
    }

    async fn require_installation_manage(&self, actor_id: &str, tenant_id: &str) -> AppResult<()> {
        self.check_permission_any(
            actor_id,
            tenant_id,
            &[
                ("network_topology", "manage"),
                ("network_routers", "manage"),
                ("work_orders", "manage"),
            ],
        )
        .await
    }

    async fn require_zones_read(&self, actor_id: &str, tenant_id: &str) -> AppResult<()> {
        self.check_permission_any(
            actor_id,
            tenant_id,
            &[
                ("service_zones", "read"),
                ("network_topology", "read"),
                ("network_routers", "read"),
            ],
        )
        .await
    }

    async fn require_zones_manage(&self, actor_id: &str, tenant_id: &str) -> AppResult<()> {
        self.check_permission_any(
            actor_id,
            tenant_id,
            &[
                ("service_zones", "manage"),
                ("network_topology", "manage"),
                ("network_routers", "manage"),
            ],
        )
        .await
    }

    async fn require_coverage_read(&self, actor_id: &str, tenant_id: &str) -> AppResult<()> {
        self.check_permission_any(
            actor_id,
            tenant_id,
            &[
                ("coverage", "read"),
                ("service_zones", "read"),
                ("network_topology", "read"),
                ("network_routers", "read"),
            ],
        )
        .await
    }

    fn cleaned_query(q: Option<String>) -> String {
        q.unwrap_or_default().trim().to_string()
    }

    fn validate_lat_lng(lat: f64, lng: f64, field: &str) -> AppResult<()> {
        if !(-90.0..=90.0).contains(&lat) {
            return Err(AppError::Validation(format!(
                "{field}.lat must be between -90 and 90"
            )));
        }
        if !(-180.0..=180.0).contains(&lng) {
            return Err(AppError::Validation(format!(
                "{field}.lng must be between -180 and 180"
            )));
        }
        Ok(())
    }

    fn validate_geojson_geometry(
        geometry: &serde_json::Value,
        expected_types: &[&str],
        field: &str,
    ) -> AppResult<()> {
        let obj = geometry
            .as_object()
            .ok_or_else(|| AppError::Validation(format!("{field} must be a GeoJSON object")))?;
        let kind = obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Validation(format!("{field}.type is required")))?;
        if !expected_types.iter().any(|t| *t == kind) {
            return Err(AppError::Validation(format!(
                "{field}.type must be one of: {}",
                expected_types.join(", ")
            )));
        }
        if !obj.contains_key("coordinates") {
            return Err(AppError::Validation(format!(
                "{field}.coordinates is required"
            )));
        }
        Ok(())
    }

    fn point_distance_sq(a: [f64; 2], b: [f64; 2]) -> f64 {
        let dx = a[0] - b[0];
        let dy = a[1] - b[1];
        (dx * dx) + (dy * dy)
    }

    fn coords_approx_equal(a: [f64; 2], b: [f64; 2]) -> bool {
        Self::point_distance_sq(a, b) <= 1e-16
    }

    fn parse_line_coords(geometry: &serde_json::Value, field: &str) -> AppResult<Vec<[f64; 2]>> {
        let obj = geometry
            .as_object()
            .ok_or_else(|| AppError::Validation(format!("{field} must be a GeoJSON object")))?;
        let kind = obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Validation(format!("{field}.type is required")))?;
        let coords = obj
            .get("coordinates")
            .ok_or_else(|| AppError::Validation(format!("{field}.coordinates is required")))?;

        let mut out: Vec<[f64; 2]> = Vec::new();
        match kind {
            "LineString" => {
                let arr = coords.as_array().ok_or_else(|| {
                    AppError::Validation(format!("{field}.coordinates must be an array"))
                })?;
                for point in arr {
                    let pt = point.as_array().ok_or_else(|| {
                        AppError::Validation(format!("{field}.coordinates contains invalid point"))
                    })?;
                    if pt.len() < 2 {
                        return Err(AppError::Validation(format!(
                            "{field}.coordinates contains invalid point"
                        )));
                    }
                    let lng = pt[0].as_f64().ok_or_else(|| {
                        AppError::Validation(format!("{field}.coordinates contains invalid lng"))
                    })?;
                    let lat = pt[1].as_f64().ok_or_else(|| {
                        AppError::Validation(format!("{field}.coordinates contains invalid lat"))
                    })?;
                    out.push([lng, lat]);
                }
            }
            "MultiLineString" => {
                let lines = coords.as_array().ok_or_else(|| {
                    AppError::Validation(format!("{field}.coordinates must be an array"))
                })?;
                for line in lines {
                    let line_points = line.as_array().ok_or_else(|| {
                        AppError::Validation(format!(
                            "{field}.coordinates contains invalid line segment"
                        ))
                    })?;
                    for point in line_points {
                        let pt = point.as_array().ok_or_else(|| {
                            AppError::Validation(format!(
                                "{field}.coordinates contains invalid point"
                            ))
                        })?;
                        if pt.len() < 2 {
                            return Err(AppError::Validation(format!(
                                "{field}.coordinates contains invalid point"
                            )));
                        }
                        let lng = pt[0].as_f64().ok_or_else(|| {
                            AppError::Validation(format!(
                                "{field}.coordinates contains invalid lng"
                            ))
                        })?;
                        let lat = pt[1].as_f64().ok_or_else(|| {
                            AppError::Validation(format!(
                                "{field}.coordinates contains invalid lat"
                            ))
                        })?;
                        let candidate = [lng, lat];
                        if out
                            .last()
                            .copied()
                            .map(|prev| Self::coords_approx_equal(prev, candidate))
                            .unwrap_or(false)
                        {
                            continue;
                        }
                        out.push(candidate);
                    }
                }
            }
            _ => {
                return Err(AppError::Validation(format!(
                    "{field}.type must be LineString or MultiLineString"
                )));
            }
        }

        if out.len() < 2 {
            return Err(AppError::Validation(format!(
                "{field} must contain at least 2 coordinates"
            )));
        }
        Ok(out)
    }

    fn build_line_geometry(coords: &[[f64; 2]]) -> serde_json::Value {
        serde_json::json!({
            "type": "LineString",
            "coordinates": coords.iter().map(|pt| vec![pt[0], pt[1]]).collect::<Vec<_>>(),
        })
    }

    fn snap_point_to_polyline(
        coords: &[[f64; 2]],
        lng: f64,
        lat: f64,
    ) -> Option<SnappedPolylinePoint> {
        if coords.len() < 2 {
            return None;
        }

        let point = [lng, lat];
        let mut best: Option<SnappedPolylinePoint> = None;

        for segment_index in 0..(coords.len() - 1) {
            let a = coords[segment_index];
            let b = coords[segment_index + 1];
            let abx = b[0] - a[0];
            let aby = b[1] - a[1];
            let denom = (abx * abx) + (aby * aby);
            let t = if denom <= 1e-18 {
                0.0
            } else {
                (((point[0] - a[0]) * abx) + ((point[1] - a[1]) * aby)) / denom
            }
            .clamp(0.0, 1.0);
            let snapped_lng = a[0] + (abx * t);
            let snapped_lat = a[1] + (aby * t);
            let distance_sq = Self::point_distance_sq(point, [snapped_lng, snapped_lat]);
            let candidate = SnappedPolylinePoint {
                lng: snapped_lng,
                lat: snapped_lat,
                segment_index,
                t,
                distance_sq,
            };

            if best
                .as_ref()
                .map(|current| candidate.distance_sq < current.distance_sq)
                .unwrap_or(true)
            {
                best = Some(candidate);
            }
        }

        best
    }

    fn dedupe_line_coords(coords: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
        let mut out: Vec<[f64; 2]> = Vec::with_capacity(coords.len());
        for point in coords {
            if out
                .last()
                .copied()
                .map(|prev| Self::coords_approx_equal(prev, point))
                .unwrap_or(false)
            {
                continue;
            }
            out.push(point);
        }
        out
    }

    fn split_polyline_at_point(
        coords: &[[f64; 2]],
        snapped: &SnappedPolylinePoint,
    ) -> AppResult<(Vec<[f64; 2]>, Vec<[f64; 2]>)> {
        if coords.len() < 2 {
            return Err(AppError::Validation(
                "Target link geometry is too short".into(),
            ));
        }

        let split_point = [snapped.lng, snapped.lat];
        let start = coords[0];
        let end = coords[coords.len() - 1];

        if Self::coords_approx_equal(start, split_point)
            || Self::coords_approx_equal(end, split_point)
            || (snapped.segment_index == 0 && snapped.t <= 1e-6)
            || (snapped.segment_index == coords.len() - 2 && snapped.t >= 1.0 - 1e-6)
        {
            return Err(AppError::Validation(
                "Selected point is too close to an existing node. Click the node instead.".into(),
            ));
        }

        let mut first = coords[..=snapped.segment_index].to_vec();
        first.push(split_point);
        let mut second = vec![split_point];
        second.extend_from_slice(&coords[(snapped.segment_index + 1)..]);

        let first = Self::dedupe_line_coords(first);
        let second = Self::dedupe_line_coords(second);

        if first.len() < 2 || second.len() < 2 {
            return Err(AppError::Validation(
                "Selected point does not create two valid link segments".into(),
            ));
        }

        Ok((first, second))
    }

    fn map_geometry_db_error(err: sqlx::Error, field: &str) -> AppError {
        let msg = err.to_string().to_lowercase();
        if msg.contains("st_geomfromgeojson")
            || msg.contains("parse error")
            || msg.contains("invalid geojson")
            || msg.contains("geometry")
            || msg.contains("lwgeom")
        {
            return AppError::Validation(format!("{field} is invalid GeoJSON geometry"));
        }
        AppError::Database(err)
    }

    fn normalize_link_status(input: &str) -> String {
        match input.trim().to_lowercase().as_str() {
            "active" => "up".to_string(),
            "inactive" => "down".to_string(),
            other => other.to_string(),
        }
    }

    fn validate_link_status(status: &str) -> AppResult<()> {
        match status {
            "up" | "down" | "degraded" | "maintenance" | "planning" | "retired" => Ok(()),
            _ => Err(AppError::Validation(
                "link status must be one of: up, down, degraded, maintenance, planning, retired"
                    .into(),
            )),
        }
    }

    fn link_cost(link: &PathLinkRow) -> f64 {
        let distance_km = (link.distance_m.max(0.0)) / 1000.0;
        let latency_component = link.latency_ms.unwrap_or(0.0) * 0.2;
        let utilization_component = link.utilization_pct.unwrap_or(0.0) * 0.1;
        let loss_component = link.loss_db.unwrap_or(0.0).abs() * 5.0;
        let status_penalty = match link.status.as_str() {
            "degraded" => 25.0,
            "planning" => 75.0,
            _ => 0.0,
        };
        (distance_km + latency_component + utilization_component + loss_component + status_penalty)
            .max(0.0001)
    }

    fn json_number(value: &serde_json::Value, key: &str) -> Option<f64> {
        value.get(key).and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_i64().map(|n| n as f64))
                .or_else(|| v.as_u64().map(|n| n as f64))
                .or_else(|| v.as_str().and_then(|s| s.trim().parse::<f64>().ok()))
        })
    }

    fn clamp_0_100(v: f64) -> f64 {
        if v.is_nan() {
            0.0
        } else {
            v.clamp(0.0, 100.0)
        }
    }

    fn compute_health_score(status: &str, health_json: &serde_json::Value) -> f64 {
        if let Some(score) = Self::json_number(health_json, "score") {
            return Self::clamp_0_100(score);
        }
        if let Some(score) = Self::json_number(health_json, "health_score") {
            return Self::clamp_0_100(score);
        }
        match status {
            "active" => 85.0,
            "maintenance" => 60.0,
            _ => 40.0,
        }
    }

    fn compute_capacity_score(
        capacity_json: &serde_json::Value,
        avg_link_utilization_pct: Option<f64>,
    ) -> f64 {
        if let Some(free_pct) = Self::json_number(capacity_json, "free_pct") {
            return Self::clamp_0_100(free_pct);
        }
        if let Some(util_pct) = Self::json_number(capacity_json, "utilization_pct") {
            return Self::clamp_0_100(100.0 - util_pct);
        }
        let available_mbps = Self::json_number(capacity_json, "available_mbps");
        let total_mbps = Self::json_number(capacity_json, "total_mbps");
        if let (Some(avail), Some(total)) = (available_mbps, total_mbps) {
            if total > 0.0 {
                return Self::clamp_0_100((avail / total) * 100.0);
            }
        }
        if let Some(util) = avg_link_utilization_pct {
            return Self::clamp_0_100(100.0 - util);
        }
        60.0
    }

    fn compute_distance_score(distance_m: Option<f64>) -> Option<f64> {
        distance_m.map(|distance| {
            let normalized = (distance / 50_000.0).clamp(0.0, 1.0);
            Self::clamp_0_100(100.0 - (normalized * 100.0))
        })
    }

    fn is_system_managed_node(metadata: &serde_json::Value) -> bool {
        metadata
            .get("system_managed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    fn system_managed_node_source_label(metadata: &serde_json::Value) -> Option<&str> {
        metadata
            .get("asset_source")
            .and_then(|v| v.as_str())
            .or_else(|| metadata.get("asset_type").and_then(|v| v.as_str()))
    }

    fn customer_subscription_to_node_status(status: &str) -> &'static str {
        match status.trim().to_lowercase().as_str() {
            "suspended" => "maintenance",
            "inactive" | "cancelled" => "inactive",
            _ => "active",
        }
    }

    async fn find_node_by_asset_reference(
        &self,
        tenant_id: &str,
        asset_type: &str,
        asset_id: &str,
    ) -> AppResult<Option<NetworkNode>> {
        sqlx::query_as(
            r#"
            SELECT
              id::text AS id,
              tenant_id::text AS tenant_id,
              name,
              node_type,
              status,
              ST_Y(geom)::float8 AS lat,
              ST_X(geom)::float8 AS lng,
              capacity_json,
              health_json,
              metadata,
              created_at,
              updated_at
            FROM network_nodes
            WHERE tenant_id = $1::uuid
              AND (
                (
                  metadata->>'asset_type' = $2::text
                  AND metadata->>'asset_id' = $3::text
                )
                OR (
                  $2::text = 'mikrotik_router'
                  AND (
                    metadata->>'router_id' = $3::text
                    OR metadata->>'routerId' = $3::text
                    OR metadata->>'mikrotik_router_id' = $3::text
                    OR metadata->>'mikrotikRouterId' = $3::text
                  )
                )
                OR (
                  $2::text = 'customer_location'
                  AND (
                    metadata->>'location_id' = $3::text
                    OR metadata->>'locationId' = $3::text
                    OR metadata->>'customer_location_id' = $3::text
                    OR metadata->>'customerLocationId' = $3::text
                  )
                )
              )
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(asset_type)
        .bind(asset_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn upsert_system_managed_node(
        &self,
        tenant_id: &str,
        asset_type: &str,
        asset_id: &str,
        name: &str,
        node_type: &str,
        status: &str,
        lat: f64,
        lng: f64,
        metadata: serde_json::Value,
    ) -> AppResult<bool> {
        Self::validate_lat_lng(lat, lng, "asset_node")?;
        let existing = self
            .find_node_by_asset_reference(tenant_id, asset_type, asset_id)
            .await?;

        if let Some(current) = existing {
            let mut merged = current.metadata;
            if let (Some(dst), Some(src)) = (merged.as_object_mut(), metadata.as_object()) {
                for (key, value) in src {
                    dst.insert(key.clone(), value.clone());
                }
            } else {
                merged = metadata;
            }

            sqlx::query(
                r#"
                UPDATE network_nodes
                SET name = $1,
                    node_type = $2,
                    status = $3,
                    geom = ST_SetSRID(ST_MakePoint($4, $5), 4326),
                    metadata = $6
                WHERE tenant_id = $7::uuid AND id = $8::uuid
                "#,
            )
            .bind(name)
            .bind(node_type)
            .bind(status)
            .bind(lng)
            .bind(lat)
            .bind(merged)
            .bind(tenant_id)
            .bind(&current.id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

            return Ok(false);
        }

        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO network_nodes
              (id, tenant_id, name, node_type, status, geom, capacity_json, health_json, metadata, created_at, updated_at)
            VALUES
              ($1::uuid, $2::uuid, $3, $4, $5, ST_SetSRID(ST_MakePoint($6, $7), 4326), '{}'::jsonb, '{}'::jsonb, $8, now(), now())
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(name)
        .bind(node_type)
        .bind(status)
        .bind(lng)
        .bind(lat)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(true)
    }

    async fn prune_system_managed_nodes_not_in_assets(
        &self,
        tenant_id: &str,
        asset_type: &str,
        asset_ids: &[String],
    ) -> AppResult<u64> {
        let rows_affected = if asset_ids.is_empty() {
            sqlx::query(
                r#"
                DELETE FROM network_nodes
                WHERE tenant_id = $1::uuid
                  AND metadata->>'asset_type' = $2::text
                  AND COALESCE((metadata->>'system_managed')::boolean, false)
                "#,
            )
            .bind(tenant_id)
            .bind(asset_type)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?
            .rows_affected()
        } else {
            sqlx::query(
                r#"
                DELETE FROM network_nodes
                WHERE tenant_id = $1::uuid
                  AND metadata->>'asset_type' = $2::text
                  AND COALESCE((metadata->>'system_managed')::boolean, false)
                  AND NOT (metadata->>'asset_id' = ANY($3))
                "#,
            )
            .bind(tenant_id)
            .bind(asset_type)
            .bind(asset_ids)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?
            .rows_affected()
        };

        Ok(rows_affected)
    }

    pub async fn sync_topology_asset_nodes(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<SyncTopologyAssetsResponse> {
        self.require_installation_manage(actor_id, tenant_id)
            .await?;

        let routers: Vec<SyncRouterRow> = sqlx::query_as(
            r#"
            SELECT
              id::text AS id,
              name,
              enabled,
              latitude::float8 AS latitude,
              longitude::float8 AS longitude
            FROM mikrotik_routers
            WHERE tenant_id = $1::text
              AND latitude IS NOT NULL
              AND longitude IS NOT NULL
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let customer_locations: Vec<SyncCustomerLocationRow> = sqlx::query_as(
            r#"
            SELECT
              cl.id::text AS location_id,
              cl.customer_id::text AS customer_id,
              c.name AS customer_name,
              COALESCE(NULLIF(BTRIM(cl.label), ''), c.name || ' Location') AS label,
              svc.subscription_status AS subscription_status,
              cl.latitude::float8 AS latitude,
              cl.longitude::float8 AS longitude
            FROM customer_locations cl
            INNER JOIN customers c
              ON c.tenant_id::text = cl.tenant_id::text
             AND c.id::text = cl.customer_id::text
            INNER JOIN LATERAL (
              SELECT cs.status AS subscription_status
              FROM customer_subscriptions cs
              WHERE cs.tenant_id = cl.tenant_id
                AND cs.location_id = cl.id
                AND cs.status IN ('active', 'pending_installation', 'suspended')
              ORDER BY
                CASE cs.status
                  WHEN 'active' THEN 0
                  WHEN 'pending_installation' THEN 1
                  WHEN 'suspended' THEN 2
                  ELSE 3
                END,
                cs.updated_at DESC,
                cs.created_at DESC
              LIMIT 1
            ) svc ON TRUE
            WHERE cl.tenant_id = $1::text
              AND cl.latitude IS NOT NULL
              AND cl.longitude IS NOT NULL
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let eligible_customer_location_ids: Vec<String> = customer_locations
            .iter()
            .map(|row| row.location_id.clone())
            .collect();
        let pruned_customer_nodes = self
            .prune_system_managed_nodes_not_in_assets(
                tenant_id,
                "customer_location",
                &eligible_customer_location_ids,
            )
            .await?;

        let mut router_nodes_created = 0_i64;
        let mut router_nodes_updated = 0_i64;
        let mut customer_nodes_created = 0_i64;
        let mut customer_nodes_updated = 0_i64;

        for row in routers {
            let created = self
                .upsert_system_managed_node(
                    tenant_id,
                    "mikrotik_router",
                    &row.id,
                    row.name.trim(),
                    "router",
                    if row.enabled { "active" } else { "inactive" },
                    row.latitude,
                    row.longitude,
                    serde_json::json!({
                        "system_managed": true,
                        "asset_source": "mikrotik_router",
                        "asset_type": "mikrotik_router",
                        "asset_id": row.id,
                        "router_id": row.id,
                    }),
                )
                .await?;
            if created {
                router_nodes_created += 1;
            } else {
                router_nodes_updated += 1;
            }
        }

        for row in customer_locations {
            let name = if row.customer_name.trim() == row.label.trim() {
                row.label.clone()
            } else {
                format!("{} - {}", row.customer_name.trim(), row.label.trim())
            };
            let created = self
                .upsert_system_managed_node(
                    tenant_id,
                    "customer_location",
                    &row.location_id,
                    name.trim(),
                    "customer_premise",
                    Self::customer_subscription_to_node_status(&row.subscription_status),
                    row.latitude,
                    row.longitude,
                    serde_json::json!({
                        "system_managed": true,
                        "asset_source": "customer_location",
                        "asset_type": "customer_location",
                        "asset_id": row.location_id,
                        "location_id": row.location_id,
                        "customer_id": row.customer_id,
                        "customer_name": row.customer_name,
                        "location_label": row.label,
                        "subscription_status": row.subscription_status,
                    }),
                )
                .await?;
            if created {
                customer_nodes_created += 1;
            } else {
                customer_nodes_updated += 1;
            }
        }

        Ok(SyncTopologyAssetsResponse {
            router_nodes_created,
            router_nodes_updated,
            customer_nodes_created,
            customer_nodes_updated,
            total_nodes_touched: router_nodes_created
                + router_nodes_updated
                + customer_nodes_created
                + customer_nodes_updated
                + pruned_customer_nodes as i64,
        })
    }

    pub async fn rank_candidate_nodes(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: RankCandidateNodesRequest,
    ) -> AppResult<RankCandidateNodesResponse> {
        self.require_read(actor_id, tenant_id).await?;

        if dto.lat.is_some() ^ dto.lng.is_some() {
            return Err(AppError::Validation(
                "lat and lng must be provided together".into(),
            ));
        }
        if let (Some(lat), Some(lng)) = (dto.lat, dto.lng) {
            Self::validate_lat_lng(lat, lng, "candidate")?;
        }

        let limit = dto.limit.unwrap_or(10).clamp(1, 100) as i64;
        let node_types = dto.node_types.filter(|v| !v.is_empty());
        let require_active_nodes = dto.require_active_nodes.unwrap_or(true);
        let zone_id = dto.zone_id.clone();
        let has_point = dto.lat.is_some() && dto.lng.is_some();

        let rows: Vec<CandidateNodeRow> = sqlx::query_as(
            r#"
            SELECT
              n.id::text AS node_id,
              n.name,
              n.node_type,
              n.status,
              n.capacity_json,
              n.health_json,
              CASE
                WHEN $2::bool = true
                THEN ST_Distance(
                  geography(n.geom),
                  geography(ST_SetSRID(ST_MakePoint($3::float8, $4::float8), 4326))
                )::float8
                ELSE NULL
              END AS distance_m,
              AVG(l.utilization_pct::float8) FILTER (WHERE l.utilization_pct IS NOT NULL) AS avg_link_utilization_pct,
              COALESCE(COUNT(l.id) FILTER (WHERE l.status = 'down'), 0)::bigint AS down_links,
              COALESCE(COUNT(l.id), 0)::bigint AS link_count
            FROM network_nodes n
            LEFT JOIN network_links l
              ON l.tenant_id = n.tenant_id
             AND (l.from_node_id = n.id OR l.to_node_id = n.id)
            WHERE n.tenant_id = $1::uuid
              AND ($5::bool = false OR n.status = 'active')
              AND ($6::text[] IS NULL OR n.node_type = ANY($6::text[]))
              AND (
                $7::uuid IS NULL
                OR EXISTS (
                  SELECT 1
                  FROM zone_node_bindings znb
                  WHERE znb.tenant_id = n.tenant_id
                    AND znb.zone_id = $7::uuid
                    AND znb.node_id = n.id
                )
              )
            GROUP BY n.id
            LIMIT 400
            "#,
        )
        .bind(tenant_id)
        .bind(has_point)
        .bind(dto.lng)
        .bind(dto.lat)
        .bind(require_active_nodes)
        .bind(node_types)
        .bind(zone_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut items: Vec<RankedCandidateNode> = rows
            .into_iter()
            .map(|row| {
                let health_score = Self::compute_health_score(&row.status, &row.health_json);
                let capacity_score =
                    Self::compute_capacity_score(&row.capacity_json, row.avg_link_utilization_pct);
                let distance_score = Self::compute_distance_score(row.distance_m);
                let distance_component = distance_score.unwrap_or(60.0);

                let stability_penalty =
                    (row.down_links as f64 * 7.5) + if row.link_count == 0 { 12.0 } else { 0.0 };
                let base_score =
                    (health_score * 0.45) + (capacity_score * 0.35) + (distance_component * 0.20);
                let score = (base_score - stability_penalty).clamp(0.0, 100.0);
                let reason = format!(
                    "health {:.0}, capacity {:.0}{}{}",
                    health_score,
                    capacity_score,
                    match distance_score {
                        Some(s) => format!(", distance {:.0}", s),
                        None => String::new(),
                    },
                    if row.down_links > 0 {
                        format!(", penalty: {} down link(s)", row.down_links)
                    } else {
                        String::new()
                    }
                );

                RankedCandidateNode {
                    node_id: row.node_id,
                    name: row.name,
                    node_type: row.node_type,
                    status: row.status,
                    score,
                    health_score,
                    capacity_score,
                    distance_score,
                    distance_m: row.distance_m,
                    avg_link_utilization_pct: row.avg_link_utilization_pct,
                    down_links: row.down_links,
                    link_count: row.link_count,
                    reason,
                }
            })
            .collect();

        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.distance_m
                        .unwrap_or(f64::INFINITY)
                        .partial_cmp(&b.distance_m.unwrap_or(f64::INFINITY))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        items.truncate(limit as usize);

        Ok(RankCandidateNodesResponse { items })
    }

    async fn ensure_link_pair_available(
        &self,
        tenant_id: &str,
        from_node_id: &str,
        to_node_id: &str,
        exclude_link_id: Option<&str>,
    ) -> AppResult<()> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM network_links
              WHERE tenant_id = $1::uuid
                AND (
                  (from_node_id = $2::uuid AND to_node_id = $3::uuid)
                  OR (from_node_id = $3::uuid AND to_node_id = $2::uuid)
                )
                AND ($4::uuid IS NULL OR id <> $4::uuid)
            )
            "#,
        )
        .bind(tenant_id)
        .bind(from_node_id)
        .bind(to_node_id)
        .bind(exclude_link_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if exists {
            return Err(AppError::Validation(
                "A link between these two nodes already exists".into(),
            ));
        }
        Ok(())
    }

    pub async fn compute_path(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: ComputePathRequest,
    ) -> AppResult<ComputePathResponse> {
        self.require_read(actor_id, tenant_id).await?;
        let source_id = dto.source_node_id.clone();
        let target_id = dto.target_node_id.clone();

        if source_id == target_id {
            return Err(AppError::Validation(
                "source_node_id and target_node_id must be different".into(),
            ));
        }

        let node_rows: Vec<NodeStatusRow> = sqlx::query_as(
            r#"
            SELECT id::text AS id, status
            FROM network_nodes
            WHERE tenant_id = $1::uuid
              AND id::text IN ($2, $3)
            "#,
        )
        .bind(tenant_id)
        .bind(&source_id)
        .bind(&target_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if node_rows.len() < 2 {
            return Err(AppError::Validation(
                "source_node_id or target_node_id not found".into(),
            ));
        }

        let source_status = node_rows
            .iter()
            .find(|n| n.id == source_id)
            .map(|n| n.status.clone())
            .unwrap_or_else(|| "inactive".to_string());
        let target_status = node_rows
            .iter()
            .find(|n| n.id == target_id)
            .map(|n| n.status.clone())
            .unwrap_or_else(|| "inactive".to_string());

        let require_active_nodes = dto.require_active_nodes.unwrap_or(true);
        if require_active_nodes && (source_status != "active" || target_status != "active") {
            return Ok(ComputePathResponse {
                found: false,
                source_node_id: source_id.clone(),
                target_node_id: target_id.clone(),
                node_ids: vec![],
                link_ids: vec![],
                hops: vec![],
                total_cost: None,
                total_distance_m: None,
            });
        }

        let allowed_statuses = if let Some(v) = dto.allowed_statuses {
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        } else {
            Some(vec!["up".to_string(), "degraded".to_string()])
        };
        let allowed_link_types = dto.allowed_link_types.filter(|v| !v.is_empty());
        let exclude_link_ids = dto.exclude_link_ids.filter(|v| !v.is_empty());

        let links: Vec<PathLinkRow> = sqlx::query_as(
            r#"
            SELECT
              id::text AS id,
              from_node_id::text AS from_node_id,
              to_node_id::text AS to_node_id,
              name,
              link_type,
              status,
              COALESCE(ST_Length(geography(geom)), 0)::float8 AS distance_m,
              utilization_pct::float8 AS utilization_pct,
              loss_db::float8 AS loss_db,
              latency_ms::float8 AS latency_ms
            FROM network_links
            WHERE tenant_id = $1::uuid
              AND ($2::text[] IS NULL OR link_type = ANY($2::text[]))
              AND ($3::text[] IS NULL OR status = ANY($3::text[]))
              AND ($4::text[] IS NULL OR NOT (id::text = ANY($4::text[])))
              AND ($5::float8 IS NULL OR utilization_pct IS NULL OR utilization_pct::float8 <= $5::float8)
            "#,
        )
        .bind(tenant_id)
        .bind(allowed_link_types)
        .bind(allowed_statuses)
        .bind(exclude_link_ids)
        .bind(dto.max_utilization_pct)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if links.is_empty() {
            return Ok(ComputePathResponse {
                found: false,
                source_node_id: source_id.clone(),
                target_node_id: target_id.clone(),
                node_ids: vec![],
                link_ids: vec![],
                hops: vec![],
                total_cost: None,
                total_distance_m: None,
            });
        }

        let node_status_rows: Vec<NodeStatusRow> = sqlx::query_as(
            r#"
            SELECT id::text AS id, status
            FROM network_nodes
            WHERE tenant_id = $1::uuid
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;
        let node_statuses: HashMap<String, String> = node_status_rows
            .into_iter()
            .map(|r| (r.id, r.status))
            .collect();

        let mut adjacency: HashMap<String, Vec<PathEdge>> = HashMap::new();
        for link in links {
            if require_active_nodes {
                let from_active = node_statuses
                    .get(&link.from_node_id)
                    .map(|s| s == "active")
                    .unwrap_or(false);
                let to_active = node_statuses
                    .get(&link.to_node_id)
                    .map(|s| s == "active")
                    .unwrap_or(false);
                if !from_active || !to_active {
                    continue;
                }
            }

            let cost = Self::link_cost(&link);
            let forward = PathEdge {
                link_id: link.id.clone(),
                from_node_id: link.from_node_id.clone(),
                to_node_id: link.to_node_id.clone(),
                name: link.name.clone(),
                link_type: link.link_type.clone(),
                status: link.status.clone(),
                distance_m: link.distance_m,
                cost,
            };
            let backward = PathEdge {
                link_id: link.id.clone(),
                from_node_id: link.to_node_id.clone(),
                to_node_id: link.from_node_id.clone(),
                name: link.name,
                link_type: link.link_type,
                status: link.status,
                distance_m: link.distance_m,
                cost,
            };
            adjacency
                .entry(forward.from_node_id.clone())
                .or_default()
                .push(forward);
            adjacency
                .entry(backward.from_node_id.clone())
                .or_default()
                .push(backward);
        }

        if !adjacency.contains_key(&source_id) {
            return Ok(ComputePathResponse {
                found: false,
                source_node_id: source_id.clone(),
                target_node_id: target_id.clone(),
                node_ids: vec![],
                link_ids: vec![],
                hops: vec![],
                total_cost: None,
                total_distance_m: None,
            });
        }

        let max_hops = dto.max_hops.unwrap_or(64).max(1) as usize;
        let mut dist: HashMap<String, f64> = HashMap::new();
        let mut hop_count: HashMap<String, usize> = HashMap::new();
        let mut prev: HashMap<String, PathEdge> = HashMap::new();
        let mut frontier: Vec<(String, f64)> = vec![(source_id.clone(), 0.0)];
        dist.insert(source_id.clone(), 0.0);
        hop_count.insert(source_id.clone(), 0);

        while !frontier.is_empty() {
            let mut min_idx = 0usize;
            for i in 1..frontier.len() {
                if frontier[i].1 < frontier[min_idx].1 {
                    min_idx = i;
                }
            }
            let (node, cost_here) = frontier.swap_remove(min_idx);
            let best = *dist.get(&node).unwrap_or(&f64::INFINITY);
            if cost_here > best {
                continue;
            }
            if node == target_id {
                break;
            }

            let current_hops = *hop_count.get(&node).unwrap_or(&0);
            if current_hops >= max_hops {
                continue;
            }

            for edge in adjacency.get(&node).cloned().unwrap_or_default() {
                let next = edge.to_node_id.clone();
                let next_hops = current_hops + 1;
                if next_hops > max_hops {
                    continue;
                }
                let candidate = cost_here + edge.cost;
                let current_best = *dist.get(&next).unwrap_or(&f64::INFINITY);
                if candidate + 1e-9 < current_best {
                    dist.insert(next.clone(), candidate);
                    hop_count.insert(next.clone(), next_hops);
                    prev.insert(next.clone(), edge);
                    frontier.push((next, candidate));
                }
            }
        }

        let Some(total_cost) = dist.get(&target_id).copied() else {
            return Ok(ComputePathResponse {
                found: false,
                source_node_id: source_id.clone(),
                target_node_id: target_id.clone(),
                node_ids: vec![],
                link_ids: vec![],
                hops: vec![],
                total_cost: None,
                total_distance_m: None,
            });
        };

        let mut reversed: Vec<PathEdge> = Vec::new();
        let mut cursor = target_id.clone();
        while cursor != source_id {
            let Some(step) = prev.get(&cursor).cloned() else {
                return Ok(ComputePathResponse {
                    found: false,
                    source_node_id: source_id.clone(),
                    target_node_id: target_id.clone(),
                    node_ids: vec![],
                    link_ids: vec![],
                    hops: vec![],
                    total_cost: None,
                    total_distance_m: None,
                });
            };
            cursor = step.from_node_id.clone();
            reversed.push(step);
        }
        reversed.reverse();

        let mut node_ids = vec![source_id.clone()];
        let mut link_ids = Vec::with_capacity(reversed.len());
        let mut hops = Vec::with_capacity(reversed.len());
        let mut total_distance = 0.0;

        for (idx, step) in reversed.into_iter().enumerate() {
            total_distance += step.distance_m;
            link_ids.push(step.link_id.clone());
            node_ids.push(step.to_node_id.clone());
            hops.push(ComputedPathHop {
                seq_no: idx as i32 + 1,
                link_id: step.link_id,
                from_node_id: step.from_node_id,
                to_node_id: step.to_node_id,
                name: step.name,
                link_type: step.link_type,
                status: step.status,
                distance_m: step.distance_m,
                cost: step.cost,
            });
        }

        Ok(ComputePathResponse {
            found: true,
            source_node_id: source_id,
            target_node_id: target_id,
            node_ids,
            link_ids,
            hops,
            total_cost: Some(total_cost),
            total_distance_m: Some(total_distance),
        })
    }

    pub async fn list_nodes(
        &self,
        actor_id: &str,
        tenant_id: &str,
        q: ListQuery,
    ) -> AppResult<PaginatedResponse<NetworkNode>> {
        self.require_installation_read(actor_id, tenant_id).await?;
        let search = Self::cleaned_query(q.q);
        let page = q.page.max(1);
        let per_page = q.per_page.clamp(1, 200);
        let offset = (page - 1) * per_page;
        let (min_lng, min_lat, max_lng, max_lat) = q.bbox.unwrap_or((0.0, 0.0, 0.0, 0.0));
        let has_bbox = q.bbox.is_some();

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM network_nodes n
            WHERE n.tenant_id = $1::uuid
              AND ($2 = '' OR n.name ILIKE '%' || $2 || '%')
              AND ($3::text IS NULL OR n.node_type = $3)
              AND ($4::text IS NULL OR n.status = $4)
              AND (
                $5::bool = false
                OR ST_Intersects(n.geom, ST_MakeEnvelope($6, $7, $8, $9, 4326))
              )
            "#,
        )
        .bind(tenant_id)
        .bind(&search)
        .bind(&q.kind)
        .bind(&q.status)
        .bind(has_bbox)
        .bind(min_lng)
        .bind(min_lat)
        .bind(max_lng)
        .bind(max_lat)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let data: Vec<NetworkNode> = sqlx::query_as(
            r#"
            SELECT
              n.id::text AS id,
              n.tenant_id::text AS tenant_id,
              n.name,
              n.node_type,
              n.status,
              ST_Y(n.geom)::float8 AS lat,
              ST_X(n.geom)::float8 AS lng,
              n.capacity_json,
              n.health_json,
              n.metadata,
              n.created_at,
              n.updated_at
            FROM network_nodes n
            WHERE n.tenant_id = $1::uuid
              AND ($2 = '' OR n.name ILIKE '%' || $2 || '%')
              AND ($3::text IS NULL OR n.node_type = $3)
              AND ($4::text IS NULL OR n.status = $4)
              AND (
                $5::bool = false
                OR ST_Intersects(n.geom, ST_MakeEnvelope($6, $7, $8, $9, 4326))
              )
            ORDER BY n.updated_at DESC
            LIMIT $10 OFFSET $11
            "#,
        )
        .bind(tenant_id)
        .bind(&search)
        .bind(&q.kind)
        .bind(&q.status)
        .bind(has_bbox)
        .bind(min_lng)
        .bind(min_lat)
        .bind(max_lng)
        .bind(max_lat)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(PaginatedResponse {
            data,
            total,
            page,
            per_page,
        })
    }

    pub async fn create_node(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateNetworkNodeRequest,
    ) -> AppResult<NetworkNode> {
        self.require_installation_manage(actor_id, tenant_id)
            .await?;
        if dto.name.trim().is_empty() {
            return Err(AppError::Validation("name is required".into()));
        }
        Self::validate_lat_lng(dto.lat, dto.lng, "node")?;
        let id = Uuid::new_v4().to_string();
        let status = dto.status.unwrap_or_else(|| "active".to_string());
        let cap = dto.capacity_json.unwrap_or_else(|| serde_json::json!({}));
        let health = dto.health_json.unwrap_or_else(|| serde_json::json!({}));
        let meta = dto.metadata.unwrap_or_else(|| serde_json::json!({}));

        sqlx::query(
            r#"
            INSERT INTO network_nodes
              (id, tenant_id, name, node_type, status, geom, capacity_json, health_json, metadata, created_at, updated_at)
            VALUES
              ($1::uuid, $2::uuid, $3, $4, $5, ST_SetSRID(ST_MakePoint($6, $7), 4326), $8, $9, $10, now(), now())
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(dto.name.trim())
        .bind(dto.node_type)
        .bind(status)
        .bind(dto.lng)
        .bind(dto.lat)
        .bind(cap)
        .bind(health)
        .bind(meta)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.get_node_by_id(tenant_id, &id).await
    }

    pub async fn update_node(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        dto: UpdateNetworkNodeRequest,
    ) -> AppResult<NetworkNode> {
        self.require_manage(actor_id, tenant_id).await?;
        let current = self.get_node_by_id(tenant_id, id).await?;
        if Self::is_system_managed_node(&current.metadata) {
            let source =
                Self::system_managed_node_source_label(&current.metadata).unwrap_or("source asset");
            return Err(AppError::Validation(format!(
                "This node is synced from {source}. Update the source map coordinates instead."
            )));
        }
        let name = dto.name.unwrap_or(current.name);
        let node_type = dto.node_type.unwrap_or(current.node_type);
        let status = dto.status.unwrap_or(current.status);
        let lat = dto.lat.unwrap_or(current.lat);
        let lng = dto.lng.unwrap_or(current.lng);
        Self::validate_lat_lng(lat, lng, "node")?;
        let capacity_json = dto.capacity_json.unwrap_or(current.capacity_json);
        let health_json = dto.health_json.unwrap_or(current.health_json);
        let metadata = dto.metadata.unwrap_or(current.metadata);

        sqlx::query(
            r#"
            UPDATE network_nodes
            SET name = $1,
                node_type = $2,
                status = $3,
                geom = ST_SetSRID(ST_MakePoint($4, $5), 4326),
                capacity_json = $6,
                health_json = $7,
                metadata = $8
            WHERE tenant_id = $9::uuid AND id = $10::uuid
            "#,
        )
        .bind(name)
        .bind(node_type)
        .bind(status)
        .bind(lng)
        .bind(lat)
        .bind(capacity_json)
        .bind(health_json)
        .bind(metadata)
        .bind(tenant_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.get_node_by_id(tenant_id, id).await
    }

    pub async fn delete_node(&self, actor_id: &str, tenant_id: &str, id: &str) -> AppResult<()> {
        self.require_manage(actor_id, tenant_id).await?;
        let current = self.get_node_by_id(tenant_id, id).await?;
        if Self::is_system_managed_node(&current.metadata) {
            let source =
                Self::system_managed_node_source_label(&current.metadata).unwrap_or("source asset");
            return Err(AppError::Validation(format!(
                "This node is synced from {source} and cannot be deleted here."
            )));
        }
        let res =
            sqlx::query("DELETE FROM network_nodes WHERE tenant_id = $1::uuid AND id = $2::uuid")
                .bind(tenant_id)
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Node not found".into()));
        }
        Ok(())
    }

    pub async fn list_links(
        &self,
        actor_id: &str,
        tenant_id: &str,
        q: ListQuery,
    ) -> AppResult<PaginatedResponse<NetworkLink>> {
        self.require_installation_read(actor_id, tenant_id).await?;
        let search = Self::cleaned_query(q.q);
        let status_filter = q.status.as_deref().map(Self::normalize_link_status);
        let page = q.page.max(1);
        let per_page = q.per_page.clamp(1, 200);
        let offset = (page - 1) * per_page;
        let (min_lng, min_lat, max_lng, max_lat) = q.bbox.unwrap_or((0.0, 0.0, 0.0, 0.0));
        let has_bbox = q.bbox.is_some();

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM network_links l
            WHERE l.tenant_id = $1::uuid
              AND ($2 = '' OR l.name ILIKE '%' || $2 || '%')
              AND ($3::text IS NULL OR l.link_type = $3)
              AND ($4::text IS NULL OR l.status = $4)
              AND (
                $5::bool = false
                OR ST_Intersects(l.geom, ST_MakeEnvelope($6, $7, $8, $9, 4326))
              )
            "#,
        )
        .bind(tenant_id)
        .bind(&search)
        .bind(&q.kind)
        .bind(&status_filter)
        .bind(has_bbox)
        .bind(min_lng)
        .bind(min_lat)
        .bind(max_lng)
        .bind(max_lat)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let data: Vec<NetworkLink> = sqlx::query_as(
            r#"
            SELECT
              l.id::text AS id,
              l.tenant_id::text AS tenant_id,
              l.from_node_id::text AS from_node_id,
              l.to_node_id::text AS to_node_id,
              l.name,
              l.link_type,
              l.status,
              l.priority,
              l.capacity_mbps::float8 AS capacity_mbps,
              l.utilization_pct::float8 AS utilization_pct,
              l.loss_db::float8 AS loss_db,
              l.latency_ms::float8 AS latency_ms,
              ST_AsGeoJSON(l.geom)::jsonb AS geometry,
              l.metadata,
              l.created_at,
              l.updated_at
            FROM network_links l
            WHERE l.tenant_id = $1::uuid
              AND ($2 = '' OR l.name ILIKE '%' || $2 || '%')
              AND ($3::text IS NULL OR l.link_type = $3)
              AND ($4::text IS NULL OR l.status = $4)
              AND (
                $5::bool = false
                OR ST_Intersects(l.geom, ST_MakeEnvelope($6, $7, $8, $9, 4326))
              )
            ORDER BY l.updated_at DESC
            LIMIT $10 OFFSET $11
            "#,
        )
        .bind(tenant_id)
        .bind(&search)
        .bind(&q.kind)
        .bind(&status_filter)
        .bind(has_bbox)
        .bind(min_lng)
        .bind(min_lat)
        .bind(max_lng)
        .bind(max_lat)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(PaginatedResponse {
            data,
            total,
            page,
            per_page,
        })
    }

    pub async fn create_link(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateNetworkLinkRequest,
    ) -> AppResult<NetworkLink> {
        self.require_installation_manage(actor_id, tenant_id)
            .await?;
        if dto.name.trim().is_empty() {
            return Err(AppError::Validation("name is required".into()));
        }
        Self::validate_geojson_geometry(
            &dto.geometry,
            &["LineString", "MultiLineString"],
            "geometry",
        )?;
        let id = Uuid::new_v4().to_string();
        let status = Self::normalize_link_status(dto.status.as_deref().unwrap_or("up"));
        Self::validate_link_status(&status)?;
        self.ensure_link_pair_available(tenant_id, &dto.from_node_id, &dto.to_node_id, None)
            .await?;
        let priority = dto.priority.unwrap_or(100);
        let metadata = dto.metadata.unwrap_or_else(|| serde_json::json!({}));

        sqlx::query(
            r#"
            INSERT INTO network_links
              (id, tenant_id, from_node_id, to_node_id, name, link_type, status, priority,
               capacity_mbps, utilization_pct, loss_db, latency_ms, geom, metadata, created_at, updated_at)
            VALUES
              ($1::uuid, $2::uuid, $3::uuid, $4::uuid, $5, $6, $7, $8,
               $9, $10, $11, $12, ST_Multi(ST_SetSRID(ST_GeomFromGeoJSON($13), 4326)), $14, now(), now())
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(dto.from_node_id)
        .bind(dto.to_node_id)
        .bind(dto.name.trim())
        .bind(dto.link_type)
        .bind(status)
        .bind(priority)
        .bind(dto.capacity_mbps)
        .bind(dto.utilization_pct)
        .bind(dto.loss_db)
        .bind(dto.latency_ms)
        .bind(dto.geometry.to_string())
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::map_geometry_db_error(e, "geometry"))?;

        self.get_link_by_id(tenant_id, &id).await
    }

    pub async fn connect_node_to_link(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: ConnectNodeToLinkRequest,
    ) -> AppResult<ConnectNodeToLinkResponse> {
        self.require_installation_manage(actor_id, tenant_id)
            .await?;
        if dto.name.trim().is_empty() {
            return Err(AppError::Validation("name is required".into()));
        }
        Self::validate_lat_lng(dto.split_lat, dto.split_lng, "split_point")?;
        Self::validate_geojson_geometry(
            &dto.geometry,
            &["LineString", "MultiLineString"],
            "geometry",
        )?;

        let status = Self::normalize_link_status(dto.status.as_deref().unwrap_or("up"));
        Self::validate_link_status(&status)?;

        let source_node = self.get_node_by_id(tenant_id, &dto.source_node_id).await?;
        let target_link = self.get_link_by_id(tenant_id, &dto.target_link_id).await?;
        if source_node.id == target_link.from_node_id || source_node.id == target_link.to_node_id {
            return Err(AppError::Validation(
                "Source node already terminates the selected cable. Click the node instead.".into(),
            ));
        }

        let target_coords = Self::parse_line_coords(&target_link.geometry, "target_link.geometry")?;
        let snapped = Self::snap_point_to_polyline(&target_coords, dto.split_lng, dto.split_lat)
            .ok_or_else(|| AppError::Validation("Target link geometry is invalid".into()))?;
        let (updated_target_coords, created_target_coords) =
            Self::split_polyline_at_point(&target_coords, &snapped)?;

        let junction_node_type = dto
            .junction_node_type
            .as_deref()
            .unwrap_or("junction")
            .trim()
            .to_lowercase();
        if junction_node_type != "junction" && junction_node_type != "splitter" {
            return Err(AppError::Validation(
                "junction_node_type must be junction or splitter".into(),
            ));
        }

        let junction_id = Uuid::new_v4().to_string();
        let created_target_link_segment_id = Uuid::new_v4().to_string();
        let created_connection_link_id = Uuid::new_v4().to_string();
        let junction_name = dto
            .junction_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("Junction {}", &target_link.name));
        let priority = dto.priority.unwrap_or(100);

        let mut connection_metadata = dto.metadata.unwrap_or_else(|| serde_json::json!({}));
        if let Some(meta) = connection_metadata.as_object_mut() {
            meta.insert(
                "generated_by".into(),
                serde_json::Value::String("connect_node_to_link".into()),
            );
            meta.insert(
                "junction_node_id".into(),
                serde_json::Value::String(junction_id.clone()),
            );
            meta.insert(
                "target_link_id".into(),
                serde_json::Value::String(target_link.id.clone()),
            );
            meta.insert(
                "source_node_id".into(),
                serde_json::Value::String(source_node.id.clone()),
            );
        }

        let mut created_target_segment_metadata = target_link.metadata.clone();
        if let Some(meta) = created_target_segment_metadata.as_object_mut() {
            meta.insert(
                "generated_by".into(),
                serde_json::Value::String("connect_node_to_link".into()),
            );
            meta.insert(
                "split_parent_link_id".into(),
                serde_json::Value::String(target_link.id.clone()),
            );
            meta.insert(
                "junction_node_id".into(),
                serde_json::Value::String(junction_id.clone()),
            );
        }

        let mut updated_target_link_metadata = target_link.metadata.clone();
        if let Some(meta) = updated_target_link_metadata.as_object_mut() {
            meta.insert(
                "last_split_junction_node_id".into(),
                serde_json::Value::String(junction_id.clone()),
            );
            meta.insert(
                "last_split_child_link_id".into(),
                serde_json::Value::String(created_target_link_segment_id.clone()),
            );
        }

        let junction_metadata = serde_json::json!({
            "generated_by": "connect_node_to_link",
            "generated_reason": "link_split",
            "target_link_id": target_link.id.clone(),
            "source_node_id": source_node.id.clone(),
        });

        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        sqlx::query(
            r#"
            INSERT INTO network_nodes
              (id, tenant_id, name, node_type, status, geom, capacity_json, health_json, metadata, created_at, updated_at)
            VALUES
              ($1::uuid, $2::uuid, $3, $4, 'active', ST_SetSRID(ST_MakePoint($5, $6), 4326), '{}'::jsonb, '{}'::jsonb, $7, now(), now())
            "#,
        )
        .bind(&junction_id)
        .bind(tenant_id)
        .bind(&junction_name)
        .bind(&junction_node_type)
        .bind(snapped.lng)
        .bind(snapped.lat)
        .bind(junction_metadata)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        sqlx::query(
            r#"
            UPDATE network_links
            SET to_node_id = $1::uuid,
                geom = ST_Multi(ST_SetSRID(ST_GeomFromGeoJSON($2), 4326)),
                metadata = $3
            WHERE tenant_id = $4::uuid AND id = $5::uuid
            "#,
        )
        .bind(&junction_id)
        .bind(Self::build_line_geometry(&updated_target_coords).to_string())
        .bind(updated_target_link_metadata)
        .bind(tenant_id)
        .bind(&target_link.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Self::map_geometry_db_error(e, "target_link.geometry"))?;

        sqlx::query(
            r#"
            INSERT INTO network_links
              (id, tenant_id, from_node_id, to_node_id, name, link_type, status, priority,
               capacity_mbps, utilization_pct, loss_db, latency_ms, geom, metadata, created_at, updated_at)
            VALUES
              ($1::uuid, $2::uuid, $3::uuid, $4::uuid, $5, $6, $7, $8,
               $9, $10, $11, $12, ST_Multi(ST_SetSRID(ST_GeomFromGeoJSON($13), 4326)), $14, now(), now())
            "#,
        )
        .bind(&created_target_link_segment_id)
        .bind(tenant_id)
        .bind(&junction_id)
        .bind(&target_link.to_node_id)
        .bind(format!("{} (segment)", target_link.name.trim()))
        .bind(&target_link.link_type)
        .bind(&target_link.status)
        .bind(target_link.priority)
        .bind(target_link.capacity_mbps)
        .bind(target_link.utilization_pct)
        .bind(target_link.loss_db)
        .bind(target_link.latency_ms)
        .bind(Self::build_line_geometry(&created_target_coords).to_string())
        .bind(created_target_segment_metadata)
        .execute(&mut *tx)
        .await
        .map_err(|e| Self::map_geometry_db_error(e, "target_link.geometry"))?;

        sqlx::query(
            r#"
            INSERT INTO network_links
              (id, tenant_id, from_node_id, to_node_id, name, link_type, status, priority,
               capacity_mbps, utilization_pct, loss_db, latency_ms, geom, metadata, created_at, updated_at)
            VALUES
              ($1::uuid, $2::uuid, $3::uuid, $4::uuid, $5, $6, $7, $8,
               $9, $10, $11, $12, ST_Multi(ST_SetSRID(ST_GeomFromGeoJSON($13), 4326)), $14, now(), now())
            "#,
        )
        .bind(&created_connection_link_id)
        .bind(tenant_id)
        .bind(&source_node.id)
        .bind(&junction_id)
        .bind(dto.name.trim())
        .bind(&dto.link_type)
        .bind(status)
        .bind(priority)
        .bind(dto.capacity_mbps)
        .bind(dto.utilization_pct)
        .bind(dto.loss_db)
        .bind(dto.latency_ms)
        .bind(dto.geometry.to_string())
        .bind(connection_metadata)
        .execute(&mut *tx)
        .await
        .map_err(|e| Self::map_geometry_db_error(e, "geometry"))?;

        tx.commit().await.map_err(AppError::Database)?;

        Ok(ConnectNodeToLinkResponse {
            junction_node: self.get_node_by_id(tenant_id, &junction_id).await?,
            updated_target_link: self.get_link_by_id(tenant_id, &target_link.id).await?,
            created_target_link_segment: self
                .get_link_by_id(tenant_id, &created_target_link_segment_id)
                .await?,
            created_connection_link: self
                .get_link_by_id(tenant_id, &created_connection_link_id)
                .await?,
        })
    }

    pub async fn update_link(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        dto: UpdateNetworkLinkRequest,
    ) -> AppResult<NetworkLink> {
        self.require_manage(actor_id, tenant_id).await?;
        let current = self.get_link_by_id(tenant_id, id).await?;

        let geometry = dto.geometry.unwrap_or(current.geometry);
        Self::validate_geojson_geometry(&geometry, &["LineString", "MultiLineString"], "geometry")?;
        let status =
            Self::normalize_link_status(dto.status.as_deref().unwrap_or(current.status.as_str()));
        Self::validate_link_status(&status)?;
        let next_from_node_id = dto
            .from_node_id
            .clone()
            .unwrap_or_else(|| current.from_node_id.clone());
        let next_to_node_id = dto
            .to_node_id
            .clone()
            .unwrap_or_else(|| current.to_node_id.clone());
        self.ensure_link_pair_available(tenant_id, &next_from_node_id, &next_to_node_id, Some(id))
            .await?;

        sqlx::query(
            r#"
            UPDATE network_links
            SET from_node_id = $1::uuid,
                to_node_id = $2::uuid,
                name = $3,
                link_type = $4,
                status = $5,
                priority = $6,
                capacity_mbps = $7,
                utilization_pct = $8,
                loss_db = $9,
                latency_ms = $10,
                geom = ST_Multi(ST_SetSRID(ST_GeomFromGeoJSON($11), 4326)),
                metadata = $12
            WHERE tenant_id = $13::uuid AND id = $14::uuid
            "#,
        )
        .bind(dto.from_node_id.unwrap_or(current.from_node_id))
        .bind(dto.to_node_id.unwrap_or(current.to_node_id))
        .bind(dto.name.unwrap_or(current.name))
        .bind(dto.link_type.unwrap_or(current.link_type))
        .bind(status)
        .bind(dto.priority.unwrap_or(current.priority))
        .bind(dto.capacity_mbps.or(current.capacity_mbps))
        .bind(dto.utilization_pct.or(current.utilization_pct))
        .bind(dto.loss_db.or(current.loss_db))
        .bind(dto.latency_ms.or(current.latency_ms))
        .bind(geometry.to_string())
        .bind(dto.metadata.unwrap_or(current.metadata))
        .bind(tenant_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::map_geometry_db_error(e, "geometry"))?;

        self.get_link_by_id(tenant_id, id).await
    }

    pub async fn delete_link(&self, actor_id: &str, tenant_id: &str, id: &str) -> AppResult<()> {
        self.require_manage(actor_id, tenant_id).await?;
        let res =
            sqlx::query("DELETE FROM network_links WHERE tenant_id = $1::uuid AND id = $2::uuid")
                .bind(tenant_id)
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Link not found".into()));
        }
        Ok(())
    }

    pub async fn list_zones(
        &self,
        actor_id: &str,
        tenant_id: &str,
        q: ListQuery,
    ) -> AppResult<PaginatedResponse<ServiceZone>> {
        self.require_zones_read(actor_id, tenant_id).await?;
        let search = Self::cleaned_query(q.q);
        let page = q.page.max(1);
        let per_page = q.per_page.clamp(1, 200);
        let offset = (page - 1) * per_page;
        let (min_lng, min_lat, max_lng, max_lat) = q.bbox.unwrap_or((0.0, 0.0, 0.0, 0.0));
        let has_bbox = q.bbox.is_some();

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM service_zones z
            WHERE z.tenant_id = $1::uuid
              AND ($2 = '' OR z.name ILIKE '%' || $2 || '%')
              AND ($3::text IS NULL OR z.zone_type = $3)
              AND ($4::text IS NULL OR z.status = $4)
              AND (
                $5::bool = false
                OR ST_Intersects(z.geom, ST_MakeEnvelope($6, $7, $8, $9, 4326))
              )
            "#,
        )
        .bind(tenant_id)
        .bind(&search)
        .bind(&q.kind)
        .bind(&q.status)
        .bind(has_bbox)
        .bind(min_lng)
        .bind(min_lat)
        .bind(max_lng)
        .bind(max_lat)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let data: Vec<ServiceZone> = sqlx::query_as(
            r#"
            SELECT
              z.id::text AS id,
              z.tenant_id::text AS tenant_id,
              z.name,
              z.zone_type,
              z.priority,
              z.status,
              ST_AsGeoJSON(z.geom)::jsonb AS geometry,
              z.metadata,
              z.created_at,
              z.updated_at
            FROM service_zones z
            WHERE z.tenant_id = $1::uuid
              AND ($2 = '' OR z.name ILIKE '%' || $2 || '%')
              AND ($3::text IS NULL OR z.zone_type = $3)
              AND ($4::text IS NULL OR z.status = $4)
              AND (
                $5::bool = false
                OR ST_Intersects(z.geom, ST_MakeEnvelope($6, $7, $8, $9, 4326))
              )
            ORDER BY z.priority ASC, z.updated_at DESC
            LIMIT $10 OFFSET $11
            "#,
        )
        .bind(tenant_id)
        .bind(&search)
        .bind(&q.kind)
        .bind(&q.status)
        .bind(has_bbox)
        .bind(min_lng)
        .bind(min_lat)
        .bind(max_lng)
        .bind(max_lat)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(PaginatedResponse {
            data,
            total,
            page,
            per_page,
        })
    }

    pub async fn create_zone(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateServiceZoneRequest,
    ) -> AppResult<ServiceZone> {
        self.require_zones_manage(actor_id, tenant_id).await?;
        if dto.name.trim().is_empty() {
            return Err(AppError::Validation("name is required".into()));
        }
        Self::validate_geojson_geometry(&dto.geometry, &["Polygon", "MultiPolygon"], "geometry")?;
        let id = Uuid::new_v4().to_string();
        let status = dto.status.unwrap_or_else(|| "active".to_string());
        let priority = dto.priority.unwrap_or(100);
        let metadata = dto.metadata.unwrap_or_else(|| serde_json::json!({}));
        sqlx::query(
            r#"
            INSERT INTO service_zones
              (id, tenant_id, name, zone_type, priority, status, geom, metadata, created_at, updated_at)
            VALUES
              ($1::uuid, $2::uuid, $3, $4, $5, $6, ST_Multi(ST_SetSRID(ST_GeomFromGeoJSON($7), 4326)), $8, now(), now())
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(dto.name.trim())
        .bind(dto.zone_type)
        .bind(priority)
        .bind(status)
        .bind(dto.geometry.to_string())
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::map_geometry_db_error(e, "geometry"))?;
        self.get_zone_by_id(tenant_id, &id).await
    }

    pub async fn update_zone(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        dto: UpdateServiceZoneRequest,
    ) -> AppResult<ServiceZone> {
        self.require_zones_manage(actor_id, tenant_id).await?;
        let current = self.get_zone_by_id(tenant_id, id).await?;
        let geometry = dto.geometry.unwrap_or(current.geometry);
        Self::validate_geojson_geometry(&geometry, &["Polygon", "MultiPolygon"], "geometry")?;

        sqlx::query(
            r#"
            UPDATE service_zones
            SET name = $1,
                zone_type = $2,
                priority = $3,
                status = $4,
                geom = ST_Multi(ST_SetSRID(ST_GeomFromGeoJSON($5), 4326)),
                metadata = $6
            WHERE tenant_id = $7::uuid AND id = $8::uuid
            "#,
        )
        .bind(dto.name.unwrap_or(current.name))
        .bind(dto.zone_type.unwrap_or(current.zone_type))
        .bind(dto.priority.unwrap_or(current.priority))
        .bind(dto.status.unwrap_or(current.status))
        .bind(geometry.to_string())
        .bind(dto.metadata.unwrap_or(current.metadata))
        .bind(tenant_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::map_geometry_db_error(e, "geometry"))?;
        self.get_zone_by_id(tenant_id, id).await
    }

    pub async fn delete_zone(&self, actor_id: &str, tenant_id: &str, id: &str) -> AppResult<()> {
        self.require_zones_manage(actor_id, tenant_id).await?;
        let res =
            sqlx::query("DELETE FROM service_zones WHERE tenant_id = $1::uuid AND id = $2::uuid")
                .bind(tenant_id)
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Zone not found".into()));
        }
        Ok(())
    }

    pub async fn list_zone_bindings(
        &self,
        actor_id: &str,
        tenant_id: &str,
        zone_id: Option<String>,
    ) -> AppResult<Vec<ZoneNodeBinding>> {
        self.require_zones_read(actor_id, tenant_id).await?;
        let rows: Vec<ZoneNodeBinding> = sqlx::query_as(
            r#"
            SELECT
              id::text AS id,
              tenant_id::text AS tenant_id,
              zone_id::text AS zone_id,
              node_id::text AS node_id,
              is_primary,
              weight,
              created_at
            FROM zone_node_bindings
            WHERE tenant_id = $1::uuid
              AND ($2::uuid IS NULL OR zone_id = $2::uuid)
            ORDER BY zone_id, is_primary DESC, weight ASC, created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(zone_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(rows)
    }

    pub async fn create_zone_binding(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateZoneNodeBindingRequest,
    ) -> AppResult<ZoneNodeBinding> {
        self.require_zones_manage(actor_id, tenant_id).await?;
        let id = Uuid::new_v4().to_string();
        let is_primary = dto.is_primary.unwrap_or(false);
        let weight = dto.weight.unwrap_or(100);

        sqlx::query(
            r#"
            INSERT INTO zone_node_bindings (id, tenant_id, zone_id, node_id, is_primary, weight, created_at)
            VALUES ($1::uuid, $2::uuid, $3::uuid, $4::uuid, $5, $6, now())
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(dto.zone_id)
        .bind(dto.node_id)
        .bind(is_primary)
        .bind(weight)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let row: ZoneNodeBinding = sqlx::query_as(
            r#"
            SELECT
              id::text AS id,
              tenant_id::text AS tenant_id,
              zone_id::text AS zone_id,
              node_id::text AS node_id,
              is_primary,
              weight,
              created_at
            FROM zone_node_bindings
            WHERE tenant_id = $1::uuid AND id = $2::uuid
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(row)
    }

    pub async fn delete_zone_binding(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<()> {
        self.require_zones_manage(actor_id, tenant_id).await?;
        let res = sqlx::query(
            "DELETE FROM zone_node_bindings WHERE tenant_id = $1::uuid AND id = $2::uuid",
        )
        .bind(tenant_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Zone-node binding not found".into()));
        }
        Ok(())
    }

    pub async fn list_zone_offers(
        &self,
        actor_id: &str,
        tenant_id: &str,
        zone_id: Option<String>,
        package_id: Option<String>,
        active_only: bool,
    ) -> AppResult<Vec<ZoneOffer>> {
        self.require_coverage_read(actor_id, tenant_id).await?;
        let rows: Vec<ZoneOffer> = sqlx::query_as(
            r#"
            SELECT
              id::text AS id,
              tenant_id::text AS tenant_id,
              zone_id::text AS zone_id,
              package_id,
              price_monthly::float8 AS price_monthly,
              price_yearly::float8 AS price_yearly,
              is_active,
              metadata,
              created_at,
              updated_at
            FROM zone_offers
            WHERE tenant_id = $1::uuid
              AND ($2::uuid IS NULL OR zone_id = $2::uuid)
              AND ($3::text IS NULL OR package_id = $3)
              AND ($4::bool = false OR is_active = true)
            ORDER BY updated_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(zone_id)
        .bind(package_id)
        .bind(active_only)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(rows)
    }

    pub async fn create_zone_offer(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateZoneOfferRequest,
    ) -> AppResult<ZoneOffer> {
        self.require_zones_manage(actor_id, tenant_id).await?;
        let id = Uuid::new_v4().to_string();
        let is_active = dto.is_active.unwrap_or(true);
        let metadata = dto.metadata.unwrap_or_else(|| serde_json::json!({}));

        sqlx::query(
            r#"
            INSERT INTO zone_offers
              (id, tenant_id, zone_id, package_id, price_monthly, price_yearly, is_active, metadata, created_at, updated_at)
            VALUES
              ($1::uuid, $2::uuid, $3::uuid, $4, $5, $6, $7, $8, now(), now())
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(dto.zone_id)
        .bind(dto.package_id)
        .bind(dto.price_monthly)
        .bind(dto.price_yearly)
        .bind(is_active)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.get_zone_offer_by_id(tenant_id, &id).await
    }

    pub async fn update_zone_offer(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        dto: UpdateZoneOfferRequest,
    ) -> AppResult<ZoneOffer> {
        self.require_zones_manage(actor_id, tenant_id).await?;
        let current = self.get_zone_offer_by_id(tenant_id, id).await?;

        sqlx::query(
            r#"
            UPDATE zone_offers
            SET zone_id = $1::uuid,
                package_id = $2,
                price_monthly = $3,
                price_yearly = $4,
                is_active = $5,
                metadata = $6,
                updated_at = now()
            WHERE tenant_id = $7::uuid AND id = $8::uuid
            "#,
        )
        .bind(dto.zone_id.unwrap_or(current.zone_id))
        .bind(dto.package_id.unwrap_or(current.package_id))
        .bind(dto.price_monthly.or(current.price_monthly))
        .bind(dto.price_yearly.or(current.price_yearly))
        .bind(dto.is_active.unwrap_or(current.is_active))
        .bind(dto.metadata.unwrap_or(current.metadata))
        .bind(tenant_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.get_zone_offer_by_id(tenant_id, id).await
    }

    pub async fn delete_zone_offer(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<()> {
        self.require_zones_manage(actor_id, tenant_id).await?;
        let res =
            sqlx::query("DELETE FROM zone_offers WHERE tenant_id = $1::uuid AND id = $2::uuid")
                .bind(tenant_id)
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Zone offer not found".into()));
        }
        Ok(())
    }

    pub async fn resolve_zone(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: ResolveZoneRequest,
    ) -> AppResult<ResolvedZoneResponse> {
        self.require_coverage_read(actor_id, tenant_id).await?;
        let zone: Option<ResolvedZone> = sqlx::query_as(
            r#"
            SELECT id::text AS id, name, priority
            FROM service_zones
            WHERE tenant_id = $1::uuid
              AND status = 'active'
              AND ST_Contains(geom, ST_SetSRID(ST_MakePoint($2, $3), 4326))
            ORDER BY priority ASC, updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(dto.lng)
        .bind(dto.lat)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(ResolvedZoneResponse { zone })
    }

    pub async fn coverage_check(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CoverageCheckRequest,
    ) -> AppResult<CoverageCheckResponse> {
        self.require_coverage_read(actor_id, tenant_id).await?;
        let zone: Option<ResolvedZone> = sqlx::query_as(
            r#"
            SELECT id::text AS id, name, priority
            FROM service_zones
            WHERE tenant_id = $1::uuid
              AND status = 'active'
              AND ST_Contains(geom, ST_SetSRID(ST_MakePoint($2, $3), 4326))
            ORDER BY priority ASC, updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(dto.lng)
        .bind(dto.lat)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let offers: Vec<ZoneOffer> = if let Some(z) = &zone {
            sqlx::query_as(
                r#"
                SELECT
                  id::text AS id,
                  tenant_id::text AS tenant_id,
                  zone_id::text AS zone_id,
                  package_id,
                  price_monthly::float8 AS price_monthly,
                  price_yearly::float8 AS price_yearly,
                  is_active,
                  metadata,
                  created_at,
                  updated_at
                FROM zone_offers
                WHERE tenant_id = $1::uuid
                  AND zone_id = $2::uuid
                  AND is_active = true
                ORDER BY updated_at DESC
                "#,
            )
            .bind(tenant_id)
            .bind(&z.id)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?
        } else {
            vec![]
        };

        Ok(CoverageCheckResponse { zone, offers })
    }

    pub async fn list_impacted_customers(
        &self,
        actor_id: &str,
        tenant_id: &str,
        node_id: Option<String>,
        link_id: Option<String>,
        router_id: Option<String>,
    ) -> AppResult<NetworkImpactResponse> {
        self.require_read(actor_id, tenant_id).await?;

        let mut node_ids = HashSet::<String>::new();
        let mut link_ids = HashSet::<String>::new();

        if let Some(id) = node_id.filter(|v| !v.trim().is_empty()) {
            node_ids.insert(id.trim().to_string());
        }
        if let Some(id) = link_id.filter(|v| !v.trim().is_empty()) {
            link_ids.insert(id.trim().to_string());
        }

        if let Some(router_id) = router_id.filter(|v| !v.trim().is_empty()) {
            let router_id = router_id.trim().to_string();
            let resolved_nodes: Vec<UuidTextRow> = sqlx::query_as(
                r#"
                SELECT id::text AS id
                FROM network_nodes
                WHERE tenant_id = $1::uuid
                  AND (
                    id::text = $2::text
                    OR metadata->>'router_id' = $2::text
                    OR metadata->>'routerId' = $2::text
                    OR metadata->>'mikrotik_router_id' = $2::text
                    OR metadata->>'mikrotikRouterId' = $2::text
                  )
                "#,
            )
            .bind(tenant_id)
            .bind(&router_id)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?;

            for row in resolved_nodes {
                node_ids.insert(row.id);
            }
        }

        let node_vec = node_ids.into_iter().collect::<Vec<_>>();

        if !node_vec.is_empty() {
            let connected_links: Vec<UuidTextRow> = sqlx::query_as(
                r#"
                SELECT id::text AS id
                FROM network_links
                WHERE tenant_id = $1::uuid
                  AND (
                    from_node_id::text = ANY($2::text[])
                    OR to_node_id::text = ANY($2::text[])
                  )
                "#,
            )
            .bind(tenant_id)
            .bind(&node_vec)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?;

            for row in connected_links {
                link_ids.insert(row.id);
            }
        }

        let link_vec = link_ids.into_iter().collect::<Vec<_>>();
        if node_vec.is_empty() && link_vec.is_empty() {
            return Ok(NetworkImpactResponse {
                node_ids: node_vec,
                link_ids: link_vec,
                customers: vec![],
            });
        }

        let rows: Vec<NetworkImpactCustomer> = sqlx::query_as(
            r#"
            SELECT
              csa.id::text AS assignment_id,
              csa.status AS assignment_status,
              csa.invoice_id::text AS invoice_id,
              csa.subscription_id::text AS subscription_id,
              cs.status AS subscription_status,
              wo.id::text AS work_order_id,
              wo.status AS work_order_status,
              c.id::text AS customer_id,
              c.name AS customer_name,
              cl.id::text AS location_id,
              cl.label AS location_label,
              csa.selected_node_id AS selected_node_id,
              nn.name AS selected_node_name,
              (
                ($2::text[] IS NOT NULL AND csa.selected_node_id = ANY($2::text[]))
                OR ($2::text[] IS NOT NULL AND EXISTS (
                  SELECT 1
                  FROM jsonb_array_elements_text(csa.path_node_ids) n(node_id)
                  WHERE n.node_id = ANY($2::text[])
                ))
              ) AS impacted_via_node,
              (
                $3::text[] IS NOT NULL AND EXISTS (
                  SELECT 1
                  FROM jsonb_array_elements_text(csa.path_link_ids) l(link_id)
                  WHERE l.link_id = ANY($3::text[])
                )
              ) AS impacted_via_link,
              csa.updated_at AS updated_at
            FROM customer_service_assignments csa
            JOIN customers c
              ON c.tenant_id = csa.tenant_id AND c.id = csa.customer_id
            JOIN customer_locations cl
              ON cl.tenant_id = csa.tenant_id AND cl.id = csa.location_id
            LEFT JOIN customer_subscriptions cs
              ON cs.tenant_id = csa.tenant_id AND cs.id = csa.subscription_id
            LEFT JOIN installation_work_orders wo
              ON wo.tenant_id = csa.tenant_id AND wo.id = csa.work_order_id
            LEFT JOIN network_nodes nn
              ON nn.tenant_id = csa.tenant_id::uuid AND nn.id::text = csa.selected_node_id
            WHERE csa.tenant_id = $1::text
              AND (
                ($2::text[] IS NOT NULL AND (
                  csa.selected_node_id = ANY($2::text[])
                  OR EXISTS (
                    SELECT 1
                    FROM jsonb_array_elements_text(csa.path_node_ids) n(node_id)
                    WHERE n.node_id = ANY($2::text[])
                  )
                ))
                OR
                ($3::text[] IS NOT NULL AND EXISTS (
                  SELECT 1
                  FROM jsonb_array_elements_text(csa.path_link_ids) l(link_id)
                  WHERE l.link_id = ANY($3::text[])
                ))
              )
            ORDER BY csa.updated_at DESC
            LIMIT 300
            "#,
        )
        .bind(tenant_id)
        .bind(if node_vec.is_empty() {
            None
        } else {
            Some(node_vec.clone())
        })
        .bind(if link_vec.is_empty() {
            None
        } else {
            Some(link_vec.clone())
        })
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(NetworkImpactResponse {
            node_ids: node_vec,
            link_ids: link_vec,
            customers: rows,
        })
    }

    async fn get_node_by_id(&self, tenant_id: &str, id: &str) -> AppResult<NetworkNode> {
        sqlx::query_as(
            r#"
            SELECT
              id::text AS id,
              tenant_id::text AS tenant_id,
              name,
              node_type,
              status,
              ST_Y(geom)::float8 AS lat,
              ST_X(geom)::float8 AS lng,
              capacity_json,
              health_json,
              metadata,
              created_at,
              updated_at
            FROM network_nodes
            WHERE tenant_id = $1::uuid AND id = $2::uuid
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Node not found".into()))
    }

    async fn get_link_by_id(&self, tenant_id: &str, id: &str) -> AppResult<NetworkLink> {
        sqlx::query_as(
            r#"
            SELECT
              id::text AS id,
              tenant_id::text AS tenant_id,
              from_node_id::text AS from_node_id,
              to_node_id::text AS to_node_id,
              name,
              link_type,
              status,
              priority,
              capacity_mbps::float8 AS capacity_mbps,
              utilization_pct::float8 AS utilization_pct,
              loss_db::float8 AS loss_db,
              latency_ms::float8 AS latency_ms,
              ST_AsGeoJSON(geom)::jsonb AS geometry,
              metadata,
              created_at,
              updated_at
            FROM network_links
            WHERE tenant_id = $1::uuid AND id = $2::uuid
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Link not found".into()))
    }

    async fn get_zone_by_id(&self, tenant_id: &str, id: &str) -> AppResult<ServiceZone> {
        sqlx::query_as(
            r#"
            SELECT
              id::text AS id,
              tenant_id::text AS tenant_id,
              name,
              zone_type,
              priority,
              status,
              ST_AsGeoJSON(geom)::jsonb AS geometry,
              metadata,
              created_at,
              updated_at
            FROM service_zones
            WHERE tenant_id = $1::uuid AND id = $2::uuid
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Zone not found".into()))
    }

    async fn get_zone_offer_by_id(&self, tenant_id: &str, id: &str) -> AppResult<ZoneOffer> {
        sqlx::query_as(
            r#"
            SELECT
              id::text AS id,
              tenant_id::text AS tenant_id,
              zone_id::text AS zone_id,
              package_id,
              price_monthly::float8 AS price_monthly,
              price_yearly::float8 AS price_yearly,
              is_active,
              metadata,
              created_at,
              updated_at
            FROM zone_offers
            WHERE tenant_id = $1::uuid AND id = $2::uuid
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Zone offer not found".into()))
    }
}
