//! OLT (Optical Line Terminal) models — tenant-scoped inventory, monitoring, and ONU history

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

// ── Entity ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Olt {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub description: Option<String>,
    pub olt_type: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_enc: Option<String>,
    pub last_stats: Option<JsonValue>,
    pub last_updated: Option<DateTime<Utc>>,
    pub is_online: bool,
    pub last_polled_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Olt {
    pub fn new(
        tenant_id: String,
        name: String,
        description: Option<String>,
        olt_type: String,
        host: String,
        port: i32,
        username: String,
        password_enc: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            name,
            description,
            olt_type,
            host,
            port,
            username,
            password_enc,
            last_stats: None,
            last_updated: None,
            is_online: false,
            last_polled_at: None,
            last_error: None,
            created_at: now,
            updated_at: now,
        }
    }
}

// ── Request DTOs ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateOltRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub olt_type: String,
    pub host: String,
    #[serde(default = "default_olt_port")]
    pub port: i32,
    pub username: String,
    pub password: String,
}

fn default_olt_port() -> i32 {
    80
}

#[derive(Debug, Deserialize)]
pub struct UpdateOltRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OltTestConnectionRequest {
    pub host: String,
    pub port: i32,
    pub username: String,
    pub password: String,
    pub olt_type: String,
}

// ── Response / Stats DTOs ────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct OltStatsResponse {
    pub status: String,
    pub data: OltGlobalStats,
    pub info: Option<OltSystemInfo>,
    pub cached: bool,
    pub is_online: bool,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OltGlobalStats {
    pub name: Option<String>,
    pub ip: Option<String>,
    pub pon_ports: Vec<PonPortStats>,
    pub total_onus: i32,
    pub online_onus: i32,
    pub offline_onus: i32,
    pub low_onus: i32,
    pub risk_onus: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PonPortStats {
    pub name: String,
    pub total: i32,
    pub online: i32,
    pub offline: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OltSystemInfo {
    pub name: String,
    pub model: String,
    pub version: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OltOnuDetail {
    pub onu_id: String,
    pub name: String,
    pub mac: String,
    pub status: String,
    pub rx: String,
    pub tx: Option<String>,
    pub distance: Option<String>,
    pub temperature: Option<String>,
    pub pon: String,
    pub olt_id: Option<String>,
    pub olt_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OltAllDetailsResponse {
    pub status: String,
    pub info: OltSystemInfo,
    pub onus: Vec<OltOnuDetail>,
    pub stats: OltGlobalStats,
}

#[derive(Debug, Clone, Serialize)]
pub struct AllOnusResponse {
    pub status: String,
    pub data: Vec<OltOnuDetail>,
}

#[derive(Debug, Deserialize)]
pub struct RebootOnuRequest {
    pub onu_id: String,
    pub onu_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub info: Option<OltSystemInfo>,
    pub error: Option<String>,
}

// ── ONU History ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OltOnuHistoryRecord {
    pub id: String,
    pub olt_id: String,
    pub tenant_id: String,
    pub onu_id: String,
    pub pon: String,
    pub mac: Option<String>,
    pub name: Option<String>,
    pub status: String,
    pub rx_power: Option<f64>,
    pub tx_power: Option<f64>,
    pub distance: Option<f64>,
    pub temperature: Option<f64>,
    pub recorded_at: DateTime<Utc>,
}

// ── Public Token ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OltPublicToken {
    pub id: String,
    pub olt_id: String,
    pub tenant_id: String,
    pub token: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePublicTokenRequest {
    pub description: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    pub expires_at: Option<String>,
}
