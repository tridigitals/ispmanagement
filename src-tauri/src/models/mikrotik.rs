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

