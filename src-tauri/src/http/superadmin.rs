use super::AppState;
use crate::http::auth::extract_ip;
use crate::models::Tenant;
use crate::commands::superadmin::{
    SuperadminManagedRadiusServer, SuperadminManagedRadiusServerListResponse,
    SuperadminManagedRadiusUser, SuperadminManagedRadiusUserListResponse,
};
use axum::{
    extract::ConnectInfo,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;

#[derive(Serialize)]
pub struct TenantListResponse {
    pub data: Vec<Tenant>,
    pub total: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
    pub custom_domain: Option<String>,
    pub owner_email: String,
    pub owner_password: String,
    pub plan_id: Option<String>,
}

// ...

// Helper to check super admin permission
async fn check_super_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::services::auth_service::Claims, crate::error::AppError> {
    let token = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(token).await?;

    if !claims.is_super_admin {
        return Err(crate::error::AppError::Unauthorized);
    }

    Ok(claims)
}

pub async fn list_tenants(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<TenantListResponse>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    let tenants: Vec<Tenant> = sqlx::query_as("SELECT * FROM tenants ORDER BY created_at DESC")
        .fetch_all(&mut *tx)
        .await?;

    let total = tenants.len() as i64;
    tx.commit().await?;

    Ok(Json(TenantListResponse {
        data: tenants,
        total,
    }))
}

pub async fn list_managed_radius_servers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SuperadminManagedRadiusServerListResponse>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    let servers: Vec<SuperadminManagedRadiusServer> = sqlx::query_as(
        r#"
        SELECT
          s.id,
          s.tenant_id,
          t.name AS tenant_name,
          s.name,
          s.host,
          s.auth_port,
          s.acct_port,
          s.db_host,
          s.db_port,
          s.db_name,
          s.is_active,
          COUNT(n.id)::bigint AS router_count,
          s.updated_at
        FROM managed_radius_servers s
        INNER JOIN tenants t
          ON t.id = s.tenant_id
        LEFT JOIN managed_radius_nas n
          ON n.radius_server_id = s.id
         AND n.tenant_id = s.tenant_id
         AND n.is_active = true
        GROUP BY
          s.id, s.tenant_id, t.name, s.name, s.host, s.auth_port, s.acct_port,
          s.db_host, s.db_port, s.db_name, s.is_active, s.updated_at
        ORDER BY s.updated_at DESC, s.name ASC
        "#,
    )
    .fetch_all(&mut *tx)
    .await?;

    let total = servers.len() as i64;
    tx.commit().await?;

    Ok(Json(SuperadminManagedRadiusServerListResponse {
        data: servers,
        total,
    }))
}

pub async fn list_managed_radius_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SuperadminManagedRadiusUserListResponse>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    let users: Vec<SuperadminManagedRadiusUser> = sqlx::query_as(
        r#"
        SELECT
          p.id,
          p.tenant_id,
          t.name AS tenant_name,
          p.router_id,
          r.name AS router_name,
          p.username,
          p.radius_identity,
          p.account_source,
          p.radius_present,
          p.radius_last_sync_at,
          p.radius_last_error,
          p.router_profile_name,
          p.updated_at
        FROM pppoe_accounts p
        INNER JOIN tenants t
          ON t.id = p.tenant_id
        LEFT JOIN mikrotik_routers r
          ON r.id = p.router_id
         AND r.tenant_id = p.tenant_id
        WHERE p.account_source = 'managed_radius'
        ORDER BY p.updated_at DESC, p.username ASC
        "#,
    )
    .fetch_all(&mut *tx)
    .await?;

    let total = users.len() as i64;
    tx.commit().await?;

    Ok(Json(SuperadminManagedRadiusUserListResponse {
        data: users,
        total,
    }))
}

pub async fn delete_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(json!({"message": "Tenant deleted successfully"})))
}

pub async fn create_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;

    // 1. Create Tenant object
    let mut tenant = Tenant::new(payload.name, payload.slug);
    tenant.custom_domain = payload.custom_domain;

    // Check if slug exists
    let exists: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM tenants WHERE slug = $1")
        .bind(&tenant.slug)
        .fetch_one(&state.auth_service.pool)
        .await?;

    if exists {
        return Err(crate::error::AppError::Validation(
            "Slug already exists".to_string(),
        ));
    }

    // 2. Hash owner password
    let password_hash = crate::services::AuthService::hash_password(&payload.owner_password)?;
    let user = crate::models::User::new(payload.owner_email, password_hash, "Admin".to_string());

    // Check if email exists
    let user_exists: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&state.auth_service.pool)
        .await?;

    if user_exists {
        return Err(crate::error::AppError::Validation(
            "User email already exists".to_string(),
        ));
    }

    // 3. Start Transaction
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    // Insert Tenant
    sqlx::query(
        "INSERT INTO tenants (id, name, slug, custom_domain, logo_url, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(&tenant.id)
    .bind(&tenant.name)
    .bind(&tenant.slug)
    .bind(&tenant.custom_domain)
    .bind(&tenant.logo_url)
    .bind(tenant.is_active)
    .bind(tenant.created_at)
    .bind(tenant.updated_at)
    .execute(&mut *tx)
    .await?;

    // Insert User
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
    )
    .bind(&user.id)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&user.name)
    .bind("admin") // Tenant admin
    .bind(false)
    .bind(true)
    .bind(user.created_at)
    .bind(user.updated_at)
    .execute(&mut *tx)
    .await?;

    // Create Membership
    let membership_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO tenant_members (id, tenant_id, user_id, role, created_at) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(membership_id)
    .bind(&tenant.id)
    .bind(&user.id)
    .bind("owner")
    .bind(chrono::Utc::now())
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(tenant))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct UpdateTenantRequest {
    pub name: String,
    pub slug: String,
    pub custom_domain: Option<String>,
    pub is_active: bool,
}

pub async fn update_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTenantRequest>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    // Check if tenant exists
    let before: Option<Tenant> = sqlx::query_as("SELECT * FROM tenants WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.auth_service.pool)
        .await?;

    let before = match before {
        Some(t) => t,
        None => {
            return Err(crate::error::AppError::NotFound(
                "Tenant not found".to_string(),
            ));
        }
    };

    // Check if slug exists (if changed)
    let slug_owner: Option<String> = sqlx::query_scalar("SELECT id FROM tenants WHERE slug = $1")
        .bind(&payload.slug)
        .fetch_optional(&state.auth_service.pool)
        .await?;

    if let Some(slug_owner_id) = slug_owner {
        if slug_owner_id != id {
            return Err(crate::error::AppError::Validation(
                "Slug already taken".to_string(),
            ));
        }
    }

    // Update
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    let tenant: Tenant = sqlx::query_as(
        "UPDATE tenants SET name = $1, slug = $2, custom_domain = $3, is_active = $4, updated_at = $5 WHERE id = $6 RETURNING *"
    )
    .bind(&payload.name)
    .bind(&payload.slug)
    .bind(&payload.custom_domain)
    .bind(payload.is_active)
    .bind(chrono::Utc::now())
    .bind(&id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    // Audit
    let details = serde_json::json!({
        "message": "Updated tenant",
        "tenant_id": id,
        "name_before": before.name,
        "name_after": tenant.name,
        "slug_before": before.slug,
        "slug_after": tenant.slug,
        "custom_domain_before": before.custom_domain,
        "custom_domain_after": tenant.custom_domain,
        "is_active_before": before.is_active,
        "is_active_after": tenant.is_active,
    })
    .to_string();
    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&id),
            "update",
            "tenant",
            Some(&id),
            Some(details.as_str()),
            Some(&ip),
        )
        .await;

    Ok(Json(tenant))
}
