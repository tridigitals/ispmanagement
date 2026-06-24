//! Payment Service - Manages invoices and bank accounts

mod core;
pub mod analytics;
pub mod dto;
mod integration;
mod mapper;
mod repository;
mod validation;

use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    BankAccount, BillingCollectionLogView, CreateBankAccountRequest, Invoice,
    InvoiceReminderLogView, PaginatedResponse,
};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Datelike, Duration, Months, Utc};
use reqwest::Client;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256, Sha512};
use uuid::Uuid;

use crate::services::subscription_lifecycle::{
    resolve_activation_status, should_disable_pppoe_for_subscription_status,
    SubscriptionLifecycleStatus,
};
use crate::services::{AuditService, NotificationService, PppoeService};

const BILLING_AUTO_SUSPEND_ENABLED_KEY: &str = "billing_auto_suspend_enabled";
const BILLING_AUTO_SUSPEND_MODE_KEY: &str = "billing_auto_suspend_mode";
const BILLING_AUTO_SUSPEND_GRACE_DAYS_KEY: &str = "billing_auto_suspend_grace_days";
const BILLING_AUTO_SUSPEND_FIXED_DAY_KEY: &str = "billing_auto_suspend_fixed_day";
const BILLING_AUTO_SUSPEND_PPPOE_ACTION_KEY: &str = "billing_auto_suspend_pppoe_action";
const BILLING_AUTO_SUSPEND_ISOLATION_POOL_KEY: &str = "billing_auto_suspend_isolation_pool";
const BILLING_AUTO_RESUME_ON_PAYMENT_KEY: &str = "billing_auto_resume_on_payment";
const BILLING_REMINDER_ENABLED_KEY: &str = "billing_reminder_enabled";
const BILLING_REMINDER_SCHEDULE_KEY: &str = "billing_reminder_schedule";
const INSTALLATION_WORK_ORDER_VISIBILITY_MODE_KEY: &str = "installation_work_order_visibility_mode";

use self::core::{
    customer_invoice_notification_action_url, customer_notification_user_ids,
    decide_midtrans_transition, is_customer_package_invoice_external_id, is_manual_payment_invoice,
    MidtransTransitionDecision, CUSTOMER_PACKAGE_INVOICE_PREFIX,
};
use self::dto::{AssignmentCandidateNodeRow, AssignmentSubscriptionRef};
use self::mapper::{filter_installation_request_user_ids, filter_owner_admin_user_ids};
#[cfg(test)]
use self::validation::is_owner_admin_or_technician_role;

fn md5_hex(input: &str) -> String {
    format!("{:x}", md5::compute(input.as_bytes()))
}

fn duitku_create_signature(
    merchant_code: &str,
    merchant_order_id: &str,
    payment_amount: i64,
    api_key: &str,
) -> String {
    md5_hex(&format!(
        "{}{}{}{}",
        merchant_code, merchant_order_id, payment_amount, api_key
    ))
}

fn duitku_callback_signature(
    merchant_code: &str,
    amount: &str,
    merchant_order_id: &str,
    api_key: &str,
) -> String {
    md5_hex(&format!(
        "{}{}{}{}",
        merchant_code, amount, merchant_order_id, api_key
    ))
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn duitku_payment_methods_signature(
    merchant_code: &str,
    payment_amount: i64,
    datetime: &str,
    api_key: &str,
) -> String {
    sha256_hex(&format!(
        "{}{}{}{}",
        merchant_code, payment_amount, datetime, api_key
    ))
}

fn parse_selected_duitku_payment_methods(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw.map(str::trim).filter(|v| !v.is_empty()) else {
        return Vec::new();
    };

    let values = if raw.starts_with('[') {
        serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
    } else {
        vec![raw.to_string()]
    };

    let mut out = Vec::new();
    for value in values {
        let code = value.trim().to_uppercase();
        if !code.is_empty() && !out.contains(&code) {
            out.push(code);
        }
    }
    out
}

fn duitku_transaction_status_code_to_invoice_status(result_code: &str) -> &'static str {
    match result_code {
        "00" => "paid",
        "02" => "failed",
        _ => "pending",
    }
}

pub(crate) fn duitku_callback_result_code_to_invoice_status(result_code: &str) -> &'static str {
    match result_code {
        "00" => "paid",
        "01" | "02" => "failed",
        _ => "pending",
    }
}

pub(crate) fn parse_auto_suspend_mode(
    value: Option<String>,
    default: AutoSuspendMode,
) -> AutoSuspendMode {
    match value
        .unwrap_or_else(|| default.as_str().to_string())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "fixed_day" => AutoSuspendMode::FixedDay,
        "grace_period" => AutoSuspendMode::GracePeriod,
        _ => default,
    }
}

pub(crate) fn parse_auto_suspend_pppoe_action(
    value: Option<String>,
    default: AutoSuspendPppoeAction,
) -> AutoSuspendPppoeAction {
    match value
        .unwrap_or_else(|| default.as_str().to_string())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "move_to_isolation_pool" => AutoSuspendPppoeAction::MoveToIsolationPool,
        "disable_secret" => AutoSuspendPppoeAction::DisableSecret,
        _ => default,
    }
}

pub(crate) fn normalize_auto_suspend_isolation_pool(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub(crate) fn clamp_auto_suspend_fixed_day(value: i64) -> i64 {
    value.clamp(1, 28)
}

pub(crate) fn auto_suspend_threshold_date(
    due_date: chrono::NaiveDate,
    mode: AutoSuspendMode,
    grace_days: i64,
    fixed_day: i64,
) -> chrono::NaiveDate {
    match mode {
        AutoSuspendMode::GracePeriod => due_date + Duration::days(grace_days.max(0)),
        AutoSuspendMode::FixedDay => {
            let target_day = clamp_auto_suspend_fixed_day(fixed_day) as u32;
            if due_date.day() <= target_day {
                due_date.with_day(target_day).unwrap_or(due_date)
            } else {
                let (year, month) = if due_date.month() == 12 {
                    (due_date.year() + 1, 1)
                } else {
                    (due_date.year(), due_date.month() + 1)
                };
                chrono::NaiveDate::from_ymd_opt(year, month, target_day).unwrap_or(due_date)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BulkGenerateInvoicesResult {
    pub created_count: u32,
    pub skipped_count: u32,
    pub failed_count: u32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct BillingCollectionRunResult {
    pub evaluated_count: u32,
    pub reminder_sent_count: u32,
    pub reminder_skipped_count: u32,
    pub suspended_count: u32,
    pub resumed_count: u32,
    pub failed_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum AutoSuspendMode {
    GracePeriod,
    FixedDay,
}

impl AutoSuspendMode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::GracePeriod => "grace_period",
            Self::FixedDay => "fixed_day",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum AutoSuspendPppoeAction {
    DisableSecret,
    MoveToIsolationPool,
}

impl AutoSuspendPppoeAction {
    fn as_str(&self) -> &'static str {
        match self {
            Self::DisableSecret => "disable_secret",
            Self::MoveToIsolationPool => "move_to_isolation_pool",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingCollectionSettings {
    pub auto_suspend_enabled: bool,
    pub auto_suspend_mode: AutoSuspendMode,
    pub auto_suspend_grace_days: i64,
    pub auto_suspend_fixed_day: i64,
    pub auto_suspend_pppoe_action: AutoSuspendPppoeAction,
    pub auto_suspend_isolation_pool: Option<String>,
    pub auto_resume_on_payment: bool,
    pub reminder_enabled: bool,
    pub reminder_schedule: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuitkuPaymentMethod {
    pub code: String,
    pub name: String,
    pub fee: Option<String>,
}

impl Default for BillingCollectionSettings {
    fn default() -> Self {
        Self {
            auto_suspend_enabled: false,
            auto_suspend_mode: AutoSuspendMode::GracePeriod,
            auto_suspend_grace_days: 3,
            auto_suspend_fixed_day: 1,
            auto_suspend_pppoe_action: AutoSuspendPppoeAction::DisableSecret,
            auto_suspend_isolation_pool: None,
            auto_resume_on_payment: true,
            reminder_enabled: true,
            reminder_schedule: vec![
                "H-3".to_string(),
                "H-1".to_string(),
                "H+1".to_string(),
                "H+3".to_string(),
            ],
        }
    }
}

#[derive(Clone)]
pub struct PaymentService {
    pool: DbPool,
    http_client: Client,
    notification_service: NotificationService,
    pppoe_service: PppoeService,
    audit_service: AuditService,
    invoice_pdf_service: crate::services::invoice_pdf_service::InvoicePdfService,
}

impl PaymentService {
    pub fn new(
        pool: DbPool,
        notification_service: NotificationService,
        pppoe_service: PppoeService,
        audit_service: AuditService,
    ) -> Self {
        Self {
            pool,
            http_client: Client::new(),
            notification_service,
            pppoe_service,
            audit_service,
            invoice_pdf_service: crate::services::invoice_pdf_service::InvoicePdfService::new(),
        }
    }

    /// Wrapper around `AuditService::log` that swallows database errors so
    /// audit-log persistence never blocks a payment-flow side effect.
    /// Builds a JSON `details` payload from a typed metadata object.
    async fn audit_log(
        &self,
        actor_user_id: Option<&str>,
        tenant_id: Option<&str>,
        action: &str,
        resource: &str,
        resource_id: Option<&str>,
        metadata: &serde_json::Value,
    ) {
        let details = metadata.to_string();
        self.audit_service
            .log(
                actor_user_id,
                tenant_id,
                action,
                resource,
                resource_id,
                Some(details.as_str()),
                None,
            )
            .await;
    }

    async fn payment_setting_for_invoice(
        &self,
        invoice: &Invoice,
        key: &str,
        default: &str,
    ) -> AppResult<String> {
        let merchant_id = invoice.merchant_id.as_deref().ok_or_else(|| {
            AppError::Configuration(format!(
                "Invoice {} has no merchant_id — tenant payment gateway not configured",
                invoice.invoice_number
            ))
        })?;

        #[cfg(feature = "postgres")]
        let tenant_value: Option<String> =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = $1 AND tenant_id = $2")
                .bind(key)
                .bind(merchant_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let tenant_value: Option<String> =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = ? AND tenant_id = ?")
                .bind(key)
                .bind(merchant_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(tenant_value.unwrap_or_else(|| default.to_string()))
    }

    async fn mark_invoice_payment_method(
        &self,
        invoice_id: &str,
        payment_method: &str,
    ) -> AppResult<()> {
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query("UPDATE invoices SET payment_method = $1, updated_at = $2 WHERE id = $3")
            .bind(payment_method)
            .bind(now)
            .bind(invoice_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE invoices SET payment_method = ?, updated_at = ? WHERE id = ?")
            .bind(payment_method)
            .bind(now.to_rfc3339())
            .bind(invoice_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(())
    }

    pub fn start_customer_invoice_scheduler(&self) {
        let svc = self.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = svc
                    .generate_due_customer_package_invoices_for_all_tenants()
                    .await
                {
                    tracing::warn!("customer invoice scheduler failed: {}", e);
                }
                if let Err(e) = svc.run_billing_collection_for_all_tenants().await {
                    tracing::warn!("billing collection scheduler failed: {}", e);
                }
                let interval_minutes = svc.resolve_scheduler_interval_minutes().await;
                let sleep_secs = (interval_minutes.max(5) as u64) * 60;
                tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
            }
        });
    }

    // ==================== INVOICES ====================

    /// Create a new invoice
    pub async fn create_invoice(
        &self,
        tenant_id: &str,
        amount: f64,
        description: Option<String>,
        external_id: Option<String>,
    ) -> AppResult<Invoice> {
        self.create_invoice_with_due_date(
            tenant_id,
            amount,
            description,
            external_id,
            Utc::now() + chrono::Duration::days(1),
        )
        .await
    }

    async fn create_invoice_with_due_date(
        &self,
        tenant_id: &str,
        amount: f64,
        description: Option<String>,
        external_id: Option<String>,
        due_date: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<Invoice> {
        let now = Utc::now();

        // Base currency for pricing (global) and tenant display currency.
        let base_currency_code = self
            .get_setting_value(None, "base_currency_code")
            .await
            .unwrap_or_else(|| "IDR".to_string())
            .to_uppercase();

        let currency_code = self
            .get_setting_value(Some(tenant_id), "currency_code")
            .await
            .unwrap_or_else(|| base_currency_code.clone())
            .to_uppercase();

        let (final_amount, fx_rate, fx_source, fx_fetched_at) =
            if currency_code != base_currency_code {
                let (rate, fetched_at, source) = self
                    .get_fx_rate(&base_currency_code, &currency_code, Some(tenant_id))
                    .await?;
                let converted = amount * rate;
                (
                    self.round_amount(converted, &currency_code),
                    Some(rate),
                    Some(source),
                    Some(fetched_at),
                )
            } else {
                (self.round_amount(amount, &currency_code), None, None, None)
            };

        #[cfg(feature = "postgres")]
        let invoice = {
            // HIGH #3 (MVP DoD audit): the previous `INV-{YYYYMMDD-HHMMSS}`
            // format collides at second granularity when the scheduler and a
            // manual create race within the same second. We now build the
            // number from a Postgres SEQUENCE and rely on the composite
            // unique index `(tenant_id, invoice_number)` as a structural
            // safety net. Postgres `nextval()` itself is atomic and
            // concurrency-safe, so two writers will never receive the same
            // sequence value. The retry loop is a safety net for paths that
            // pre-claim invoice numbers out-of-band (tenant-specific
            // numbering schemes or recovery scripts using `setval`).
            const MAX_ATTEMPTS: u32 = 3;
            let mut last_error: Option<sqlx::Error> = None;
            let mut inserted: Option<Invoice> = None;
            for attempt in 0..MAX_ATTEMPTS {
                let invoice_number = self.next_invoice_number(now).await?;
                let id = Uuid::new_v4().to_string();
                let res = sqlx::query_as::<_, Invoice>(
                    r#"
                    INSERT INTO invoices (
                        id, tenant_id, invoice_number, amount, currency_code, base_currency_code, fx_rate, fx_source, fx_fetched_at,
                        status, description, due_date, external_id, merchant_id, created_at, updated_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending', $10, $11, $12, $13, $14, $14)
                    RETURNING
                        id, tenant_id, invoice_number,
                        amount::FLOAT8 as amount,
                        currency_code, base_currency_code,
                        fx_rate::FLOAT8 as fx_rate, fx_source, fx_fetched_at,
                        status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, rejection_reason, created_at, updated_at
                    "#
                )
                .bind(&id)
                .bind(tenant_id)
                .bind(&invoice_number)
                .bind(final_amount)
                .bind(&currency_code)
                .bind(&base_currency_code)
                .bind(fx_rate)
                .bind(&fx_source)
                .bind(fx_fetched_at)
                .bind(&description)
                .bind(due_date)
                .bind(&external_id)
                .bind(tenant_id)
                .bind(now)
                .fetch_one(&self.pool)
                .await;

                match res {
                    Ok(inv) => {
                        inserted = Some(inv);
                        break;
                    }
                    Err(err) => {
                        // Be strict: only retry when sqlx can confirm the
                        // 23505 originated from one of our invoice_number
                        // unique constraints. If `constraint()` returns
                        // `None`, that's an unexpected error condition
                        // (different unique constraint, or sqlx behavior
                        // change) — surface it instead of papering over.
                        let is_invoice_unique_conflict = err
                            .as_database_error()
                            .map(|db| {
                                db.code().as_deref() == Some("23505")
                                    && db
                                        .constraint()
                                        .map(|c| {
                                            c == "idx_invoices_tenant_invoice_number"
                                                || c == "invoices_invoice_number_key"
                                        })
                                        .unwrap_or(false)
                            })
                            .unwrap_or(false);
                        if is_invoice_unique_conflict && attempt + 1 < MAX_ATTEMPTS {
                            tracing::warn!(
                                tenant_id = %tenant_id,
                                attempt = attempt + 1,
                                "invoice_number unique conflict, retrying with fresh sequence value"
                            );
                            last_error = Some(err);
                            continue;
                        }
                        return Err(AppError::Internal(err.to_string()));
                    }
                }
            }
            inserted.ok_or_else(|| {
                AppError::Internal(format!(
                    "create_invoice exceeded {MAX_ATTEMPTS} retry attempts; last error: {}",
                    last_error
                        .as_ref()
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ))
            })?
        };

        #[cfg(feature = "sqlite")]
        let invoice = {
            // SQLite path is single-tenant standalone (desktop/dev); the
            // burst-collision concern from MVP DoD HIGH #3 does not apply,
            // but we still avoid HHMMSS-only collisions by suffixing with
            // a UUID short to keep numbers unique without introducing a
            // sequence dependency that SQLite doesn't have.
            let id = Uuid::new_v4().to_string();
            let invoice_number = format!(
                "INV-{}-{}",
                now.format("%Y%m%d-%H%M%S"),
                &id.replace('-', "")[..8]
            );
            sqlx::query(
                r#"
                INSERT INTO invoices (
                    id, tenant_id, invoice_number, amount, currency_code, base_currency_code, fx_rate, fx_source, fx_fetched_at,
                    status, description, due_date, external_id, merchant_id, created_at, updated_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(&invoice_number)
            .bind(final_amount)
            .bind(&currency_code)
            .bind(&base_currency_code)
            .bind(fx_rate)
            .bind(&fx_source)
            .bind(fx_fetched_at.map(|d| d.to_rfc3339()))
            .bind(&description)
            .bind(due_date.to_rfc3339())
            .bind(&external_id)
            .bind(tenant_id)
            .bind(now.to_rfc3339()).bind(now.to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            self.get_invoice(&id).await?
        };

        Ok(invoice)
    }

    /// Build the next invoice number using the Postgres `invoice_number_seq`
    /// sequence. Returns `INV-YYYYMMDD-NNNNNN` (zero-padded, 6-digit suffix).
    /// The sequence is monotonic and global, not per-day. See the migration
    /// `20260529082913_invoice_number_uniqueness` for rationale.
    #[cfg(feature = "postgres")]
    async fn next_invoice_number(&self, now: chrono::DateTime<chrono::Utc>) -> AppResult<String> {
        let seq: i64 = sqlx::query_scalar("SELECT nextval('invoice_number_seq')")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(format!("INV-{}-{:06}", now.format("%Y%m%d"), seq))
    }

    /// Get invoice by ID
    pub async fn get_invoice(&self, id: &str) -> AppResult<Invoice> {
        #[cfg(feature = "postgres")]
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT
                id, tenant_id, invoice_number,
                amount::FLOAT8 as amount,
                currency_code, base_currency_code,
                COALESCE(fx_rate, 1.0)::FLOAT8 as fx_rate, fx_source, fx_fetched_at,
                status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, rejection_reason, created_at, updated_at
            FROM invoices WHERE id = $1
            "#
        )
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| AppError::NotFound("Invoice not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let invoice = sqlx::query_as("SELECT * FROM invoices WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| AppError::NotFound("Invoice not found".to_string()))?;

        Ok(invoice)
    }

    async fn get_invoice_by_number(&self, invoice_number: &str) -> AppResult<Invoice> {
        #[cfg(feature = "postgres")]
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT
                id, tenant_id, invoice_number,
                amount::FLOAT8 as amount,
                currency_code, base_currency_code,
                COALESCE(fx_rate, 1.0)::FLOAT8 as fx_rate, fx_source, fx_fetched_at,
                status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, rejection_reason, created_at, updated_at
            FROM invoices WHERE invoice_number = $1
            "#,
        )
        .bind(invoice_number)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| AppError::NotFound("Invoice not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let invoice = sqlx::query_as("SELECT * FROM invoices WHERE invoice_number = ?")
            .bind(invoice_number)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| AppError::NotFound("Invoice not found".to_string()))?;

        Ok(invoice)
    }

    /// List invoices with optional tenant filter
    pub async fn list_invoices(&self, tenant_id: Option<&str>) -> AppResult<Vec<Invoice>> {
        #[cfg(feature = "postgres")]
        let invoices = if let Some(tid) = tenant_id {
            sqlx::query_as::<_, Invoice>(
                r#"
                SELECT
                    id, tenant_id, invoice_number,
                    amount::FLOAT8 as amount,
                    currency_code, base_currency_code,
                    COALESCE(fx_rate, 1.0)::FLOAT8 as fx_rate, fx_source, fx_fetched_at,
                    status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, rejection_reason, created_at, updated_at
                FROM invoices
                WHERE tenant_id = $1
                  AND (external_id IS NULL OR external_id NOT LIKE 'pkgsub:%')
                ORDER BY created_at DESC
                "#
            )
            .bind(tid)
            .fetch_all(&self.pool).await
        } else {
            sqlx::query_as::<_, Invoice>(
                r#"
                SELECT
                    id, tenant_id, invoice_number,
                    amount::FLOAT8 as amount,
                    currency_code, base_currency_code,
                    COALESCE(fx_rate, 1.0)::FLOAT8 as fx_rate, fx_source, fx_fetched_at,
                    status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, rejection_reason, created_at, updated_at
                FROM invoices
                WHERE external_id IS NULL OR external_id NOT LIKE 'pkgsub:%'
                ORDER BY created_at DESC
                "#
            )
            .fetch_all(&self.pool).await
        }.map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let invoices = if let Some(tid) = tenant_id {
            sqlx::query_as::<_, Invoice>(
                "SELECT * FROM invoices WHERE tenant_id = ? AND (external_id IS NULL OR external_id NOT LIKE 'pkgsub:%') ORDER BY created_at DESC",
            )
            .bind(tid)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, Invoice>(
                "SELECT * FROM invoices WHERE external_id IS NULL OR external_id NOT LIKE 'pkgsub:%' ORDER BY created_at DESC",
            )
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(invoices)
    }

    pub async fn list_customer_package_invoices(
        &self,
        tenant_id: &str,
        sort_by: Option<String>,
        sort_dir: Option<String>,
        page: u32,
        per_page: u32,
    ) -> AppResult<PaginatedResponse<Invoice>> {
        let page = page.max(1);
        let per_page = per_page.clamp(1, 100);
        let offset = ((page - 1) * per_page) as i64;
        let per_page_i64 = per_page as i64;
        let sort_column = match sort_by
            .unwrap_or_else(|| "created_at".to_string())
            .trim()
            .to_lowercase()
            .as_str()
        {
            "invoice_number" => "invoice_number",
            "description" => "description",
            "amount" => "amount",
            "status" => "status",
            "due_date" => "due_date",
            "created_at" => "created_at",
            _ => "created_at",
        };
        let sort_direction = match sort_dir
            .unwrap_or_else(|| "desc".to_string())
            .trim()
            .to_lowercase()
            .as_str()
        {
            "asc" => "ASC",
            _ => "DESC",
        };
        let prefix = format!("{}%", core::CUSTOMER_PACKAGE_INVOICE_PREFIX);

        #[cfg(feature = "postgres")]
        let total: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM invoices WHERE tenant_id = $1 AND external_id LIKE $2"#,
        )
        .bind(tenant_id)
        .bind(&prefix)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "postgres")]
        let invoices = sqlx::query_as::<_, Invoice>(&format!(
            r#"
            SELECT
                id, tenant_id, invoice_number,
                amount::FLOAT8 as amount,
                currency_code, base_currency_code,
                COALESCE(fx_rate, 1.0)::FLOAT8 as fx_rate, fx_source, fx_fetched_at,
                status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, rejection_reason, created_at, updated_at
            FROM invoices
            WHERE tenant_id = $1 AND external_id LIKE $2
            ORDER BY {sort_column} {sort_direction}
            LIMIT $3 OFFSET $4
            "#,
        ))
        .bind(tenant_id)
        .bind(&prefix)
        .bind(per_page_i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM invoices WHERE tenant_id = ? AND external_id LIKE ?",
        )
        .bind(tenant_id)
        .bind(&prefix)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let invoices = sqlx::query_as::<_, Invoice>(&format!(
            "SELECT * FROM invoices WHERE tenant_id = ? AND external_id LIKE ? ORDER BY {sort_column} {sort_direction} LIMIT ? OFFSET ?"
        ))
        .bind(tenant_id)
        .bind(&prefix)
        .bind(per_page_i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(PaginatedResponse {
            data: invoices,
            total,
            page,
            per_page,
        })
    }

    pub async fn list_customer_portal_invoices(
        &self,
        tenant_id: &str,
        customer_id: &str,
    ) -> AppResult<Vec<Invoice>> {
        #[cfg(feature = "postgres")]
        let invoices = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT
                i.id, i.tenant_id, i.invoice_number,
                i.amount::FLOAT8 as amount,
                i.currency_code, i.base_currency_code,
                COALESCE(i.fx_rate, 1.0)::FLOAT8 as fx_rate, i.fx_source, i.fx_fetched_at,
                i.status, i.description, i.due_date, i.paid_at, i.payment_method, i.proof_attachment, i.external_id, i.merchant_id, i.rejection_reason, i.created_at, i.updated_at
            FROM invoices i
            INNER JOIN customer_subscriptions cs
              ON cs.tenant_id = i.tenant_id
             AND (
                i.external_id = 'pkgsub:' || cs.id
                OR i.external_id LIKE 'pkgsub:' || cs.id || ':%'
             )
            WHERE i.tenant_id = $1
              AND cs.customer_id = $2
            ORDER BY i.created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let invoices = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT i.*
            FROM invoices i
            INNER JOIN customer_subscriptions cs
              ON cs.tenant_id = i.tenant_id
             AND (
                i.external_id = 'pkgsub:' || cs.id
                OR i.external_id LIKE 'pkgsub:' || cs.id || ':%'
             )
            WHERE i.tenant_id = ?
              AND cs.customer_id = ?
            ORDER BY i.created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(invoices)
    }

    pub async fn customer_owns_package_invoice(
        &self,
        tenant_id: &str,
        customer_id: &str,
        invoice_id: &str,
    ) -> AppResult<bool> {
        #[cfg(feature = "postgres")]
        let owns: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM invoices i
              INNER JOIN customer_subscriptions cs
                ON cs.tenant_id = i.tenant_id
               AND (
                  i.external_id = 'pkgsub:' || cs.id
                  OR i.external_id LIKE 'pkgsub:' || cs.id || ':%'
               )
              WHERE i.id = $1
                AND i.tenant_id = $2
                AND cs.customer_id = $3
            )
            "#,
        )
        .bind(invoice_id)
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let owns: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM invoices i
              INNER JOIN customer_subscriptions cs
                ON cs.tenant_id = i.tenant_id
               AND (
                  i.external_id = 'pkgsub:' || cs.id
                  OR i.external_id LIKE 'pkgsub:' || cs.id || ':%'
               )
              WHERE i.id = ?
                AND i.tenant_id = ?
                AND cs.customer_id = ?
            )
            "#,
        )
        .bind(invoice_id)
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(owns)
    }

    /// Change a customer's subscription package mid-cycle with pro-rata billing.
    /// Calculates credit for unused days on old package and charge for remaining days on new package.
    pub async fn change_subscription_package(
        &self,
        tenant_id: &str,
        request: dto::ChangePackageRequest,
    ) -> AppResult<dto::ChangePackageResult> {
        use chrono::{DateTime, Utc};

        // Parse effective date or use now
        let effective_date = match &request.effective_date {
            Some(d) => DateTime::parse_from_rfc3339(d)
                .or_else(|_| {
                    DateTime::parse_from_str(
                        &format!("{}T00:00:00+00:00", d),
                        "%Y-%m-%dT%H:%M:%S%:z",
                    )
                })
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| AppError::Validation("Invalid effective_date format".to_string()))?,
            None => Utc::now(),
        };

        // 1. Get current subscription with package info
        #[cfg(feature = "postgres")]
        let sub_row: Option<(
            String,
            String,
            String,
            f64,
            Option<DateTime<Utc>>,
            Option<DateTime<Utc>>,
        )> = sqlx::query_as(
            r#"
            SELECT
                cs.billing_cycle,
                COALESCE(p.name, 'Package') AS package_name,
                cs.package_id,
                cs.price::FLOAT8 AS price,
                cs.starts_at,
                cs.ends_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p ON p.id = cs.package_id AND p.tenant_id = cs.tenant_id
            WHERE cs.id = $1 AND cs.tenant_id = $2 AND cs.status = 'active'
            LIMIT 1
            "#,
        )
        .bind(&request.subscription_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let sub_row: Option<(
            String,
            String,
            String,
            f64,
            Option<DateTime<Utc>>,
            Option<DateTime<Utc>>,
        )> = sqlx::query_as(
            r#"
            SELECT
                cs.billing_cycle,
                COALESCE(p.name, 'Package') AS package_name,
                cs.package_id,
                cs.price AS price,
                cs.starts_at,
                cs.ends_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p ON p.id = cs.package_id AND p.tenant_id = cs.tenant_id
            WHERE cs.id = ? AND cs.tenant_id = ? AND cs.status = 'active'
            LIMIT 1
            "#,
        )
        .bind(&request.subscription_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (billing_cycle, old_package_name, old_package_id, old_price, starts_at, ends_at) =
            sub_row
                .ok_or_else(|| AppError::NotFound("Active subscription not found".to_string()))?;

        // Don't allow changing to the same package
        if old_package_id == request.new_package_id {
            return Err(AppError::Validation(
                "New package is the same as current package".to_string(),
            ));
        }

        // Check subscription hasn't ended
        if let Some(ends) = ends_at {
            if effective_date > ends {
                return Err(AppError::Validation(
                    "Subscription already ended".to_string(),
                ));
            }
        }

        // 2. Get new package price
        #[cfg(feature = "postgres")]
        let new_pkg: Option<(String, f64)> = sqlx::query_as(
            r#"
            SELECT name,
                CASE WHEN $3 = 'yearly' THEN price_yearly::FLOAT8 ELSE price_monthly::FLOAT8 END AS price
            FROM isp_packages
            WHERE id = $1 AND tenant_id = $2 AND is_active = true
            LIMIT 1
            "#,
        )
        .bind(&request.new_package_id)
        .bind(tenant_id)
        .bind(&billing_cycle)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let new_pkg: Option<(String, f64)> = sqlx::query_as(
            r#"
            SELECT name,
                CASE WHEN ?3 = 'yearly' THEN price_yearly ELSE price_monthly END AS price
            FROM isp_packages
            WHERE id = ?1 AND tenant_id = ?2 AND is_active = true
            LIMIT 1
            "#,
        )
        .bind(&request.new_package_id)
        .bind(tenant_id)
        .bind(&billing_cycle)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (new_package_name, new_price) = new_pkg
            .ok_or_else(|| AppError::NotFound("New package not found or inactive".to_string()))?;

        // 3. Calculate current billing period
        let anchor = starts_at.unwrap_or(effective_date);
        let (period_start, period_end) =
            Self::current_billing_period(&billing_cycle, anchor, effective_date)?;

        // 4. Calculate pro-rata amounts
        let days_remaining = (period_end - effective_date).num_days().max(0) as f64;
        let total_days = (period_end - period_start).num_days().max(1) as f64;

        // Credit: refund unused portion of old package
        let pro_rata_credit = (old_price * days_remaining / total_days * 100.0).round() / 100.0;
        // Charge: new package for remaining days
        let pro_rata_charge = (new_price * days_remaining / total_days * 100.0).round() / 100.0;
        let net_amount = ((pro_rata_charge - pro_rata_credit) * 100.0).round() / 100.0;

        // 5. Update subscription package and price
        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE customer_subscriptions
            SET package_id = $1, price = $2, updated_at = NOW()
            WHERE id = $3 AND tenant_id = $4
            "#,
        )
        .bind(&request.new_package_id)
        .bind(new_price)
        .bind(&request.subscription_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE customer_subscriptions
            SET package_id = ?1, price = ?2, updated_at = datetime('now')
            WHERE id = ?3 AND tenant_id = ?4
            "#,
        )
        .bind(&request.new_package_id)
        .bind(new_price)
        .bind(&request.subscription_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // 6. Create pro-rata invoice if net amount > 0
        let invoice_id = if net_amount > 0.0 {
            let description = format!(
                "Pro-rata upgrade: {} → {} ({} remaining days)",
                old_package_name, new_package_name, days_remaining as i64
            );
            let external_id = format!(
                "PRATA:{}:{}:{}",
                request.subscription_id,
                effective_date.format("%Y%m%d"),
                uuid::Uuid::new_v4()
            );
            let invoice = self
                .create_invoice_with_due_date(
                    tenant_id,
                    net_amount,
                    Some(description),
                    Some(external_id),
                    Utc::now() + chrono::Duration::days(7),
                )
                .await?;
            Some(invoice.id)
        } else {
            // If credit > charge, we could create a credit note, but for now just skip
            None
        };

        Ok(dto::ChangePackageResult {
            subscription_id: request.subscription_id,
            old_package_name,
            new_package_name,
            old_price,
            new_price,
            pro_rata_credit,
            pro_rata_charge,
            net_amount,
            invoice_id,
            effective_date: effective_date.to_rfc3339(),
            billing_cycle,
        })
    }

    pub async fn create_invoice_for_customer_subscription(
        &self,
        tenant_id: &str,
        subscription_id: &str,
    ) -> AppResult<Invoice> {
        self.create_invoice_for_customer_subscription_at(tenant_id, subscription_id, Utc::now())
            .await
    }

    pub async fn create_invoice_for_installation_work_order(
        &self,
        tenant_id: &str,
        work_order_id: &str,
    ) -> AppResult<Invoice> {
        #[cfg(feature = "postgres")]
        let subscription_id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT subscription_id
            FROM installation_work_orders
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let subscription_id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT subscription_id
            FROM installation_work_orders
            WHERE tenant_id = ? AND id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let subscription_id = subscription_id
            .ok_or_else(|| AppError::NotFound("Installation work order not found".to_string()))?;

        self.create_invoice_for_customer_subscription(tenant_id, &subscription_id)
            .await
    }

    async fn create_invoice_for_customer_subscription_at(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        period_ref: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<Invoice> {
        self.create_invoice_for_customer_subscription_at_with_due_date(
            tenant_id,
            subscription_id,
            period_ref,
            None,
        )
        .await
    }

    pub async fn create_bootstrap_invoice_for_customer_subscription(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        period_ref: chrono::DateTime<chrono::Utc>,
        due_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> AppResult<Invoice> {
        self.create_invoice_for_customer_subscription_at_with_due_date(
            tenant_id,
            subscription_id,
            period_ref,
            due_date,
        )
        .await
    }

    async fn create_invoice_for_customer_subscription_at_with_due_date(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        period_ref: chrono::DateTime<chrono::Utc>,
        due_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> AppResult<Invoice> {
        #[cfg(feature = "postgres")]
        let row: Option<(
            String,
            String,
            String,
            String,
            f64,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        )> = sqlx::query_as(
            r#"
            SELECT
                cs.customer_id,
                c.name AS customer_name,
                COALESCE(p.name, 'Package') AS package_name,
                cs.billing_cycle,
                cs.price::FLOAT8 AS price,
                cs.starts_at,
                cs.ends_at
            FROM customer_subscriptions cs
            INNER JOIN customers c ON c.id = cs.customer_id AND c.tenant_id = cs.tenant_id
            LEFT JOIN isp_packages p ON p.id = cs.package_id AND p.tenant_id = cs.tenant_id
            WHERE cs.id = $1 AND cs.tenant_id = $2
            LIMIT 1
            "#,
        )
        .bind(subscription_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let row: Option<(
            String,
            String,
            String,
            String,
            f64,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        )> = sqlx::query_as(
            r#"
            SELECT
                cs.customer_id,
                c.name AS customer_name,
                COALESCE(p.name, 'Package') AS package_name,
                cs.billing_cycle,
                cs.price AS price,
                cs.starts_at,
                cs.ends_at
            FROM customer_subscriptions cs
            INNER JOIN customers c ON c.id = cs.customer_id AND c.tenant_id = cs.tenant_id
            LEFT JOIN isp_packages p ON p.id = cs.package_id AND p.tenant_id = cs.tenant_id
            WHERE cs.id = ? AND cs.tenant_id = ?
            LIMIT 1
            "#,
        )
        .bind(subscription_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (customer_id, customer_name, package_name, billing_cycle, price, starts_at, ends_at) =
            row.ok_or_else(|| AppError::NotFound("Customer subscription not found".to_string()))?;
        if let Some(ends) = ends_at {
            if period_ref > ends {
                return Err(AppError::Validation(
                    "Subscription already ended".to_string(),
                ));
            }
        }

        let period_key = Self::billing_period_key(&billing_cycle, starts_at.as_ref(), period_ref)?;
        let external_id = format!(
            "{}{}:{}",
            core::CUSTOMER_PACKAGE_INVOICE_PREFIX,
            subscription_id,
            period_key
        );

        #[cfg(feature = "postgres")]
        let existing_current_period: Option<Invoice> = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT
                id, tenant_id, invoice_number,
                amount::FLOAT8 as amount,
                currency_code, base_currency_code,
                COALESCE(fx_rate, 1.0)::FLOAT8 as fx_rate, fx_source, fx_fetched_at,
                status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, rejection_reason, created_at, updated_at
            FROM invoices
            WHERE tenant_id = $1
              AND external_id = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&external_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let existing_current_period: Option<Invoice> = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT *
            FROM invoices
            WHERE tenant_id = ?
              AND external_id = ?
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&external_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Idempotent checkout: reuse existing invoice for the same billing period.
        if let Some(existing) = existing_current_period {
            return Ok(existing);
        }

        let description = format!(
            "Customer {} - {} ({} billing, period {})",
            customer_name, package_name, billing_cycle, period_key
        );

        let invoice = self
            .create_invoice_with_due_date(
                tenant_id,
                price,
                Some(description),
                Some(external_id),
                due_date.unwrap_or_else(|| Utc::now() + chrono::Duration::days(1)),
            )
            .await?;

        // Ensure customer_service_assignments row exists so bulk-send and
        // other invoice→customer resolution paths work immediately.
        self.ensure_customer_service_assignment_for_invoice(
            tenant_id,
            &invoice.id,
            subscription_id,
            &customer_id,
        )
        .await;

        if let Err(err) = self
            .notify_subscription_invoice_created(
                tenant_id,
                subscription_id,
                &invoice.id,
                &invoice.invoice_number,
                invoice.amount,
                &invoice.currency_code,
            )
            .await
        {
            tracing::warn!(
                "failed to send invoice-created notification: tenant={}, subscription={}, invoice={}, error={}",
                tenant_id,
                subscription_id,
                invoice.invoice_number,
                err
            );
        }

        Ok(invoice)
    }

    pub async fn generate_due_customer_package_invoices(
        &self,
        tenant_id: &str,
    ) -> AppResult<BulkGenerateInvoicesResult> {
        let lead_raw = match self
            .get_setting_value(Some(tenant_id), "customer_invoice_generate_days_before_due")
            .await
        {
            Some(v) => Some(v),
            None => {
                self.get_setting_value(None, "customer_invoice_generate_days_before_due")
                    .await
            }
        };
        let lead_days = lead_raw
            .and_then(|v| v.parse::<i64>().ok())
            .map(|v| v.clamp(0, 60))
            .unwrap_or(7);
        let lead_duration = Duration::days(lead_days);
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let subscriptions: Vec<(
            String,
            String,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        )> = sqlx::query_as(
            r#"
            SELECT cs.id, cs.billing_cycle, cs.starts_at, cs.ends_at
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = $1
              AND cs.status = 'active'
              AND (cs.starts_at IS NULL OR cs.starts_at <= NOW())
              AND (cs.ends_at IS NULL OR cs.ends_at >= NOW())
            ORDER BY cs.created_at ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let subscriptions: Vec<(
            String,
            String,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        )> = sqlx::query_as(
            r#"
            SELECT cs.id, cs.billing_cycle, cs.starts_at, cs.ends_at
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = ?
              AND cs.status = 'active'
              AND (cs.starts_at IS NULL OR cs.starts_at <= ?)
              AND (cs.ends_at IS NULL OR cs.ends_at >= ?)
            ORDER BY cs.created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let mut created_count = 0_u32;
        let mut skipped_count = 0_u32;
        let mut failed_count = 0_u32;

        for (subscription_id, billing_cycle, starts_at, ends_at) in subscriptions {
            if let Some(next_renewal) =
                Self::next_renewal_at(&billing_cycle, starts_at.as_ref(), now)?
            {
                if now < (next_renewal - lead_duration) {
                    skipped_count += 1;
                    continue;
                }
                if let Some(ends) = ends_at {
                    if next_renewal > ends {
                        skipped_count += 1;
                        continue;
                    }
                }
                match self
                    .create_invoice_for_customer_subscription_at(
                        tenant_id,
                        &subscription_id,
                        next_renewal,
                    )
                    .await
                {
                    Ok(_) => created_count += 1,
                    Err(AppError::Validation(_)) => skipped_count += 1,
                    Err(_) => failed_count += 1,
                }
                continue;
            }

            match self
                .create_invoice_for_customer_subscription_at(tenant_id, &subscription_id, now)
                .await
            {
                Ok(_) => created_count += 1,
                Err(AppError::Validation(_)) => skipped_count += 1,
                Err(_) => failed_count += 1,
            }
        }

        let _ = self
            .upsert_tenant_setting(
                tenant_id,
                "customer_invoice_last_run_at",
                &now.to_rfc3339(),
                "Last customer invoice generation run timestamp (UTC)",
            )
            .await;

        Ok(BulkGenerateInvoicesResult {
            created_count,
            skipped_count,
            failed_count,
        })
    }

    pub async fn generate_due_customer_package_invoices_for_all_tenants(
        &self,
    ) -> AppResult<BulkGenerateInvoicesResult> {
        #[cfg(feature = "postgres")]
        let tenant_ids: Vec<String> =
            sqlx::query_scalar("SELECT id FROM tenants WHERE is_active = true")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let tenant_ids: Vec<String> =
            sqlx::query_scalar("SELECT id FROM tenants WHERE is_active = 1")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

        let mut created_count = 0_u32;
        let mut skipped_count = 0_u32;
        let mut failed_count = 0_u32;

        for tenant_id in tenant_ids {
            let enabled = match self
                .get_setting_value(Some(&tenant_id), "customer_invoice_auto_generate_enabled")
                .await
            {
                Some(v) => v == "true",
                None => self
                    .get_setting_value(None, "customer_invoice_auto_generate_enabled")
                    .await
                    .map(|v| v == "true")
                    .unwrap_or(true),
            };
            if !enabled {
                continue;
            }

            match self
                .generate_due_customer_package_invoices(&tenant_id)
                .await
            {
                Ok(res) => {
                    created_count += res.created_count;
                    skipped_count += res.skipped_count;
                    failed_count += res.failed_count;
                }
                Err(e) => {
                    tracing::warn!(
                        "customer invoice scheduler tenant {} failed: {}",
                        tenant_id,
                        e
                    );
                    failed_count += 1;
                }
            }
        }

        Ok(BulkGenerateInvoicesResult {
            created_count,
            skipped_count,
            failed_count,
        })
    }

    pub async fn run_billing_collection_for_all_tenants(
        &self,
    ) -> AppResult<BillingCollectionRunResult> {
        #[cfg(feature = "postgres")]
        let tenant_ids: Vec<String> =
            sqlx::query_scalar("SELECT id FROM tenants WHERE is_active = true")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let tenant_ids: Vec<String> =
            sqlx::query_scalar("SELECT id FROM tenants WHERE is_active = 1")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

        let mut total = BillingCollectionRunResult::default();

        for tenant_id in tenant_ids {
            match self.run_billing_collection_for_tenant(&tenant_id).await {
                Ok(partial) => Self::merge_collection_result(&mut total, &partial),
                Err(e) => {
                    tracing::warn!("billing collection tenant {} failed: {}", tenant_id, e);
                    total.failed_count += 1;
                }
            }
        }

        Ok(total)
    }

    async fn run_billing_collection_for_tenant(
        &self,
        tenant_id: &str,
    ) -> AppResult<BillingCollectionRunResult> {
        let settings = self
            .resolve_billing_collection_settings(Some(tenant_id))
            .await;

        if !settings.reminder_enabled && !settings.auto_suspend_enabled {
            return Ok(BillingCollectionRunResult::default());
        }

        let mut result = BillingCollectionRunResult::default();
        let now = Utc::now();
        let today = now.date_naive();

        if settings.auto_suspend_enabled {
            #[cfg(feature = "postgres")]
            let grace_expired: Vec<(String, Option<String>)> = sqlx::query_as(
                r#"
                SELECT cs.id, (
                    SELECT i.id
                    FROM invoices i
                    WHERE i.tenant_id = cs.tenant_id
                      AND i.external_id LIKE ('pkgsub:' || cs.id || ':%')
                    ORDER BY i.created_at DESC
                    LIMIT 1
                ) AS invoice_id
                FROM customer_subscriptions cs
                WHERE cs.tenant_id = $1
                  AND cs.status = 'grace_active'
                  AND cs.grace_until IS NOT NULL
                  AND cs.grace_until <= $2
                  AND NOT EXISTS (
                      SELECT 1
                      FROM invoices i
                      WHERE i.tenant_id = cs.tenant_id
                        AND i.external_id LIKE ('pkgsub:' || cs.id || ':%')
                        AND i.status = 'paid'
                  )
                "#,
            )
            .bind(tenant_id)
            .bind(now)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            #[cfg(feature = "sqlite")]
            let grace_expired: Vec<(String, Option<String>)> = sqlx::query_as(
                r#"
                SELECT cs.id, (
                    SELECT i.id
                    FROM invoices i
                    WHERE i.tenant_id = cs.tenant_id
                      AND i.external_id LIKE ('pkgsub:' || cs.id || ':%')
                    ORDER BY i.created_at DESC
                    LIMIT 1
                ) AS invoice_id
                FROM customer_subscriptions cs
                WHERE cs.tenant_id = ?
                  AND cs.status = 'grace_active'
                  AND cs.grace_until IS NOT NULL
                  AND cs.grace_until <= ?
                  AND NOT EXISTS (
                      SELECT 1
                      FROM invoices i
                      WHERE i.tenant_id = cs.tenant_id
                        AND i.external_id LIKE ('pkgsub:' || cs.id || ':%')
                        AND i.status = 'paid'
                  )
                "#,
            )
            .bind(tenant_id)
            .bind(now.to_rfc3339())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            for (subscription_id, invoice_id) in grace_expired {
                match self
                    .update_customer_subscription_status_if(
                        tenant_id,
                        &subscription_id,
                        "grace_active",
                        "suspended",
                    )
                    .await
                {
                    Ok(true) => {
                        result.suspended_count += 1;
                        let _ = self
                            .apply_subscription_pppoe_billing_state(
                                tenant_id,
                                &subscription_id,
                                "suspended",
                            )
                            .await;
                        self.audit_log(
                            None,
                            Some(tenant_id),
                            "subscription.auto_suspended",
                            "subscription",
                            Some(&subscription_id),
                            &json!({
                                "reason": "grace_period_overdue",
                                "invoice_id": invoice_id,
                                "trigger": "grace_expire",
                            }),
                        )
                        .await;
                        if let Some(invoice_id) = invoice_id.as_deref() {
                            let _ = self
                                .insert_billing_collection_log(
                                    tenant_id,
                                    invoice_id,
                                    Some(&subscription_id),
                                    "grace_expire_suspend",
                                    "success",
                                    Some("Grace activation expired without paid first invoice"),
                                    "system",
                                    None,
                                )
                                .await;
                        }
                    }
                    Ok(false) => {
                        if let Some(invoice_id) = invoice_id.as_deref() {
                            let _ = self
                                .insert_billing_collection_log(
                                    tenant_id,
                                    invoice_id,
                                    Some(&subscription_id),
                                    "grace_expire_suspend",
                                    "skipped",
                                    Some("Subscription already changed from grace_active"),
                                    "system",
                                    None,
                                )
                                .await;
                        }
                    }
                    Err(e) => {
                        result.failed_count += 1;
                        let err_text = e.to_string();
                        if let Some(invoice_id) = invoice_id.as_deref() {
                            let _ = self
                                .insert_billing_collection_log(
                                    tenant_id,
                                    invoice_id,
                                    Some(&subscription_id),
                                    "grace_expire_suspend",
                                    "failed",
                                    Some(&err_text),
                                    "system",
                                    None,
                                )
                                .await;
                        }
                    }
                }
            }
        }

        #[cfg(feature = "postgres")]
        let invoices: Vec<(
            String,
            String,
            chrono::DateTime<chrono::Utc>,
            String,
            Option<String>,
        )> = sqlx::query_as(
            r#"
            SELECT id, invoice_number, due_date, status, external_id
            FROM invoices
            WHERE tenant_id = $1
              AND external_id LIKE 'pkgsub:%'
              AND status IN ('pending', 'verification_pending', 'failed')
            ORDER BY due_date ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let invoices: Vec<(
            String,
            String,
            chrono::DateTime<chrono::Utc>,
            String,
            Option<String>,
        )> = sqlx::query_as(
            r#"
            SELECT id, invoice_number, due_date, status, external_id
            FROM invoices
            WHERE tenant_id = ?
              AND external_id LIKE 'pkgsub:%'
              AND status IN ('pending', 'verification_pending', 'failed')
            ORDER BY due_date ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        for (invoice_id, invoice_number, due_date, _status, external_id) in invoices {
            result.evaluated_count += 1;

            let Some(subscription_id) =
                core::parse_customer_subscription_id(external_id.as_deref())
            else {
                let _ = self
                    .insert_billing_collection_log(
                        tenant_id,
                        &invoice_id,
                        None,
                        "evaluate",
                        "skipped",
                        Some("Missing or invalid pkg subscription external_id"),
                        "system",
                        None,
                    )
                    .await;
                continue;
            };

            let due_day = due_date.date_naive();
            let day_offset = (today - due_day).num_days();
            let reminder_code = core::reminder_code_for_day_offset(day_offset);

            if settings.reminder_enabled && settings.reminder_schedule.contains(&reminder_code) {
                let already_sent = self
                    .has_sent_invoice_reminder(tenant_id, &invoice_id, &reminder_code)
                    .await
                    .unwrap_or(false);

                if already_sent {
                    result.reminder_skipped_count += 1;
                    let _ = self
                        .insert_billing_collection_log(
                            tenant_id,
                            &invoice_id,
                            Some(&subscription_id),
                            "reminder",
                            "skipped",
                            Some("Reminder already sent for this code"),
                            "system",
                            None,
                        )
                        .await;
                } else {
                    match self
                        .send_invoice_reminder(
                            tenant_id,
                            &subscription_id,
                            &invoice_id,
                            &invoice_number,
                            due_date,
                            day_offset,
                        )
                        .await
                    {
                        Ok(recipients) if recipients > 0 => {
                            result.reminder_sent_count += 1;
                            let detail = format!("Notified {} user(s)", recipients);
                            let _ = self
                                .insert_invoice_reminder_log(
                                    tenant_id,
                                    &invoice_id,
                                    &reminder_code,
                                    "in_app",
                                    None,
                                    "sent",
                                    Some(&detail),
                                )
                                .await;
                            let _ = self
                                .insert_billing_collection_log(
                                    tenant_id,
                                    &invoice_id,
                                    Some(&subscription_id),
                                    "reminder",
                                    "success",
                                    Some(&detail),
                                    "system",
                                    None,
                                )
                                .await;
                        }
                        Ok(_) => {
                            result.reminder_skipped_count += 1;
                            let _ = self
                                .insert_invoice_reminder_log(
                                    tenant_id,
                                    &invoice_id,
                                    &reminder_code,
                                    "in_app",
                                    None,
                                    "skipped",
                                    Some("No recipients found"),
                                )
                                .await;
                            let _ = self
                                .insert_billing_collection_log(
                                    tenant_id,
                                    &invoice_id,
                                    Some(&subscription_id),
                                    "reminder",
                                    "skipped",
                                    Some("No recipients found"),
                                    "system",
                                    None,
                                )
                                .await;
                        }
                        Err(e) => {
                            result.failed_count += 1;
                            let err_text = e.to_string();
                            let _ = self
                                .insert_invoice_reminder_log(
                                    tenant_id,
                                    &invoice_id,
                                    &reminder_code,
                                    "in_app",
                                    None,
                                    "failed",
                                    Some(&err_text),
                                )
                                .await;
                            let _ = self
                                .insert_billing_collection_log(
                                    tenant_id,
                                    &invoice_id,
                                    Some(&subscription_id),
                                    "reminder",
                                    "failed",
                                    Some(&err_text),
                                    "system",
                                    None,
                                )
                                .await;
                        }
                    }
                }
            }

            let threshold_day = auto_suspend_threshold_date(
                due_day,
                settings.auto_suspend_mode.clone(),
                settings.auto_suspend_grace_days,
                settings.auto_suspend_fixed_day,
            );

            if settings.auto_suspend_enabled && today >= threshold_day {
                match self
                    .update_customer_subscription_status_if(
                        tenant_id,
                        &subscription_id,
                        "active",
                        "suspended",
                    )
                    .await
                {
                    Ok(true) => {
                        result.suspended_count += 1;
                        let _ = self
                            .apply_subscription_pppoe_billing_state(
                                tenant_id,
                                &subscription_id,
                                "suspended",
                            )
                            .await;
                        let _ = self
                            .insert_billing_collection_log(
                                tenant_id,
                                &invoice_id,
                                Some(&subscription_id),
                                "suspend",
                                "success",
                                Some("Subscription suspended due to overdue invoice"),
                                "system",
                                None,
                            )
                            .await;
                        self.audit_log(
                            None,
                            Some(tenant_id),
                            "subscription.auto_suspended",
                            "subscription",
                            Some(&subscription_id),
                            &json!({
                                "reason": "grace_period_overdue",
                                "invoice_id": invoice_id,
                                "invoice_number": invoice_number,
                                "day_offset": day_offset,
                                "threshold_date": threshold_day.to_string(),
                            }),
                        )
                        .await;
                        let _ = self
                            .notify_subscription_suspension(
                                tenant_id,
                                &subscription_id,
                                &invoice_id,
                                &invoice_number,
                                day_offset,
                            )
                            .await;
                    }
                    Ok(false) => {
                        let _ = self
                            .insert_billing_collection_log(
                                tenant_id,
                                &invoice_id,
                                Some(&subscription_id),
                                "suspend",
                                "skipped",
                                Some("Subscription already not active"),
                                "system",
                                None,
                            )
                            .await;
                    }
                    Err(e) => {
                        result.failed_count += 1;
                        let err_text = e.to_string();
                        let _ = self
                            .insert_billing_collection_log(
                                tenant_id,
                                &invoice_id,
                                Some(&subscription_id),
                                "suspend",
                                "failed",
                                Some(&err_text),
                                "system",
                                None,
                            )
                            .await;
                    }
                }
            }
        }

        self.audit_log(
            None,
            Some(tenant_id),
            "billing.collection_run",
            "billing",
            Some(tenant_id),
            &json!({
                "evaluated_count": result.evaluated_count,
                "reminder_sent_count": result.reminder_sent_count,
                "reminder_skipped_count": result.reminder_skipped_count,
                "suspended_count": result.suspended_count,
                "resumed_count": result.resumed_count,
                "failed_count": result.failed_count,
            }),
        )
        .await;

        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list_billing_collection_logs(
        &self,
        tenant_id: &str,
        action: Option<&str>,
        result: Option<&str>,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        search: Option<&str>,
        limit: u32,
    ) -> AppResult<Vec<BillingCollectionLogView>> {
        let limit = (limit as i64).clamp(1, 1000);
        let action = action.map(str::trim).filter(|v| !v.is_empty());
        let result = result.map(str::trim).filter(|v| !v.is_empty());
        let search = search.map(str::trim).filter(|v| !v.is_empty());

        #[cfg(feature = "postgres")]
        let rows = sqlx::query_as::<_, BillingCollectionLogView>(
            r#"
            SELECT
                l.id,
                l.tenant_id,
                l.invoice_id,
                l.subscription_id,
                l.action,
                l.result,
                l.reason,
                l.actor_type,
                l.actor_id,
                l.created_at,
                i.invoice_number,
                i.status AS invoice_status,
                i.due_date,
                cs.status AS subscription_status,
                c.name AS customer_name
            FROM billing_collection_logs l
            LEFT JOIN invoices i ON i.id = l.invoice_id
            LEFT JOIN customer_subscriptions cs
              ON cs.id = l.subscription_id
             AND cs.tenant_id = l.tenant_id
            LEFT JOIN customers c
              ON c.id = cs.customer_id
             AND c.tenant_id = l.tenant_id
            WHERE l.tenant_id = $1
              AND ($2::text IS NULL OR l.action = $2)
              AND ($3::text IS NULL OR l.result = $3)
              AND ($4::timestamptz IS NULL OR l.created_at >= $4)
              AND ($5::timestamptz IS NULL OR l.created_at <= $5)
              AND (
                    $6::text IS NULL
                 OR i.invoice_number ILIKE ('%' || $6 || '%')
                 OR COALESCE(c.name, '') ILIKE ('%' || $6 || '%')
                 OR COALESCE(l.reason, '') ILIKE ('%' || $6 || '%')
              )
            ORDER BY l.created_at DESC
            LIMIT $7
            "#,
        )
        .bind(tenant_id)
        .bind(action)
        .bind(result)
        .bind(from)
        .bind(to)
        .bind(search)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let from_s = from.map(|v| v.to_rfc3339());
        #[cfg(feature = "sqlite")]
        let to_s = to.map(|v| v.to_rfc3339());

        #[cfg(feature = "sqlite")]
        let rows = sqlx::query_as::<_, BillingCollectionLogView>(
            r#"
            SELECT
                l.id,
                l.tenant_id,
                l.invoice_id,
                l.subscription_id,
                l.action,
                l.result,
                l.reason,
                l.actor_type,
                l.actor_id,
                l.created_at,
                i.invoice_number,
                i.status AS invoice_status,
                i.due_date,
                cs.status AS subscription_status,
                c.name AS customer_name
            FROM billing_collection_logs l
            LEFT JOIN invoices i ON i.id = l.invoice_id
            LEFT JOIN customer_subscriptions cs
              ON cs.id = l.subscription_id
             AND cs.tenant_id = l.tenant_id
            LEFT JOIN customers c
              ON c.id = cs.customer_id
             AND c.tenant_id = l.tenant_id
            WHERE l.tenant_id = ?
              AND (? IS NULL OR l.action = ?)
              AND (? IS NULL OR l.result = ?)
              AND (? IS NULL OR l.created_at >= ?)
              AND (? IS NULL OR l.created_at <= ?)
              AND (
                    ? IS NULL
                 OR LOWER(COALESCE(i.invoice_number, '')) LIKE '%' || LOWER(?) || '%'
                 OR LOWER(COALESCE(c.name, '')) LIKE '%' || LOWER(?) || '%'
                 OR LOWER(COALESCE(l.reason, '')) LIKE '%' || LOWER(?) || '%'
              )
            ORDER BY l.created_at DESC
            LIMIT ?
            "#,
        )
        .bind(tenant_id)
        .bind(action)
        .bind(action)
        .bind(result)
        .bind(result)
        .bind(from_s.clone())
        .bind(from_s.clone())
        .bind(to_s.clone())
        .bind(to_s.clone())
        .bind(search)
        .bind(search)
        .bind(search)
        .bind(search)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list_invoice_reminder_logs(
        &self,
        tenant_id: &str,
        reminder_code: Option<&str>,
        status: Option<&str>,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        search: Option<&str>,
        limit: u32,
    ) -> AppResult<Vec<InvoiceReminderLogView>> {
        let limit = (limit as i64).clamp(1, 1000);
        let reminder_code = reminder_code.map(str::trim).filter(|v| !v.is_empty());
        let status = status.map(str::trim).filter(|v| !v.is_empty());
        let search = search.map(str::trim).filter(|v| !v.is_empty());

        #[cfg(feature = "postgres")]
        let rows = sqlx::query_as::<_, InvoiceReminderLogView>(
            r#"
            SELECT
                l.id,
                l.tenant_id,
                l.invoice_id,
                l.reminder_code,
                l.channel,
                l.recipient,
                l.status,
                l.detail,
                l.created_at,
                i.invoice_number,
                i.status AS invoice_status,
                i.due_date
            FROM invoice_reminder_logs l
            LEFT JOIN invoices i ON i.id = l.invoice_id
            WHERE l.tenant_id = $1
              AND ($2::text IS NULL OR l.reminder_code = $2)
              AND ($3::text IS NULL OR l.status = $3)
              AND ($4::timestamptz IS NULL OR l.created_at >= $4)
              AND ($5::timestamptz IS NULL OR l.created_at <= $5)
              AND (
                    $6::text IS NULL
                 OR i.invoice_number ILIKE ('%' || $6 || '%')
                 OR COALESCE(l.detail, '') ILIKE ('%' || $6 || '%')
              )
            ORDER BY l.created_at DESC
            LIMIT $7
            "#,
        )
        .bind(tenant_id)
        .bind(reminder_code)
        .bind(status)
        .bind(from)
        .bind(to)
        .bind(search)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let from_s = from.map(|v| v.to_rfc3339());
        #[cfg(feature = "sqlite")]
        let to_s = to.map(|v| v.to_rfc3339());

        #[cfg(feature = "sqlite")]
        let rows = sqlx::query_as::<_, InvoiceReminderLogView>(
            r#"
            SELECT
                l.id,
                l.tenant_id,
                l.invoice_id,
                l.reminder_code,
                l.channel,
                l.recipient,
                l.status,
                l.detail,
                l.created_at,
                i.invoice_number,
                i.status AS invoice_status,
                i.due_date
            FROM invoice_reminder_logs l
            LEFT JOIN invoices i ON i.id = l.invoice_id
            WHERE l.tenant_id = ?
              AND (? IS NULL OR l.reminder_code = ?)
              AND (? IS NULL OR l.status = ?)
              AND (? IS NULL OR l.created_at >= ?)
              AND (? IS NULL OR l.created_at <= ?)
              AND (
                    ? IS NULL
                 OR LOWER(COALESCE(i.invoice_number, '')) LIKE '%' || LOWER(?) || '%'
                 OR LOWER(COALESCE(l.detail, '')) LIKE '%' || LOWER(?) || '%'
              )
            ORDER BY l.created_at DESC
            LIMIT ?
            "#,
        )
        .bind(tenant_id)
        .bind(reminder_code)
        .bind(reminder_code)
        .bind(status)
        .bind(status)
        .bind(from_s.clone())
        .bind(from_s.clone())
        .bind(to_s.clone())
        .bind(to_s.clone())
        .bind(search)
        .bind(search)
        .bind(search)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows)
    }

    pub async fn run_billing_collection_now(
        &self,
        tenant_id: &str,
    ) -> AppResult<BillingCollectionRunResult> {
        self.run_billing_collection_for_tenant(tenant_id).await
    }

    async fn resolve_scheduler_interval_minutes(&self) -> i64 {
        let default_global = self
            .get_setting_value(None, "customer_invoice_scheduler_interval_minutes")
            .await
            .and_then(|v| v.parse::<i64>().ok())
            .map(|v| v.clamp(5, 1440))
            .unwrap_or(60);

        #[cfg(feature = "postgres")]
        let tenant_values: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT s.value
            FROM settings s
            INNER JOIN tenants t ON t.id = s.tenant_id
            WHERE s.key = 'customer_invoice_scheduler_interval_minutes'
              AND t.is_active = true
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        #[cfg(feature = "sqlite")]
        let tenant_values: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT s.value
            FROM settings s
            INNER JOIN tenants t ON t.id = s.tenant_id
            WHERE s.key = 'customer_invoice_scheduler_interval_minutes'
              AND t.is_active = 1
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        tenant_values
            .into_iter()
            .filter_map(|v| v.parse::<i64>().ok())
            .map(|v| v.clamp(5, 1440))
            .min()
            .unwrap_or(default_global)
    }

    async fn upsert_tenant_setting(
        &self,
        tenant_id: &str,
        key: &str,
        value: &str,
        description: &str,
    ) -> AppResult<()> {
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        {
            let rows = sqlx::query(
                "UPDATE settings SET value = $1, description = $2, updated_at = $3 WHERE tenant_id = $4 AND key = $5",
            )
            .bind(value)
            .bind(description)
            .bind(now)
            .bind(tenant_id)
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .rows_affected();

            if rows == 0 {
                sqlx::query(
                    r#"
                    INSERT INTO settings (id, tenant_id, key, value, description, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $6)
                    "#,
                )
                .bind(Uuid::new_v4().to_string())
                .bind(tenant_id)
                .bind(key)
                .bind(value)
                .bind(description)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            }
        }

        #[cfg(feature = "sqlite")]
        {
            let now_s = now.to_rfc3339();
            let rows = sqlx::query(
                "UPDATE settings SET value = ?, description = ?, updated_at = ? WHERE tenant_id = ? AND key = ?",
            )
            .bind(value)
            .bind(description)
            .bind(&now_s)
            .bind(tenant_id)
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .rows_affected();

            if rows == 0 {
                sqlx::query(
                    r#"
                    INSERT INTO settings (id, tenant_id, key, value, description, created_at, updated_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(Uuid::new_v4().to_string())
                .bind(tenant_id)
                .bind(key)
                .bind(value)
                .bind(description)
                .bind(&now_s)
                .bind(&now_s)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Initiate Midtrans Payment (Get Snap Token)
    pub async fn initiate_midtrans(&self, invoice_id: &str) -> AppResult<String> {
        let invoice = self.get_invoice(invoice_id).await?;

        if invoice.currency_code.to_uppercase() != "IDR" {
            return Err(AppError::Configuration(format!(
                "Midtrans only supports IDR in this implementation (invoice currency: {}).",
                invoice.currency_code
            )));
        }

        // 1. Fetch Settings — tenant only, no global fallback
        let merchant_id = invoice.merchant_id.as_deref().ok_or_else(|| {
            AppError::Configuration(format!(
                "Invoice {} has no merchant_id — tenant payment gateway not configured",
                invoice.invoice_number
            ))
        })?;

        let sk: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'payment_midtrans_server_key' AND tenant_id = $1")
            .bind(merchant_id).fetch_optional(&self.pool).await.unwrap_or(None).unwrap_or_default();

        let prod_str: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'payment_midtrans_is_production' AND tenant_id = $1")
            .bind(merchant_id).fetch_optional(&self.pool).await.unwrap_or(None).unwrap_or("false".to_string());

        let server_key = sk;
        let is_production = prod_str == "true";

        let app_url: String = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'app_public_url' AND tenant_id IS NULL",
        )
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None)
        .unwrap_or("http://localhost:3000".to_string());

        if server_key.is_empty() {
            return Err(AppError::Configuration(
                "Midtrans Server Key not configured for this merchant".to_string(),
            ));
        }

        // 2. Prepare API URL
        let base_url = if is_production {
            "https://app.midtrans.com/snap/v1/transactions"
        } else {
            "https://app.sandbox.midtrans.com/snap/v1/transactions"
        };

        // Construct Webhook URL for Override
        let webhook_url = format!(
            "{}/api/payment/midtrans/notification",
            app_url.trim_end_matches('/')
        );

        // 3. Prepare Payload
        let payload = json!({
            "transaction_details": {
                "order_id": invoice.invoice_number,
                "gross_amount": invoice.amount as i64 // IDR usually no decimals
            },
            "item_details": [{
                "id": invoice.id,
                "price": invoice.amount as i64,
                "quantity": 1,
                "name": invoice.description.clone().unwrap_or("Payment".to_string())
            }],
            "callbacks": {
                "finish": format!("{}/pay/{}", app_url, invoice.id),
                "error": format!("{}/pay/{}?status=error", app_url, invoice.id),
                "unfinish": format!("{}/pay/{}?status=pending", app_url, invoice.id)
            }
        });

        // 4. Send Request
        let auth_header = format!("{}:", server_key);
        let auth_b64 = general_purpose::STANDARD.encode(auth_header);

        let res = self
            .http_client
            .post(base_url)
            .header("Authorization", format!("Basic {}", auth_b64))
            .header("Content-Type", "application/json")
            .header("X-Override-Notification", webhook_url) // Override Webhook URL
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Midtrans API Req Failed: {}", e)))?;

        let resp_json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Midtrans API Parse Failed: {}", e)))?;

        if let Some(token) = resp_json.get("token").and_then(|v| v.as_str()) {
            Ok(token.to_string())
        } else {
            Err(AppError::Internal(format!(
                "Midtrans Error: {:?}",
                resp_json
            )))
        }
    }

    pub async fn list_duitku_payment_methods(
        &self,
        tenant_id: Option<&str>,
        payment_amount: Option<i64>,
    ) -> AppResult<Vec<DuitkuPaymentMethod>> {
        let merchant_code = self
            .get_setting_value(tenant_id, "payment_duitku_merchant_code")
            .await
            .unwrap_or_default();
        let api_key = self
            .get_setting_value(tenant_id, "payment_duitku_api_key")
            .await
            .unwrap_or_default();
        let is_production = self
            .get_setting_value(tenant_id, "payment_duitku_is_production")
            .await
            .unwrap_or_else(|| "false".to_string())
            == "true";

        if merchant_code.trim().is_empty() || api_key.trim().is_empty() {
            return Err(AppError::Configuration(
                "Duitku Merchant Code or API Key not configured".to_string(),
            ));
        }

        let amount = payment_amount.unwrap_or(10000).max(1);
        let datetime = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let signature = duitku_payment_methods_signature(
            merchant_code.trim(),
            amount,
            &datetime,
            api_key.trim(),
        );
        let base_url = if is_production {
            "https://passport.duitku.com/webapi/api/merchant/paymentmethod/getpaymentmethod"
        } else {
            "https://sandbox.duitku.com/webapi/api/merchant/paymentmethod/getpaymentmethod"
        };
        let payload = json!({
            "merchantcode": merchant_code.trim(),
            "amount": amount,
            "datetime": datetime,
            "signature": signature,
        });

        let res = self
            .http_client
            .post(base_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Duitku API request failed: {}", e)))?;

        let resp_json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Duitku API parse failed: {}", e)))?;

        let items = resp_json
            .get("paymentFee")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                AppError::Internal(format!(
                    "Duitku response missing paymentFee: {:?}",
                    resp_json
                ))
            })?;

        let mut methods = Vec::new();
        for item in items {
            let code = item
                .get("paymentMethod")
                .or_else(|| item.get("code"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_uppercase();
            if code.is_empty() {
                continue;
            }
            let name = item
                .get("paymentName")
                .or_else(|| item.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or(&code)
                .to_string();
            let fee = item
                .get("totalFee")
                .or_else(|| item.get("fee"))
                .and_then(|v| {
                    v.as_str()
                        .map(|s| s.to_string())
                        .or_else(|| v.as_i64().map(|n| n.to_string()))
                });
            methods.push(DuitkuPaymentMethod { code, name, fee });
        }

        Ok(methods)
    }

    /// Initiate Duitku payment and return the hosted payment URL.
    pub async fn initiate_duitku(
        &self,
        invoice_id: &str,
        payment_method_override: Option<&str>,
    ) -> AppResult<String> {
        let invoice = self.get_invoice(invoice_id).await?;

        if invoice.currency_code.to_uppercase() != "IDR" {
            return Err(AppError::Configuration(format!(
                "Duitku only supports IDR in this implementation (invoice currency: {}).",
                invoice.currency_code
            )));
        }

        let merchant_code = self
            .payment_setting_for_invoice(&invoice, "payment_duitku_merchant_code", "")
            .await?;
        let api_key = self
            .payment_setting_for_invoice(&invoice, "payment_duitku_api_key", "")
            .await?;
        let selected_methods_raw = self
            .payment_setting_for_invoice(&invoice, "payment_duitku_payment_methods", "")
            .await?;
        let legacy_method = self
            .payment_setting_for_invoice(&invoice, "payment_duitku_payment_method", "")
            .await?;
        let mut selected_methods =
            parse_selected_duitku_payment_methods(Some(selected_methods_raw.as_str()));
        if selected_methods.is_empty() {
            selected_methods = parse_selected_duitku_payment_methods(Some(legacy_method.as_str()));
        }
        let requested_method = payment_method_override
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| v.to_uppercase())
            .or_else(|| selected_methods.first().cloned());
        let is_production = self
            .payment_setting_for_invoice(&invoice, "payment_duitku_is_production", "false")
            .await?
            == "true";

        if merchant_code.trim().is_empty() || api_key.trim().is_empty() {
            return Err(AppError::Configuration(
                "Duitku Merchant Code or API Key not configured for this merchant".to_string(),
            ));
        }

        let Some(payment_method) = requested_method else {
            return Err(AppError::Configuration(
                "No Duitku payment method selected for this merchant".to_string(),
            ));
        };

        if !selected_methods.is_empty() && !selected_methods.contains(&payment_method) {
            return Err(AppError::Configuration(
                "Selected Duitku payment method is not enabled for this merchant".to_string(),
            ));
        }

        let app_url: String = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'app_public_url' AND tenant_id IS NULL",
        )
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None)
        .unwrap_or("http://localhost:3000".to_string());

        let base_url = if is_production {
            "https://passport.duitku.com/webapi/api/merchant/v2/inquiry"
        } else {
            "https://sandbox.duitku.com/webapi/api/merchant/v2/inquiry"
        };

        let payment_amount = invoice.amount.round() as i64;
        let signature = duitku_create_signature(
            merchant_code.trim(),
            &invoice.invoice_number,
            payment_amount,
            api_key.trim(),
        );

        let payload = json!({
            "merchantCode": merchant_code.trim(),
            "paymentAmount": payment_amount,
            "paymentMethod": payment_method,
            "merchantOrderId": invoice.invoice_number,
            "productDetails": invoice.description.clone().unwrap_or_else(|| "Invoice payment".to_string()),
            "customerVaName": invoice.tenant_id,
            "callbackUrl": format!("{}/api/payment/duitku/callback", app_url.trim_end_matches('/')),
            "returnUrl": format!("{}/pay/{}", app_url.trim_end_matches('/'), invoice.id),
            "signature": signature,
        });

        let res = self
            .http_client
            .post(base_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Duitku API request failed: {}", e)))?;

        let resp_json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Duitku API parse failed: {}", e)))?;

        let success = resp_json
            .get("statusCode")
            .and_then(|v| v.as_str())
            .map(|v| v == "00")
            .unwrap_or(true);

        if !success {
            return Err(AppError::Internal(format!("Duitku Error: {:?}", resp_json)));
        }

        if let Some(payment_url) = resp_json.get("paymentUrl").and_then(|v| v.as_str()) {
            self.mark_invoice_payment_method(invoice_id, "duitku")
                .await?;
            Ok(payment_url.to_string())
        } else {
            Err(AppError::Internal(format!(
                "Duitku response missing paymentUrl: {:?}",
                resp_json
            )))
        }
    }

    // ==================== BANK ACCOUNTS ====================

    /// Check Transaction Status (Manual/Poll)
    pub async fn check_transaction_status(&self, invoice_id: &str) -> AppResult<String> {
        let invoice = self.get_invoice(invoice_id).await?;

        if invoice.payment_method.as_deref() == Some("duitku") {
            return self.check_duitku_transaction_status(&invoice).await;
        }

        // 1. Fetch Settings — tenant only, no global fallback
        let merchant_id = invoice.merchant_id.as_deref().ok_or_else(|| {
            AppError::Configuration(format!(
                "Invoice {} has no merchant_id — tenant payment gateway not configured",
                invoice.invoice_number
            ))
        })?;

        let sk: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'payment_midtrans_server_key' AND tenant_id = $1")
            .bind(merchant_id).fetch_optional(&self.pool).await.unwrap_or(None).unwrap_or_default();

        let prod_str: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'payment_midtrans_is_production' AND tenant_id = $1")
            .bind(merchant_id).fetch_optional(&self.pool).await.unwrap_or(None).unwrap_or("false".to_string());

        let server_key = sk;
        let is_production = prod_str == "true";

        if server_key.is_empty() {
            return Err(AppError::Configuration(
                "Midtrans Server Key not configured".to_string(),
            ));
        }

        // 2. Prepare API URL (Core API)
        let base_url = if is_production {
            format!(
                "https://api.midtrans.com/v2/{}/status",
                invoice.invoice_number
            )
        } else {
            format!(
                "https://api.sandbox.midtrans.com/v2/{}/status",
                invoice.invoice_number
            )
        };

        // 3. Send Request
        let auth_header = format!("{}:", server_key);
        let auth_b64 = general_purpose::STANDARD.encode(auth_header);

        let res = self
            .http_client
            .get(&base_url)
            .header("Authorization", format!("Basic {}", auth_b64))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Midtrans API Req Failed: {}", e)))?;

        let resp_json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Midtrans API Parse Failed: {}", e)))?;

        // 4. Parse Status
        let transaction_status = resp_json["transaction_status"]
            .as_str()
            .unwrap_or("pending");
        let fraud_status = resp_json["fraud_status"].as_str().unwrap_or("");

        let mut payment_status = match transaction_status {
            "capture" => "paid",
            "settlement" => "paid",
            "pending" => "pending",
            "deny" | "expire" | "cancel" => "failed",
            _ => "pending",
        };

        if transaction_status == "capture" && fraud_status == "challenge" {
            payment_status = "pending";
        }

        // 5. Update Local Status
        // Only update if it changed
        if payment_status != invoice.status {
            self.process_midtrans_notification(&invoice.invoice_number, payment_status, None, None)
                .await?;
        }

        Ok(payment_status.to_string())
    }

    async fn check_duitku_transaction_status(&self, invoice: &Invoice) -> AppResult<String> {
        let merchant_code = self
            .payment_setting_for_invoice(invoice, "payment_duitku_merchant_code", "")
            .await?;
        let api_key = self
            .payment_setting_for_invoice(invoice, "payment_duitku_api_key", "")
            .await?;
        let is_production = self
            .payment_setting_for_invoice(invoice, "payment_duitku_is_production", "false")
            .await?
            == "true";

        if merchant_code.trim().is_empty() || api_key.trim().is_empty() {
            return Err(AppError::Configuration(
                "Duitku Merchant Code or API Key not configured".to_string(),
            ));
        }

        let base_url = if is_production {
            "https://passport.duitku.com/webapi/api/merchant/transactionStatus"
        } else {
            "https://sandbox.duitku.com/webapi/api/merchant/transactionStatus"
        };
        let signature = md5_hex(&format!(
            "{}{}{}",
            merchant_code.trim(),
            invoice.invoice_number,
            api_key.trim()
        ));
        let payload = json!({
            "merchantCode": merchant_code.trim(),
            "merchantOrderId": invoice.invoice_number,
            "signature": signature,
        });

        let res = self
            .http_client
            .post(base_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Duitku API request failed: {}", e)))?;

        let resp_json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Duitku API parse failed: {}", e)))?;

        let result_code = resp_json
            .get("statusCode")
            .or_else(|| resp_json.get("resultCode"))
            .and_then(|v| v.as_str())
            .unwrap_or("01");
        let payment_status = duitku_transaction_status_code_to_invoice_status(result_code);

        if payment_status != invoice.status {
            self.process_midtrans_notification(
                &invoice.invoice_number,
                payment_status,
                None,
                resp_json
                    .get("reference")
                    .and_then(|v| v.as_str())
                    .or(Some("duitku-status")),
            )
            .await?;
        }

        Ok(payment_status.to_string())
    }

    /// List all bank accounts
    pub async fn list_bank_accounts(&self, tenant_id: Option<&str>) -> Result<Vec<BankAccount>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let accounts = if let Some(tid) = tenant_id {
            sqlx::query_as("SELECT * FROM bank_accounts WHERE tenant_id = $1 ORDER BY created_at DESC")
                .bind(tid)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT * FROM bank_accounts WHERE 1=0 ORDER BY created_at DESC")
                .fetch_all(&self.pool)
                .await?
        };

        #[cfg(feature = "sqlite")]
        let accounts = if let Some(tid) = tenant_id {
            sqlx::query_as("SELECT * FROM bank_accounts WHERE tenant_id = ? ORDER BY created_at DESC")
                .bind(tid)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT * FROM bank_accounts WHERE 1=0 ORDER BY created_at DESC")
                .fetch_all(&self.pool)
                .await?
        };

        Ok(accounts)
    }

    /// Create a new bank account
    pub async fn create_bank_account(
        &self,
        req: CreateBankAccountRequest,
        tenant_id: Option<String>,
    ) -> Result<BankAccount, sqlx::Error> {
        println!(
            "Creating bank account: {} - {} (tenant: {:?})",
            req.bank_name, req.account_number, tenant_id
        );
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO bank_accounts (id, tenant_id, bank_name, account_number, account_holder, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(&id)
        .bind(&tenant_id)
        .bind(&req.bank_name)
        .bind(&req.account_number)
        .bind(&req.account_holder)
        .bind(true)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO bank_accounts (id, tenant_id, bank_name, account_number, account_holder, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&id)
        .bind(&tenant_id)
        .bind(&req.bank_name)
        .bind(&req.account_number)
        .bind(&req.account_holder)
        .bind(true)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        // Return the created account
        Ok(BankAccount {
            id,
            tenant_id,
            bank_name: req.bank_name,
            account_number: req.account_number,
            account_holder: req.account_holder,
            is_active: true,
            created_at: now,
            updated_at: now,
        })
    }

    /// Delete bank account
    pub async fn delete_bank_account(&self, id: &str) -> Result<(), sqlx::Error> {
        #[cfg(feature = "postgres")]
        sqlx::query("DELETE FROM bank_accounts WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query("DELETE FROM bank_accounts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Toggle active status
    #[allow(dead_code)]
    pub async fn toggle_bank_account(&self, id: &str, is_active: bool) -> Result<(), sqlx::Error> {
        #[cfg(feature = "postgres")]
        sqlx::query("UPDATE bank_accounts SET is_active = $1, updated_at = $2 WHERE id = $3")
            .bind(is_active)
            .bind(Utc::now())
            .bind(id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE bank_accounts SET is_active = ?, updated_at = ? WHERE id = ?")
            .bind(is_active)
            .bind(Utc::now().to_rfc3339())
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Process Midtrans Notification (Webhook)
    pub async fn process_midtrans_notification(
        &self,
        invoice_number: &str,
        status: &str,
        request_id: Option<&str>,
        callback_ref: Option<&str>,
    ) -> AppResult<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // 1. Get Invoice inside transaction boundary.
        #[cfg(feature = "postgres")]
        let invoice: Option<Invoice> = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT
                id, tenant_id, invoice_number,
                amount::FLOAT8 as amount,
                currency_code, base_currency_code,
                fx_rate::FLOAT8 as fx_rate, fx_source, fx_fetched_at,
                status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, rejection_reason, created_at, updated_at
            FROM invoices
            WHERE invoice_number = $1
            FOR UPDATE
            "#,
        )
        .bind(invoice_number)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let invoice: Option<Invoice> =
            sqlx::query_as("SELECT * FROM invoices WHERE invoice_number = ?")
                .bind(invoice_number)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

        let invoice = match invoice {
            Some(i) => i,
            None => {
                return Err(AppError::NotFound(format!(
                    "Invoice {} not found",
                    invoice_number
                )))
            }
        };

        let current_status = invoice.status.as_str();
        match decide_midtrans_transition(current_status, status) {
            MidtransTransitionDecision::Apply => {}
            MidtransTransitionDecision::SkipDuplicate => {
                let _ = tx.commit().await;
                let reason = format!(
                    "Duplicate Midtrans callback ignored (request_id={}, callback_ref={})",
                    request_id.unwrap_or("-"),
                    callback_ref.unwrap_or("-")
                );
                let _ = self
                    .insert_billing_collection_log(
                        &invoice.tenant_id,
                        &invoice.id,
                        core::parse_customer_subscription_id(invoice.external_id.as_deref())
                            .as_deref(),
                        "payment_callback",
                        "skipped",
                        Some(&reason),
                        "system",
                        None,
                    )
                    .await;
                return Ok(());
            }
            MidtransTransitionDecision::SkipDowngrade => {
                let _ = tx.commit().await;
                let reason = format!(
                    "Midtrans status downgrade ignored (current={}, incoming={}, request_id={})",
                    current_status,
                    status,
                    request_id.unwrap_or("-")
                );
                let _ = self
                    .insert_billing_collection_log(
                        &invoice.tenant_id,
                        &invoice.id,
                        core::parse_customer_subscription_id(invoice.external_id.as_deref())
                            .as_deref(),
                        "payment_callback",
                        "skipped",
                        Some(&reason),
                        "system",
                        None,
                    )
                    .await;
                return Ok(());
            }
            MidtransTransitionDecision::SkipPendingAfterFailed => {
                let _ = tx.commit().await;
                let reason = format!(
                    "Pending after failed ignored (request_id={}, callback_ref={})",
                    request_id.unwrap_or("-"),
                    callback_ref.unwrap_or("-")
                );
                let _ = self
                    .insert_billing_collection_log(
                        &invoice.tenant_id,
                        &invoice.id,
                        core::parse_customer_subscription_id(invoice.external_id.as_deref())
                            .as_deref(),
                        "payment_callback",
                        "skipped",
                        Some(&reason),
                        "system",
                        None,
                    )
                    .await;
                return Ok(());
            }
        }

        // 2. Update Status
        let now = Utc::now();
        let paid_at = if status == "paid" { Some(now) } else { None };

        #[cfg(feature = "postgres")]
        let rows = sqlx::query("UPDATE invoices SET status = $1, paid_at = $2, rejection_reason = CASE WHEN $1 = 'paid' THEN NULL ELSE rejection_reason END, updated_at = $3 WHERE id = $4 AND status = $5")
            .bind(status)
            .bind(paid_at)
            .bind(now)
            .bind(&invoice.id)
            .bind(current_status)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .rows_affected();

        #[cfg(feature = "sqlite")]
        let rows = {
            let paid_str = paid_at.map(|t| t.to_rfc3339());
            sqlx::query("UPDATE invoices SET status = ?, paid_at = ?, rejection_reason = CASE WHEN ? = 'paid' THEN NULL ELSE rejection_reason END, updated_at = ? WHERE id = ? AND status = ?")
                .bind(status)
                .bind(paid_str)
                .bind(status)
                .bind(now.to_rfc3339())
                .bind(&invoice.id)
                .bind(current_status)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .rows_affected()
        };

        tx.commit()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if rows == 0 {
            let reason = format!(
                "Callback skipped by conditional update (invoice={}, from={}, to={}, request_id={}, callback_ref={})",
                invoice.invoice_number,
                current_status,
                status,
                request_id.unwrap_or("-"),
                callback_ref.unwrap_or("-")
            );
            let _ = self
                .insert_billing_collection_log(
                    &invoice.tenant_id,
                    &invoice.id,
                    core::parse_customer_subscription_id(invoice.external_id.as_deref()).as_deref(),
                    "payment_callback",
                    "skipped",
                    Some(&reason),
                    "system",
                    None,
                )
                .await;
            return Ok(());
        }

        let callback_reason = format!(
            "Processed Midtrans callback (status={}, request_id={}, callback_ref={})",
            status,
            request_id.unwrap_or("-"),
            callback_ref.unwrap_or("-")
        );
        let _ = self
            .insert_billing_collection_log(
                &invoice.tenant_id,
                &invoice.id,
                core::parse_customer_subscription_id(invoice.external_id.as_deref()).as_deref(),
                "payment_callback",
                "success",
                Some(&callback_reason),
                "system",
                None,
            )
            .await;

        self.audit_log(
            None,
            Some(&invoice.tenant_id),
            "invoice.status_changed",
            "invoice",
            Some(&invoice.id),
            &json!({
                "gateway": "midtrans",
                "old_status": current_status,
                "new_status": status,
                "invoice_number": invoice.invoice_number,
                "request_id": request_id,
                "callback_ref": callback_ref,
            }),
        )
        .await;

        tracing::info!(
            request_id = request_id.unwrap_or("-"),
            callback_ref = callback_ref.unwrap_or("-"),
            invoice_number = invoice.invoice_number,
            previous_status = current_status,
            new_status = status,
            "Midtrans callback state transition committed"
        );

        // 3. Activate Subscription if Paid
        if status == "paid" {
            if let Err(e) = self
                .try_auto_resume_customer_subscription_from_paid_invoice(&invoice)
                .await
            {
                tracing::warn!(
                    "auto-resume check failed for invoice {}: {}",
                    invoice.invoice_number,
                    e
                );
            }

            println!(
                "DEBUG: Invoice {} is PAID. External ID: {:?}",
                invoice.invoice_number, invoice.external_id
            );
            // external_id stores either:
            // - "pkgsub:{subscription_id}" for customer package invoices
            // - "plan:{plan_id}:{billing_cycle}" for SaaS plan invoices
            // - legacy "{plan_id}:{billing_cycle}" for old SaaS plan invoices
            if let Some(ext_id) = &invoice.external_id {
                if ext_id.starts_with(CUSTOMER_PACKAGE_INVOICE_PREFIX) {
                    println!(
                        "DEBUG: Customer package invoice handled by customer-flow; tenant SaaS activation skipped for {}",
                        invoice.invoice_number
                    );
                } else if let Some(rest) = ext_id.strip_prefix("plan:") {
                    let parts: Vec<&str> = rest.split(':').collect();
                    if parts.len() == 2 {
                        let plan_id = parts[0];
                        let cycle = parts[1];
                        println!(
                            "DEBUG: Activating subscription for Tenant {}: Plan={}, Cycle={}",
                            invoice.tenant_id, plan_id, cycle
                        );
                        self.activate_subscription(&invoice.tenant_id, plan_id, cycle)
                            .await?;
                    }
                } else {
                    let parts: Vec<&str> = ext_id.split(':').collect();
                    if parts.len() == 2 {
                        let plan_id = parts[0];
                        let cycle = parts[1];
                        println!(
                            "DEBUG: Activating subscription for Tenant {}: Plan={}, Cycle={}",
                            invoice.tenant_id, plan_id, cycle
                        );
                        self.activate_subscription(&invoice.tenant_id, plan_id, cycle)
                            .await?;
                    } else {
                        println!(
                            "DEBUG: Activating subscription (fallback) for Tenant {}: Plan={}, Cycle=monthly",
                            invoice.tenant_id, ext_id
                        );
                        // Fallback for legacy records
                        self.activate_subscription(&invoice.tenant_id, ext_id, "monthly")
                            .await?;
                    }
                }
            } else {
                println!(
                    "ERROR: Invoice {} has NO external_id. Cannot activate subscription.",
                    invoice.invoice_number
                );
            }
        }

        let is_customer_package =
            is_customer_package_invoice_external_id(invoice.external_id.as_deref());

        // 4. Send status notification
        // - Customer package invoice:
        //   - customer/member role => customer-facing page (/pay/{invoice_id})
        //   - owner/admin role => admin invoice page (/admin/invoices)
        // - SaaS plan invoice: notify Owner/Admin tenant members only (/admin/subscription)
        if status == "paid" || status == "failed" {
            let title = if status == "paid" {
                "Payment Successful".to_string()
            } else {
                "Payment Failed".to_string()
            };
            let manual_failure = status == "failed" && is_manual_payment_invoice(&invoice);
            let message = if status == "paid" {
                format!(
                    "Invoice {} has been successfully paid. Thank you!",
                    invoice.invoice_number
                )
            } else if manual_failure {
                format!(
                    "Payment proof for invoice {} was rejected. Please review the reason and upload a new proof.",
                    invoice.invoice_number
                )
            } else {
                format!(
                    "Payment for invoice {} failed. Please check your payment method.",
                    invoice.invoice_number
                )
            };

            if is_customer_package {
                if manual_failure {
                    let subscription_id =
                        core::parse_customer_subscription_id(invoice.external_id.as_deref());
                    if let Some(subscription_id) = subscription_id {
                        let customer_user_ids = self
                            .list_customer_user_ids_for_subscription(
                                &invoice.tenant_id,
                                &subscription_id,
                            )
                            .await
                            .unwrap_or_default();

                        for user_id in customer_user_ids {
                            let _ = self
                                .notification_service
                                .create_notification(
                                    user_id,
                                    Some(invoice.tenant_id.clone()),
                                    title.clone(),
                                    message.clone(),
                                    "info".to_string(),
                                    "billing".to_string(),
                                    Some(format!("/pay/{}", invoice.id)),
                                )
                                .await;
                        }
                    }
                    // For manual failed, stop here: no admin/owner or other tenant roles.
                    // Requirement: notify only the affected customer.
                    return Ok(());
                }

                // Notify admins about the payment outcome.
                let admin_user_ids = self
                    .list_tenant_owner_admin_user_ids(&invoice.tenant_id)
                    .await
                    .unwrap_or_default();

                for user_id in admin_user_ids {
                    let _ = self
                        .notification_service
                        .create_notification(
                            user_id,
                            Some(invoice.tenant_id.clone()),
                            title.clone(),
                            message.clone(),
                            "info".to_string(),
                            "billing".to_string(),
                            Some(format!("/admin/invoices/{}", invoice.id)),
                        )
                        .await;
                }

                // Notify only the customer users linked to this invoice's subscription.
                let subscription_id =
                    core::parse_customer_subscription_id(invoice.external_id.as_deref());
                if let Some(subscription_id) = subscription_id {
                    let customer_user_ids = self
                        .list_customer_user_ids_for_subscription(
                            &invoice.tenant_id,
                            &subscription_id,
                        )
                        .await
                        .unwrap_or_default();

                    for user_id in customer_user_ids {
                        let _ = self
                            .notification_service
                            .create_notification(
                                user_id,
                                Some(invoice.tenant_id.clone()),
                                title.clone(),
                                message.clone(),
                                "info".to_string(),
                                "billing".to_string(),
                                Some(format!("/pay/{}", invoice.id)),
                            )
                            .await;
                    }
                }
            } else {
                let users = self
                    .list_tenant_owner_admin_user_ids(&invoice.tenant_id)
                    .await
                    .unwrap_or_default();

                for user_id in users {
                    let _ = self
                        .notification_service
                        .create_notification(
                            user_id,
                            Some(invoice.tenant_id.clone()),
                            title.clone(),
                            message.clone(),
                            "info".to_string(),
                            "billing".to_string(),
                            Some("/admin/subscription".to_string()),
                        )
                        .await;
                }
            }
        }

        // 5. Notify payment stakeholders
        if status == "paid" {
            if is_customer_package {
                let tenant_admins = self
                    .list_tenant_owner_admin_user_ids(&invoice.tenant_id)
                    .await
                    .unwrap_or_default();

                for user_id in tenant_admins {
                    let _ = self
                        .notification_service
                        .create_notification(
                            user_id,
                            Some(invoice.tenant_id.clone()),
                            "Customer Payment Received".to_string(),
                            format!(
                                "Customer invoice {} has been paid. Amount: {}",
                                invoice.invoice_number, invoice.amount
                            ),
                            "success".to_string(),
                            "billing".to_string(),
                            Some(format!("/admin/invoices/{}", invoice.id)),
                        )
                        .await;
                }
            } else {
                #[cfg(feature = "postgres")]
                let super_admins: Vec<(String,)> =
                    sqlx::query_as("SELECT id FROM users WHERE is_super_admin = true")
                        .fetch_all(&self.pool)
                        .await
                        .unwrap_or_default();

                #[cfg(feature = "sqlite")]
                let super_admins: Vec<(String,)> =
                    sqlx::query_as("SELECT id FROM users WHERE is_super_admin = 1")
                        .fetch_all(&self.pool)
                        .await
                        .unwrap_or_default();

                for (admin_id,) in super_admins {
                    let _ = self
                        .notification_service
                        .create_notification(
                            admin_id,
                            None, // System notification for SaaS billing
                            "New Subscription Sale!".to_string(),
                            format!(
                                "Invoice {} has been paid. Amount: {}",
                                invoice.invoice_number, invoice.amount
                            ),
                            "success".to_string(),
                            "billing".to_string(),
                            Some("/superadmin/invoices".to_string()),
                        )
                        .await;
                }
            }
        }

        Ok(())
    }

    /// Verify Midtrans webhook signature for a given invoice number.
    pub async fn verify_midtrans_signature(
        &self,
        invoice_number: &str,
        status_code: &str,
        gross_amount: &str,
        signature_key: &str,
    ) -> AppResult<bool> {
        #[cfg(feature = "postgres")]
        let merchant_id: Option<String> =
            sqlx::query_scalar("SELECT merchant_id FROM invoices WHERE invoice_number = $1")
                .bind(invoice_number)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .flatten();

        #[cfg(feature = "sqlite")]
        let merchant_id: Option<String> =
            sqlx::query_scalar("SELECT merchant_id FROM invoices WHERE invoice_number = ?")
                .bind(invoice_number)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .flatten();

        let server_key = if let Some(mid) = merchant_id {
            #[cfg(feature = "postgres")]
            let key: Option<String> = sqlx::query_scalar(
                "SELECT value FROM settings WHERE key = 'payment_midtrans_server_key' AND tenant_id = $1",
            )
            .bind(mid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            #[cfg(feature = "sqlite")]
            let key: Option<String> = sqlx::query_scalar(
                "SELECT value FROM settings WHERE key = 'payment_midtrans_server_key' AND tenant_id = ?",
            )
            .bind(mid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            key.unwrap_or_default()
        } else {
            #[cfg(feature = "postgres")]
            let key: Option<String> = sqlx::query_scalar(
                "SELECT value FROM settings WHERE key = 'payment_midtrans_server_key' AND tenant_id IS NULL",
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            #[cfg(feature = "sqlite")]
            let key: Option<String> = sqlx::query_scalar(
                "SELECT value FROM settings WHERE key = 'payment_midtrans_server_key' AND tenant_id IS NULL",
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            key.unwrap_or_default()
        };

        if server_key.is_empty() {
            return Err(AppError::Configuration(
                "Midtrans Server Key not configured for webhook verification".to_string(),
            ));
        }

        let payload = format!("{invoice_number}{status_code}{gross_amount}{server_key}");
        let mut hasher = Sha512::new();
        hasher.update(payload.as_bytes());
        let expected = format!("{:x}", hasher.finalize());

        Ok(expected.eq_ignore_ascii_case(signature_key))
    }

    pub async fn verify_duitku_callback_signature(
        &self,
        merchant_code: &str,
        amount: &str,
        merchant_order_id: &str,
        signature: &str,
    ) -> AppResult<bool> {
        let invoice = self.get_invoice_by_number(merchant_order_id).await?;
        let expected_merchant_code = self
            .payment_setting_for_invoice(&invoice, "payment_duitku_merchant_code", "")
            .await?;
        let api_key = self
            .payment_setting_for_invoice(&invoice, "payment_duitku_api_key", "")
            .await?;

        if expected_merchant_code.trim().is_empty() || api_key.trim().is_empty() {
            return Err(AppError::Configuration(
                "Duitku Merchant Code or API Key not configured for callback verification"
                    .to_string(),
            ));
        }

        if !expected_merchant_code
            .trim()
            .eq_ignore_ascii_case(merchant_code.trim())
        {
            return Ok(false);
        }

        let expected = duitku_callback_signature(
            merchant_code.trim(),
            amount.trim(),
            merchant_order_id.trim(),
            api_key.trim(),
        );
        Ok(expected.eq_ignore_ascii_case(signature.trim()))
    }

    async fn activate_subscription(
        &self,
        tenant_id: &str,
        plan_id: &str,
        billing_cycle: &str,
    ) -> AppResult<()> {
        let now = Utc::now();

        // Calculate end date based on cycle
        let end_date = if billing_cycle == "yearly" {
            now + chrono::Duration::days(365)
        } else {
            now + chrono::Duration::days(30)
        };

        println!(
            "DEBUG: DB Update - Tenant: {}, Plan: {}, Start: {}, End: {}",
            tenant_id, plan_id, now, end_date
        );

        // Explicit Upsert: Update first, if no match, Insert.
        // This avoids issues if the UNIQUE constraint is missing or broken.

        #[cfg(feature = "postgres")]
        {
            let rows = sqlx::query(
                "UPDATE tenant_subscriptions SET plan_id = $1, status = 'active', current_period_start = $2, current_period_end = $3, updated_at = $4 WHERE tenant_id = $5"
            )
            .bind(plan_id)
            .bind(now)
            .bind(Some(end_date))
            .bind(now)
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .rows_affected();

            if rows == 0 {
                sqlx::query(
                    "INSERT INTO tenant_subscriptions (id, tenant_id, plan_id, status, current_period_start, current_period_end, created_at, updated_at) VALUES ($1, $2, $3, 'active', $4, $5, $6, $6)"
                )
                .bind(Uuid::new_v4().to_string())
                .bind(tenant_id)
                .bind(plan_id)
                .bind(now)
                .bind(Some(end_date))
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            }
        }

        #[cfg(feature = "sqlite")]
        {
            let rows = sqlx::query(
                "UPDATE tenant_subscriptions SET plan_id = ?, status = 'active', current_period_start = ?, current_period_end = ?, updated_at = ? WHERE tenant_id = ?"
            )
            .bind(plan_id)
            .bind(now.to_rfc3339())
            .bind(end_date.to_rfc3339())
            .bind(now.to_rfc3339())
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .rows_affected();

            if rows == 0 {
                sqlx::query(
                    "INSERT INTO tenant_subscriptions (id, tenant_id, plan_id, status, current_period_start, current_period_end, created_at, updated_at) VALUES (?, ?, ?, 'active', ?, ?, ?, ?)"
                )
                .bind(Uuid::new_v4().to_string())
                .bind(tenant_id)
                .bind(plan_id)
                .bind(now.to_rfc3339())
                .bind(end_date.to_rfc3339())
                .bind(now.to_rfc3339())
                .bind(now.to_rfc3339())
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Submit Payment Proof (Manual Transfer)
    pub async fn submit_payment_proof(&self, invoice_id: &str, file_path: &str) -> AppResult<()> {
        let invoice = self.get_invoice(invoice_id).await?;
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query("UPDATE invoices SET status = 'verification_pending', proof_attachment = $1, rejection_reason = NULL, updated_at = $2 WHERE id = $3")
            .bind(file_path)
            .bind(now)
            .bind(invoice_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE invoices SET status = 'verification_pending', proof_attachment = ?, rejection_reason = NULL, updated_at = ? WHERE id = ?")
            .bind(file_path)
            .bind(now.to_rfc3339())
            .bind(invoice_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let is_customer_package =
            is_customer_package_invoice_external_id(invoice.external_id.as_deref());

        if is_customer_package {
            let tenant_admins = self
                .list_tenant_owner_admin_user_ids(&invoice.tenant_id)
                .await
                .unwrap_or_default();

            for user_id in tenant_admins {
                let _ = self
                    .notification_service
                    .create_notification(
                        user_id,
                        Some(invoice.tenant_id.clone()),
                        "New Payment Proof Uploaded".to_string(),
                        format!(
                            "A payment proof has been uploaded for customer invoice {}",
                            invoice.invoice_number
                        ),
                        "info".to_string(),
                        "billing".to_string(),
                        Some(format!("/admin/invoices/{}", invoice.id)),
                    )
                    .await;
            }
        } else {
            #[cfg(feature = "postgres")]
            let super_admins: Vec<(String,)> =
                sqlx::query_as("SELECT id FROM users WHERE is_super_admin = true")
                    .fetch_all(&self.pool)
                    .await
                    .unwrap_or_default();

            #[cfg(feature = "sqlite")]
            let super_admins: Vec<(String,)> =
                sqlx::query_as("SELECT id FROM users WHERE is_super_admin = 1")
                    .fetch_all(&self.pool)
                    .await
                    .unwrap_or_default();

            // Resolve tenant slug for correct invoice action URL
            let tenant_action_url: String = {
                let slug: Option<String> = sqlx::query_scalar(
                    "SELECT slug FROM tenants WHERE id = $1 LIMIT 1"
                )
                .bind(&invoice.tenant_id)
                .fetch_optional(&self.pool)
                .await
                .unwrap_or(None);
                slug.map(|s| format!("/{}/admin/invoices/{}", s, invoice.id))
                    .unwrap_or_else(|| format!("/superadmin/invoices/{}", invoice.id))
            };

            for (admin_id,) in super_admins {
                let _ = self
                    .notification_service
                    .create_notification(
                        admin_id,
                        None,
                        "New Payment Proof Uploaded".to_string(),
                        format!(
                            "A payment proof has been uploaded for invoice {}",
                            invoice.invoice_number
                        ),
                        "info".to_string(),
                        "billing".to_string(),
                        Some(tenant_action_url.clone()),
                    )
                    .await;
            }
        }

        self.audit_log(
            None,
            Some(&invoice.tenant_id),
            "invoice.payment_proof_uploaded",
            "invoice",
            Some(invoice_id),
            &json!({
                "file_path": file_path,
                "invoice_number": invoice.invoice_number,
                "is_customer_package": is_customer_package,
            }),
        )
        .await;

        Ok(())
    }

    /// Verify Payment (Approve/Reject)
    pub async fn verify_payment(
        &self,
        invoice_id: &str,
        status: &str,
        rejection_reason: Option<String>,
    ) -> AppResult<()> {
        if status != "paid" && status != "failed" {
            return Err(AppError::Validation(
                "Status must be 'paid' or 'failed'".to_string(),
            ));
        }

        let normalized_reason = rejection_reason
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        if status == "failed" && normalized_reason.is_none() {
            return Err(AppError::Validation(
                "rejection_reason is required when status is failed".to_string(),
            ));
        }

        // 1. Get Invoice to reuse existing logic
        let invoice = self.get_invoice(invoice_id).await?;

        // 2. Reuse process_midtrans_notification logic
        // process_midtrans_notification(&self, invoice: &Invoice, status: &str)
        self.process_midtrans_notification(&invoice.invoice_number, status, None, None)
            .await?;

        #[cfg(feature = "postgres")]
        sqlx::query("UPDATE invoices SET rejection_reason = $1 WHERE id = $2")
            .bind(if status == "failed" {
                normalized_reason.as_deref()
            } else {
                None
            })
            .bind(invoice_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE invoices SET rejection_reason = ? WHERE id = ?")
            .bind(if status == "failed" {
                normalized_reason.as_deref()
            } else {
                None
            })
            .bind(invoice_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let action = if status == "paid" {
            "invoice.verified"
        } else {
            "invoice.rejected"
        };
        self.audit_log(
            None,
            Some(&invoice.tenant_id),
            action,
            "invoice",
            Some(invoice_id),
            &json!({
                "status": status,
                "amount": invoice.amount,
                "rejection_reason": normalized_reason,
                "invoice_number": invoice.invoice_number,
            }),
        )
        .await;

        Ok(())
    }

    fn merge_collection_result(
        total: &mut BillingCollectionRunResult,
        partial: &BillingCollectionRunResult,
    ) {
        total.evaluated_count += partial.evaluated_count;
        total.reminder_sent_count += partial.reminder_sent_count;
        total.reminder_skipped_count += partial.reminder_skipped_count;
        total.suspended_count += partial.suspended_count;
        total.resumed_count += partial.resumed_count;
        total.failed_count += partial.failed_count;
    }

    async fn try_auto_resume_customer_subscription_from_paid_invoice(
        &self,
        invoice: &Invoice,
    ) -> AppResult<()> {
        let Some(subscription_id) =
            core::parse_customer_subscription_id(invoice.external_id.as_deref())
        else {
            return Ok(());
        };

        let current_status = self
            .get_customer_subscription_status(&invoice.tenant_id, &subscription_id)
            .await?;

        let Some(current_status) = current_status else {
            return Err(AppError::NotFound(
                "Customer subscription not found".to_string(),
            ));
        };

        let current = SubscriptionLifecycleStatus::parse(&current_status)
            .map_err(|e| AppError::Validation(e.to_string()))?;
        if current == SubscriptionLifecycleStatus::Cancelled {
            let _ = self
                .insert_billing_collection_log(
                    &invoice.tenant_id,
                    &invoice.id,
                    Some(&subscription_id),
                    "installation",
                    "skipped",
                    Some("Subscription is cancelled"),
                    "system",
                    None,
                )
                .await;
            return Ok(());
        }

        let settings = self
            .resolve_billing_collection_settings(Some(&invoice.tenant_id))
            .await;
        if current == SubscriptionLifecycleStatus::Suspended && !settings.auto_resume_on_payment {
            let _ = self
                .insert_billing_collection_log(
                    &invoice.tenant_id,
                    &invoice.id,
                    Some(&subscription_id),
                    "resume",
                    "skipped",
                    Some("Auto resume disabled by billing setting"),
                    "system",
                    None,
                )
                .await;
            return Ok(());
        }

        let installation_completed = self
            .has_completed_installation_work_order(&invoice.tenant_id, &subscription_id)
            .await?;

        let resolved = self
            .resolve_subscription_after_activation_event(
                &invoice.tenant_id,
                &subscription_id,
                true,
                installation_completed,
            )
            .await?;

        let should_disable_pppoe = !matches!(
            resolved,
            SubscriptionLifecycleStatus::Active | SubscriptionLifecycleStatus::GraceActive
        );
        let _ = self
            .apply_subscription_pppoe_billing_state(
                &invoice.tenant_id,
                &subscription_id,
                if should_disable_pppoe {
                    "suspended"
                } else {
                    resolved.as_str()
                },
            )
            .await;

        match resolved {
            SubscriptionLifecycleStatus::PendingInstallation => {
                let (work_order_id, work_order_created) = self
                    .ensure_installation_work_order(
                        &invoice.tenant_id,
                        &subscription_id,
                        &invoice.id,
                    )
                    .await?;

                match self
                    .upsert_customer_service_assignment_from_paid_invoice(
                        &invoice.tenant_id,
                        &subscription_id,
                        &invoice.id,
                        &work_order_id,
                    )
                    .await
                {
                    Ok(()) => {
                        let _ = self
                            .insert_billing_collection_log(
                                &invoice.tenant_id,
                                &invoice.id,
                                Some(&subscription_id),
                                "assignment",
                                "success",
                                Some("Stored candidate node assignment for installation"),
                                "system",
                                None,
                            )
                            .await;
                    }
                    Err(e) => {
                        let _ = self
                            .insert_billing_collection_log(
                                &invoice.tenant_id,
                                &invoice.id,
                                Some(&subscription_id),
                                "assignment",
                                "failed",
                                Some(&e.to_string()),
                                "system",
                                None,
                            )
                            .await;
                    }
                }

                let _ = self
                    .insert_billing_collection_log(
                        &invoice.tenant_id,
                        &invoice.id,
                        Some(&subscription_id),
                        "installation",
                        "success",
                        Some("Payment confirmed: subscription pending installation"),
                        "system",
                        None,
                    )
                    .await;
                let _ = self
                    .notify_subscription_installation_pending(
                        &invoice.tenant_id,
                        &subscription_id,
                        &invoice.invoice_number,
                    )
                    .await;
                if work_order_created {
                    let _ = self
                        .notify_new_installation_request(
                            &invoice.tenant_id,
                            &invoice.invoice_number,
                            &work_order_id,
                        )
                        .await;
                }
            }
            SubscriptionLifecycleStatus::Active => {
                let _ = self
                    .insert_billing_collection_log(
                        &invoice.tenant_id,
                        &invoice.id,
                        Some(&subscription_id),
                        "resume",
                        "success",
                        Some("Subscription activated after payment + completed installation"),
                        "system",
                        None,
                    )
                    .await;
                self.audit_log(
                    None,
                    Some(&invoice.tenant_id),
                    "subscription.auto_resumed",
                    "subscription",
                    Some(&subscription_id),
                    &json!({
                        "triggering_invoice_id": invoice.id,
                        "invoice_number": invoice.invoice_number,
                        "previous_status": current_status,
                    }),
                )
                .await;
                let _ = self
                    .notify_subscription_resumed(
                        &invoice.tenant_id,
                        &subscription_id,
                        &invoice.id,
                        &invoice.invoice_number,
                    )
                    .await;
            }
            SubscriptionLifecycleStatus::GraceActive => {}
            SubscriptionLifecycleStatus::InstallationDoneAwaitingPayment
            | SubscriptionLifecycleStatus::Suspended
            | SubscriptionLifecycleStatus::Cancelled => {}
        }

        Ok(())
    }

    async fn send_invoice_reminder(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        invoice_id: &str,
        invoice_number: &str,
        due_date: chrono::DateTime<chrono::Utc>,
        day_offset: i64,
    ) -> AppResult<usize> {
        let user_ids = self
            .list_notification_user_ids_for_subscription(tenant_id, subscription_id)
            .await?;
        if user_ids.is_empty() {
            return Ok(0);
        }

        let title = if day_offset < 0 {
            format!("Invoice due in {} day(s)", day_offset.abs())
        } else if day_offset == 0 {
            "Invoice due today".to_string()
        } else {
            format!("Invoice overdue by {} day(s)", day_offset)
        };

        let message = format!(
            "Invoice {} is due on {}. Please complete payment to keep service active.",
            invoice_number,
            due_date.format("%Y-%m-%d %H:%M UTC")
        );

        let mut sent = 0usize;
        for user_id in user_ids {
            if self
                .notification_service
                .create_notification(
                    user_id,
                    Some(tenant_id.to_string()),
                    title.clone(),
                    message.clone(),
                    "warning".to_string(),
                    "billing".to_string(),
                    Some(customer_invoice_notification_action_url(invoice_id)),
                )
                .await
                .is_ok()
            {
                sent += 1;
            }
        }

        Ok(sent)
    }

    async fn notify_subscription_invoice_created(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        invoice_id: &str,
        invoice_number: &str,
        amount: f64,
        currency_code: &str,
    ) -> AppResult<usize> {
        let user_ids = self
            .list_notification_user_ids_for_subscription(tenant_id, subscription_id)
            .await?;
        if user_ids.is_empty() {
            return Ok(0);
        }

        let title = "Invoice created".to_string();
        let message = format!(
            "New invoice {} is ready ({} {:.2}). Please complete payment to activate/keep service.",
            invoice_number, currency_code, amount
        );

        let mut sent = 0usize;
        for user_id in user_ids {
            if self
                .notification_service
                .create_notification(
                    user_id,
                    Some(tenant_id.to_string()),
                    title.clone(),
                    message.clone(),
                    "info".to_string(),
                    "billing".to_string(),
                    Some(format!("/pay/{}", invoice_id)),
                )
                .await
                .is_ok()
            {
                sent += 1;
            }
        }

        Ok(sent)
    }

    async fn notify_subscription_suspension(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        invoice_id: &str,
        invoice_number: &str,
        overdue_days: i64,
    ) -> AppResult<usize> {
        let user_ids = self
            .list_notification_user_ids_for_subscription(tenant_id, subscription_id)
            .await?;
        if user_ids.is_empty() {
            return Ok(0);
        }

        let title = "Subscription suspended".to_string();
        let message = format!(
            "Your subscription has been suspended (invoice {} overdue {} day(s)).",
            invoice_number, overdue_days
        );

        let mut sent = 0usize;
        for user_id in user_ids {
            if self
                .notification_service
                .create_notification(
                    user_id,
                    Some(tenant_id.to_string()),
                    title.clone(),
                    message.clone(),
                    "warning".to_string(),
                    "billing".to_string(),
                    Some(format!("/pay/{}", invoice_id)),
                )
                .await
                .is_ok()
            {
                sent += 1;
            }
        }

        Ok(sent)
    }

    async fn notify_subscription_resumed(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        invoice_id: &str,
        invoice_number: &str,
    ) -> AppResult<usize> {
        let user_ids = self
            .list_notification_user_ids_for_subscription(tenant_id, subscription_id)
            .await?;
        if user_ids.is_empty() {
            return Ok(0);
        }

        let title = "Subscription resumed".to_string();
        let message = format!(
            "Payment received for invoice {}. Your subscription is active again.",
            invoice_number
        );

        let mut sent = 0usize;
        for user_id in user_ids {
            if self
                .notification_service
                .create_notification(
                    user_id,
                    Some(tenant_id.to_string()),
                    title.clone(),
                    message.clone(),
                    "success".to_string(),
                    "billing".to_string(),
                    Some(format!("/pay/{}", invoice_id)),
                )
                .await
                .is_ok()
            {
                sent += 1;
            }
        }

        Ok(sent)
    }

    async fn notify_subscription_installation_pending(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        invoice_number: &str,
    ) -> AppResult<usize> {
        let user_ids = self
            .list_notification_user_ids_for_subscription(tenant_id, subscription_id)
            .await?;
        if user_ids.is_empty() {
            return Ok(0);
        }

        let title = "Order Queued for Installation".to_string();
        let message = format!(
            "Payment for invoice {} is confirmed. Your order is now Pending Installation and waiting assignment/schedule from admin or technician.",
            invoice_number
        );

        let mut sent = 0usize;
        for user_id in user_ids {
            if self
                .notification_service
                .create_notification(
                    user_id,
                    Some(tenant_id.to_string()),
                    title.clone(),
                    message.clone(),
                    "info".to_string(),
                    "operations".to_string(),
                    Some("/dashboard/services".to_string()),
                )
                .await
                .is_ok()
            {
                sent += 1;
            }
        }

        Ok(sent)
    }

    async fn notify_new_installation_request(
        &self,
        tenant_id: &str,
        invoice_number: &str,
        work_order_id: &str,
    ) -> AppResult<usize> {
        let user_ids = self
            .list_tenant_installation_alert_user_ids(tenant_id)
            .await?;
        if user_ids.is_empty() {
            return Ok(0);
        }

        let title = "Installation Work Order: New Request".to_string();
        let message = format!(
            "Invoice {} is paid. A new installation work order is ready for assignment and scheduling (WO {}).",
            invoice_number, work_order_id
        );

        let mut sent = 0usize;
        for user_id in user_ids {
            if self
                .notification_service
                .create_notification(
                    user_id,
                    Some(tenant_id.to_string()),
                    title.clone(),
                    message.clone(),
                    "info".to_string(),
                    "operations".to_string(),
                    Some("/admin/network/installations".to_string()),
                )
                .await
                .is_ok()
            {
                sent += 1;
            }
        }

        Ok(sent)
    }

    async fn list_notification_user_ids_for_subscription(
        &self,
        tenant_id: &str,
        subscription_id: &str,
    ) -> AppResult<Vec<String>> {
        #[cfg(feature = "postgres")]
        let customer_user_ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT cu.user_id
            FROM customer_subscriptions cs
            INNER JOIN customer_users cu
              ON cu.tenant_id = cs.tenant_id
             AND cu.customer_id = cs.customer_id
            WHERE cs.tenant_id = $1
              AND cs.id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let customer_user_ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT cu.user_id
            FROM customer_subscriptions cs
            INNER JOIN customer_users cu
              ON cu.tenant_id = cs.tenant_id
             AND cu.customer_id = cs.customer_id
            WHERE cs.tenant_id = ?
              AND cs.id = ?
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(customer_notification_user_ids(
            customer_user_ids,
            Vec::new(),
        ))
    }

    async fn list_customer_user_ids_for_subscription(
        &self,
        tenant_id: &str,
        subscription_id: &str,
    ) -> AppResult<Vec<String>> {
        #[cfg(feature = "postgres")]
        let customer_user_ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT cu.user_id
            FROM customer_subscriptions cs
            INNER JOIN customer_users cu
              ON cu.tenant_id = cs.tenant_id
             AND cu.customer_id = cs.customer_id
            WHERE cs.tenant_id = $1
              AND cs.id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let customer_user_ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT cu.user_id
            FROM customer_subscriptions cs
            INNER JOIN customer_users cu
              ON cu.tenant_id = cs.tenant_id
             AND cu.customer_id = cs.customer_id
            WHERE cs.tenant_id = ?
              AND cs.id = ?
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(customer_user_ids)
    }

    async fn list_tenant_owner_admin_user_ids(&self, tenant_id: &str) -> AppResult<Vec<String>> {
        #[cfg(feature = "postgres")]
        let rows: Vec<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT DISTINCT user_id, role
            FROM tenant_members
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT DISTINCT user_id, role
            FROM tenant_members
            WHERE tenant_id = ?
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(filter_owner_admin_user_ids(rows))
    }

    async fn list_tenant_installation_alert_user_ids(
        &self,
        tenant_id: &str,
    ) -> AppResult<Vec<String>> {
        let include_technician = self
            .should_include_technicians_in_installation_request_alerts(tenant_id)
            .await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT DISTINCT user_id, role
            FROM tenant_members
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT DISTINCT user_id, role
            FROM tenant_members
            WHERE tenant_id = ?
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(filter_installation_request_user_ids(
            rows,
            include_technician,
        ))
    }

    async fn should_include_technicians_in_installation_request_alerts(
        &self,
        tenant_id: &str,
    ) -> AppResult<bool> {
        #[cfg(feature = "postgres")]
        let raw: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = $1 AND tenant_id = $2 LIMIT 1",
        )
        .bind(INSTALLATION_WORK_ORDER_VISIBILITY_MODE_KEY)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let raw: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = ? AND tenant_id = ? LIMIT 1",
        )
        .bind(INSTALLATION_WORK_ORDER_VISIBILITY_MODE_KEY)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(matches!(
            raw.as_deref()
                .map(str::trim)
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("all_staff")
        ))
    }

    async fn has_sent_invoice_reminder(
        &self,
        tenant_id: &str,
        invoice_id: &str,
        reminder_code: &str,
    ) -> AppResult<bool> {
        #[cfg(feature = "postgres")]
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM invoice_reminder_logs
              WHERE tenant_id = $1
                AND invoice_id = $2
                AND reminder_code = $3
                AND status = 'sent'
            )
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id)
        .bind(reminder_code)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM invoice_reminder_logs
              WHERE tenant_id = ?
                AND invoice_id = ?
                AND reminder_code = ?
                AND status = 'sent'
            )
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id)
        .bind(reminder_code)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(exists)
    }

    async fn resolve_subscription_after_activation_event(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        payment_paid: bool,
        installation_completed: bool,
    ) -> AppResult<SubscriptionLifecycleStatus> {
        let current_status = self
            .get_customer_subscription_status(tenant_id, subscription_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Customer subscription not found".to_string()))?;

        let current = SubscriptionLifecycleStatus::parse(&current_status)
            .map_err(|e| AppError::Validation(e.to_string()))?;
        if current == SubscriptionLifecycleStatus::Cancelled {
            return Ok(current);
        }

        let target = resolve_activation_status(current, installation_completed, payment_paid)
            .map_err(|e| AppError::Validation(e.to_string()))?;

        if target != current {
            let updated = self
                .set_customer_subscription_status(
                    tenant_id,
                    subscription_id,
                    current.as_str(),
                    target.as_str(),
                )
                .await?;
            if !updated {
                return Err(AppError::Validation(
                    "Subscription status changed concurrently; retry activation flow".to_string(),
                ));
            }
        }

        Ok(target)
    }

    async fn update_customer_subscription_status_if(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        expected_status: &str,
        new_status: &str,
    ) -> AppResult<bool> {
        #[cfg(feature = "postgres")]
        let current_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let current_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM customer_subscriptions WHERE tenant_id = ? AND id = ?",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let Some(current_status) = current_status else {
            return Err(AppError::NotFound(
                "Customer subscription not found".to_string(),
            ));
        };

        if current_status != expected_status {
            return Ok(false);
        }

        let now = Utc::now();
        #[cfg(feature = "postgres")]
        let rows = sqlx::query(
            r#"
            UPDATE customer_subscriptions
            SET status = $1,
                starts_at = CASE WHEN $1 IN ('active', 'grace_active') THEN COALESCE(starts_at, $2) ELSE starts_at END,
                grace_started_at = CASE WHEN $1 = 'grace_active' THEN COALESCE(grace_started_at, $2) WHEN $1 = 'active' THEN NULL ELSE grace_started_at END,
                grace_until = CASE WHEN $1 = 'active' THEN NULL WHEN $1 = 'suspended' THEN NULL ELSE grace_until END,
                updated_at = $2
            WHERE tenant_id = $3 AND id = $4 AND status = $5
            "#,
        )
        .bind(new_status)
        .bind(now)
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(expected_status)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .rows_affected();

        #[cfg(feature = "sqlite")]
        let rows = sqlx::query(
            r#"
            UPDATE customer_subscriptions
            SET status = ?,
                starts_at = CASE WHEN ? IN ('active', 'grace_active') THEN COALESCE(starts_at, ?) ELSE starts_at END,
                grace_started_at = CASE WHEN ? = 'grace_active' THEN COALESCE(grace_started_at, ?) WHEN ? = 'active' THEN NULL ELSE grace_started_at END,
                grace_until = CASE WHEN ? = 'active' THEN NULL WHEN ? = 'suspended' THEN NULL ELSE grace_until END,
                updated_at = ?
            WHERE tenant_id = ? AND id = ? AND status = ?
            "#,
        )
        .bind(new_status)
        .bind(new_status)
        .bind(now.to_rfc3339())
        .bind(new_status)
        .bind(now.to_rfc3339())
        .bind(new_status)
        .bind(new_status)
        .bind(new_status)
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(expected_status)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .rows_affected();

        Ok(rows > 0)
    }

    async fn has_completed_installation_work_order(
        &self,
        tenant_id: &str,
        subscription_id: &str,
    ) -> AppResult<bool> {
        #[cfg(feature = "postgres")]
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM installation_work_orders
              WHERE tenant_id = $1
                AND subscription_id = $2
                AND status = 'completed'
            )
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM installation_work_orders
              WHERE tenant_id = ?
                AND subscription_id = ?
                AND status = 'completed'
            )
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(exists)
    }

    async fn get_customer_subscription_status(
        &self,
        tenant_id: &str,
        subscription_id: &str,
    ) -> AppResult<Option<String>> {
        #[cfg(feature = "postgres")]
        let current_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let current_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM customer_subscriptions WHERE tenant_id = ? AND id = ?",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(current_status)
    }

    async fn set_customer_subscription_status(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        expected_status: &str,
        new_status: &str,
    ) -> AppResult<bool> {
        let now = Utc::now();
        #[cfg(feature = "postgres")]
        let rows = sqlx::query(
            r#"
            UPDATE customer_subscriptions
            SET status = $1,
                starts_at = CASE WHEN $1 IN ('active', 'grace_active') THEN COALESCE(starts_at, $2) ELSE starts_at END,
                grace_started_at = CASE WHEN $1 = 'grace_active' THEN COALESCE(grace_started_at, $2) WHEN $1 = 'active' THEN NULL ELSE grace_started_at END,
                grace_until = CASE WHEN $1 = 'grace_active' THEN grace_until ELSE NULL END,
                updated_at = $2
            WHERE tenant_id = $3 AND id = $4 AND status = $5
            "#,
        )
        .bind(new_status)
        .bind(now)
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(expected_status)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .rows_affected();

        #[cfg(feature = "sqlite")]
        let rows = sqlx::query(
            r#"
            UPDATE customer_subscriptions
            SET status = ?,
                starts_at = CASE WHEN ? IN ('active', 'grace_active') THEN COALESCE(starts_at, ?) ELSE starts_at END,
                grace_started_at = CASE WHEN ? = 'grace_active' THEN COALESCE(grace_started_at, ?) WHEN ? = 'active' THEN NULL ELSE grace_started_at END,
                grace_until = CASE WHEN ? = 'grace_active' THEN grace_until ELSE NULL END,
                updated_at = ?
            WHERE tenant_id = ? AND id = ? AND status = ?
            "#,
        )
        .bind(new_status)
        .bind(new_status)
        .bind(now.to_rfc3339())
        .bind(new_status)
        .bind(now.to_rfc3339())
        .bind(new_status)
        .bind(new_status)
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(expected_status)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .rows_affected();

        Ok(rows > 0)
    }

    async fn set_subscription_pppoe_disabled_state(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        disabled: bool,
    ) -> AppResult<u64> {
        #[cfg(feature = "postgres")]
        let location_id: Option<String> = sqlx::query_scalar(
            "SELECT location_id FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let location_id: Option<String> = sqlx::query_scalar(
            "SELECT location_id FROM customer_subscriptions WHERE tenant_id = ? AND id = ? LIMIT 1",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let Some(location_id) = location_id else {
            return Ok(0);
        };

        self.pppoe_service
            .set_location_accounts_disabled_state(tenant_id, &location_id, disabled)
            .await
    }

    async fn apply_subscription_pppoe_billing_state(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        subscription_status: &str,
    ) -> AppResult<u64> {
        #[derive(sqlx::FromRow)]
        struct SubscriptionPppoeContext {
            location_id: String,
            router_id: Option<String>,
            package_id: String,
        }

        #[cfg(feature = "postgres")]
        let context: Option<SubscriptionPppoeContext> = sqlx::query_as(
            r#"
            SELECT location_id, router_id, package_id
            FROM customer_subscriptions
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let context: Option<SubscriptionPppoeContext> = sqlx::query_as(
            r#"
            SELECT location_id, router_id, package_id
            FROM customer_subscriptions
            WHERE tenant_id = ? AND id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let Some(context) = context else {
            return Ok(0);
        };

        if should_disable_pppoe_for_subscription_status(subscription_status) {
            let settings = self
                .resolve_billing_collection_settings(Some(tenant_id))
                .await;

            match settings.auto_suspend_pppoe_action {
                AutoSuspendPppoeAction::MoveToIsolationPool => {
                    let mapping_isolation_pool: Option<String> =
                        sqlx::query_scalar::<_, Option<String>>(
                            r#"
                        SELECT isolation_pool
                        FROM isp_package_router_mappings
                        WHERE tenant_id = $1
                          AND router_id = $2
                          AND package_id = $3
                        LIMIT 1
                        "#,
                        )
                        .bind(tenant_id)
                        .bind(context.router_id.as_deref().unwrap_or_default())
                        .bind(&context.package_id)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(|e| AppError::Internal(e.to_string()))?
                        .flatten();

                    let isolation_pool = mapping_isolation_pool
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .or(settings.auto_suspend_isolation_pool.as_deref());

                    if let Some(pool_name) = isolation_pool {
                        return self
                            .pppoe_service
                            .set_location_accounts_address_pool_state(
                                tenant_id,
                                &context.location_id,
                                Some(pool_name),
                                false,
                                true,
                            )
                            .await;
                    }

                    tracing::warn!(
                        "billing auto suspend isolation pool is empty for tenant {}, falling back to disable_secret",
                        tenant_id
                    );
                }
                AutoSuspendPppoeAction::DisableSecret => {}
            }

            return self
                .set_subscription_pppoe_disabled_state(tenant_id, subscription_id, true)
                .await;
        }

        #[cfg(feature = "postgres")]
        let package_pool: Option<String> = sqlx::query_scalar(
            r#"
            SELECT address_pool
            FROM isp_package_router_mappings
            WHERE tenant_id = $1
              AND router_id = $2
              AND package_id = $3
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(context.router_id.as_deref().unwrap_or_default())
        .bind(&context.package_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let package_pool: Option<String> = sqlx::query_scalar(
            r#"
            SELECT address_pool
            FROM isp_package_router_mappings
            WHERE tenant_id = ?
              AND router_id = ?
              AND package_id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(context.router_id.as_deref().unwrap_or_default())
        .bind(&context.package_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.pppoe_service
            .set_location_accounts_address_pool_state(
                tenant_id,
                &context.location_id,
                package_pool.as_deref(),
                false,
                true,
            )
            .await
    }

    async fn upsert_customer_service_assignment_from_paid_invoice(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        invoice_id: &str,
        work_order_id: &str,
    ) -> AppResult<()> {
        #[cfg(feature = "postgres")]
        let sub: AssignmentSubscriptionRef = sqlx::query_as(
            r#"
            SELECT
              cs.customer_id,
              cs.location_id,
              cs.router_id,
              cl.latitude::float8 AS latitude,
              cl.longitude::float8 AS longitude
            FROM customer_subscriptions cs
            INNER JOIN customer_locations cl
              ON cl.tenant_id = cs.tenant_id
             AND cl.id = cs.location_id
            WHERE cs.tenant_id = $1
              AND cs.id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Customer subscription not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let sub: AssignmentSubscriptionRef = sqlx::query_as(
            r#"
            SELECT
              cs.customer_id,
              cs.location_id,
              cs.router_id,
              cl.latitude AS latitude,
              cl.longitude AS longitude
            FROM customer_subscriptions cs
            INNER JOIN customer_locations cl
              ON cl.tenant_id = cs.tenant_id
             AND cl.id = cs.location_id
            WHERE cs.tenant_id = ?
              AND cs.id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Customer subscription not found".to_string()))?;

        #[cfg(feature = "postgres")]
        let zone_id: Option<String> = if let (Some(lat), Some(lng)) = (sub.latitude, sub.longitude)
        {
            sqlx::query_scalar(
                r#"
                SELECT z.id::text
                FROM service_zones z
                WHERE z.tenant_id = $1::uuid
                  AND z.status = 'active'
                  AND ST_Contains(z.geom, ST_SetSRID(ST_MakePoint($2::float8, $3::float8), 4326))
                ORDER BY z.priority ASC
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(lng)
            .bind(lat)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
        } else {
            None
        };

        #[cfg(feature = "sqlite")]
        let zone_id: Option<String> = None;

        #[cfg(feature = "postgres")]
        let candidates: Vec<AssignmentCandidateNodeRow> = sqlx::query_as(
            r#"
            SELECT
              n.id::text AS node_id,
              n.name,
              n.node_type,
              n.status,
              n.capacity_json,
              n.health_json,
              CASE
                WHEN $2::float8 IS NOT NULL AND $3::float8 IS NOT NULL
                THEN ST_Distance(
                  geography(n.geom),
                  geography(ST_SetSRID(ST_MakePoint($2::float8, $3::float8), 4326))
                )::float8
                ELSE NULL
              END AS distance_m,
              AVG(l.utilization_pct::float8) FILTER (WHERE l.utilization_pct IS NOT NULL) AS avg_link_utilization_pct,
              COALESCE(COUNT(l.id) FILTER (WHERE l.status = 'down'), 0)::bigint AS down_links,
              COALESCE(COUNT(l.id), 0)::bigint AS link_count
            FROM network_nodes n
            LEFT JOIN network_links l
              ON l.tenant_id = n.tenant_id
             AND (l.from_node_id = n.id OR l.to_node_id = n.id)
            WHERE n.tenant_id = $1::uuid
              AND n.status = 'active'
              AND (
                $4::uuid IS NULL
                OR EXISTS (
                  SELECT 1
                  FROM zone_node_bindings znb
                  WHERE znb.tenant_id = n.tenant_id
                    AND znb.zone_id = $4::uuid
                    AND znb.node_id = n.id
                )
              )
            GROUP BY n.id
            LIMIT 30
            "#,
        )
        .bind(tenant_id)
        .bind(sub.longitude)
        .bind(sub.latitude)
        .bind(zone_id.as_deref())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let candidates: Vec<AssignmentCandidateNodeRow> = Vec::new();

        let mut ranked: Vec<(f64, serde_json::Value)> = Vec::new();
        for row in candidates {
            let health_score = core::assignment_health_score(&row.status, &row.health_json);
            let capacity_score =
                core::assignment_capacity_score(&row.capacity_json, row.avg_link_utilization_pct);
            let distance_score = core::assignment_distance_score(row.distance_m).unwrap_or(60.0);
            let stability_penalty =
                (row.down_links as f64 * 7.5) + if row.link_count == 0 { 12.0 } else { 0.0 };
            let score = ((health_score * 0.45) + (capacity_score * 0.35) + (distance_score * 0.20)
                - stability_penalty)
                .clamp(0.0, 100.0);

            ranked.push((
                score,
                json!({
                    "node_id": row.node_id,
                    "name": row.name,
                    "node_type": row.node_type,
                    "health_score": health_score,
                    "capacity_score": capacity_score,
                    "distance_m": row.distance_m,
                    "avg_link_utilization_pct": row.avg_link_utilization_pct,
                    "down_links": row.down_links,
                    "link_count": row.link_count,
                    "score": score
                }),
            ));
        }
        ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        if ranked.len() > 10 {
            ranked.truncate(10);
        }

        let selected_node_id = ranked
            .first()
            .and_then(|(_, value)| value.get("node_id"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let selected_node_score = ranked.first().map(|(score, _)| *score);
        let candidate_snapshot =
            serde_json::Value::Array(ranked.iter().map(|(_, value)| value.clone()).collect());

        let resolution_notes = if selected_node_id.is_some() {
            format!(
                "Auto-assignment generated from paid invoice. Router ref: {}",
                sub.router_id
                    .as_deref()
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or("-")
            )
        } else {
            "Auto-assignment generated but no eligible active node found".to_string()
        };

        let path_node_ids = serde_json::json!([]);
        let path_link_ids = serde_json::json!([]);
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customer_service_assignments
              (id, tenant_id, invoice_id, subscription_id, work_order_id, customer_id, location_id,
               selected_zone_id, selected_node_id, selected_node_score, candidate_snapshot, path_node_ids, path_link_ids,
               status, resolution_notes, created_at, updated_at)
            VALUES
              ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::jsonb, $12::jsonb, $13::jsonb, 'pending_installation', $14, $15, $16)
            ON CONFLICT (tenant_id, invoice_id)
            DO UPDATE SET
              subscription_id = EXCLUDED.subscription_id,
              work_order_id = EXCLUDED.work_order_id,
              customer_id = EXCLUDED.customer_id,
              location_id = EXCLUDED.location_id,
              selected_zone_id = EXCLUDED.selected_zone_id,
              selected_node_id = EXCLUDED.selected_node_id,
              selected_node_score = EXCLUDED.selected_node_score,
              candidate_snapshot = EXCLUDED.candidate_snapshot,
              path_node_ids = EXCLUDED.path_node_ids,
              path_link_ids = EXCLUDED.path_link_ids,
              status = EXCLUDED.status,
              resolution_notes = EXCLUDED.resolution_notes,
              updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(invoice_id)
        .bind(subscription_id)
        .bind(work_order_id)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(zone_id)
        .bind(selected_node_id)
        .bind(selected_node_score)
        .bind(candidate_snapshot)
        .bind(path_node_ids)
        .bind(path_link_ids)
        .bind(resolution_notes)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        {
            // Kept for cross-feature compilation. SQLite deployments currently do not use
            // PostGIS-backed network mapping, so store a minimal placeholder assignment.
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"
                INSERT INTO customer_service_assignments
                  (id, tenant_id, invoice_id, subscription_id, work_order_id, customer_id, location_id,
                   selected_zone_id, selected_node_id, selected_node_score, candidate_snapshot, path_node_ids, path_link_ids,
                   status, resolution_notes, created_at, updated_at)
                VALUES
                  (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending_installation', ?, ?, ?)
                "#,
            )
            .bind(id)
            .bind(tenant_id)
            .bind(invoice_id)
            .bind(subscription_id)
            .bind(work_order_id)
            .bind(&sub.customer_id)
            .bind(&sub.location_id)
            .bind(zone_id)
            .bind(selected_node_id)
            .bind(selected_node_score)
            .bind(candidate_snapshot.to_string())
            .bind(path_node_ids.to_string())
            .bind(path_link_ids.to_string())
            .bind(resolution_notes)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        Ok(())
    }

    async fn ensure_installation_work_order(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        invoice_id: &str,
    ) -> AppResult<(String, bool)> {
        #[derive(sqlx::FromRow)]
        struct SubRef {
            customer_id: String,
            location_id: String,
            router_id: Option<String>,
        }

        #[cfg(feature = "postgres")]
        let sub: SubRef = sqlx::query_as(
            r#"
            SELECT customer_id, location_id, router_id
            FROM customer_subscriptions
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Customer subscription not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let sub: SubRef = sqlx::query_as(
            r#"
            SELECT customer_id, location_id, router_id
            FROM customer_subscriptions
            WHERE tenant_id = ? AND id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Customer subscription not found".to_string()))?;

        #[derive(sqlx::FromRow)]
        struct ExistingOrder {
            id: String,
        }

        #[cfg(feature = "postgres")]
        let existing: Option<ExistingOrder> = sqlx::query_as(
            r#"
            SELECT id
            FROM installation_work_orders
            WHERE tenant_id = $1
              AND subscription_id = $2
              AND status IN ('pending', 'in_progress')
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let existing: Option<ExistingOrder> = sqlx::query_as(
            r#"
            SELECT id
            FROM installation_work_orders
            WHERE tenant_id = ?
              AND subscription_id = ?
              AND status IN ('pending', 'in_progress')
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if let Some(ex) = existing {
            return Ok((ex.id, false));
        }

        let now = Utc::now();
        let id = Uuid::new_v4().to_string();
        let notes = "Auto-created from paid invoice; awaiting technician installation/activation.";

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO installation_work_orders
              (id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, notes, created_at, updated_at)
            VALUES
              ($1,$2,$3,$4,$5,$6,$7,'pending',$8,$9,$10)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(invoice_id)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(&sub.router_id)
        .bind(notes)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO installation_work_orders
              (id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, notes, created_at, updated_at)
            VALUES
              (?,?,?,?,?,?,?,'pending',?,?,?)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(invoice_id)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(&sub.router_id)
        .bind(notes)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok((id, true))
    }

    async fn insert_invoice_reminder_log(
        &self,
        tenant_id: &str,
        invoice_id: &str,
        reminder_code: &str,
        channel: &str,
        recipient: Option<&str>,
        status: &str,
        detail: Option<&str>,
    ) -> AppResult<()> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO invoice_reminder_logs
              (id, tenant_id, invoice_id, reminder_code, channel, recipient, status, detail, created_at)
            VALUES
              ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(invoice_id)
        .bind(reminder_code)
        .bind(channel)
        .bind(recipient)
        .bind(status)
        .bind(detail)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO invoice_reminder_logs
              (id, tenant_id, invoice_id, reminder_code, channel, recipient, status, detail, created_at)
            VALUES
              (?,?,?,?,?,?,?,?,?)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(invoice_id)
        .bind(reminder_code)
        .bind(channel)
        .bind(recipient)
        .bind(status)
        .bind(detail)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn insert_billing_collection_log(
        &self,
        tenant_id: &str,
        invoice_id: &str,
        subscription_id: Option<&str>,
        action: &str,
        result: &str,
        reason: Option<&str>,
        actor_type: &str,
        actor_id: Option<&str>,
    ) -> AppResult<()> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO billing_collection_logs
              (id, tenant_id, invoice_id, subscription_id, action, result, reason, actor_type, actor_id, created_at)
            VALUES
              ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(invoice_id)
        .bind(subscription_id)
        .bind(action)
        .bind(result)
        .bind(reason)
        .bind(actor_type)
        .bind(actor_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO billing_collection_logs
              (id, tenant_id, invoice_id, subscription_id, action, result, reason, actor_type, actor_id, created_at)
            VALUES
              (?,?,?,?,?,?,?,?,?,?)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(invoice_id)
        .bind(subscription_id)
        .bind(action)
        .bind(result)
        .bind(reason)
        .bind(actor_type)
        .bind(actor_id)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(())
    }

    pub async fn resolve_billing_collection_settings(
        &self,
        tenant_id: Option<&str>,
    ) -> BillingCollectionSettings {
        let defaults = BillingCollectionSettings::default();

        let auto_suspend_enabled = Self::parse_bool_setting(
            self.get_setting_value_fallback(tenant_id, BILLING_AUTO_SUSPEND_ENABLED_KEY)
                .await,
            defaults.auto_suspend_enabled,
        );

        let auto_suspend_mode = parse_auto_suspend_mode(
            self.get_setting_value_fallback(tenant_id, BILLING_AUTO_SUSPEND_MODE_KEY)
                .await,
            defaults.auto_suspend_mode.clone(),
        );

        let auto_suspend_grace_days = Self::parse_i64_setting(
            self.get_setting_value_fallback(tenant_id, BILLING_AUTO_SUSPEND_GRACE_DAYS_KEY)
                .await,
            defaults.auto_suspend_grace_days,
            0,
            365,
        );

        let auto_suspend_fixed_day = clamp_auto_suspend_fixed_day(Self::parse_i64_setting(
            self.get_setting_value_fallback(tenant_id, BILLING_AUTO_SUSPEND_FIXED_DAY_KEY)
                .await,
            defaults.auto_suspend_fixed_day,
            1,
            28,
        ));

        let auto_suspend_pppoe_action = parse_auto_suspend_pppoe_action(
            self.get_setting_value_fallback(tenant_id, BILLING_AUTO_SUSPEND_PPPOE_ACTION_KEY)
                .await,
            defaults.auto_suspend_pppoe_action.clone(),
        );

        let auto_suspend_isolation_pool = normalize_auto_suspend_isolation_pool(
            self.get_setting_value_fallback(tenant_id, BILLING_AUTO_SUSPEND_ISOLATION_POOL_KEY)
                .await,
        );

        let auto_resume_on_payment = Self::parse_bool_setting(
            self.get_setting_value_fallback(tenant_id, BILLING_AUTO_RESUME_ON_PAYMENT_KEY)
                .await,
            defaults.auto_resume_on_payment,
        );

        let reminder_enabled = Self::parse_bool_setting(
            self.get_setting_value_fallback(tenant_id, BILLING_REMINDER_ENABLED_KEY)
                .await,
            defaults.reminder_enabled,
        );

        let reminder_schedule = Self::parse_reminder_schedule(
            self.get_setting_value_fallback(tenant_id, BILLING_REMINDER_SCHEDULE_KEY)
                .await,
            defaults.reminder_schedule.clone(),
        );

        BillingCollectionSettings {
            auto_suspend_enabled,
            auto_suspend_mode,
            auto_suspend_grace_days,
            auto_suspend_fixed_day,
            auto_suspend_pppoe_action,
            auto_suspend_isolation_pool,
            auto_resume_on_payment,
            reminder_enabled,
            reminder_schedule,
        }
    }

    async fn get_setting_value_fallback(
        &self,
        tenant_id: Option<&str>,
        key: &str,
    ) -> Option<String> {
        if let Some(tid) = tenant_id {
            let local = self.get_setting_value(Some(tid), key).await;
            if let Some(value) = local {
                if !value.trim().is_empty() {
                    return Some(value);
                }
            }
        }
        self.get_setting_value(None, key).await
    }

    async fn get_setting_value(&self, tenant_id: Option<&str>, key: &str) -> Option<String> {
        #[cfg(feature = "postgres")]
        let q = if let Some(tid) = tenant_id {
            sqlx::query_scalar("SELECT value FROM settings WHERE key = $1 AND tenant_id = $2")
                .bind(key)
                .bind(tid)
        } else {
            sqlx::query_scalar("SELECT value FROM settings WHERE key = $1 AND tenant_id IS NULL")
                .bind(key)
        };

        #[cfg(feature = "sqlite")]
        let q = if let Some(tid) = tenant_id {
            sqlx::query_scalar("SELECT value FROM settings WHERE key = ? AND tenant_id = ?")
                .bind(key)
                .bind(tid)
        } else {
            sqlx::query_scalar("SELECT value FROM settings WHERE key = ? AND tenant_id IS NULL")
                .bind(key)
        };

        q.fetch_optional(&self.pool).await.ok().flatten()
    }

    fn parse_bool_setting(value: Option<String>, default: bool) -> bool {
        match value
            .unwrap_or_else(|| default.to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        }
    }

    fn parse_i64_setting(value: Option<String>, default: i64, min: i64, max: i64) -> i64 {
        value
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(default)
            .clamp(min, max)
    }

    fn parse_reminder_schedule(value: Option<String>, default: Vec<String>) -> Vec<String> {
        let mut parsed: Vec<String> = Vec::new();
        for token in value.unwrap_or_default().split(',') {
            let item = token.trim().to_ascii_uppercase();
            if item.is_empty() || parsed.contains(&item) {
                continue;
            }
            parsed.push(item);
        }

        if parsed.is_empty() {
            return default;
        }

        parsed
    }

    fn currency_decimals(&self, currency: &str) -> i32 {
        match currency.to_uppercase().as_str() {
            "IDR" | "JPY" | "KRW" => 0,
            _ => 2,
        }
    }

    fn round_amount(&self, amount: f64, currency: &str) -> f64 {
        let d = self.currency_decimals(currency);
        let factor = 10_f64.powi(d);
        (amount * factor).round() / factor
    }

    /// Calculate pro-rata amount for a partial billing period.
    /// Returns the proportional charge for remaining days from `change_date` to `period_end`.
    /// Used when a customer upgrades/downgrades mid-cycle.
    pub fn calculate_pro_rata_amount(
        full_cycle_amount: f64,
        period_start: chrono::DateTime<chrono::Utc>,
        period_end: chrono::DateTime<chrono::Utc>,
        change_date: chrono::DateTime<chrono::Utc>,
    ) -> f64 {
        if change_date <= period_start {
            return (full_cycle_amount * 100.0).round() / 100.0;
        }
        if change_date >= period_end {
            return 0.0;
        }

        let total_days = (period_end - period_start).num_days().max(1) as f64;
        let remaining_days = (period_end - change_date).num_days().max(0) as f64;
        let ratio = remaining_days / total_days;

        (full_cycle_amount * ratio * 100.0).round() / 100.0
    }

    /// Calculate the current billing period boundaries for a subscription.
    /// Returns (period_start, period_end) based on anchor date and billing cycle.
    pub fn current_billing_period(
        billing_cycle: &str,
        anchor: chrono::DateTime<chrono::Utc>,
        now: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> {
        let cycle = billing_cycle.trim().to_ascii_lowercase();

        if cycle == "monthly" {
            // Walk forward from anchor to find current period
            let mut cursor = anchor;
            while cursor
                .checked_add_months(Months::new(1))
                .map_or(true, |next| next <= now)
            {
                cursor = cursor.checked_add_months(Months::new(1)).ok_or_else(|| {
                    AppError::Internal("Failed to compute monthly period".to_string())
                })?;
            }
            let period_end = cursor.checked_add_months(Months::new(1)).ok_or_else(|| {
                AppError::Internal("Failed to compute monthly period end".to_string())
            })?;
            return Ok((cursor, period_end));
        }

        if cycle == "yearly" {
            let mut cursor = anchor;
            while cursor
                .checked_add_months(Months::new(12))
                .map_or(true, |next| next <= now)
            {
                cursor = cursor.checked_add_months(Months::new(12)).ok_or_else(|| {
                    AppError::Internal("Failed to compute yearly period".to_string())
                })?;
            }
            let period_end = cursor.checked_add_months(Months::new(12)).ok_or_else(|| {
                AppError::Internal("Failed to compute yearly period end".to_string())
            })?;
            return Ok((cursor, period_end));
        }

        Err(AppError::Validation(
            "billing_cycle must be monthly or yearly".to_string(),
        ))
    }

    fn billing_period_key(
        billing_cycle: &str,
        starts_at: Option<&chrono::DateTime<chrono::Utc>>,
        now: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<String> {
        let start_day = starts_at.map(|d| d.day()).unwrap_or(1);
        let start_month = starts_at.map(|d| d.month()).unwrap_or(1);
        let cycle = billing_cycle.trim().to_ascii_lowercase();

        if cycle == "monthly" {
            let mut year = now.year();
            let mut month = now.month();
            if now.day() < start_day {
                if month == 1 {
                    month = 12;
                    year -= 1;
                } else {
                    month -= 1;
                }
            }
            return Ok(format!("{:04}-{:02}", year, month));
        }

        if cycle == "yearly" {
            let mut year = now.year();
            if now.month() < start_month || (now.month() == start_month && now.day() < start_day) {
                year -= 1;
            }
            return Ok(format!("{:04}", year));
        }

        Err(AppError::Validation(
            "billing_cycle must be monthly or yearly".to_string(),
        ))
    }

    fn next_renewal_at(
        billing_cycle: &str,
        starts_at: Option<&chrono::DateTime<chrono::Utc>>,
        now: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<Option<chrono::DateTime<chrono::Utc>>> {
        let Some(anchor) = starts_at.copied() else {
            return Ok(None);
        };
        if now < anchor {
            return Ok(Some(anchor));
        }

        let cycle = billing_cycle.trim().to_ascii_lowercase();
        let mut cursor = anchor;

        if cycle == "monthly" {
            while cursor <= now {
                cursor = cursor.checked_add_months(Months::new(1)).ok_or_else(|| {
                    AppError::Internal("Failed to compute monthly renewal".to_string())
                })?;
            }
            return Ok(Some(cursor));
        }

        if cycle == "yearly" {
            while cursor <= now {
                cursor = cursor.checked_add_months(Months::new(12)).ok_or_else(|| {
                    AppError::Internal("Failed to compute yearly renewal".to_string())
                })?;
            }
            return Ok(Some(cursor));
        }

        Err(AppError::Validation(
            "billing_cycle must be monthly or yearly".to_string(),
        ))
    }

    pub async fn get_fx_rate(
        &self,
        base: &str,
        quote: &str,
        tenant_id: Option<&str>,
    ) -> AppResult<(f64, chrono::DateTime<chrono::Utc>, String)> {
        let now = chrono::Utc::now();
        let ttl_minutes: i64 = self
            .get_setting_value(None, "fx_cache_minutes")
            .await
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(1440);

        // Check cached rate
        #[cfg(feature = "postgres")]
        let cached: Option<(f64, chrono::DateTime<chrono::Utc>, String)> = sqlx::query_as(
            "SELECT rate::FLOAT8 as rate, fetched_at, source FROM fx_rates WHERE base_currency = $1 AND quote_currency = $2",
        )
        .bind(base)
        .bind(quote)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        #[cfg(feature = "sqlite")]
        let cached: Option<(f64, String, String)> = sqlx::query_as(
            "SELECT rate as rate, fetched_at, source FROM fx_rates WHERE base_currency = ? AND quote_currency = ?",
        )
        .bind(base)
        .bind(quote)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        #[cfg(feature = "sqlite")]
        let cached: Option<(f64, chrono::DateTime<chrono::Utc>, String)> =
            cached.and_then(|(rate, fetched_at, source)| {
                chrono::DateTime::parse_from_rfc3339(&fetched_at)
                    .ok()
                    .map(|dt| (rate, dt.with_timezone(&chrono::Utc), source))
            });

        if let Some((rate, fetched_at, source)) = cached {
            if (now - fetched_at).num_minutes() < ttl_minutes {
                return Ok((rate, fetched_at, source));
            }
        }

        // Fetch from provider (Frankfurter)
        let url = format!(
            "https://api.frankfurter.app/latest?from={}&to={}",
            base, quote
        );

        let resp: serde_json::Value = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("FX fetch failed: {}", e)))?
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("FX parse failed: {}", e)))?;

        let raw_rate = resp
            .get("rates")
            .and_then(|r| r.get(quote))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| AppError::Internal("FX rate missing in response".to_string()))?;

        let markup_setting = match self.get_setting_value(tenant_id, "fx_markup_bps").await {
            Some(v) => Some(v),
            None => self.get_setting_value(None, "fx_markup_bps").await,
        };

        let markup_bps: f64 = markup_setting
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);

        let effective_rate = raw_rate * (1.0 + (markup_bps / 10_000.0));
        let source = "frankfurter".to_string();

        // Upsert cache
        #[cfg(feature = "postgres")]
        {
            let _ = sqlx::query(
                r#"
                INSERT INTO fx_rates (base_currency, quote_currency, rate, fetched_at, source)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (base_currency, quote_currency)
                DO UPDATE SET rate = EXCLUDED.rate, fetched_at = EXCLUDED.fetched_at, source = EXCLUDED.source
            "#,
            )
            .bind(base)
            .bind(quote)
            .bind(effective_rate)
            .bind(now)
            .bind(&source)
            .execute(&self.pool)
            .await;
        }

        #[cfg(feature = "sqlite")]
        {
            let _ = sqlx::query(
                r#"
                INSERT OR REPLACE INTO fx_rates (base_currency, quote_currency, rate, fetched_at, source)
                VALUES (?, ?, ?, ?, ?)
            "#,
            )
            .bind(base)
            .bind(quote)
            .bind(effective_rate)
            .bind(now.to_rfc3339())
            .bind(&source)
            .execute(&self.pool)
            .await;
        }

        Ok((effective_rate, now, source))
    }

    // =========================================================================
    // Bulk Send Invoice (Phase 3)
    // =========================================================================

    /// Bulk-send a batch of invoices via email + in-app notification.
    ///
    /// Resolves each invoice → subscription → customer through
    /// `customer_service_assignments`, generates a PDF if requested, and fans
    /// out to the email and notification channels per `req.channels`.
    ///
    /// Per-invoice outcomes are aggregated into `BulkSendInvoiceResult` so
    /// partial failures never abort the run.
    ///
    /// Tenant isolation: invoices not owned by `tenant_id` (via either
    /// `invoices.tenant_id` or `invoices.merchant_id`) are reported as `failed`
    /// with reason `tenant_mismatch`.
    pub async fn bulk_send_invoices(
        &self,
        actor_user_id: &str,
        tenant_id: &str,
        req: crate::services::payment_service::dto::BulkSendInvoiceRequest,
    ) -> AppResult<crate::services::payment_service::dto::BulkSendInvoiceResult> {
        use crate::services::payment_service::dto::{
            BulkSendInvoiceItemResult, BulkSendInvoiceResult,
        };

        // ---- input validation ----
        if req.invoice_ids.is_empty() {
            return Err(AppError::Validation(
                "invoice_ids must not be empty".to_string(),
            ));
        }
        const BULK_CAP: usize = 200;
        if req.invoice_ids.len() > BULK_CAP {
            return Err(AppError::Validation(format!(
                "Bulk send limited to {} invoices per call (got {})",
                BULK_CAP,
                req.invoice_ids.len()
            )));
        }

        // Default: both channels.
        let channels = req
            .channels
            .clone()
            .unwrap_or_else(|| vec!["email".to_string(), "notification".to_string()]);
        let want_email = channels.iter().any(|c| c == "email");
        let want_notification = channels.iter().any(|c| c == "notification");
        let want_whatsapp = channels.iter().any(|c| c == "whatsapp");
        if !want_email && !want_notification && !want_whatsapp {
            return Err(AppError::Validation(
                "channels must include at least one of: email, notification, whatsapp".to_string(),
            ));
        }

        let mut items: Vec<BulkSendInvoiceItemResult> = Vec::with_capacity(req.invoice_ids.len());
        let mut sent_count = 0usize;
        let mut skipped_count = 0usize;
        let mut failed_count = 0usize;

        for invoice_id in &req.invoice_ids {
            let outcome = self
                .send_one_invoice(
                    tenant_id,
                    invoice_id,
                    want_email,
                    want_notification,
                    want_whatsapp,
                    req.attach_pdf,
                    req.template_id.as_deref(),
                )
                .await;

            let item = match outcome {
                Ok(item) => item,
                Err(e) => BulkSendInvoiceItemResult {
                    invoice_id: invoice_id.clone(),
                    invoice_number: String::new(),
                    status: "failed".to_string(),
                    email_sent: false,
                    notification_sent: false,
                    whatsapp_sent: false,
                    pdf_attached: false,
                    reason: Some(e.to_string()),
                },
            };

            match item.status.as_str() {
                "sent" => sent_count += 1,
                "skipped" => skipped_count += 1,
                _ => failed_count += 1,
            }
            items.push(item);
        }

        // Single audit-log summary entry per bulk run. Per-invoice traces are
        // captured in the response payload; surfacing all of them as audit rows
        // would flood the log on a 200-item call.
        let summary = serde_json::json!({
            "invoice_ids_count": req.invoice_ids.len(),
            "sent": sent_count,
            "skipped": skipped_count,
            "failed": failed_count,
            "channels": channels,
            "attach_pdf": req.attach_pdf,
            "template_id": req.template_id,
        });
        self.audit_log(
            Some(actor_user_id),
            Some(tenant_id),
            "bulk_send",
            "invoice",
            None,
            &summary,
        )
        .await;

        Ok(BulkSendInvoiceResult {
            sent_count,
            skipped_count,
            failed_count,
            items,
        })
    }

    /// Send-one-invoice helper. Resolves invoice + customer link, skips
    /// already-settled invoices, and fans out to the requested channels.
    async fn send_one_invoice(
        &self,
        tenant_id: &str,
        invoice_id: &str,
        want_email: bool,
        want_notification: bool,
        want_whatsapp: bool,
        attach_pdf: bool,
        _template_id: Option<&str>,
    ) -> AppResult<crate::services::payment_service::dto::BulkSendInvoiceItemResult> {
        use crate::services::payment_service::dto::BulkSendInvoiceItemResult;

        // Fetch invoice (any tenant) and enforce tenant ownership ourselves so
        // we can return a structured result instead of a 500.
        let invoice = match self.get_invoice(invoice_id).await {
            Ok(inv) => inv,
            Err(_) => {
                return Ok(BulkSendInvoiceItemResult {
                    invoice_id: invoice_id.to_string(),
                    invoice_number: String::new(),
                    status: "failed".to_string(),
                    email_sent: false,
                    notification_sent: false,
                    whatsapp_sent: false,
                    pdf_attached: false,
                    reason: Some("invoice_not_found".to_string()),
                });
            }
        };

        let invoice_tenant = invoice
            .merchant_id
            .as_deref()
            .unwrap_or(invoice.tenant_id.as_str());
        if invoice_tenant != tenant_id {
            return Ok(BulkSendInvoiceItemResult {
                invoice_id: invoice_id.to_string(),
                invoice_number: invoice.invoice_number.clone(),
                status: "failed".to_string(),
                email_sent: false,
                notification_sent: false,
                whatsapp_sent: false,
                pdf_attached: false,
                reason: Some("tenant_mismatch".to_string()),
            });
        }

        let already_settled = matches!(invoice.status.as_str(), "paid" | "cancelled");
        if already_settled {
            return Ok(BulkSendInvoiceItemResult {
                invoice_id: invoice_id.to_string(),
                invoice_number: invoice.invoice_number.clone(),
                status: "skipped".to_string(),
                email_sent: false,
                notification_sent: false,
                whatsapp_sent: false,
                pdf_attached: false,
                reason: Some("already_settled".to_string()),
            });
        }

        // Resolve subscription_id + customer_id via service-assignment link
        // (with fallback via external_id → subscription → customer chain).
        let link = self
            .resolve_invoice_customer_link(tenant_id, &invoice)
            .await
            .ok();
        let (subscription_id, customer_email, customer_name, customer_phone) = match link {
            Some(l) => (l.subscription_id, l.customer_email, l.customer_name, l.customer_phone),
            None => (None, None, None, None),
        };

        // ---- email channel ----
        let mut email_sent = false;
        let mut pdf_attached = false;
        if want_email {
            let to = customer_email
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            if let Some(to_email) = to {
                let subject = format!("Invoice {} – please complete payment", invoice.invoice_number);
                let body_text = format!(
                    "Hi {customer},\n\n\
                     Your invoice {number} for {currency} {amount:.2} is ready.\n\
                     Due date: {due}\n\n\
                     You can pay online here: /pay/{id}\n\n\
                     Thank you.",
                    customer = customer_name.as_deref().unwrap_or("there"),
                    number = invoice.invoice_number,
                    currency = invoice.currency_code,
                    amount = invoice.amount,
                    due = invoice.due_date.format("%Y-%m-%d"),
                    id = invoice.id,
                );

                // PDF attachment (best-effort: failure to render does not
                // block the email — caller sees pdf_attached=false).
                let attachments = if attach_pdf {
                    match self.render_invoice_pdf_for_email(&invoice, customer_name.as_deref(), customer_email.as_deref()) {
                        Ok(bytes) => {
                            pdf_attached = true;
                            vec![crate::services::email_service::EmailAttachment {
                                filename: format!("invoice-{}.pdf", invoice.invoice_number),
                                content_type: "application/pdf".to_string(),
                                content: bytes,
                            }]
                        }
                        Err(e) => {
                            tracing::warn!(
                                "bulk_send: PDF render failed for {}: {} — sending without attachment",
                                invoice.invoice_number, e
                            );
                            Vec::new()
                        }
                    }
                } else {
                    Vec::new()
                };

                match self
                    .notification_service
                    .force_send_email_with_attachments(
                        Some(tenant_id.to_string()),
                        to_email,
                        &subject,
                        &body_text,
                        None,
                        attachments,
                    )
                    .await
                {
                    Ok(_) => email_sent = true,
                    Err(e) => {
                        tracing::warn!(
                            "bulk_send: email failed for invoice {}: {}",
                            invoice.invoice_number, e
                        );
                    }
                }
            }
        }

        // ---- notification channel ----
        let mut notification_sent = false;
        if want_notification {
            if let Some(sub_id) = subscription_id.as_deref() {
                let sent = self
                    .notify_subscription_invoice_created(
                        tenant_id,
                        sub_id,
                        &invoice.id,
                        &invoice.invoice_number,
                        invoice.amount,
                        &invoice.currency_code,
                    )
                    .await
                    .unwrap_or(0);
                notification_sent = sent > 0;
            }
        }

        // ---- WhatsApp channel ----
        // Explicit admin-triggered send: routed through NotificationService's
        // force_send_whatsapp, which bypasses the per-event WA toggle (this is
        // an explicit action, not an auto-notification). Uses customer.phone
        // resolved via the same link as email.
        let mut whatsapp_sent = false;
        if want_whatsapp {
            let phone = customer_phone
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            if let Some(p) = phone {
                let msg = format!(
                    "Halo {cust},\n\n\
                     Invoice {num} sebesar {cur} {amt:.2} sudah terbit.\n\
                     Jatuh tempo: {due}\n\
                     Bayar online: /pay/{id}\n\n\
                     Terima kasih.",
                    cust = customer_name.as_deref().unwrap_or("Pelanggan"),
                    num = invoice.invoice_number,
                    cur = invoice.currency_code,
                    amt = invoice.amount,
                    due = invoice.due_date.format("%Y-%m-%d"),
                    id = invoice.id,
                );
                whatsapp_sent = self
                    .notification_service
                    .force_send_whatsapp(
                        Some(tenant_id),
                        "customer_invoice_due",
                        None,
                        p,
                        &msg,
                    )
                    .await
                    .unwrap_or(false);
            }
        }

        // Resolve final status. If no channel produced a send AND there was no
        // recipient at all on every requested channel, mark skipped instead of
        // failed (we did nothing wrong — the customer simply has no contact path).
        let no_email_target = want_email && customer_email.as_deref().unwrap_or("").trim().is_empty();
        let no_notif_target = want_notification && subscription_id.is_none();
        let no_wa_target = want_whatsapp && customer_phone.as_deref().unwrap_or("").trim().is_empty();
        let status = if email_sent || notification_sent || whatsapp_sent {
            "sent".to_string()
        } else if no_email_target && no_notif_target && no_wa_target {
            "skipped".to_string()
        } else {
            "failed".to_string()
        };
        let reason = if status == "skipped" {
            Some("no_contact_path".to_string())
        } else if status == "failed" {
            Some("delivery_failed".to_string())
        } else {
            None
        };

        Ok(BulkSendInvoiceItemResult {
            invoice_id: invoice.id.clone(),
            invoice_number: invoice.invoice_number.clone(),
            status,
            email_sent,
            notification_sent,
            whatsapp_sent,
            pdf_attached,
            reason,
        })
    }

    /// Look up the customer + subscription tied to an invoice.
    ///
    /// Resolution strategy (in order):
    /// 1. `customer_service_assignments` table — populated for work-order and
    ///    manually-linked invoices.
    /// 2. Parse `invoice.external_id` — format `pkgsub:<subscription_id>:<period>`.
    ///    Resolves subscription → customer → email. This handles auto-generated
    ///    package invoices that were never inserted into `customer_service_assignments`.
    #[cfg(feature = "postgres")]
    async fn resolve_invoice_customer_link(
        &self,
        tenant_id: &str,
        invoice: &crate::models::invoice::Invoice,
    ) -> AppResult<InvoiceCustomerLink> {

        // --- Path 1: customer_service_assignments (existing) ---
        let row: Option<(Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT csa.subscription_id, csa.customer_id, c.email, c.name, c.phone
            FROM customer_service_assignments csa
            INNER JOIN customers c ON c.id = csa.customer_id AND c.tenant_id = csa.tenant_id
            WHERE csa.tenant_id = $1 AND csa.invoice_id = $2
            ORDER BY csa.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&invoice.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if let Some((sub, _cust, email, name, phone)) = row {
            return Ok(InvoiceCustomerLink {
                subscription_id: sub,
                customer_email: email,
                customer_name: name,
                customer_phone: phone,
            });
        }

        // --- Path 2: fallback via external_id → subscription → customer ---
        // Best-effort: log errors but don't propagate them, so the caller
        // still gets a usable (possibly empty) link instead of an Err that
        // gets .ok()'d into None.
        if let Some(ref ext_id) = invoice.external_id {
            if let Some(subscription_id) = parse_subscription_id_from_external_id(ext_id) {

                tracing::info!(
                    invoice_id = %invoice.id,
                    subscription_id = %subscription_id,
                    "resolve_invoice_customer_link: falling back to external_id subscription chain"
                );

                // Look up subscription to get customer_id
                let sub_result = sqlx::query_as::<_, (String,)>(
                    r#"SELECT customer_id FROM customer_subscriptions WHERE id = $1 AND tenant_id = $2"#,
                )
                .bind(&subscription_id)
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await;

                match sub_result {
                    Ok(Some((customer_id,))) => {
                        // Look up customer to get email + name + phone
                        let cust_result = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>)>(
                            r#"SELECT email, name, phone FROM customers WHERE id = $1 AND tenant_id = $2"#,
                        )
                        .bind(&customer_id)
                        .bind(tenant_id)
                        .fetch_optional(&self.pool)
                        .await;

                        match cust_result {
                            Ok(Some((email, name, phone))) => {
                                tracing::info!(
                                    invoice_id = %invoice.id,
                                    customer_id = %customer_id,
                                    email = ?email,
                                    "resolve_invoice_customer_link: fallback resolved customer successfully"
                                );
                                return Ok(InvoiceCustomerLink {
                                    subscription_id: Some(subscription_id),
                                    customer_email: email,
                                    customer_name: name,
                                    customer_phone: phone,
                                });
                            }
                            Ok(None) => {
                                tracing::warn!(
                                    invoice_id = %invoice.id,
                                    customer_id = %customer_id,
                                    "resolve_invoice_customer_link: customer not found in fallback"
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    invoice_id = %invoice.id,
                                    error = %e,
                                    "resolve_invoice_customer_link: customer query failed in fallback"
                                );
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::warn!(
                            invoice_id = %invoice.id,
                            subscription_id = %subscription_id,
                            "resolve_invoice_customer_link: subscription not found in fallback"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            invoice_id = %invoice.id,
                            error = %e,
                            "resolve_invoice_customer_link: subscription query failed in fallback"
                        );
                    }
                }
            } else {
                tracing::debug!(
                    invoice_id = %invoice.id,
                    external_id = %ext_id,
                    "resolve_invoice_customer_link: external_id does not match pkgsub: format"
                );
            }
        }

        // --- No path resolved ---
        Ok(InvoiceCustomerLink::default())
    }

    #[cfg(not(feature = "postgres"))]
    async fn resolve_invoice_customer_link(
        &self,
        _tenant_id: &str,
        _invoice: &crate::models::invoice::Invoice,
    ) -> AppResult<InvoiceCustomerLink> {
        Ok(InvoiceCustomerLink::default())
    }

    /// Render an invoice PDF for email attachment. Pulls minimal company info
    /// from tenant settings; falls back to defaults so a rendering failure
    /// never blocks the email.
    fn render_invoice_pdf_for_email(
        &self,
        invoice: &Invoice,
        customer_name: Option<&str>,
        customer_email: Option<&str>,
    ) -> AppResult<Vec<u8>> {
        use crate::services::invoice_pdf_service::{
            InvoicePdfCompany, InvoicePdfContext, InvoicePdfCustomer, InvoicePdfLineItem,
            InvoicePdfTotals,
        };

        let total = invoice.amount;
        let amount_str = format!("{} {:.2}", invoice.currency_code, total);

        let company = InvoicePdfCompany {
            name: "ISPManagement".to_string(),
            address: None,
            ..InvoicePdfCompany::default()
        };
        let customer = InvoicePdfCustomer {
            name: customer_name.unwrap_or("Customer").to_string(),
            address: customer_email.map(|s| s.to_string()),
            ..InvoicePdfCustomer::default()
        };
        let items = vec![InvoicePdfLineItem {
            description: invoice
                .description
                .clone()
                .unwrap_or_else(|| format!("Invoice {}", invoice.invoice_number)),
            quantity: "1".to_string(),
            unit_price: amount_str.clone(),
            subtotal: amount_str.clone(),
        }];
        let totals = InvoicePdfTotals {
            subtotal: amount_str.clone(),
            tax_label: None,
            ..InvoicePdfTotals::default()
        };
        let due = invoice.due_date.format("%Y-%m-%d").to_string();
        let issued = invoice
            .due_date
            .checked_sub_signed(chrono::Duration::days(7))
            .unwrap_or(invoice.due_date)
            .format("%Y-%m-%d")
            .to_string();
        let status_label = invoice.status.to_uppercase();

        let ctx = InvoicePdfContext {
            company,
            customer,
            invoice_number: invoice.invoice_number.clone(),
            status_label,
            issued_at: issued,
            due_at: due,
            items,
            totals,
            payment_url: Some(format!("/pay/{}", invoice.id)),
            notes: None,
        };

        self.invoice_pdf_service.render_invoice(&ctx)
    }
}

/// Resolved customer-side info for an invoice. Used by `bulk_send_invoices`
/// to fan out across email + notification channels.
#[derive(Default, Debug, Clone)]
struct InvoiceCustomerLink {
    subscription_id: Option<String>,
    customer_email: Option<String>,
    customer_name: Option<String>,
    customer_phone: Option<String>,
}

/// Parse subscription ID from invoice `external_id`.
///
/// Format: `pkgsub:<subscription_uuid>:<billing_period>`
/// Example: `pkgsub:3bb3157a-86f5-443d-975d-f71e3fed01b0:2026-04`
///
/// Returns `None` if the external_id doesn't match the expected format.
fn parse_subscription_id_from_external_id(external_id: &str) -> Option<String> {
    let rest = external_id.strip_prefix("pkgsub:")?;
    let subscription_id = rest.split(':').next()?;
    let trimmed = subscription_id.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Validate it looks like a UUID (36 chars with hyphens)
    if trimmed.len() >= 36 && trimmed.contains('-') {
        Some(trimmed.to_string())
    } else {
        None
    }
}

impl PaymentService {
    /// Ensure a `customer_service_assignments` row exists for an invoice.
    ///
    /// Uses `INSERT ... ON CONFLICT DO NOTHING` so it's safe to call multiple
    /// times (idempotent). This is called during invoice generation so that
    /// bulk-send and other resolution paths work immediately.
    async fn ensure_customer_service_assignment_for_invoice(
        &self,
        tenant_id: &str,
        invoice_id: &str,
        subscription_id: &str,
        customer_id: &str,
    ) {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let result = sqlx::query(
            r#"
            INSERT INTO customer_service_assignments
              (id, tenant_id, invoice_id, subscription_id, customer_id, status, created_at, updated_at)
            SELECT $1, $2, $3, $4, $5, 'active', $6, $6
            WHERE NOT EXISTS (
              SELECT 1 FROM customer_service_assignments
              WHERE tenant_id = $2 AND invoice_id = $3
            )
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(invoice_id)
        .bind(subscription_id)
        .bind(customer_id)
        .bind(now)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(
                    "ensure_customer_service_assignment_for_invoice: failed for invoice {}: {}",
                    invoice_id, e
                );
            }
        }
    }
}

#[cfg(all(test, feature = "postgres"))]
mod integration_tests;
#[cfg(test)]
mod tests;
