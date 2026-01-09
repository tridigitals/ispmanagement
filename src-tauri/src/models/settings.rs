//! Settings Model for key-value configuration storage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Settings entity for app configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Setting {
    pub id: String,
    pub tenant_id: Option<String>,
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Setting {
    pub fn new(tenant_id: Option<String>, key: String, value: String, description: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            key,
            value,
            description,
            created_at: now,
            updated_at: now,
        }
    }
}

/// DTO for creating/updating settings
#[derive(Debug, Deserialize)]
pub struct UpsertSettingDto {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
}
