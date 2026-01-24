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
    pub status: String, // pending, paid, cancelled, failed
    pub description: Option<String>,
    pub due_date: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub payment_method: Option<String>,
    pub external_id: Option<String>,
    pub merchant_id: Option<String>, // NULL = System, Some = Tenant
    pub proof_attachment: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct CreateInvoiceRequest {
    pub tenant_id: String,
    pub amount: f64,
    pub description: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBankAccountRequest {
    pub bank_name: String,
    pub account_number: String,
    pub account_holder: String,
}
