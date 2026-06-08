use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Registered FCM device for mobile push notifications
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserDevice {
    pub id: String,
    pub user_id: String,
    pub fcm_token: String,
    pub platform: String, // "android" | "ios"
    pub device_info: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// Request DTOs

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterDeviceRequest {
    pub fcm_token: String,
    pub platform: String,
    #[serde(default)]
    pub device_info: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnregisterDeviceRequest {
    pub fcm_token: String,
}
