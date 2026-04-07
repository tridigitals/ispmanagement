use super::*;

impl CustomerService {
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
        page: u32,
        per_page: u32,
    ) -> AppResult<PaginatedResponse<Customer>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read")
            .await?;

        let q = q.unwrap_or_default().trim().to_string();
        let offset = (page.saturating_sub(1)) * per_page;

        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT
                c.*,
                COUNT(*) OVER() AS total_count
            FROM customers c
            WHERE c.tenant_id = $1
              AND ($2 = '' OR c.name ILIKE '%' || $2 || '%' OR c.email ILIKE '%' || $2 || '%')
            ORDER BY c.created_at DESC
            LIMIT $3 OFFSET $4
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT
                c.*,
                (SELECT COUNT(*) FROM customers cc WHERE cc.tenant_id = ? AND (? = '' OR cc.name LIKE '%' || ? || '%' OR cc.email LIKE '%' || ? || '%')) AS total_count
            FROM customers c
            WHERE c.tenant_id = ?
              AND (? = '' OR c.name LIKE '%' || ? || '%' OR c.email LIKE '%' || ? || '%')
            ORDER BY c.created_at DESC
            LIMIT ? OFFSET ?
        "#;

        #[derive(sqlx::FromRow)]
        struct Row {
            #[sqlx(flatten)]
            customer: Customer,
            total_count: i64,
        }

        #[cfg(feature = "postgres")]
        let rows: Vec<Row> = sqlx::query_as(query)
            .bind(tenant_id)
            .bind(&q)
            .bind(per_page as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<Row> = sqlx::query_as(query)
            .bind(tenant_id)
            .bind(&q)
            .bind(&q)
            .bind(&q)
            .bind(tenant_id)
            .bind(&q)
            .bind(&q)
            .bind(&q)
            .bind(per_page as i64)
            .bind(offset as i64)
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

    pub async fn get_customer(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
    ) -> AppResult<Customer> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read")
            .await?;

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

        let customer = Customer::new(
            tenant_id.to_string(),
            dto.name,
            dto.email,
            dto.phone,
            dto.notes,
            dto.is_active,
        );

        #[cfg(feature = "postgres")]
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
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
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

        let customer = Customer::new(
            tenant_id.to_string(),
            dto.name,
            dto.email,
            dto.phone,
            dto.notes,
            dto.is_active,
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
        ip_address: Option<&str>,
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

        if let Some(existing) = existing_customer {
            return Ok(existing);
        }

        let customer = Customer::new(
            tenant_id.to_string(),
            name,
            Some(email),
            None,
            None,
            Some(true),
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
        let invite_id = Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customer_registration_invites
                (id, tenant_id, token_hash, created_by, max_uses, used_count, expires_at, is_revoked, revoked_at, last_used_at, note, created_at)
            VALUES
                ($1,$2,$3,$4,$5,0,$6,false,NULL,NULL,$7,$8)
            "#,
        )
        .bind(&invite_id)
        .bind(tenant_id)
        .bind(&token_hash)
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
                (id, tenant_id, token_hash, created_by, max_uses, used_count, expires_at, is_revoked, revoked_at, last_used_at, note, created_at)
            VALUES
                (?,?,?,?,?,0,?,0,NULL,NULL,?,?)
            "#,
        )
        .bind(&invite_id)
        .bind(tenant_id)
        .bind(&token_hash)
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
        };
        let invite_url = format!("https://{domain}/register?invite={invite_token}");

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
        if let Some(email) = dto.email {
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

        #[cfg(feature = "postgres")]
        let res = sqlx::query("DELETE FROM customers WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(customer_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let res = sqlx::query("DELETE FROM customers WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(customer_id)
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Customer not found".to_string()));
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_DELETE",
                "customers",
                Some(customer_id),
                Some("Deleted customer"),
                ip_address,
            )
            .await;

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
        self.auth_service
            .check_permission(actor_id, tenant_id, "customer_locations", "read")
            .await?;

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

    // =========================
    // Admin: Customer Subscriptions
    // =========================
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

        let offset = (page.saturating_sub(1)) * per_page;

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
            WHERE cs.tenant_id = $1 AND cs.customer_id = $2
            ORDER BY cs.updated_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<CustomerSubscriptionView> = sqlx::query_as(
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
            WHERE cs.tenant_id = ? AND cs.customer_id = ?
            ORDER BY cs.updated_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
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
