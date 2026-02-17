use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SupportTicket {
    pub id: String,
    pub tenant_id: String,
    pub created_by: Option<String>,
    pub subject: String,
    pub status: String,
    pub priority: String,
    pub assigned_to: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SupportTicketListItem {
    pub id: String,
    pub tenant_id: String,
    pub created_by: Option<String>,
    pub created_by_name: Option<String>,
    pub subject: String,
    pub status: String,
    pub priority: String,
    pub assigned_to: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub message_count: i64,
    pub last_message_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SupportTicketMessage {
    pub id: String,
    pub ticket_id: String,
    pub author_id: Option<String>,
    pub body: String,
    pub is_internal: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTicketMessageWithAttachments {
    pub id: String,
    pub ticket_id: String,
    pub author_id: Option<String>,
    pub body: String,
    pub is_internal: bool,
    pub created_at: DateTime<Utc>,
    pub attachments: Vec<crate::models::FileRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTicketDetail {
    pub ticket: SupportTicket,
    pub messages: Vec<SupportTicketMessageWithAttachments>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CreateSupportTicketDto {
    pub subject: String,
    pub message: String,
    pub priority: Option<String>, // low|normal|high|urgent
    pub attachment_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ReplySupportTicketDto {
    pub message: String,
    pub is_internal: Option<bool>,
    pub attachment_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct UpdateSupportTicketDto {
    pub status: Option<String>,   // open|pending|closed
    pub priority: Option<String>, // low|normal|high|urgent
    pub assigned_to: Option<String>,
}
