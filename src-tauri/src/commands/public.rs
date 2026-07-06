use crate::models::{CustomerRegistrationInviteValidationView, RegisterDto, Tenant, User};
use crate::services::{AuthResponse, AuthService, CustomerService, SettingsService};
use tauri::State;
use validator::Validate;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomerRegistrationStatus {
    pub enabled: bool,
    pub global_registration_enabled: bool,
    pub tenant_self_registration_enabled: bool,
}

async fn get_tenant_self_registration_enabled(
    settings: &SettingsService,
    tenant_id: &str,
) -> Result<bool, String> {
    let enabled = settings
        .get_value(Some(tenant_id), "customer_self_registration_enabled")
        .await
        .map_err(|e| e.to_string())?
        .map(|value| value == "true")
        .unwrap_or(false);
    Ok(enabled)
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
            Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => host,
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

fn is_local_or_ip(host: &str) -> bool {
    host == "localhost"
        || host.ends_with(".localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host.parse::<std::net::IpAddr>().is_ok()
}

fn is_platform_domain(host: &str, configured_main_domain: Option<&str>) -> bool {
    if let Some(main) = configured_main_domain.and_then(normalize_host) {
        if host == main {
            return true;
        }
    }

    if let Some(env_main) = std::env::var("APP_MAIN_DOMAIN")
        .ok()
        .and_then(|value| normalize_host(&value))
    {
        if host == env_main {
            return true;
        }
    }

    host == "billing.tridigitals.com"
}

async fn find_tenant_by_slug(auth: &AuthService, slug: &str) -> Result<Option<Tenant>, String> {
    sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE slug = $1")
        .bind(slug)
        .fetch_optional(&auth.pool)
        .await
        .map_err(|e| e.to_string())
}

async fn find_tenant_by_domain(
    auth: &AuthService,
    domain: &str,
    active_only: bool,
) -> Result<Option<Tenant>, String> {
    if active_only {
        sqlx::query_as::<_, Tenant>(
            "SELECT * FROM tenants WHERE custom_domain = $1 AND is_active = true",
        )
        .bind(domain)
        .fetch_optional(&auth.pool)
        .await
        .map_err(|e| e.to_string())
    } else {
        sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE custom_domain = $1")
            .bind(domain)
            .fetch_optional(&auth.pool)
            .await
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn get_tenant_by_slug(
    slug: String,
    auth: State<'_, AuthService>,
) -> Result<Tenant, String> {
    find_tenant_by_slug(&auth, &slug)
        .await?
        .ok_or_else(|| "Tenant not found".to_string())
}

#[tauri::command]
pub async fn get_tenant_by_domain(
    domain: String,
    auth: State<'_, AuthService>,
) -> Result<Tenant, String> {
    find_tenant_by_domain(&auth, &domain, false)
        .await?
        .ok_or_else(|| "Tenant not found".to_string())
}

#[tauri::command]
pub async fn get_customer_registration_status_by_domain(
    domain: String,
    auth: State<'_, AuthService>,
    settings: State<'_, SettingsService>,
) -> Result<CustomerRegistrationStatus, String> {
    let auth_settings = auth.get_auth_settings().await;
    let global_registration_enabled = auth_settings.allow_registration;
    let tenant = find_tenant_by_domain(&auth, &domain, true).await?;
    let tenant_self_registration_enabled = if let Some(tenant) = tenant.as_ref() {
        get_tenant_self_registration_enabled(&settings, &tenant.id).await?
    } else {
        false
    };

    Ok(CustomerRegistrationStatus {
        enabled: global_registration_enabled && tenant_self_registration_enabled,
        global_registration_enabled,
        tenant_self_registration_enabled,
    })
}

#[tauri::command]
pub async fn validate_customer_registration_invite_by_domain(
    token: String,
    domain: Option<String>,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<CustomerRegistrationInviteValidationView, String> {
    let token = token.trim();
    if token.is_empty() {
        return Ok(CustomerRegistrationInviteValidationView {
            valid: false,
            status: "invalid".to_string(),
            message: "Invite token is required".to_string(),
            expires_at: None,
            max_uses: None,
            used_count: None,
            remaining_uses: None,
        });
    }

    let domain = match domain.as_deref().and_then(normalize_host) {
        Some(value) => value,
        None => {
            return Ok(CustomerRegistrationInviteValidationView {
                valid: false,
                status: "invalid_domain".to_string(),
                message: "Invite can only be used from a tenant custom domain".to_string(),
                expires_at: None,
                max_uses: None,
                used_count: None,
                remaining_uses: None,
            });
        }
    };

    let auth_settings = auth.get_auth_settings().await;
    if is_local_or_ip(&domain) || is_platform_domain(&domain, auth_settings.main_domain.as_deref())
    {
        return Ok(CustomerRegistrationInviteValidationView {
            valid: false,
            status: "invalid_domain".to_string(),
            message: "Invite can only be used from a tenant custom domain".to_string(),
            expires_at: None,
            max_uses: None,
            used_count: None,
            remaining_uses: None,
        });
    }

    let tenant = match find_tenant_by_domain(&auth, &domain, true).await? {
        Some(tenant) => tenant,
        None => {
            return Ok(CustomerRegistrationInviteValidationView {
                valid: false,
                status: "tenant_not_found".to_string(),
                message: "No active tenant was found for this domain".to_string(),
                expires_at: None,
                max_uses: None,
                used_count: None,
                remaining_uses: None,
            });
        }
    };

    customers
        .validate_customer_registration_invite(&tenant.id, token)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn register_customer_by_domain(
    email: String,
    password: String,
    name: String,
    invite_token: Option<String>,
    domain: Option<String>,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
    settings: State<'_, SettingsService>,
) -> Result<AuthResponse, String> {
    let payload = RegisterDto {
        email,
        password,
        name,
    };
    payload
        .validate()
        .map_err(|e| format!("Validation error: {e}"))?;

    let invite_token = invite_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let auth_settings = auth.get_auth_settings().await;
    if !auth_settings.allow_registration && invite_token.is_none() {
        return Err("Public registration is currently disabled".to_string());
    }

    let domain = domain
        .as_deref()
        .and_then(normalize_host)
        .ok_or_else(|| "Unable to detect request domain for tenant registration".to_string())?;
    if is_local_or_ip(&domain) {
        return Err(
            "Customer registration is only allowed from a tenant custom domain".to_string(),
        );
    }
    if is_platform_domain(&domain, auth_settings.main_domain.as_deref()) {
        return Err("Use a tenant custom domain to register customer portal access".to_string());
    }

    let tenant = find_tenant_by_domain(&auth, &domain, true)
        .await?
        .ok_or_else(|| "No active tenant matched this custom domain".to_string())?;

    let invite_id = if let Some(invite_token) = invite_token.as_deref() {
        customers
            .consume_customer_registration_invite(&tenant.id, invite_token)
            .await
            .map_err(|e| e.to_string())?
    } else {
        let tenant_self_registration_enabled =
            get_tenant_self_registration_enabled(&settings, &tenant.id).await?;
        if !tenant_self_registration_enabled {
            return Err("Customer self registration is disabled for this tenant".to_string());
        }
        None
    };

    let require_email_verification = auth
        .get_effective_require_email_verification(Some(&tenant.id))
        .await;
    let registration = auth
        .register_with_email_verification_policy(
            payload,
            Some("127.0.0.1".to_string()),
            Some(require_email_verification),
        )
        .await
        .map_err(|e| e.to_string())?;

    customers
        .create_customer_from_public_registration(
            &tenant.id,
            &registration.user.id,
            &registration.user.name,
            &registration.user.email,
            None,
            Some("127.0.0.1"),
            invite_id.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;

    if registration.token.is_some() {
        let user: User = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(&registration.user.id)
            .fetch_one(&auth.pool)
            .await
            .map_err(|e| e.to_string())?;

        return auth.complete_login(user).await.map_err(|e| e.to_string());
    }

    Ok(registration)
}

#[cfg(test)]
mod tests {
    #[test]
    fn public_commands_are_defined() {
        let source = include_str!("public.rs");

        assert!(source.contains("pub async fn get_tenant_by_slug"));
        assert!(source.contains("pub async fn get_tenant_by_domain"));
        assert!(source.contains("pub async fn get_customer_registration_status_by_domain"));
        assert!(source.contains("pub async fn validate_customer_registration_invite_by_domain"));
        assert!(source.contains("pub async fn register_customer_by_domain"));
    }
}
