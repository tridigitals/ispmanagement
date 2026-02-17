use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserAddress {
    pub id: String,
    pub user_id: String,
    pub label: Option<String>,
    pub recipient_name: Option<String>,
    pub phone: Option<String>,
    pub line1: String,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country_code: String,
    pub is_default_shipping: bool,
    pub is_default_billing: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserAddress {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_id: String,
        label: Option<String>,
        recipient_name: Option<String>,
        phone: Option<String>,
        line1: String,
        line2: Option<String>,
        city: Option<String>,
        state: Option<String>,
        postal_code: Option<String>,
        country_code: String,
        is_default_shipping: bool,
        is_default_billing: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            label,
            recipient_name,
            phone,
            line1,
            line2,
            city,
            state,
            postal_code,
            country_code,
            is_default_shipping,
            is_default_billing,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CreateUserAddressDto {
    pub label: Option<String>,
    pub recipient_name: Option<String>,
    pub phone: Option<String>,
    pub line1: String,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country_code: Option<String>,
    pub is_default_shipping: Option<bool>,
    pub is_default_billing: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct UpdateUserAddressDto {
    pub label: Option<String>,
    pub recipient_name: Option<String>,
    pub phone: Option<String>,
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country_code: Option<String>,
    pub is_default_shipping: Option<bool>,
    pub is_default_billing: Option<bool>,
}
