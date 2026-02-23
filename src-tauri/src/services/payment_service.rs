//! Payment Service - Manages invoices and bank accounts

use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    BankAccount, BillingCollectionLogView, CreateBankAccountRequest, Invoice, InvoiceReminderLogView,
};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Datelike, Duration, Months, Utc};
use reqwest::Client;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha512};
use std::collections::HashSet;
use uuid::Uuid;

use crate::services::NotificationService;

const CUSTOMER_PACKAGE_INVOICE_PREFIX: &str = "pkgsub:";
const BILLING_AUTO_SUSPEND_ENABLED_KEY: &str = "billing_auto_suspend_enabled";
const BILLING_AUTO_SUSPEND_GRACE_DAYS_KEY: &str = "billing_auto_suspend_grace_days";
const BILLING_AUTO_RESUME_ON_PAYMENT_KEY: &str = "billing_auto_resume_on_payment";
const BILLING_REMINDER_ENABLED_KEY: &str = "billing_reminder_enabled";
const BILLING_REMINDER_SCHEDULE_KEY: &str = "billing_reminder_schedule";

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

#[derive(Debug, Clone, Serialize)]
pub struct BillingCollectionSettings {
    pub auto_suspend_enabled: bool,
    pub auto_suspend_grace_days: i64,
    pub auto_resume_on_payment: bool,
    pub reminder_enabled: bool,
    pub reminder_schedule: Vec<String>,
}

impl Default for BillingCollectionSettings {
    fn default() -> Self {
        Self {
            auto_suspend_enabled: false,
            auto_suspend_grace_days: 3,
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
}

impl PaymentService {
    pub fn new(pool: DbPool, notification_service: NotificationService) -> Self {
        Self {
            pool,
            http_client: Client::new(),
            notification_service,
        }
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
        let id = Uuid::new_v4().to_string();
        // Simple invoice number generation: INV-YYYYMMDD-HHMMSS
        let now = Utc::now();
        let invoice_number = format!("INV-{}", now.format("%Y%m%d-%H%M%S"));

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
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            INSERT INTO invoices (
                id, tenant_id, invoice_number, amount, currency_code, base_currency_code, fx_rate, fx_source, fx_fetched_at,
                status, description, due_date, external_id, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending', $10, $11, $12, $13, $13)
            RETURNING
                id, tenant_id, invoice_number,
                amount::FLOAT8 as amount,
                currency_code, base_currency_code,
                fx_rate::FLOAT8 as fx_rate, fx_source, fx_fetched_at,
                status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, created_at, updated_at
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
        .bind(now + chrono::Duration::days(1))
        .bind(&external_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let invoice = {
            sqlx::query(
                r#"
                INSERT INTO invoices (
                    id, tenant_id, invoice_number, amount, currency_code, base_currency_code, fx_rate, fx_source, fx_fetched_at,
                    status, description, due_date, external_id, created_at, updated_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?, ?, ?, ?)
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
            .bind((now + chrono::Duration::days(1)).to_rfc3339())
            .bind(&external_id)
            .bind(now.to_rfc3339()).bind(now.to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            self.get_invoice(&id).await?
        };

        Ok(invoice)
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
                status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, created_at, updated_at
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
                    status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, created_at, updated_at
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
                    status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, created_at, updated_at
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

    pub async fn list_customer_package_invoices(&self, tenant_id: &str) -> AppResult<Vec<Invoice>> {
        #[cfg(feature = "postgres")]
        let invoices = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT
                id, tenant_id, invoice_number,
                amount::FLOAT8 as amount,
                currency_code, base_currency_code,
                COALESCE(fx_rate, 1.0)::FLOAT8 as fx_rate, fx_source, fx_fetched_at,
                status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, created_at, updated_at
            FROM invoices
            WHERE tenant_id = $1 AND external_id LIKE $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(format!("{}%", CUSTOMER_PACKAGE_INVOICE_PREFIX))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let invoices = sqlx::query_as::<_, Invoice>(
            "SELECT * FROM invoices WHERE tenant_id = ? AND external_id LIKE ? ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .bind(format!("{}%", CUSTOMER_PACKAGE_INVOICE_PREFIX))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(invoices)
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
                i.status, i.description, i.due_date, i.paid_at, i.payment_method, i.proof_attachment, i.external_id, i.merchant_id, i.created_at, i.updated_at
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

    pub async fn create_invoice_for_customer_subscription(
        &self,
        tenant_id: &str,
        subscription_id: &str,
    ) -> AppResult<Invoice> {
        self.create_invoice_for_customer_subscription_at(tenant_id, subscription_id, Utc::now())
            .await
    }

    async fn create_invoice_for_customer_subscription_at(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        period_ref: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<Invoice> {
        #[cfg(feature = "postgres")]
        let row: Option<(
            String,
            String,
            String,
            f64,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        )> = sqlx::query_as(
            r#"
            SELECT
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
            f64,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        )> = sqlx::query_as(
            r#"
            SELECT
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

        let (customer_name, package_name, billing_cycle, price, starts_at, ends_at) =
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
            CUSTOMER_PACKAGE_INVOICE_PREFIX, subscription_id, period_key
        );

        #[cfg(feature = "postgres")]
        let exists_current_period: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM invoices
                WHERE tenant_id = $1
                  AND external_id = $2
            )
            "#,
        )
        .bind(tenant_id)
        .bind(&external_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let exists_current_period: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM invoices
                WHERE tenant_id = ?
                  AND external_id = ?
            )
            "#,
        )
        .bind(tenant_id)
        .bind(&external_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if exists_current_period {
            return Err(AppError::Validation(
                "Invoice for current billing period already exists".to_string(),
            ));
        }

        let description = format!(
            "Customer {} - {} ({} billing, period {})",
            customer_name, package_name, billing_cycle, period_key
        );

        self.create_invoice(tenant_id, price, Some(description), Some(external_id))
            .await
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
                    tracing::warn!(
                        "billing collection tenant {} failed: {}",
                        tenant_id,
                        e
                    );
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
                Self::parse_customer_subscription_id(external_id.as_deref())
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
            let reminder_code = Self::reminder_code_for_day_offset(day_offset);

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

            if settings.auto_suspend_enabled && day_offset >= settings.auto_suspend_grace_days {
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
                        let _ = self
                            .notify_subscription_suspension(
                                tenant_id,
                                &subscription_id,
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

        // 1. Fetch Settings (Context Aware)
        // If merchant_id is present, use Tenant's keys. Otherwise, use Global (System) keys.
        let (server_key, is_production) = if let Some(mid) = &invoice.merchant_id {
            // Tenant Merchant
            let sk: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'payment_midtrans_server_key' AND tenant_id = $1")
                .bind(mid).fetch_optional(&self.pool).await.unwrap_or(None).unwrap_or_default();

            let prod_str: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'payment_midtrans_is_production' AND tenant_id = $1")
                .bind(mid).fetch_optional(&self.pool).await.unwrap_or(None).unwrap_or("false".to_string());

            (sk, prod_str == "true")
        } else {
            // System Merchant (Global)
            let sk: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'payment_midtrans_server_key' AND tenant_id IS NULL")
                .fetch_optional(&self.pool).await.unwrap_or(None).unwrap_or_default();

            let prod_str: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'payment_midtrans_is_production' AND tenant_id IS NULL")
                .fetch_optional(&self.pool).await.unwrap_or(None).unwrap_or("false".to_string());

            (sk, prod_str == "true")
        };

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

    // ==================== BANK ACCOUNTS ====================

    /// Check Transaction Status (Manual/Poll)
    pub async fn check_transaction_status(&self, invoice_id: &str) -> AppResult<String> {
        let invoice = self.get_invoice(invoice_id).await?;

        // 1. Fetch Settings (Context Aware)
        let (server_key, is_production) = if let Some(mid) = &invoice.merchant_id {
            let sk: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'payment_midtrans_server_key' AND tenant_id = $1")
                .bind(mid).fetch_optional(&self.pool).await.unwrap_or(None).unwrap_or_default();

            let prod_str: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'payment_midtrans_is_production' AND tenant_id = $1")
                .bind(mid).fetch_optional(&self.pool).await.unwrap_or(None).unwrap_or("false".to_string());

            (sk, prod_str == "true")
        } else {
            let sk: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'payment_midtrans_server_key' AND tenant_id IS NULL")
                .fetch_optional(&self.pool).await.unwrap_or(None).unwrap_or_default();

            let prod_str: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'payment_midtrans_is_production' AND tenant_id IS NULL")
                .fetch_optional(&self.pool).await.unwrap_or(None).unwrap_or("false".to_string());

            (sk, prod_str == "true")
        };

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
            self.process_midtrans_notification(&invoice.invoice_number, payment_status)
                .await?;
        }

        Ok(payment_status.to_string())
    }

    /// List all bank accounts
    pub async fn list_bank_accounts(&self) -> Result<Vec<BankAccount>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let accounts = sqlx::query_as("SELECT * FROM bank_accounts ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let accounts = sqlx::query_as("SELECT * FROM bank_accounts ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;

        Ok(accounts)
    }

    /// Create a new bank account
    pub async fn create_bank_account(
        &self,
        req: CreateBankAccountRequest,
    ) -> Result<BankAccount, sqlx::Error> {
        println!(
            "Creating bank account: {} - {}",
            req.bank_name, req.account_number
        );
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO bank_accounts (id, bank_name, account_number, account_holder, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#
        )
        .bind(&id)
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
            INSERT INTO bank_accounts (id, bank_name, account_number, account_holder, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&id)
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
    ) -> AppResult<()> {
        // 1. Get Invoice
        #[cfg(feature = "postgres")]
        let invoice: Option<Invoice> = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT
                id, tenant_id, invoice_number,
                amount::FLOAT8 as amount,
                currency_code, base_currency_code,
                fx_rate::FLOAT8 as fx_rate, fx_source, fx_fetched_at,
                status, description, due_date, paid_at, payment_method, proof_attachment, external_id, merchant_id, created_at, updated_at
            FROM invoices WHERE invoice_number = $1
            "#
        )
        .bind(invoice_number)
        .fetch_optional(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let invoice: Option<Invoice> =
            sqlx::query_as("SELECT * FROM invoices WHERE invoice_number = ?")
                .bind(invoice_number)
                .fetch_optional(&self.pool)
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
        if current_status == status {
            println!(
                "DEBUG: Duplicate Midtrans notification ignored. Invoice={}, status={}",
                invoice.invoice_number, status
            );
            return Ok(());
        }

        if current_status == "paid" && status != "paid" {
            println!(
                "DEBUG: Ignoring Midtrans status downgrade. Invoice={}, current={}, incoming={}",
                invoice.invoice_number, current_status, status
            );
            return Ok(());
        }

        if current_status == "failed" && status == "pending" {
            println!(
                "DEBUG: Ignoring Midtrans pending after failed. Invoice={}",
                invoice.invoice_number
            );
            return Ok(());
        }

        // 2. Update Status
        let now = Utc::now();
        let paid_at = if status == "paid" { Some(now) } else { None };

        #[cfg(feature = "postgres")]
        sqlx::query("UPDATE invoices SET status = $1, paid_at = $2, updated_at = $3 WHERE id = $4")
            .bind(status)
            .bind(paid_at)
            .bind(now)
            .bind(&invoice.id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        {
            let paid_str = paid_at.map(|t| t.to_rfc3339());
            sqlx::query("UPDATE invoices SET status = ?, paid_at = ?, updated_at = ? WHERE id = ?")
                .bind(status)
                .bind(paid_str)
                .bind(now.to_rfc3339())
                .bind(&invoice.id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

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
                        "DEBUG: Skipping SaaS subscription activation for customer package invoice {}",
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

        // 4. Send Notification to Tenant Users
        if status == "paid" || status == "failed" {
            // Get all users in this tenant
            #[cfg(feature = "postgres")]
            let users: Vec<(String,)> =
                sqlx::query_as("SELECT user_id FROM tenant_members WHERE tenant_id = $1")
                    .bind(&invoice.tenant_id)
                    .fetch_all(&self.pool)
                    .await
                    .unwrap_or_default();

            #[cfg(feature = "sqlite")]
            let users: Vec<(String,)> =
                sqlx::query_as("SELECT user_id FROM tenant_members WHERE tenant_id = ?")
                    .bind(&invoice.tenant_id)
                    .fetch_all(&self.pool)
                    .await
                    .unwrap_or_default();

            let title = if status == "paid" {
                "Payment Successful".to_string()
            } else {
                "Payment Failed".to_string()
            };
            let message = if status == "paid" {
                format!(
                    "Invoice {} has been successfully paid. Thank you!",
                    invoice.invoice_number
                )
            } else {
                format!(
                    "Payment for invoice {} failed. Please check your payment method.",
                    invoice.invoice_number
                )
            };

            for (user_id,) in users {
                // Fire and forget notification
                let _ = self
                    .notification_service
                    .create_notification(
                        user_id,
                        Some(invoice.tenant_id.clone()),
                        title.clone(),
                        message.clone(),
                        "info".to_string(),                      // type
                        "billing".to_string(),                   // category
                        Some("/admin/subscription".to_string()), // action_url
                    )
                    .await;
            }
        }

        // 5. Notify Superadmins (New Sale Alert)
        if status == "paid" {
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
                        None, // System notification
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
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query("UPDATE invoices SET status = 'verification_pending', proof_attachment = $1, updated_at = $2 WHERE id = $3")
            .bind(file_path)
            .bind(now)
            .bind(invoice_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE invoices SET status = 'verification_pending', proof_attachment = ?, updated_at = ? WHERE id = ?")
            .bind(file_path)
            .bind(now.to_rfc3339())
            .bind(invoice_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // Notify Admins about new proof
        // TODO: This should ideally notify Superadmins.
        // Reusing the same Superadmin notification logic could be good here.
        // For now, let's keep it simple and just update the status.
        // We will add a Notification Trigger next.

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
                    None,
                    "New Payment Proof Uploaded".to_string(),
                    format!(
                        "A payment proof has been uploaded for invoice {}",
                        invoice_id
                    ),
                    "info".to_string(),
                    "billing".to_string(),
                    Some("/superadmin/invoices".to_string()),
                )
                .await;
        }

        Ok(())
    }

    /// Verify Payment (Approve/Reject)
    pub async fn verify_payment(
        &self,
        invoice_id: &str,
        status: &str,
        _rejection_reason: Option<String>,
    ) -> AppResult<()> {
        if status != "paid" && status != "failed" {
            return Err(AppError::Validation(
                "Status must be 'paid' or 'failed'".to_string(),
            ));
        }

        // 1. Get Invoice to reuse existing logic
        let invoice = self.get_invoice(invoice_id).await?;

        // 2. Reuse process_midtrans_notification logic
        // process_midtrans_notification(&self, invoice: &Invoice, status: &str)
        self.process_midtrans_notification(&invoice.invoice_number, status)
            .await?;

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

    fn parse_customer_subscription_id(external_id: Option<&str>) -> Option<String> {
        let rest = external_id?.strip_prefix(CUSTOMER_PACKAGE_INVOICE_PREFIX)?;
        let id = rest.split(':').next()?.trim();
        if id.is_empty() {
            return None;
        }
        Some(id.to_string())
    }

    fn reminder_code_for_day_offset(day_offset: i64) -> String {
        if day_offset >= 0 {
            format!("H+{}", day_offset)
        } else {
            format!("H{}", day_offset)
        }
    }

    async fn try_auto_resume_customer_subscription_from_paid_invoice(
        &self,
        invoice: &Invoice,
    ) -> AppResult<()> {
        let Some(subscription_id) =
            Self::parse_customer_subscription_id(invoice.external_id.as_deref())
        else {
            return Ok(());
        };

        let settings = self
            .resolve_billing_collection_settings(Some(&invoice.tenant_id))
            .await;
        if !settings.auto_resume_on_payment {
            let _ = self
                .insert_billing_collection_log(
                    &invoice.tenant_id,
                    &invoice.id,
                    Some(&subscription_id),
                    "resume",
                    "skipped",
                    Some("Auto resume disabled by setting"),
                    "system",
                    None,
                )
                .await;
            return Ok(());
        }

        match self
            .update_customer_subscription_status_if(
                &invoice.tenant_id,
                &subscription_id,
                "suspended",
                "active",
            )
            .await
        {
            Ok(true) => {
                let _ = self
                    .insert_billing_collection_log(
                        &invoice.tenant_id,
                        &invoice.id,
                        Some(&subscription_id),
                        "resume",
                        "success",
                        Some("Subscription resumed because invoice is paid"),
                        "system",
                        None,
                    )
                    .await;
                let _ = self
                    .notify_subscription_resumed(
                        &invoice.tenant_id,
                        &subscription_id,
                        &invoice.invoice_number,
                    )
                    .await;
            }
            Ok(false) => {
                let _ = self
                    .insert_billing_collection_log(
                        &invoice.tenant_id,
                        &invoice.id,
                        Some(&subscription_id),
                        "resume",
                        "skipped",
                        Some("Subscription is not suspended"),
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
                        "resume",
                        "failed",
                        Some(&e.to_string()),
                        "system",
                        None,
                    )
                    .await;
                return Err(e);
            }
        }

        Ok(())
    }

    async fn send_invoice_reminder(
        &self,
        tenant_id: &str,
        subscription_id: &str,
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
                    Some("/dashboard/invoices".to_string()),
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
                    Some("/dashboard/invoices".to_string()),
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
                    Some("/dashboard/invoices".to_string()),
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

        let mut merged: HashSet<String> = customer_user_ids.into_iter().collect();
        if merged.is_empty() {
            for user_id in self.list_tenant_member_user_ids(tenant_id).await? {
                merged.insert(user_id);
            }
        }

        Ok(merged.into_iter().collect())
    }

    async fn list_tenant_member_user_ids(&self, tenant_id: &str) -> AppResult<Vec<String>> {
        #[cfg(feature = "postgres")]
        let rows: Vec<String> = sqlx::query_scalar("SELECT user_id FROM tenant_members WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<String> = sqlx::query_scalar("SELECT user_id FROM tenant_members WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows)
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
            return Err(AppError::NotFound("Customer subscription not found".to_string()));
        };

        if current_status != expected_status {
            return Ok(false);
        }

        let now = Utc::now();
        #[cfg(feature = "postgres")]
        let rows = sqlx::query(
            "UPDATE customer_subscriptions SET status = $1, updated_at = $2 WHERE tenant_id = $3 AND id = $4",
        )
        .bind(new_status)
        .bind(now)
        .bind(tenant_id)
        .bind(subscription_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .rows_affected();

        #[cfg(feature = "sqlite")]
        let rows = sqlx::query(
            "UPDATE customer_subscriptions SET status = ?, updated_at = ? WHERE tenant_id = ? AND id = ?",
        )
        .bind(new_status)
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(subscription_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .rows_affected();

        Ok(rows > 0)
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

        let auto_suspend_grace_days = Self::parse_i64_setting(
            self.get_setting_value_fallback(tenant_id, BILLING_AUTO_SUSPEND_GRACE_DAYS_KEY)
                .await,
            defaults.auto_suspend_grace_days,
            0,
            365,
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
            auto_suspend_grace_days,
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
}
