use super::*;

impl CustomerService {
    pub(super) async fn get_actor_role_name(
        &self,
        tenant_id: &str,
        actor_id: &str,
    ) -> AppResult<Option<String>> {
        #[cfg(feature = "postgres")]
        let role_name: Option<String> = sqlx::query_scalar(
            r#"
            SELECT LOWER(COALESCE(r.name, tm.role, ''))
            FROM tenant_members tm
            LEFT JOIN roles r ON r.id = tm.role_id
            WHERE tm.tenant_id = $1 AND tm.user_id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let role_name: Option<String> = sqlx::query_scalar(
            r#"
            SELECT LOWER(COALESCE(r.name, tm.role, ''))
            FROM tenant_members tm
            LEFT JOIN roles r ON r.id = tm.role_id
            WHERE tm.tenant_id = ? AND tm.user_id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(role_name)
    }

    pub(super) async fn get_installation_work_order_row(
        &self,
        tenant_id: &str,
        work_order_id: &str,
    ) -> AppResult<InstallationWorkOrder> {
        #[cfg(feature = "postgres")]
        let row: Option<InstallationWorkOrder> = sqlx::query_as(
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
        .await?;

        #[cfg(feature = "sqlite")]
        let row: Option<InstallationWorkOrder> = sqlx::query_as(
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
        .await?;

        row.ok_or_else(|| AppError::NotFound("Work order not found".to_string()))
    }

    pub(super) async fn is_actor_admin_or_owner(
        &self,
        tenant_id: &str,
        actor_id: &str,
    ) -> AppResult<bool> {
        let role_name = self.get_actor_role_name(tenant_id, actor_id).await?;

        Ok(matches!(
            role_name.as_deref(),
            Some("owner") | Some("admin")
        ))
    }

    pub(super) async fn is_installation_assignee_eligible(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> AppResult<bool> {
        #[cfg(feature = "postgres")]
        let eligible: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM tenant_members tm
              JOIN users u ON u.id = tm.user_id
              LEFT JOIN roles r ON r.id = tm.role_id
              WHERE tm.tenant_id = $1
                AND tm.user_id = $2
                AND u.is_active = TRUE
                AND (
                  EXISTS(
                    SELECT 1
                    FROM role_permissions rp
                    JOIN permissions p ON p.id = rp.permission_id
                    WHERE rp.role_id = tm.role_id
                      AND p.resource = 'work_orders'
                      AND p.action = 'manage'
                  )
                  OR LOWER(COALESCE(r.name, tm.role, '')) IN ('owner', 'admin', 'technician', 'teknisi')
                )
            )
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let eligible: bool = {
            let raw: i64 = sqlx::query_scalar(
                r#"
                SELECT EXISTS(
                  SELECT 1
                  FROM tenant_members tm
                  JOIN users u ON u.id = tm.user_id
                  LEFT JOIN roles r ON r.id = tm.role_id
                  WHERE tm.tenant_id = ?
                    AND tm.user_id = ?
                    AND u.is_active = 1
                    AND (
                      EXISTS(
                        SELECT 1
                        FROM role_permissions rp
                        JOIN permissions p ON p.id = rp.permission_id
                        WHERE rp.role_id = tm.role_id
                          AND p.resource = 'work_orders'
                          AND p.action = 'manage'
                      )
                      OR LOWER(COALESCE(r.name, tm.role, '')) IN ('owner', 'admin', 'technician', 'teknisi')
                    )
                )
                "#,
            )
            .bind(tenant_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
            raw != 0
        };

        Ok(eligible)
    }

    pub async fn list_installation_assignees(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<Vec<TeamMemberWithUser>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<TeamMemberWithUser> = sqlx::query_as(
            r#"
            SELECT
              tm.id,
              tm.user_id,
              u.name,
              u.email,
              tm.role,
              tm.role_id,
              r.name AS role_name,
              u.is_active,
              tm.created_at,
              r.level AS role_level,
              tm.deleted_at
            FROM tenant_members tm
            JOIN users u ON tm.user_id = u.id
            LEFT JOIN roles r ON tm.role_id = r.id
            WHERE tm.tenant_id = $1
              AND u.is_active = TRUE
              AND tm.deleted_at IS NULL
              AND (
                EXISTS(
                  SELECT 1
                  FROM role_permissions rp
                  JOIN permissions p ON p.id = rp.permission_id
                  WHERE rp.role_id = tm.role_id
                    AND p.resource = 'work_orders'
                    AND p.action = 'manage'
                )
                OR LOWER(COALESCE(r.name, tm.role, '')) IN ('owner', 'admin', 'technician', 'teknisi')
              )
            ORDER BY LOWER(u.name), LOWER(u.email)
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<TeamMemberWithUser> = sqlx::query_as(
            r#"
            SELECT
              tm.id,
              tm.user_id,
              u.name,
              u.email,
              tm.role,
              tm.role_id,
              r.name AS role_name,
              u.is_active,
              tm.created_at,
              r.level AS role_level,
              tm.deleted_at
            FROM tenant_members tm
            JOIN users u ON tm.user_id = u.id
            LEFT JOIN roles r ON tm.role_id = r.id
            WHERE tm.tenant_id = ?
              AND u.is_active = 1
              AND tm.deleted_at IS NULL
              AND (
                EXISTS(
                  SELECT 1
                  FROM role_permissions rp
                  JOIN permissions p ON p.id = rp.permission_id
                  WHERE rp.role_id = tm.role_id
                    AND p.resource = 'work_orders'
                    AND p.action = 'manage'
                )
                OR LOWER(COALESCE(r.name, tm.role, '')) IN ('owner', 'admin', 'technician', 'teknisi')
              )
            ORDER BY LOWER(u.name), LOWER(u.email)
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub(super) async fn get_system_role_id_by_name(&self, name: &str) -> AppResult<String> {
        #[cfg(feature = "postgres")]
        let row: Option<(String,)> =
            sqlx::query_as("SELECT id FROM roles WHERE tenant_id IS NULL AND name = $1")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let row: Option<(String,)> =
            sqlx::query_as("SELECT id FROM roles WHERE tenant_id IS NULL AND name = ?")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;

        row.map(|(id,)| id).ok_or_else(|| {
            AppError::Internal(format!(
                "Missing system role '{}'. Ensure RoleService seeds default roles.",
                name
            ))
        })
    }

    pub(super) async fn ensure_tenant_member_role(
        &self,
        tenant_id: &str,
        user_id: &str,
        role_id: &str,
    ) -> AppResult<()> {
        // If user already has membership in this tenant, do not overwrite role.
        #[cfg(feature = "postgres")]
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM tenant_members WHERE tenant_id = $1 AND user_id = $2)",
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM tenant_members WHERE tenant_id = ? AND user_id = ?)",
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if exists {
            return Ok(());
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(user_id)
        .bind("customer")
        .bind(role_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(user_id)
        .bind("customer")
        .bind(role_id)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(super) async fn resolve_installation_sla_overdue_minutes(&self) -> i64 {
        let raw = self
            .read_global_setting_value(INSTALLATION_SLA_OVERDUE_MINUTES_KEY)
            .await
            .ok()
            .flatten();
        Self::parse_setting_i64(raw, 120, 15, 7 * 24 * 60)
    }

    pub(super) async fn resolve_installation_sla_reminder_cooldown_minutes(&self) -> i64 {
        let raw = self
            .read_global_setting_value(INSTALLATION_SLA_REMINDER_COOLDOWN_MINUTES_KEY)
            .await
            .ok()
            .flatten();
        Self::parse_setting_i64(raw, 180, 15, 7 * 24 * 60)
    }

    pub(super) async fn resolve_installation_sla_scheduler_interval_minutes(&self) -> i64 {
        let default_global = self
            .read_global_setting_value(INSTALLATION_SLA_SCHEDULER_INTERVAL_MINUTES_KEY)
            .await
            .ok()
            .flatten();
        let default_global = Self::parse_setting_i64(default_global, 15, 5, 24 * 60);

        #[cfg(feature = "postgres")]
        let tenant_values: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT s.value
            FROM settings s
            INNER JOIN tenants t ON t.id = s.tenant_id
            WHERE s.key = $1
              AND t.is_active = true
            "#,
        )
        .bind(INSTALLATION_SLA_SCHEDULER_INTERVAL_MINUTES_KEY)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        #[cfg(feature = "sqlite")]
        let tenant_values: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT s.value
            FROM settings s
            INNER JOIN tenants t ON t.id = s.tenant_id
            WHERE s.key = ?
              AND t.is_active = 1
            "#,
        )
        .bind(INSTALLATION_SLA_SCHEDULER_INTERVAL_MINUTES_KEY)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        tenant_values
            .into_iter()
            .filter_map(|v| v.parse::<i64>().ok())
            .map(|v| v.clamp(5, 24 * 60))
            .min()
            .unwrap_or(default_global)
    }

    pub(super) async fn upsert_tenant_setting_value(
        &self,
        tenant_id: &str,
        key: &str,
        value: &str,
        description: &str,
    ) -> AppResult<()> {
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let update_res = sqlx::query(
            "UPDATE settings SET value = $1, description = $2, updated_at = $3 WHERE tenant_id = $4 AND key = $5",
        )
        .bind(value)
        .bind(description)
        .bind(now)
        .bind(tenant_id)
        .bind(key)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let update_res = sqlx::query(
            "UPDATE settings SET value = ?, description = ?, updated_at = ? WHERE tenant_id = ? AND key = ?",
        )
        .bind(value)
        .bind(description)
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(key)
        .execute(&self.pool)
        .await?;

        if update_res.rows_affected() == 0 {
            let id = Uuid::new_v4().to_string();

            #[cfg(feature = "postgres")]
            sqlx::query(
                "INSERT INTO settings (id, tenant_id, key, value, description, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$6)",
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(key)
            .bind(value)
            .bind(description)
            .bind(now)
            .execute(&self.pool)
            .await?;

            #[cfg(feature = "sqlite")]
            sqlx::query(
                "INSERT INTO settings (id, tenant_id, key, value, description, created_at, updated_at) VALUES (?,?,?,?,?,?,?)",
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(key)
            .bind(value)
            .bind(description)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub(super) async fn resolve_invite_policy_for_tenant(
        &self,
        tenant_id: &str,
    ) -> AppResult<CustomerRegistrationInvitePolicy> {
        let expires_raw = self
            .read_tenant_setting_value(tenant_id, INVITE_DEFAULT_EXPIRES_KEY)
            .await?;
        let max_uses_raw = self
            .read_tenant_setting_value(tenant_id, INVITE_DEFAULT_MAX_USES_KEY)
            .await?;

        Ok(CustomerRegistrationInvitePolicy {
            default_expires_in_hours: Self::parse_invite_policy_u32(
                expires_raw,
                INVITE_DEFAULT_EXPIRES_HOURS,
                1,
                24 * 30,
            ),
            default_max_uses: Self::parse_invite_policy_u32(
                max_uses_raw,
                INVITE_DEFAULT_MAX_USES,
                1,
                100,
            ),
        })
    }

    pub(super) async fn ensure_installation_work_order_for_subscription(
        &self,
        tenant_id: &str,
        sub: &CustomerSubscription,
    ) -> AppResult<(InstallationWorkOrder, bool)> {
        #[cfg(feature = "postgres")]
        let existing: Option<InstallationWorkOrder> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = $1
              AND subscription_id = $2
              AND status IN ('pending', 'in_progress')
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&sub.id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let existing: Option<InstallationWorkOrder> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = ?
              AND subscription_id = ?
              AND status IN ('pending', 'in_progress')
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&sub.id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = existing {
            return Ok((row, false));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let notes = Some(
            "Created from customer order request; awaiting assignment and schedule.".to_string(),
        );

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO installation_work_orders
              (id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, notes, created_at, updated_at)
            VALUES
              ($1,$2,$3,NULL,$4,$5,$6,'pending',$7,$8,$9)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(&sub.id)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(&sub.router_id)
        .bind(&notes)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO installation_work_orders
              (id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, notes, created_at, updated_at)
            VALUES
              (?,?,?,NULL,?,?,?,'pending',?,?,?)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(&sub.id)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(&sub.router_id)
        .bind(notes.clone())
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let row: InstallationWorkOrder = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: InstallationWorkOrder = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = ? AND id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&id)
        .fetch_one(&self.pool)
        .await?;

        Ok((row, true))
    }

    pub(super) async fn has_paid_customer_package_invoice_for_subscription(
        &self,
        tenant_id: &str,
        subscription_id: &str,
    ) -> AppResult<bool> {
        #[cfg(feature = "postgres")]
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM invoices
              WHERE tenant_id = $1
                AND status = 'paid'
                AND (
                    external_id = $2
                    OR external_id LIKE $3
                )
            )
            "#,
        )
        .bind(tenant_id)
        .bind(format!(
            "{}{}",
            CUSTOMER_PACKAGE_INVOICE_PREFIX, subscription_id
        ))
        .bind(format!(
            "{}{}:%",
            CUSTOMER_PACKAGE_INVOICE_PREFIX, subscription_id
        ))
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM invoices
              WHERE tenant_id = ?
                AND status = 'paid'
                AND (
                    external_id = ?
                    OR external_id LIKE ?
                )
            )
            "#,
        )
        .bind(tenant_id)
        .bind(format!(
            "{}{}",
            CUSTOMER_PACKAGE_INVOICE_PREFIX, subscription_id
        ))
        .bind(format!(
            "{}{}:%",
            CUSTOMER_PACKAGE_INVOICE_PREFIX, subscription_id
        ))
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }
}
