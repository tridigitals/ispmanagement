use super::AppState;
use crate::error::AppError;
use crate::http::auth::extract_ip;
use crate::http::domain_resolver::normalize_custom_domain_input;
use crate::models::tenant::resolve_custom_domain_lifecycle_transition;
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

async fn ensure_unique_custom_domain(
    state: &AppState,
    tenant_id: &str,
    custom_domain: Option<&str>,
) -> Result<(), AppError> {
    let Some(custom_domain) = custom_domain else {
        return Ok(());
    };

    let owner: Option<String> =
        sqlx::query_scalar("SELECT id FROM tenants WHERE custom_domain = $1")
            .bind(custom_domain)
            .fetch_optional(&state.auth_service.pool)
            .await?;

    if let Some(owner_id) = owner {
        if owner_id != tenant_id {
            return Err(AppError::Validation(
                "Custom domain already used by another tenant".to_string(),
            ));
        }
    }

    Ok(())
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

    let normalized_custom_domain = normalize_custom_domain_input(payload.custom_domain.as_deref())
        .map_err(AppError::Validation)?;

    // 2. Check Feature Access for Custom Domain
    if current.custom_domain != normalized_custom_domain {
        if normalized_custom_domain.is_some() {
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

    ensure_unique_custom_domain(&state, &tenant_id, normalized_custom_domain.as_deref()).await?;

    // 3. Update
    let new_name = payload.name.unwrap_or_else(|| current.name.clone());
    let new_domain = normalized_custom_domain
        .clone()
        .or_else(|| current.custom_domain.clone());
    let new_enforce = payload.enforce_2fa.unwrap_or(current.enforce_2fa);
    let now = Utc::now();
    let (next_domain_status, next_verified_at, next_failure_reason) =
        resolve_custom_domain_lifecycle_transition(
            current.custom_domain.as_deref(),
            current.custom_domain_status.as_deref(),
            current.custom_domain_verified_at,
            current.custom_domain_failure_reason.as_deref(),
            new_domain.as_deref(),
        );

    #[cfg(feature = "postgres")]
    let sql = "UPDATE tenants SET name = $1, custom_domain = $2, custom_domain_status = $3, custom_domain_verified_at = $4, custom_domain_failure_reason = $5, enforce_2fa = $6, updated_at = $7 WHERE id = $8 RETURNING *";
    #[cfg(feature = "sqlite")]
    let sql = "UPDATE tenants SET name = ?, custom_domain = ?, custom_domain_status = ?, custom_domain_verified_at = ?, custom_domain_failure_reason = ?, enforce_2fa = ?, updated_at = ? WHERE id = ? RETURNING *";

    let q = sqlx::query_as::<_, Tenant>(sql)
        .bind(new_name)
        .bind(new_domain)
        .bind(next_domain_status)
        .bind(next_verified_at)
        .bind(next_failure_reason)
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
        "custom_domain_status_before": current.custom_domain_status,
        "custom_domain_status_after": tenant.custom_domain_status,
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

    // Notify superadmins when custom domain changes
    let domain_changed = before_domain != tenant.custom_domain;
    if domain_changed {
        #[cfg(feature = "postgres")]
        let super_admins: Vec<(String,)> =
            match sqlx::query_as("SELECT id FROM users WHERE is_super_admin = true AND is_active = true")
                .fetch_all(&state.auth_service.pool)
                .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!(error = %e, "failed to list superadmins for custom domain notif");
                    Vec::new()
                }
            };

        #[cfg(feature = "sqlite")]
        let super_admins: Vec<(String,)> =
            match sqlx::query_as("SELECT id FROM users WHERE is_super_admin = 1 AND is_active = 1")
                .fetch_all(&state.auth_service.pool)
                .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!(error = %e, "failed to list superadmins for custom domain notif");
                    Vec::new()
                }
            };

        let title = "Tenant Custom Domain Updated";
        let msg = format!(
            "Tenant **{}** ({}) updated their custom domain.\n• Before: {}\n• After: {}\n• Status: {}",
            tenant.name,
            tenant_id,
            before_domain.unwrap_or_else(|| "none".to_string()),
            tenant.custom_domain.clone().unwrap_or_else(|| "none".to_string()),
            tenant.custom_domain_status.clone().unwrap_or_else(|| "unknown".to_string()),
        );

        for (user_id,) in &super_admins {
            if let Err(e) = state
                .notification_service
                .create_notification(
                    user_id.clone(),
                    None,
                    title.to_string(),
                    msg.clone(),
                    "info".to_string(),
                    "system".to_string(),
                    None,
                )
                .await
            {
                tracing::error!(user_id = %user_id, error = %e, "failed to create custom-domain notification for superadmin");
            }
        }
    }

    Ok(Json(tenant))
}

#[cfg(test)]
mod tests {
    use crate::http::domain_resolver::normalize_custom_domain_input;
    use crate::models::tenant::{
        resolve_custom_domain_lifecycle_transition, CUSTOM_DOMAIN_STATUS_ACTIVE,
        CUSTOM_DOMAIN_STATUS_PENDING,
    };

    #[test]
    fn tenant_custom_domain_input_is_normalized() {
        let normalized = normalize_custom_domain_input(Some("https://Portal.Customer.Net/"))
            .expect("domain should normalize");

        assert_eq!(normalized.as_deref(), Some("portal.customer.net"));
    }

    #[test]
    fn tenant_custom_domain_change_resets_status_to_pending() {
        let next = resolve_custom_domain_lifecycle_transition(
            Some("old.customer.net"),
            Some(CUSTOM_DOMAIN_STATUS_ACTIVE),
            None,
            None,
            Some("new.customer.net"),
        );

        assert_eq!(next.0, CUSTOM_DOMAIN_STATUS_PENDING);
    }
}
