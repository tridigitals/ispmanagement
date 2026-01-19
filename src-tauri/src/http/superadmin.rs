use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::models::Tenant;
use super::AppState;

#[derive(Serialize)]
pub struct TenantListResponse {
    pub data: Vec<Tenant>,
    pub total: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
    pub custom_domain: Option<String>,
    pub owner_email: String,
    pub owner_password: String,
}

// ...

// Helper to check super admin permission
async fn check_super_admin(state: &AppState, headers: &HeaderMap) -> Result<(), crate::error::AppError> {
    let token = headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(token).await?;
    
    if !claims.is_super_admin {
        return Err(crate::error::AppError::Unauthorized);
    }
    
    Ok(())
}

pub async fn list_tenants(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<TenantListResponse>, crate::error::AppError> {
    check_super_admin(&state, &headers).await?;

    let tenants: Vec<Tenant> = sqlx::query_as("SELECT * FROM tenants ORDER BY created_at DESC")
        .fetch_all(&state.auth_service.pool)
        .await?;

    let total = tenants.len() as i64;

    Ok(Json(TenantListResponse {
        data: tenants,
        total,
    }))
}

pub async fn delete_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    check_super_admin(&state, &headers).await?;

    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(id)
        .execute(&state.auth_service.pool)
        .await?;

    Ok(Json(json!({"message": "Tenant deleted successfully"})))
}

pub async fn create_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    check_super_admin(&state, &headers).await?;

    // 1. Create Tenant object
    let mut tenant = Tenant::new(payload.name, payload.slug);
    tenant.custom_domain = payload.custom_domain;
    
    // Check if slug exists
    let exists: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM tenants WHERE slug = $1")
        .bind(&tenant.slug)
        .fetch_one(&state.auth_service.pool)
        .await?;

    if exists {
        return Err(crate::error::AppError::Validation("Slug already exists".to_string()));
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
        return Err(crate::error::AppError::Validation("User email already exists".to_string()));
    }

    // 3. Start Transaction
    let mut tx = state.auth_service.pool.begin().await?;

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
pub struct UpdateTenantRequest {
    pub name: String,
    pub slug: String,
    pub custom_domain: Option<String>,
    pub is_active: bool,
}

pub async fn update_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTenantRequest>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    check_super_admin(&state, &headers).await?;

    // Check if tenant exists
    let existing: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM tenants WHERE id = $1")
        .bind(&id)
        .fetch_one(&state.auth_service.pool)
        .await?;

    if !existing {
        return Err(crate::error::AppError::NotFound("Tenant not found".to_string()));
    }

    // Check if slug exists (if changed)
    let slug_owner: Option<String> = sqlx::query_scalar("SELECT id FROM tenants WHERE slug = $1")
        .bind(&payload.slug)
        .fetch_optional(&state.auth_service.pool)
        .await?;

    if let Some(slug_owner_id) = slug_owner {
        if slug_owner_id != id {
            return Err(crate::error::AppError::Validation("Slug already taken".to_string()));
        }
    }

    // Update
    let mut tx = state.auth_service.pool.begin().await?;

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

    Ok(Json(tenant))
}
