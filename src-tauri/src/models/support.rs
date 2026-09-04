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
    pub category: Option<String>,
    pub subscription_id: Option<String>,
    pub satisfaction_rating: Option<i32>,
    pub satisfaction_comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    /// When the assigned technician marked the ticket as in_progress.
    pub started_at: Option<DateTime<Utc>>,
    /// When the assigned technician marked the ticket as resolved.
    pub resolved_at: Option<DateTime<Utc>>,
    /// Free-text notes from the technician at resolve time.
    pub completion_notes: Option<String>,
    /// FileRecord ID for the technician's signature image (PNG).
    pub signature_url: Option<String>,
    /// FileRecord IDs attached as proof-of-work photos at resolve time.
    /// Stored as a JSON array of strings (file_record.id values).
    #[sqlx(json)]
    pub completion_photos: Vec<String>,
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
    pub category: Option<String>,
    pub subscription_id: Option<String>,
    pub assigned_to: Option<String>,
    /// Nama penerima tugas, dari `users.name`. Sebelumnya hanya UUID
    /// `assigned_to` yang tersedia, jadi UI tidak bisa menampilkan siapa yang
    /// menangani tiket — hanya "sudah/belum ditugaskan".
    pub assigned_to_name: Option<String>,
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
    /// Resolved display name of the author at the time the message was
    /// created. Populated from the `users.name` column. May be None if
    /// the user was deleted (author_id → NULL via FK ON DELETE SET NULL)
    /// — the UI should fall back to a generic placeholder in that case.
    pub author_name: Option<String>,
    pub body: String,
    pub is_internal: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTicketMessageWithAttachments {
    pub id: String,
    pub ticket_id: String,
    pub author_id: Option<String>,
    pub author_name: Option<String>,
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
    pub category: Option<String>, // general|billing|technical|installation
    #[serde(alias = "subscription_id")]
    pub subscription_id: Option<String>,
    #[serde(alias = "attachment_ids")]
    pub attachment_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ReplySupportTicketDto {
    pub message: String,
    #[serde(alias = "is_internal")]
    pub is_internal: Option<bool>,
    #[serde(alias = "attachment_ids")]
    pub attachment_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct UpdateSupportTicketDto {
    pub status: Option<String>,   // open|pending|closed
    pub priority: Option<String>, // low|normal|high|urgent
    pub category: Option<String>, // general|billing|technical|installation
    pub assigned_to: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SatisfactionDto {
    pub rating: i32,
    pub comment: Option<String>,
}
