use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(any(feature = "postgres", feature = "sqlite"), derive(sqlx::FromRow))]
pub struct FileRecord {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub original_name: String,
    pub path: String,
    pub size: i64,
    pub content_type: String,
    #[sqlx(default)]
    pub storage_provider: String,
    pub uploaded_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
