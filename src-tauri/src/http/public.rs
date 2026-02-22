use super::AppState;
use crate::http::auth::extract_ip;
use crate::models::{RegisterDto, Tenant, User};
use crate::services::decode_unsubscribe_token;
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::HeaderMap,
    response::Html,
    Json,
};
use chrono::Utc;
use std::net::{IpAddr, SocketAddr};
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

#[derive(serde::Serialize)]
pub struct CustomerRegistrationStatus {
    pub enabled: bool,
    pub global_registration_enabled: bool,
    pub tenant_self_registration_enabled: bool,
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

pub async fn customer_registration_status_by_domain(
    State(state): State<AppState>,
    Query(query): Query<DomainQuery>,
) -> Result<Json<CustomerRegistrationStatus>, crate::error::AppError> {
    let auth_settings = state.auth_service.get_auth_settings().await;
    let global_registration_enabled = auth_settings.allow_registration;

    #[cfg(feature = "postgres")]
    let tenant: Option<Tenant> =
        sqlx::query_as("SELECT * FROM tenants WHERE custom_domain = $1 AND is_active = true")
            .bind(&query.domain)
            .fetch_optional(&state.auth_service.pool)
            .await?;
    #[cfg(feature = "sqlite")]
    let tenant: Option<Tenant> =
        sqlx::query_as("SELECT * FROM tenants WHERE custom_domain = ? AND is_active = 1")
            .bind(&query.domain)
            .fetch_optional(&state.auth_service.pool)
            .await?;

    let tenant_self_registration_enabled = if let Some(t) = tenant.as_ref() {
        get_tenant_self_registration_enabled(&state, &t.id).await?
    } else {
        false
    };

    Ok(Json(CustomerRegistrationStatus {
        enabled: global_registration_enabled && tenant_self_registration_enabled,
        global_registration_enabled,
        tenant_self_registration_enabled,
    }))
}

fn normalize_host(raw: &str) -> Option<String> {
    let first = raw.split(',').next()?.trim().to_lowercase();
    if first.is_empty() {
        return None;
    }

    let no_scheme = first
        .strip_prefix("https://")
        .or_else(|| first.strip_prefix("http://"))
        .unwrap_or(first.as_str());

    let no_path = no_scheme.split('/').next()?.trim();
    if no_path.is_empty() {
        return None;
    }

    let host_no_port = if no_path.starts_with('[') {
        let end = no_path.find(']').unwrap_or(no_path.len());
        &no_path[1..end]
    } else {
        match no_path.rsplit_once(':') {
            Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) => h,
            _ => no_path,
        }
    };

    let host = host_no_port.trim().trim_end_matches('.');
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
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

fn is_local_or_ip(host: &str) -> bool {
    host == "localhost"
        || host.ends_with(".localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host.parse::<IpAddr>().is_ok()
}

fn is_platform_domain(host: &str, configured_main_domain: Option<&str>) -> bool {
    if let Some(main) = configured_main_domain.and_then(normalize_host) {
        if host == main {
            return true;
        }
    }

    if let Some(env_main) = std::env::var("APP_MAIN_DOMAIN")
        .ok()
        .and_then(|v| normalize_host(&v))
    {
        if host == env_main {
            return true;
        }
    }

    host == "billing.tridigitals.com"
}

pub async fn register_customer_by_domain(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<RegisterDto>,
) -> Result<Json<crate::services::AuthResponse>, crate::error::AppError> {
    use validator::Validate;
    if let Err(e) = payload.validate() {
        return Err(crate::error::AppError::Validation(format!(
            "Validation error: {}",
            e
        )));
    }

    let auth_settings = state.auth_service.get_auth_settings().await;
    if !auth_settings.allow_registration {
        return Err(crate::error::AppError::Validation(
            "Public registration is currently disabled".to_string(),
        ));
    }

    let host = request_host(&headers).ok_or_else(|| {
        crate::error::AppError::Validation(
            "Unable to detect request domain for tenant registration".to_string(),
        )
    })?;
    if is_local_or_ip(&host) {
        return Err(crate::error::AppError::Validation(
            "Customer registration is only allowed from a tenant custom domain".to_string(),
        ));
    }
    if is_platform_domain(&host, auth_settings.main_domain.as_deref()) {
        return Err(crate::error::AppError::Validation(
            "Use a tenant custom domain to register customer portal access".to_string(),
        ));
    }

    #[cfg(feature = "postgres")]
    let tenant: Option<Tenant> =
        sqlx::query_as("SELECT * FROM tenants WHERE custom_domain = $1 AND is_active = true")
            .bind(&host)
            .fetch_optional(&state.auth_service.pool)
            .await?;
    #[cfg(feature = "sqlite")]
    let tenant: Option<Tenant> =
        sqlx::query_as("SELECT * FROM tenants WHERE custom_domain = ? AND is_active = 1")
            .bind(&host)
            .fetch_optional(&state.auth_service.pool)
            .await?;

    let tenant = tenant.ok_or_else(|| {
        crate::error::AppError::NotFound("No active tenant matched this custom domain".to_string())
    })?;

    let tenant_self_registration_enabled =
        get_tenant_self_registration_enabled(&state, &tenant.id).await?;
    if !tenant_self_registration_enabled {
        return Err(crate::error::AppError::Validation(
            "Customer self registration is disabled for this tenant".to_string(),
        ));
    }

    let ip = extract_ip(&headers, addr);
    let require_email_verification = state
        .auth_service
        .get_effective_require_email_verification(Some(&tenant.id))
        .await;
    let registration = state
        .auth_service
        .register_with_email_verification_policy(
            payload,
            Some(ip.clone()),
            Some(require_email_verification),
        )
        .await?;

    state
        .customer_service
        .create_customer_from_public_registration(
            &tenant.id,
            &registration.user.id,
            &registration.user.name,
            &registration.user.email,
            Some(&ip),
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
