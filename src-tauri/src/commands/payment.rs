//! Payment Commands

mod access;
mod helpers;

use crate::models::{
    BankAccount, BillingCollectionLogView, CreateBankAccountRequest, Invoice,
    InvoiceReminderLogView,
};
use crate::services::{
    AuthService, BillingCollectionRunResult, BulkGenerateInvoicesResult, PaymentService,
    PlanService,
};
use access::{
    authorize_invoice_access, require_payment_manage_access, require_payment_read_access,
    require_work_order_manage_access,
};
use chrono::{DateTime, Utc};
use helpers::{is_customer_package_invoice, parse_datetime_opt};
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
    sort_by: Option<String>,
    sort_dir: Option<String>,
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
        .list_customer_package_invoices(&tenant_id, sort_by, sort_dir)
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
pub async fn create_invoice_for_installation_work_order(
    token: String,
    work_order_id: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Invoice, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_work_order_manage_access(&auth_service, &claims).await?;
    let tenant_id = claims.tenant_id.ok_or("No tenant context")?;

    payment_service
        .create_invoice_for_installation_work_order(&tenant_id, &work_order_id)
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
pub async fn list_billing_collection_logs(
    token: String,
    action: Option<String>,
    result: Option<String>,
    from: Option<String>,
    to: Option<String>,
    search: Option<String>,
    limit: Option<u32>,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Vec<BillingCollectionLogView>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_read_access(&auth_service, &claims).await?;
    let tenant_id = claims.tenant_id.ok_or("No tenant context")?;
    let from = parse_datetime_opt(from, "from")?;
    let to = parse_datetime_opt(to, "to")?;

    payment_service
        .list_billing_collection_logs(
            &tenant_id,
            action.as_deref(),
            result.as_deref(),
            from,
            to,
            search.as_deref(),
            limit.unwrap_or(200),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_invoice_reminder_logs(
    token: String,
    reminder_code: Option<String>,
    status: Option<String>,
    from: Option<String>,
    to: Option<String>,
    search: Option<String>,
    limit: Option<u32>,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Vec<InvoiceReminderLogView>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_read_access(&auth_service, &claims).await?;
    let tenant_id = claims.tenant_id.ok_or("No tenant context")?;
    let from = parse_datetime_opt(from, "from")?;
    let to = parse_datetime_opt(to, "to")?;

    payment_service
        .list_invoice_reminder_logs(
            &tenant_id,
            reminder_code.as_deref(),
            status.as_deref(),
            from,
            to,
            search.as_deref(),
            limit.unwrap_or(200),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_billing_collection_now(
    token: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<BillingCollectionRunResult, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    require_payment_manage_access(&auth_service, &claims).await?;
    let tenant_id = claims.tenant_id.ok_or("No tenant context")?;

    payment_service
        .run_billing_collection_now(&tenant_id)
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

#[cfg(test)]
mod tests {
    use super::{is_customer_package_invoice, parse_datetime_opt};
    use crate::models::Invoice;
    use chrono::{TimeZone, Utc};

    fn sample_invoice(external_id: Option<&str>) -> Invoice {
        let ts = Utc
            .with_ymd_and_hms(2026, 3, 27, 0, 0, 0)
            .single()
            .expect("valid UTC timestamp");

        Invoice {
            id: "inv-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            invoice_number: "INV-1".to_string(),
            amount: 100_000.0,
            currency_code: "IDR".to_string(),
            base_currency_code: "IDR".to_string(),
            fx_rate: None,
            fx_source: None,
            fx_fetched_at: None,
            status: "pending".to_string(),
            description: Some("sample".to_string()),
            due_date: ts,
            paid_at: None,
            payment_method: None,
            external_id: external_id.map(str::to_string),
            merchant_id: None,
            proof_attachment: None,
            rejection_reason: None,
            created_at: ts,
            updated_at: ts,
        }
    }

    #[test]
    fn parse_datetime_opt_accepts_none_and_blank_as_absent() {
        let none_result = parse_datetime_opt(None, "from").expect("none should parse");
        let blank_result = parse_datetime_opt(Some("   ".to_string()), "from")
            .expect("blank should parse as absent");

        assert_eq!(none_result, None);
        assert_eq!(blank_result, None);
    }

    #[test]
    fn parse_datetime_opt_parses_rfc3339_and_normalizes_to_utc() {
        let parsed = parse_datetime_opt(Some("2026-03-27T07:00:00+07:00".to_string()), "from")
            .expect("datetime should parse")
            .expect("datetime should be present");

        assert_eq!(parsed, Utc.with_ymd_and_hms(2026, 3, 27, 0, 0, 0).unwrap());
    }

    #[test]
    fn parse_datetime_opt_rejects_invalid_format_with_field_specific_message() {
        let err = parse_datetime_opt(Some("2026/03/27 00:00:00".to_string()), "to")
            .expect_err("invalid datetime must fail");

        assert_eq!(err, "to must be ISO-8601 datetime (RFC3339)");
    }

    #[test]
    fn is_customer_package_invoice_only_accepts_pkgsub_prefixed_external_id() {
        let customer_package = sample_invoice(Some("pkgsub:sub-1"));
        let tenant_plan = sample_invoice(Some("plan:plan-1:monthly"));
        let no_external_id = sample_invoice(None);

        assert!(is_customer_package_invoice(&customer_package));
        assert!(!is_customer_package_invoice(&tenant_plan));
        assert!(!is_customer_package_invoice(&no_external_id));
    }
}
