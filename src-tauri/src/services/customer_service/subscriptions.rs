use super::*;

impl CustomerService {
    pub async fn get_customer_subscription(
        &self,
        actor_id: &str,
        tenant_id: &str,
        subscription_id: &str,
    ) -> AppResult<CustomerSubscriptionView> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "billing", "read")
            .await?;

        #[cfg(feature = "postgres")]
        let row: Option<CustomerSubscriptionView> = sqlx::query_as(
            r#"
            SELECT
              cs.id, cs.tenant_id, cs.customer_id, cs.location_id, cs.package_id, cs.router_id,
              cs.billing_cycle, cs.price::float8 as price, cs.currency_code, cs.status,
              cs.starts_at, cs.ends_at, cs.grace_started_at, cs.grace_until, cs.notes,
              cs.created_at, cs.updated_at,
              p.name AS package_name,
              cl.label AS location_label,
              mr.name AS router_name,
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
              EXISTS (
                SELECT 1
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                  AND iwo.status = 'cancelled'
              ) AS can_request_reopen,
              (
                SELECT wrr.status
                FROM work_order_reschedule_requests wrr
                INNER JOIN installation_work_orders iwo
                  ON iwo.id = wrr.work_order_id
                 AND iwo.tenant_id = wrr.tenant_id
                WHERE wrr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY wrr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_status,
              (
                SELECT wrr.requested_schedule_at::text
                FROM work_order_reschedule_requests wrr
                INNER JOIN installation_work_orders iwo
                  ON iwo.id = wrr.work_order_id
                 AND iwo.tenant_id = wrr.tenant_id
                WHERE wrr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY wrr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p
              ON p.id = cs.package_id
             AND p.tenant_id = cs.tenant_id
            LEFT JOIN customer_locations cl
              ON cl.id = cs.location_id
             AND cl.tenant_id = cs.tenant_id
            LEFT JOIN mikrotik_routers mr
              ON mr.id = cs.router_id
             AND mr.tenant_id = cs.tenant_id
            WHERE cs.tenant_id = $1
              AND cs.id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: Option<CustomerSubscriptionView> = sqlx::query_as(
            r#"
            SELECT
              cs.id, cs.tenant_id, cs.customer_id, cs.location_id, cs.package_id, cs.router_id,
              cs.billing_cycle, cs.price as price, cs.currency_code, cs.status,
              cs.starts_at, cs.ends_at, cs.grace_started_at, cs.grace_until, cs.notes,
              cs.created_at, cs.updated_at,
              p.name AS package_name,
              cl.label AS location_label,
              mr.name AS router_name,
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
              EXISTS (
                SELECT 1
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                  AND iwo.status = 'cancelled'
              ) AS can_request_reopen,
              (
                SELECT wrr.status
                FROM work_order_reschedule_requests wrr
                INNER JOIN installation_work_orders iwo
                  ON iwo.id = wrr.work_order_id
                 AND iwo.tenant_id = wrr.tenant_id
                WHERE wrr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY wrr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_status,
              (
                SELECT wrr.requested_schedule_at
                FROM work_order_reschedule_requests wrr
                INNER JOIN installation_work_orders iwo
                  ON iwo.id = wrr.work_order_id
                 AND iwo.tenant_id = wrr.tenant_id
                WHERE wrr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY wrr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p
              ON p.id = cs.package_id
             AND p.tenant_id = cs.tenant_id
            LEFT JOIN customer_locations cl
              ON cl.id = cs.location_id
             AND cl.tenant_id = cs.tenant_id
            LEFT JOIN mikrotik_routers mr
              ON mr.id = cs.router_id
             AND mr.tenant_id = cs.tenant_id
            WHERE cs.tenant_id = ?
              AND cs.id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        row.ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))
    }

    pub async fn create_customer_subscription(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateCustomerSubscriptionRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerSubscription> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        if dto.price <= 0.0 {
            return Err(AppError::Validation(
                "price must be greater than 0".to_string(),
            ));
        }

        let billing_cycle = Self::normalize_billing_cycle(&dto.billing_cycle)?;
        let status =
            Self::normalize_subscription_status(dto.status.as_deref().unwrap_or("active"))?;
        let starts_at = Self::parse_optional_datetime(dto.starts_at)?;
        let ends_at = Self::parse_optional_datetime(dto.ends_at)?;

        #[cfg(feature = "postgres")]
        let exists_customer: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customers WHERE id = $1 AND tenant_id = $2)",
        )
        .bind(&dto.customer_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let exists_customer: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customers WHERE id = ? AND tenant_id = ?)",
        )
        .bind(&dto.customer_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        if !exists_customer {
            return Err(AppError::NotFound("Customer not found".to_string()));
        }

        #[cfg(feature = "postgres")]
        let exists_location: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customer_locations WHERE id = $1 AND tenant_id = $2 AND customer_id = $3)",
        )
        .bind(&dto.location_id)
        .bind(tenant_id)
        .bind(&dto.customer_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let exists_location: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customer_locations WHERE id = ? AND tenant_id = ? AND customer_id = ?)",
        )
        .bind(&dto.location_id)
        .bind(tenant_id)
        .bind(&dto.customer_id)
        .fetch_one(&self.pool)
        .await?;

        if !exists_location {
            return Err(AppError::Validation(
                "Location does not belong to this customer".to_string(),
            ));
        }

        #[cfg(feature = "postgres")]
        let exists_package: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM isp_packages WHERE id = $1 AND tenant_id = $2)",
        )
        .bind(&dto.package_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let exists_package: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM isp_packages WHERE id = ? AND tenant_id = ?)",
        )
        .bind(&dto.package_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        if !exists_package {
            return Err(AppError::Validation("Package not found".to_string()));
        }

        if let Some(router_id) = dto.router_id.as_deref() {
            #[cfg(feature = "postgres")]
            let exists_router: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2)",
            )
            .bind(router_id)
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;

            #[cfg(feature = "sqlite")]
            let exists_router: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM mikrotik_routers WHERE id = ? AND tenant_id = ?)",
            )
            .bind(router_id)
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;

            if !exists_router {
                return Err(AppError::Validation("Router not found".to_string()));
            }
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let currency = dto
            .currency_code
            .unwrap_or_else(|| "IDR".to_string())
            .trim()
            .to_uppercase();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customer_subscriptions
              (id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price, currency_code, status, starts_at, ends_at, grace_started_at, grace_until, notes, created_at, updated_at)
            VALUES
              ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,NULL,NULL,$13,$14,$15)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(&dto.customer_id)
        .bind(&dto.location_id)
        .bind(&dto.package_id)
        .bind(&dto.router_id)
        .bind(&billing_cycle)
        .bind(dto.price)
        .bind(&currency)
        .bind(&status)
        .bind(starts_at)
        .bind(ends_at)
        .bind(dto.notes.as_deref().map(str::trim).filter(|s| !s.is_empty()))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO customer_subscriptions
              (id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price, currency_code, status, starts_at, ends_at, grace_started_at, grace_until, notes, created_at, updated_at)
            VALUES
              (?,?,?,?,?,?,?,?,?,?,?,?,NULL,NULL,?,?,?)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(&dto.customer_id)
        .bind(&dto.location_id)
        .bind(&dto.package_id)
        .bind(&dto.router_id)
        .bind(&billing_cycle)
        .bind(dto.price)
        .bind(&currency)
        .bind(&status)
        .bind(starts_at)
        .bind(ends_at)
        .bind(dto.notes.as_deref().map(str::trim).filter(|s| !s.is_empty()))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let row: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price::float8 as price, currency_code, status, starts_at, ends_at, grace_started_at, grace_until, notes, created_at, updated_at FROM customer_subscriptions WHERE id = $1 AND tenant_id = $2",
        )
        .bind(&id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price as price, currency_code, status, starts_at, ends_at, grace_started_at, grace_until, notes, created_at, updated_at FROM customer_subscriptions WHERE id = ? AND tenant_id = ?",
        )
        .bind(&id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_SUBSCRIPTION_CREATE",
                "customer_subscriptions",
                Some(&id),
                Some("Created customer subscription"),
                ip_address,
            )
            .await;

        // For portal self-checkout, PPPoE provisioning is deferred until
        // installation work order is completed by technician.

        Ok(row)
    }

    pub async fn update_customer_subscription(
        &self,
        actor_id: &str,
        tenant_id: &str,
        subscription_id: &str,
        dto: UpdateCustomerSubscriptionRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerSubscription> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        #[cfg(feature = "postgres")]
        let mut row: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price::float8 as price, currency_code, status, starts_at, ends_at, grace_started_at, grace_until, notes, created_at, updated_at FROM customer_subscriptions WHERE id = $1 AND tenant_id = $2",
        )
        .bind(subscription_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let mut row: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price as price, currency_code, status, starts_at, ends_at, grace_started_at, grace_until, notes, created_at, updated_at FROM customer_subscriptions WHERE id = ? AND tenant_id = ?",
        )
        .bind(subscription_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        if let Some(price) = dto.price {
            if price <= 0.0 {
                return Err(AppError::Validation(
                    "price must be greater than 0".to_string(),
                ));
            }
            row.price = price;
        }
        if let Some(v) = dto.billing_cycle {
            row.billing_cycle = Self::normalize_billing_cycle(&v)?;
        }
        if let Some(v) = dto.status {
            row.status = Self::normalize_subscription_status(&v)?;
        }
        if let Some(v) = dto.currency_code {
            let x = v.trim().to_uppercase();
            if !x.is_empty() {
                row.currency_code = x;
            }
        }
        if let Some(v) = dto.location_id {
            row.location_id = v;
        }
        if let Some(v) = dto.package_id {
            row.package_id = v;
        }
        if dto.router_id.is_some() {
            row.router_id = dto.router_id;
        }
        if dto.starts_at.is_some() {
            row.starts_at = Self::parse_optional_datetime(dto.starts_at)?;
        }
        if dto.ends_at.is_some() {
            row.ends_at = Self::parse_optional_datetime(dto.ends_at)?;
        }
        if let Some(v) = dto.notes {
            let x = v.trim().to_string();
            row.notes = if x.is_empty() { None } else { Some(x) };
        }
        row.updated_at = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE customer_subscriptions
            SET
              location_id = $1,
              package_id = $2,
              router_id = $3,
              billing_cycle = $4,
              price = $5,
              currency_code = $6,
              status = $7,
              starts_at = $8,
              ends_at = $9,
              grace_started_at = $10,
              grace_until = $11,
              notes = $12,
              updated_at = $13
            WHERE id = $14 AND tenant_id = $15
            "#,
        )
        .bind(&row.location_id)
        .bind(&row.package_id)
        .bind(&row.router_id)
        .bind(&row.billing_cycle)
        .bind(row.price)
        .bind(&row.currency_code)
        .bind(&row.status)
        .bind(row.starts_at)
        .bind(row.ends_at)
        .bind(row.grace_started_at)
        .bind(row.grace_until)
        .bind(&row.notes)
        .bind(row.updated_at)
        .bind(subscription_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE customer_subscriptions
            SET
              location_id = ?,
              package_id = ?,
              router_id = ?,
              billing_cycle = ?,
              price = ?,
              currency_code = ?,
              status = ?,
              starts_at = ?,
              ends_at = ?,
              grace_started_at = ?,
              grace_until = ?,
              notes = ?,
              updated_at = ?
            WHERE id = ? AND tenant_id = ?
            "#,
        )
        .bind(&row.location_id)
        .bind(&row.package_id)
        .bind(&row.router_id)
        .bind(&row.billing_cycle)
        .bind(row.price)
        .bind(&row.currency_code)
        .bind(&row.status)
        .bind(row.starts_at)
        .bind(row.ends_at)
        .bind(row.grace_started_at)
        .bind(row.grace_until)
        .bind(&row.notes)
        .bind(row.updated_at)
        .bind(subscription_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_SUBSCRIPTION_UPDATE",
                "customer_subscriptions",
                Some(subscription_id),
                Some("Updated customer subscription"),
                ip_address,
            )
            .await;

        let should_disable_pppoe =
            should_disable_pppoe_for_subscription_status(row.status.as_str());
        let _ = self
            .set_location_pppoe_disabled_state(tenant_id, &row.location_id, should_disable_pppoe)
            .await;

        self.auto_provision_pppoe_for_subscription(actor_id, tenant_id, &row, ip_address)
            .await?;

        Ok(row)
    }

    pub async fn delete_customer_subscription(
        &self,
        actor_id: &str,
        tenant_id: &str,
        subscription_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        #[cfg(feature = "postgres")]
        let res =
            sqlx::query("DELETE FROM customer_subscriptions WHERE id = $1 AND tenant_id = $2")
                .bind(subscription_id)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let res = sqlx::query("DELETE FROM customer_subscriptions WHERE id = ? AND tenant_id = ?")
            .bind(subscription_id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Subscription not found".to_string()));
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_SUBSCRIPTION_DELETE",
                "customer_subscriptions",
                Some(subscription_id),
                Some("Deleted customer subscription"),
                ip_address,
            )
            .await;

        Ok(())
    }
}
