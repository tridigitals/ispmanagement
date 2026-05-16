mod core;
mod dto;
mod integration;
mod mapper;
mod repository;
mod validation;

pub use dto::ListQuery;

use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    ComputePathRequest, ComputePathResponse, ConnectNodeToLinkRequest, ConnectNodeToLinkResponse,
    CoverageCheckRequest, CoverageCheckResponse, CreateNetworkLinkRequest,
    CreateNetworkNodeRequest, CreateServiceZoneRequest, CreateZoneNodeBindingRequest,
    CreateZoneOfferRequest, NetworkImpactResponse, NetworkLink, NetworkNode, PaginatedResponse,
    RankCandidateNodesRequest, RankCandidateNodesResponse, ResolveZoneRequest, ResolvedZone,
    ResolvedZoneResponse, ServiceZone, SyncTopologyAssetsResponse, UpdateNetworkLinkRequest,
    UpdateNetworkNodeRequest, UpdateServiceZoneRequest, UpdateZoneOfferRequest, ZoneNodeBinding,
    ZoneOffer,
};
use crate::services::AuthService;
use uuid::Uuid;

#[derive(Clone)]
pub struct NetworkMappingService {
    pool: DbPool,
    auth_service: AuthService,
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
            &[("network_topology", "read"), ("router_inventory", "read")],
        )
        .await
    }

    async fn require_manage(&self, actor_id: &str, tenant_id: &str) -> AppResult<()> {
        self.check_permission_any(
            actor_id,
            tenant_id,
            &[
                ("network_topology", "manage"),
                ("router_inventory", "manage"),
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
                ("router_inventory", "read"),
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
                ("router_inventory", "manage"),
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
                ("router_inventory", "read"),
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
                ("router_inventory", "manage"),
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
                ("router_inventory", "read"),
            ],
        )
        .await
    }

    pub async fn sync_topology_asset_nodes(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<SyncTopologyAssetsResponse> {
        self.sync_topology_asset_nodes_flow(actor_id, tenant_id)
            .await
    }

    pub async fn rank_candidate_nodes(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: RankCandidateNodesRequest,
    ) -> AppResult<RankCandidateNodesResponse> {
        self.rank_candidate_nodes_flow(actor_id, tenant_id, dto)
            .await
    }

    pub async fn compute_path(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: ComputePathRequest,
    ) -> AppResult<ComputePathResponse> {
        self.compute_path_flow(actor_id, tenant_id, dto).await
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
        let include_legacy_ftth = q.include_legacy_ftth;

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
              AND (
                $10::bool = true
                OR LOWER(n.node_type) NOT IN ('olt', 'odc', 'odp', 'fat', 'nap', 'splitter')
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
        .bind(include_legacy_ftth)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut data: Vec<NetworkNode> = sqlx::query_as(
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
              AND (
                $10::bool = true
                OR LOWER(n.node_type) NOT IN ('olt', 'odc', 'odp', 'fat', 'nap', 'splitter')
              )
            ORDER BY n.updated_at DESC
            LIMIT $11 OFFSET $12
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
        .bind(include_legacy_ftth)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.overlay_live_customer_location_metadata(tenant_id, &mut data)
            .await?;

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
        Self::validate_manual_node_type(&dto.node_type)?;
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
        Self::validate_manual_node_type(&node_type)?;
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
        self.list_impacted_customers_flow(actor_id, tenant_id, node_id, link_id, router_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::dto::{CandidateNodeRow, PathLinkRow, SyncCustomerLocationRow};
    use super::NetworkMappingService;
    use crate::error::AppError;

    #[test]
    fn link_status_normalization_and_validation_contract() {
        assert_eq!(NetworkMappingService::normalize_link_status("active"), "up");
        assert_eq!(
            NetworkMappingService::normalize_link_status("inactive"),
            "down"
        );
        assert_eq!(
            NetworkMappingService::normalize_link_status(" maintenance "),
            "maintenance"
        );

        assert!(NetworkMappingService::validate_link_status("up").is_ok());
        assert!(NetworkMappingService::validate_link_status("degraded").is_ok());

        let err = NetworkMappingService::validate_link_status("offline").unwrap_err();
        match err {
            AppError::Validation(message) => {
                assert!(message.contains("link status must be one of"));
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn lat_lng_validation_contract_for_node_and_split_point_inputs() {
        assert!(NetworkMappingService::validate_lat_lng(0.0, 0.0, "node").is_ok());

        let lat_err = NetworkMappingService::validate_lat_lng(91.0, 0.0, "node").unwrap_err();
        let lng_err =
            NetworkMappingService::validate_lat_lng(0.0, 181.0, "split_point").unwrap_err();

        match lat_err {
            AppError::Validation(message) => {
                assert!(message.contains("node.lat must be between -90 and 90"));
            }
            other => panic!("expected validation error, got {other:?}"),
        }

        match lng_err {
            AppError::Validation(message) => {
                assert!(message.contains("split_point.lng must be between -180 and 180"));
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn geometry_validation_and_line_parsing_characterization() {
        let line = serde_json::json!({
            "type": "LineString",
            "coordinates": [[106.0, -6.0], [106.1, -6.1], [106.2, -6.2]],
        });
        assert!(NetworkMappingService::validate_geojson_geometry(
            &line,
            &["LineString", "MultiLineString"],
            "geometry"
        )
        .is_ok());

        let parsed = NetworkMappingService::parse_line_coords(&line, "geometry")
            .expect("LineString coordinates should parse");
        assert_eq!(parsed.len(), 3);

        let unsupported = serde_json::json!({
            "type": "Polygon",
            "coordinates": [[[106.0, -6.0], [106.1, -6.1], [106.2, -6.2], [106.0, -6.0]]],
        });
        let err = NetworkMappingService::parse_line_coords(&unsupported, "geometry").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn split_polyline_rejects_endpoint_and_accepts_mid_segment_split() {
        let coords = vec![[106.0, -6.0], [106.1, -6.1], [106.2, -6.2]];

        let endpoint_snap = NetworkMappingService::snap_point_to_polyline(&coords, 106.0, -6.0)
            .expect("endpoint snap should resolve");
        let endpoint_err =
            NetworkMappingService::split_polyline_at_point(&coords, &endpoint_snap).unwrap_err();
        match endpoint_err {
            AppError::Validation(message) => {
                assert!(message.contains("too close to an existing node"));
            }
            other => panic!("expected validation error, got {other:?}"),
        }

        let interior_snap = NetworkMappingService::snap_point_to_polyline(&coords, 106.15, -6.15)
            .expect("interior snap should resolve");
        let (first, second) =
            NetworkMappingService::split_polyline_at_point(&coords, &interior_snap)
                .expect("interior split should succeed");
        assert!(first.len() >= 2);
        assert!(second.len() >= 2);
        assert_eq!(first.last(), second.first());
    }

    #[test]
    fn path_cost_and_candidate_scoring_helpers_preserve_existing_shape() {
        let link = PathLinkRow {
            id: "link-1".into(),
            from_node_id: "node-a".into(),
            to_node_id: "node-b".into(),
            name: "Core Link".into(),
            link_type: "fiber".into(),
            status: "degraded".into(),
            distance_m: 2_000.0,
            utilization_pct: Some(40.0),
            loss_db: Some(1.2),
            latency_ms: Some(4.0),
        };

        let cost = NetworkMappingService::link_cost(&link);
        assert!(cost > 0.0);

        let health = NetworkMappingService::compute_health_score("active", &serde_json::json!({}));
        let capacity = NetworkMappingService::compute_capacity_score(
            &serde_json::json!({"available_mbps": 75, "total_mbps": 100}),
            None,
        );
        let distance = NetworkMappingService::compute_distance_score(Some(500.0));

        assert_eq!(health, 85.0);
        assert_eq!(capacity, 75.0);
        assert!(distance.is_some());
    }

    #[test]
    fn customer_subscription_status_mapping_contract() {
        assert_eq!(
            NetworkMappingService::customer_subscription_to_node_status("active"),
            "active"
        );
        assert_eq!(
            NetworkMappingService::customer_subscription_to_node_status("grace_active"),
            "active"
        );
        assert_eq!(
            NetworkMappingService::customer_subscription_to_node_status(
                "installation_done_awaiting_payment"
            ),
            "active"
        );
        assert_eq!(
            NetworkMappingService::customer_subscription_to_node_status("suspended"),
            "maintenance"
        );
        assert_eq!(
            NetworkMappingService::customer_subscription_to_node_status("inactive"),
            "inactive"
        );
        assert_eq!(
            NetworkMappingService::customer_subscription_to_node_status("cancelled"),
            "inactive"
        );
    }

    #[test]
    fn network_asset_status_mapping_contract() {
        assert_eq!(
            NetworkMappingService::network_asset_to_node_status("available"),
            "active"
        );
        assert_eq!(
            NetworkMappingService::network_asset_to_node_status("installed"),
            "active"
        );
        assert_eq!(
            NetworkMappingService::network_asset_to_node_status("faulty"),
            "maintenance"
        );
        assert_eq!(
            NetworkMappingService::network_asset_to_node_status("retired"),
            "inactive"
        );
    }

    #[test]
    fn system_managed_asset_reference_matching_contract() {
        let network_asset = serde_json::json!({
            "system_managed": true,
            "asset_source": "network_asset",
            "asset_type": "odp",
            "asset_id": "asset-1",
        });
        assert!(NetworkMappingService::system_managed_node_matches_asset_reference(
            &network_asset,
            "network_asset",
            "asset-1"
        ));
        assert!(NetworkMappingService::system_managed_node_matches_asset_source(
            &network_asset,
            "network_asset"
        ));
        assert!(!NetworkMappingService::system_managed_node_matches_asset_reference(
            &network_asset,
            "network_asset",
            "asset-2"
        ));

        let customer_location = serde_json::json!({
            "system_managed": true,
            "asset_source": "customer_location",
            "asset_type": "customer_location",
            "asset_id": "loc-1",
            "location_id": "loc-1",
        });
        assert!(NetworkMappingService::system_managed_node_matches_asset_reference(
            &customer_location,
            "customer_location",
            "loc-1"
        ));

        let router = serde_json::json!({
            "system_managed": true,
            "asset_source": "mikrotik_router",
            "asset_type": "mikrotik_router",
            "asset_id": "router-1",
            "router_id": "router-1",
        });
        assert!(NetworkMappingService::system_managed_node_matches_asset_reference(
            &router,
            "mikrotik_router",
            "router-1"
        ));
    }

    #[test]
    fn customer_pppoe_visual_state_contract() {
        assert_eq!(
            NetworkMappingService::customer_pppoe_visual_state(
                true,
                "active",
                Some("alice"),
                true,
                false
            ),
            "connected"
        );
        assert_eq!(
            NetworkMappingService::customer_pppoe_visual_state(
                true,
                "active",
                Some("alice"),
                false,
                false
            ),
            "disconnected"
        );
        assert_eq!(
            NetworkMappingService::customer_pppoe_visual_state(
                true,
                "suspended",
                Some("alice"),
                true,
                true
            ),
            "neutral"
        );
        assert_eq!(
            NetworkMappingService::customer_pppoe_visual_state(
                false,
                "active",
                Some("alice"),
                true,
                false
            ),
            "neutral"
        );
        assert_eq!(
            NetworkMappingService::customer_pppoe_visual_state(true, "active", None, false, false),
            "neutral"
        );
    }

    #[test]
    fn customer_location_sync_metadata_includes_service_fields() {
        let row = SyncCustomerLocationRow {
            location_id: "loc-1".into(),
            customer_id: "cust-1".into(),
            customer_name: "Budi".into(),
            customer_is_active: true,
            label: "Rumah Budi".into(),
            subscription_id: "sub-1".into(),
            subscription_status: "active".into(),
            package_name: Some("Paket 20 Mbps".into()),
            package_service_type: Some("internet_pppoe".into()),
            pppoe_username: Some("budi-pppoe".into()),
            pppoe_disabled: Some(false),
            pppoe_session_active: Some(true),
            pppoe_account_source: Some("managed_radius".into()),
            pppoe_router_profile_name: Some("PPPOE-20M".into()),
            router_id: Some("router-1".into()),
            latitude: -7.0,
            longitude: 110.0,
        };

        let metadata = NetworkMappingService::build_customer_location_metadata(&row);

        assert_eq!(
            metadata.get("service_id").and_then(|value| value.as_str()),
            Some("sub-1")
        );
        assert_eq!(
            metadata
                .get("service_name")
                .and_then(|value| value.as_str()),
            Some("Paket 20 Mbps")
        );
        assert_eq!(
            metadata
                .get("service_label")
                .and_then(|value| value.as_str()),
            Some("PPPOE-20M")
        );
        assert_eq!(
            metadata
                .get("service_type")
                .and_then(|value| value.as_str()),
            Some("internet_pppoe")
        );
        assert_eq!(
            metadata
                .get("package_name")
                .and_then(|value| value.as_str()),
            Some("Paket 20 Mbps")
        );
        assert_eq!(
            metadata
                .get("pppoe_username")
                .and_then(|value| value.as_str()),
            Some("budi-pppoe")
        );
        assert_eq!(
            metadata
                .get("pppoe_account_source")
                .and_then(|value| value.as_str()),
            Some("managed_radius")
        );
        assert_eq!(
            metadata
                .get("pppoe_visual_state")
                .and_then(|value| value.as_str()),
            Some("connected")
        );
        assert_eq!(
            metadata
                .get("pppoe_session_active")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            metadata.get("router_id").and_then(|value| value.as_str()),
            Some("router-1")
        );
    }

    #[test]
    fn geometry_db_error_mapping_preserves_validation_vs_database_boundary() {
        let geo_err = sqlx::Error::Protocol("invalid GeoJSON geometry".to_string());
        let mapped_geo = NetworkMappingService::map_geometry_db_error(geo_err, "geometry");
        assert!(matches!(mapped_geo, AppError::Validation(_)));

        let db_err = sqlx::Error::Protocol("connection reset by peer".to_string());
        let mapped_db = NetworkMappingService::map_geometry_db_error(db_err, "geometry");
        assert!(matches!(mapped_db, AppError::Database(_)));
    }
    #[test]
    fn mapper_candidate_row_transforms_into_ranked_item() {
        let ranked = NetworkMappingService::candidate_row_to_ranked(CandidateNodeRow {
            node_id: "node-1".into(),
            name: "Node 1".into(),
            node_type: "router".into(),
            status: "active".into(),
            capacity_json: serde_json::json!({"available_mbps": 25, "total_mbps": 100}),
            health_json: serde_json::json!({}),
            distance_m: Some(1200.0),
            avg_link_utilization_pct: Some(30.0),
            down_links: 1,
            link_count: 4,
        });

        assert_eq!(ranked.node_id, "node-1");
        assert_eq!(ranked.name, "Node 1");
        assert_eq!(ranked.node_type, "router");
        assert_eq!(ranked.status, "active");
        assert!(ranked.score >= 0.0 && ranked.score <= 100.0);
        assert!(ranked.reason.contains("health"));
    }

    #[test]
    fn mapper_builds_not_found_path_response_shape() {
        let source = "source-node".to_string();
        let target = "target-node".to_string();
        let out =
            NetworkMappingService::build_path_not_found_response(source.clone(), target.clone());

        assert!(!out.found);
        assert_eq!(out.source_node_id, source);
        assert_eq!(out.target_node_id, target);
        assert!(out.node_ids.is_empty());
        assert!(out.link_ids.is_empty());
        assert!(out.hops.is_empty());
        assert!(out.total_cost.is_none());
        assert!(out.total_distance_m.is_none());
    }
}
