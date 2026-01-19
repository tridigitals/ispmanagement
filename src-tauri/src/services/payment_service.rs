//! Payment Service - Manages invoices and bank accounts

use crate::db::DbPool;
use crate::models::{BankAccount, CreateBankAccountRequest, Invoice};
use crate::error::{AppError, AppResult};
use chrono::Utc;
use uuid::Uuid;
use reqwest::Client;
use serde_json::json;
use base64::{Engine as _, engine::general_purpose};

#[derive(Clone)]
pub struct PaymentService {
    pool: DbPool,
    http_client: Client,
}

impl PaymentService {
    pub fn new(pool: DbPool) -> Self {
        Self { 
            pool,
            http_client: Client::new(),
        }
    }

    // ==================== INVOICES ====================

    /// Create a new invoice
    pub async fn create_invoice(&self, tenant_id: &str, amount: f64, description: Option<String>, external_id: Option<String>) -> AppResult<Invoice> {
        let id = Uuid::new_v4().to_string();
        // Simple invoice number generation: INV-YYYYMMDD-HHMMSS
        let now = Utc::now();
        let invoice_number = format!("INV-{}", now.format("%Y%m%d-%H%M%S"));

        #[cfg(feature = "postgres")]
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            INSERT INTO invoices (id, tenant_id, invoice_number, amount, status, description, due_date, external_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 'pending', $5, $6, $7, $8, $8)
            RETURNING id, tenant_id, invoice_number, amount::FLOAT8 as amount, status, description, due_date, paid_at, payment_method, external_id, merchant_id, created_at, updated_at
            "#
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(&invoice_number)
        .bind(amount)
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
                INSERT INTO invoices (id, tenant_id, invoice_number, amount, status, description, due_date, external_id, created_at, updated_at)
                VALUES (?, ?, ?, ?, 'pending', ?, ?, ?, ?, ?)
                "#
            )
            .bind(&id).bind(tenant_id).bind(&invoice_number).bind(amount).bind(&description)
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
            SELECT id, tenant_id, invoice_number, amount::FLOAT8 as amount, status, description, due_date, paid_at, payment_method, external_id, merchant_id, created_at, updated_at 
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
                SELECT id, tenant_id, invoice_number, amount::FLOAT8 as amount, status, description, due_date, paid_at, payment_method, external_id, merchant_id, created_at, updated_at 
                FROM invoices WHERE tenant_id = $1 ORDER BY created_at DESC
                "#
            )
            .bind(tid)
            .fetch_all(&self.pool).await
        } else {
            sqlx::query_as::<_, Invoice>(
                r#"
                SELECT id, tenant_id, invoice_number, amount::FLOAT8 as amount, status, description, due_date, paid_at, payment_method, external_id, merchant_id, created_at, updated_at 
                FROM invoices ORDER BY created_at DESC
                "#
            )
            .fetch_all(&self.pool).await
        }.map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let invoices = if let Some(tid) = tenant_id {
            sqlx::query_as::<_, Invoice>("SELECT * FROM invoices WHERE tenant_id = ? ORDER BY created_at DESC")
                .bind(tid).fetch_all(&self.pool).await
        } else {
            sqlx::query_as::<_, Invoice>("SELECT * FROM invoices ORDER BY created_at DESC")
                .fetch_all(&self.pool).await
        }.map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(invoices)
    }

    /// Initiate Midtrans Payment (Get Snap Token)
    pub async fn initiate_midtrans(&self, invoice_id: &str) -> AppResult<String> {
        let invoice = self.get_invoice(invoice_id).await?;

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
        
        let app_url: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'app_public_url' AND tenant_id IS NULL")
            .fetch_optional(&self.pool).await.unwrap_or(None).unwrap_or("http://localhost:3000".to_string());
        
        if server_key.is_empty() {
            return Err(AppError::Configuration("Midtrans Server Key not configured for this merchant".to_string()));
        }

        // 2. Prepare API URL
        let base_url = if is_production {
            "https://app.midtrans.com/snap/v1/transactions"
        } else {
            "https://app.sandbox.midtrans.com/snap/v1/transactions"
        };

        // Construct Webhook URL for Override
        let webhook_url = format!("{}/api/payment/midtrans/notification", app_url.trim_end_matches('/'));

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

        let res = self.http_client.post(base_url)
            .header("Authorization", format!("Basic {}", auth_b64))
            .header("Content-Type", "application/json")
            .header("X-Override-Notification", webhook_url) // Override Webhook URL
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Midtrans API Req Failed: {}", e)))?;

        let resp_json: serde_json::Value = res.json().await
            .map_err(|e| AppError::Internal(format!("Midtrans API Parse Failed: {}", e)))?;

        if let Some(token) = resp_json.get("token").and_then(|v| v.as_str()) {
            Ok(token.to_string())
        } else {
            Err(AppError::Internal(format!("Midtrans Error: {:?}", resp_json)))
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
            return Err(AppError::Configuration("Midtrans Server Key not configured".to_string()));
        }

        // 2. Prepare API URL (Core API)
        let base_url = if is_production {
            format!("https://api.midtrans.com/v2/{}/status", invoice.invoice_number)
        } else {
            format!("https://api.sandbox.midtrans.com/v2/{}/status", invoice.invoice_number)
        };

        // 3. Send Request
        let auth_header = format!("{}:", server_key);
        let auth_b64 = general_purpose::STANDARD.encode(auth_header);

        let res = self.http_client.get(&base_url)
            .header("Authorization", format!("Basic {}", auth_b64))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Midtrans API Req Failed: {}", e)))?;

        let resp_json: serde_json::Value = res.json().await
            .map_err(|e| AppError::Internal(format!("Midtrans API Parse Failed: {}", e)))?;

        // 4. Parse Status
        let transaction_status = resp_json["transaction_status"].as_str().unwrap_or("pending");
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
            self.process_midtrans_notification(&invoice.invoice_number, payment_status).await?;
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
    pub async fn create_bank_account(&self, req: CreateBankAccountRequest) -> Result<BankAccount, sqlx::Error> {
        println!("Creating bank account: {} - {}", req.bank_name, req.account_number);
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
    pub async fn process_midtrans_notification(&self, invoice_number: &str, status: &str) -> AppResult<()> {
        // 1. Get Invoice
        #[cfg(feature = "postgres")]
        let invoice: Option<Invoice> = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT id, tenant_id, invoice_number, amount::FLOAT8 as amount, status, description, due_date, paid_at, payment_method, external_id, merchant_id, created_at, updated_at 
            FROM invoices WHERE invoice_number = $1
            "#
        )
        .bind(invoice_number)
        .fetch_optional(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let invoice: Option<Invoice> = sqlx::query_as("SELECT * FROM invoices WHERE invoice_number = ?")
            .bind(invoice_number).fetch_optional(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let invoice = match invoice {
            Some(i) => i,
            None => return Err(AppError::NotFound(format!("Invoice {} not found", invoice_number))),
        };

        // 2. Update Status
        let now = Utc::now();
        let paid_at = if status == "paid" { Some(now) } else { None };

        #[cfg(feature = "postgres")]
        sqlx::query(
            "UPDATE invoices SET status = $1, paid_at = $2, updated_at = $3 WHERE id = $4"
        )
        .bind(status).bind(paid_at).bind(now).bind(&invoice.id)
        .execute(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        {
            let paid_str = paid_at.map(|t| t.to_rfc3339());
            sqlx::query(
                "UPDATE invoices SET status = ?, paid_at = ?, updated_at = ? WHERE id = ?"
            )
            .bind(status).bind(paid_str).bind(now.to_rfc3339()).bind(&invoice.id)
            .execute(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;
        }

        // 3. Activate Subscription if Paid
        if status == "paid" {
            // external_id stores `plan_id:billing_cycle`
            if let Some(ext_id) = &invoice.external_id {
                let parts: Vec<&str> = ext_id.split(':').collect();
                if parts.len() == 2 {
                    let plan_id = parts[0];
                    let cycle = parts[1];
                    self.activate_subscription(&invoice.tenant_id, plan_id, cycle).await?;
                } else {
                    // Fallback for legacy records
                    self.activate_subscription(&invoice.tenant_id, ext_id, "monthly").await?;
                }
            }
        }

        Ok(())
    }

    async fn activate_subscription(&self, tenant_id: &str, plan_id: &str, billing_cycle: &str) -> AppResult<()> {
        let now = Utc::now();
        
        // Calculate end date based on cycle
        let end_date = if billing_cycle == "yearly" {
            now + chrono::Duration::days(365)
        } else {
            now + chrono::Duration::days(30)
        };

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO tenant_subscriptions (id, tenant_id, plan_id, status, current_period_start, current_period_end, created_at, updated_at)
            VALUES ($1, $2, $3, 'active', $4, $5, $6, $6)
            ON CONFLICT (tenant_id) DO UPDATE SET 
                plan_id = $3, 
                status = 'active',
                current_period_start = $4,
                current_period_end = $5,
                updated_at = $6
            "#
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(plan_id)
        .bind(now)
        .bind(Some(end_date))
        .bind(now)
        .execute(&self.pool)
        .await.map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO tenant_subscriptions (id, tenant_id, plan_id, status, current_period_start, current_period_end, created_at, updated_at)
            VALUES (?, ?, ?, 'active', ?, ?, ?, ?)
            ON CONFLICT (tenant_id) DO UPDATE SET 
                plan_id = excluded.plan_id, 
                status = 'active',
                current_period_start = excluded.current_period_start,
                current_period_end = excluded.current_period_end,
                updated_at = excluded.updated_at
            "#
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(plan_id)
        .bind(now.to_rfc3339())
        .bind(end_date.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(())
    }
}