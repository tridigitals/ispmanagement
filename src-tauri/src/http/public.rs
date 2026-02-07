use super::AppState;
use crate::models::Tenant;
use crate::services::decode_unsubscribe_token;
use axum::{
    extract::{Path, Query, State},
    response::Html,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

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
        None => Err(crate::error::AppError::NotFound(
            "Tenant not found".to_string(),
        )),
    }
}

// GET /api/public/unsubscribe/:token
pub async fn unsubscribe(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Html<String>, crate::error::AppError> {
    let claims = decode_unsubscribe_token(&state.auth_service.pool, &token).await?;

    // We only support email channel preferences for now.
    if claims.channel != "email" {
        return Ok(Html("Unsupported unsubscribe channel.".to_string()));
    }

    let now = Utc::now();
    let id = Uuid::new_v4().to_string();

    #[cfg(feature = "postgres")]
    {
        let _ = sqlx::query(
            r#"
            INSERT INTO notification_preferences (id, user_id, channel, category, enabled, updated_at)
            VALUES ($1,$2,$3,$4,false,$5)
            ON CONFLICT (user_id, channel, category)
            DO UPDATE SET enabled = false, updated_at = EXCLUDED.updated_at
        "#,
        )
        .bind(&id)
        .bind(&claims.sub)
        .bind(&claims.channel)
        .bind(&claims.category)
        .bind(now)
        .execute(&state.auth_service.pool)
        .await?;
    }

    Ok(Html(
        "You have been unsubscribed from this email category. You can re-enable it in Notification Settings.".to_string(),
    ))
}
