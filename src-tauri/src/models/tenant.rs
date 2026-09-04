use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub const CUSTOM_DOMAIN_STATUS_NONE: &str = "none";
pub const CUSTOM_DOMAIN_STATUS_PENDING: &str = "pending";
pub const CUSTOM_DOMAIN_STATUS_ACTIVE: &str = "active";
pub const CUSTOM_DOMAIN_STATUS_FAILED: &str = "failed";

pub fn resolve_custom_domain_lifecycle_transition(
    current_domain: Option<&str>,
    current_status: Option<&str>,
    current_verified_at: Option<DateTime<Utc>>,
    current_failure_reason: Option<&str>,
    next_domain: Option<&str>,
) -> (String, Option<DateTime<Utc>>, Option<String>) {
    if current_domain == next_domain {
        return (
            current_status
                .unwrap_or(CUSTOM_DOMAIN_STATUS_NONE)
                .to_string(),
            current_verified_at,
            current_failure_reason.map(str::to_string),
        );
    }

    if next_domain.is_some() {
        return (CUSTOM_DOMAIN_STATUS_PENDING.to_string(), None, None);
    }

    (CUSTOM_DOMAIN_STATUS_NONE.to_string(), None, None)
}

pub fn apply_manual_custom_domain_status(
    current_domain: Option<&str>,
    next_status: &str,
    failure_reason: Option<&str>,
) -> Result<(String, Option<DateTime<Utc>>, Option<String>), String> {
    if current_domain.is_none() {
        return Err("Custom domain belum diatur untuk tenant ini".to_string());
    }

    match next_status.trim().to_lowercase().as_str() {
        CUSTOM_DOMAIN_STATUS_PENDING => Ok((CUSTOM_DOMAIN_STATUS_PENDING.to_string(), None, None)),
        CUSTOM_DOMAIN_STATUS_ACTIVE => Ok((
            CUSTOM_DOMAIN_STATUS_ACTIVE.to_string(),
            Some(Utc::now()),
            None,
        )),
        CUSTOM_DOMAIN_STATUS_FAILED => {
            let reason = String::from(failure_reason.unwrap_or(""))
                .trim()
                .to_string();
            if reason.is_empty() {
                return Err(
                    "Alasan gagal wajib diisi saat status domain diubah ke failed".to_string(),
                );
            }
            Ok((CUSTOM_DOMAIN_STATUS_FAILED.to_string(), None, Some(reason)))
        }
        _ => Err("Status domain tidak valid".to_string()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub custom_domain: Option<String>,
    pub custom_domain_status: Option<String>,
    pub custom_domain_verified_at: Option<DateTime<Utc>>,
    pub custom_domain_failure_reason: Option<String>,
    pub logo_url: Option<String>,
    pub is_active: bool,
    #[serde(default)]
    pub enforce_2fa: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Tenant {
    pub fn new(name: String, slug: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            slug,
            custom_domain: None,
            custom_domain_status: Some(CUSTOM_DOMAIN_STATUS_NONE.to_string()),
            custom_domain_verified_at: None,
            custom_domain_failure_reason: None,
            logo_url: None,
            is_active: true,
            enforce_2fa: false,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TenantMember {
    pub id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub role: String,            // String representation for backward compatibility
    pub role_id: Option<String>, // New RBAC role ID
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl TenantMember {
    pub fn new(tenant_id: String, user_id: String, role: String, role_id: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            user_id,
            role,
            role_id,
            created_at: Utc::now(),
            deleted_at: None,
        }
    }
}

/// Helper struct for team member details with user info
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TeamMemberWithUser {
    pub id: String, // tenant_member id
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub role_id: Option<String>,
    pub role_name: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    /// Role level for filtering (e.g. only show users with role_level > 20 for ticket assignment)
    pub role_level: Option<i32>,
    /// Apakah akun memakai 2FA. `None` = kueri pemanggil tidak menyertakan kolom
    /// ini (beberapa SELECT hanya butuh nama/role), BUKAN berarti 2FA mati.
    #[sqlx(default)]
    pub two_factor_enabled: Option<bool>,
    /// Kapan email diverifikasi. `None` di dalam Some(...) tidak dibedakan dari
    /// kolom yang tidak di-SELECT, jadi frontend hanya menuduh saat field ini
    /// benar-benar hadir bernilai null.
    #[sqlx(default)]
    pub email_verified_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::{
        apply_manual_custom_domain_status, resolve_custom_domain_lifecycle_transition, Tenant,
        CUSTOM_DOMAIN_STATUS_ACTIVE, CUSTOM_DOMAIN_STATUS_FAILED, CUSTOM_DOMAIN_STATUS_NONE,
        CUSTOM_DOMAIN_STATUS_PENDING,
    };

    #[test]
    fn tenant_new_defaults_to_no_custom_domain_lifecycle() {
        let tenant = Tenant::new("Acme".to_string(), "acme".to_string());

        assert_eq!(tenant.custom_domain.as_deref(), None);
        assert_eq!(
            tenant.custom_domain_status.as_deref(),
            Some(CUSTOM_DOMAIN_STATUS_NONE)
        );
        assert!(tenant.custom_domain_verified_at.is_none());
        assert!(tenant.custom_domain_failure_reason.is_none());
    }

    #[test]
    fn lifecycle_transition_resets_to_pending_when_domain_changes() {
        let now = Utc::now();
        let next = resolve_custom_domain_lifecycle_transition(
            Some("old.example.com"),
            Some(CUSTOM_DOMAIN_STATUS_ACTIVE),
            Some(now),
            Some("old failure"),
            Some("new.example.com"),
        );

        assert_eq!(next.0, CUSTOM_DOMAIN_STATUS_PENDING);
        assert!(next.1.is_none());
        assert!(next.2.is_none());
    }

    #[test]
    fn lifecycle_transition_preserves_state_when_domain_unchanged() {
        let now = Utc::now();
        let next = resolve_custom_domain_lifecycle_transition(
            Some("same.example.com"),
            Some(CUSTOM_DOMAIN_STATUS_ACTIVE),
            Some(now),
            Some("ignored"),
            Some("same.example.com"),
        );

        assert_eq!(next.0, CUSTOM_DOMAIN_STATUS_ACTIVE);
        assert_eq!(next.1, Some(now));
        assert_eq!(next.2.as_deref(), Some("ignored"));
    }

    #[test]
    fn lifecycle_transition_clears_state_when_domain_removed() {
        let next = resolve_custom_domain_lifecycle_transition(
            Some("same.example.com"),
            Some(CUSTOM_DOMAIN_STATUS_ACTIVE),
            Some(Utc::now()),
            Some("ignored"),
            None,
        );

        assert_eq!(next.0, CUSTOM_DOMAIN_STATUS_NONE);
        assert!(next.1.is_none());
        assert!(next.2.is_none());
    }

    #[test]
    fn manual_status_change_sets_active_timestamp() {
        let next = apply_manual_custom_domain_status(
            Some("portal.customer.net"),
            CUSTOM_DOMAIN_STATUS_ACTIVE,
            None,
        )
        .expect("active transition should succeed");

        assert_eq!(next.0, CUSTOM_DOMAIN_STATUS_ACTIVE);
        assert!(next.1.is_some());
        assert!(next.2.is_none());
    }

    #[test]
    fn manual_status_change_requires_reason_for_failed() {
        let err = apply_manual_custom_domain_status(
            Some("portal.customer.net"),
            CUSTOM_DOMAIN_STATUS_FAILED,
            None,
        )
        .expect_err("failed transition should require reason");

        assert!(err.contains("Alasan gagal"));
    }

    #[test]
    fn manual_status_change_sets_failed_reason() {
        let next = apply_manual_custom_domain_status(
            Some("portal.customer.net"),
            CUSTOM_DOMAIN_STATUS_FAILED,
            Some("DNS belum mengarah ke target yang benar"),
        )
        .expect("failed transition should succeed");

        assert_eq!(next.0, CUSTOM_DOMAIN_STATUS_FAILED);
        assert!(next.1.is_none());
        assert_eq!(
            next.2.as_deref(),
            Some("DNS belum mengarah ke target yang benar")
        );
    }

    #[test]
    fn manual_status_change_rejects_missing_custom_domain() {
        let err = apply_manual_custom_domain_status(None, CUSTOM_DOMAIN_STATUS_PENDING, None)
            .expect_err("status change should fail without custom domain");

        assert!(err.contains("belum diatur"));
    }
}
