use crate::models::Tenant;
use crate::services::AuthService;
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
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    
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
) -> Result<(), String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(id)
        .execute(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn create_tenant(
    token: String,
    name: String,
    slug: String,
    owner_email: String,
    owner_password: String,
    auth_service: State<'_, AuthService>,
) -> Result<Tenant, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let tenant = Tenant::new(name, slug);

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
    let password_hash = crate::services::AuthService::hash_password(&owner_password).map_err(|e| e.to_string())?;
    let user = crate::models::User::new(owner_email, password_hash, "Admin".to_string());

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
    let q_t = q_t.bind(tenant.is_active).bind(tenant.created_at).bind(tenant.updated_at);
    #[cfg(feature = "sqlite")]
    let q_t = q_t.bind(if tenant.is_active { 1 } else { 0 }).bind(tenant.created_at.to_rfc3339()).bind(tenant.updated_at.to_rfc3339());

    q_t.execute(&mut *tx).await.map_err(|e| e.to_string())?;

    #[cfg(feature = "postgres")]
    let sql_u = "INSERT INTO users (id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)";
    #[cfg(feature = "sqlite")]
    let sql_u = "INSERT INTO users (id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";

    let q_u = sqlx::query(sql_u)
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.name)
        .bind("admin")
        .bind(false)
        .bind(true);

    #[cfg(feature = "postgres")]
    let q_u = q_u.bind(user.created_at).bind(user.updated_at);
    #[cfg(feature = "sqlite")]
    let q_u = q_u.bind(user.created_at.to_rfc3339()).bind(user.updated_at.to_rfc3339());

    q_u.execute(&mut *tx).await.map_err(|e| e.to_string())?;

    let membership_id = uuid::Uuid::new_v4().to_string();
    #[cfg(feature = "postgres")]
    let sql_m = "INSERT INTO tenant_members (id, tenant_id, user_id, role, created_at) VALUES ($1, $2, $3, $4, $5)";
    #[cfg(feature = "sqlite")]
    let sql_m = "INSERT INTO tenant_members (id, tenant_id, user_id, role, created_at) VALUES (?, ?, ?, ?, ?)";

    let q_m = sqlx::query(sql_m)
        .bind(membership_id)
        .bind(&tenant.id)
        .bind(&user.id)
        .bind("owner");

    #[cfg(feature = "postgres")]
    let q_m = q_m.bind(chrono::Utc::now());
    #[cfg(feature = "sqlite")]
    let q_m = q_m.bind(chrono::Utc::now().to_rfc3339());

    q_m.execute(&mut *tx).await.map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(tenant)
}
