use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NetworkNode {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub node_type: String,
    pub status: String,
    pub lat: f64,
    pub lng: f64,
    pub capacity_json: serde_json::Value,
    pub health_json: serde_json::Value,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NetworkLink {
    pub id: String,
    pub tenant_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub name: String,
    pub link_type: String,
    pub status: String,
    pub priority: i32,
    pub capacity_mbps: Option<f64>,
    pub utilization_pct: Option<f64>,
    pub loss_db: Option<f64>,
    pub latency_ms: Option<f64>,
    pub geometry: serde_json::Value,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServiceZone {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub zone_type: String,
    pub priority: i32,
    pub status: String,
    pub geometry: serde_json::Value,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ZoneNodeBinding {
    pub id: String,
    pub tenant_id: String,
    pub zone_id: String,
    pub node_id: String,
    pub is_primary: bool,
    pub weight: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ZoneOffer {
    pub id: String,
    pub tenant_id: String,
    pub zone_id: String,
    pub package_id: String,
    pub price_monthly: Option<f64>,
    pub price_yearly: Option<f64>,
    pub is_active: bool,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNetworkNodeRequest {
    pub name: String,
    pub node_type: String,
    pub status: Option<String>,
    pub lat: f64,
    pub lng: f64,
    pub capacity_json: Option<serde_json::Value>,
    pub health_json: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNetworkNodeRequest {
    pub name: Option<String>,
    pub node_type: Option<String>,
    pub status: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub capacity_json: Option<serde_json::Value>,
    pub health_json: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNetworkLinkRequest {
    pub from_node_id: String,
    pub to_node_id: String,
    pub name: String,
    pub link_type: String,
    pub status: Option<String>,
    pub priority: Option<i32>,
    pub capacity_mbps: Option<f64>,
    pub utilization_pct: Option<f64>,
    pub loss_db: Option<f64>,
    pub latency_ms: Option<f64>,
    pub geometry: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNetworkLinkRequest {
    pub from_node_id: Option<String>,
    pub to_node_id: Option<String>,
    pub name: Option<String>,
    pub link_type: Option<String>,
    pub status: Option<String>,
    pub priority: Option<i32>,
    pub capacity_mbps: Option<f64>,
    pub utilization_pct: Option<f64>,
    pub loss_db: Option<f64>,
    pub latency_ms: Option<f64>,
    pub geometry: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateServiceZoneRequest {
    pub name: String,
    pub zone_type: String,
    pub priority: Option<i32>,
    pub status: Option<String>,
    pub geometry: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateServiceZoneRequest {
    pub name: Option<String>,
    pub zone_type: Option<String>,
    pub priority: Option<i32>,
    pub status: Option<String>,
    pub geometry: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateZoneNodeBindingRequest {
    pub zone_id: String,
    pub node_id: String,
    pub is_primary: Option<bool>,
    pub weight: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateZoneOfferRequest {
    pub zone_id: String,
    pub package_id: String,
    pub price_monthly: Option<f64>,
    pub price_yearly: Option<f64>,
    pub is_active: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateZoneOfferRequest {
    pub zone_id: Option<String>,
    pub package_id: Option<String>,
    pub price_monthly: Option<f64>,
    pub price_yearly: Option<f64>,
    pub is_active: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveZoneRequest {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageCheckRequest {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedZoneResponse {
    pub zone: Option<ResolvedZone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageCheckResponse {
    pub zone: Option<ResolvedZone>,
    pub offers: Vec<ZoneOffer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputePathRequest {
    pub source_node_id: String,
    pub target_node_id: String,
    pub max_hops: Option<u32>,
    pub max_utilization_pct: Option<f64>,
    pub allowed_link_types: Option<Vec<String>>,
    pub allowed_statuses: Option<Vec<String>>,
    pub exclude_link_ids: Option<Vec<String>>,
    pub require_active_nodes: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedPathHop {
    pub seq_no: i32,
    pub link_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub name: String,
    pub link_type: String,
    pub status: String,
    pub distance_m: f64,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputePathResponse {
    pub found: bool,
    pub source_node_id: String,
    pub target_node_id: String,
    pub node_ids: Vec<String>,
    pub link_ids: Vec<String>,
    pub hops: Vec<ComputedPathHop>,
    pub total_cost: Option<f64>,
    pub total_distance_m: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ResolvedZone {
    pub id: String,
    pub name: String,
    pub priority: i32,
}
