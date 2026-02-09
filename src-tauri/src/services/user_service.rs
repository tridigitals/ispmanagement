//! User Service for CRUD operations

use crate::db::connection::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    CreateUserAddressDto, CreateUserDto, UpdateUserAddressDto, UpdateUserDto, User, UserAddress,
    UserResponse,
};
use crate::services::audit_service::AuditService;
use crate::services::auth_service::AuthService;
use chrono::Utc;

/// User service for managing users
#[derive(Clone)]
pub struct UserService {
    pool: DbPool,
    audit_service: AuditService,
}

impl UserService {
    pub fn new(pool: DbPool, audit_service: AuditService) -> Self {
        Self {
            pool,
            audit_service,
        }
    }

    /// List all users with pagination (optimized single query with window function)
    pub async fn list(&self, page: u32, per_page: u32) -> AppResult<(Vec<UserResponse>, i64)> {
        let offset = (page.saturating_sub(1)) * per_page;

        // Optimized query: fetch users with tenant info AND total count in single query
        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT 
                u.*, 
                t.slug as tenant_slug,
                r.name as tenant_role_name,
                COUNT(*) OVER() as total_count
            FROM users u
            LEFT JOIN tenant_members tm ON u.id = tm.user_id
            LEFT JOIN tenants t ON tm.tenant_id = t.id
            LEFT JOIN roles r ON tm.role_id = r.id
            ORDER BY u.created_at DESC 
            LIMIT $1 OFFSET $2
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT 
                u.*, 
                t.slug as tenant_slug,
                r.name as tenant_role_name,
                (SELECT COUNT(*) FROM users) as total_count
            FROM users u
            LEFT JOIN tenant_members tm ON u.id = tm.user_id
            LEFT JOIN tenants t ON tm.tenant_id = t.id
            LEFT JOIN roles r ON tm.role_id = r.id
            ORDER BY u.created_at DESC 
            LIMIT ? OFFSET ?
        "#;

        // Custom struct to handle the projection with count
        #[derive(sqlx::FromRow)]
        struct UserRow {
            #[sqlx(flatten)]
            user: User,
            tenant_slug: Option<String>,
            tenant_role_name: Option<String>,
            total_count: i64,
        }

        let rows: Vec<UserRow> = sqlx::query_as(query)
            .bind(per_page as i32)
            .bind(offset as i32)
            .fetch_all(&self.pool)
            .await?;

        // Extract total count from first row (same for all rows due to window function)
        let total_count = rows.first().map(|r| r.total_count).unwrap_or(0);

        let response = rows
            .into_iter()
            .map(|row| {
                let mut res: UserResponse = row.user.into();
                res.tenant_slug = row.tenant_slug;
                res.tenant_role = row.tenant_role_name;
                res
            })
            .collect();

        Ok((response, total_count))
    }

    /// Get user by ID
    pub async fn get_by_id(&self, id: &str) -> AppResult<UserResponse> {
        let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::UserNotFound)?;

        Ok(user.into())
    }

    /// Create new user
    pub async fn create(
        &self,
        dto: CreateUserDto,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> AppResult<UserResponse> {
        // Check if email already exists
        let existing: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = $1")
            .bind(&dto.email)
            .fetch_optional(&self.pool)
            .await?;

        if existing.is_some() {
            return Err(AppError::UserAlreadyExists);
        }

        let password_hash = AuthService::hash_password(&dto.password)?;
        let user = User::new(dto.email, password_hash, dto.name);

        let query = sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.name)
        .bind(&user.role)
        .bind(user.is_super_admin)
        .bind(user.is_active);

        #[cfg(feature = "postgres")]
        let query = query.bind(user.created_at).bind(user.updated_at);

        #[cfg(not(feature = "postgres"))]
        let query = query
            .bind(user.created_at.to_rfc3339())
            .bind(user.updated_at.to_rfc3339());

        query.execute(&self.pool).await?;

        // Audit Log
        self.audit_service
            .log(
                actor_id,
                None,
                "USER_CREATE",
                "user",
                Some(&user.id),
                Some(&format!("Created user {}", user.email)),
                ip_address,
            )
            .await;

        Ok(user.into())
    }

    /// Update user
    pub async fn update(
        &self,
        id: &str,
        dto: UpdateUserDto,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> AppResult<UserResponse> {
        let mut user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::UserNotFound)?;

        let before_email = user.email.clone();
        let before_name = user.name.clone();
        let before_role = user.role.clone();
        let before_is_super_admin = user.is_super_admin;
        let before_is_active = user.is_active;

        // Update fields if provided
        if let Some(email) = dto.email {
            // Check if email is taken by another user
            let existing: Option<User> =
                sqlx::query_as("SELECT * FROM users WHERE email = $1 AND id != $2")
                    .bind(&email)
                    .bind(id)
                    .fetch_optional(&self.pool)
                    .await?;

            if existing.is_some() {
                return Err(AppError::UserAlreadyExists);
            }
            user.email = email;
        }
        if let Some(name) = dto.name {
            user.name = name;
        }
        if let Some(role) = dto.role {
            user.role = role;
        }
        if let Some(is_super_admin) = dto.is_super_admin {
            user.is_super_admin = is_super_admin;
        }
        if let Some(is_active) = dto.is_active {
            user.is_active = is_active;
        }

        let updated_at = Utc::now();

        let query = sqlx::query(
            r#"
            UPDATE users SET email = $1, name = $2, role = $3, is_super_admin = $4, is_active = $5, updated_at = $6
            WHERE id = $7
            "#,
        )
        .bind(&user.email)
        .bind(&user.name)
        .bind(&user.role)
        .bind(user.is_super_admin)
        .bind(user.is_active);

        #[cfg(feature = "postgres")]
        let query = query.bind(updated_at);

        #[cfg(not(feature = "postgres"))]
        let query = query.bind(updated_at.to_rfc3339());

        query.bind(id).execute(&self.pool).await?;

        user.updated_at = updated_at;

        // Audit Log
        let details = serde_json::json!({
            "message": "Updated user",
            "user_id": id,
            "email_before": before_email,
            "email_after": user.email,
            "name_before": before_name,
            "name_after": user.name,
            "role_before": before_role,
            "role_after": user.role,
            "is_super_admin_before": before_is_super_admin,
            "is_super_admin_after": user.is_super_admin,
            "is_active_before": before_is_active,
            "is_active_after": user.is_active,
        })
        .to_string();
        self.audit_service
            .log(
                actor_id,
                None,
                "USER_UPDATE",
                "user",
                Some(id),
                Some(details.as_str()),
                ip_address,
            )
            .await;

        Ok(user.into())
    }

    /// Delete user
    pub async fn delete(
        &self,
        id: &str,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::UserNotFound);
        }

        // Audit Log
        self.audit_service
            .log(
                actor_id,
                None,
                "USER_DELETE",
                "user",
                Some(id),
                Some("Deleted user"),
                ip_address,
            )
            .await;

        Ok(())
    }

    /// Count all users
    pub async fn count(&self) -> AppResult<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.0)
    }

    // --- User Addresses (Multi Address Support) ---

    pub async fn list_addresses(&self, user_id: &str) -> AppResult<Vec<UserAddress>> {
        #[cfg(feature = "postgres")]
        let query = "SELECT * FROM user_addresses WHERE user_id = $1 ORDER BY created_at DESC";
        #[cfg(feature = "sqlite")]
        let query = "SELECT * FROM user_addresses WHERE user_id = ? ORDER BY created_at DESC";

        let rows = sqlx::query_as::<_, UserAddress>(query)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_address(
        &self,
        user_id: &str,
        dto: CreateUserAddressDto,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> AppResult<UserAddress> {
        let line1 = dto.line1.trim().to_string();
        if line1.is_empty() {
            return Err(AppError::Validation(
                "Address line1 is required".to_string(),
            ));
        }

        let addr = UserAddress::new(
            user_id.to_string(),
            dto.label,
            dto.recipient_name,
            dto.phone,
            line1,
            dto.line2,
            dto.city,
            dto.state,
            dto.postal_code,
            dto.country_code.unwrap_or_else(|| "ID".to_string()),
            dto.is_default_shipping.unwrap_or(false),
            dto.is_default_billing.unwrap_or(false),
        );

        let mut tx = self.pool.begin().await?;

        if addr.is_default_shipping {
            #[cfg(feature = "postgres")]
            sqlx::query("UPDATE user_addresses SET is_default_shipping = false WHERE user_id = $1")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            #[cfg(feature = "sqlite")]
            sqlx::query("UPDATE user_addresses SET is_default_shipping = 0 WHERE user_id = ?")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
        }

        if addr.is_default_billing {
            #[cfg(feature = "postgres")]
            sqlx::query("UPDATE user_addresses SET is_default_billing = false WHERE user_id = $1")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            #[cfg(feature = "sqlite")]
            sqlx::query("UPDATE user_addresses SET is_default_billing = 0 WHERE user_id = ?")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
        }

        #[cfg(feature = "postgres")]
        {
            sqlx::query(
                r#"
                INSERT INTO user_addresses (
                    id, user_id, label, recipient_name, phone,
                    line1, line2, city, state, postal_code, country_code,
                    is_default_shipping, is_default_billing,
                    created_at, updated_at
                )
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
                "#,
            )
            .bind(&addr.id)
            .bind(&addr.user_id)
            .bind(&addr.label)
            .bind(&addr.recipient_name)
            .bind(&addr.phone)
            .bind(&addr.line1)
            .bind(&addr.line2)
            .bind(&addr.city)
            .bind(&addr.state)
            .bind(&addr.postal_code)
            .bind(&addr.country_code)
            .bind(addr.is_default_shipping)
            .bind(addr.is_default_billing)
            .bind(addr.created_at)
            .bind(addr.updated_at)
            .execute(&mut *tx)
            .await?;
        }

        #[cfg(feature = "sqlite")]
        {
            sqlx::query(
                r#"
                INSERT INTO user_addresses (
                    id, user_id, label, recipient_name, phone,
                    line1, line2, city, state, postal_code, country_code,
                    is_default_shipping, is_default_billing,
                    created_at, updated_at
                )
                VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
                "#,
            )
            .bind(&addr.id)
            .bind(&addr.user_id)
            .bind(&addr.label)
            .bind(&addr.recipient_name)
            .bind(&addr.phone)
            .bind(&addr.line1)
            .bind(&addr.line2)
            .bind(&addr.city)
            .bind(&addr.state)
            .bind(&addr.postal_code)
            .bind(&addr.country_code)
            .bind(if addr.is_default_shipping { 1 } else { 0 })
            .bind(if addr.is_default_billing { 1 } else { 0 })
            .bind(addr.created_at.to_rfc3339())
            .bind(addr.updated_at.to_rfc3339())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        let details = serde_json::json!({
            "message": "Created user address",
            "user_id": user_id,
            "address_id": addr.id,
            "label": addr.label,
            "country_code": addr.country_code,
            "is_default_shipping": addr.is_default_shipping,
            "is_default_billing": addr.is_default_billing
        })
        .to_string();
        self.audit_service
            .log(
                actor_id.or(Some(user_id)),
                None,
                "USER_ADDRESS_CREATE",
                "user_address",
                Some(&addr.id),
                Some(details.as_str()),
                ip_address,
            )
            .await;

        Ok(addr)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_address(
        &self,
        user_id: &str,
        address_id: &str,
        dto: UpdateUserAddressDto,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> AppResult<UserAddress> {
        #[cfg(feature = "postgres")]
        let select_q = "SELECT * FROM user_addresses WHERE id = $1 AND user_id = $2";
        #[cfg(feature = "sqlite")]
        let select_q = "SELECT * FROM user_addresses WHERE id = ? AND user_id = ?";

        let mut addr: UserAddress = sqlx::query_as(select_q)
            .bind(address_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Address not found".to_string()))?;

        let before = addr.clone();

        if let Some(v) = dto.label {
            addr.label = Some(v);
        }
        if let Some(v) = dto.recipient_name {
            addr.recipient_name = Some(v);
        }
        if let Some(v) = dto.phone {
            addr.phone = Some(v);
        }
        if let Some(v) = dto.line1 {
            let v = v.trim().to_string();
            if v.is_empty() {
                return Err(AppError::Validation(
                    "Address line1 cannot be empty".to_string(),
                ));
            }
            addr.line1 = v;
        }
        if let Some(v) = dto.line2 {
            addr.line2 = Some(v);
        }
        if let Some(v) = dto.city {
            addr.city = Some(v);
        }
        if let Some(v) = dto.state {
            addr.state = Some(v);
        }
        if let Some(v) = dto.postal_code {
            addr.postal_code = Some(v);
        }
        if let Some(v) = dto.country_code {
            addr.country_code = v;
        }
        if let Some(v) = dto.is_default_shipping {
            addr.is_default_shipping = v;
        }
        if let Some(v) = dto.is_default_billing {
            addr.is_default_billing = v;
        }

        addr.updated_at = Utc::now();

        let mut tx = self.pool.begin().await?;

        if addr.is_default_shipping {
            #[cfg(feature = "postgres")]
            sqlx::query("UPDATE user_addresses SET is_default_shipping = false WHERE user_id = $1 AND id != $2")
                .bind(user_id)
                .bind(address_id)
                .execute(&mut *tx)
                .await?;
            #[cfg(feature = "sqlite")]
            sqlx::query(
                "UPDATE user_addresses SET is_default_shipping = 0 WHERE user_id = ? AND id != ?",
            )
            .bind(user_id)
            .bind(address_id)
            .execute(&mut *tx)
            .await?;
        }

        if addr.is_default_billing {
            #[cfg(feature = "postgres")]
            sqlx::query("UPDATE user_addresses SET is_default_billing = false WHERE user_id = $1 AND id != $2")
                .bind(user_id)
                .bind(address_id)
                .execute(&mut *tx)
                .await?;
            #[cfg(feature = "sqlite")]
            sqlx::query(
                "UPDATE user_addresses SET is_default_billing = 0 WHERE user_id = ? AND id != ?",
            )
            .bind(user_id)
            .bind(address_id)
            .execute(&mut *tx)
            .await?;
        }

        #[cfg(feature = "postgres")]
        {
            sqlx::query(
                r#"
                UPDATE user_addresses
                SET label = $1,
                    recipient_name = $2,
                    phone = $3,
                    line1 = $4,
                    line2 = $5,
                    city = $6,
                    state = $7,
                    postal_code = $8,
                    country_code = $9,
                    is_default_shipping = $10,
                    is_default_billing = $11,
                    updated_at = $12
                WHERE id = $13 AND user_id = $14
                "#,
            )
            .bind(&addr.label)
            .bind(&addr.recipient_name)
            .bind(&addr.phone)
            .bind(&addr.line1)
            .bind(&addr.line2)
            .bind(&addr.city)
            .bind(&addr.state)
            .bind(&addr.postal_code)
            .bind(&addr.country_code)
            .bind(addr.is_default_shipping)
            .bind(addr.is_default_billing)
            .bind(addr.updated_at)
            .bind(&addr.id)
            .bind(&addr.user_id)
            .execute(&mut *tx)
            .await?;
        }

        #[cfg(feature = "sqlite")]
        {
            sqlx::query(
                r#"
                UPDATE user_addresses
                SET label = ?,
                    recipient_name = ?,
                    phone = ?,
                    line1 = ?,
                    line2 = ?,
                    city = ?,
                    state = ?,
                    postal_code = ?,
                    country_code = ?,
                    is_default_shipping = ?,
                    is_default_billing = ?,
                    updated_at = ?
                WHERE id = ? AND user_id = ?
                "#,
            )
            .bind(&addr.label)
            .bind(&addr.recipient_name)
            .bind(&addr.phone)
            .bind(&addr.line1)
            .bind(&addr.line2)
            .bind(&addr.city)
            .bind(&addr.state)
            .bind(&addr.postal_code)
            .bind(&addr.country_code)
            .bind(if addr.is_default_shipping { 1 } else { 0 })
            .bind(if addr.is_default_billing { 1 } else { 0 })
            .bind(addr.updated_at.to_rfc3339())
            .bind(&addr.id)
            .bind(&addr.user_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        let details = serde_json::json!({
            "message": "Updated user address",
            "user_id": user_id,
            "address_id": addr.id,
            "is_default_shipping_before": before.is_default_shipping,
            "is_default_shipping_after": addr.is_default_shipping,
            "is_default_billing_before": before.is_default_billing,
            "is_default_billing_after": addr.is_default_billing,
            "country_code_before": before.country_code,
            "country_code_after": addr.country_code,
        })
        .to_string();
        self.audit_service
            .log(
                actor_id.or(Some(user_id)),
                None,
                "USER_ADDRESS_UPDATE",
                "user_address",
                Some(&addr.id),
                Some(details.as_str()),
                ip_address,
            )
            .await;

        Ok(addr)
    }

    pub async fn delete_address(
        &self,
        user_id: &str,
        address_id: &str,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        #[cfg(feature = "postgres")]
        let query = "DELETE FROM user_addresses WHERE id = $1 AND user_id = $2";
        #[cfg(feature = "sqlite")]
        let query = "DELETE FROM user_addresses WHERE id = ? AND user_id = ?";

        let res = sqlx::query(query)
            .bind(address_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Address not found".to_string()));
        }

        let details = serde_json::json!({
            "message": "Deleted user address",
            "user_id": user_id,
            "address_id": address_id,
        })
        .to_string();
        self.audit_service
            .log(
                actor_id.or(Some(user_id)),
                None,
                "USER_ADDRESS_DELETE",
                "user_address",
                Some(address_id),
                Some(details.as_str()),
                ip_address,
            )
            .await;

        Ok(())
    }
}
