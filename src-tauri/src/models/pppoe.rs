use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum PppoeAccountSource {
    #[default]
    Router,
    ManagedRadius,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PppoeProfile {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub rate_limit: Option<String>,
    pub session_timeout_seconds: Option<i32>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PppoeProfile {
    pub fn new(
        tenant_id: String,
        name: String,
        rate_limit: Option<String>,
        session_timeout_seconds: Option<i32>,
        is_active: Option<bool>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            name,
            rate_limit,
            session_timeout_seconds,
            is_active: is_active.unwrap_or(true),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PppoeAccount {
    pub id: String,
    pub tenant_id: String,
    pub router_id: String,
    pub customer_id: String,
    pub location_id: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_enc: String,
    pub package_id: Option<String>,
    pub profile_id: Option<String>,
    pub router_profile_name: Option<String>,
    pub remote_address: Option<String>,
    pub address_pool: Option<String>,
    pub disabled: bool,
    pub comment: Option<String>,
    pub account_source: PppoeAccountSource,
    pub router_present: bool,
    pub router_secret_id: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub radius_present: bool,
    pub radius_identity: Option<String>,
    pub radius_last_sync_at: Option<DateTime<Utc>>,
    pub radius_last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PppoeAccount {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: String,
        router_id: String,
        customer_id: String,
        location_id: String,
        username: String,
        password_enc: String,
        package_id: Option<String>,
        profile_id: Option<String>,
        router_profile_name: Option<String>,
        remote_address: Option<String>,
        address_pool: Option<String>,
        disabled: Option<bool>,
        comment: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            router_id,
            customer_id,
            location_id,
            username,
            password_enc,
            package_id,
            profile_id,
            router_profile_name,
            remote_address,
            address_pool,
            disabled: disabled.unwrap_or(false),
            comment,
            account_source: PppoeAccountSource::Router,
            router_present: false,
            router_secret_id: None,
            last_sync_at: None,
            last_error: None,
            radius_present: false,
            radius_identity: None,
            radius_last_sync_at: None,
            radius_last_error: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatePppoeAccountRequest {
    pub router_id: String,
    pub customer_id: String,
    pub location_id: String,
    pub work_order_id: Option<String>,
    pub username: String,
    pub password: String,
    pub package_id: Option<String>,
    pub profile_id: Option<String>,
    pub router_profile_name: Option<String>,
    pub remote_address: Option<String>,
    pub address_pool: Option<String>,
    pub disabled: Option<bool>,
    pub comment: Option<String>,
    pub account_source: Option<PppoeAccountSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePppoeAccountRequest {
    pub work_order_id: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub package_id: Option<String>,
    pub profile_id: Option<String>,
    pub router_profile_name: Option<String>,
    pub remote_address: Option<String>,
    pub address_pool: Option<String>,
    pub disabled: Option<bool>,
    pub comment: Option<String>,
    pub account_source: Option<PppoeAccountSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PppoeAccountPublic {
    pub id: String,
    pub tenant_id: String,
    pub router_id: String,
    pub customer_id: String,
    pub location_id: String,
    pub username: String,
    pub package_id: Option<String>,
    pub profile_id: Option<String>,
    pub router_profile_name: Option<String>,
    pub remote_address: Option<String>,
    pub address_pool: Option<String>,
    pub disabled: bool,
    pub comment: Option<String>,
    pub account_source: PppoeAccountSource,
    pub router_present: bool,
    pub router_secret_id: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub radius_present: bool,
    pub radius_identity: Option<String>,
    pub radius_last_sync_at: Option<DateTime<Utc>>,
    pub radius_last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<PppoeAccount> for PppoeAccountPublic {
    fn from(a: PppoeAccount) -> Self {
        Self {
            id: a.id,
            tenant_id: a.tenant_id,
            router_id: a.router_id,
            customer_id: a.customer_id,
            location_id: a.location_id,
            username: a.username,
            package_id: a.package_id,
            profile_id: a.profile_id,
            router_profile_name: a.router_profile_name,
            remote_address: a.remote_address,
            address_pool: a.address_pool,
            disabled: a.disabled,
            comment: a.comment,
            account_source: a.account_source,
            router_present: a.router_present,
            router_secret_id: a.router_secret_id,
            last_sync_at: a.last_sync_at,
            last_error: a.last_error,
            radius_present: a.radius_present,
            radius_identity: a.radius_identity,
            radius_last_sync_at: a.radius_last_sync_at,
            radius_last_error: a.radius_last_error,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PppoeImportAction {
    New,
    Update,
    Same,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PppoeImportCandidate {
    pub username: String,
    pub router_secret_id: Option<String>,
    pub profile_name: Option<String>,
    pub remote_address: Option<String>,
    pub disabled: bool,
    pub comment: Option<String>,
    pub password_available: bool,
    pub action: PppoeImportAction,
    pub existing_account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PppoeImportFromRouterRequest {
    pub usernames: Vec<String>,
    pub customer_id: Option<String>,
    pub location_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PppoeImportError {
    pub username: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PppoeImportResult {
    pub created: u32,
    pub updated: u32,
    pub skipped: u32,
    pub missing_password: u32,
    pub errors: Vec<PppoeImportError>,
    pub used_customer_id: String,
    pub used_location_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ManagedRadiusServer {
    pub id: String,
    pub name: String,
    pub db_host: String,
    pub db_port: i32,
    pub db_name: String,
    pub db_user: String,
    #[serde(skip_serializing)]
    pub db_password_enc: String,
    pub is_active: bool,
    pub is_default: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TenantRadiusAssignment {
    pub id: String,
    pub tenant_id: String,
    pub radius_server_id: String,
    pub is_active: bool,
    pub assigned_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ManagedRadiusNas {
    pub id: String,
    pub tenant_id: String,
    pub router_id: String,
    pub radius_server_id: String,
    pub nas_name: String,
    pub nas_ip_or_cidr: String,
    #[serde(skip_serializing)]
    pub shared_secret_enc: String,
    pub shortname: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pppoe_account_source_defaults_to_router() {
        assert_eq!(PppoeAccountSource::default(), PppoeAccountSource::Router);
    }

    #[test]
    fn pppoe_account_source_uses_snake_case_serde_contract() {
        let raw = serde_json::from_value::<PppoeAccountSource>(json!("managed_radius"))
            .expect("managed_radius should deserialize");
        assert_eq!(raw, PppoeAccountSource::ManagedRadius);

        let serialized =
            serde_json::to_value(PppoeAccountSource::ManagedRadius).expect("serialize source");
        assert_eq!(serialized, json!("managed_radius"));
    }
}
