#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct AssignmentSubscriptionRef {
    pub(super) customer_id: String,
    pub(super) location_id: String,
    pub(super) router_id: Option<String>,
    pub(super) latitude: Option<f64>,
    pub(super) longitude: Option<f64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct AssignmentCandidateNodeRow {
    pub(super) node_id: String,
    pub(super) name: String,
    pub(super) node_type: String,
    pub(super) status: String,
    pub(super) capacity_json: serde_json::Value,
    pub(super) health_json: serde_json::Value,
    pub(super) distance_m: Option<f64>,
    pub(super) avg_link_utilization_pct: Option<f64>,
    pub(super) down_links: i64,
    pub(super) link_count: i64,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePackageRequest {
    pub subscription_id: String,
    pub new_package_id: String,
    /// ISO 8601 date. Default: now (UTC).
    pub effective_date: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangePackageResult {
    pub subscription_id: String,
    pub old_package_name: String,
    pub new_package_name: String,
    pub old_price: f64,
    pub new_price: f64,
    /// Pro-rata credit for unused days on old package.
    pub pro_rata_credit: f64,
    /// Pro-rata charge for remaining days on new package.
    pub pro_rata_charge: f64,
    /// Net amount to charge (positive) or credit (negative).
    pub net_amount: f64,
    /// Invoice ID if a pro-rata invoice was generated.
    pub invoice_id: Option<String>,
    pub effective_date: String,
    pub billing_cycle: String,
}

// =============================================================================
// Bulk Send Invoice (Phase 3)
// =============================================================================

/// Request body for `POST /api/payment/invoices/bulk-send`.
///
/// Lets admins fan out a multi-invoice send in one call. Channels are opt-in:
/// when `channels` is omitted, both `email` and `notification` are attempted.
/// `attach_pdf` is on by default — caller must explicitly set false to send the
/// email body without a PDF.
#[derive(Debug, Deserialize)]
pub struct BulkSendInvoiceRequest {
    pub invoice_ids: Vec<String>,
    #[serde(default)]
    pub channels: Option<Vec<String>>, // ["email","notification"]; default both
    #[serde(default)]
    pub template_id: Option<String>,
    #[serde(default = "default_attach_pdf")]
    pub attach_pdf: bool,
}

fn default_attach_pdf() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct BulkSendInvoiceItemResult {
    pub invoice_id: String,
    pub invoice_number: String,
    pub status: String, // "sent" | "skipped" | "failed"
    pub email_sent: bool,
    pub notification_sent: bool,
    pub pdf_attached: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BulkSendInvoiceResult {
    pub sent_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub items: Vec<BulkSendInvoiceItemResult>,
}
