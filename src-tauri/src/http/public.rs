use super::AppState;
use crate::error::AppResult;
use crate::http::auth::extract_ip;
use crate::http::domain_resolver::{normalize_host, resolve_request_domain, ResolvedDomainContext};
use crate::models::{CustomerRegistrationInviteValidationView, RegisterDto, Tenant, User};
use crate::services::decode_unsubscribe_token;
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::HeaderMap,
    response::Html,
    Json,
};
use chrono::Utc;
use std::net::SocketAddr;
use uuid::Uuid;
use validator::Validate;

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
    let domain = normalize_host(&domain)
        .ok_or_else(|| crate::error::AppError::Validation("Invalid domain".to_string()))?;
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

#[derive(serde::Serialize)]
pub struct CustomerRegistrationStatus {
    pub enabled: bool,
    pub global_registration_enabled: bool,
    pub tenant_self_registration_enabled: bool,
}

#[derive(serde::Deserialize)]
pub struct ValidateInviteQuery {
    pub token: String,
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicCustomerRegisterDto {
    #[validate(
        email(message = "Invalid email format"),
        length(max = 255, message = "Email too long")
    )]
    pub email: String,
    #[validate(length(min = 8, max = 128, message = "Password must be 8-128 characters"))]
    pub password: String,
    #[validate(length(min = 2, max = 100, message = "Name must be 2-100 characters"))]
    pub name: String,
    pub invite_token: Option<String>,
}

async fn get_tenant_self_registration_enabled(
    state: &AppState,
    tenant_id: &str,
) -> Result<bool, crate::error::AppError> {
    let enabled = state
        .settings_service
        .get_value(Some(tenant_id), "customer_self_registration_enabled")
        .await?
        .map(|v| v == "true")
        .unwrap_or(false);
    Ok(enabled)
}

pub async fn lookup_tenant_by_domain(
    State(state): State<AppState>,
    Query(query): Query<DomainQuery>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    let domain = normalize_host(&query.domain)
        .ok_or_else(|| crate::error::AppError::Validation("Invalid domain".to_string()))?;
    let tenant = sqlx::query_as("SELECT * FROM tenants WHERE custom_domain = $1")
        .bind(&domain)
        .fetch_optional(&state.auth_service.pool)
        .await?;

    match tenant {
        Some(t) => Ok(Json(t)),
        None => Err(crate::error::AppError::NotFound(
            "Tenant not found".to_string(),
        )),
    }
}

pub async fn customer_registration_status_by_domain(
    State(state): State<AppState>,
    Query(query): Query<DomainQuery>,
) -> Result<Json<CustomerRegistrationStatus>, crate::error::AppError> {
    let auth_settings = state.auth_service.get_auth_settings().await;
    let global_registration_enabled = auth_settings.allow_registration;
    let tenant_self_registration_enabled = match resolve_request_domain(
        &state.auth_service.pool,
        &query.domain,
        auth_settings.main_domain.as_deref(),
    )
    .await?
    {
        ResolvedDomainContext::TenantCustomDomain { tenant_id, .. } => {
            get_tenant_self_registration_enabled(&state, &tenant_id).await?
        }
        _ => false,
    };

    Ok(Json(CustomerRegistrationStatus {
        enabled: global_registration_enabled && tenant_self_registration_enabled,
        global_registration_enabled,
        tenant_self_registration_enabled,
    }))
}

pub async fn validate_customer_registration_invite_by_domain(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ValidateInviteQuery>,
) -> Result<Json<CustomerRegistrationInviteValidationView>, crate::error::AppError> {
    let token = query.token.trim();
    if token.is_empty() {
        return Ok(Json(CustomerRegistrationInviteValidationView {
            valid: false,
            status: "invalid".to_string(),
            message: "Invite token is required".to_string(),
            expires_at: None,
            max_uses: None,
            used_count: None,
            remaining_uses: None,
        }));
    }

    let host = if let Some(domain) = query.domain.as_deref().and_then(normalize_host) {
        domain
    } else {
        match request_host(&headers) {
            Some(v) => v,
            None => {
                return Ok(Json(CustomerRegistrationInviteValidationView {
                    valid: false,
                    status: "invalid".to_string(),
                    message: "Unable to detect request host".to_string(),
                    expires_at: None,
                    max_uses: None,
                    used_count: None,
                    remaining_uses: None,
                }));
            }
        }
    };
    let auth_settings = state.auth_service.get_auth_settings().await;
    let domain_context = resolve_request_domain(
        &state.auth_service.pool,
        &host,
        auth_settings.main_domain.as_deref(),
    )
    .await?;
    if let Some(view) = invite_domain_rejection(&domain_context) {
        return Ok(Json(view));
    }

    let ResolvedDomainContext::TenantCustomDomain { tenant_id, .. } = domain_context else {
        return Ok(Json(CustomerRegistrationInviteValidationView {
            valid: false,
            status: "invalid_domain".to_string(),
            message: "Invite can only be used from a tenant custom domain".to_string(),
            expires_at: None,
            max_uses: None,
            used_count: None,
            remaining_uses: None,
        }));
    };

    let result = state
        .customer_service
        .validate_customer_registration_invite(&tenant_id, token)
        .await?;
    Ok(Json(result))
}

fn request_host(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-host")
        .and_then(|h| h.to_str().ok())
        .and_then(normalize_host)
        .or_else(|| {
            headers
                .get("host")
                .and_then(|h| h.to_str().ok())
                .and_then(normalize_host)
        })
}

fn invite_domain_rejection(
    context: &ResolvedDomainContext,
) -> Option<CustomerRegistrationInviteValidationView> {
    match context {
        ResolvedDomainContext::TenantCustomDomain { .. } => None,
        ResolvedDomainContext::UnknownExternalDomain { .. } => {
            Some(CustomerRegistrationInviteValidationView {
                valid: false,
                status: "tenant_not_found".to_string(),
                message: "No active tenant was found for this domain".to_string(),
                expires_at: None,
                max_uses: None,
                used_count: None,
                remaining_uses: None,
            })
        }
        ResolvedDomainContext::InvalidHost
        | ResolvedDomainContext::LocalDevelopment { .. }
        | ResolvedDomainContext::PlatformDomain { .. }
        | ResolvedDomainContext::PlatformSubdomain { .. } => {
            Some(CustomerRegistrationInviteValidationView {
                valid: false,
                status: "invalid_domain".to_string(),
                message: "Invite can only be used from a tenant custom domain".to_string(),
                expires_at: None,
                max_uses: None,
                used_count: None,
                remaining_uses: None,
            })
        }
    }
}

fn registration_domain_error(context: &ResolvedDomainContext) -> Option<crate::error::AppError> {
    match context {
        ResolvedDomainContext::TenantCustomDomain { .. } => None,
        ResolvedDomainContext::UnknownExternalDomain { .. } => {
            Some(crate::error::AppError::NotFound(
                "No active tenant matched this custom domain".to_string(),
            ))
        }
        ResolvedDomainContext::InvalidHost
        | ResolvedDomainContext::LocalDevelopment { .. }
        | ResolvedDomainContext::PlatformDomain { .. }
        | ResolvedDomainContext::PlatformSubdomain { .. } => {
            Some(crate::error::AppError::Validation(
                "Customer registration is only allowed from a tenant custom domain".to_string(),
            ))
        }
    }
}

pub async fn register_customer_by_domain(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<PublicCustomerRegisterDto>,
) -> Result<Json<crate::services::AuthResponse>, crate::error::AppError> {
    if let Err(e) = payload.validate() {
        return Err(crate::error::AppError::Validation(format!(
            "Validation error: {}",
            e
        )));
    }
    let invite_token = payload
        .invite_token
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());

    let auth_settings = state.auth_service.get_auth_settings().await;
    if !auth_settings.allow_registration && invite_token.is_none() {
        return Err(crate::error::AppError::Validation(
            "Public registration is currently disabled".to_string(),
        ));
    }

    let host = request_host(&headers).ok_or_else(|| {
        crate::error::AppError::Validation(
            "Unable to detect request domain for tenant registration".to_string(),
        )
    })?;
    let domain_context = resolve_request_domain(
        &state.auth_service.pool,
        &host,
        auth_settings.main_domain.as_deref(),
    )
    .await?;
    if let Some(err) = registration_domain_error(&domain_context) {
        return Err(err);
    }
    let tenant_id = match domain_context {
        ResolvedDomainContext::TenantCustomDomain { tenant_id, .. } => tenant_id,
        _ => unreachable!("tenant custom domain already validated"),
    };

    if let Some(invite_token) = invite_token {
        state
            .customer_service
            .consume_customer_registration_invite(&tenant_id, invite_token)
            .await?;
    } else {
        let tenant_self_registration_enabled =
            get_tenant_self_registration_enabled(&state, &tenant_id).await?;
        if !tenant_self_registration_enabled {
            return Err(crate::error::AppError::Validation(
                "Customer self registration is disabled for this tenant".to_string(),
            ));
        }
    }

    let ip = extract_ip(&headers, addr);
    let register_dto = RegisterDto {
        email: payload.email,
        password: payload.password,
        name: payload.name,
    };
    let require_email_verification = state
        .auth_service
        .get_effective_require_email_verification(Some(&tenant_id))
        .await;
    let registration = state
        .auth_service
        .register_with_email_verification_policy(
            register_dto,
            Some(ip.clone()),
            Some(require_email_verification),
        )
        .await?;

    state
        .customer_service
        .create_customer_from_public_registration(
            &tenant_id,
            &registration.user.id,
            &registration.user.name,
            &registration.user.email,
            Some(&ip),
            None,
        )
        .await?;

    if registration.token.is_some() {
        #[cfg(feature = "postgres")]
        let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(&registration.user.id)
            .fetch_one(&state.auth_service.pool)
            .await?;
        #[cfg(feature = "sqlite")]
        let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(&registration.user.id)
            .fetch_one(&state.auth_service.pool)
            .await?;

        let auth_response = state.auth_service.complete_login(user).await?;
        return Ok(Json(auth_response));
    }

    Ok(Json(registration))
}

#[cfg(test)]
mod tests {
    use super::{invite_domain_rejection, registration_domain_error};
    use crate::error::AppError;
    use crate::http::domain_resolver::ResolvedDomainContext;

    #[test]
    fn invite_validation_rejects_platform_domain() {
        let result = invite_domain_rejection(&ResolvedDomainContext::PlatformDomain {
            host: "billing.acme.net".to_string(),
        });

        assert_eq!(result.expect("rejection").status, "invalid_domain");
    }

    #[test]
    fn invite_validation_rejects_local_ip_hosts() {
        let result = invite_domain_rejection(&ResolvedDomainContext::LocalDevelopment {
            host: "127.0.0.1".to_string(),
        });

        assert_eq!(result.expect("rejection").status, "invalid_domain");
    }

    #[test]
    fn tenant_registration_rejects_unknown_domain() {
        let error = registration_domain_error(&ResolvedDomainContext::UnknownExternalDomain {
            host: "unknown.customer.net".to_string(),
        })
        .expect("registration should be rejected");

        assert!(matches!(error, AppError::NotFound(_)));
    }

    #[test]
    fn tenant_registration_accepts_only_active_tenant_domain_context() {
        let error = registration_domain_error(&ResolvedDomainContext::TenantCustomDomain {
            host: "portal.customer.net".to_string(),
            tenant_id: "tenant-1".to_string(),
            slug: "tenant-a".to_string(),
        });

        assert!(error.is_none());
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

// ── OLT Public Traffic/Signal (NO AUTH) ──────────────────────

/// GET /api/public/olt/traffic/{token}
/// Public MRTG-style traffic endpoint — NO AUTH, token-validated
pub async fn olt_public_traffic(
    State(state): State<super::AppState>,
    Path(token): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let result = state.olt_service.get_stats_by_token(&token).await?;
    Ok(Json(result))
}

/// GET /api/public/olt/signal/{token}
/// Public signal distribution endpoint — NO AUTH, token-validated
pub async fn olt_public_signal(
    State(state): State<super::AppState>,
    Path(token): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let result = state.olt_service.get_signal_by_token(&token).await?;
    Ok(Json(result))
}
