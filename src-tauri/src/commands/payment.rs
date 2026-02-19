//! Payment Commands

use crate::models::{BankAccount, CreateBankAccountRequest, Invoice};
use crate::services::{AuthService, BulkGenerateInvoicesResult, Claims, PaymentService, PlanService};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct FxRateResponse {
    pub base_currency: String,
    pub quote_currency: String,
    pub rate: f64,
    pub source: String,
    pub fetched_at: DateTime<Utc>,
}

async fn require_payment_read_access(
    auth_service: &AuthService,
    claims: &Claims,
) -> Result<(), String> {
    if claims.is_super_admin {
        return Ok(());
    }
    let tenant_id = claims
        .tenant_id
        .as_deref()
        .ok_or_else(|| "Tenant context required".to_string())?;
    auth_service
        .check_permission(&claims.sub, tenant_id, "billing", "read")
        .await
        .map_err(|e| e.to_string())
}

async fn require_payment_manage_access(
    auth_service: &AuthService,
    claims: &Claims,
) -> Result<(), String> {
    if claims.is_super_admin {
        return Ok(());
    }
    let tenant_id = claims
        .tenant_id
        .as_deref()
        .ok_or_else(|| "Tenant context required".to_string())?;
    auth_service
        .check_permission(&claims.sub, tenant_id, "billing", "manage")
        .await
        .map_err(|e| e.to_string())
}

async fn authorize_invoice_access(
    claims: &Claims,
    payment_service: &PaymentService,
    invoice_id: &str,
) -> Result<Invoice, String> {
    let invoice = payment_service
        .get_invoice(invoice_id)
        .await
        .map_err(|e| e.to_string())?;
    if claims.is_super_admin {
        return Ok(invoice);
    }
    let tenant_id = claims
        .tenant_id
        .as_deref()
        .ok_or_else(|| "Tenant context required".to_string())?;
    if tenant_id != invoice.tenant_id {
        return Err("Invoice access denied".to_string());
    }
    Ok(invoice)
}

fn is_customer_package_invoice(invoice: &Invoice) -> bool {
    invoice
        .external_id
        .as_deref()
        .map(|v| v.starts_with("pkgsub:"))
        .unwrap_or(false)
}

#[tauri::command]
pub async fn get_fx_rate(
    token: String,
    base_currency: String,
    quote_currency: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<FxRateResponse, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_read_access(&auth_service, &claims).await?;

    let base = base_currency.trim().to_uppercase();
    let quote = quote_currency.trim().to_uppercase();

    let (rate, fetched_at, source) = payment_service
        .get_fx_rate(&base, &quote, claims.tenant_id.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(FxRateResponse {
        base_currency: base,
        quote_currency: quote,
        rate,
        source,
        fetched_at,
    })
}

#[tauri::command]
pub async fn create_invoice_for_plan(
    token: String,
    plan_id: String,
    billing_cycle: String, // "monthly" or "yearly"
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Invoice, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_manage_access(&auth_service, &claims).await?;
    let tenant_id = claims.tenant_id.ok_or("No tenant context")?;
    let billing_cycle = billing_cycle.trim().to_ascii_lowercase();
    if billing_cycle != "monthly" && billing_cycle != "yearly" {
        return Err("billing_cycle must be monthly or yearly".to_string());
    }

    let plan = plan_service
        .get_plan(&plan_id)
        .await
        .map_err(|e| e.to_string())?;

    let amount = if billing_cycle == "yearly" {
        plan.price_yearly
    } else {
        plan.price_monthly
    };

    let desc = format!("{} Plan ({} billing)", plan.name, billing_cycle);

    // Store as "plan:plan_id:billing_cycle" in external_id
    let ext_id = format!("plan:{}:{}", plan_id, billing_cycle);

    payment_service
        .create_invoice(&tenant_id, amount, Some(desc), Some(ext_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_invoice(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Invoice, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_read_access(&auth_service, &claims).await?;
    authorize_invoice_access(&claims, &payment_service, &id).await
}

#[tauri::command]
pub async fn list_invoices(
    token: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Vec<Invoice>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_read_access(&auth_service, &claims).await?;
    let tenant_id = claims.tenant_id.ok_or("No tenant context")?;
    payment_service
        .list_invoices(Some(&tenant_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_customer_package_invoices(
    token: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Vec<Invoice>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_read_access(&auth_service, &claims).await?;
    let tenant_id = claims.tenant_id.ok_or("No tenant context")?;

    payment_service
        .list_customer_package_invoices(&tenant_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_invoice_for_customer_subscription(
    token: String,
    subscription_id: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Invoice, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_manage_access(&auth_service, &claims).await?;
    let tenant_id = claims.tenant_id.ok_or("No tenant context")?;

    payment_service
        .create_invoice_for_customer_subscription(&tenant_id, &subscription_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_due_customer_package_invoices(
    token: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<BulkGenerateInvoicesResult, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_manage_access(&auth_service, &claims).await?;
    let tenant_id = claims.tenant_id.ok_or("No tenant context")?;

    payment_service
        .generate_due_customer_package_invoices(&tenant_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_all_invoices(
    token: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Vec<Invoice>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }
    payment_service
        .list_invoices(None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pay_invoice_midtrans(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<String, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_manage_access(&auth_service, &claims).await?;
    let _ = authorize_invoice_access(&claims, &payment_service, &id).await?;
    payment_service
        .initiate_midtrans(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_payment_status(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<String, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_read_access(&auth_service, &claims).await?;
    let _ = authorize_invoice_access(&claims, &payment_service, &id).await?;
    payment_service
        .check_transaction_status(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_bank_accounts(
    token: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Vec<BankAccount>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_read_access(&auth_service, &claims).await?;

    payment_service
        .list_bank_accounts()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_bank_account(
    token: String,
    bank_name: String,
    account_number: String,
    account_holder: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<BankAccount, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let req = CreateBankAccountRequest {
        bank_name,
        account_number,
        account_holder,
    };

    payment_service
        .create_bank_account(req)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_bank_account(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    payment_service
        .delete_bank_account(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn submit_payment_proof(
    token: String,
    invoice_id: String,
    file_path: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_manage_access(&auth_service, &claims).await?;
    let _ = authorize_invoice_access(&claims, &payment_service, &invoice_id).await?;

    payment_service
        .submit_payment_proof(&invoice_id, &file_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn verify_payment(
    token: String,
    invoice_id: String,
    status: String,
    rejection_reason: Option<String>,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    payment_service
        .verify_payment(&invoice_id, &status, rejection_reason)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn verify_customer_package_payment(
    token: String,
    invoice_id: String,
    status: String,
    rejection_reason: Option<String>,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_manage_access(&auth_service, &claims).await?;
    let invoice = authorize_invoice_access(&claims, &payment_service, &invoice_id).await?;
    if !is_customer_package_invoice(&invoice) {
        return Err("Only customer package invoices can be verified here".to_string());
    }

    payment_service
        .verify_payment(&invoice_id, &status, rejection_reason)
        .await
        .map_err(|e| e.to_string())
}
