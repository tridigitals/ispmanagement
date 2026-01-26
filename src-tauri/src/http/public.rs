use axum::{
    extract::{Path, Query, State},
    Json,
};
use crate::models::Tenant;
use super::AppState;

pub async fn get_tenant_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    let tenant = sqlx::query_as("SELECT * FROM tenants WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(&state.auth_service.pool)
        .await?;

    match tenant {
        Some(t) => Ok(Json(t)),
        None => Err(crate::error::AppError::NotFound("Tenant not found".into())),
    }
}

pub async fn get_tenant_by_domain(
    State(state): State<AppState>,
    Path(domain): Path<String>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    let tenant = sqlx::query_as("SELECT * FROM tenants WHERE custom_domain = $1")
        .bind(&domain)
        .fetch_optional(&state.auth_service.pool)
        .await?;

    match tenant {
        Some(t) => Ok(Json(t)),
        None => Err(crate::error::AppError::NotFound("Tenant not found".into())),
    }
}

#[derive(serde::Deserialize)]
pub struct DomainQuery {
    pub domain: String,
}

pub async fn lookup_tenant_by_domain(
    State(state): State<AppState>,
    Query(query): Query<DomainQuery>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    let tenant = sqlx::query_as("SELECT * FROM tenants WHERE custom_domain = $1")
        .bind(&query.domain)
        .fetch_optional(&state.auth_service.pool)
        .await?;

    match tenant {
        Some(t) => Ok(Json(t)),
        None => Err(crate::error::AppError::NotFound("Tenant not found".to_string())),
    }
}
