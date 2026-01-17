//! Subscription Plan Models

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
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
pub struct CreatePlanRequest {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub price_monthly: Option<f64>,
    pub price_yearly: Option<f64>,
    pub is_active: Option<bool>,
    pub is_default: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlanRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub price_monthly: Option<f64>,
    pub price_yearly: Option<f64>,
    pub is_active: Option<bool>,
    pub is_default: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
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
pub struct SetPlanFeatureRequest {
    pub plan_id: String,
    pub feature_id: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
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
