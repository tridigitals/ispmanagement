//! Subscription Plan Models
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Subscription Plan
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Plan {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    #[sqlx(try_from = "f64")]
    pub price_monthly: f64,
    #[sqlx(try_from = "f64")]
    pub price_yearly: f64,
    pub is_active: bool,
    pub is_default: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Plan with features included
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanWithFeatures {
    #[serde(flatten)]
    pub plan: Plan,
    pub features: Vec<PlanFeatureValue>,
}

/// Feature Definition
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FeatureDefinition {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub value_type: String, // "boolean", "number", "unlimited"
    pub category: String,
    pub default_value: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

/// Plan Feature mapping (what value a plan has for a feature)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlanFeature {
    pub id: String,
    pub plan_id: String,
    pub feature_id: String,
    pub value: String,
}

/// Feature value for display (includes feature metadata)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlanFeatureValue {
    pub feature_id: String,
    pub code: String,
    pub name: String,
    pub value_type: String,
    pub value: String,
    pub category: String,
}

/// Tenant Subscription
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TenantSubscription {
    pub id: String,
    pub tenant_id: String,
    pub plan_id: String,
    pub status: String, // "active", "cancelled", "past_due", "trial"
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub feature_overrides: Option<String>, // JSON string
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Subscription with plan details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantSubscriptionWithPlan {
    #[serde(flatten)]
    pub subscription: TenantSubscription,
    pub plan: Plan,
}

// ==================== Request/Response DTOs ====================

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatePlanRequest {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    #[serde(alias = "priceMonthly")]
    pub price_monthly: Option<f64>,
    #[serde(alias = "priceYearly")]
    pub price_yearly: Option<f64>,
    #[serde(alias = "isActive")]
    pub is_active: Option<bool>,
    #[serde(alias = "isDefault")]
    pub is_default: Option<bool>,
    #[serde(alias = "sortOrder")]
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePlanRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    #[serde(alias = "priceMonthly")]
    pub price_monthly: Option<f64>,
    #[serde(alias = "priceYearly")]
    pub price_yearly: Option<f64>,
    #[serde(alias = "isActive")]
    pub is_active: Option<bool>,
    #[serde(alias = "isDefault")]
    pub is_default: Option<bool>,
    #[serde(alias = "sortOrder")]
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateFeatureRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub value_type: Option<String>,
    pub category: Option<String>,
    pub default_value: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateFeatureRequest {
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub value_type: Option<String>,
    pub category: Option<String>,
    pub default_value: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(deny_unknown_fields)]
pub struct SetPlanFeatureRequest {
    pub plan_id: String,
    pub feature_id: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(deny_unknown_fields)]
pub struct AssignPlanRequest {
    pub tenant_id: String,
    pub plan_id: String,
}

/// Feature access result
#[derive(Debug, Clone, Serialize)]
pub struct FeatureAccess {
    pub code: String,
    pub has_access: bool,
    pub value: String,
    pub value_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantSubscriptionDetails {
    pub plan_name: String,
    pub plan_slug: String,
    pub status: String,
    pub current_period_end: Option<DateTime<Utc>>,
    pub storage_usage: i64,
    pub storage_limit: Option<i64>,
    pub member_usage: i64,
    pub member_limit: Option<i64>,
    /// Entitlement nyata plan dari DB (features + nilai). Halaman
    /// subscription lama menampilkan daftar HARDCODE per slug — pelanggan
    /// melihat copy marketing, bukan batas sebenarnya.
    pub features: Vec<PlanFeatureValue>,
}

#[cfg(test)]
mod tests {
    use super::{CreatePlanRequest, UpdatePlanRequest};

    #[test]
    fn create_plan_request_accepts_camel_case_payload() {
        let req: CreatePlanRequest = serde_json::from_str(
            r#"{
                "name":"Pro",
                "slug":"pro",
                "priceMonthly":199000,
                "priceYearly":1990000,
                "isActive":true,
                "isDefault":false,
                "sortOrder":10
            }"#,
        )
        .expect("camelCase payload should deserialize");

        assert_eq!(req.price_monthly, Some(199000.0));
        assert_eq!(req.price_yearly, Some(1990000.0));
        assert_eq!(req.is_active, Some(true));
        assert_eq!(req.is_default, Some(false));
        assert_eq!(req.sort_order, Some(10));
    }

    #[test]
    fn update_plan_request_accepts_camel_case_payload() {
        let req: UpdatePlanRequest = serde_json::from_str(
            r#"{
                "priceMonthly":299000,
                "priceYearly":2990000,
                "isActive":false,
                "isDefault":true,
                "sortOrder":20
            }"#,
        )
        .expect("camelCase payload should deserialize");

        assert_eq!(req.price_monthly, Some(299000.0));
        assert_eq!(req.price_yearly, Some(2990000.0));
        assert_eq!(req.is_active, Some(false));
        assert_eq!(req.is_default, Some(true));
        assert_eq!(req.sort_order, Some(20));
    }
}
