use super::*;
use crate::security::secret::encrypt_secret_for;

impl CustomerService {
    /// Resolve user_ids of every Owner/Admin in a given tenant. Used to
    /// fan-out in-app notifications for new self-registrations and other
    /// tenant-scoped events.
    async fn fetch_tenant_admin_user_ids_for_notification(
        &self,
        tenant_id: &str,
    ) -> AppResult<Vec<String>> {
        // Pull tenant_members linked to roles that should see registration alerts
        // (Owner/Admin). Falls back to `tm.role = 'owner'/'admin'` strings in case
        // role_id is NULL on legacy rows.
        #[cfg(feature = "postgres")]
        let ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT tm.user_id
            FROM tenant_members tm
            LEFT JOIN roles r ON r.id = tm.role_id
            WHERE tm.tenant_id = $1
              AND tm.deleted_at IS NULL
              AND (
                r.name IN ('Owner', 'Admin')
                OR lower(tm.role) IN ('owner', 'admin')
              )
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(not(feature = "postgres"))]
        let ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT tm.user_id
            FROM tenant_members tm
            LEFT JOIN roles r ON r.id = tm.role_id
            WHERE tm.tenant_id = ?
              AND tm.deleted_at IS NULL
              AND (
                r.name IN ('Owner', 'Admin')
                OR lower(tm.role) IN ('owner', 'admin')
              )
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        // De-duplicate (a user may appear via both role_id and tm.role).
        let mut seen = std::collections::HashSet::<String>::new();
        Ok(ids
            .into_iter()
            .filter(|id| seen.insert(id.clone()))
            .collect())
    }

    pub async fn run_installation_sla_reminders_for_all_tenants(&self) -> AppResult<u64> {
        if !self.resolve_installation_sla_reminder_enabled().await {
            return Ok(0);
        }

        let overdue_minutes = self.resolve_installation_sla_overdue_minutes().await;
        let unscheduled_minutes = (overdue_minutes * 2).max(120);
        let cooldown_minutes = self
            .resolve_installation_sla_reminder_cooldown_minutes()
            .await;

        #[cfg(feature = "postgres")]
        let tenant_ids: Vec<String> =
            sqlx::query_scalar("SELECT id FROM tenants WHERE is_active = true")
                .fetch_all(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let tenant_ids: Vec<String> =
            sqlx::query_scalar("SELECT id FROM tenants WHERE is_active = 1")
                .fetch_all(&self.pool)
                .await?;

        let mut sent = 0_u64;
        for tenant_id in tenant_ids {
            sent += self
                .run_installation_sla_reminders_for_tenant(
                    &tenant_id,
                    overdue_minutes,
                    unscheduled_minutes,
                    cooldown_minutes,
                )
                .await?;
        }

        Ok(sent)
    }

    pub async fn list_customers(
        &self,
        actor_id: &str,
        tenant_id: &str,
        q: Option<String>,
        status: Option<String>,
        service: Option<String>,
        installation: Option<String>,
        page: u32,
        per_page: u32,
    ) -> AppResult<PaginatedResponse<CustomerListItem>> {
        if self
            .auth_service
            .check_permission(actor_id, tenant_id, "customers", "read")
            .await
            .is_err()
        {
            self.auth_service
                .check_permission(actor_id, tenant_id, "orders", "create")
                .await?;
        }

        let q = q.unwrap_or_default().trim().to_string();
        let status = match status.unwrap_or_default().trim() {
            "active" => "active".to_string(),
            "inactive" => "inactive".to_string(),
            _ => String::new(),
        };
        let service = match service.unwrap_or_default().trim() {
            "active" => "active".to_string(),
            "inactive" => "inactive".to_string(),
            "none" => "none".to_string(),
            _ => String::new(),
        };
        let installation = match installation.unwrap_or_default().trim() {
            "pending" => "pending".to_string(),
            _ => String::new(),
        };
        let pg = crate::services::pagination::normalize(page, per_page);
        let per_page = pg.per_page;
        let offset = pg.offset;

        #[cfg(feature = "postgres")]
        let query = r#"
            WITH subscription_rollup AS (
                SELECT
                    cs.tenant_id,
                    cs.customer_id,
                    COUNT(*)::bigint AS subscription_count,
                    COALESCE(SUM(CASE WHEN cs.status IN ('active', 'grace_active') THEN 1 ELSE 0 END), 0)::bigint AS active_subscriptions,
                    COALESCE(SUM(CASE WHEN cs.status = 'pending_installation' THEN 1 ELSE 0 END), 0)::bigint AS pending_installations
                FROM customer_subscriptions cs
                WHERE cs.tenant_id = $1
                GROUP BY cs.tenant_id, cs.customer_id
            )
            SELECT
                c.*,
                COALESCE(svc.subscription_count, 0) AS subscription_count,
                COALESCE(svc.active_subscriptions, 0) AS active_subscriptions,
                COALESCE(svc.pending_installations, 0) AS pending_installations,
                CASE
                    WHEN COALESCE(svc.active_subscriptions, 0) > 0 THEN 'active'
                    WHEN COALESCE(svc.subscription_count, 0) > 0 THEN 'inactive'
                    ELSE 'none'
                END AS service_status,
                COUNT(*) OVER() AS total_count
            FROM customers c
            LEFT JOIN subscription_rollup svc
              ON svc.tenant_id = c.tenant_id AND svc.customer_id = c.id
            WHERE c.tenant_id = $1
              AND ($2 = '' OR c.name ILIKE '%' || $2 || '%' OR c.email ILIKE '%' || $2 || '%')
              AND ($3 = '' OR ($3 = 'active' AND c.is_active) OR ($3 = 'inactive' AND NOT c.is_active))
              AND (
                  $4 = ''
                  OR ($4 = 'active' AND COALESCE(svc.active_subscriptions, 0) > 0)
                  OR ($4 = 'inactive' AND COALESCE(svc.subscription_count, 0) > 0 AND COALESCE(svc.active_subscriptions, 0) = 0)
                  OR ($4 = 'none' AND COALESCE(svc.subscription_count, 0) = 0)
              )
              AND ($5 = '' OR ($5 = 'pending' AND COALESCE(svc.pending_installations, 0) > 0))
            ORDER BY c.created_at DESC
            LIMIT $6 OFFSET $7
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            WITH subscription_rollup AS (
                SELECT
                    cs.tenant_id,
                    cs.customer_id,
                    COUNT(*) AS subscription_count,
                    COALESCE(SUM(CASE WHEN cs.status IN ('active', 'grace_active') THEN 1 ELSE 0 END), 0) AS active_subscriptions,
                    COALESCE(SUM(CASE WHEN cs.status = 'pending_installation' THEN 1 ELSE 0 END), 0) AS pending_installations
                FROM customer_subscriptions cs
                WHERE cs.tenant_id = ?
                GROUP BY cs.tenant_id, cs.customer_id
            )
            SELECT
                c.*,
                COALESCE(svc.subscription_count, 0) AS subscription_count,
                COALESCE(svc.active_subscriptions, 0) AS active_subscriptions,
                COALESCE(svc.pending_installations, 0) AS pending_installations,
                CASE
                    WHEN COALESCE(svc.active_subscriptions, 0) > 0 THEN 'active'
                    WHEN COALESCE(svc.subscription_count, 0) > 0 THEN 'inactive'
                    ELSE 'none'
                END AS service_status,
                COUNT(*) OVER() AS total_count
            FROM customers c
            LEFT JOIN subscription_rollup svc
              ON svc.tenant_id = c.tenant_id AND svc.customer_id = c.id
            WHERE c.tenant_id = ?
              AND (? = '' OR c.name LIKE '%' || ? || '%' OR c.email LIKE '%' || ? || '%')
              AND (? = '' OR (? = 'active' AND c.is_active = 1) OR (? = 'inactive' AND c.is_active = 0))
              AND (
                  ? = ''
                  OR (? = 'active' AND COALESCE(svc.active_subscriptions, 0) > 0)
                  OR (? = 'inactive' AND COALESCE(svc.subscription_count, 0) > 0 AND COALESCE(svc.active_subscriptions, 0) = 0)
                  OR (? = 'none' AND COALESCE(svc.subscription_count, 0) = 0)
              )
              AND (? = '' OR (? = 'pending' AND COALESCE(svc.pending_installations, 0) > 0))
            ORDER BY c.created_at DESC
            LIMIT ? OFFSET ?
        "#;

        #[derive(sqlx::FromRow)]
        struct Row {
            #[sqlx(flatten)]
            customer: CustomerListItem,
            total_count: i64,
        }

        #[cfg(feature = "postgres")]
        let rows: Vec<Row> = sqlx::query_as(query)
            .bind(tenant_id)
            .bind(&q)
            .bind(&status)
            .bind(&service)
            .bind(&installation)
            .bind(per_page as i64)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<Row> = sqlx::query_as(query)
            .bind(tenant_id)
            .bind(tenant_id)
            .bind(&q)
            .bind(&q)
            .bind(&q)
            .bind(&status)
            .bind(&status)
            .bind(&status)
            .bind(&service)
            .bind(&service)
            .bind(&service)
            .bind(&service)
            .bind(&installation)
            .bind(&installation)
            .bind(per_page as i64)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let total = rows.first().map(|r| r.total_count).unwrap_or(0);
        Ok(PaginatedResponse {
            data: rows.into_iter().map(|r| r.customer).collect(),
            total,
            page,
            per_page,
        })
    }

    pub async fn get_customer_summary(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<CustomerSummary> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read")
            .await?;

        #[cfg(feature = "postgres")]
        let row = sqlx::query_as::<_, CustomerSummary>(
            r#"
            SELECT
                COUNT(DISTINCT c.id)::bigint AS total,
                COUNT(DISTINCT CASE WHEN c.is_active THEN c.id END)::bigint AS active,
                COUNT(DISTINCT CASE WHEN NOT c.is_active THEN c.id END)::bigint AS inactive,
                COUNT(DISTINCT CASE WHEN cs.status = 'pending_installation' THEN c.id END)::bigint AS pending_installation
            FROM customers c
            LEFT JOIN customer_subscriptions cs
              ON cs.tenant_id = c.tenant_id AND cs.customer_id = c.id
            WHERE c.tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row = sqlx::query_as::<_, CustomerSummary>(
            r#"
            SELECT
                COUNT(*) AS total,
                COALESCE(SUM(CASE WHEN is_active THEN 1 ELSE 0 END), 0) AS active,
                COALESCE(SUM(CASE WHEN is_active THEN 0 ELSE 1 END), 0) AS inactive,
                (
                    SELECT COUNT(DISTINCT cc.id)
                    FROM customers cc
                    JOIN customer_subscriptions cs
                      ON cs.tenant_id = cc.tenant_id AND cs.customer_id = cc.id
                    WHERE cc.tenant_id = ? AND cs.status = 'pending_installation'
                ) AS pending_installation
            FROM customers
            WHERE tenant_id = ?
            "#,
        )
        .bind(tenant_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn get_service_lifecycle_report(
        &self,
        actor_id: &str,
        tenant_id: &str,
        q: Option<String>,
        issue_type: Option<String>,
        page: u32,
        per_page: u32,
    ) -> AppResult<CustomerServiceLifecycleReport> {
        if self
            .auth_service
            .check_permission(actor_id, tenant_id, "customers", "read")
            .await
            .is_err()
        {
            self.auth_service
                .check_permission(actor_id, tenant_id, "orders", "create")
                .await?;
        }

        let pg = crate::services::pagination::normalize(page, per_page);
        let per_page = pg.per_page;
        let offset = pg.offset;
        let q = q.unwrap_or_default().trim().to_string();
        let issue_type = issue_type
            .unwrap_or_else(|| "all".to_string())
            .trim()
            .to_ascii_lowercase();

        #[derive(sqlx::FromRow)]
        struct IssueRow {
            #[sqlx(flatten)]
            issue: CustomerServiceLifecycleIssue,
            total_count: i64,
        }

        #[cfg(feature = "postgres")]
        let rows: Vec<IssueRow> = sqlx::query_as(
            r#"
            WITH lifecycle_issues AS (
              SELECT
                'missing_bootstrap_invoice' AS issue_type,
                cs.customer_id,
                c.name AS customer_name,
                cs.id AS subscription_id,
                cs.status AS subscription_status,
                p.name AS package_name,
                cl.label AS location_label,
                cs.starts_at,
                cs.ends_at,
                'bootstrap_invoice' AS recommended_action,
                COALESCE(cs.ends_at, cs.updated_at) AS issue_sort_at
              FROM customer_subscriptions cs
              INNER JOIN customers c
                ON c.tenant_id = cs.tenant_id AND c.id = cs.customer_id
              LEFT JOIN isp_packages p
                ON p.tenant_id = cs.tenant_id AND p.id = cs.package_id
              LEFT JOIN customer_locations cl
                ON cl.tenant_id = cs.tenant_id AND cl.id = cs.location_id
              WHERE cs.tenant_id = $1
                AND LOWER(cs.status) IN ('active', 'grace_active')
                AND LOWER(COALESCE(cs.billing_cycle, '')) IN ('monthly', 'yearly')
                AND (cs.starts_at IS NULL OR cs.starts_at <= NOW())
                AND (cs.ends_at IS NULL OR cs.ends_at >= NOW())
                AND (
                  cs.starts_at IS NULL
                  OR cs.ends_at IS NULL
                  OR cs.starts_at <= cs.ends_at
                )
                AND NOT EXISTS (
                  SELECT 1
                  FROM invoices i
                  WHERE i.tenant_id = cs.tenant_id
                    AND (
                      i.external_id = 'pkgsub:' || cs.id
                      OR i.external_id LIKE 'pkgsub:' || cs.id || ':%'
                    )
                )
              UNION ALL
              SELECT
                'invalid_active_lifecycle' AS issue_type,
                cs.customer_id,
                c.name AS customer_name,
                cs.id AS subscription_id,
                cs.status AS subscription_status,
                p.name AS package_name,
                cl.label AS location_label,
                cs.starts_at,
                cs.ends_at,
                'review_lifecycle_data' AS recommended_action,
                COALESCE(cs.ends_at, cs.updated_at) AS issue_sort_at
              FROM customer_subscriptions cs
              INNER JOIN customers c
                ON c.tenant_id = cs.tenant_id AND c.id = cs.customer_id
              LEFT JOIN isp_packages p
                ON p.tenant_id = cs.tenant_id AND p.id = cs.package_id
              LEFT JOIN customer_locations cl
                ON cl.tenant_id = cs.tenant_id AND cl.id = cs.location_id
              WHERE cs.tenant_id = $1
                AND LOWER(cs.status) IN ('active', 'grace_active')
                AND (
                  LOWER(COALESCE(cs.billing_cycle, '')) NOT IN ('monthly', 'yearly')
                  OR (cs.starts_at IS NOT NULL AND cs.starts_at > NOW())
                  OR (cs.ends_at IS NOT NULL AND cs.ends_at < NOW())
                  OR (
                    cs.starts_at IS NOT NULL
                    AND cs.ends_at IS NOT NULL
                    AND cs.starts_at > cs.ends_at
                  )
                )
            )
            SELECT
              issue_type,
              customer_id,
              customer_name,
              subscription_id,
              subscription_status,
              package_name,
              location_label,
              starts_at,
              ends_at,
              recommended_action,
              COUNT(*) OVER() AS total_count
            FROM lifecycle_issues
            WHERE ($2 = 'all' OR issue_type = $2)
              AND (
                $3 = ''
                OR customer_name ILIKE '%' || $3 || '%'
                OR COALESCE(package_name, '') ILIKE '%' || $3 || '%'
                OR COALESCE(location_label, '') ILIKE '%' || $3 || '%'
                OR subscription_id ILIKE '%' || $3 || '%'
              )
            ORDER BY
              CASE WHEN issue_type = 'invalid_active_lifecycle' THEN 0 ELSE 1 END,
              issue_sort_at ASC,
              subscription_id DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(tenant_id)
        .bind(&issue_type)
        .bind(&q)
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<IssueRow> = sqlx::query_as(
            r#"
            WITH lifecycle_issues AS (
              SELECT
                'missing_bootstrap_invoice' AS issue_type,
                cs.customer_id,
                c.name AS customer_name,
                cs.id AS subscription_id,
                cs.status AS subscription_status,
                p.name AS package_name,
                cl.label AS location_label,
                cs.starts_at,
                cs.ends_at,
                'bootstrap_invoice' AS recommended_action,
                COALESCE(cs.ends_at, cs.updated_at) AS issue_sort_at
              FROM customer_subscriptions cs
              INNER JOIN customers c
                ON c.tenant_id = cs.tenant_id AND c.id = cs.customer_id
              LEFT JOIN isp_packages p
                ON p.tenant_id = cs.tenant_id AND p.id = cs.package_id
              LEFT JOIN customer_locations cl
                ON cl.tenant_id = cs.tenant_id AND cl.id = cs.location_id
              WHERE cs.tenant_id = ?
                AND LOWER(cs.status) IN ('active', 'grace_active')
                AND LOWER(COALESCE(cs.billing_cycle, '')) IN ('monthly', 'yearly')
                AND (cs.starts_at IS NULL OR cs.starts_at <= ?)
                AND (cs.ends_at IS NULL OR cs.ends_at >= ?)
                AND (
                  cs.starts_at IS NULL
                  OR cs.ends_at IS NULL
                  OR cs.starts_at <= cs.ends_at
                )
                AND NOT EXISTS (
                  SELECT 1
                  FROM invoices i
                  WHERE i.tenant_id = cs.tenant_id
                    AND (
                      i.external_id = 'pkgsub:' || cs.id
                      OR i.external_id LIKE 'pkgsub:' || cs.id || ':%'
                    )
                )
              UNION ALL
              SELECT
                'invalid_active_lifecycle' AS issue_type,
                cs.customer_id,
                c.name AS customer_name,
                cs.id AS subscription_id,
                cs.status AS subscription_status,
                p.name AS package_name,
                cl.label AS location_label,
                cs.starts_at,
                cs.ends_at,
                'review_lifecycle_data' AS recommended_action,
                COALESCE(cs.ends_at, cs.updated_at) AS issue_sort_at
              FROM customer_subscriptions cs
              INNER JOIN customers c
                ON c.tenant_id = cs.tenant_id AND c.id = cs.customer_id
              LEFT JOIN isp_packages p
                ON p.tenant_id = cs.tenant_id AND p.id = cs.package_id
              LEFT JOIN customer_locations cl
                ON cl.tenant_id = cs.tenant_id AND cl.id = cs.location_id
              WHERE cs.tenant_id = ?
                AND LOWER(cs.status) IN ('active', 'grace_active')
                AND (
                  LOWER(COALESCE(cs.billing_cycle, '')) NOT IN ('monthly', 'yearly')
                  OR (cs.starts_at IS NOT NULL AND cs.starts_at > ?)
                  OR (cs.ends_at IS NOT NULL AND cs.ends_at < ?)
                  OR (
                    cs.starts_at IS NOT NULL
                    AND cs.ends_at IS NOT NULL
                    AND cs.starts_at > cs.ends_at
                  )
                )
            )
            SELECT
              issue_type,
              customer_id,
              customer_name,
              subscription_id,
              subscription_status,
              package_name,
              location_label,
              starts_at,
              ends_at,
              recommended_action,
              COUNT(*) OVER() AS total_count
            FROM lifecycle_issues
            WHERE (? = 'all' OR issue_type = ?)
              AND (
                ? = ''
                OR customer_name LIKE '%' || ? || '%'
                OR COALESCE(package_name, '') LIKE '%' || ? || '%'
                OR COALESCE(location_label, '') LIKE '%' || ? || '%'
                OR subscription_id LIKE '%' || ? || '%'
              )
            ORDER BY
              CASE WHEN issue_type = 'invalid_active_lifecycle' THEN 0 ELSE 1 END,
              issue_sort_at ASC,
              subscription_id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(tenant_id)
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .bind(tenant_id)
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .bind(&issue_type)
        .bind(&issue_type)
        .bind(&q)
        .bind(&q)
        .bind(&q)
        .bind(&q)
        .bind(&q)
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let missing_bootstrap_invoice: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = $1
              AND LOWER(cs.status) IN ('active', 'grace_active')
              AND LOWER(COALESCE(cs.billing_cycle, '')) IN ('monthly', 'yearly')
              AND (cs.starts_at IS NULL OR cs.starts_at <= NOW())
              AND (cs.ends_at IS NULL OR cs.ends_at >= NOW())
              AND (
                cs.starts_at IS NULL
                OR cs.ends_at IS NULL
                OR cs.starts_at <= cs.ends_at
              )
              AND NOT EXISTS (
                SELECT 1
                FROM invoices i
                WHERE i.tenant_id = cs.tenant_id
                  AND (
                    i.external_id = 'pkgsub:' || cs.id
                    OR i.external_id LIKE 'pkgsub:' || cs.id || ':%'
                  )
              )
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let missing_bootstrap_invoice: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = ?
              AND LOWER(cs.status) IN ('active', 'grace_active')
              AND LOWER(COALESCE(cs.billing_cycle, '')) IN ('monthly', 'yearly')
              AND (cs.starts_at IS NULL OR cs.starts_at <= ?)
              AND (cs.ends_at IS NULL OR cs.ends_at >= ?)
              AND (
                cs.starts_at IS NULL
                OR cs.ends_at IS NULL
                OR cs.starts_at <= cs.ends_at
              )
              AND NOT EXISTS (
                SELECT 1
                FROM invoices i
                WHERE i.tenant_id = cs.tenant_id
                  AND (
                    i.external_id = 'pkgsub:' || cs.id
                    OR i.external_id LIKE 'pkgsub:' || cs.id || ':%'
                  )
              )
            "#,
        )
        .bind(tenant_id)
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let invalid_active_lifecycle: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = $1
              AND LOWER(cs.status) IN ('active', 'grace_active')
              AND (
                LOWER(COALESCE(cs.billing_cycle, '')) NOT IN ('monthly', 'yearly')
                OR (cs.starts_at IS NOT NULL AND cs.starts_at > NOW())
                OR (cs.ends_at IS NOT NULL AND cs.ends_at < NOW())
                OR (
                  cs.starts_at IS NOT NULL
                  AND cs.ends_at IS NOT NULL
                  AND cs.starts_at > cs.ends_at
                )
              )
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let invalid_active_lifecycle: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = ?
              AND LOWER(cs.status) IN ('active', 'grace_active')
              AND (
                LOWER(COALESCE(cs.billing_cycle, '')) NOT IN ('monthly', 'yearly')
                OR (cs.starts_at IS NOT NULL AND cs.starts_at > ?)
                OR (cs.ends_at IS NOT NULL AND cs.ends_at < ?)
                OR (
                  cs.starts_at IS NOT NULL
                  AND cs.ends_at IS NOT NULL
                  AND cs.starts_at > cs.ends_at
                )
              )
            "#,
        )
        .bind(tenant_id)
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .fetch_one(&self.pool)
        .await?;

        let total_issues = rows
            .first()
            .map(|row| row.total_count)
            .unwrap_or(missing_bootstrap_invoice + invalid_active_lifecycle);

        Ok(CustomerServiceLifecycleReport {
            generated_at: Utc::now(),
            total_issues,
            missing_bootstrap_invoice,
            invalid_active_lifecycle,
            page,
            per_page,
            data: rows.into_iter().map(|row| row.issue).collect(),
        })
    }

    pub async fn repair_service_lifecycle_issues(
        &self,
        actor_id: &str,
        tenant_id: &str,
        request: RepairCustomerServiceLifecycleRequest,
    ) -> AppResult<CustomerServiceLifecycleRepairResult> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "billing", "manage")
            .await?;

        let issue_type =
            Self::normalize_service_lifecycle_issue_type(Some(request.issue_type.as_str()))?;

        match issue_type {
            "missing_bootstrap_invoice" => {
                #[derive(sqlx::FromRow)]
                struct RepairRow {
                    subscription_id: String,
                    starts_at: Option<DateTime<Utc>>,
                    ends_at: Option<DateTime<Utc>>,
                }

                #[cfg(feature = "postgres")]
                let rows: Vec<RepairRow> = sqlx::query_as(
                    r#"
                    SELECT cs.id AS subscription_id, cs.starts_at, cs.ends_at
                    FROM customer_subscriptions cs
                    WHERE cs.tenant_id = $1
                      AND LOWER(cs.status) IN ('active', 'grace_active')
                      AND LOWER(COALESCE(cs.billing_cycle, '')) IN ('monthly', 'yearly')
                      AND (cs.starts_at IS NULL OR cs.starts_at <= NOW())
                      AND (cs.ends_at IS NULL OR cs.ends_at >= NOW())
                      AND (
                        cs.starts_at IS NULL
                        OR cs.ends_at IS NULL
                        OR cs.starts_at <= cs.ends_at
                      )
                      AND NOT EXISTS (
                        SELECT 1
                        FROM invoices i
                        WHERE i.tenant_id = cs.tenant_id
                          AND (
                            i.external_id = 'pkgsub:' || cs.id
                            OR i.external_id LIKE 'pkgsub:' || cs.id || ':%'
                          )
                      )
                    ORDER BY COALESCE(cs.ends_at, cs.updated_at) ASC, cs.updated_at DESC, cs.id DESC
                    "#,
                )
                .bind(tenant_id)
                .fetch_all(&self.pool)
                .await?;

                #[cfg(feature = "sqlite")]
                let rows: Vec<RepairRow> = sqlx::query_as(
                    r#"
                    SELECT cs.id AS subscription_id, cs.starts_at, cs.ends_at
                    FROM customer_subscriptions cs
                    WHERE cs.tenant_id = ?
                      AND LOWER(cs.status) IN ('active', 'grace_active')
                      AND LOWER(COALESCE(cs.billing_cycle, '')) IN ('monthly', 'yearly')
                      AND (cs.starts_at IS NULL OR cs.starts_at <= ?)
                      AND (cs.ends_at IS NULL OR cs.ends_at >= ?)
                      AND (
                        cs.starts_at IS NULL
                        OR cs.ends_at IS NULL
                        OR cs.starts_at <= cs.ends_at
                      )
                      AND NOT EXISTS (
                        SELECT 1
                        FROM invoices i
                        WHERE i.tenant_id = cs.tenant_id
                          AND (
                            i.external_id = 'pkgsub:' || cs.id
                            OR i.external_id LIKE 'pkgsub:' || cs.id || ':%'
                          )
                      )
                    ORDER BY COALESCE(cs.ends_at, cs.updated_at) ASC, cs.updated_at DESC, cs.id DESC
                    "#,
                )
                .bind(tenant_id)
                .bind(Utc::now().to_rfc3339())
                .bind(Utc::now().to_rfc3339())
                .fetch_all(&self.pool)
                .await?;

                let matched_count = rows.len() as i64;
                let payment_service = PaymentService::new(
                    self.pool.clone(),
                    self.notification_service.clone(),
                    self.pppoe_service.clone(),
                    self.audit_service.clone(),
                );
                let mut repaired_count = 0_i64;
                let mut skipped_count = 0_i64;
                let mut failed_count = 0_i64;
                let mut errors = Vec::new();

                for row in rows {
                    let period_ref = row.starts_at.or(row.ends_at).unwrap_or_else(Utc::now);
                    match payment_service
                        .create_bootstrap_invoice_for_customer_subscription(
                            tenant_id,
                            &row.subscription_id,
                            period_ref,
                            row.ends_at,
                        )
                        .await
                    {
                        Ok(_) => repaired_count += 1,
                        Err(AppError::Validation(err)) => {
                            skipped_count += 1;
                            errors.push(format!("{}: {}", row.subscription_id, err));
                        }
                        Err(err) => {
                            failed_count += 1;
                            errors.push(format!("{}: {}", row.subscription_id, err));
                        }
                    }
                }

                let details = format!(
                    "Lifecycle repair summary: issue_type={}, matched={}, repaired={}, skipped={}, failed={}",
                    issue_type, matched_count, repaired_count, skipped_count, failed_count
                );
                self.audit_service
                    .log(
                        Some(actor_id),
                        Some(tenant_id),
                        "CUSTOMER_SERVICE_LIFECYCLE_REPAIR",
                        "customer_service_lifecycle",
                        None,
                        Some(&details),
                        None,
                    )
                    .await;

                Ok(CustomerServiceLifecycleRepairResult {
                    issue_type: issue_type.to_string(),
                    matched_count,
                    repaired_count,
                    skipped_count,
                    failed_count,
                    errors,
                })
            }
            "invalid_active_lifecycle" => {
                #[derive(sqlx::FromRow)]
                struct InvalidLifecycleRow {
                    subscription_id: String,
                }

                #[cfg(feature = "postgres")]
                let rows: Vec<InvalidLifecycleRow> = sqlx::query_as(
                    r#"
                    SELECT cs.id AS subscription_id
                    FROM customer_subscriptions cs
                    WHERE cs.tenant_id = $1
                      AND LOWER(cs.status) IN ('active', 'grace_active')
                      AND (
                        LOWER(COALESCE(cs.billing_cycle, '')) NOT IN ('monthly', 'yearly')
                        OR (cs.starts_at IS NOT NULL AND cs.starts_at > NOW())
                        OR (cs.ends_at IS NOT NULL AND cs.ends_at < NOW())
                        OR (
                          cs.starts_at IS NOT NULL
                          AND cs.ends_at IS NOT NULL
                          AND cs.starts_at > cs.ends_at
                        )
                      )
                    ORDER BY COALESCE(cs.ends_at, cs.updated_at) ASC, cs.updated_at DESC, cs.id DESC
                    "#,
                )
                .bind(tenant_id)
                .fetch_all(&self.pool)
                .await?;

                #[cfg(feature = "sqlite")]
                let rows: Vec<InvalidLifecycleRow> = sqlx::query_as(
                    r#"
                    SELECT cs.id AS subscription_id
                    FROM customer_subscriptions cs
                    WHERE cs.tenant_id = ?
                      AND LOWER(cs.status) IN ('active', 'grace_active')
                      AND (
                        LOWER(COALESCE(cs.billing_cycle, '')) NOT IN ('monthly', 'yearly')
                        OR (cs.starts_at IS NOT NULL AND cs.starts_at > ?)
                        OR (cs.ends_at IS NOT NULL AND cs.ends_at < ?)
                        OR (
                          cs.starts_at IS NOT NULL
                          AND cs.ends_at IS NOT NULL
                          AND cs.starts_at > cs.ends_at
                        )
                      )
                    ORDER BY COALESCE(cs.ends_at, cs.updated_at) ASC, cs.updated_at DESC, cs.id DESC
                    "#,
                )
                .bind(tenant_id)
                .bind(Utc::now().to_rfc3339())
                .bind(Utc::now().to_rfc3339())
                .fetch_all(&self.pool)
                .await?;

                let matched_count = rows.len() as i64;
                let mut repaired_count = 0_i64;
                let mut skipped_count = 0_i64;
                let mut failed_count = 0_i64;
                let mut errors = Vec::new();

                for row in rows {
                    match self
                        .normalize_invalid_active_lifecycle_subscription(
                            actor_id,
                            tenant_id,
                            &row.subscription_id,
                        )
                        .await
                    {
                        Ok(true) => repaired_count += 1,
                        Ok(false) => skipped_count += 1,
                        Err(AppError::Validation(err)) => {
                            skipped_count += 1;
                            errors.push(format!("{}: {}", row.subscription_id, err));
                        }
                        Err(err) => {
                            failed_count += 1;
                            errors.push(format!("{}: {}", row.subscription_id, err));
                        }
                    }
                }

                let details = format!(
                    "Lifecycle repair summary: issue_type={}, matched={}, repaired={}, skipped={}, failed={}",
                    issue_type, matched_count, repaired_count, skipped_count, failed_count
                );
                self.audit_service
                    .log(
                        Some(actor_id),
                        Some(tenant_id),
                        "CUSTOMER_SERVICE_LIFECYCLE_REPAIR",
                        "customer_service_lifecycle",
                        None,
                        Some(&details),
                        None,
                    )
                    .await;

                Ok(CustomerServiceLifecycleRepairResult {
                    issue_type: issue_type.to_string(),
                    matched_count,
                    repaired_count,
                    skipped_count,
                    failed_count,
                    errors,
                })
            }
            _ => Err(AppError::Validation(
                "unsupported service lifecycle issue type".to_string(),
            )),
        }
    }

    async fn normalize_invalid_active_lifecycle_subscription(
        &self,
        actor_id: &str,
        tenant_id: &str,
        subscription_id: &str,
    ) -> AppResult<bool> {
        let updated = self
            .update_customer_subscription(
                actor_id,
                tenant_id,
                subscription_id,
                crate::models::UpdateCustomerSubscriptionRequest {
                    location_id: None,
                    package_id: None,
                    router_id: None,
                    billing_cycle: None,
                    price: None,
                    currency_code: None,
                    status: Some("suspended".to_string()),
                    starts_at: None,
                    ends_at: None,
                    notes: Some("Auto-normalized from lifecycle reconciliation".to_string()),
                },
                Some("127.0.0.1"),
            )
            .await?;

        Ok(updated.status == "suspended")
    }

    pub async fn get_customer(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
    ) -> AppResult<Customer> {
        if self
            .auth_service
            .check_permission(actor_id, tenant_id, "customers", "read")
            .await
            .is_err()
        {
            self.auth_service
                .check_permission(actor_id, tenant_id, "orders", "create")
                .await?;
        }

        #[cfg(feature = "postgres")]
        let customer: Option<Customer> =
            sqlx::query_as("SELECT * FROM customers WHERE tenant_id = $1 AND id = $2")
                .bind(tenant_id)
                .bind(customer_id)
                .fetch_optional(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let customer: Option<Customer> =
            sqlx::query_as("SELECT * FROM customers WHERE tenant_id = ? AND id = ?")
                .bind(tenant_id)
                .bind(customer_id)
                .fetch_optional(&self.pool)
                .await?;

        customer.ok_or_else(|| AppError::NotFound("Customer not found".to_string()))
    }

    pub async fn next_customer_number(&self, tenant_id: &str) -> AppResult<String> {
        let now = chrono::Utc::now();
        let prefix = format!("{}-", now.format("%y%m")); // "2607-"
        let pattern = format!("{}%", prefix);

        #[cfg(feature = "postgres")]
        let max_num: Option<String> = sqlx::query_scalar(
            "SELECT customer_number FROM customers WHERE tenant_id = $1 AND customer_number LIKE $2 ORDER BY customer_number DESC LIMIT 1"
        )
        .bind(tenant_id)
        .bind(&pattern)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let max_num: Option<String> = sqlx::query_scalar(
            "SELECT customer_number FROM customers WHERE tenant_id = ? AND customer_number LIKE ? ORDER BY customer_number DESC LIMIT 1"
        )
        .bind(tenant_id)
        .bind(&pattern)
        .fetch_optional(&self.pool)
        .await?;

        let next = match max_num {
            Some(last) => {
                let parts: Vec<&str> = last.splitn(2, '-').collect();
                let n: i64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                n + 1
            }
            None => 1,
        };
        Ok(format!("{}{:05}", prefix, next))
    }

    pub async fn create_customer(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateCustomerRequest,
        ip_address: Option<&str>,
    ) -> AppResult<Customer> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let customer_number = self.next_customer_number(tenant_id).await?;
        let customer = Customer::new(
            tenant_id.to_string(),
            dto.name,
            dto.email,
            dto.phone,
            dto.notes,
            dto.is_active,
            Some(customer_number),
        );

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customers
                (id, tenant_id, name, email, phone, notes, is_active, customer_number, created_at, updated_at)
            VALUES
                ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(&customer.id)
        .bind(&customer.tenant_id)
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.phone)
        .bind(&customer.notes)
        .bind(customer.is_active)
        .bind(&customer.customer_number)
        .bind(customer.created_at)
        .bind(customer.updated_at)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO customers
                (id, tenant_id, name, email, phone, notes, is_active, customer_number, created_at, updated_at)
            VALUES
                (?,?,?,?,?,?,?,?,?,?)
            "#,
        )
        .bind(&customer.id)
        .bind(&customer.tenant_id)
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.phone)
        .bind(&customer.notes)
        .bind(customer.is_active)
        .bind(&customer.customer_number)
        .bind(customer.created_at.to_rfc3339())
        .bind(customer.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_CREATE",
                "customers",
                Some(&customer.id),
                Some(&format!("Created customer {}", customer.name)),
                ip_address,
            )
            .await;

        Ok(customer)
    }

    pub async fn create_customer_with_portal(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateCustomerWithPortalRequest,
        ip_address: Option<&str>,
    ) -> AppResult<Customer> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let portal_email = dto.portal_email.trim().to_lowercase();
        if portal_email.is_empty() {
            return Err(AppError::Validation("portal_email is required".to_string()));
        }
        if dto.portal_password.trim().len() < 6 {
            return Err(AppError::Validation(
                "portal_password must be at least 6 characters".to_string(),
            ));
        }

        let customer_number = self.next_customer_number(tenant_id).await?;
        let customer = Customer::new(
            tenant_id.to_string(),
            dto.name,
            dto.email,
            dto.phone,
            dto.notes,
            dto.is_active,
            Some(customer_number),
        );

        let portal_user_name = dto
            .portal_name
            .unwrap_or_else(|| customer.name.clone())
            .trim()
            .to_string();
        if portal_user_name.is_empty() {
            return Err(AppError::Validation("portal_name is required".to_string()));
        }

        let user_id = Uuid::new_v4().to_string();
        let customer_user_id = Uuid::new_v4().to_string();
        let tenant_member_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let role_id = self.get_system_role_id_by_name("Customer").await?;
        let password_hash = AuthService::hash_password(&dto.portal_password)?;

        let mut tx = self.pool.begin().await?;
        self.auth_service
            .apply_rls_context_tx_values(&mut tx, Some(tenant_id), Some(actor_id), false)
            .await?;

        #[cfg(feature = "postgres")]
        {
            let existing: Option<String> =
                sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
                    .bind(&portal_email)
                    .fetch_optional(&mut *tx)
                    .await?;
            if existing.is_some() {
                return Err(AppError::UserAlreadyExists);
            }

            sqlx::query(
                r#"
                INSERT INTO customers
                    (id, tenant_id, name, email, phone, notes, is_active, customer_number, created_at, updated_at)
                VALUES
                    ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                "#,
            )
            .bind(&customer.id)
            .bind(&customer.tenant_id)
            .bind(&customer.name)
            .bind(&customer.email)
            .bind(&customer.phone)
            .bind(&customer.notes)
            .bind(customer.is_active)
            .bind(&customer.customer_number)
            .bind(customer.created_at)
            .bind(customer.updated_at)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO users (id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(&user_id)
            .bind(&portal_email)
            .bind(&password_hash)
            .bind(&portal_user_name)
            .bind("user")
            .bind(false)
            .bind(true)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO customer_users (id, tenant_id, customer_id, user_id, created_at) VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(&customer_user_id)
            .bind(tenant_id)
            .bind(&customer.id)
            .bind(&user_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&tenant_member_id)
            .bind(tenant_id)
            .bind(&user_id)
            .bind("customer")
            .bind(&role_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }

        #[cfg(feature = "sqlite")]
        {
            let existing: Option<String> =
                sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
                    .bind(&portal_email)
                    .fetch_optional(&mut *tx)
                    .await?;
            if existing.is_some() {
                return Err(AppError::UserAlreadyExists);
            }

            sqlx::query(
                r#"
                INSERT INTO customers
                    (id, tenant_id, name, email, phone, notes, is_active, customer_number, created_at, updated_at)
                VALUES
                    (?,?,?,?,?,?,?,?,?,?)
                "#,
            )
            .bind(&customer.id)
            .bind(&customer.tenant_id)
            .bind(&customer.name)
            .bind(&customer.email)
            .bind(&customer.phone)
            .bind(&customer.notes)
            .bind(customer.is_active)
            .bind(&customer.customer_number)
            .bind(customer.created_at.to_rfc3339())
            .bind(customer.updated_at.to_rfc3339())
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO users (id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&user_id)
            .bind(&portal_email)
            .bind(&password_hash)
            .bind(&portal_user_name)
            .bind("user")
            .bind(false)
            .bind(true)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO customer_users (id, tenant_id, customer_id, user_id, created_at) VALUES (?,?,?,?,?)",
            )
            .bind(&customer_user_id)
            .bind(tenant_id)
            .bind(&customer.id)
            .bind(&user_id)
            .bind(now.to_rfc3339())
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(&tenant_member_id)
            .bind(tenant_id)
            .bind(&user_id)
            .bind("customer")
            .bind(&role_id)
            .bind(now.to_rfc3339())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_CREATE",
                "customers",
                Some(&customer.id),
                Some(&format!("Created customer {}", customer.name)),
                ip_address,
            )
            .await;
        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_USER_CREATE",
                "customer_users",
                Some(&customer_user_id),
                Some("Created portal login while creating customer"),
                ip_address,
            )
            .await;

        Ok(customer)
    }

    pub async fn create_customer_from_public_registration(
        &self,
        tenant_id: &str,
        user_id: &str,
        customer_name: &str,
        customer_email: &str,
        phone: Option<&str>,
        ip_address: Option<&str>,
        registration_invite_id: Option<&str>,
    ) -> AppResult<Customer> {
        let name = customer_name.trim().to_string();
        if name.len() < 2 {
            return Err(AppError::Validation(
                "Customer name must be at least 2 characters".to_string(),
            ));
        }
        let email = customer_email.trim().to_lowercase();
        if email.is_empty() {
            return Err(AppError::Validation(
                "Customer email is required".to_string(),
            ));
        }

        let customer_number = self.next_customer_number(tenant_id).await?;

        #[cfg(feature = "postgres")]
        let existing_customer: Option<Customer> = sqlx::query_as(
            r#"
            SELECT c.*
            FROM customers c
            JOIN customer_users cu ON cu.customer_id = c.id AND cu.tenant_id = c.tenant_id
            WHERE cu.tenant_id = $1 AND cu.user_id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let existing_customer: Option<Customer> = sqlx::query_as(
            r#"
            SELECT c.*
            FROM customers c
            JOIN customer_users cu ON cu.customer_id = c.id AND cu.tenant_id = c.tenant_id
            WHERE cu.tenant_id = ? AND cu.user_id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(c) = existing_customer {
            return Ok(c);
        }

        let customer = Customer::new(
            tenant_id.to_string(),
            name,
            Some(email),
            phone
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty()),
            None,
            // Customers created via public registration start inactive. They become
            // active once the corresponding users.registration_status flips to
            // 'active' (approved by admin/owner). Until then, the customer record
            // exists for foreknowledge but no billing actions should run.
            //
            // Exception: if registration arrived with a valid invite token issued
            // by an authorized user (tenant admin/owner), the customer is active
            // immediately — invite path is trusted.
            Some(registration_invite_id.is_some()),
            Some(customer_number),
        );

        let customer_user_id = Uuid::new_v4().to_string();
        let tenant_member_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let role_id = self.get_system_role_id_by_name("Customer").await?;

        let mut tx = self.pool.begin().await?;
        self.auth_service
            .apply_rls_context_tx_values(&mut tx, Some(tenant_id), Some(user_id), false)
            .await?;

        #[cfg(feature = "postgres")]
        {
            sqlx::query(
                r#"
                INSERT INTO customers
                    (id, tenant_id, name, email, phone, notes, is_active, registration_invite_id, customer_number, created_at, updated_at)
                VALUES
                    ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
                "#,
            )
            .bind(&customer.id)
            .bind(&customer.tenant_id)
            .bind(&customer.name)
            .bind(&customer.email)
            .bind(&customer.phone)
            .bind(&customer.notes)
            .bind(customer.is_active)
            .bind(registration_invite_id)
            .bind(&customer.customer_number)
            .bind(customer.created_at)
            .bind(customer.updated_at)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO customer_users (id, tenant_id, customer_id, user_id, created_at) VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(&customer_user_id)
            .bind(tenant_id)
            .bind(&customer.id)
            .bind(user_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            let member_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM tenant_members WHERE tenant_id = $1 AND user_id = $2)",
            )
            .bind(tenant_id)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

            if !member_exists {
                sqlx::query(
                    "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(&tenant_member_id)
                .bind(tenant_id)
                .bind(user_id)
                .bind("customer")
                .bind(&role_id)
                .bind(now)
                .execute(&mut *tx)
                .await?;
            }
        }

        #[cfg(feature = "sqlite")]
        {
            sqlx::query(
                r#"
                INSERT INTO customers
                    (id, tenant_id, name, email, phone, notes, is_active, registration_invite_id, customer_number, created_at, updated_at)
                VALUES
                    (?,?,?,?,?,?,?,?,?,?,?)
                "#,
            )
            .bind(&customer.id)
            .bind(&customer.tenant_id)
            .bind(&customer.name)
            .bind(&customer.email)
            .bind(&customer.phone)
            .bind(&customer.notes)
            .bind(customer.is_active)
            .bind(registration_invite_id)
            .bind(&customer.customer_number)
            .bind(customer.created_at.to_rfc3339())
            .bind(customer.updated_at.to_rfc3339())
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO customer_users (id, tenant_id, customer_id, user_id, created_at) VALUES (?,?,?,?,?)",
            )
            .bind(&customer_user_id)
            .bind(tenant_id)
            .bind(&customer.id)
            .bind(user_id)
            .bind(now.to_rfc3339())
            .execute(&mut *tx)
            .await?;

            let member_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM tenant_members WHERE tenant_id = ? AND user_id = ?)",
            )
            .bind(tenant_id)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

            if !member_exists {
                sqlx::query(
                    "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(&tenant_member_id)
                .bind(tenant_id)
                .bind(user_id)
                .bind("customer")
                .bind(&role_id)
                .bind(now.to_rfc3339())
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        self.audit_service
            .log(
                Some(user_id),
                Some(tenant_id),
                "CUSTOMER_SELF_REGISTER",
                "customers",
                Some(&customer.id),
                Some("Created customer via custom-domain public registration"),
                ip_address,
            )
            .await;
        self.audit_service
            .log(
                Some(user_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_USER_CREATE",
                "customer_users",
                Some(&customer_user_id),
                Some("Linked self-registered user as customer portal user"),
                ip_address,
            )
            .await;

        // Notify tenant Owner/Admin users (in-app) about the new
        // registration so they can review/approve from the dashboard.
        // Email fan-out (if any) is handled separately in
        // auth_service::register_with_email_verification_policy.
        let via_invite_signup = registration_invite_id.is_some();
        match self
            .fetch_tenant_admin_user_ids_for_notification(tenant_id)
            .await
        {
            Ok(admin_ids) => {
                for admin_user_id in admin_ids {
                    if admin_user_id == user_id {
                        continue; // don't notify the user about themselves
                    }
                    let (title, level) = if via_invite_signup {
                        (
                            format!("New customer (via invite): {}", customer.name),
                            "info".to_string(),
                        )
                    } else {
                        (
                            format!(
                                "New customer registered (pending approval): {}",
                                customer.name
                            ),
                            "warning".to_string(),
                        )
                    };
                    let message = format!(
                        "{name} ({email}) just registered on the customer portal.\nStatus: {status}.\nCustomer #: {custno}.",
                        name = customer.name,
                        email = customer.email.as_deref().unwrap_or("-"),
                        status = if via_invite_signup {
                            "active (invite)"
                        } else {
                            "pending approval"
                        },
                        custno = customer.customer_number.as_deref().unwrap_or("-"),
                    );
                    if let Err(e) = self
                        .notification_service
                        .create_notification(
                            admin_user_id.clone(),
                            Some(tenant_id.to_string()),
                            title,
                            message,
                            level,
                            "customers".to_string(),
                            Some("/admin/customers".to_string()),
                        )
                        .await
                    {
                        warn!(
                            "Failed to notify tenant admin {} about new customer {}: {}",
                            admin_user_id, customer.id, e
                        );
                    }
                }
            }
            Err(e) => warn!(
                "Failed to fetch tenant admins for new-customer notification: {}",
                e
            ),
        }

        Ok(customer)
    }

    pub async fn create_customer_registration_invite(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateCustomerRegistrationInviteRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerRegistrationInviteCreateResponse> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let policy = self.resolve_invite_policy_for_tenant(tenant_id).await?;
        let expires_in_hours = dto
            .expires_in_hours
            .unwrap_or(policy.default_expires_in_hours)
            .clamp(1, 24 * 30);
        let max_uses = dto
            .max_uses
            .unwrap_or(policy.default_max_uses)
            .clamp(1, 100);
        let note = dto.note.and_then(|v| {
            let trimmed = v.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.chars().take(500).collect::<String>())
            }
        });

        #[cfg(feature = "postgres")]
        let tenant_domain: Option<Option<String>> = sqlx::query_scalar(
            "SELECT custom_domain FROM tenants WHERE id = $1 AND is_active = true",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let tenant_domain: Option<Option<String>> =
            sqlx::query_scalar("SELECT custom_domain FROM tenants WHERE id = ? AND is_active = 1")
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await?;

        let tenant_domain = tenant_domain.flatten();

        let domain = tenant_domain
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                AppError::Validation(
                    "Tenant custom domain is required before generating customer invite link"
                        .to_string(),
                )
            })?;

        let now = Utc::now();
        let expires_at = now + chrono::Duration::hours(expires_in_hours as i64);
        let invite_token = Self::build_registration_invite_token();
        let token_hash = Self::hash_registration_invite_token(&invite_token);
        let token_enc = Self::encrypt_registration_invite_token(&invite_token)?;
        let invite_id = Uuid::new_v4().to_string();
        let invite_url = Self::build_registration_invite_url(&domain, &invite_token);

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customer_registration_invites
                (id, tenant_id, token_hash, token_enc, invite_url, created_by, max_uses, used_count, expires_at, is_revoked, revoked_at, last_used_at, note, created_at)
            VALUES
                ($1,$2,$3,$4,$5,$6,$7,0,$8,false,NULL,NULL,$9,$10)
            "#,
        )
        .bind(&invite_id)
        .bind(tenant_id)
        .bind(&token_hash)
        .bind(&token_enc)
        .bind(&invite_url)
        .bind(actor_id)
        .bind(max_uses as i64)
        .bind(expires_at)
        .bind(&note)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO customer_registration_invites
                (id, tenant_id, token_hash, token_enc, invite_url, created_by, max_uses, used_count, expires_at, is_revoked, revoked_at, last_used_at, note, created_at)
            VALUES
                (?,?,?,?,?,?,?,0,?,0,NULL,NULL,?,?)
            "#,
        )
        .bind(&invite_id)
        .bind(tenant_id)
        .bind(&token_hash)
        .bind(&token_enc)
        .bind(&invite_url)
        .bind(actor_id)
        .bind(max_uses as i64)
        .bind(expires_at.to_rfc3339())
        .bind(&note)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        let invite = CustomerRegistrationInviteView {
            id: invite_id.clone(),
            tenant_id: tenant_id.to_string(),
            created_by: Some(actor_id.to_string()),
            max_uses: max_uses as i64,
            used_count: 0,
            expires_at,
            is_revoked: false,
            revoked_at: None,
            last_used_at: None,
            note,
            created_at: now,
            invite_url: Some(invite_url.clone()),
        };

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_INVITE_CREATE",
                "customer_registration_invites",
                Some(&invite_id),
                Some(&format!(
                    "Generated customer registration invite (expires in {}h, max uses {})",
                    expires_in_hours, max_uses
                )),
                ip_address,
            )
            .await;

        Ok(CustomerRegistrationInviteCreateResponse {
            invite,
            invite_token,
            invite_url,
        })
    }

    pub async fn update_customer(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
        dto: UpdateCustomerRequest,
        ip_address: Option<&str>,
    ) -> AppResult<Customer> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let mut customer = self.get_customer(actor_id, tenant_id, customer_id).await?;
        if let Some(name) = dto.name {
            customer.name = name;
        }
        if let Some(ref email) = dto.email {
            let v = email.trim().to_string();
            customer.email = if v.is_empty() { None } else { Some(v) };
        }
        if let Some(phone) = dto.phone {
            let v = phone.trim().to_string();
            customer.phone = if v.is_empty() { None } else { Some(v) };
        }
        if let Some(notes) = dto.notes {
            let v = notes.trim().to_string();
            customer.notes = if v.is_empty() { None } else { Some(v) };
        }
        if let Some(is_active) = dto.is_active {
            customer.is_active = is_active;
        }
        customer.updated_at = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE customers
            SET name=$1, email=$2, phone=$3, notes=$4, is_active=$5, updated_at=$6
            WHERE tenant_id=$7 AND id=$8
            "#,
        )
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.phone)
        .bind(&customer.notes)
        .bind(customer.is_active)
        .bind(customer.updated_at)
        .bind(tenant_id)
        .bind(customer_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE customers
            SET name=?, email=?, phone=?, notes=?, is_active=?, updated_at=?
            WHERE tenant_id=? AND id=?
            "#,
        )
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.phone)
        .bind(&customer.notes)
        .bind(customer.is_active)
        .bind(customer.updated_at.to_rfc3339())
        .bind(tenant_id)
        .bind(customer_id)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_UPDATE",
                "customers",
                Some(customer_id),
                Some("Updated customer"),
                ip_address,
            )
            .await;

        // Sync customer email to linked users table
        if let Some(ref email) = dto.email {
            let new_email = email.trim();
            if !new_email.is_empty() {
                #[cfg(feature = "postgres")]
                sqlx::query(
                    r#"
                    UPDATE users SET email = $1, updated_at = NOW()
                    WHERE id IN (
                        SELECT user_id FROM customer_users
                        WHERE customer_id = $2 AND tenant_id = $3
                    )
                    "#,
                )
                .bind(new_email)
                .bind(customer_id)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;

                #[cfg(feature = "sqlite")]
                sqlx::query(
                    r#"
                    UPDATE users SET email = ?, updated_at = ?
                    WHERE id IN (
                        SELECT user_id FROM customer_users
                        WHERE customer_id = ? AND tenant_id = ?
                    )
                    "#,
                )
                .bind(new_email)
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(customer_id)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
        }

        if dto.is_active.is_some() {
            self.sync_customer_pppoe_disabled_state(tenant_id, customer_id, customer.is_active)
                .await?;
        }

        Ok(customer)
    }

    pub async fn delete_customer(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        // Pre-fetch linked portal user_ids BEFORE the cascade so we can
        // invalidate their sessions and broadcast a SessionInvalidated
        // event. Customer record alone does not delete the user (FK is on
        // customer_users), so without this the deleted customer's tab
        // would keep living on until JWT expiry.
        #[cfg(feature = "postgres")]
        let linked_user_ids: Vec<String> = sqlx::query_scalar(
            "SELECT user_id FROM customer_users WHERE tenant_id = $1 AND customer_id = $2",
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await?;
        #[cfg(not(feature = "postgres"))]
        let linked_user_ids: Vec<String> = sqlx::query_scalar(
            "SELECT user_id FROM customer_users WHERE tenant_id = ? AND customer_id = ?",
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await?;

        // FK guard: customers direferensikan RESTRICT oleh langganan, WO,
        // PPPoE, DHCP statis, lokasi, dsb. Tanpa guard ini DELETE membalas
        // 500 unique/FK-violation mentah. Hitung dulu, balas 400 deskriptif.
        let blockers = customer_delete_blockers(&self.pool, tenant_id, customer_id).await?;
        if !blockers.is_empty() {
            return Err(AppError::Validation(format!(
                "cannot delete: still referenced by {}",
                blockers.join(", ")
            )));
        }

        let mut tx = self.pool.begin().await?;
        self.auth_service
            .apply_rls_context_tx_values(&mut tx, Some(tenant_id), Some(actor_id), false)
            .await?;

        // Invalidate every session for the linked portal users so any
        // currently-logged-in customer tab immediately fails auth checks.
        for user_id in &linked_user_ids {
            #[cfg(feature = "postgres")]
            let _ = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            #[cfg(not(feature = "postgres"))]
            let _ = sqlx::query("DELETE FROM sessions WHERE user_id = ?")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
        }

        // Drop the customer_users link row(s).
        #[cfg(feature = "postgres")]
        let cust_user_res =
            sqlx::query("DELETE FROM customer_users WHERE tenant_id = $1 AND customer_id = $2")
                .bind(tenant_id)
                .bind(customer_id)
                .execute(&mut *tx)
                .await?;
        #[cfg(not(feature = "postgres"))]
        let cust_user_res =
            sqlx::query("DELETE FROM customer_users WHERE tenant_id = ? AND customer_id = ?")
                .bind(tenant_id)
                .bind(customer_id)
                .execute(&mut *tx)
                .await?;
        if cust_user_res.rows_affected() == 0 {
            warn!(
                "delete_customer called for {}/{} but no customer_users rows were linked",
                tenant_id, customer_id
            );
        }

        // Active customer row.
        #[cfg(feature = "postgres")]
        let res = sqlx::query("DELETE FROM customers WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(customer_id)
            .execute(&mut *tx)
            .await?;
        #[cfg(not(feature = "postgres"))]
        let res = sqlx::query("DELETE FROM customers WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(customer_id)
            .execute(&mut *tx)
            .await?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Customer not found".to_string()));
        }

        tx.commit().await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_DELETE",
                "customers",
                Some(customer_id),
                Some(&format!(
                    "Deleted customer and invalidated {} portal session(s)",
                    linked_user_ids.len()
                )),
                ip_address,
            )
            .await;

        // Broadcast SessionInvalidated so connected FEs can re-logout in real
        // time instead of waiting for the next JWT check. We do this AFTER
        // commit so the broadcast never references a state we rolled back.
        for user_id in &linked_user_ids {
            let event = crate::http::WsEvent::SessionInvalidated {
                user_id: user_id.clone(),
                reason: "customer_deleted".to_string(),
            };
            let _ = self.ws_hub.as_ref().map(|hub| hub.broadcast(event));
        }

        // Best-effort: also notify tenant admins that the customer was deleted
        // so they have an audit trail in their notification inbox.
        if let Ok(admin_ids) = self
            .fetch_tenant_admin_user_ids_for_notification(tenant_id)
            .await
        {
            for admin_user_id in admin_ids {
                if let Err(e) = self
                    .notification_service
                    .create_notification(
                        admin_user_id.clone(),
                        Some(tenant_id.to_string()),
                        format!("Customer deleted: {}", customer_id),
                        format!(
                            "Customer record {} was removed. Linked portal users have been logged out.",
                            customer_id
                        ),
                        "warning".to_string(),
                        "customers".to_string(),
                        Some("/admin/customers".to_string()),
                    )
                    .await
                {
                    warn!(
                        "Failed to notify tenant admin {} about customer deletion: {}",
                        admin_user_id, e
                    );
                }
            }
        }

        Ok(())
    }

    // =========================
    // Admin: Locations
    // =========================

    pub async fn list_locations(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
    ) -> AppResult<Vec<CustomerLocation>> {
        if self
            .auth_service
            .check_permission(actor_id, tenant_id, "customer_locations", "read")
            .await
            .is_err()
        {
            self.auth_service
                .check_permission(actor_id, tenant_id, "orders", "create")
                .await?;
        }

        // Ensure customer is within tenant
        let _ = self.get_customer(actor_id, tenant_id, customer_id).await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<CustomerLocation> = sqlx::query_as(
            r#"
            SELECT
                id,
                tenant_id,
                customer_id,
                label,
                address_line1,
                address_line2,
                city,
                state,
                postal_code,
                country,
                latitude::float8 AS latitude,
                longitude::float8 AS longitude,
                notes,
                created_at,
                updated_at
            FROM customer_locations
            WHERE tenant_id = $1 AND customer_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<CustomerLocation> = sqlx::query_as(
            "SELECT * FROM customer_locations WHERE tenant_id = ? AND customer_id = ? ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn create_location(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateCustomerLocationRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerLocation> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customer_locations", "manage")
            .await?;

        let _ = self
            .get_customer(actor_id, tenant_id, &dto.customer_id)
            .await?;

        let loc = CustomerLocation::new(
            tenant_id.to_string(),
            dto.customer_id,
            dto.label,
            dto.address_line1,
            dto.address_line2,
            dto.city,
            dto.state,
            dto.postal_code,
            dto.country,
            dto.latitude,
            dto.longitude,
            dto.notes,
        );

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customer_locations
                (id, tenant_id, customer_id, label, address_line1, address_line2, city, state, postal_code, country, latitude, longitude, notes, created_at, updated_at)
            VALUES
                ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            "#,
        )
        .bind(&loc.id)
        .bind(&loc.tenant_id)
        .bind(&loc.customer_id)
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.created_at)
        .bind(loc.updated_at)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO customer_locations
                (id, tenant_id, customer_id, label, address_line1, address_line2, city, state, postal_code, country, latitude, longitude, notes, created_at, updated_at)
            VALUES
                (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
            "#,
        )
        .bind(&loc.id)
        .bind(&loc.tenant_id)
        .bind(&loc.customer_id)
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.created_at.to_rfc3339())
        .bind(loc.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_LOCATION_CREATE",
                "customer_locations",
                Some(&loc.id),
                Some("Created customer location"),
                ip_address,
            )
            .await;

        Ok(loc)
    }

    pub async fn update_location(
        &self,
        actor_id: &str,
        tenant_id: &str,
        location_id: &str,
        dto: UpdateCustomerLocationRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerLocation> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customer_locations", "manage")
            .await?;

        #[cfg(feature = "postgres")]
        let mut loc: CustomerLocation = sqlx::query_as(
            r#"
            SELECT
                id,
                tenant_id,
                customer_id,
                label,
                address_line1,
                address_line2,
                city,
                state,
                postal_code,
                country,
                latitude::float8 AS latitude,
                longitude::float8 AS longitude,
                notes,
                created_at,
                updated_at
            FROM customer_locations
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Location not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let mut loc: CustomerLocation =
            sqlx::query_as("SELECT * FROM customer_locations WHERE tenant_id = ? AND id = ?")
                .bind(tenant_id)
                .bind(location_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| AppError::NotFound("Location not found".to_string()))?;

        if let Some(v) = dto.label {
            let vv = v.trim().to_string();
            if !vv.is_empty() {
                loc.label = vv;
            }
        }
        if let Some(v) = dto.address_line1 {
            let vv = v.trim().to_string();
            loc.address_line1 = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.address_line2 {
            let vv = v.trim().to_string();
            loc.address_line2 = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.city {
            let vv = v.trim().to_string();
            loc.city = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.state {
            let vv = v.trim().to_string();
            loc.state = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.postal_code {
            let vv = v.trim().to_string();
            loc.postal_code = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.country {
            let vv = v.trim().to_string();
            loc.country = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.latitude {
            loc.latitude = Some(v);
        }
        if let Some(v) = dto.longitude {
            loc.longitude = Some(v);
        }
        if let Some(v) = dto.notes {
            let vv = v.trim().to_string();
            loc.notes = if vv.is_empty() { None } else { Some(vv) };
        }
        loc.updated_at = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE customer_locations
            SET label=$1, address_line1=$2, address_line2=$3, city=$4, state=$5, postal_code=$6, country=$7,
                latitude=$8, longitude=$9, notes=$10, updated_at=$11
            WHERE tenant_id=$12 AND id=$13
            "#,
        )
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.updated_at)
        .bind(tenant_id)
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE customer_locations
            SET label=?, address_line1=?, address_line2=?, city=?, state=?, postal_code=?, country=?,
                latitude=?, longitude=?, notes=?, updated_at=?
            WHERE tenant_id=? AND id=?
            "#,
        )
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.updated_at.to_rfc3339())
        .bind(tenant_id)
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_LOCATION_UPDATE",
                "customer_locations",
                Some(location_id),
                Some("Updated customer location"),
                ip_address,
            )
            .await;

        Ok(loc)
    }

    pub async fn delete_location(
        &self,
        actor_id: &str,
        tenant_id: &str,
        location_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customer_locations", "manage")
            .await?;

        let blockers = location_delete_blockers(&self.pool, tenant_id, location_id).await?;
        if !blockers.is_empty() {
            return Err(AppError::Validation(format!(
                "cannot delete: still referenced by {}",
                blockers.join(", ")
            )));
        }

        #[cfg(feature = "postgres")]
        let res = sqlx::query("DELETE FROM customer_locations WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(location_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let res = sqlx::query("DELETE FROM customer_locations WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(location_id)
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Location not found".to_string()));
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_LOCATION_DELETE",
                "customer_locations",
                Some(location_id),
                Some("Deleted customer location"),
                ip_address,
            )
            .await;

        Ok(())
    }

    // =========================
    // Admin: Portal Users
    // =========================

    pub async fn list_portal_users(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
    ) -> AppResult<Vec<CustomerPortalUser>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read")
            .await?;

        let _ = self.get_customer(actor_id, tenant_id, customer_id).await?;

        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT
                cu.id as customer_user_id,
                u.id as user_id,
                u.email as email,
                u.name as name,
                cu.created_at as created_at
            FROM customer_users cu
            JOIN users u ON u.id = cu.user_id
            WHERE cu.tenant_id = $1 AND cu.customer_id = $2
            ORDER BY cu.created_at DESC
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT
                cu.id as customer_user_id,
                u.id as user_id,
                u.email as email,
                u.name as name,
                cu.created_at as created_at
            FROM customer_users cu
            JOIN users u ON u.id = cu.user_id
            WHERE cu.tenant_id = ? AND cu.customer_id = ?
            ORDER BY cu.created_at DESC
        "#;

        let rows: Vec<CustomerPortalUser> = sqlx::query_as(query)
            .bind(tenant_id)
            .bind(customer_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows)
    }

    pub async fn add_portal_user(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: AddCustomerPortalUserRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerPortalUser> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let _ = self
            .get_customer(actor_id, tenant_id, &dto.customer_id)
            .await?;

        let cu = CustomerUser::new(tenant_id.to_string(), dto.customer_id, dto.user_id);

        #[cfg(feature = "postgres")]
        {
            let res = sqlx::query(
                "INSERT INTO customer_users (id, tenant_id, customer_id, user_id, created_at) VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(&cu.id)
            .bind(&cu.tenant_id)
            .bind(&cu.customer_id)
            .bind(&cu.user_id)
            .bind(cu.created_at)
            .execute(&self.pool)
            .await;

            if let Err(e) = res {
                let is_unique = e
                    .as_database_error()
                    .and_then(|d| d.code().map(|c| c == "23505"))
                    .unwrap_or(false);
                if is_unique {
                    return Err(AppError::Validation(
                        "This user is already linked to a customer in this tenant.".to_string(),
                    ));
                }
                return Err(e.into());
            }
        }

        #[cfg(feature = "sqlite")]
        {
            // SQLite uses OR IGNORE to avoid hard failure on duplicates.
            sqlx::query(
                "INSERT OR IGNORE INTO customer_users (id, tenant_id, customer_id, user_id, created_at) VALUES (?,?,?,?,?)",
            )
            .bind(&cu.id)
            .bind(&cu.tenant_id)
            .bind(&cu.customer_id)
            .bind(&cu.user_id)
            .bind(cu.created_at.to_rfc3339())
            .execute(&self.pool)
            .await?;
        }

        // Ensure customer can login: add tenant_members entry with Customer role if missing.
        let customer_role_id = self.get_system_role_id_by_name("Customer").await?;
        self.ensure_tenant_member_role(tenant_id, &cu.user_id, &customer_role_id)
            .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_USER_ADD",
                "customer_users",
                Some(&cu.id),
                Some("Added portal user to customer"),
                ip_address,
            )
            .await;

        // Return joined projection
        #[cfg(feature = "postgres")]
        let row: CustomerPortalUser = sqlx::query_as(
            r#"
            SELECT
                cu.id as customer_user_id,
                u.id as user_id,
                u.email as email,
                u.name as name,
                cu.created_at as created_at
            FROM customer_users cu
            JOIN users u ON u.id = cu.user_id
            WHERE cu.id = $1
            "#,
        )
        .bind(&cu.id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: CustomerPortalUser = sqlx::query_as(
            r#"
            SELECT
                cu.id as customer_user_id,
                u.id as user_id,
                u.email as email,
                u.name as name,
                cu.created_at as created_at
            FROM customer_users cu
            JOIN users u ON u.id = cu.user_id
            WHERE cu.id = ?
            "#,
        )
        .bind(&cu.id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub fn normalize_service_lifecycle_issue_type(value: Option<&str>) -> AppResult<&'static str> {
        match value
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "missing_bootstrap_invoice" => Ok("missing_bootstrap_invoice"),
            "invalid_active_lifecycle" => Ok("invalid_active_lifecycle"),
            _ => Err(AppError::Validation(
                "issue_type must be missing_bootstrap_invoice or invalid_active_lifecycle"
                    .to_string(),
            )),
        }
    }

    pub async fn create_portal_user(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateCustomerPortalUserRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerPortalUser> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let _ = self
            .get_customer(actor_id, tenant_id, &dto.customer_id)
            .await?;

        let user = self
            .user_service
            .create(
                crate::models::CreateUserDto {
                    email: dto.email,
                    name: dto.name,
                    password: dto.password,
                },
                Some(actor_id),
                ip_address,
            )
            .await?;

        let row = self
            .add_portal_user(
                actor_id,
                tenant_id,
                AddCustomerPortalUserRequest {
                    customer_id: dto.customer_id,
                    user_id: user.id.clone(),
                },
                ip_address,
            )
            .await?;

        Ok(row)
    }

    pub async fn remove_portal_user(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_user_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        #[cfg(feature = "postgres")]
        let res = sqlx::query("DELETE FROM customer_users WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(customer_user_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let res = sqlx::query("DELETE FROM customer_users WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(customer_user_id)
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Portal user mapping not found".to_string(),
            ));
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_USER_REMOVE",
                "customer_users",
                Some(customer_user_id),
                Some("Removed portal user from customer"),
                ip_address,
            )
            .await;

        Ok(())
    }

    pub async fn reset_portal_user_password(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_user_id: &str,
        new_password: Option<&str>,
        ip_address: Option<&str>,
    ) -> AppResult<ResetCustomerPortalPasswordResponse> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        #[cfg(feature = "postgres")]
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT u.id, u.email FROM customer_users cu JOIN users u ON u.id = cu.user_id WHERE cu.tenant_id = $1 AND cu.id = $2",
        )
        .bind(tenant_id)
        .bind(customer_user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Portal user not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT u.id, u.email FROM customer_users cu JOIN users u ON u.id = cu.user_id WHERE cu.tenant_id = ? AND cu.id = ?",
        )
        .bind(tenant_id)
        .bind(customer_user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Portal user not found".to_string()))?;

        let (user_id, email) = row;
        let password = match new_password {
            Some(p) if !p.trim().is_empty() => p.trim().to_string(),
            _ => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                (0..12)
                    .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                    .collect()
            }
        };

        if password.len() < 6 {
            return Err(AppError::Validation(
                "Password must be at least 6 characters".to_string(),
            ));
        }

        let hash = AuthService::hash_password(&password)?;

        #[cfg(feature = "postgres")]
        sqlx::query("UPDATE users SET password_hash = $1, updated_at = $2 WHERE id = $3")
            .bind(&hash)
            .bind(Utc::now())
            .bind(&user_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
            .bind(&hash)
            .bind(Utc::now().to_rfc3339())
            .bind(&user_id)
            .execute(&self.pool)
            .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_PASSWORD_RESET",
                "customer_users",
                Some(customer_user_id),
                Some("Admin reset portal user password"),
                ip_address,
            )
            .await;

        Ok(ResetCustomerPortalPasswordResponse {
            customer_user_id: customer_user_id.to_string(),
            email,
            generated_password: if new_password.is_none()
                || new_password.unwrap_or("").trim().is_empty()
            {
                Some(password)
            } else {
                None
            },
        })
    }

    // =========================
    // Admin: Customer Subscriptions
    // =========================
    pub async fn list_customer_subscription_options(
        &self,
        actor_id: &str,
        tenant_id: &str,
        limit: u32,
    ) -> AppResult<Vec<CustomerSubscriptionOption>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "billing", "read")
            .await?;

        let capped_limit = limit.clamp(1, 5000) as i64;

        #[cfg(feature = "postgres")]
        let rows: Vec<CustomerSubscriptionOption> = sqlx::query_as(
            r#"
            SELECT
              cs.id,
              cs.customer_id,
              COALESCE(c.name, '') AS customer_name,
              p.name AS package_name,
              cs.billing_cycle,
              cs.status
            FROM customer_subscriptions cs
            LEFT JOIN customers c ON c.id = cs.customer_id
            LEFT JOIN isp_packages p ON p.id = cs.package_id
            WHERE cs.tenant_id = $1
            ORDER BY c.name ASC, cs.updated_at DESC
            LIMIT $2
            "#,
        )
        .bind(tenant_id)
        .bind(capped_limit)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<CustomerSubscriptionOption> = sqlx::query_as(
            r#"
            SELECT
              cs.id,
              cs.customer_id,
              COALESCE(c.name, '') AS customer_name,
              p.name AS package_name,
              cs.billing_cycle,
              cs.status
            FROM customer_subscriptions cs
            LEFT JOIN customers c ON c.id = cs.customer_id
            LEFT JOIN isp_packages p ON p.id = cs.package_id
            WHERE cs.tenant_id = ?
            ORDER BY c.name ASC, cs.updated_at DESC
            LIMIT ?
            "#,
        )
        .bind(tenant_id)
        .bind(capped_limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn list_customer_subscriptions(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
        page: u32,
        per_page: u32,
    ) -> AppResult<PaginatedResponse<CustomerSubscriptionView>> {
        if self
            .auth_service
            .check_permission(actor_id, tenant_id, "customers", "read")
            .await
            .is_err()
        {
            if self
                .auth_service
                .check_permission(actor_id, tenant_id, "work_orders", "manage")
                .await
                .is_err()
            {
                self.auth_service
                    .check_permission(actor_id, tenant_id, "work_orders", "read")
                    .await?;
            }
        }

        let pg = crate::services::pagination::normalize(page, per_page);
        let per_page = pg.per_page;
        let offset = pg.offset;

        #[cfg(feature = "postgres")]
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM customer_subscriptions WHERE tenant_id = $1 AND customer_id = $2",
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM customer_subscriptions WHERE tenant_id = ? AND customer_id = ?",
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<CustomerSubscriptionView> = sqlx::query_as(
            r#"
            WITH latest_work_orders AS (
              SELECT
                iwo.id,
                iwo.tenant_id,
                iwo.subscription_id,
                iwo.status,
                ROW_NUMBER() OVER (
                  PARTITION BY iwo.tenant_id, iwo.subscription_id
                  ORDER BY iwo.created_at DESC, iwo.id DESC
                ) AS rn
              FROM installation_work_orders iwo
              WHERE iwo.tenant_id = $1
            ),
            latest_reschedules AS (
              SELECT
                worr.tenant_id,
                iwo.subscription_id,
                worr.status,
                CAST(worr.requested_schedule_at AS TEXT) AS requested_schedule_at,
                ROW_NUMBER() OVER (
                  PARTITION BY worr.tenant_id, iwo.subscription_id
                  ORDER BY worr.created_at DESC, worr.id DESC
                ) AS rn
              FROM work_order_reschedule_requests worr
              JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
              WHERE worr.tenant_id = $1
            ),
            latest_pppoe_accounts AS (
              SELECT
                pa.tenant_id,
                pa.location_id,
                pa.address_pool,
                pa.disabled,
                ROW_NUMBER() OVER (
                  PARTITION BY pa.tenant_id, pa.location_id
                  ORDER BY pa.updated_at DESC, pa.created_at DESC, pa.id DESC
                ) AS rn
              FROM pppoe_accounts pa
              WHERE pa.tenant_id = $1
            )
            SELECT
              cs.id,
              cs.tenant_id,
              cs.customer_id,
              cs.location_id,
              cs.package_id,
              cs.router_id,
              cs.billing_cycle,
              cs.price::float8 AS price,
              cs.currency_code,
              cs.status,
              cs.starts_at,
              cs.ends_at,
              cs.grace_started_at,
              cs.grace_until,
              cs.notes,
              cs.created_at,
              cs.updated_at,
              p.name AS package_name,
              l.label AS location_label,
              r.name AS router_name,
              lpa.address_pool AS pppoe_address_pool,
              iprm.isolation_pool AS pppoe_isolation_pool,
              lpa.disabled AS pppoe_disabled,
              lwo.id AS latest_work_order_id,
              lwo.status AS latest_work_order_status,
              CASE
                WHEN LOWER(cs.status) = 'cancelled' THEN true
                WHEN COALESCE(LOWER(lwo.status), '') = 'cancelled' THEN true
                ELSE false
              END AS can_request_reopen,
              lr.status AS latest_reschedule_status,
              lr.requested_schedule_at AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p ON p.id = cs.package_id
            LEFT JOIN customer_locations l ON l.id = cs.location_id
            LEFT JOIN mikrotik_routers r ON r.id = cs.router_id
            LEFT JOIN latest_pppoe_accounts lpa
              ON lpa.tenant_id = cs.tenant_id
             AND lpa.location_id = cs.location_id
             AND lpa.rn = 1
            LEFT JOIN isp_package_router_mappings iprm
              ON iprm.tenant_id = cs.tenant_id
             AND iprm.package_id = cs.package_id
             AND iprm.router_id = cs.router_id
            LEFT JOIN latest_work_orders lwo
              ON lwo.tenant_id = cs.tenant_id
             AND lwo.subscription_id = cs.id
             AND lwo.rn = 1
            LEFT JOIN latest_reschedules lr
              ON lr.tenant_id = cs.tenant_id
             AND lr.subscription_id = cs.id
             AND lr.rn = 1
            WHERE cs.tenant_id = $1 AND cs.customer_id = $2
            ORDER BY cs.updated_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<CustomerSubscriptionView> = sqlx::query_as(
            r#"
            WITH latest_work_orders AS (
              SELECT
                iwo.id,
                iwo.tenant_id,
                iwo.subscription_id,
                iwo.status,
                ROW_NUMBER() OVER (
                  PARTITION BY iwo.tenant_id, iwo.subscription_id
                  ORDER BY iwo.created_at DESC, iwo.id DESC
                ) AS rn
              FROM installation_work_orders iwo
              WHERE iwo.tenant_id = ?
            ),
            latest_reschedules AS (
              SELECT
                worr.tenant_id,
                iwo.subscription_id,
                worr.status,
                CAST(worr.requested_schedule_at AS TEXT) AS requested_schedule_at,
                ROW_NUMBER() OVER (
                  PARTITION BY worr.tenant_id, iwo.subscription_id
                  ORDER BY worr.created_at DESC, worr.id DESC
                ) AS rn
              FROM work_order_reschedule_requests worr
              JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
              WHERE worr.tenant_id = ?
            ),
            latest_pppoe_accounts AS (
              SELECT
                pa.tenant_id,
                pa.location_id,
                pa.address_pool,
                pa.disabled,
                ROW_NUMBER() OVER (
                  PARTITION BY pa.tenant_id, pa.location_id
                  ORDER BY pa.updated_at DESC, pa.created_at DESC, pa.id DESC
                ) AS rn
              FROM pppoe_accounts pa
              WHERE pa.tenant_id = ?
            )
            SELECT
              cs.id,
              cs.tenant_id,
              cs.customer_id,
              cs.location_id,
              cs.package_id,
              cs.router_id,
              cs.billing_cycle,
              cs.price AS price,
              cs.currency_code,
              cs.status,
              cs.starts_at,
              cs.ends_at,
              cs.notes,
              cs.created_at,
              cs.updated_at,
              p.name AS package_name,
              l.label AS location_label,
              r.name AS router_name,
              lpa.address_pool AS pppoe_address_pool,
              iprm.isolation_pool AS pppoe_isolation_pool,
              lpa.disabled AS pppoe_disabled,
              lwo.id AS latest_work_order_id,
              lwo.status AS latest_work_order_status,
              CASE
                WHEN LOWER(cs.status) = 'cancelled' THEN 1
                WHEN COALESCE(LOWER(lwo.status), '') = 'cancelled' THEN 1
                ELSE 0
              END AS can_request_reopen,
              lr.status AS latest_reschedule_status,
              lr.requested_schedule_at AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p ON p.id = cs.package_id
            LEFT JOIN customer_locations l ON l.id = cs.location_id
            LEFT JOIN mikrotik_routers r ON r.id = cs.router_id
            LEFT JOIN latest_pppoe_accounts lpa
              ON lpa.tenant_id = cs.tenant_id
             AND lpa.location_id = cs.location_id
             AND lpa.rn = 1
            LEFT JOIN isp_package_router_mappings iprm
              ON iprm.tenant_id = cs.tenant_id
             AND iprm.package_id = cs.package_id
             AND iprm.router_id = cs.router_id
            LEFT JOIN latest_work_orders lwo
              ON lwo.tenant_id = cs.tenant_id
             AND lwo.subscription_id = cs.id
             AND lwo.rn = 1
            LEFT JOIN latest_reschedules lr
              ON lr.tenant_id = cs.tenant_id
             AND lr.subscription_id = cs.id
             AND lr.rn = 1
            WHERE cs.tenant_id = ? AND cs.customer_id = ?
            ORDER BY cs.updated_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(tenant_id)
        .bind(tenant_id)
        .bind(tenant_id)
        .bind(tenant_id)
        .bind(customer_id)
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(PaginatedResponse {
            data: rows,
            total,
            page,
            per_page,
        })
    }

    pub async fn get_lifecycle_observability(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: Option<&str>,
    ) -> AppResult<CustomerLifecycleObservability> {
        if self
            .auth_service
            .check_permission(actor_id, tenant_id, "customers", "read")
            .await
            .is_err()
        {
            if self
                .auth_service
                .check_permission(actor_id, tenant_id, "work_orders", "manage")
                .await
                .is_err()
            {
                self.auth_service
                    .check_permission(actor_id, tenant_id, "work_orders", "read")
                    .await?;
            }
        }

        if let Some(customer_id) = customer_id {
            #[cfg(feature = "postgres")]
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM customers WHERE tenant_id = $1 AND id = $2)",
            )
            .bind(tenant_id)
            .bind(customer_id)
            .fetch_one(&self.pool)
            .await?;

            #[cfg(feature = "sqlite")]
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM customers WHERE tenant_id = ? AND id = ?)",
            )
            .bind(tenant_id)
            .bind(customer_id)
            .fetch_one(&self.pool)
            .await?;

            if !exists {
                return Err(AppError::NotFound("Customer not found".to_string()));
            }
        }

        #[cfg(feature = "postgres")]
        let lifecycle_rows: Vec<LifecycleStageRow> = sqlx::query_as(
            r#"
            WITH lifecycle_stages(stage, rank) AS (
              VALUES
                ('pending_installation', 1),
                ('grace_active', 2),
                ('active', 3),
                ('cancelled', 4)
            )
            SELECT ls.stage, COALESCE(COUNT(cs.id), 0) AS count
            FROM lifecycle_stages ls
            LEFT JOIN customer_subscriptions cs
              ON cs.tenant_id = $1
             AND LOWER(cs.status) = ls.stage
             AND ($2::text IS NULL OR cs.customer_id = $2)
            GROUP BY ls.stage, ls.rank
            ORDER BY ls.rank
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let lifecycle_rows: Vec<LifecycleStageRow> = sqlx::query_as(
            r#"
            WITH lifecycle_stages(stage, rank) AS (
              VALUES
                ('pending_installation', 1),
                ('grace_active', 2),
                ('active', 3),
                ('cancelled', 4)
            )
            SELECT ls.stage, COALESCE(COUNT(cs.id), 0) AS count
            FROM lifecycle_stages ls
            LEFT JOIN customer_subscriptions cs
              ON cs.tenant_id = ?
             AND LOWER(cs.status) = ls.stage
             AND (? IS NULL OR cs.customer_id = ?)
            GROUP BY ls.stage, ls.rank
            ORDER BY ls.rank
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let work_order_rows: Vec<LifecycleStageRow> = sqlx::query_as(
            r#"
            WITH work_order_stages(stage, rank) AS (
              VALUES
                ('pending', 1),
                ('in_progress', 2),
                ('completed', 3)
            )
            SELECT ws.stage, COALESCE(COUNT(iwo.id), 0) AS count
            FROM work_order_stages ws
            LEFT JOIN installation_work_orders iwo
              ON iwo.tenant_id = $1
             AND LOWER(iwo.status) = ws.stage
             AND ($2::text IS NULL OR iwo.customer_id = $2)
            GROUP BY ws.stage, ws.rank
            ORDER BY ws.rank
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let work_order_rows: Vec<LifecycleStageRow> = sqlx::query_as(
            r#"
            WITH work_order_stages(stage, rank) AS (
              VALUES
                ('pending', 1),
                ('in_progress', 2),
                ('completed', 3)
            )
            SELECT ws.stage, COALESCE(COUNT(iwo.id), 0) AS count
            FROM work_order_stages ws
            LEFT JOIN installation_work_orders iwo
              ON iwo.tenant_id = ?
             AND LOWER(iwo.status) = ws.stage
             AND (? IS NULL OR iwo.customer_id = ?)
            GROUP BY ws.stage, ws.rank
            ORDER BY ws.rank
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let aging_rows: Vec<AgingBucketRow> = sqlx::query_as(
            r#"
            WITH buckets(bucket, rank, min_days, max_days) AS (
              VALUES
                ('0-1d', 1, 0, 1),
                ('2-3d', 2, 2, 3),
                ('4-7d', 3, 4, 7),
                ('>7d', 4, 8, NULL)
            ),
            waiting_subscriptions AS (
              SELECT GREATEST(0, FLOOR(EXTRACT(EPOCH FROM ($3 - COALESCE(cs.updated_at, cs.created_at))) / 86400))::int AS age_days
              FROM customer_subscriptions cs
              WHERE cs.tenant_id = $1
                AND LOWER(cs.status) IN ('pending_installation', 'installation_done_awaiting_payment', 'grace_active')
                AND ($2::text IS NULL OR cs.customer_id = $2)
            )
            SELECT b.bucket, COALESCE(COUNT(ws.age_days), 0) AS count
            FROM buckets b
            LEFT JOIN waiting_subscriptions ws
              ON ws.age_days >= b.min_days
             AND (b.max_days IS NULL OR ws.age_days <= b.max_days)
            GROUP BY b.bucket, b.rank
            ORDER BY b.rank
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(Utc::now())
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let aging_rows: Vec<AgingBucketRow> = sqlx::query_as(
            r#"
            WITH buckets(bucket, rank, min_days, max_days) AS (
              VALUES
                ('0-1d', 1, 0, 1),
                ('2-3d', 2, 2, 3),
                ('4-7d', 3, 4, 7),
                ('>7d', 4, 8, NULL)
            ),
            waiting_subscriptions AS (
              SELECT CAST(
                MAX(
                  0,
                  CAST((julianday(?) - julianday(COALESCE(cs.updated_at, cs.created_at))) AS INTEGER)
                ) AS INTEGER
              ) AS age_days
              FROM customer_subscriptions cs
              WHERE cs.tenant_id = ?
                AND LOWER(cs.status) IN ('pending_installation', 'installation_done_awaiting_payment', 'grace_active')
                AND (? IS NULL OR cs.customer_id = ?)
            )
            SELECT b.bucket, COALESCE(COUNT(ws.age_days), 0) AS count
            FROM buckets b
            LEFT JOIN waiting_subscriptions ws
              ON ws.age_days >= b.min_days
             AND (b.max_days IS NULL OR ws.age_days <= b.max_days)
            GROUP BY b.bucket, b.rank
            ORDER BY b.rank
            "#,
        )
        .bind(Utc::now())
        .bind(tenant_id)
        .bind(customer_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(CustomerLifecycleObservability {
            generated_at: Utc::now(),
            lifecycle_funnel: lifecycle_rows
                .into_iter()
                .map(|row| CustomerLifecycleStageMetric {
                    stage: row.stage,
                    count: row.count,
                })
                .collect(),
            work_order_funnel: work_order_rows
                .into_iter()
                .map(|row| CustomerLifecycleStageMetric {
                    stage: row.stage,
                    count: row.count,
                })
                .collect(),
            aging_buckets: aging_rows
                .into_iter()
                .map(|row| CustomerLifecycleAgingBucket {
                    bucket: row.bucket,
                    count: row.count,
                })
                .collect(),
        })
    }

    // =========================
    // Portal: Self-service
    // =========================

    pub async fn get_portal_customer_id(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<String> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read_own")
            .await?;

        #[cfg(feature = "postgres")]
        let customer_id: Option<String> = sqlx::query_scalar(
            "SELECT customer_id FROM customer_users WHERE tenant_id = $1 AND user_id = $2 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let customer_id: Option<String> = sqlx::query_scalar(
            "SELECT customer_id FROM customer_users WHERE tenant_id = ? AND user_id = ? LIMIT 1",
        )
        .bind(tenant_id)
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?;

        customer_id
            .ok_or_else(|| AppError::Forbidden("You are not linked to any customer".to_string()))
    }

    pub async fn list_my_locations(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<Vec<CustomerLocation>> {
        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<CustomerLocation> = sqlx::query_as(
            r#"
            SELECT
                id,
                tenant_id,
                customer_id,
                label,
                address_line1,
                address_line2,
                city,
                state,
                postal_code,
                country,
                latitude::float8 AS latitude,
                longitude::float8 AS longitude,
                notes,
                created_at,
                updated_at
            FROM customer_locations
            WHERE tenant_id = $1 AND customer_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<CustomerLocation> = sqlx::query_as(
            "SELECT * FROM customer_locations WHERE tenant_id = ? AND customer_id = ? ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn get_my_location_or_404(
        &self,
        actor_id: &str,
        tenant_id: &str,
        location_id: &str,
    ) -> AppResult<CustomerLocation> {
        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let loc: Option<CustomerLocation> = sqlx::query_as(
            r#"
            SELECT
                id,
                tenant_id,
                customer_id,
                label,
                address_line1,
                address_line2,
                city,
                state,
                postal_code,
                country,
                latitude::float8 AS latitude,
                longitude::float8 AS longitude,
                notes,
                created_at,
                updated_at
            FROM customer_locations
            WHERE tenant_id = $1 AND customer_id = $2 AND id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let loc: Option<CustomerLocation> = sqlx::query_as(
            "SELECT * FROM customer_locations WHERE tenant_id = ? AND customer_id = ? AND id = ?",
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await?;

        loc.ok_or_else(|| AppError::NotFound("Location not found".to_string()))
    }

    pub async fn create_my_location(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateMyCustomerLocationRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerLocation> {
        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;
        let label = dto.label.trim().to_string();
        if label.is_empty() {
            return Err(AppError::Validation("label is required".to_string()));
        }
        let (latitude, longitude) =
            Self::validate_location_coordinates(dto.latitude, dto.longitude)?;

        let loc = CustomerLocation::new(
            tenant_id.to_string(),
            customer_id,
            label,
            dto.address_line1,
            dto.address_line2,
            dto.city,
            dto.state,
            dto.postal_code,
            dto.country,
            Some(latitude),
            Some(longitude),
            dto.notes,
        );

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customer_locations
                (id, tenant_id, customer_id, label, address_line1, address_line2, city, state, postal_code, country, latitude, longitude, notes, created_at, updated_at)
            VALUES
                ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            "#,
        )
        .bind(&loc.id)
        .bind(&loc.tenant_id)
        .bind(&loc.customer_id)
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.created_at)
        .bind(loc.updated_at)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO customer_locations
                (id, tenant_id, customer_id, label, address_line1, address_line2, city, state, postal_code, country, latitude, longitude, notes, created_at, updated_at)
            VALUES
                (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
            "#,
        )
        .bind(&loc.id)
        .bind(&loc.tenant_id)
        .bind(&loc.customer_id)
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.created_at.to_rfc3339())
        .bind(loc.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_LOCATION_SELF_CREATE",
                "customer_locations",
                Some(&loc.id),
                Some("Created own customer location from portal"),
                ip_address,
            )
            .await;

        Ok(loc)
    }

    pub async fn update_my_location(
        &self,
        actor_id: &str,
        tenant_id: &str,
        location_id: &str,
        dto: UpdateCustomerLocationRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerLocation> {
        let mut loc = self
            .get_my_location_or_404(actor_id, tenant_id, location_id)
            .await?;

        if let Some(v) = dto.label {
            let vv = v.trim().to_string();
            if vv.is_empty() {
                return Err(AppError::Validation("label is required".to_string()));
            }
            loc.label = vv;
        }
        if let Some(v) = dto.address_line1 {
            let vv = v.trim().to_string();
            loc.address_line1 = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.address_line2 {
            let vv = v.trim().to_string();
            loc.address_line2 = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.city {
            let vv = v.trim().to_string();
            loc.city = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.state {
            let vv = v.trim().to_string();
            loc.state = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.postal_code {
            let vv = v.trim().to_string();
            loc.postal_code = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.country {
            let vv = v.trim().to_string();
            loc.country = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.latitude {
            loc.latitude = Some(v);
        }
        if let Some(v) = dto.longitude {
            loc.longitude = Some(v);
        }
        if let Some(v) = dto.notes {
            let vv = v.trim().to_string();
            loc.notes = if vv.is_empty() { None } else { Some(vv) };
        }

        let (latitude, longitude) =
            Self::validate_location_coordinates(loc.latitude, loc.longitude)?;
        loc.latitude = Some(latitude);
        loc.longitude = Some(longitude);
        loc.updated_at = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE customer_locations
            SET label=$1, address_line1=$2, address_line2=$3, city=$4, state=$5, postal_code=$6, country=$7,
                latitude=$8, longitude=$9, notes=$10, updated_at=$11
            WHERE tenant_id=$12 AND customer_id=$13 AND id=$14
            "#,
        )
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.updated_at)
        .bind(tenant_id)
        .bind(&loc.customer_id)
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE customer_locations
            SET label=?, address_line1=?, address_line2=?, city=?, state=?, postal_code=?, country=?,
                latitude=?, longitude=?, notes=?, updated_at=?
            WHERE tenant_id=? AND customer_id=? AND id=?
            "#,
        )
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.updated_at.to_rfc3339())
        .bind(tenant_id)
        .bind(&loc.customer_id)
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "PORTAL_CUSTOMER_LOCATION_UPDATE",
                "customer_locations",
                Some(location_id),
                Some("Portal user updated customer location"),
                ip_address,
            )
            .await;

        Ok(loc)
    }

    pub async fn delete_my_location(
        &self,
        actor_id: &str,
        tenant_id: &str,
        location_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        let loc = self
            .get_my_location_or_404(actor_id, tenant_id, location_id)
            .await?;

        #[cfg(feature = "postgres")]
        let res = sqlx::query(
            "DELETE FROM customer_locations WHERE tenant_id = $1 AND customer_id = $2 AND id = $3",
        )
        .bind(tenant_id)
        .bind(&loc.customer_id)
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let res = sqlx::query(
            "DELETE FROM customer_locations WHERE tenant_id = ? AND customer_id = ? AND id = ?",
        )
        .bind(tenant_id)
        .bind(&loc.customer_id)
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Location not found".to_string()));
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "PORTAL_CUSTOMER_LOCATION_DELETE",
                "customer_locations",
                Some(location_id),
                Some("Portal user deleted customer location"),
                ip_address,
            )
            .await;

        Ok(())
    }

    pub async fn list_my_packages(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<Vec<IspPackage>> {
        let _customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<IspPackage> = sqlx::query_as(
            r#"
            SELECT
              id,
              tenant_id,
              service_type,
              provisioning_type,
              name,
              description,
              features,
              is_active,
              price_monthly::float8 AS price_monthly,
              price_yearly::float8 AS price_yearly,
              created_at,
              updated_at
            FROM isp_packages
            WHERE tenant_id = $1
              AND is_active = true
            ORDER BY price_monthly ASC, name ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<IspPackage> = sqlx::query_as(
            r#"
            SELECT
              id,
              tenant_id,
              service_type,
              provisioning_type,
              name,
              description,
              features,
              is_active,
              price_monthly AS price_monthly,
              price_yearly AS price_yearly,
              created_at,
              updated_at
            FROM isp_packages
            WHERE tenant_id = ?
              AND is_active = 1
            ORDER BY price_monthly ASC, name ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub(super) async fn auto_provision_pppoe_for_subscription(
        &self,
        actor_id: &str,
        tenant_id: &str,
        sub: &CustomerSubscription,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        if sub.status != "active" {
            return Ok(());
        }
        let Some(router_id) = sub.router_id.as_deref() else {
            return Ok(());
        };
        if router_id.trim().is_empty() {
            return Ok(());
        }

        #[derive(sqlx::FromRow)]
        struct MappingRow {
            router_profile_name: String,
            address_pool: Option<String>,
        }

        #[cfg(feature = "postgres")]
        let mapping: Option<MappingRow> = sqlx::query_as(
            r#"
            SELECT router_profile_name, address_pool
            FROM isp_package_router_mappings
            WHERE tenant_id = $1 AND router_id = $2 AND package_id = $3
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(&sub.package_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let mapping: Option<MappingRow> = sqlx::query_as(
            r#"
            SELECT router_profile_name, address_pool
            FROM isp_package_router_mappings
            WHERE tenant_id = ? AND router_id = ? AND package_id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(&sub.package_id)
        .fetch_optional(&self.pool)
        .await?;

        let mapping = mapping.ok_or_else(|| {
            AppError::Validation(
                "PPPoE auto-provision requires package mapping (router profile) for selected router"
                    .to_string(),
            )
        })?;

        #[cfg(feature = "postgres")]
        let customer_name: String =
            sqlx::query_scalar("SELECT name FROM customers WHERE tenant_id = $1 AND id = $2")
                .bind(tenant_id)
                .bind(&sub.customer_id)
                .fetch_optional(&self.pool)
                .await?
                .unwrap_or_else(|| "customer".to_string());

        #[cfg(feature = "sqlite")]
        let customer_name: String =
            sqlx::query_scalar("SELECT name FROM customers WHERE tenant_id = ? AND id = ?")
                .bind(tenant_id)
                .bind(&sub.customer_id)
                .fetch_optional(&self.pool)
                .await?
                .unwrap_or_else(|| "customer".to_string());

        let username =
            Self::build_auto_pppoe_username(&customer_name, &sub.customer_id, &sub.location_id);

        #[cfg(feature = "postgres")]
        let username_conflict: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1 FROM pppoe_accounts
              WHERE tenant_id = $1
                AND username = $2
                AND (customer_id <> $3 OR location_id <> $4 OR router_id <> $5)
            )
            "#,
        )
        .bind(tenant_id)
        .bind(&username)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(router_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let username_conflict: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1 FROM pppoe_accounts
              WHERE tenant_id = ?
                AND username = ?
                AND (customer_id <> ? OR location_id <> ? OR router_id <> ?)
            )
            "#,
        )
        .bind(tenant_id)
        .bind(&username)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(router_id)
        .fetch_one(&self.pool)
        .await?;

        if username_conflict {
            return Err(AppError::Validation(format!(
                "PPPoE username conflict detected across tenant routers: {}",
                username
            )));
        }

        #[derive(sqlx::FromRow)]
        struct ExistingPppoe {
            id: String,
        }

        #[cfg(feature = "postgres")]
        let existing: Option<ExistingPppoe> = sqlx::query_as(
            r#"
            SELECT id FROM pppoe_accounts
            WHERE tenant_id = $1
              AND customer_id = $2
              AND location_id = $3
              AND router_id = $4
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(router_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let existing: Option<ExistingPppoe> = sqlx::query_as(
            r#"
            SELECT id FROM pppoe_accounts
            WHERE tenant_id = ?
              AND customer_id = ?
              AND location_id = ?
              AND router_id = ?
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(router_id)
        .fetch_optional(&self.pool)
        .await?;

        let now = Utc::now();
        let note = format!(
            "Auto-provisioned from active subscription {}. Pending apply.",
            sub.id
        );

        if let Some(ex) = existing {
            #[cfg(feature = "postgres")]
            sqlx::query(
                r#"
                UPDATE pppoe_accounts
                SET username = $1,
                    package_id = $2,
                    router_profile_name = $3,
                    remote_address = NULL,
                    address_pool = $4,
                    disabled = true,
                    comment = $5,
                    updated_at = $6
                WHERE tenant_id = $7 AND id = $8
                "#,
            )
            .bind(&username)
            .bind(&sub.package_id)
            .bind(&mapping.router_profile_name)
            .bind(&mapping.address_pool)
            .bind(&note)
            .bind(now)
            .bind(tenant_id)
            .bind(&ex.id)
            .execute(&self.pool)
            .await?;

            #[cfg(feature = "sqlite")]
            sqlx::query(
                r#"
                UPDATE pppoe_accounts
                SET username = ?,
                    package_id = ?,
                    router_profile_name = ?,
                    remote_address = NULL,
                    address_pool = ?,
                    disabled = 1,
                    comment = ?,
                    updated_at = ?
                WHERE tenant_id = ? AND id = ?
                "#,
            )
            .bind(&username)
            .bind(&sub.package_id)
            .bind(&mapping.router_profile_name)
            .bind(&mapping.address_pool)
            .bind(&note)
            .bind(now)
            .bind(tenant_id)
            .bind(&ex.id)
            .execute(&self.pool)
            .await?;
        } else {
            let pwd_seed = Uuid::new_v4().simple().to_string();
            let password_raw = format!("Pppoe#{}", &pwd_seed[..10]);
            let password_enc = encrypt_secret_for(PURPOSE_PPPOE, &password_raw)?;
            let id = Uuid::new_v4().to_string();

            #[cfg(feature = "postgres")]
            sqlx::query(
                r#"
                INSERT INTO pppoe_accounts
                  (id, tenant_id, router_id, customer_id, location_id, username, password_enc, package_id, profile_id, router_profile_name,
                   remote_address, address_pool, disabled, comment, router_present, router_secret_id, last_sync_at, last_error, created_at, updated_at)
                VALUES
                  ($1,$2,$3,$4,$5,$6,$7,$8,NULL,$9,NULL,$10,true,$11,false,NULL,NULL,NULL,$12,$13)
                "#,
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(router_id)
            .bind(&sub.customer_id)
            .bind(&sub.location_id)
            .bind(&username)
            .bind(&password_enc)
            .bind(&sub.package_id)
            .bind(&mapping.router_profile_name)
            .bind(&mapping.address_pool)
            .bind(&note)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await?;

            #[cfg(feature = "sqlite")]
            sqlx::query(
                r#"
                INSERT INTO pppoe_accounts
                  (id, tenant_id, router_id, customer_id, location_id, username, password_enc, package_id, profile_id, router_profile_name,
                   remote_address, address_pool, disabled, comment, router_present, router_secret_id, last_sync_at, last_error, created_at, updated_at)
                VALUES
                  (?,?,?,?,?,?,?,?,NULL,?,NULL,?,1,?,0,NULL,NULL,NULL,?,?)
                "#,
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(router_id)
            .bind(&sub.customer_id)
            .bind(&sub.location_id)
            .bind(&username)
            .bind(&password_enc)
            .bind(&sub.package_id)
            .bind(&mapping.router_profile_name)
            .bind(&mapping.address_pool)
            .bind(&note)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await?;
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "PPPOE_AUTO_PROVISION",
                "pppoe",
                Some(&sub.id),
                Some("Auto provisioned PPPoE draft from active subscription"),
                ip_address,
            )
            .await;

        Ok(())
    }

    pub(super) async fn transition_customer_subscription_status(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        event: SubscriptionLifecycleEvent,
    ) -> AppResult<SubscriptionLifecycleStatus> {
        #[cfg(feature = "postgres")]
        let current_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let current_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM customer_subscriptions WHERE tenant_id = ? AND id = ? LIMIT 1",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        let current_raw = current_status
            .ok_or_else(|| AppError::NotFound("Customer subscription not found".to_string()))?;
        let current = SubscriptionLifecycleStatus::parse(&current_raw)
            .map_err(|e| AppError::Validation(e.to_string()))?;
        let target =
            transition_status(current, event).map_err(|e| AppError::Validation(e.to_string()))?;

        if target != current {
            let updated = self
                .set_customer_subscription_status(
                    tenant_id,
                    subscription_id,
                    current.as_str(),
                    target.as_str(),
                )
                .await?;
            if !updated {
                return Err(AppError::Validation(
                    "Subscription status changed concurrently; retry transition".to_string(),
                ));
            }
        }

        Ok(target)
    }
}

/// Count RESTRICT references that block deleting a customer. Returns a
/// human list like `["3 subscriptions", "1 work orders"]`. Wave 21: tanpa
/// ini, hapus pelanggan yang masih punya langganan membalas 500 FK-violation.
pub(crate) async fn customer_delete_blockers(
    pool: &DbPool,
    tenant_id: &str,
    customer_id: &str,
) -> AppResult<Vec<String>> {
    let mut out = Vec::new();
    let checks: [(&str, &str); 5] = [
        (
            "subscriptions",
            "SELECT count(*) FROM customer_subscriptions WHERE tenant_id = $1 AND customer_id = $2",
        ),
        (
            "work orders",
            "SELECT count(*) FROM installation_work_orders WHERE tenant_id = $1 AND customer_id = $2",
        ),
        (
            "pppoe accounts",
            "SELECT count(*) FROM pppoe_accounts WHERE tenant_id = $1 AND customer_id = $2",
        ),
        (
            "dhcp services",
            "SELECT count(*) FROM dhcp_static_services WHERE tenant_id = $1 AND customer_id = $2",
        ),
        (
            "locations",
            "SELECT count(*) FROM customer_locations WHERE tenant_id = $1 AND customer_id = $2",
        ),
    ];
    for (label, sql) in checks {
        let n: i64 = sqlx::query_scalar(sql)
            .bind(tenant_id)
            .bind(customer_id)
            .fetch_one(pool)
            .await?;
        if n > 0 {
            out.push(format!("{n} {label}"));
        }
    }
    Ok(out)
}

/// Same idea for a single location (wave 21): subscriptions/WO/PPPoE/DHCP
/// hold RESTRICT FKs to customer_locations.
pub(crate) async fn location_delete_blockers(
    pool: &DbPool,
    tenant_id: &str,
    location_id: &str,
) -> AppResult<Vec<String>> {
    let mut out = Vec::new();
    let checks: [(&str, &str); 4] = [
        (
            "subscriptions",
            "SELECT count(*) FROM customer_subscriptions WHERE tenant_id = $1 AND location_id = $2",
        ),
        (
            "work orders",
            "SELECT count(*) FROM installation_work_orders WHERE tenant_id = $1 AND location_id = $2",
        ),
        (
            "pppoe accounts",
            "SELECT count(*) FROM pppoe_accounts WHERE tenant_id = $1 AND location_id = $2",
        ),
        (
            "dhcp services",
            "SELECT count(*) FROM dhcp_static_services WHERE tenant_id = $1 AND location_id = $2",
        ),
    ];
    for (label, sql) in checks {
        let n: i64 = sqlx::query_scalar(sql)
            .bind(tenant_id)
            .bind(location_id)
            .fetch_one(pool)
            .await?;
        if n > 0 {
            out.push(format!("{n} {label}"));
        }
    }
    Ok(out)
}
