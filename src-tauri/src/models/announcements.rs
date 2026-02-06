use chrono::{DateTime, Utc};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Announcement {
    pub id: String,
    pub tenant_id: Option<String>,
    pub created_by: Option<String>,
    pub cover_file_id: Option<String>,
    pub title: String,
    pub body: String,
    pub severity: String,
    pub audience: String,
    pub mode: String,           // post|banner
    pub format: String,         // plain|markdown
    pub deliver_in_app: bool,
    pub deliver_email: bool,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub notified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateAnnouncementDto {
    pub scope: Option<String>, // "tenant" | "global"
    pub tenant_id: Option<String>,
    pub cover_file_id: Option<String>,
    pub title: String,
    pub body: String,
    pub severity: Option<String>, // info|success|warning|error
    pub audience: Option<String>, // all|admins
    pub mode: Option<String>,     // post|banner
    pub format: Option<String>,   // plain|markdown
    pub deliver_in_app: Option<bool>,
    pub deliver_email: Option<bool>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateAnnouncementDto {
    pub cover_file_id: Option<Option<String>>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub severity: Option<String>,
    pub audience: Option<String>,
    pub mode: Option<String>,
    pub format: Option<String>,
    pub deliver_in_app: Option<bool>,
    pub deliver_email: Option<bool>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}
