use super::*;

impl CustomerService {
    pub(crate) fn should_auto_create_first_invoice_on_completion(
        resolved: SubscriptionLifecycleStatus,
        has_paid_invoice: bool,
    ) -> bool {
        !has_paid_invoice && resolved == SubscriptionLifecycleStatus::GraceActive
    }

    async fn attach_installation_work_order_invoice(
        &self,
        tenant_id: &str,
        work_order_id: &str,
        invoice_id: &str,
    ) -> AppResult<()> {
        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE installation_work_orders
            SET invoice_id = $3, updated_at = NOW()
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .bind(invoice_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE installation_work_orders
            SET invoice_id = ?, updated_at = ?
            WHERE tenant_id = ? AND id = ?
            "#,
        )
        .bind(invoice_id)
        .bind(Utc::now().to_rfc3339())
        .bind(tenant_id)
        .bind(work_order_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn load_network_asset(&self, tenant_id: &str, asset_id: &str) -> AppResult<NetworkAsset> {
        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT
              id, tenant_id, asset_group, asset_type, name, code, vendor, model, serial_number, status,
              customer_id, location_id, work_order_id, parent_asset_id, notes, metadata, created_at, updated_at
            FROM network_assets
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT
              id, tenant_id, asset_group, asset_type, name, code, vendor, model, serial_number, status,
              customer_id, location_id, work_order_id, parent_asset_id, notes, metadata, created_at, updated_at
            FROM network_assets
            WHERE tenant_id = ?1 AND id = ?2
            LIMIT 1
        "#;

        sqlx::query_as::<_, NetworkAsset>(query)
            .bind(tenant_id)
            .bind(asset_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Network asset not found".into()))
    }

    async fn bind_assets_to_completed_installation(
        &self,
        tenant_id: &str,
        work_order: &InstallationWorkOrder,
        terminal_asset_id: &str,
        parent_asset_id: Option<&str>,
    ) -> AppResult<()> {
        let terminal_asset = self
            .load_network_asset(tenant_id, terminal_asset_id)
            .await?;
        if !matches!(terminal_asset.asset_type.as_str(), "ont" | "onu") {
            return Err(AppError::Validation(
                "Terminal asset must be ONT or ONU".into(),
            ));
        }
        if matches!(terminal_asset.status.as_str(), "faulty" | "retired") {
            return Err(AppError::Validation(
                "Selected terminal asset is not usable".into(),
            ));
        }
        if terminal_asset.customer_id.as_deref().is_some()
            && terminal_asset.customer_id.as_deref() != Some(work_order.customer_id.as_str())
        {
            return Err(AppError::Conflict(
                "Selected terminal asset is already assigned to another customer".into(),
            ));
        }

        if let Some(parent_asset_id) = parent_asset_id {
            let parent_asset = self.load_network_asset(tenant_id, parent_asset_id).await?;
            if !matches!(
                parent_asset.asset_type.as_str(),
                "olt" | "odc" | "odp" | "splitter" | "fat" | "nap" | "odf"
            ) {
                return Err(AppError::Validation(
                    "Parent asset must be an FTTH upstream asset".into(),
                ));
            }
            if matches!(parent_asset.status.as_str(), "faulty" | "retired") {
                return Err(AppError::Validation(
                    "Selected parent asset is not usable".into(),
                ));
            }
        }

        #[cfg(feature = "postgres")]
        {
            sqlx::query(
                r#"
                UPDATE network_assets
                SET customer_id = NULL,
                    location_id = NULL,
                    work_order_id = NULL,
                    parent_asset_id = NULL,
                    status = 'available',
                    updated_at = NOW()
                WHERE tenant_id = $1
                  AND work_order_id = $2
                  AND asset_type IN ('ont', 'onu')
                  AND id <> $3
                "#,
            )
            .bind(tenant_id)
            .bind(&work_order.id)
            .bind(terminal_asset_id)
            .execute(&self.pool)
            .await?;

            sqlx::query(
                r#"
                UPDATE network_assets
                SET work_order_id = NULL,
                    updated_at = NOW()
                WHERE tenant_id = $1
                  AND work_order_id = $2
                  AND asset_type IN ('olt', 'odc', 'odp', 'splitter', 'fat', 'nap', 'odf')
                  AND ($3::text IS NULL OR id <> $3)
                "#,
            )
            .bind(tenant_id)
            .bind(&work_order.id)
            .bind(parent_asset_id)
            .execute(&self.pool)
            .await?;

            sqlx::query(
                r#"
                UPDATE network_assets
                SET customer_id = $3,
                    location_id = $4,
                    work_order_id = $5,
                    parent_asset_id = $6,
                    status = 'installed',
                    updated_at = NOW()
                WHERE tenant_id = $1 AND id = $2
                "#,
            )
            .bind(tenant_id)
            .bind(terminal_asset_id)
            .bind(&work_order.customer_id)
            .bind(&work_order.location_id)
            .bind(&work_order.id)
            .bind(parent_asset_id)
            .execute(&self.pool)
            .await?;

            if let Some(parent_asset_id) = parent_asset_id {
                sqlx::query(
                    r#"
                    UPDATE network_assets
                    SET work_order_id = $3,
                        updated_at = NOW()
                    WHERE tenant_id = $1 AND id = $2
                    "#,
                )
                .bind(tenant_id)
                .bind(parent_asset_id)
                .bind(&work_order.id)
                .execute(&self.pool)
                .await?;
            }
        }

        #[cfg(feature = "sqlite")]
        {
            let now = Utc::now().to_rfc3339();
            sqlx::query(
                r#"
                UPDATE network_assets
                SET customer_id = NULL,
                    location_id = NULL,
                    work_order_id = NULL,
                    parent_asset_id = NULL,
                    status = 'available',
                    updated_at = ?4
                WHERE tenant_id = ?1
                  AND work_order_id = ?2
                  AND asset_type IN ('ont', 'onu')
                  AND id <> ?3
                "#,
            )
            .bind(tenant_id)
            .bind(&work_order.id)
            .bind(terminal_asset_id)
            .bind(&now)
            .execute(&self.pool)
            .await?;

            sqlx::query(
                r#"
                UPDATE network_assets
                SET work_order_id = NULL,
                    updated_at = ?4
                WHERE tenant_id = ?1
                  AND work_order_id = ?2
                  AND asset_type IN ('olt', 'odc', 'odp', 'splitter', 'fat', 'nap', 'odf')
                  AND (?3 IS NULL OR id <> ?3)
                "#,
            )
            .bind(tenant_id)
            .bind(&work_order.id)
            .bind(parent_asset_id)
            .bind(&now)
            .execute(&self.pool)
            .await?;

            sqlx::query(
                r#"
                UPDATE network_assets
                SET customer_id = ?3,
                    location_id = ?4,
                    work_order_id = ?5,
                    parent_asset_id = ?6,
                    status = 'installed',
                    updated_at = ?7
                WHERE tenant_id = ?1 AND id = ?2
                "#,
            )
            .bind(tenant_id)
            .bind(terminal_asset_id)
            .bind(&work_order.customer_id)
            .bind(&work_order.location_id)
            .bind(&work_order.id)
            .bind(parent_asset_id)
            .bind(&now)
            .execute(&self.pool)
            .await?;

            if let Some(parent_asset_id) = parent_asset_id {
                sqlx::query(
                    r#"
                    UPDATE network_assets
                    SET work_order_id = ?3,
                        updated_at = ?4
                    WHERE tenant_id = ?1 AND id = ?2
                    "#,
                )
                .bind(tenant_id)
                .bind(parent_asset_id)
                .bind(&work_order.id)
                .bind(&now)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn list_installation_work_orders(
        &self,
        actor_id: &str,
        tenant_id: &str,
        status: Option<String>,
        assigned_to: Option<String>,
        include_closed: bool,
        limit: u32,
    ) -> AppResult<Vec<InstallationWorkOrderView>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "read")
            .await?;

        let limit = limit.clamp(1, 500);
        let status_filter = status
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(Self::normalize_work_order_status)
            .transpose()?;
        let assigned_filter = assigned_to
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let actor_role_name = self.get_actor_role_name(tenant_id, actor_id).await?;
        let is_admin_owner = matches!(actor_role_name.as_deref(), Some("owner") | Some("admin"));
        let can_manage_work_orders = self
            .auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await
            .is_ok();
        let has_full_visibility = is_admin_owner
            || (can_manage_work_orders && !Self::is_technician_role(actor_role_name.as_deref()));
        let visibility_mode = self
            .resolve_installation_work_order_visibility_mode(tenant_id)
            .await;
        let can_view_unassigned = has_full_visibility
            || Self::should_non_admin_see_unassigned_installation_work_orders(visibility_mode);

        #[cfg(feature = "postgres")]
        let rows: Vec<InstallationWorkOrderView> = sqlx::query_as(
            r#"
            SELECT
              wo.id, wo.tenant_id, wo.subscription_id, wo.invoice_id, wo.customer_id, wo.location_id,
              cs.package_id AS package_id,
              COALESCE(wo.router_id, cs.router_id) AS router_id,
              wo.status, wo.assigned_to, wo.scheduled_at, wo.completed_at, wo.notes, wo.created_at, wo.updated_at,
              c.name AS customer_name,
              c.phone AS customer_phone,
              l.label AS location_label,
              l.latitude::float8 AS location_latitude,
              l.longitude::float8 AS location_longitude,
              p.name AS package_name,
              p.provisioning_type AS package_provisioning_type,
              r.name AS router_name,
              u.name AS assigned_to_name,
              u.email AS assigned_to_email,
              csa.id AS assignment_id,
              csa.status AS assignment_status,
              cs.status AS subscription_status,
              cs.starts_at AS subscription_starts_at,
              cs.grace_until AS subscription_grace_until,
              EXISTS(
                SELECT 1
                FROM invoices i
                WHERE i.tenant_id = wo.tenant_id
                  AND (
                    i.external_id = 'pkgsub:' || wo.subscription_id
                    OR i.external_id LIKE 'pkgsub:' || wo.subscription_id || ':%'
                  )
              ) AS has_customer_package_invoice,
              csa.selected_zone_id AS selected_zone_id,
              sz.name AS selected_zone_name,
              csa.selected_node_id AS selected_node_id,
              nn.name AS selected_node_name,
              csa.selected_node_score::float8 AS selected_node_score,
              csa.path_node_ids AS path_node_ids,
              csa.path_link_ids AS path_link_ids
            FROM installation_work_orders wo
            LEFT JOIN customers c ON c.tenant_id = wo.tenant_id AND c.id = wo.customer_id
            LEFT JOIN customer_locations l ON l.tenant_id = wo.tenant_id AND l.id = wo.location_id
            LEFT JOIN customer_subscriptions cs ON cs.tenant_id = wo.tenant_id AND cs.id = wo.subscription_id
            LEFT JOIN isp_packages p ON p.tenant_id = wo.tenant_id AND p.id = cs.package_id
            LEFT JOIN mikrotik_routers r
              ON r.tenant_id = wo.tenant_id
             AND r.id = COALESCE(wo.router_id, cs.router_id)
            LEFT JOIN users u ON u.id = wo.assigned_to
            LEFT JOIN customer_service_assignments csa ON csa.tenant_id = wo.tenant_id AND csa.work_order_id = wo.id
            LEFT JOIN service_zones sz ON sz.tenant_id = wo.tenant_id::uuid AND sz.id::text = csa.selected_zone_id
            LEFT JOIN network_nodes nn ON nn.tenant_id = wo.tenant_id::uuid AND nn.id::text = csa.selected_node_id
            WHERE wo.tenant_id = $1
              AND ($2::text IS NULL OR wo.status = $2)
              AND ($3::text IS NULL OR wo.assigned_to = $3)
              AND (
                $4::bool
                OR wo.status NOT IN ('completed', 'cancelled')
                OR (
                  wo.status = 'completed'
                  AND LOWER(COALESCE(cs.status, '')) IN ('pending_installation', 'grace_active', 'suspended')
                )
              )
              AND (
                $6::bool
                OR wo.assigned_to = $7
                OR (
                  $5::bool
                  AND wo.status = 'pending'
                  AND (wo.assigned_to IS NULL OR btrim(wo.assigned_to) = '')
                )
              )
            ORDER BY
              CASE wo.status
                WHEN 'pending' THEN 0
                WHEN 'in_progress' THEN 1
                WHEN 'completed' THEN 2
                WHEN 'cancelled' THEN 3
                ELSE 4
              END ASC,
              wo.updated_at DESC
            LIMIT $8
            "#,
        )
        .bind(tenant_id)
        .bind(status_filter)
        .bind(assigned_filter)
        .bind(include_closed)
        .bind(can_view_unassigned)
        .bind(has_full_visibility)
        .bind(actor_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<InstallationWorkOrderView> = sqlx::query_as(
            r#"
            SELECT
              wo.id, wo.tenant_id, wo.subscription_id, wo.invoice_id, wo.customer_id, wo.location_id,
              cs.package_id AS package_id,
              COALESCE(wo.router_id, cs.router_id) AS router_id,
              wo.status, wo.assigned_to, wo.scheduled_at, wo.completed_at, wo.notes, wo.created_at, wo.updated_at,
              c.name AS customer_name,
              c.phone AS customer_phone,
              l.label AS location_label,
              l.latitude::float8 AS location_latitude,
              l.longitude::float8 AS location_longitude,
              p.name AS package_name,
              p.provisioning_type AS package_provisioning_type,
              r.name AS router_name,
              u.name AS assigned_to_name,
              u.email AS assigned_to_email,
              csa.id AS assignment_id,
              csa.status AS assignment_status,
              cs.status AS subscription_status,
              cs.starts_at AS subscription_starts_at,
              cs.grace_until AS subscription_grace_until,
              EXISTS(
                SELECT 1
                FROM invoices i
                WHERE i.tenant_id = wo.tenant_id
                  AND (
                    i.external_id = 'pkgsub:' || wo.subscription_id
                    OR i.external_id LIKE 'pkgsub:' || wo.subscription_id || ':%'
                  )
              ) AS has_customer_package_invoice,
              csa.selected_zone_id AS selected_zone_id,
              sz.name AS selected_zone_name,
              csa.selected_node_id AS selected_node_id,
              nn.name AS selected_node_name,
              csa.selected_node_score AS selected_node_score,
              csa.path_node_ids AS path_node_ids,
              csa.path_link_ids AS path_link_ids
            FROM installation_work_orders wo
            LEFT JOIN customers c ON c.tenant_id = wo.tenant_id AND c.id = wo.customer_id
            LEFT JOIN customer_locations l ON l.tenant_id = wo.tenant_id AND l.id = wo.location_id
            LEFT JOIN customer_subscriptions cs ON cs.tenant_id = wo.tenant_id AND cs.id = wo.subscription_id
            LEFT JOIN isp_packages p ON p.tenant_id = wo.tenant_id AND p.id = cs.package_id
            LEFT JOIN mikrotik_routers r
              ON r.tenant_id = wo.tenant_id
             AND r.id = COALESCE(wo.router_id, cs.router_id)
            LEFT JOIN users u ON u.id = wo.assigned_to
            LEFT JOIN customer_service_assignments csa ON csa.tenant_id = wo.tenant_id AND csa.work_order_id = wo.id
            LEFT JOIN service_zones sz ON sz.tenant_id = wo.tenant_id AND sz.id = csa.selected_zone_id
            LEFT JOIN network_nodes nn ON nn.tenant_id = wo.tenant_id AND nn.id = csa.selected_node_id
            WHERE wo.tenant_id = ?
              AND (? IS NULL OR wo.status = ?)
              AND (? IS NULL OR wo.assigned_to = ?)
              AND (
                ? = 1
                OR wo.status NOT IN ('completed', 'cancelled')
                OR (
                  wo.status = 'completed'
                  AND LOWER(COALESCE(cs.status, '')) IN ('pending_installation', 'grace_active', 'suspended')
                )
              )
              AND (
                ? = 1
                OR wo.assigned_to = ?
                OR (
                  ? = 1
                  AND
                  wo.status = 'pending'
                  AND (wo.assigned_to IS NULL OR trim(wo.assigned_to) = '')
                )
              )
            ORDER BY
              CASE wo.status
                WHEN 'pending' THEN 0
                WHEN 'in_progress' THEN 1
                WHEN 'completed' THEN 2
                WHEN 'cancelled' THEN 3
                ELSE 4
              END ASC,
              wo.updated_at DESC
            LIMIT ?
            "#,
        )
        .bind(tenant_id)
        .bind(&status_filter)
        .bind(&status_filter)
        .bind(&assigned_filter)
        .bind(&assigned_filter)
        .bind(if include_closed { 1 } else { 0 })
        .bind(if can_view_unassigned { 1 } else { 0 })
        .bind(if has_full_visibility { 1 } else { 0 })
        .bind(actor_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn get_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
    ) -> AppResult<InstallationWorkOrderView> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "read")
            .await?;

        let actor_role_name = self.get_actor_role_name(tenant_id, actor_id).await?;
        let is_admin_owner = matches!(actor_role_name.as_deref(), Some("owner") | Some("admin"));
        let can_manage_work_orders = self
            .auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await
            .is_ok();
        let has_full_visibility = is_admin_owner
            || (can_manage_work_orders && !Self::is_technician_role(actor_role_name.as_deref()));
        let visibility_mode = self
            .resolve_installation_work_order_visibility_mode(tenant_id)
            .await;
        let can_view_unassigned = has_full_visibility
            || Self::should_non_admin_see_unassigned_installation_work_orders(visibility_mode);

        #[cfg(feature = "postgres")]
        let row: Option<InstallationWorkOrderView> = sqlx::query_as(
            r#"
            SELECT
              wo.id, wo.tenant_id, wo.subscription_id, wo.invoice_id, wo.customer_id, wo.location_id,
              cs.package_id AS package_id,
              COALESCE(wo.router_id, cs.router_id) AS router_id,
              wo.status, wo.assigned_to, wo.scheduled_at, wo.completed_at, wo.notes, wo.created_at, wo.updated_at,
              c.name AS customer_name,
              c.phone AS customer_phone,
              l.label AS location_label,
              l.latitude::float8 AS location_latitude,
              l.longitude::float8 AS location_longitude,
              p.name AS package_name,
              p.provisioning_type AS package_provisioning_type,
              r.name AS router_name,
              u.name AS assigned_to_name,
              u.email AS assigned_to_email,
              csa.id AS assignment_id,
              csa.status AS assignment_status,
              cs.status AS subscription_status,
              cs.starts_at AS subscription_starts_at,
              cs.grace_until AS subscription_grace_until,
              EXISTS(
                SELECT 1
                FROM invoices i
                WHERE i.tenant_id = wo.tenant_id
                  AND (
                    i.external_id = 'pkgsub:' || wo.subscription_id
                    OR i.external_id LIKE 'pkgsub:' || wo.subscription_id || ':%'
                  )
              ) AS has_customer_package_invoice,
              csa.selected_zone_id AS selected_zone_id,
              sz.name AS selected_zone_name,
              csa.selected_node_id AS selected_node_id,
              nn.name AS selected_node_name,
              csa.selected_node_score::float8 AS selected_node_score,
              csa.path_node_ids AS path_node_ids,
              csa.path_link_ids AS path_link_ids
            FROM installation_work_orders wo
            LEFT JOIN customers c ON c.tenant_id = wo.tenant_id AND c.id = wo.customer_id
            LEFT JOIN customer_locations l ON l.tenant_id = wo.tenant_id AND l.id = wo.location_id
            LEFT JOIN customer_subscriptions cs ON cs.tenant_id = wo.tenant_id AND cs.id = wo.subscription_id
            LEFT JOIN isp_packages p ON p.tenant_id = wo.tenant_id AND p.id = cs.package_id
            LEFT JOIN mikrotik_routers r
              ON r.tenant_id = wo.tenant_id
             AND r.id = COALESCE(wo.router_id, cs.router_id)
            LEFT JOIN users u ON u.id = wo.assigned_to
            LEFT JOIN customer_service_assignments csa ON csa.tenant_id = wo.tenant_id AND csa.work_order_id = wo.id
            LEFT JOIN service_zones sz ON sz.tenant_id = wo.tenant_id::uuid AND sz.id::text = csa.selected_zone_id
            LEFT JOIN network_nodes nn ON nn.tenant_id = wo.tenant_id::uuid AND nn.id::text = csa.selected_node_id
            WHERE wo.tenant_id = $1 AND wo.id = $2
              AND (
                $3::bool
                OR wo.assigned_to = $4
                OR (
                  $5::bool
                  AND wo.status = 'pending'
                  AND (wo.assigned_to IS NULL OR btrim(wo.assigned_to) = '')
                )
              )
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .bind(has_full_visibility)
        .bind(actor_id)
        .bind(can_view_unassigned)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: Option<InstallationWorkOrderView> = sqlx::query_as(
            r#"
            SELECT
              wo.id, wo.tenant_id, wo.subscription_id, wo.invoice_id, wo.customer_id, wo.location_id,
              cs.package_id AS package_id,
              COALESCE(wo.router_id, cs.router_id) AS router_id,
              wo.status, wo.assigned_to, wo.scheduled_at, wo.completed_at, wo.notes, wo.created_at, wo.updated_at,
              c.name AS customer_name,
              c.phone AS customer_phone,
              l.label AS location_label,
              l.latitude::float8 AS location_latitude,
              l.longitude::float8 AS location_longitude,
              p.name AS package_name,
              p.provisioning_type AS package_provisioning_type,
              r.name AS router_name,
              u.name AS assigned_to_name,
              u.email AS assigned_to_email,
              csa.id AS assignment_id,
              csa.status AS assignment_status,
              cs.status AS subscription_status,
              cs.starts_at AS subscription_starts_at,
              cs.grace_until AS subscription_grace_until,
              EXISTS(
                SELECT 1
                FROM invoices i
                WHERE i.tenant_id = wo.tenant_id
                  AND (
                    i.external_id = 'pkgsub:' || wo.subscription_id
                    OR i.external_id LIKE 'pkgsub:' || wo.subscription_id || ':%'
                  )
              ) AS has_customer_package_invoice,
              csa.selected_zone_id AS selected_zone_id,
              sz.name AS selected_zone_name,
              csa.selected_node_id AS selected_node_id,
              nn.name AS selected_node_name,
              csa.selected_node_score AS selected_node_score,
              csa.path_node_ids AS path_node_ids,
              csa.path_link_ids AS path_link_ids
            FROM installation_work_orders wo
            LEFT JOIN customers c ON c.tenant_id = wo.tenant_id AND c.id = wo.customer_id
            LEFT JOIN customer_locations l ON l.tenant_id = wo.tenant_id AND l.id = wo.location_id
            LEFT JOIN customer_subscriptions cs ON cs.tenant_id = wo.tenant_id AND cs.id = wo.subscription_id
            LEFT JOIN isp_packages p ON p.tenant_id = wo.tenant_id AND p.id = cs.package_id
            LEFT JOIN mikrotik_routers r
              ON r.tenant_id = wo.tenant_id
             AND r.id = COALESCE(wo.router_id, cs.router_id)
            LEFT JOIN users u ON u.id = wo.assigned_to
            LEFT JOIN customer_service_assignments csa ON csa.tenant_id = wo.tenant_id AND csa.work_order_id = wo.id
            LEFT JOIN service_zones sz ON sz.tenant_id = wo.tenant_id AND sz.id = csa.selected_zone_id
            LEFT JOIN network_nodes nn ON nn.tenant_id = wo.tenant_id AND nn.id = csa.selected_node_id
            WHERE wo.tenant_id = ? AND wo.id = ?
              AND (
                ? = 1
                OR wo.assigned_to = ?
                OR (
                  ? = 1
                  AND wo.status = 'pending'
                  AND (wo.assigned_to IS NULL OR trim(wo.assigned_to) = '')
                )
              )
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .bind(if has_full_visibility { 1 } else { 0 })
        .bind(actor_id)
        .bind(if can_view_unassigned { 1 } else { 0 })
        .fetch_optional(&self.pool)
        .await?;

        row.ok_or_else(|| AppError::NotFound("Work order not found".to_string()))
    }

    pub async fn assign_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        assigned_to: &str,
        scheduled_at: Option<String>,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        let current = self
            .get_installation_work_order_row(tenant_id, work_order_id)
            .await?;
        let is_admin_owner = self.is_actor_admin_or_owner(tenant_id, actor_id).await?;
        if !is_admin_owner {
            // Technician is allowed to save schedule/notes for own pending or in-progress work order,
            // but cannot reassign to another user.
            if current.status != "pending" && current.status != "in_progress" {
                return Err(AppError::Validation(
                    "Only pending or in-progress work order can be updated".to_string(),
                ));
            }

            let current_assigned = current.assigned_to.as_deref().map(str::trim).unwrap_or("");
            if current_assigned != actor_id {
                return Err(AppError::Forbidden(
                    "Technician can only update own assigned work order".to_string(),
                ));
            }

            if assigned_to.trim() != actor_id {
                return Err(AppError::Forbidden(
                    "Technician cannot reassign installation work order".to_string(),
                ));
            }
        }

        let assignee_eligible = self
            .is_installation_assignee_eligible(tenant_id, assigned_to)
            .await?;
        if !assignee_eligible {
            return Err(AppError::Validation(
                "Assignee must be an eligible installer (Admin/Technician or role with work_orders:manage)"
                    .to_string(),
            ));
        }

        let row = self
            .set_installation_work_order_status_internal(
                actor_id,
                tenant_id,
                work_order_id,
                if current.status == "pending" {
                    Some("pending")
                } else {
                    None
                },
                Some(assigned_to),
                scheduled_at,
                notes,
                false,
                ip_address,
                "WORK_ORDER_ASSIGN",
                "Assigned installation work order",
            )
            .await?;

        let previous_assigned = current.assigned_to.as_deref().map(str::trim).unwrap_or("");
        let next_assigned = row.assigned_to.as_deref().map(str::trim).unwrap_or("");
        if is_admin_owner && !next_assigned.is_empty() && previous_assigned != next_assigned {
            let _ = self
                .notify_installation_work_order_assigned(tenant_id, &row, actor_id)
                .await;
        }

        Ok(row)
    }

    pub async fn claim_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        let eligible = self
            .is_installation_assignee_eligible(tenant_id, actor_id)
            .await?;
        if !eligible {
            return Err(AppError::Forbidden(
                "Only eligible installers can take installation work orders".to_string(),
            ));
        }

        let current = self
            .get_installation_work_order_row(tenant_id, work_order_id)
            .await?;
        if current.status != "pending" {
            return Err(AppError::Validation(
                "Only pending work order can be taken".to_string(),
            ));
        }
        if let Some(assigned) = current
            .assigned_to
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            if assigned != actor_id {
                return Err(AppError::Conflict(
                    "Work order already taken by another technician".to_string(),
                ));
            }
            return Ok(current);
        }

        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let affected = sqlx::query(
            r#"
            UPDATE installation_work_orders
            SET assigned_to = $1, updated_at = $2
            WHERE tenant_id = $3
              AND id = $4
              AND status = 'pending'
              AND (assigned_to IS NULL OR btrim(assigned_to) = '')
            "#,
        )
        .bind(actor_id)
        .bind(now)
        .bind(tenant_id)
        .bind(work_order_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        #[cfg(feature = "sqlite")]
        let affected = sqlx::query(
            r#"
            UPDATE installation_work_orders
            SET assigned_to = ?, updated_at = ?
            WHERE tenant_id = ?
              AND id = ?
              AND status = 'pending'
              AND (assigned_to IS NULL OR trim(assigned_to) = '')
            "#,
        )
        .bind(actor_id)
        .bind(now)
        .bind(tenant_id)
        .bind(work_order_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::Conflict(
                "Work order already taken by another technician".to_string(),
            ));
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "WORK_ORDER_CLAIM",
                "installation_work_orders",
                Some(work_order_id),
                Some("Technician took installation work order"),
                ip_address,
            )
            .await;

        let mut row = self
            .get_installation_work_order_row(tenant_id, work_order_id)
            .await?;
        if notes
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .is_some()
        {
            row = self
                .set_installation_work_order_status_internal(
                    actor_id,
                    tenant_id,
                    work_order_id,
                    None,
                    None,
                    None,
                    notes,
                    false,
                    ip_address,
                    "WORK_ORDER_UPDATE_NOTE",
                    "Updated work order notes while claiming",
                )
                .await?;
        }

        Ok(row)
    }

    pub async fn release_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        if !self.is_actor_admin_or_owner(tenant_id, actor_id).await? {
            return Err(AppError::Forbidden(
                "Only admin/owner can release installation work orders".to_string(),
            ));
        }

        let current = self
            .get_installation_work_order_row(tenant_id, work_order_id)
            .await?;
        if current.status != "pending" {
            return Err(AppError::Validation(
                "Only pending work order can be released".to_string(),
            ));
        }

        // Release means making assignment empty and clearing schedule so next assignee can re-plan.
        self.set_installation_work_order_status_internal(
            actor_id,
            tenant_id,
            work_order_id,
            Some("pending"),
            Some(""),
            Some("".to_string()),
            notes,
            false,
            ip_address,
            "WORK_ORDER_RELEASE",
            "Released installation work order assignment",
        )
        .await
    }

    pub async fn start_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        let is_admin = self.is_actor_admin_or_owner(tenant_id, actor_id).await?;
        if !is_admin {
            let current = self
                .get_installation_work_order_row(tenant_id, work_order_id)
                .await?;
            let assigned = current.assigned_to.as_deref().map(str::trim).unwrap_or("");
            if assigned != actor_id {
                return Err(AppError::Forbidden(
                    "Technician can only start own assigned work order".to_string(),
                ));
            }
        }

        self.set_installation_work_order_status_internal(
            actor_id,
            tenant_id,
            work_order_id,
            Some("in_progress"),
            None,
            None,
            notes,
            false,
            ip_address,
            "WORK_ORDER_START",
            "Started installation work order",
        )
        .await
    }

    pub async fn complete_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        notes: Option<String>,
        terminal_asset_id: Option<String>,
        parent_asset_id: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        let is_admin = self.is_actor_admin_or_owner(tenant_id, actor_id).await?;
        if !is_admin {
            let current = self
                .get_installation_work_order_row(tenant_id, work_order_id)
                .await?;
            let assigned = current.assigned_to.as_deref().map(str::trim).unwrap_or("");
            if assigned != actor_id {
                return Err(AppError::Forbidden(
                    "Technician can only complete own assigned work order".to_string(),
                ));
            }
        }

        let terminal_asset_id = terminal_asset_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::Validation("Terminal asset must be selected before completion".into())
            })?;
        let parent_asset_id = parent_asset_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let row = self
            .set_installation_work_order_status_internal(
                actor_id,
                tenant_id,
                work_order_id,
                Some("completed"),
                None,
                None,
                notes,
                false,
                ip_address,
                "WORK_ORDER_COMPLETE",
                "Completed installation work order",
            )
            .await?;

        self.bind_assets_to_completed_installation(
            tenant_id,
            &row,
            &terminal_asset_id,
            parent_asset_id.as_deref(),
        )
        .await?;

        #[cfg(feature = "postgres")]
        let sub: Option<CustomerSubscription> = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price::float8 as price, currency_code, status, starts_at, ends_at, grace_started_at, grace_until, notes, created_at, updated_at FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(&row.subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let sub: Option<CustomerSubscription> = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price as price, currency_code, status, starts_at, ends_at, grace_started_at, grace_until, notes, created_at, updated_at FROM customer_subscriptions WHERE tenant_id = ? AND id = ?",
        )
        .bind(tenant_id)
        .bind(&row.subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(mut s) = sub {
            if s.status != "cancelled" {
                let has_paid_invoice = self
                    .has_paid_customer_package_invoice_for_subscription(tenant_id, &s.id)
                    .await?;
                let current = SubscriptionLifecycleStatus::parse(&s.status)
                    .map_err(|e| AppError::Validation(e.to_string()))?;
                let resolved = resolve_activation_status(current, true, has_paid_invoice)
                    .map_err(|e| AppError::Validation(e.to_string()))?;

                let updated = self
                    .set_customer_subscription_status(
                        tenant_id,
                        &s.id,
                        current.as_str(),
                        resolved.as_str(),
                    )
                    .await?;
                if !updated {
                    return Err(AppError::Validation(
                        "Subscription status changed concurrently; retry completion".to_string(),
                    ));
                }
                s.status = resolved.as_str().to_string();
                s.updated_at = Utc::now();

                if resolved == SubscriptionLifecycleStatus::GraceActive {
                    let grace_started_at = Utc::now();
                    let grace_hours = self.resolve_installation_grace_hours(tenant_id).await;
                    let grace_until = grace_started_at + Duration::hours(grace_hours);
                    let _ = self
                        .set_customer_subscription_grace_window(
                            tenant_id,
                            &s.id,
                            grace_started_at,
                            grace_until,
                        )
                        .await?;
                    s.grace_started_at = Some(grace_started_at);
                    s.grace_until = Some(grace_until);
                } else {
                    s.grace_started_at = None;
                    s.grace_until = None;
                }

                let should_disable_pppoe =
                    should_disable_pppoe_for_subscription_status(resolved.as_str());
                let _ = self
                    .set_location_pppoe_disabled_state(
                        tenant_id,
                        &s.location_id,
                        should_disable_pppoe,
                    )
                    .await;

                let _ = self
                    .auto_provision_pppoe_for_subscription(actor_id, tenant_id, &s, ip_address)
                    .await;

                if Self::should_auto_create_first_invoice_on_completion(resolved, has_paid_invoice)
                {
                    let payment_service = PaymentService::new(
                        self.pool.clone(),
                        self.notification_service.clone(),
                        self.pppoe_service.clone(),
                        self.audit_service.clone(),
                    );

                    match payment_service
                        .create_invoice_for_customer_subscription(tenant_id, &s.id)
                        .await
                    {
                        Ok(invoice) => {
                            if let Err(err) = self
                                .attach_installation_work_order_invoice(
                                    tenant_id,
                                    work_order_id,
                                    &invoice.id,
                                )
                                .await
                            {
                                tracing::warn!(
                                    "failed to link installation work order invoice: tenant={}, work_order={}, invoice={}, error={}",
                                    tenant_id,
                                    work_order_id,
                                    invoice.id,
                                    err
                                );
                            }
                        }
                        Err(err) => {
                            tracing::warn!(
                                "failed to auto-create first invoice after installation completion: tenant={}, work_order={}, subscription={}, error={}",
                                tenant_id,
                                work_order_id,
                                s.id,
                                err
                            );
                        }
                    }
                }
            }
        }

        Ok(row)
    }

    pub async fn cancel_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        if !self.is_actor_admin_or_owner(tenant_id, actor_id).await? {
            return Err(AppError::Forbidden(
                "Only admin/owner can cancel installation work orders".to_string(),
            ));
        }

        let reason = notes
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                AppError::Validation(
                    "Cancellation reason is required (minimum 10 characters)".to_string(),
                )
            })?
            .to_string();

        if reason.chars().count() < 10 {
            return Err(AppError::Validation(
                "Cancellation reason is too short (minimum 10 characters)".to_string(),
            ));
        }

        let row = self
            .set_installation_work_order_status_internal(
                actor_id,
                tenant_id,
                work_order_id,
                Some("cancelled"),
                None,
                None,
                notes,
                false,
                ip_address,
                "WORK_ORDER_CANCEL",
                "Cancelled installation work order",
            )
            .await?;

        self.transition_customer_subscription_status(
            tenant_id,
            &row.subscription_id,
            SubscriptionLifecycleEvent::Cancel,
        )
        .await?;

        if let Err(err) = self
            .notify_customer_installation_cancelled(tenant_id, &row.subscription_id, &reason)
            .await
        {
            warn!(
                "failed to send installation cancellation notification: tenant_id={}, work_order_id={}, error={}",
                tenant_id, row.id, err
            );
        }

        Ok(row)
    }

    pub async fn reopen_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        let row = self
            .set_installation_work_order_status_internal(
                actor_id,
                tenant_id,
                work_order_id,
                Some("pending"),
                None,
                None,
                notes,
                true,
                ip_address,
                "WORK_ORDER_REOPEN",
                "Reopened installation work order",
            )
            .await?;

        self.transition_customer_subscription_status(
            tenant_id,
            &row.subscription_id,
            SubscriptionLifecycleEvent::Reopen,
        )
        .await?;

        Ok(row)
    }

    pub(super) async fn set_installation_work_order_status_internal(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        new_status: Option<&str>,
        assigned_to: Option<&str>,
        scheduled_at: Option<String>,
        notes: Option<String>,
        allow_closed_update: bool,
        ip_address: Option<&str>,
        audit_action: &str,
        audit_desc: &str,
    ) -> AppResult<InstallationWorkOrder> {
        #[cfg(feature = "postgres")]
        let mut row: InstallationWorkOrder = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let mut row: InstallationWorkOrder = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = ? AND id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

        if allow_closed_update && row.status != "cancelled" {
            return Err(AppError::Validation(
                "Only cancelled work order can be reopened".to_string(),
            ));
        }

        if row.status == "completed" {
            return Err(AppError::Validation(
                "Closed work order cannot be changed".to_string(),
            ));
        }
        if row.status == "cancelled" {
            if !allow_closed_update {
                return Err(AppError::Validation(
                    "Cancelled work order cannot be changed. Reopen it first.".to_string(),
                ));
            }
            if new_status != Some("pending") {
                return Err(AppError::Validation(
                    "Cancelled work order can only be reopened to pending status".to_string(),
                ));
            }
        }

        let normalized_new_status = match new_status {
            Some(s) => Some(Self::normalize_work_order_status(s)?),
            None => None,
        };

        if let Some(target_status) = normalized_new_status.as_deref() {
            match target_status {
                "pending" => {
                    if row.status == "in_progress" && !allow_closed_update {
                        return Err(AppError::Validation(
                            "In-progress work order cannot be moved back to pending".to_string(),
                        ));
                    }
                }
                "in_progress" => {
                    if row.status != "pending" {
                        return Err(AppError::Validation(
                            "Only pending work order can be started".to_string(),
                        ));
                    }
                    if row
                        .assigned_to
                        .as_deref()
                        .map(str::trim)
                        .unwrap_or("")
                        .is_empty()
                    {
                        return Err(AppError::Validation(
                            "Set assignee before starting work order".to_string(),
                        ));
                    }
                    if row.scheduled_at.is_none() {
                        return Err(AppError::Validation(
                            "Set installation schedule before starting work order".to_string(),
                        ));
                    }
                }
                "completed" => {
                    if row.status != "in_progress" {
                        return Err(AppError::Validation(
                            "Only in-progress work order can be completed".to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }

        if let Some(s) = normalized_new_status {
            row.status = s;
            row.completed_at = if row.status == "completed" {
                Some(Utc::now())
            } else {
                None
            };
        }
        if let Some(uid) = assigned_to {
            let normalized_uid = uid.trim();
            row.assigned_to = if normalized_uid.is_empty() {
                None
            } else {
                Some(normalized_uid.to_string())
            };
        }
        if scheduled_at.is_some() {
            row.scheduled_at = Self::parse_optional_datetime(scheduled_at)?;
        }
        row.notes = Self::merge_work_order_notes(row.notes, actor_id, notes.as_deref());
        row.updated_at = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE installation_work_orders
            SET status = $1,
                assigned_to = $2,
                scheduled_at = $3,
                completed_at = $4,
                notes = $5,
                updated_at = $6
            WHERE tenant_id = $7 AND id = $8
            "#,
        )
        .bind(&row.status)
        .bind(&row.assigned_to)
        .bind(row.scheduled_at)
        .bind(row.completed_at)
        .bind(&row.notes)
        .bind(row.updated_at)
        .bind(tenant_id)
        .bind(work_order_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE installation_work_orders
            SET status = ?,
                assigned_to = ?,
                scheduled_at = ?,
                completed_at = ?,
                notes = ?,
                updated_at = ?
            WHERE tenant_id = ? AND id = ?
            "#,
        )
        .bind(&row.status)
        .bind(&row.assigned_to)
        .bind(row.scheduled_at)
        .bind(row.completed_at)
        .bind(&row.notes)
        .bind(row.updated_at)
        .bind(tenant_id)
        .bind(work_order_id)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                audit_action,
                "installation_work_orders",
                Some(work_order_id),
                Some(audit_desc),
                ip_address,
            )
            .await;

        Ok(row)
    }

    async fn notify_customer_installation_cancelled(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        reason: &str,
    ) -> AppResult<()> {
        let user_ids = self
            .list_customer_user_ids_for_subscription(tenant_id, subscription_id)
            .await?;
        if user_ids.is_empty() {
            return Ok(());
        }

        let short_reason = reason.trim();
        let message = format!(
            "Your installation request was cancelled by admin/technician. Reason: {}. You can request reopen from Services page.",
            short_reason
        );

        for user_id in user_ids {
            self.notification_service
                .create_notification(
                    user_id,
                    Some(tenant_id.to_string()),
                    "Installation Request Cancelled".to_string(),
                    message.clone(),
                    "warning".to_string(),
                    "operations".to_string(),
                    Some("/dashboard/services".to_string()),
                )
                .await?;
        }

        Ok(())
    }

    async fn notify_installation_work_order_assigned(
        &self,
        tenant_id: &str,
        work_order: &InstallationWorkOrder,
        actor_id: &str,
    ) -> AppResult<()> {
        let Some(assignee_id) = work_order.assigned_to.as_deref().map(str::trim) else {
            return Ok(());
        };
        if assignee_id.is_empty() || assignee_id == actor_id {
            return Ok(());
        }

        #[cfg(feature = "postgres")]
        let row: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT c.name, l.label, p.name
            FROM installation_work_orders wo
            LEFT JOIN customers c ON c.tenant_id = wo.tenant_id AND c.id = wo.customer_id
            LEFT JOIN customer_locations l ON l.tenant_id = wo.tenant_id AND l.id = wo.location_id
            LEFT JOIN customer_subscriptions cs ON cs.tenant_id = wo.tenant_id AND cs.id = wo.subscription_id
            LEFT JOIN isp_packages p ON p.tenant_id = wo.tenant_id AND p.id = cs.package_id
            WHERE wo.tenant_id = $1 AND wo.id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&work_order.id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT c.name, l.label, p.name
            FROM installation_work_orders wo
            LEFT JOIN customers c ON c.tenant_id = wo.tenant_id AND c.id = wo.customer_id
            LEFT JOIN customer_locations l ON l.tenant_id = wo.tenant_id AND l.id = wo.location_id
            LEFT JOIN customer_subscriptions cs ON cs.tenant_id = wo.tenant_id AND cs.id = wo.subscription_id
            LEFT JOIN isp_packages p ON p.tenant_id = wo.tenant_id AND p.id = cs.package_id
            WHERE wo.tenant_id = ? AND wo.id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&work_order.id)
        .fetch_optional(&self.pool)
        .await?;

        let (customer_name, location_label, package_name) = row.unwrap_or((None, None, None));
        let mut message = format!(
            "A work order has been assigned to you (WO {}).",
            work_order.id
        );
        if customer_name.is_some() || location_label.is_some() || package_name.is_some() {
            let customer = customer_name.unwrap_or_else(|| "Customer".to_string());
            let location = location_label.unwrap_or_else(|| "-".to_string());
            let package = package_name.unwrap_or_else(|| "-".to_string());
            message = format!(
                "A work order has been assigned to you for {} at {} ({}) (WO {}).",
                customer, location, package, work_order.id
            );
        }

        self.notification_service
            .create_notification(
                assignee_id.to_string(),
                Some(tenant_id.to_string()),
                "Installation Work Order Assigned".to_string(),
                message,
                "info".to_string(),
                "operations".to_string(),
                Some("/admin/network/installations".to_string()),
            )
            .await?;

        Ok(())
    }
}
