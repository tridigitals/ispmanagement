use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    CreateNetworkLinkRequest, CreateNetworkNodeRequest, CreateServiceZoneRequest,
    CreateZoneNodeBindingRequest, NetworkLink, NetworkNode, PaginatedResponse, ResolvedZone,
    ResolvedZoneResponse, ResolveZoneRequest, ServiceZone, UpdateNetworkLinkRequest,
    UpdateNetworkNodeRequest, UpdateServiceZoneRequest, ZoneNodeBinding,
};
use crate::services::AuthService;
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
            &[("network_topology", "manage"), ("network_routers", "manage")],
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
            return Err(AppError::Validation(format!("{field}.lat must be between -90 and 90")));
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

    pub async fn list_nodes(
        &self,
        actor_id: &str,
        tenant_id: &str,
        q: ListQuery,
    ) -> AppResult<PaginatedResponse<NetworkNode>> {
        self.require_read(actor_id, tenant_id).await?;
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
        self.require_manage(actor_id, tenant_id).await?;
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
        let res = sqlx::query("DELETE FROM network_nodes WHERE tenant_id = $1::uuid AND id = $2::uuid")
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
        self.require_read(actor_id, tenant_id).await?;
        let search = Self::cleaned_query(q.q);
        let status_filter = q
            .status
            .as_deref()
            .map(Self::normalize_link_status);
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
        self.require_manage(actor_id, tenant_id).await?;
        if dto.name.trim().is_empty() {
            return Err(AppError::Validation("name is required".into()));
        }
        Self::validate_geojson_geometry(&dto.geometry, &["LineString", "MultiLineString"], "geometry")?;
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
        let status = Self::normalize_link_status(
            dto.status
                .as_deref()
                .unwrap_or(current.status.as_str()),
        );
        Self::validate_link_status(&status)?;
        let next_from_node_id = dto
            .from_node_id
            .clone()
            .unwrap_or_else(|| current.from_node_id.clone());
        let next_to_node_id = dto
            .to_node_id
            .clone()
            .unwrap_or_else(|| current.to_node_id.clone());
        self.ensure_link_pair_available(
            tenant_id,
            &next_from_node_id,
            &next_to_node_id,
            Some(id),
        )
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
        let res = sqlx::query("DELETE FROM network_links WHERE tenant_id = $1::uuid AND id = $2::uuid")
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
        let res = sqlx::query("DELETE FROM service_zones WHERE tenant_id = $1::uuid AND id = $2::uuid")
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
        let res = sqlx::query("DELETE FROM zone_node_bindings WHERE tenant_id = $1::uuid AND id = $2::uuid")
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
}
