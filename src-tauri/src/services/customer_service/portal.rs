use super::*;

impl CustomerService {

    pub async fn list_my_subscriptions(
        &self,
        actor_id: &str,
        tenant_id: &str,
        page: u32,
        per_page: u32,
        status: Option<String>,
        sort_by: Option<String>,
        sort_dir: Option<String>,
    ) -> AppResult<PaginatedResponse<CustomerSubscriptionView>> {
        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;
        let offset = (page.saturating_sub(1)) * per_page;
        let status_filter = status
            .map(|v| v.trim().to_lowercase())
            .filter(|v| !v.is_empty());
        let sort_column = match sort_by
            .unwrap_or_else(|| "updated_at".to_string())
            .trim()
            .to_lowercase()
            .as_str()
        {
            "price" => "cs.price",
            "status" => "LOWER(cs.status)",
            "package_name" => "LOWER(COALESCE(p.name, ''))",
            "location_label" => "LOWER(COALESCE(l.label, ''))",
            "updated_at" => "cs.updated_at",
            _ => "cs.updated_at",
        };
        let sort_direction = match sort_dir
            .unwrap_or_else(|| "desc".to_string())
            .trim()
            .to_lowercase()
            .as_str()
        {
            "asc" => "ASC",
            _ => "DESC",
        };

        #[cfg(feature = "postgres")]
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = $1
              AND cs.customer_id = $2
              AND (
                    $3::text IS NULL
                    OR LOWER(cs.status) = $3
                    OR (
                      $3 = 'needs_attention'
                      AND (
                        LOWER(cs.status) IN ('suspended', 'cancelled')
                        OR COALESCE((
                          SELECT LOWER(iwo.status)
                          FROM installation_work_orders iwo
                          WHERE iwo.tenant_id = cs.tenant_id
                            AND iwo.subscription_id = cs.id
                          ORDER BY iwo.created_at DESC
                          LIMIT 1
                        ), '') = 'cancelled'
                      )
                    )
              )
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(&status_filter)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = ?
              AND cs.customer_id = ?
              AND (
                    ? IS NULL
                    OR LOWER(cs.status) = ?
                    OR (
                      ? = 'needs_attention'
                      AND (
                        LOWER(cs.status) IN ('suspended', 'cancelled')
                        OR COALESCE((
                          SELECT LOWER(iwo.status)
                          FROM installation_work_orders iwo
                          WHERE iwo.tenant_id = cs.tenant_id
                            AND iwo.subscription_id = cs.id
                          ORDER BY iwo.created_at DESC
                          LIMIT 1
                        ), '') = 'cancelled'
                      )
                    )
              )
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(status_filter.clone())
        .bind(status_filter.clone())
        .bind(status_filter.clone())
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<CustomerSubscriptionView> = sqlx::query_as(&format!(
            r#"
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
              cs.notes,
              cs.created_at,
              cs.updated_at,
              p.name AS package_name,
              l.label AS location_label,
              r.name AS router_name,
              (
                SELECT iwo.id
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_id,
              (
                SELECT iwo.status
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_status,
              CASE
                WHEN LOWER(cs.status) = 'cancelled' THEN true
                WHEN COALESCE((
                  SELECT LOWER(iwo.status)
                  FROM installation_work_orders iwo
                  WHERE iwo.tenant_id = cs.tenant_id
                    AND iwo.subscription_id = cs.id
                  ORDER BY iwo.created_at DESC
                  LIMIT 1
                ), '') = 'cancelled' THEN true
                ELSE false
              END AS can_request_reopen,
              (
                SELECT worr.status
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_status,
              (
                SELECT CAST(worr.requested_schedule_at AS TEXT)
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p ON p.id = cs.package_id
            LEFT JOIN customer_locations l ON l.id = cs.location_id
            LEFT JOIN mikrotik_routers r ON r.id = cs.router_id
            WHERE cs.tenant_id = $1
              AND cs.customer_id = $2
              AND (
                    $3::text IS NULL
                    OR LOWER(cs.status) = $3
                    OR (
                      $3 = 'needs_attention'
                      AND (
                        LOWER(cs.status) IN ('suspended', 'cancelled')
                        OR COALESCE((
                          SELECT LOWER(iwo.status)
                          FROM installation_work_orders iwo
                          WHERE iwo.tenant_id = cs.tenant_id
                            AND iwo.subscription_id = cs.id
                          ORDER BY iwo.created_at DESC
                          LIMIT 1
                        ), '') = 'cancelled'
                      )
                    )
              )
            ORDER BY {sort_column} {sort_direction}
            LIMIT $4 OFFSET $5
            "#,
        ))
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(&status_filter)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<CustomerSubscriptionView> = sqlx::query_as(&format!(
            r#"
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
              (
                SELECT iwo.id
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_id,
              (
                SELECT iwo.status
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_status,
              CASE
                WHEN LOWER(cs.status) = 'cancelled' THEN 1
                WHEN COALESCE((
                  SELECT LOWER(iwo.status)
                  FROM installation_work_orders iwo
                  WHERE iwo.tenant_id = cs.tenant_id
                    AND iwo.subscription_id = cs.id
                  ORDER BY iwo.created_at DESC
                  LIMIT 1
                ), '') = 'cancelled' THEN 1
                ELSE 0
              END AS can_request_reopen,
              (
                SELECT worr.status
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_status,
              (
                SELECT CAST(worr.requested_schedule_at AS TEXT)
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p ON p.id = cs.package_id
            LEFT JOIN customer_locations l ON l.id = cs.location_id
            LEFT JOIN mikrotik_routers r ON r.id = cs.router_id
            WHERE cs.tenant_id = ?
              AND cs.customer_id = ?
              AND (
                    ? IS NULL
                    OR LOWER(cs.status) = ?
                    OR (
                      ? = 'needs_attention'
                      AND (
                        LOWER(cs.status) IN ('suspended', 'cancelled')
                        OR COALESCE((
                          SELECT LOWER(iwo.status)
                          FROM installation_work_orders iwo
                          WHERE iwo.tenant_id = cs.tenant_id
                            AND iwo.subscription_id = cs.id
                          ORDER BY iwo.created_at DESC
                          LIMIT 1
                        ), '') = 'cancelled'
                      )
                    )
              )
            ORDER BY {sort_column} {sort_direction}
            LIMIT ? OFFSET ?
            "#,
        ))
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(status_filter.clone())
        .bind(status_filter.clone())
        .bind(status_filter.clone())
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(PaginatedResponse {
            data: rows,
            total,
            page,
            per_page,
        })
    }

    pub async fn get_my_subscription_stats(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<CustomerPortalSubscriptionStats> {
        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let stats: CustomerPortalSubscriptionStats = sqlx::query_as(
            r#"
            SELECT
              COUNT(*)::bigint AS total,
              COUNT(*) FILTER (WHERE LOWER(cs.status) = 'active')::bigint AS active,
              COUNT(*) FILTER (
                WHERE LOWER(cs.status) = 'pending_installation'
                  AND COALESCE((
                    SELECT LOWER(iwo.status)
                    FROM installation_work_orders iwo
                    WHERE iwo.tenant_id = cs.tenant_id
                      AND iwo.subscription_id = cs.id
                    ORDER BY iwo.created_at DESC
                    LIMIT 1
                  ), '') <> 'cancelled'
              )::bigint AS pending_installation,
              COUNT(*) FILTER (
                WHERE LOWER(cs.status) IN ('suspended', 'cancelled')
                   OR COALESCE((
                    SELECT LOWER(iwo.status)
                    FROM installation_work_orders iwo
                    WHERE iwo.tenant_id = cs.tenant_id
                      AND iwo.subscription_id = cs.id
                    ORDER BY iwo.created_at DESC
                    LIMIT 1
                  ), '') = 'cancelled'
              )::bigint AS needs_attention
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = $1
              AND cs.customer_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let stats: CustomerPortalSubscriptionStats = sqlx::query_as(
            r#"
            SELECT
              COUNT(*) AS total,
              COALESCE(SUM(CASE WHEN LOWER(cs.status) = 'active' THEN 1 ELSE 0 END), 0) AS active,
              COALESCE(SUM(
                CASE
                  WHEN LOWER(cs.status) = 'pending_installation'
                   AND COALESCE((
                    SELECT LOWER(iwo.status)
                    FROM installation_work_orders iwo
                    WHERE iwo.tenant_id = cs.tenant_id
                      AND iwo.subscription_id = cs.id
                    ORDER BY iwo.created_at DESC
                    LIMIT 1
                   ), '') <> 'cancelled'
                  THEN 1 ELSE 0
                END
              ), 0) AS pending_installation,
              COALESCE(SUM(
                CASE
                  WHEN LOWER(cs.status) IN ('suspended', 'cancelled')
                    OR COALESCE((
                      SELECT LOWER(iwo.status)
                      FROM installation_work_orders iwo
                      WHERE iwo.tenant_id = cs.tenant_id
                        AND iwo.subscription_id = cs.id
                      ORDER BY iwo.created_at DESC
                      LIMIT 1
                    ), '') = 'cancelled'
                  THEN 1 ELSE 0
                END
              ), 0) AS needs_attention
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = ?
              AND cs.customer_id = ?
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }

    pub async fn create_my_subscription(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: PortalCheckoutSubscriptionRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerSubscription> {
        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        let location_id = dto.location_id.trim().to_string();
        if location_id.is_empty() {
            return Err(AppError::Validation("location_id is required".to_string()));
        }

        let package_id = dto.package_id.trim().to_string();
        if package_id.is_empty() {
            return Err(AppError::Validation("package_id is required".to_string()));
        }

        let billing_cycle = Self::normalize_billing_cycle(&dto.billing_cycle)?;
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let location_ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customer_locations WHERE tenant_id = $1 AND id = $2 AND customer_id = $3)",
        )
        .bind(tenant_id)
        .bind(&location_id)
        .bind(&customer_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let location_ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customer_locations WHERE tenant_id = ? AND id = ? AND customer_id = ?)",
        )
        .bind(tenant_id)
        .bind(&location_id)
        .bind(&customer_id)
        .fetch_one(&self.pool)
        .await?;

        if !location_ok {
            return Err(AppError::Validation(
                "Location does not belong to your customer account".to_string(),
            ));
        }

        #[cfg(feature = "postgres")]
        let pkg_row: Option<(f64, f64)> = sqlx::query_as(
            "SELECT price_monthly::float8, price_yearly::float8 FROM isp_packages WHERE tenant_id = $1 AND id = $2 AND is_active = true LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&package_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let pkg_row: Option<(f64, f64)> = sqlx::query_as(
            "SELECT price_monthly AS price_monthly, price_yearly AS price_yearly FROM isp_packages WHERE tenant_id = ? AND id = ? AND is_active = 1 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&package_id)
        .fetch_optional(&self.pool)
        .await?;

        let (price_monthly, price_yearly) =
            pkg_row.ok_or_else(|| AppError::Validation("Package not found".to_string()))?;

        let price = if billing_cycle == "yearly" {
            if price_yearly <= 0.0 {
                return Err(AppError::Validation(
                    "Yearly billing is not available for this package".to_string(),
                ));
            }
            price_yearly
        } else {
            if price_monthly <= 0.0 {
                return Err(AppError::Validation(
                    "Package monthly price is invalid".to_string(),
                ));
            }
            price_monthly
        };

        // Portal checkout must create a new service order/subscription each time.
        // Renewal is handled by recurring invoice generation, not by overwriting
        // currently active subscription on the same location.
        let subscription_id = Uuid::new_v4().to_string();
        let currency = "IDR".to_string();
        let notes = Some("Self-service checkout".to_string());

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customer_subscriptions
              (id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at)
            VALUES
              ($1,$2,$3,$4,$5,NULL,$6,$7,$8,'pending_installation',NULL,NULL,$9,$10,$11)
            "#,
        )
        .bind(&subscription_id)
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(&location_id)
        .bind(&package_id)
        .bind(&billing_cycle)
        .bind(price)
        .bind(&currency)
        .bind(&notes)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO customer_subscriptions
              (id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at)
            VALUES
              (?,?,?,?,?,NULL,?,?,?,'pending_installation',NULL,NULL,?,?,?)
            "#,
        )
        .bind(&subscription_id)
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(&location_id)
        .bind(&package_id)
        .bind(&billing_cycle)
        .bind(price)
        .bind(&currency)
        .bind(&notes)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let row: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price::float8 as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE id = $1 AND tenant_id = $2",
        )
        .bind(&subscription_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE id = ? AND tenant_id = ?",
        )
        .bind(&subscription_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_SUBSCRIPTION_ORDER_REQUEST",
                "customer_subscriptions",
                Some(&subscription_id),
                Some("Customer portal created a subscription order request"),
                ip_address,
            )
            .await;

        Ok(row)
    }

    pub async fn create_my_subscription_order_request(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: PortalCheckoutSubscriptionRequest,
        ip_address: Option<&str>,
    ) -> AppResult<(CustomerSubscription, InstallationWorkOrder)> {
        let subscription = self
            .create_my_subscription(actor_id, tenant_id, dto, ip_address)
            .await?;

        let (work_order, _created) = self
            .ensure_installation_work_order_for_subscription(tenant_id, &subscription)
            .await?;

        if let Err(err) = self
            .notify_new_installation_request(tenant_id, &subscription, &work_order)
            .await
        {
            warn!(
                "failed to send new installation request notification: tenant_id={}, subscription_id={}, work_order_id={}, error={}",
                tenant_id, subscription.id, work_order.id, err
            );
        }

        Ok((subscription, work_order))
    }

    pub async fn reopen_my_subscription_order_request(
        &self,
        actor_id: &str,
        tenant_id: &str,
        subscription_id: &str,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<(CustomerSubscription, InstallationWorkOrder)> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read_own")
            .await?;

        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let mut sub: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price::float8 as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2 AND customer_id = $3 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(&customer_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let mut sub: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE tenant_id = ? AND id = ? AND customer_id = ? LIMIT 1",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(&customer_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        #[cfg(feature = "postgres")]
        let latest_work_order_status: Option<String> = sqlx::query_scalar(
            r#"
            SELECT status
            FROM installation_work_orders
            WHERE tenant_id = $1
              AND subscription_id = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let latest_work_order_status: Option<String> = sqlx::query_scalar(
            r#"
            SELECT status
            FROM installation_work_orders
            WHERE tenant_id = ?
              AND subscription_id = ?
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        let has_cancelled_state =
            sub.status == "cancelled" || latest_work_order_status.as_deref() == Some("cancelled");
        if !has_cancelled_state {
            return Err(AppError::Validation(
                "Only cancelled subscription/order can be reopened".to_string(),
            ));
        }

        let reopened_status = self
            .transition_customer_subscription_status(
                tenant_id,
                &sub.id,
                SubscriptionLifecycleEvent::Reopen,
            )
            .await?;
        sub.status = reopened_status.as_str().to_string();
        sub.updated_at = Utc::now();

        let (work_order, _created) = self
            .ensure_installation_work_order_for_subscription(tenant_id, &sub)
            .await?;

        let mut note = "Reopened by customer request from portal".to_string();
        if let Some(extra) = notes.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
            note.push_str(". ");
            note.push_str(extra);
        }
        let merged_notes =
            Self::merge_work_order_notes(work_order.notes.clone(), actor_id, Some(&note));
        let now = Utc::now();
        #[cfg(feature = "postgres")]
        let _ = sqlx::query(
            "UPDATE installation_work_orders SET notes = $1, updated_at = $2 WHERE tenant_id = $3 AND id = $4",
        )
        .bind(&merged_notes)
        .bind(now)
        .bind(tenant_id)
        .bind(&work_order.id)
        .execute(&self.pool)
        .await;
        #[cfg(feature = "sqlite")]
        let _ = sqlx::query(
            "UPDATE installation_work_orders SET notes = ?, updated_at = ? WHERE tenant_id = ? AND id = ?",
        )
        .bind(&merged_notes)
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(&work_order.id)
        .execute(&self.pool)
        .await;

        if let Err(err) = self
            .notify_new_installation_request(tenant_id, &sub, &work_order)
            .await
        {
            warn!(
                "failed to notify tenant about customer reopen request: tenant_id={}, subscription_id={}, work_order_id={}, error={}",
                tenant_id, sub.id, work_order.id, err
            );
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_ORDER_REQUEST_REOPEN",
                "customer_subscriptions",
                Some(&sub.id),
                Some("Customer requested installation order reopen"),
                ip_address,
            )
            .await;

        Ok((sub, work_order))
    }

    pub async fn get_my_subscription_installation_tracker(
        &self,
        actor_id: &str,
        tenant_id: &str,
        subscription_id: &str,
    ) -> AppResult<(
        CustomerSubscriptionView,
        Option<InstallationWorkOrderView>,
        Option<WorkOrderRescheduleRequestView>,
    )> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read_own")
            .await?;

        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let subscription: CustomerSubscriptionView = sqlx::query_as(
            r#"
            SELECT
              cs.id, cs.tenant_id, cs.customer_id, cs.location_id, cs.package_id, cs.router_id,
              cs.billing_cycle, cs.price::float8 as price, cs.currency_code, cs.status,
              cs.starts_at, cs.ends_at, cs.notes, cs.created_at, cs.updated_at,
              p.name AS package_name,
              l.label AS location_label,
              r.name AS router_name,
              (
                SELECT iwo.id
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_id,
              (
                SELECT iwo.status
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_status,
              (
                (
                  cs.status = 'cancelled'
                ) OR EXISTS (
                  SELECT 1
                  FROM installation_work_orders iwo
                  WHERE iwo.tenant_id = cs.tenant_id
                    AND iwo.subscription_id = cs.id
                    AND iwo.status = 'cancelled'
                )
              ) AS can_request_reopen,
              (
                SELECT worr.status
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_status,
              (
                SELECT CAST(worr.requested_schedule_at AS TEXT)
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p
              ON p.tenant_id = cs.tenant_id
             AND p.id = cs.package_id
            LEFT JOIN customer_locations l
              ON l.tenant_id = cs.tenant_id
             AND l.id = cs.location_id
            LEFT JOIN mikrotik_routers r
              ON r.tenant_id = cs.tenant_id
             AND r.id = cs.router_id
            WHERE cs.tenant_id = $1
              AND cs.customer_id = $2
              AND cs.id = $3
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let subscription: CustomerSubscriptionView = sqlx::query_as(
            r#"
            SELECT
              cs.id, cs.tenant_id, cs.customer_id, cs.location_id, cs.package_id, cs.router_id,
              cs.billing_cycle, cs.price as price, cs.currency_code, cs.status,
              cs.starts_at, cs.ends_at, cs.notes, cs.created_at, cs.updated_at,
              p.name AS package_name,
              l.label AS location_label,
              r.name AS router_name,
              (
                SELECT iwo.id
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_id,
              (
                SELECT iwo.status
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_status,
              (
                (
                  cs.status = 'cancelled'
                ) OR EXISTS (
                  SELECT 1
                  FROM installation_work_orders iwo
                  WHERE iwo.tenant_id = cs.tenant_id
                    AND iwo.subscription_id = cs.id
                    AND iwo.status = 'cancelled'
                )
              ) AS can_request_reopen,
              (
                SELECT worr.status
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_status,
              (
                SELECT CAST(worr.requested_schedule_at AS TEXT)
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p
              ON p.tenant_id = cs.tenant_id
             AND p.id = cs.package_id
            LEFT JOIN customer_locations l
              ON l.tenant_id = cs.tenant_id
             AND l.id = cs.location_id
            LEFT JOIN mikrotik_routers r
              ON r.tenant_id = cs.tenant_id
             AND r.id = cs.router_id
            WHERE cs.tenant_id = ?
              AND cs.customer_id = ?
              AND cs.id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        #[cfg(feature = "postgres")]
        let work_order: Option<InstallationWorkOrderView> = sqlx::query_as(
            r#"
            SELECT
              wo.id, wo.tenant_id, wo.subscription_id, wo.invoice_id, wo.customer_id, wo.location_id,
              cs.package_id AS package_id,
              COALESCE(wo.router_id, cs.router_id) AS router_id,
              wo.status, wo.assigned_to, wo.scheduled_at, wo.completed_at, wo.notes, wo.created_at, wo.updated_at,
              c.name AS customer_name,
              l.label AS location_label,
              p.name AS package_name,
              r.name AS router_name,
              u.name AS assigned_to_name,
              u.email AS assigned_to_email,
              csa.id AS assignment_id,
              csa.status AS assignment_status,
              cs.status AS subscription_status,
              cs.starts_at AS subscription_starts_at,
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
              AND wo.customer_id = $2
              AND wo.subscription_id = $3
            ORDER BY wo.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let work_order: Option<InstallationWorkOrderView> = sqlx::query_as(
            r#"
            SELECT
              wo.id, wo.tenant_id, wo.subscription_id, wo.invoice_id, wo.customer_id, wo.location_id,
              cs.package_id AS package_id,
              COALESCE(wo.router_id, cs.router_id) AS router_id,
              wo.status, wo.assigned_to, wo.scheduled_at, wo.completed_at, wo.notes, wo.created_at, wo.updated_at,
              c.name AS customer_name,
              l.label AS location_label,
              p.name AS package_name,
              r.name AS router_name,
              u.name AS assigned_to_name,
              u.email AS assigned_to_email,
              csa.id AS assignment_id,
              csa.status AS assignment_status,
              cs.status AS subscription_status,
              cs.starts_at AS subscription_starts_at,
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
              AND wo.customer_id = ?
              AND wo.subscription_id = ?
            ORDER BY wo.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let latest_reschedule_request: Option<WorkOrderRescheduleRequestView> =
            if let Some(ref wo) = work_order {
                sqlx::query_as(
                    r#"
                    SELECT
                      r.id,
                      r.work_order_id,
                      CAST(r.requested_schedule_at AS TEXT) AS requested_schedule_at,
                      r.reason,
                      r.status,
                      req.name AS requested_by_name,
                      req.email AS requested_by_email,
                      rev.name AS reviewed_by_name,
                      CAST(r.reviewed_at AS TEXT) AS reviewed_at,
                      r.review_notes,
                      CAST(r.created_at AS TEXT) AS created_at
                    FROM work_order_reschedule_requests r
                    LEFT JOIN users req ON req.id = r.requested_by
                    LEFT JOIN users rev ON rev.id = r.reviewed_by
                    WHERE r.tenant_id = $1
                      AND r.work_order_id = $2
                    ORDER BY r.created_at DESC
                    LIMIT 1
                    "#,
                )
                .bind(tenant_id)
                .bind(&wo.id)
                .fetch_optional(&self.pool)
                .await?
            } else {
                None
            };

        #[cfg(feature = "sqlite")]
        let latest_reschedule_request: Option<WorkOrderRescheduleRequestView> =
            if let Some(ref wo) = work_order {
                sqlx::query_as(
                    r#"
                    SELECT
                      r.id,
                      r.work_order_id,
                      CAST(r.requested_schedule_at AS TEXT) AS requested_schedule_at,
                      r.reason,
                      r.status,
                      req.name AS requested_by_name,
                      req.email AS requested_by_email,
                      rev.name AS reviewed_by_name,
                      CAST(r.reviewed_at AS TEXT) AS reviewed_at,
                      r.review_notes,
                      CAST(r.created_at AS TEXT) AS created_at
                    FROM work_order_reschedule_requests r
                    LEFT JOIN users req ON req.id = r.requested_by
                    LEFT JOIN users rev ON rev.id = r.reviewed_by
                    WHERE r.tenant_id = ?
                      AND r.work_order_id = ?
                    ORDER BY r.created_at DESC
                    LIMIT 1
                    "#,
                )
                .bind(tenant_id)
                .bind(&wo.id)
                .fetch_optional(&self.pool)
                .await?
            } else {
                None
            };

        Ok((subscription, work_order, latest_reschedule_request))
    }

    pub async fn request_my_subscription_reschedule(
        &self,
        actor_id: &str,
        tenant_id: &str,
        subscription_id: &str,
        requested_at: String,
        reason: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<(CustomerSubscription, InstallationWorkOrder)> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read_own")
            .await?;

        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let sub: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price::float8 as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2 AND customer_id = $3 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(&customer_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let sub: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE tenant_id = ? AND id = ? AND customer_id = ? LIMIT 1",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(&customer_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        let requested_dt = Self::parse_optional_datetime(Some(requested_at.clone()))?
            .ok_or_else(|| AppError::Validation("Requested schedule is required".to_string()))?;
        let now = Utc::now();
        if requested_dt < (now + Duration::hours(2)) {
            return Err(AppError::Validation(
                "Reschedule must be at least 2 hours from now".to_string(),
            ));
        }

        let (current_sub, current_wo_opt, _current_reschedule) = self
            .get_my_subscription_installation_tracker(actor_id, tenant_id, &sub.id)
            .await?;
        let current_wo_view = current_wo_opt.ok_or_else(|| {
            AppError::Validation(
                "No installation work order found for this subscription".to_string(),
            )
        })?;
        if current_wo_view.status != "pending" {
            return Err(AppError::Validation(
                "Reschedule is only allowed before installation starts".to_string(),
            ));
        }
        let current_wo = self
            .get_installation_work_order_row(tenant_id, &current_wo_view.id)
            .await?;

        let reason_txt = reason
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("Customer requested schedule change");
        #[cfg(feature = "postgres")]
        let pending_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM work_order_reschedule_requests
              WHERE tenant_id = $1
                AND work_order_id = $2
                AND status = 'pending'
            )
            "#,
        )
        .bind(tenant_id)
        .bind(&current_wo_view.id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let pending_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM work_order_reschedule_requests
              WHERE tenant_id = ?
                AND work_order_id = ?
                AND status = 'pending'
            )
            "#,
        )
        .bind(tenant_id)
        .bind(&current_wo_view.id)
        .fetch_one(&self.pool)
        .await?;

        if pending_exists {
            return Err(AppError::Validation(
                "There is already a pending reschedule request for this work order".to_string(),
            ));
        }

        let request_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO work_order_reschedule_requests
              (id, tenant_id, work_order_id, subscription_id, requested_by, requested_schedule_at, reason, status, created_at, updated_at)
            VALUES
              ($1, $2, $3, $4, $5, $6, $7, 'pending', $8, $8)
            "#,
        )
        .bind(&request_id)
        .bind(tenant_id)
        .bind(&current_wo_view.id)
        .bind(&sub.id)
        .bind(actor_id)
        .bind(requested_dt)
        .bind(reason_txt)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO work_order_reschedule_requests
              (id, tenant_id, work_order_id, subscription_id, requested_by, requested_schedule_at, reason, status, created_at, updated_at)
            VALUES
              (?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?)
            "#,
        )
        .bind(&request_id)
        .bind(tenant_id)
        .bind(&current_wo_view.id)
        .bind(&sub.id)
        .bind(actor_id)
        .bind(requested_dt.to_rfc3339())
        .bind(reason_txt)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_RESCHEDULE_REQUEST",
                "customer_subscriptions",
                Some(&current_sub.id),
                Some("Customer requested installation reschedule"),
                ip_address,
            )
            .await;

        if let Err(err) = self
            .notify_installation_rescheduled(tenant_id, &sub, &current_wo, reason_txt)
            .await
        {
            warn!(
                "failed to send installation reschedule notification: tenant_id={}, subscription_id={}, work_order_id={}, error={}",
                tenant_id, sub.id, current_wo.id, err
            );
        }

        Ok((sub, current_wo))
    }
}
