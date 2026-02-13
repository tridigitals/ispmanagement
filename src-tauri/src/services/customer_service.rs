
use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    AddCustomerPortalUserRequest, CreateCustomerLocationRequest, CreateCustomerPortalUserRequest,
    CreateCustomerRequest, Customer, CustomerLocation, CustomerPortalUser, CustomerUser,
    PaginatedResponse, UpdateCustomerLocationRequest, UpdateCustomerRequest,
};
use crate::services::{AuditService, AuthService, UserService};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct CustomerService {
    pool: DbPool,
    auth_service: AuthService,
    audit_service: AuditService,
    user_service: UserService,
}

impl CustomerService {
    pub fn new(
        pool: DbPool,
        auth_service: AuthService,
        audit_service: AuditService,
        user_service: UserService,
    ) -> Self {
        Self {
            pool,
            auth_service,
            audit_service,
            user_service,
        }
    }

    async fn get_system_role_id_by_name(&self, name: &str) -> AppResult<String> {
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

    async fn ensure_tenant_member_role(
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

    // =========================
    // Admin: Customers
    // =========================

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
            "SELECT * FROM customer_locations WHERE tenant_id = $1 AND customer_id = $2 ORDER BY created_at DESC",
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

        let _ = self.get_customer(actor_id, tenant_id, &dto.customer_id).await?;

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
            "SELECT * FROM customer_locations WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Location not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let mut loc: CustomerLocation = sqlx::query_as(
            "SELECT * FROM customer_locations WHERE tenant_id = ? AND id = ?",
        )
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

        let _ = self.get_customer(actor_id, tenant_id, &dto.customer_id).await?;

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

        let _ = self.get_customer(actor_id, tenant_id, &dto.customer_id).await?;

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
            return Err(AppError::NotFound("Portal user mapping not found".to_string()));
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
    // Portal: Self-service
    // =========================

    pub async fn list_my_locations(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<Vec<CustomerLocation>> {
        // Explicit permission for customer portal.
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

        let customer_id = customer_id.ok_or_else(|| {
            AppError::Forbidden("You are not linked to any customer".to_string())
        })?;

        #[cfg(feature = "postgres")]
        let rows: Vec<CustomerLocation> = sqlx::query_as(
            "SELECT * FROM customer_locations WHERE tenant_id = $1 AND customer_id = $2 ORDER BY created_at DESC",
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
}
