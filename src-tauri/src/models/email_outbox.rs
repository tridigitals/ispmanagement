use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmailOutboxItem {
    pub id: String,
    pub tenant_id: Option<String>,
    pub to_email: String,
    pub subject: String,
    pub body: String,
    pub body_html: Option<String>,
    pub status: String, // queued | sending | sent | failed
    pub attempts: i32,
    pub max_attempts: i32,
    pub scheduled_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailOutboxStats {
    pub all: i64,
    pub queued: i64,
    pub sending: i64,
    pub sent: i64,
    pub failed: i64,
}
