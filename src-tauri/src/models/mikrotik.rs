//! MikroTik Router models (tenant-scoped inventory + monitoring snapshots)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MikrotikRouter {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub use_tls: bool,
    pub enabled: bool,
    pub identity: Option<String>,
    pub ros_version: Option<String>,
    pub is_online: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub latency_ms: Option<i32>,
    pub last_error: Option<String>,
    pub maintenance_until: Option<DateTime<Utc>>,
    pub maintenance_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MikrotikRouter {
    pub fn new(
        tenant_id: String,
        name: String,
        host: String,
        port: i32,
        username: String,
        password: String,
        use_tls: bool,
        enabled: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            name,
            host,
            port,
            username,
            password,
            use_tls,
            enabled,
            identity: None,
            ros_version: None,
            is_online: false,
            last_seen_at: None,
            latency_ms: None,
            last_error: None,
            maintenance_until: None,
            maintenance_reason: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMikrotikRouterRequest {
    pub name: String,
    pub host: String,
    pub port: Option<i32>,
    pub username: String,
    pub password: String,
    pub use_tls: Option<bool>,
    pub enabled: Option<bool>,
    pub maintenance_until: Option<DateTime<Utc>>,
    pub maintenance_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMikrotikRouterRequest {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub username: Option<String>,
    /// If omitted, keep existing password.
    pub password: Option<String>,
    pub use_tls: Option<bool>,
    pub enabled: Option<bool>,
    pub maintenance_until: Option<DateTime<Utc>>,
    pub maintenance_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MikrotikTestResult {
    pub ok: bool,
    pub identity: Option<String>,
    pub ros_version: Option<String>,
    pub latency_ms: Option<i32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MikrotikRouterMetric {
    pub id: String,
    pub router_id: String,
    pub ts: DateTime<Utc>,
    pub cpu_load: Option<i32>,
    pub total_memory_bytes: Option<i64>,
    pub free_memory_bytes: Option<i64>,
    pub total_hdd_bytes: Option<i64>,
    pub free_hdd_bytes: Option<i64>,
    pub uptime_seconds: Option<i64>,
    pub rx_bps: Option<i64>,
    pub tx_bps: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MikrotikInterfaceMetric {
    pub id: String,
    pub router_id: String,
    pub interface_name: String,
    pub ts: DateTime<Utc>,
    pub rx_byte: Option<i64>,
    pub tx_byte: Option<i64>,
    pub rx_bps: Option<i64>,
    pub tx_bps: Option<i64>,
    pub running: Option<bool>,
    pub disabled: Option<bool>,
    pub link_downs: Option<i64>,
}

impl MikrotikInterfaceMetric {
    pub fn new(router_id: String, interface_name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            router_id,
            interface_name,
            ts: Utc::now(),
            rx_byte: None,
            tx_byte: None,
            rx_bps: None,
            tx_bps: None,
            running: None,
            disabled: None,
            link_downs: None,
        }
    }
}

impl MikrotikRouterMetric {
    pub fn new(router_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            router_id,
            ts: Utc::now(),
            cpu_load: None,
            total_memory_bytes: None,
            free_memory_bytes: None,
            total_hdd_bytes: None,
            free_hdd_bytes: None,
            uptime_seconds: None,
            rx_bps: None,
            tx_bps: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MikrotikInterfaceSnapshot {
    pub name: String,
    pub interface_type: Option<String>,
    pub running: Option<bool>,
    pub disabled: Option<bool>,
    pub mtu: Option<i32>,
    pub mac_address: Option<String>,
    pub rx_byte: Option<i64>,
    pub tx_byte: Option<i64>,
    pub rx_packet: Option<i64>,
    pub tx_packet: Option<i64>,
    pub link_downs: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MikrotikInterfaceCounter {
    pub name: String,
    pub running: Option<bool>,
    pub disabled: Option<bool>,
    pub rx_byte: Option<i64>,
    pub tx_byte: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MikrotikIpAddressSnapshot {
    pub address: String,
    pub network: Option<String>,
    pub interface: Option<String>,
    pub disabled: Option<bool>,
    pub dynamic: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MikrotikHealthSnapshot {
    pub temperature_c: Option<f64>,
    pub voltage_v: Option<f64>,
    pub cpu_temperature_c: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MikrotikRouterSnapshot {
    pub router: MikrotikRouter,
    pub cpu_load: Option<i32>,
    pub total_memory_bytes: Option<i64>,
    pub free_memory_bytes: Option<i64>,
    pub total_hdd_bytes: Option<i64>,
    pub free_hdd_bytes: Option<i64>,
    pub uptime_seconds: Option<i64>,
    pub board_name: Option<String>,
    pub architecture: Option<String>,
    pub cpu: Option<String>,
    pub interfaces: Vec<MikrotikInterfaceSnapshot>,
    pub ip_addresses: Vec<MikrotikIpAddressSnapshot>,
    pub health: Option<MikrotikHealthSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MikrotikRouterNocRow {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub use_tls: bool,
    pub enabled: bool,
    pub identity: Option<String>,
    pub ros_version: Option<String>,
    pub is_online: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub latency_ms: Option<i32>,
    pub last_error: Option<String>,
    pub maintenance_until: Option<DateTime<Utc>>,
    pub maintenance_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Latest router metrics (optional if no samples yet)
    pub cpu_load: Option<i32>,
    pub total_memory_bytes: Option<i64>,
    pub free_memory_bytes: Option<i64>,
    pub total_hdd_bytes: Option<i64>,
    pub free_hdd_bytes: Option<i64>,
    pub uptime_seconds: Option<i64>,
    pub rx_bps: Option<i64>,
    pub tx_bps: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MikrotikAlert {
    pub id: String,
    pub tenant_id: String,
    pub router_id: String,
    pub alert_type: String, // offline | cpu | latency
    pub severity: String,   // info | warning | critical
    pub status: String,     // open | ack | resolved
    pub title: String,
    pub message: String,
    pub value_num: Option<f64>,
    pub threshold_num: Option<f64>,
    pub triggered_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acked_at: Option<DateTime<Utc>>,
    pub acked_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MikrotikAlert {
    pub fn new(
        tenant_id: String,
        router_id: String,
        alert_type: String,
        severity: String,
        title: String,
        message: String,
        value_num: Option<f64>,
        threshold_num: Option<f64>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            router_id,
            alert_type,
            severity,
            status: "open".to_string(),
            title,
            message,
            value_num,
            threshold_num,
            triggered_at: now,
            last_seen_at: now,
            resolved_at: None,
            acked_at: None,
            acked_by: None,
            created_at: now,
            updated_at: now,
        }
    }
}
