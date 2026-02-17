
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Customer {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Customer {
    pub fn new(
        tenant_id: String,
        name: String,
        email: Option<String>,
        phone: Option<String>,
        notes: Option<String>,
        is_active: Option<bool>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            name,
            email,
            phone,
            notes,
            is_active: is_active.unwrap_or(true),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CustomerLocation {
    pub id: String,
    pub tenant_id: String,
    pub customer_id: String,
    pub label: String,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CustomerLocation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: String,
        customer_id: String,
        label: String,
        address_line1: Option<String>,
        address_line2: Option<String>,
        city: Option<String>,
        state: Option<String>,
        postal_code: Option<String>,
        country: Option<String>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        notes: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            customer_id,
            label,
            address_line1,
            address_line2,
            city,
            state,
            postal_code,
            country,
            latitude,
            longitude,
            notes,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CustomerUser {
    pub id: String,
    pub tenant_id: String,
    pub customer_id: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
}

impl CustomerUser {
    pub fn new(tenant_id: String, customer_id: String, user_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            customer_id,
            user_id,
            created_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCustomerRequest {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub notes: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCustomerWithPortalRequest {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub notes: Option<String>,
    pub is_active: Option<bool>,
    pub portal_email: String,
    pub portal_name: Option<String>,
    pub portal_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateCustomerRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub notes: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCustomerLocationRequest {
    pub customer_id: String,
    pub label: String,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateCustomerLocationRequest {
    pub label: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddCustomerPortalUserRequest {
    pub customer_id: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCustomerPortalUserRequest {
    pub customer_id: String,
    pub email: String,
    pub name: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CustomerPortalUser {
    pub customer_user_id: String,
    pub user_id: String,
    pub email: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CustomerSubscription {
    pub id: String,
    pub tenant_id: String,
    pub customer_id: String,
    pub location_id: String,
    pub package_id: String,
    pub router_id: Option<String>,
    pub billing_cycle: String, // monthly | yearly
    #[sqlx(try_from = "f64")]
    pub price: f64,
    pub currency_code: String,
    pub status: String, // active | suspended | cancelled
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CustomerSubscriptionView {
    pub id: String,
    pub tenant_id: String,
    pub customer_id: String,
    pub location_id: String,
    pub package_id: String,
    pub router_id: Option<String>,
    pub billing_cycle: String,
    #[sqlx(try_from = "f64")]
    pub price: f64,
    pub currency_code: String,
    pub status: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub package_name: Option<String>,
    pub location_label: Option<String>,
    pub router_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCustomerSubscriptionRequest {
    pub customer_id: String,
    pub location_id: String,
    pub package_id: String,
    pub router_id: Option<String>,
    pub billing_cycle: String,
    pub price: f64,
    pub currency_code: Option<String>,
    pub status: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateCustomerSubscriptionRequest {
    pub location_id: Option<String>,
    pub package_id: Option<String>,
    pub router_id: Option<String>,
    pub billing_cycle: Option<String>,
    pub price: Option<f64>,
    pub currency_code: Option<String>,
    pub status: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub notes: Option<String>,
}
