use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhatsappProvider {
    Disabled,
    Fonnte,
    CustomHttp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WhatsappHttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsappGatewayConfig {
    pub provider: WhatsappProvider,
    pub fonnte_base_url: Option<String>,
    pub fonnte_token: Option<String>,
    pub fonnte_sender: Option<String>,
    pub custom_http: Option<WhatsappCustomHttpConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsappCustomHttpConfig {
    pub url: String,
    pub method: WhatsappHttpMethod,
    pub headers_json: Option<String>,
    pub body_template: Option<String>,
    pub success_statuses: Vec<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhatsappEventScope {
    Platform,
    Tenant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsappEventDefinition {
    pub scope: WhatsappEventScope,
    pub code: String,
    pub label: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsappTestSendRequest {
    pub phone: String,
    pub message: String,
    pub event_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsappTestSendResponse {
    pub ok: bool,
    pub provider: String,
    pub status: Option<u16>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct WhatsappDeliveryLog {
    pub id: String,
    pub tenant_id: Option<String>,
    pub scope: String,
    pub event_code: String,
    pub provider: String,
    pub recipient_user_id: Option<String>,
    pub recipient_phone: String,
    pub status: String,
    pub error_summary: Option<String>,
    pub created_at: DateTime<Utc>,
}
