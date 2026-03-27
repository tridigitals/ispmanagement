use crate::models::Invoice;
use crate::services::{AuthService, Claims, PaymentService};

async fn require_access(
    auth_service: &AuthService,
    claims: &Claims,
    resource: &str,
    action: &str,
) -> Result<(), String> {
    if claims.is_super_admin {
        return Ok(());
    }
    let tenant_id = claims
        .tenant_id
        .as_deref()
        .ok_or_else(|| "Tenant context required".to_string())?;
    auth_service
        .check_permission(&claims.sub, tenant_id, resource, action)
        .await
        .map_err(|e| e.to_string())
}

pub(super) async fn require_payment_read_access(
    auth_service: &AuthService,
    claims: &Claims,
) -> Result<(), String> {
    require_access(auth_service, claims, "billing", "read").await
}

pub(super) async fn require_payment_manage_access(
    auth_service: &AuthService,
    claims: &Claims,
) -> Result<(), String> {
    require_access(auth_service, claims, "billing", "manage").await
}

pub(super) async fn require_work_order_manage_access(
    auth_service: &AuthService,
    claims: &Claims,
) -> Result<(), String> {
    require_access(auth_service, claims, "work_orders", "manage").await
}

pub(super) async fn authorize_invoice_access(
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
