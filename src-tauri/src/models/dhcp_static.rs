use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum DhcpStaticQueueMode {
    #[default]
    None,
    SimpleQueue,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DhcpStaticService {
    pub id: String,
    pub tenant_id: String,
    pub subscription_id: String,
    pub router_id: String,
    pub customer_id: String,
    pub location_id: String,
    pub package_id: String,
    pub dhcp_server_name: String,
    pub mac_address: String,
    pub ip_address: String,
    pub comment: Option<String>,
    pub disabled: bool,
    pub lease_present: bool,
    pub lease_router_ref: Option<String>,
    pub lease_last_sync_at: Option<DateTime<Utc>>,
    pub lease_last_error: Option<String>,
    pub queue_mode: DhcpStaticQueueMode,
    pub queue_name: Option<String>,
    pub queue_target: Option<String>,
    pub queue_rate_limit: Option<String>,
    pub queue_present: bool,
    pub queue_last_sync_at: Option<DateTime<Utc>>,
    pub queue_last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DhcpStaticService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: String,
        subscription_id: String,
        router_id: String,
        customer_id: String,
        location_id: String,
        package_id: String,
        dhcp_server_name: String,
        mac_address: String,
        ip_address: String,
        comment: Option<String>,
        disabled: Option<bool>,
        queue_mode: Option<DhcpStaticQueueMode>,
        queue_rate_limit: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            subscription_id,
            router_id,
            customer_id,
            location_id,
            package_id,
            dhcp_server_name,
            mac_address,
            ip_address: ip_address.clone(),
            comment,
            disabled: disabled.unwrap_or(false),
            lease_present: false,
            lease_router_ref: None,
            lease_last_sync_at: None,
            lease_last_error: None,
            queue_mode: queue_mode.unwrap_or_default(),
            queue_name: None,
            queue_target: Some(format!("{ip_address}/32")),
            queue_rate_limit,
            queue_present: false,
            queue_last_sync_at: None,
            queue_last_error: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateDhcpStaticServiceRequest {
    pub subscription_id: String,
    pub router_id: String,
    pub customer_id: String,
    pub location_id: String,
    pub package_id: String,
    pub dhcp_server_name: String,
    pub mac_address: String,
    pub ip_address: String,
    pub comment: Option<String>,
    pub disabled: Option<bool>,
    pub queue_mode: Option<DhcpStaticQueueMode>,
    pub queue_rate_limit: Option<String>,
    pub work_order_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateDhcpStaticServiceRequest {
    pub router_id: Option<String>,
    pub package_id: Option<String>,
    pub dhcp_server_name: Option<String>,
    pub mac_address: Option<String>,
    pub ip_address: Option<String>,
    pub comment: Option<String>,
    pub disabled: Option<bool>,
    pub queue_mode: Option<DhcpStaticQueueMode>,
    pub queue_rate_limit: Option<String>,
    pub work_order_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhcpStaticServicePublic {
    pub id: String,
    pub tenant_id: String,
    pub subscription_id: String,
    pub router_id: String,
    pub customer_id: String,
    pub location_id: String,
    pub package_id: String,
    pub dhcp_server_name: String,
    pub mac_address: String,
    pub ip_address: String,
    pub comment: Option<String>,
    pub disabled: bool,
    pub lease_present: bool,
    pub lease_router_ref: Option<String>,
    pub lease_last_sync_at: Option<DateTime<Utc>>,
    pub lease_last_error: Option<String>,
    pub queue_mode: DhcpStaticQueueMode,
    pub queue_name: Option<String>,
    pub queue_target: Option<String>,
    pub queue_rate_limit: Option<String>,
    pub queue_present: bool,
    pub queue_last_sync_at: Option<DateTime<Utc>>,
    pub queue_last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<DhcpStaticService> for DhcpStaticServicePublic {
    fn from(value: DhcpStaticService) -> Self {
        Self {
            id: value.id,
            tenant_id: value.tenant_id,
            subscription_id: value.subscription_id,
            router_id: value.router_id,
            customer_id: value.customer_id,
            location_id: value.location_id,
            package_id: value.package_id,
            dhcp_server_name: value.dhcp_server_name,
            mac_address: value.mac_address,
            ip_address: value.ip_address,
            comment: value.comment,
            disabled: value.disabled,
            lease_present: value.lease_present,
            lease_router_ref: value.lease_router_ref,
            lease_last_sync_at: value.lease_last_sync_at,
            lease_last_error: value.lease_last_error,
            queue_mode: value.queue_mode,
            queue_name: value.queue_name,
            queue_target: value.queue_target,
            queue_rate_limit: value.queue_rate_limit,
            queue_present: value.queue_present,
            queue_last_sync_at: value.queue_last_sync_at,
            queue_last_error: value.queue_last_error,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DhcpStaticQueueMode;

    #[test]
    fn queue_mode_defaults_to_none() {
        assert_eq!(DhcpStaticQueueMode::default(), DhcpStaticQueueMode::None);
    }
}
