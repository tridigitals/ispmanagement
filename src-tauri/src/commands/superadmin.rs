use crate::models::Tenant;
use crate::services::{AuditService, AuthService, PlanService};
use tauri::State;

#[derive(serde::Serialize)]
pub struct TenantListResponse {
    pub data: Vec<Tenant>,
    pub total: i64,
}

#[tauri::command]
pub async fn list_tenants(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<TenantListResponse, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    // Access pool directly (or via a new TenantService if refactored)
    let tenants: Vec<Tenant> = sqlx::query_as("SELECT * FROM tenants ORDER BY created_at DESC")
        .fetch_all(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    let total = tenants.len() as i64;

    Ok(TenantListResponse {
        data: tenants,
        total,
    })
}

#[tauri::command]
pub async fn delete_tenant(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(&id)
        .execute(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            None,
            "TENANT_DELETED",
            "tenant",
            Some(&id),
            Some("Tenant deleted by Superadmin"),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_tenant(
    token: String,
    name: String,
    slug: String,
    custom_domain: Option<String>,
    owner_email: String,
    owner_password: String,
    plan_id: Option<String>,
    auth_service: State<'_, AuthService>,
    audit_service: State<'_, AuditService>,
    plan_service: State<'_, PlanService>,
) -> Result<Tenant, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let mut tenant = Tenant::new(name, slug);
    tenant.custom_domain = custom_domain;

    // Check if slug exists
    let exists: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM tenants WHERE slug = $1")
        .bind(&tenant.slug)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    if exists {
        return Err("Slug already exists".to_string());
    }

    // Hash owner password
    let password_hash =
        crate::services::AuthService::hash_password(&owner_password).map_err(|e| e.to_string())?;
    let user = crate::models::User::new(owner_email.clone(), password_hash, "Admin".to_string());

    // Check if email exists
    let user_exists: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    if user_exists {
        return Err("User email already exists".to_string());
    }

    // Start Transaction
    let mut tx = auth_service.pool.begin().await.map_err(|e| e.to_string())?;

    // 1. Create Tenant
    #[cfg(feature = "postgres")]
    let sql_t = "INSERT INTO tenants (id, name, slug, custom_domain, logo_url, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
    #[cfg(feature = "sqlite")]
    let sql_t = "INSERT INTO tenants (id, name, slug, custom_domain, logo_url, is_active, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";

    let q_t = sqlx::query(sql_t)
        .bind(&tenant.id)
        .bind(&tenant.name)
        .bind(&tenant.slug)
        .bind(&tenant.custom_domain)
        .bind(&tenant.logo_url);

    #[cfg(feature = "postgres")]
    let q_t = q_t
        .bind(tenant.is_active)
        .bind(tenant.created_at)
        .bind(tenant.updated_at);
    #[cfg(feature = "sqlite")]
    let q_t = q_t
        .bind(if tenant.is_active { 1 } else { 0 })
        .bind(tenant.created_at.to_rfc3339())
        .bind(tenant.updated_at.to_rfc3339());

    q_t.execute(&mut *tx).await.map_err(|e| e.to_string())?;

    // 2. Create User
    #[cfg(feature = "postgres")]
    let sql_u = "INSERT INTO users (id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)";
    #[cfg(feature = "sqlite")]
    let sql_u = "INSERT INTO users (id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";

    let q_u = sqlx::query(sql_u)
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.name)
        .bind("admin") // Global role (legacy)
        .bind(false)
        .bind(true);

    #[cfg(feature = "postgres")]
    let q_u = q_u.bind(user.created_at).bind(user.updated_at);
    #[cfg(feature = "sqlite")]
    let q_u = q_u
        .bind(user.created_at.to_rfc3339())
        .bind(user.updated_at.to_rfc3339());

    q_u.execute(&mut *tx).await.map_err(|e| e.to_string())?;

    // 3. Create 'Owner' Role for this Tenant
    let role_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    #[cfg(feature = "postgres")]
    let sql_r = "INSERT INTO roles (id, tenant_id, name, description, is_system, level, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
    #[cfg(feature = "sqlite")]
    let sql_r = "INSERT INTO roles (id, tenant_id, name, description, is_system, level, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";

    let q_r = sqlx::query(sql_r)
        .bind(&role_id)
        .bind(&tenant.id)
        .bind("Owner") // Role Name MUST be "Owner" for default logic in auth_service
        .bind("Tenant Owner with full access")
        .bind(true)
        .bind(100); // High level

    #[cfg(feature = "postgres")]
    let q_r = q_r.bind(now).bind(now);
    #[cfg(feature = "sqlite")]
    let q_r = q_r.bind(now.to_rfc3339()).bind(now.to_rfc3339());

    q_r.execute(&mut *tx).await.map_err(|e| e.to_string())?;

    // 4. Assign 'Owner' Role to User (Tenant Membership)
    let membership_id = uuid::Uuid::new_v4().to_string();
    #[cfg(feature = "postgres")]
    let sql_m = "INSERT INTO tenant_members (id, tenant_id, user_id, role_id, created_at) VALUES ($1, $2, $3, $4, $5)";
    #[cfg(feature = "sqlite")]
    let sql_m = "INSERT INTO tenant_members (id, tenant_id, user_id, role_id, created_at) VALUES (?, ?, ?, ?, ?)";

    let q_m = sqlx::query(sql_m)
        .bind(membership_id)
        .bind(&tenant.id)
        .bind(&user.id)
        .bind(&role_id); // Use the Role ID we just created

    #[cfg(feature = "postgres")]
    let q_m = q_m.bind(now);
    #[cfg(feature = "sqlite")]
    let q_m = q_m.bind(now.to_rfc3339());

    q_m.execute(&mut *tx).await.map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    // 5. Assign Plan
    let plan_id_to_assign = if let Some(pid) = plan_id {
        Some(pid)
    } else {
        // Try to find default plan
        let default_plan_id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM plans WHERE is_default = 1 OR is_default = true LIMIT 1",
        )
        .fetch_optional(&auth_service.pool)
        .await
        .unwrap_or(None);

        default_plan_id
    };

    if let Some(pid) = plan_id_to_assign {
        if let Err(e) = plan_service.assign_plan_to_tenant(&tenant.id, &pid).await {
            // Log error but don't fail the request since tenant is created
            tracing::error!(
                "Failed to assign plan {} to tenant {}: {}",
                pid,
                tenant.id,
                e
            );
        }
    }

    audit_service
        .log(
            Some(&claims.sub),
            None,
            "TENANT_CREATED",
            "tenant",
            Some(&tenant.id),
            Some(&format!(
                "Created tenant {} with owner {}",
                tenant.name, owner_email
            )),
            None,
        )
        .await;

    Ok(tenant)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_tenant(
    token: String,
    id: String,
    name: String,
    slug: String,
    custom_domain: Option<String>,
    is_active: bool,
    auth_service: State<'_, AuthService>,
    audit_service: State<'_, AuditService>,
) -> Result<Tenant, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    // Check if tenant exists
    let existing: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM tenants WHERE id = $1")
        .bind(&id)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    if !existing {
        return Err("Tenant not found".to_string());
    }

    // Check if slug exists (if changed)
    let slug_owner: Option<String> = sqlx::query_scalar("SELECT id FROM tenants WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(slug_owner_id) = slug_owner {
        if slug_owner_id != id {
            return Err("Slug already taken".to_string());
        }
    }

    #[cfg(feature = "postgres")]
    let sql = "UPDATE tenants SET name = $1, slug = $2, custom_domain = $3, is_active = $4, updated_at = $5 WHERE id = $6 RETURNING *";
    #[cfg(feature = "sqlite")]
    let sql = "UPDATE tenants SET name = ?, slug = ?, custom_domain = ?, is_active = ?, updated_at = ? WHERE id = ? RETURNING *";

    let q = sqlx::query_as::<_, Tenant>(sql)
        .bind(&name)
        .bind(&slug)
        .bind(&custom_domain);

    #[cfg(feature = "postgres")]
    let q = q.bind(is_active).bind(chrono::Utc::now());
    #[cfg(feature = "sqlite")]
    let q = q
        .bind(if is_active { 1 } else { 0 })
        .bind(chrono::Utc::now().to_rfc3339());

    let tenant = q
        .bind(&id)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            None,
            "TENANT_UPDATED",
            "tenant",
            Some(&id),
            Some(&format!("Updated tenant {}, active: {}", name, is_active)),
            None,
        )
        .await;

    Ok(tenant)
}
