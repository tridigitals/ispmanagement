use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NetworkAsset {
    pub id: String,
    pub tenant_id: String,
    pub asset_group: String,
    pub asset_type: String,
    pub name: String,
    pub code: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub status: String,
    pub customer_id: Option<String>,
    pub location_id: Option<String>,
    pub work_order_id: Option<String>,
    pub parent_asset_id: Option<String>,
    pub olt_id: Option<String>,
    pub pon_port: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub notes: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NetworkAsset {
    pub fn asset_group_for_type(asset_type: &str) -> &'static str {
        match asset_type {
            t if t.starts_with("olt_") => "olt",
            "ont" | "onu" => "cpe",
            "switch" | "router" | "media_converter" | "odf" | "ups" => "infrastructure",
            _ => "access_fiber",
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: String,
        asset_type: String,
        name: String,
        code: Option<String>,
        vendor: Option<String>,
        model: Option<String>,
        serial_number: Option<String>,
        status: Option<String>,
        customer_id: Option<String>,
        location_id: Option<String>,
        work_order_id: Option<String>,
        parent_asset_id: Option<String>,
        olt_id: Option<String>,
        pon_port: Option<String>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        notes: Option<String>,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            asset_group: Self::asset_group_for_type(&asset_type).to_string(),
            asset_type,
            name,
            code,
            vendor,
            model,
            serial_number,
            status: status.unwrap_or_else(|| "available".to_string()),
            customer_id,
            location_id,
            work_order_id,
            parent_asset_id,
            olt_id,
            pon_port,
            latitude,
            longitude,
            notes,
            metadata: metadata.unwrap_or_else(|| serde_json::json!({})),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NetworkAssetListItem {
    pub id: String,
    pub tenant_id: String,
    pub asset_group: String,
    pub asset_type: String,
    pub name: String,
    pub code: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub status: String,
    pub customer_id: Option<String>,
    pub location_id: Option<String>,
    pub work_order_id: Option<String>,
    pub parent_asset_id: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub notes: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub customer_name: Option<String>,
    pub location_label: Option<String>,
    pub work_order_status: Option<String>,
    pub parent_asset_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateNetworkAssetRequest {
    pub asset_type: String,
    pub name: String,
    pub code: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub status: Option<String>,
    pub customer_id: Option<String>,
    pub location_id: Option<String>,
    pub work_order_id: Option<String>,
    pub parent_asset_id: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub notes: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateNetworkAssetRequest {
    pub asset_type: Option<String>,
    pub name: Option<String>,
    pub code: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub status: Option<String>,
    pub customer_id: Option<String>,
    pub location_id: Option<String>,
    pub work_order_id: Option<String>,
    pub parent_asset_id: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub notes: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListNetworkAssetsParams {
    pub q: Option<String>,
    pub asset_type: Option<String>,
    pub status: Option<String>,
    pub customer_id: Option<String>,
    pub location_id: Option<String>,
    pub parent_asset_id: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}
