use super::dto::{SyncCustomerLocationRow, SyncRouterRow, UuidTextRow};
use super::NetworkMappingService;
use crate::error::{AppError, AppResult};
use crate::models::{
    NetworkImpactCustomer, NetworkImpactResponse, NetworkNode, SyncTopologyAssetsResponse,
};
use std::collections::{HashMap, HashSet};

impl NetworkMappingService {
    pub(super) fn build_customer_location_metadata(
        row: &SyncCustomerLocationRow,
    ) -> serde_json::Map<String, serde_json::Value> {
        let mut metadata = serde_json::Map::from_iter([
            ("system_managed".to_string(), serde_json::Value::Bool(true)),
            (
                "asset_source".to_string(),
                serde_json::Value::String("customer_location".to_string()),
            ),
            (
                "asset_type".to_string(),
                serde_json::Value::String("customer_location".to_string()),
            ),
            (
                "asset_id".to_string(),
                serde_json::Value::String(row.location_id.clone()),
            ),
            (
                "location_id".to_string(),
                serde_json::Value::String(row.location_id.clone()),
            ),
            (
                "customer_id".to_string(),
                serde_json::Value::String(row.customer_id.clone()),
            ),
            (
                "customer_name".to_string(),
                serde_json::Value::String(row.customer_name.clone()),
            ),
            (
                "location_label".to_string(),
                serde_json::Value::String(row.label.clone()),
            ),
            (
                "subscription_id".to_string(),
                serde_json::Value::String(row.subscription_id.clone()),
            ),
            (
                "subscription_status".to_string(),
                serde_json::Value::String(row.subscription_status.clone()),
            ),
        ]);

        let package_name = row
            .package_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let package_service_type = row
            .package_service_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let pppoe_username = row
            .pppoe_username
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let pppoe_visual_state = Self::customer_pppoe_visual_state(
            row.customer_is_active,
            &row.subscription_status,
            pppoe_username.as_deref(),
            row.pppoe_session_active.unwrap_or(false),
            row.pppoe_disabled.unwrap_or(false),
        );
        let pppoe_account_source = row
            .pppoe_account_source
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let pppoe_router_profile_name = row
            .pppoe_router_profile_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let router_id = row
            .router_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        metadata.insert(
            "service_id".to_string(),
            serde_json::Value::String(row.subscription_id.clone()),
        );

        if let Some(service_name) = package_name
            .clone()
            .or_else(|| pppoe_router_profile_name.clone())
            .or_else(|| pppoe_username.clone())
        {
            metadata.insert(
                "service_name".to_string(),
                serde_json::Value::String(service_name),
            );
        }

        if let Some(service_label) = pppoe_router_profile_name
            .clone()
            .or_else(|| package_name.clone())
        {
            metadata.insert(
                "service_label".to_string(),
                serde_json::Value::String(service_label),
            );
        }

        if let Some(service_type) = package_service_type
            .clone()
            .or_else(|| pppoe_username.as_ref().map(|_| "pppoe".to_string()))
        {
            metadata.insert(
                "service_type".to_string(),
                serde_json::Value::String(service_type),
            );
        }

        if let Some(value) = package_name {
            metadata.insert("package_name".to_string(), serde_json::Value::String(value));
        }
        metadata.insert(
            "customer_is_active".to_string(),
            serde_json::Value::Bool(row.customer_is_active),
        );
        metadata.insert(
            "pppoe_visual_state".to_string(),
            serde_json::Value::String(pppoe_visual_state.to_string()),
        );
        metadata.insert(
            "pppoe_session_active".to_string(),
            serde_json::Value::Bool(row.pppoe_session_active.unwrap_or(false)),
        );
        if let Some(value) = pppoe_username {
            metadata.insert(
                "pppoe_username".to_string(),
                serde_json::Value::String(value),
            );
        }
        if let Some(value) = pppoe_account_source {
            metadata.insert(
                "pppoe_account_source".to_string(),
                serde_json::Value::String(value),
            );
        }
        if let Some(value) = pppoe_router_profile_name {
            metadata.insert(
                "router_profile_name".to_string(),
                serde_json::Value::String(value),
            );
        }
        if let Some(value) = router_id {
            metadata.insert("router_id".to_string(), serde_json::Value::String(value));
        }

        metadata
    }

    pub(super) fn merge_customer_location_metadata(
        metadata: &mut serde_json::Value,
        row: &SyncCustomerLocationRow,
    ) {
        let next = serde_json::Value::Object(Self::build_customer_location_metadata(row));
        let Some(current) = metadata.as_object_mut() else {
            *metadata = next;
            return;
        };
        let Some(next_object) = next.as_object() else {
            return;
        };

        for (key, value) in next_object {
            current.insert(key.clone(), value.clone());
        }
    }

    pub(super) async fn overlay_live_customer_location_metadata(
        &self,
        tenant_id: &str,
        nodes: &mut [NetworkNode],
    ) -> AppResult<()> {
        let location_ids: Vec<String> = nodes
            .iter()
            .filter_map(|node| {
                let metadata = node.metadata.as_object()?;
                if metadata.get("asset_type").and_then(|value| value.as_str())
                    != Some("customer_location")
                {
                    return None;
                }
                metadata
                    .get("asset_id")
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
            })
            .collect();

        if location_ids.is_empty() {
            return Ok(());
        }

        let rows: Vec<SyncCustomerLocationRow> = sqlx::query_as(
            r#"
            SELECT
              cl.id::text AS location_id,
              cl.customer_id::text AS customer_id,
              c.name AS customer_name,
              c.is_active AS customer_is_active,
              COALESCE(NULLIF(BTRIM(cl.label), ''), c.name || ' Location') AS label,
              svc.subscription_id AS subscription_id,
              svc.subscription_status AS subscription_status,
              p.name AS package_name,
              p.service_type AS package_service_type,
              acct.username AS pppoe_username,
              acct.disabled AS pppoe_disabled,
              (sess.username IS NOT NULL) AS pppoe_session_active,
              acct.account_source AS pppoe_account_source,
              acct.router_profile_name AS pppoe_router_profile_name,
              COALESCE(acct.router_id, svc.router_id) AS router_id,
              cl.latitude::float8 AS latitude,
              cl.longitude::float8 AS longitude
            FROM customer_locations cl
            INNER JOIN customers c
              ON c.tenant_id::text = cl.tenant_id::text
             AND c.id::text = cl.customer_id::text
            INNER JOIN LATERAL (
              SELECT
                cs.id::text AS subscription_id,
                cs.status AS subscription_status,
                cs.package_id,
                cs.router_id
              FROM customer_subscriptions cs
              WHERE cs.tenant_id = cl.tenant_id
                AND cs.location_id = cl.id
                AND cs.status IN (
                  'active',
                  'grace_active',
                  'pending_installation',
                  'installation_done_awaiting_payment',
                  'suspended'
                )
              ORDER BY
                CASE cs.status
                  WHEN 'active' THEN 0
                  WHEN 'grace_active' THEN 1
                  WHEN 'pending_installation' THEN 2
                  WHEN 'installation_done_awaiting_payment' THEN 3
                  WHEN 'suspended' THEN 4
                  ELSE 5
                END,
                cs.updated_at DESC,
                cs.created_at DESC
              LIMIT 1
            ) svc ON TRUE
            LEFT JOIN isp_packages p
              ON p.tenant_id = cl.tenant_id
             AND p.id = svc.package_id
            LEFT JOIN LATERAL (
              SELECT
                pa.username,
                pa.disabled,
                pa.router_present,
                pa.account_source,
                pa.router_profile_name,
                pa.router_id
              FROM pppoe_accounts pa
              WHERE pa.tenant_id = cl.tenant_id
                AND pa.location_id = cl.id
              ORDER BY
                CASE
                  WHEN pa.account_source = 'managed_radius' THEN 0
                  ELSE 1
                END,
                pa.updated_at DESC,
                pa.created_at DESC
              LIMIT 1
            ) acct ON TRUE
            LEFT JOIN mikrotik_ppp_active_sessions sess
              ON sess.tenant_id = cl.tenant_id
             AND sess.router_id = COALESCE(acct.router_id, svc.router_id)
             AND sess.username = acct.username
            WHERE cl.tenant_id = $1::text
              AND cl.id::text = ANY($2)
            "#,
        )
        .bind(tenant_id)
        .bind(&location_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let rows_by_location_id: HashMap<String, SyncCustomerLocationRow> = rows
            .into_iter()
            .map(|row| (row.location_id.clone(), row))
            .collect();

        for node in nodes.iter_mut() {
            let asset_type = node
                .metadata
                .get("asset_type")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            if asset_type != "customer_location" {
                continue;
            }
            let Some(location_id) = node
                .metadata
                .get("asset_id")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            let Some(row) = rows_by_location_id.get(location_id) else {
                continue;
            };
            Self::merge_customer_location_metadata(&mut node.metadata, row);
        }

        Ok(())
    }

    pub(super) async fn sync_topology_asset_nodes_flow(
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
              c.is_active AS customer_is_active,
              COALESCE(NULLIF(BTRIM(cl.label), ''), c.name || ' Location') AS label,
              svc.subscription_id AS subscription_id,
              svc.subscription_status AS subscription_status,
              p.name AS package_name,
              p.service_type AS package_service_type,
              acct.username AS pppoe_username,
              acct.disabled AS pppoe_disabled,
              (sess.username IS NOT NULL) AS pppoe_session_active,
              acct.account_source AS pppoe_account_source,
              acct.router_profile_name AS pppoe_router_profile_name,
              COALESCE(acct.router_id, svc.router_id) AS router_id,
              cl.latitude::float8 AS latitude,
              cl.longitude::float8 AS longitude
            FROM customer_locations cl
            INNER JOIN customers c
              ON c.tenant_id::text = cl.tenant_id::text
             AND c.id::text = cl.customer_id::text
            INNER JOIN LATERAL (
              SELECT
                cs.id::text AS subscription_id,
                cs.status AS subscription_status,
                cs.package_id,
                cs.router_id
              FROM customer_subscriptions cs
              WHERE cs.tenant_id = cl.tenant_id
                AND cs.location_id = cl.id
                AND cs.status IN (
                  'active',
                  'grace_active',
                  'pending_installation',
                  'installation_done_awaiting_payment',
                  'suspended'
                )
              ORDER BY
                CASE cs.status
                  WHEN 'active' THEN 0
                  WHEN 'grace_active' THEN 1
                  WHEN 'pending_installation' THEN 2
                  WHEN 'installation_done_awaiting_payment' THEN 3
                  WHEN 'suspended' THEN 4
                  ELSE 5
                END,
                cs.updated_at DESC,
                cs.created_at DESC
              LIMIT 1
            ) svc ON TRUE
            LEFT JOIN isp_packages p
              ON p.tenant_id = cl.tenant_id
             AND p.id = svc.package_id
            LEFT JOIN LATERAL (
              SELECT
                pa.username,
                pa.disabled,
                pa.router_present,
                pa.account_source,
                pa.router_profile_name,
                pa.router_id
              FROM pppoe_accounts pa
              WHERE pa.tenant_id = cl.tenant_id
                AND pa.location_id = cl.id
              ORDER BY
                CASE
                  WHEN pa.account_source = 'managed_radius' THEN 0
                  ELSE 1
                END,
                pa.updated_at DESC,
                pa.created_at DESC
              LIMIT 1
            ) acct ON TRUE
            LEFT JOIN mikrotik_ppp_active_sessions sess
              ON sess.tenant_id = cl.tenant_id
             AND sess.router_id = COALESCE(acct.router_id, svc.router_id)
             AND sess.username = acct.username
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
                    serde_json::Value::Object(Self::build_customer_location_metadata(&row)),
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

    pub(super) async fn list_impacted_customers_flow(
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_customer_location_metadata_overrides_stale_pppoe_state() {
        let row = SyncCustomerLocationRow {
            location_id: "loc-1".into(),
            customer_id: "cust-1".into(),
            customer_name: "User 1".into(),
            customer_is_active: true,
            label: "Rumah".into(),
            subscription_id: "sub-1".into(),
            subscription_status: "active".into(),
            package_name: Some("Paket".into()),
            package_service_type: Some("internet_pppoe".into()),
            pppoe_username: Some("test".into()),
            pppoe_disabled: Some(false),
            pppoe_session_active: Some(true),
            pppoe_account_source: Some("managed_radius".into()),
            pppoe_router_profile_name: Some("PPPOE".into()),
            router_id: Some("router-1".into()),
            latitude: -7.0,
            longitude: 110.0,
        };
        let mut metadata = serde_json::json!({
            "asset_type": "customer_location",
            "asset_id": "loc-1",
            "customer_name": "User 1",
            "pppoe_username": "test",
            "pppoe_visual_state": "disconnected",
            "pppoe_session_active": false,
            "router_id": "router-1"
        });

        NetworkMappingService::merge_customer_location_metadata(&mut metadata, &row);

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
    }
}
