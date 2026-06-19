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
    /// Geographic coordinates for the OLT physical location.
    /// Sprint C: surfaced via the network map (network_mapping_service).
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub address_line: Option<String>,
    /// Sprint D: upstream MikroTik router that this OLT connects to.
    pub uplink_router_id: Option<uuid::Uuid>,
    /// Sprint D: port name on the upstream router (e.g. ether1).
    pub uplink_port: Option<String>,
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
        latitude: Option<f64>,
        longitude: Option<f64>,
        address_line: Option<String>,
        uplink_router_id: Option<uuid::Uuid>,
        uplink_port: Option<String>,
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
            latitude,
            longitude,
            address_line,
            uplink_router_id,
            uplink_port,
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
    /// Sprint C: optional geographic coordinates for map placement.
    /// Both must be provided together or both omitted.
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    #[serde(default)]
    pub address_line: Option<String>,
    /// Sprint D: link to upstream MikroTik router.
    #[serde(default)]
    pub uplink_router_id: Option<uuid::Uuid>,
    /// Sprint D: port name on upstream router.
    #[serde(default)]
    pub uplink_port: Option<String>,
}

fn default_olt_port() -> i32 {
    80
}

#[derive(Debug, Deserialize)]
pub struct UpdateOltRequest {
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub username: Option<String>,
    pub password: Option<String>,
    /// Sprint C: `Some(None)` clears the value, `None` leaves it unchanged.
    /// Use empty `Option<Option<f64>>` triple-state semantics via custom deserializer.
    #[serde(default, deserialize_with = "deserialize_some_optional_f64")]
    pub latitude: Option<Option<f64>>,
    #[serde(default, deserialize_with = "deserialize_some_optional_f64")]
    pub longitude: Option<Option<f64>>,
    #[serde(default, deserialize_with = "deserialize_some_optional_string")]
    pub address_line: Option<Option<String>>,
    /// Sprint D: upstream MikroTik router. None = unchanged, Some(None) = clear.
    #[serde(default, deserialize_with = "deserialize_some_optional_uuid")]
    pub uplink_router_id: Option<Option<uuid::Uuid>>,
    /// Sprint D: upstream port name. None = unchanged, Some(None) = clear.
    #[serde(default, deserialize_with = "deserialize_some_optional_string")]
    pub uplink_port: Option<Option<String>>,
}

/// Triple-state deserializer: distinguish between field absent (None),
/// field present with null (Some(None)), and field present with value (Some(Some(v))).
fn deserialize_some_optional_f64<'de, D>(deserializer: D) -> Result<Option<Option<f64>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

fn deserialize_some_optional_string<'de, D>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

fn deserialize_some_optional_uuid<'de, D>(
    deserializer: D,
) -> Result<Option<Option<uuid::Uuid>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

#[derive(Debug, Deserialize)]
pub struct OltTestConnectionRequest {
    /// Optional OLT ID — if provided, use the stored password (decrypted) instead
    /// of the request's `password` field. Useful for "test connection" actions
    /// on existing OLTs where the user shouldn't have to retype credentials.
    #[serde(default)]
    pub id: Option<String>,
    pub host: String,
    pub port: i32,
    pub username: String,
    #[serde(default)]
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
    pub rx_power: Option<f32>,
    pub tx_power: Option<f32>,
    pub distance: Option<f32>,
    pub temperature: Option<f32>,
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

// ── ONU with customer link (Sprint B.6) ────────────────────

/// Enriched ONU with linked network_asset + customer info.
/// Returned by `GET /api/admin/olts/{id}/onu-customer`.
#[derive(Debug, Clone, Serialize)]
pub struct OnuWithCustomer {
    pub onu_id: String,
    pub name: String,
    pub mac: String,
    pub status: String,
    pub rx: String,
    pub tx: Option<String>,
    pub distance: Option<String>,
    pub temperature: Option<String>,
    pub pon: String,
    pub olt_id: String,
    pub asset_id: Option<String>,
    pub customer_id: Option<String>,
    pub linked_at: Option<DateTime<Utc>>,
}
