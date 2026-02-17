use super::AppState;
use crate::error::AppError;
use crate::http::auth::extract_ip;
use crate::models::Tenant;
use axum::{extract::ConnectInfo, extract::State, http::HeaderMap, Json};
use chrono::Utc;
use serde::Deserialize;
use std::net::SocketAddr;

pub async fn get_current_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Tenant>, AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| AppError::Validation("Not a tenant user".to_string()))?;

    let tenant: Tenant = sqlx::query_as("SELECT * FROM tenants WHERE id = $1")
        .bind(&tenant_id)
        .fetch_one(&state.auth_service.pool)
        .await?;

    Ok(Json(tenant))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct UpdateTenantSelfRequest {
    pub name: Option<String>,
    pub custom_domain: Option<String>,
    pub enforce_2fa: Option<bool>,
}

pub async fn update_current_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<UpdateTenantSelfRequest>,
) -> Result<Json<Tenant>, AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| AppError::Validation("Not a tenant user".to_string()))?;
    let ip = extract_ip(&headers, addr);

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "settings", "update")
        .await?;

    // 1. Get Current Tenant
    let current: Tenant = sqlx::query_as("SELECT * FROM tenants WHERE id = $1")
        .bind(&tenant_id)
        .fetch_one(&state.auth_service.pool)
        .await?;

    let before_name = current.name.clone();
    let before_domain = current.custom_domain.clone();
    let before_enforce = current.enforce_2fa;

    // 2. Check Feature Access for Custom Domain
    if let Some(ref domain) = payload.custom_domain {
        if current.custom_domain.as_ref() != Some(domain) {
            let access = state
                .plan_service
                .check_feature_access(&tenant_id, "custom_domain")
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            if !access.has_access {
                return Err(AppError::Forbidden(
                    "Your plan does not support Custom Domains. Please upgrade.".to_string(),
                ));
            }
        }
    }

    // 3. Update
    let new_name = payload.name.unwrap_or_else(|| current.name.clone());
    let new_domain = payload
        .custom_domain
        .or_else(|| current.custom_domain.clone());
    let new_enforce = payload.enforce_2fa.unwrap_or(current.enforce_2fa);
    let now = Utc::now();

    #[cfg(feature = "postgres")]
    let sql = "UPDATE tenants SET name = $1, custom_domain = $2, enforce_2fa = $3, updated_at = $4 WHERE id = $5 RETURNING *";
    #[cfg(feature = "sqlite")]
    let sql = "UPDATE tenants SET name = ?, custom_domain = ?, enforce_2fa = ?, updated_at = ? WHERE id = ? RETURNING *";

    let q = sqlx::query_as::<_, Tenant>(sql)
        .bind(new_name)
        .bind(new_domain)
        .bind(new_enforce);

    #[cfg(feature = "postgres")]
    let q = q.bind(now);
    #[cfg(feature = "sqlite")]
    let q = q.bind(now.to_rfc3339());

    let tenant = q
        .bind(&tenant_id)
        .fetch_one(&state.auth_service.pool)
        .await?;

    // Audit
    let details = serde_json::json!({
        "message": "Updated tenant settings",
        "tenant_id": tenant_id,
        "name_before": before_name,
        "name_after": tenant.name,
        "custom_domain_before": before_domain,
        "custom_domain_after": tenant.custom_domain,
        "enforce_2fa_before": before_enforce,
        "enforce_2fa_after": tenant.enforce_2fa,
    })
    .to_string();
    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "update",
            "tenant",
            Some(&tenant_id),
            Some(details.as_str()),
            Some(&ip),
        )
        .await;

    Ok(Json(tenant))
}
