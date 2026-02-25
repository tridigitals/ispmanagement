#![allow(dead_code)]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invoice {
    pub id: String,
    pub tenant_id: String,
    pub invoice_number: String,
    #[sqlx(try_from = "f64")]
    pub amount: f64,
    pub currency_code: String,
    pub base_currency_code: String,
    pub fx_rate: Option<f64>,
    pub fx_source: Option<String>,
    pub fx_fetched_at: Option<DateTime<Utc>>,
    pub status: String, // pending, paid, cancelled, failed
    pub description: Option<String>,
    pub due_date: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub payment_method: Option<String>,
    pub external_id: Option<String>,
    pub merchant_id: Option<String>, // NULL = System, Some = Tenant
    pub proof_attachment: Option<String>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BankAccount {
    pub id: String,
    pub bank_name: String,
    pub account_number: String,
    pub account_holder: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InvoiceReminderLog {
    pub id: String,
    pub tenant_id: String,
    pub invoice_id: String,
    pub reminder_code: String,
    pub channel: String,
    pub recipient: Option<String>,
    pub status: String,
    pub detail: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BillingCollectionLog {
    pub id: String,
    pub tenant_id: String,
    pub invoice_id: String,
    pub subscription_id: Option<String>,
    pub action: String,
    pub result: String,
    pub reason: Option<String>,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BillingCollectionLogView {
    pub id: String,
    pub tenant_id: String,
    pub invoice_id: String,
    pub subscription_id: Option<String>,
    pub action: String,
    pub result: String,
    pub reason: Option<String>,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub invoice_number: Option<String>,
    pub invoice_status: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub subscription_status: Option<String>,
    pub customer_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InvoiceReminderLogView {
    pub id: String,
    pub tenant_id: String,
    pub invoice_id: String,
    pub reminder_code: String,
    pub channel: String,
    pub recipient: Option<String>,
    pub status: String,
    pub detail: Option<String>,
    pub created_at: DateTime<Utc>,
    pub invoice_number: Option<String>,
    pub invoice_status: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateInvoiceRequest {
    pub tenant_id: String,
    pub amount: f64,
    pub description: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CreateBankAccountRequest {
    pub bank_name: String,
    pub account_number: String,
    pub account_holder: String,
}
