use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageTemplateChannel {
    Whatsapp,
    Email,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageTemplateStatus {
    Draft,
    Active,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageTemplate {
    pub id: String,
    pub tenant_id: String,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub use_case: String,
    pub target: String,
    pub trigger_mode: String,
    pub event_key: Option<String>,
    pub channel: String,
    pub locale: String,
    pub status: String,
    pub whatsapp_body: Option<String>,
    pub email_subject: Option<String>,
    pub email_body: Option<String>,
    pub variables: Vec<String>,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageTemplateListQuery {
    pub q: Option<String>,
    pub use_case: Option<String>,
    pub channel: Option<String>,
    pub status: Option<String>,
    pub target: Option<String>,
    pub trigger_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageTemplatePayload {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub use_case: String,
    pub target: String,
    pub trigger_mode: String,
    pub event_key: Option<String>,
    pub channel: String,
    pub locale: Option<String>,
    pub status: String,
    pub whatsapp_body: Option<String>,
    pub email_subject: Option<String>,
    pub email_body: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageTemplatePreviewRequest {
    pub whatsapp_body: Option<String>,
    pub email_subject: Option<String>,
    pub email_body: Option<String>,
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageTemplatePreviewResponse {
    pub whatsapp_body: Option<String>,
    pub email_subject: Option<String>,
    pub email_body: Option<String>,
    pub variables: Vec<String>,
}
