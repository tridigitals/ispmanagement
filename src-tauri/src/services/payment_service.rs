//! Payment Service - Manages invoices and bank accounts

use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{BankAccount, CreateBankAccountRequest, Invoice};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Datelike, Duration, Months, Utc};
use reqwest::Client;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha512};
use uuid::Uuid;

use crate::services::NotificationService;

const CUSTOMER_PACKAGE_INVOICE_PREFIX: &str = "pkgsub:";

#[derive(Debug, Clone, Serialize)]
pub struct BulkGenerateInvoicesResult {
    pub created_count: u32,
    pub skipped_count: u32,
    pub failed_count: u32,
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
