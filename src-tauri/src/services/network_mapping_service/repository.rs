use super::NetworkMappingService;
use crate::error::{AppError, AppResult};
use crate::models::{NetworkLink, NetworkNode, ServiceZone, ZoneOffer};
use uuid::Uuid;

impl NetworkMappingService {
    pub(super) async fn find_node_by_asset_reference(
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
                  $2::text = 'network_asset'
                  AND metadata->>'asset_source' = $2::text
                  AND metadata->>'asset_id' = $3::text
                )
                OR (
                  $2::text NOT IN ('network_asset', 'mikrotik_router', 'customer_location')
                  AND
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

    pub(super) async fn dedupe_system_managed_nodes_by_asset_source(
        &self,
        tenant_id: &str,
        asset_source: &str,
    ) -> AppResult<u64> {
        let rows_affected = if asset_source == "network_asset" || asset_source == "customer_location" {
            sqlx::query(
                r#"
                DELETE FROM network_nodes n
                USING (
                  SELECT id
                  FROM (
                    SELECT
                      id,
                      ROW_NUMBER() OVER (
                        PARTITION BY metadata->>'asset_source', metadata->>'asset_id'
                        ORDER BY updated_at DESC, created_at DESC, id DESC
                      ) AS row_num
                    FROM network_nodes
                    WHERE tenant_id = $1::uuid
                      AND metadata->>'asset_source' = $2::text
                      AND COALESCE((metadata->>'system_managed')::boolean, false)
                  ) ranked
                  WHERE ranked.row_num > 1
                ) duplicates
                WHERE n.id = duplicates.id::uuid
                "#,
            )
            .bind(tenant_id)
            .bind(asset_source)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?
            .rows_affected()
        } else {
            0
        };

        Ok(rows_affected)
    }

    pub(super) async fn upsert_system_managed_node(
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

    pub(super) async fn prune_system_managed_nodes_not_in_assets(
        &self,
        tenant_id: &str,
        asset_type: &str,
        asset_ids: &[String],
    ) -> AppResult<u64> {
        let rows_affected = if asset_type == "network_asset" {
            if asset_ids.is_empty() {
                sqlx::query(
                    r#"
                    DELETE FROM network_nodes
                    WHERE tenant_id = $1::uuid
                      AND metadata->>'asset_source' = $2::text
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
                      AND metadata->>'asset_source' = $2::text
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
            }
        } else if asset_ids.is_empty() {
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

    pub(super) async fn ensure_link_pair_available(
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

    pub(super) async fn get_node_by_id(&self, tenant_id: &str, id: &str) -> AppResult<NetworkNode> {
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

    pub(super) async fn get_link_by_id(&self, tenant_id: &str, id: &str) -> AppResult<NetworkLink> {
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

    pub(super) async fn get_zone_by_id(&self, tenant_id: &str, id: &str) -> AppResult<ServiceZone> {
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

    pub(super) async fn get_zone_offer_by_id(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<ZoneOffer> {
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
