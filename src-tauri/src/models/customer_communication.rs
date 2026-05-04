use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerEmailSendRequest {
    pub customer_id: String,
    pub subject: String,
    pub body: String,
    pub template_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerEmailSendResponse {
    pub ok: bool,
    pub queued: bool,
}
