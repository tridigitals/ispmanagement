//! Payment Commands

use crate::services::{AuthService, PaymentService, PlanService};
use crate::models::{BankAccount, CreateBankAccountRequest, Invoice};
use tauri::State;

#[tauri::command]
pub async fn create_invoice_for_plan(
    token: String,
    plan_id: String,
    billing_cycle: String, // "monthly" or "yearly"
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Invoice, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims.tenant_id.ok_or("No tenant context")?;

    let plan = plan_service.get_plan(&plan_id).await.map_err(|e| e.to_string())?;
    
    let amount = if billing_cycle == "yearly" {
        plan.price_yearly
    } else {
        plan.price_monthly
    };

    let desc = format!("{} Plan ({} billing)", plan.name, billing_cycle);

    // Store as "plan_id:billing_cycle" in external_id
    let ext_id = format!("{}:{}", plan_id, billing_cycle);

    payment_service.create_invoice(&tenant_id, amount, Some(desc), Some(ext_id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_invoice(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Invoice, String> {
    let _ = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    // Add logic to ensure user owns invoice (tenant check)
    payment_service.get_invoice(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_invoices(
    token: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Vec<Invoice>, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims.tenant_id.ok_or("No tenant context")?;
    payment_service.list_invoices(Some(&tenant_id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_all_invoices(
    token: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Vec<Invoice>, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }
    payment_service.list_invoices(None).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pay_invoice_midtrans(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<String, String> {
    let _ = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    payment_service.initiate_midtrans(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_payment_status(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<String, String> {
    let _ = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    payment_service.check_transaction_status(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_bank_accounts(
    token: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<Vec<BankAccount>, String> {
    // Basic auth check
    let _ = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    // In future, maybe restrict to superadmin or allow tenants to see them?
    // Tenants need to SEE them to pay. Superadmin needs to MANAGE them.
    // For list, let's allow authenticated users.
    
    payment_service.list_bank_accounts().await.map_err(|e| e.to_string())
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
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let req = CreateBankAccountRequest {
        bank_name,
        account_number,
        account_holder,
    };

    payment_service.create_bank_account(req).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_bank_account(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
    payment_service: State<'_, PaymentService>,
) -> Result<(), String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    payment_service.delete_bank_account(&id).await.map_err(|e| e.to_string())
}
