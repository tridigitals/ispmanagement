//! `TrustedDevice` model
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct TrustedDevice {
    pub id: String,
    pub user_id: String,
    pub device_fingerprint: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub trusted_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}
