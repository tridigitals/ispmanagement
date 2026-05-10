use crate::db::DbPool;

const DEFAULT_PLATFORM_DOMAIN: &str = "billing.tridigitals.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveTenantDomain {
    pub tenant_id: String,
    pub slug: String,
    pub custom_domain: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedDomainContext {
    InvalidHost,
    LocalDevelopment {
        host: String,
    },
    PlatformDomain {
        host: String,
    },
    PlatformSubdomain {
        host: String,
        slug: String,
    },
    TenantCustomDomain {
        host: String,
        tenant_id: String,
        slug: String,
    },
    UnknownExternalDomain {
        host: String,
    },
}

#[derive(Debug, sqlx::FromRow)]
struct TenantDomainRow {
    id: String,
    slug: String,
}

pub fn normalize_host(raw: &str) -> Option<String> {
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

pub fn is_platform_domain(host: &str, configured_main_domain: Option<&str>) -> bool {
    let Some(host) = normalize_host(host) else {
        return false;
    };

    platform_domains(configured_main_domain)
        .into_iter()
        .any(|domain| host == domain)
}

fn is_local_or_ip(host: &str) -> bool {
    host == "localhost"
        || host.ends_with(".localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host.parse::<std::net::IpAddr>().is_ok()
}

fn platform_domains(configured_main_domain: Option<&str>) -> Vec<String> {
    let mut domains = Vec::new();

    if let Some(domain) = configured_main_domain.and_then(normalize_host) {
        domains.push(domain);
    }

    if let Some(domain) = std::env::var("APP_MAIN_DOMAIN")
        .ok()
        .as_deref()
        .and_then(normalize_host)
    {
        if !domains.iter().any(|existing| existing == &domain) {
            domains.push(domain);
        }
    }

    if !domains
        .iter()
        .any(|existing| existing == DEFAULT_PLATFORM_DOMAIN)
    {
        domains.push(DEFAULT_PLATFORM_DOMAIN.to_string());
    }

    domains
}

fn extract_platform_subdomain(host: &str, configured_main_domain: Option<&str>) -> Option<String> {
    for domain in platform_domains(configured_main_domain) {
        let suffix = format!(".{domain}");
        let remainder = host.strip_suffix(&suffix)?;
        if remainder.is_empty() || remainder.contains('.') {
            continue;
        }

        let candidate = remainder.trim();
        let reserved = ["www", "api", "app", "cdn", "mail", "smtp", "imap"];
        if reserved.contains(&candidate) {
            continue;
        }

        return Some(candidate.to_string());
    }

    None
}

pub fn resolve_request_domain_from_records(
    raw_host: &str,
    configured_main_domain: Option<&str>,
    active_custom_domains: &[ActiveTenantDomain],
) -> ResolvedDomainContext {
    let Some(host) = normalize_host(raw_host) else {
        return ResolvedDomainContext::InvalidHost;
    };

    if is_local_or_ip(&host) {
        return ResolvedDomainContext::LocalDevelopment { host };
    }

    if is_platform_domain(&host, configured_main_domain) {
        return ResolvedDomainContext::PlatformDomain { host };
    }

    if let Some(slug) = extract_platform_subdomain(&host, configured_main_domain) {
        return ResolvedDomainContext::PlatformSubdomain { host, slug };
    }

    if let Some(tenant) = active_custom_domains
        .iter()
        .find(|tenant| tenant.custom_domain == host)
    {
        return ResolvedDomainContext::TenantCustomDomain {
            host,
            tenant_id: tenant.tenant_id.clone(),
            slug: tenant.slug.clone(),
        };
    }

    ResolvedDomainContext::UnknownExternalDomain { host }
}

pub async fn resolve_request_domain(
    pool: &DbPool,
    raw_host: &str,
    configured_main_domain: Option<&str>,
) -> Result<ResolvedDomainContext, sqlx::Error> {
    let Some(host) = normalize_host(raw_host) else {
        return Ok(ResolvedDomainContext::InvalidHost);
    };

    if is_local_or_ip(&host) {
        return Ok(ResolvedDomainContext::LocalDevelopment { host });
    }

    if is_platform_domain(&host, configured_main_domain) {
        return Ok(ResolvedDomainContext::PlatformDomain { host });
    }

    if let Some(slug) = extract_platform_subdomain(&host, configured_main_domain) {
        return Ok(ResolvedDomainContext::PlatformSubdomain { host, slug });
    }

    #[cfg(feature = "postgres")]
    let row: Option<TenantDomainRow> = sqlx::query_as(
        "SELECT id, slug FROM tenants WHERE custom_domain = $1 AND is_active = true LIMIT 1",
    )
    .bind(&host)
    .fetch_optional(pool)
    .await?;

    #[cfg(feature = "sqlite")]
    let row: Option<TenantDomainRow> = sqlx::query_as(
        "SELECT id, slug FROM tenants WHERE custom_domain = ? AND is_active = 1 LIMIT 1",
    )
    .bind(&host)
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some(row) => ResolvedDomainContext::TenantCustomDomain {
            host,
            tenant_id: row.id,
            slug: row.slug,
        },
        None => ResolvedDomainContext::UnknownExternalDomain { host },
    })
}

#[cfg(test)]
mod tests {
    use super::{
        is_platform_domain, normalize_host, resolve_request_domain_from_records,
        ActiveTenantDomain, ResolvedDomainContext,
    };

    #[test]
    fn normalizes_host_and_strips_port() {
        assert_eq!(
            normalize_host("Portal.Acme.Net:443"),
            Some("portal.acme.net".to_string())
        );
    }

    #[test]
    fn rejects_empty_host() {
        assert_eq!(normalize_host(""), None);
    }

    #[test]
    fn detects_platform_domain() {
        assert!(is_platform_domain(
            "billing.acme.net",
            Some("billing.acme.net")
        ));
    }

    #[test]
    fn resolves_platform_subdomain_for_main_domain() {
        let resolved = resolve_request_domain_from_records(
            "tenant-a.billing.acme.net",
            Some("billing.acme.net"),
            &[],
        );

        assert_eq!(
            resolved,
            ResolvedDomainContext::PlatformSubdomain {
                host: "tenant-a.billing.acme.net".to_string(),
                slug: "tenant-a".to_string(),
            }
        );
    }

    #[test]
    fn resolves_active_custom_domain_match() {
        let resolved = resolve_request_domain_from_records(
            "Portal.Customer.Net",
            Some("billing.acme.net"),
            &[ActiveTenantDomain {
                tenant_id: "tenant-1".to_string(),
                slug: "tenant-a".to_string(),
                custom_domain: "portal.customer.net".to_string(),
            }],
        );

        assert_eq!(
            resolved,
            ResolvedDomainContext::TenantCustomDomain {
                host: "portal.customer.net".to_string(),
                tenant_id: "tenant-1".to_string(),
                slug: "tenant-a".to_string(),
            }
        );
    }

    #[test]
    fn rejects_unknown_domain_when_no_match_exists() {
        let resolved = resolve_request_domain_from_records(
            "random.customer.net",
            Some("billing.acme.net"),
            &[],
        );

        assert_eq!(
            resolved,
            ResolvedDomainContext::UnknownExternalDomain {
                host: "random.customer.net".to_string(),
            }
        );
    }
}
