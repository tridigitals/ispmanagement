use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IspPackage {
    pub id: String,
    pub tenant_id: String,
    pub service_type: String,
    pub name: String,
    pub description: Option<String>,
    pub features: Vec<String>,
    pub is_active: bool,
    #[sqlx(try_from = "f64")]
    pub price_monthly: f64,
    #[sqlx(try_from = "f64")]
    pub price_yearly: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl IspPackage {
    pub fn new(
        tenant_id: String,
        service_type: Option<String>,
        name: String,
        description: Option<String>,
        features: Option<Vec<String>>,
        is_active: Option<bool>,
        price_monthly: Option<f64>,
        price_yearly: Option<f64>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            service_type: service_type
                .unwrap_or_else(|| "internet_pppoe".to_string())
                .trim()
                .to_string(),
            name,
            description,
            features: features.unwrap_or_default(),
            is_active: is_active.unwrap_or(true),
            price_monthly: price_monthly.unwrap_or(0.0),
            price_yearly: price_yearly.unwrap_or(0.0),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIspPackageRequest {
    pub service_type: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub features: Option<Vec<String>>,
    pub is_active: Option<bool>,
    pub price_monthly: Option<f64>,
    pub price_yearly: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateIspPackageRequest {
    pub service_type: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub features: Option<Vec<String>>,
    pub is_active: Option<bool>,
    pub price_monthly: Option<f64>,
    pub price_yearly: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IspPackageRouterMapping {
    pub id: String,
    pub tenant_id: String,
    pub router_id: String,
    pub package_id: String,
    pub router_profile_name: String,
    pub address_pool: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl IspPackageRouterMapping {
    pub fn new(
        tenant_id: String,
        router_id: String,
        package_id: String,
        router_profile_name: String,
        address_pool: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            router_id,
            package_id,
            router_profile_name,
            address_pool,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertIspPackageRouterMappingRequest {
    pub router_id: String,
    pub package_id: String,
    pub router_profile_name: String,
    pub address_pool: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IspPackageRouterMappingView {
    pub id: String,
    pub tenant_id: String,
    pub router_id: String,
    pub package_id: String,
    pub package_name: String,
    pub router_name: Option<String>,
    pub router_profile_name: String,
    pub address_pool: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
