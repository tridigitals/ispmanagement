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
                SELECT pa.address_pool
                FROM pppoe_accounts pa
                WHERE pa.tenant_id = cs.tenant_id
                  AND pa.location_id = cs.location_id
                ORDER BY pa.updated_at DESC, pa.created_at DESC
                LIMIT 1
              ) AS pppoe_address_pool,
              (
                SELECT iprm.isolation_pool
                FROM isp_package_router_mappings iprm
                WHERE iprm.tenant_id = cs.tenant_id
                  AND iprm.package_id = cs.package_id
                  AND iprm.router_id = cs.router_id
                LIMIT 1
              ) AS pppoe_isolation_pool,
              (
                SELECT pa.disabled
                FROM pppoe_accounts pa
                WHERE pa.tenant_id = cs.tenant_id
                  AND pa.location_id = cs.location_id
                ORDER BY pa.updated_at DESC, pa.created_at DESC
                LIMIT 1
              ) AS pppoe_disabled,
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
                SELECT pa.address_pool
                FROM pppoe_accounts pa
                WHERE pa.tenant_id = cs.tenant_id
                  AND pa.location_id = cs.location_id
                ORDER BY pa.updated_at DESC, pa.created_at DESC
                LIMIT 1
              ) AS pppoe_address_pool,
              (
                SELECT iprm.isolation_pool
                FROM isp_package_router_mappings iprm
                WHERE iprm.tenant_id = cs.tenant_id
                  AND iprm.package_id = cs.package_id
                  AND iprm.router_id = cs.router_id
                LIMIT 1
              ) AS pppoe_isolation_pool,
              (
                SELECT pa.disabled
                FROM pppoe_accounts pa
                WHERE pa.tenant_id = cs.tenant_id
                  AND pa.location_id = cs.location_id
                ORDER BY pa.updated_at DESC, pa.created_at DESC
                LIMIT 1
              ) AS pppoe_disabled,
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
        let customer_id = dto.customer_id.ok_or_else(|| {
            AppError::Validation("customer_id is required".to_string())
        })?;
        let status =
            Self::normalize_subscription_status(dto.status.as_deref().unwrap_or("active"))?;
        let starts_at = Self::parse_optional_datetime(dto.starts_at)?;
        let ends_at = Self::parse_optional_datetime(dto.ends_at)?;

        #[cfg(feature = "postgres")]
        let exists_customer: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customers WHERE id = $1 AND tenant_id = $2)",
        )
        .bind(&customer_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let exists_customer: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customers WHERE id = ? AND tenant_id = ?)",
        )
        .bind(&customer_id)
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
        .bind(&customer_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let exists_location: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customer_locations WHERE id = ? AND tenant_id = ? AND customer_id = ?)",
        )
        .bind(&dto.location_id)
        .bind(tenant_id)
        .bind(&customer_id)
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
        .bind(&customer_id)
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
        .bind(&customer_id)
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

    pub async fn create_backoffice_installation_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateBackofficeInstallationOrderRequest,
        ip_address: Option<&str>,
    ) -> AppResult<BackofficeInstallationOrderResponse> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "orders", "create")
            .await?;

        let package_id = dto.package_id.trim().to_string();
        if package_id.is_empty() {
            return Err(AppError::Validation("package_id is required".to_string()));
        }

        let billing_cycle =
            Self::normalize_billing_cycle(dto.billing_cycle.as_deref().unwrap_or("monthly"))?;
        let price = self
            .resolve_active_package_price(tenant_id, &package_id, &billing_cycle)
            .await?;

        let maybe_customer_input = dto.customer.clone();
        let customer = match dto.customer_mode {
            crate::models::BackofficeOrderCustomerMode::Existing => {
                let customer_id = dto
                    .customer_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| AppError::Validation("customer_id is required".to_string()))?;
                self.get_customer_by_id_in_tenant(tenant_id, customer_id)
                    .await?
            }
            crate::models::BackofficeOrderCustomerMode::New => {
                let input = maybe_customer_input.ok_or_else(|| {
                    AppError::Validation("customer payload is required".to_string())
                })?;
                self.build_order_customer(tenant_id, input)?
            }
        };

        let maybe_location_input = dto.location.clone();
        let location = match dto.location_mode {
            crate::models::BackofficeOrderLocationMode::Existing => {
                let location_id = dto
                    .location_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| AppError::Validation("location_id is required".to_string()))?;
                self.get_customer_location_in_tenant(tenant_id, &customer.id, location_id)
                    .await?
            }
            crate::models::BackofficeOrderLocationMode::New => {
                let input = maybe_location_input.ok_or_else(|| {
                    AppError::Validation("location payload is required".to_string())
                })?;
                self.build_order_location(tenant_id, &customer.id, input)?
            }
        };

        let order_notes = dto
            .notes
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        let subscription_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let subscription = CustomerSubscription {
            id: subscription_id.clone(),
            tenant_id: tenant_id.to_string(),
            customer_id: customer.id.clone(),
            location_id: location.id.clone(),
            package_id: package_id.clone(),
            router_id: None,
            billing_cycle: billing_cycle.clone(),
            price,
            currency_code: "IDR".to_string(),
            status: "pending_installation".to_string(),
            starts_at: None,
            ends_at: None,
            grace_started_at: None,
            grace_until: None,
            notes: order_notes.clone(),
            created_at: now,
            updated_at: now,
        };

        let mut tx = self.pool.begin().await?;

        if matches!(
            dto.customer_mode,
            crate::models::BackofficeOrderCustomerMode::New
        ) {
            self.insert_customer_tx(&mut tx, &customer).await?;
        }

        if matches!(
            dto.location_mode,
            crate::models::BackofficeOrderLocationMode::New
        ) {
            self.insert_customer_location_tx(&mut tx, &location).await?;
        }

        self.insert_subscription_tx(&mut tx, &subscription).await?;
        tx.commit().await?;

        let (mut work_order, created_work_order) = self
            .ensure_installation_work_order_for_subscription(tenant_id, &subscription)
            .await?;

        if dto.requested_installation_date.is_some() {
            work_order = self
                .apply_requested_installation_schedule(
                    tenant_id,
                    &work_order.id,
                    dto.requested_installation_date.as_deref(),
                )
                .await?;
        }

        if matches!(
            dto.customer_mode,
            crate::models::BackofficeOrderCustomerMode::New
        ) {
            self.audit_service
                .log(
                    Some(actor_id),
                    Some(tenant_id),
                    "CUSTOMER_CREATE_FROM_ORDER",
                    "customers",
                    Some(&customer.id),
                    Some(&format!(
                        "Created customer {} from order flow",
                        customer.name
                    )),
                    ip_address,
                )
                .await;
        }

        if matches!(
            dto.location_mode,
            crate::models::BackofficeOrderLocationMode::New
        ) {
            self.audit_service
                .log(
                    Some(actor_id),
                    Some(tenant_id),
                    "CUSTOMER_LOCATION_CREATE_FROM_ORDER",
                    "customer_locations",
                    Some(&location.id),
                    Some("Created customer location from order flow"),
                    ip_address,
                )
                .await;
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_SUBSCRIPTION_CREATE_FROM_ORDER",
                "customer_subscriptions",
                Some(&subscription.id),
                Some("Created pending installation subscription from order flow"),
                ip_address,
            )
            .await;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                if created_work_order {
                    "INSTALLATION_WORK_ORDER_CREATE_FROM_ORDER"
                } else {
                    "ORDER_CREATE"
                },
                "installation_work_orders",
                Some(&work_order.id),
                Some("Created or reused installation work order from order flow"),
                ip_address,
            )
            .await;

        Ok(BackofficeInstallationOrderResponse {
            customer,
            location,
            subscription,
            work_order,
        })
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

    fn build_order_customer(
        &self,
        tenant_id: &str,
        input: crate::models::BackofficeOrderCustomerInput,
    ) -> AppResult<Customer> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Validation(
                "customer.name is required".to_string(),
            ));
        }

        let email = input
            .email
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let phone = input
            .phone
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        if email.is_none() && phone.is_none() {
            return Err(AppError::Validation(
                "customer email or phone is required".to_string(),
            ));
        }

        Ok(Customer::new(
            tenant_id.to_string(),
            name,
            email,
            phone,
            input.notes,
            input.is_active,
            None,
        ))
    }

    fn build_order_location(
        &self,
        tenant_id: &str,
        customer_id: &str,
        input: crate::models::BackofficeOrderLocationInput,
    ) -> AppResult<CustomerLocation> {
        let label = input.label.trim().to_string();
        if label.is_empty() {
            return Err(AppError::Validation(
                "location.label is required".to_string(),
            ));
        }

        let address_line1 = input
            .address_line1
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if address_line1.is_none() {
            return Err(AppError::Validation(
                "location.address_line1 is required".to_string(),
            ));
        }

        Ok(CustomerLocation::new(
            tenant_id.to_string(),
            customer_id.to_string(),
            label,
            address_line1,
            input.address_line2,
            input.city,
            input.state,
            input.postal_code,
            input.country,
            input.latitude,
            input.longitude,
            input.notes,
        ))
    }

    async fn get_customer_by_id_in_tenant(
        &self,
        tenant_id: &str,
        customer_id: &str,
    ) -> AppResult<Customer> {
        #[cfg(feature = "postgres")]
        let customer: Option<Customer> =
            sqlx::query_as("SELECT * FROM customers WHERE tenant_id = $1 AND id = $2 LIMIT 1")
                .bind(tenant_id)
                .bind(customer_id)
                .fetch_optional(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let customer: Option<Customer> =
            sqlx::query_as("SELECT * FROM customers WHERE tenant_id = ? AND id = ? LIMIT 1")
                .bind(tenant_id)
                .bind(customer_id)
                .fetch_optional(&self.pool)
                .await?;

        customer.ok_or_else(|| AppError::NotFound("Customer not found".to_string()))
    }

    async fn get_customer_location_in_tenant(
        &self,
        tenant_id: &str,
        customer_id: &str,
        location_id: &str,
    ) -> AppResult<CustomerLocation> {
        #[cfg(feature = "postgres")]
        let location: Option<CustomerLocation> = sqlx::query_as(
            "SELECT * FROM customer_locations WHERE tenant_id = $1 AND customer_id = $2 AND id = $3 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let location: Option<CustomerLocation> = sqlx::query_as(
            "SELECT * FROM customer_locations WHERE tenant_id = ? AND customer_id = ? AND id = ? LIMIT 1",
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await?;

        location.ok_or_else(|| {
            AppError::Validation("Location does not belong to this customer".to_string())
        })
    }

    async fn resolve_active_package_price(
        &self,
        tenant_id: &str,
        package_id: &str,
        billing_cycle: &str,
    ) -> AppResult<f64> {
        #[cfg(feature = "postgres")]
        let pkg_row: Option<(f64, f64)> = sqlx::query_as(
            "SELECT price_monthly::float8, price_yearly::float8 FROM isp_packages WHERE tenant_id = $1 AND id = $2 AND is_active = true LIMIT 1",
        )
        .bind(tenant_id)
        .bind(package_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let pkg_row: Option<(f64, f64)> = sqlx::query_as(
            "SELECT price_monthly AS price_monthly, price_yearly AS price_yearly FROM isp_packages WHERE tenant_id = ? AND id = ? AND is_active = 1 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(package_id)
        .fetch_optional(&self.pool)
        .await?;

        let (price_monthly, price_yearly) =
            pkg_row.ok_or_else(|| AppError::Validation("Package not found".to_string()))?;

        if billing_cycle == "yearly" {
            if price_yearly <= 0.0 {
                return Err(AppError::Validation(
                    "Yearly billing is not available for this package".to_string(),
                ));
            }
            return Ok(price_yearly);
        }

        if price_monthly <= 0.0 {
            return Err(AppError::Validation(
                "Package monthly price is invalid".to_string(),
            ));
        }

        Ok(price_monthly)
    }

    #[cfg(feature = "postgres")]
    async fn insert_customer_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        customer: &Customer,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO customers
                (id, tenant_id, name, email, phone, notes, is_active, created_at, updated_at)
            VALUES
                ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(&customer.id)
        .bind(&customer.tenant_id)
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.phone)
        .bind(&customer.notes)
        .bind(customer.is_active)
        .bind(customer.created_at)
        .bind(customer.updated_at)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn insert_customer_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        customer: &Customer,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO customers
                (id, tenant_id, name, email, phone, notes, is_active, created_at, updated_at)
            VALUES
                (?,?,?,?,?,?,?,?,?)
            "#,
        )
        .bind(&customer.id)
        .bind(&customer.tenant_id)
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.phone)
        .bind(&customer.notes)
        .bind(customer.is_active)
        .bind(customer.created_at.to_rfc3339())
        .bind(customer.updated_at.to_rfc3339())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[cfg(feature = "postgres")]
    async fn insert_customer_location_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        location: &CustomerLocation,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO customer_locations
                (id, tenant_id, customer_id, label, address_line1, address_line2, city, state, postal_code, country, latitude, longitude, notes, created_at, updated_at)
            VALUES
                ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            "#,
        )
        .bind(&location.id)
        .bind(&location.tenant_id)
        .bind(&location.customer_id)
        .bind(&location.label)
        .bind(&location.address_line1)
        .bind(&location.address_line2)
        .bind(&location.city)
        .bind(&location.state)
        .bind(&location.postal_code)
        .bind(&location.country)
        .bind(location.latitude)
        .bind(location.longitude)
        .bind(&location.notes)
        .bind(location.created_at)
        .bind(location.updated_at)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn insert_customer_location_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        location: &CustomerLocation,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO customer_locations
                (id, tenant_id, customer_id, label, address_line1, address_line2, city, state, postal_code, country, latitude, longitude, notes, created_at, updated_at)
            VALUES
                (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
            "#,
        )
        .bind(&location.id)
        .bind(&location.tenant_id)
        .bind(&location.customer_id)
        .bind(&location.label)
        .bind(&location.address_line1)
        .bind(&location.address_line2)
        .bind(&location.city)
        .bind(&location.state)
        .bind(&location.postal_code)
        .bind(&location.country)
        .bind(location.latitude)
        .bind(location.longitude)
        .bind(&location.notes)
        .bind(location.created_at.to_rfc3339())
        .bind(location.updated_at.to_rfc3339())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[cfg(feature = "postgres")]
    async fn insert_subscription_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        subscription: &CustomerSubscription,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO customer_subscriptions
              (id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price, currency_code, status, starts_at, ends_at, grace_started_at, grace_until, notes, created_at, updated_at)
            VALUES
              ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)
            "#,
        )
        .bind(&subscription.id)
        .bind(&subscription.tenant_id)
        .bind(&subscription.customer_id)
        .bind(&subscription.location_id)
        .bind(&subscription.package_id)
        .bind(&subscription.router_id)
        .bind(&subscription.billing_cycle)
        .bind(subscription.price)
        .bind(&subscription.currency_code)
        .bind(&subscription.status)
        .bind(subscription.starts_at)
        .bind(subscription.ends_at)
        .bind(subscription.grace_started_at)
        .bind(subscription.grace_until)
        .bind(&subscription.notes)
        .bind(subscription.created_at)
        .bind(subscription.updated_at)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn insert_subscription_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        subscription: &CustomerSubscription,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO customer_subscriptions
              (id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price, currency_code, status, starts_at, ends_at, grace_started_at, grace_until, notes, created_at, updated_at)
            VALUES
              (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
            "#,
        )
        .bind(&subscription.id)
        .bind(&subscription.tenant_id)
        .bind(&subscription.customer_id)
        .bind(&subscription.location_id)
        .bind(&subscription.package_id)
        .bind(&subscription.router_id)
        .bind(&subscription.billing_cycle)
        .bind(subscription.price)
        .bind(&subscription.currency_code)
        .bind(&subscription.status)
        .bind(subscription.starts_at)
        .bind(subscription.ends_at)
        .bind(subscription.grace_started_at)
        .bind(subscription.grace_until)
        .bind(&subscription.notes)
        .bind(subscription.created_at.to_rfc3339())
        .bind(subscription.updated_at.to_rfc3339())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn apply_requested_installation_schedule(
        &self,
        tenant_id: &str,
        work_order_id: &str,
        requested_installation_date: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        let scheduled_at =
            Self::parse_optional_datetime(requested_installation_date.map(str::to_string))?;

        #[cfg(feature = "postgres")]
        sqlx::query(
            "UPDATE installation_work_orders SET scheduled_at = $1, updated_at = $2 WHERE tenant_id = $3 AND id = $4",
        )
        .bind(scheduled_at)
        .bind(Utc::now())
        .bind(tenant_id)
        .bind(work_order_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            "UPDATE installation_work_orders SET scheduled_at = ?, updated_at = ? WHERE tenant_id = ? AND id = ?",
        )
        .bind(scheduled_at)
        .bind(Utc::now())
        .bind(tenant_id)
        .bind(work_order_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let row: InstallationWorkOrder = sqlx::query_as(
            "SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at FROM installation_work_orders WHERE tenant_id = $1 AND id = $2 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: InstallationWorkOrder = sqlx::query_as(
            "SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at FROM installation_work_orders WHERE tenant_id = ? AND id = ? LIMIT 1",
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_one(&self.pool)
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
