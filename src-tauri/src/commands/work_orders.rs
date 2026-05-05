use crate::models::{
    InstallationWorkOrder, WorkOrderRescheduleDecisionRequest, WorkOrderRescheduleRequestView,
};
use crate::services::{AuthService, CustomerService};
use tauri::State;

async fn tenant_and_claims(
    auth: &AuthService,
    token: &str,
) -> Result<(crate::services::auth_service::Claims, String), String> {
    let claims = auth
        .validate_token(token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    Ok((claims, tenant_id))
}

#[tauri::command]
pub async fn reopen_installation_work_order(
    token: String,
    id: String,
    notes: Option<String>,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<InstallationWorkOrder, String> {
    let (claims, tenant_id) = tenant_and_claims(&auth, &token).await?;
    customers
        .reopen_installation_work_order(&claims.sub, &tenant_id, &id, notes, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_pending_work_order_reschedule_request(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<Option<WorkOrderRescheduleRequestView>, String> {
    let (claims, tenant_id) = tenant_and_claims(&auth, &token).await?;
    customers
        .get_pending_work_order_reschedule_request(&claims.sub, &tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn approve_work_order_reschedule_request(
    token: String,
    id: String,
    scheduled_at: Option<String>,
    notes: Option<String>,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<InstallationWorkOrder, String> {
    let (claims, tenant_id) = tenant_and_claims(&auth, &token).await?;
    customers
        .approve_work_order_reschedule_request(
            &claims.sub,
            &tenant_id,
            &id,
            WorkOrderRescheduleDecisionRequest {
                scheduled_at,
                notes,
            },
            Some("127.0.0.1"),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reject_work_order_reschedule_request(
    token: String,
    id: String,
    notes: Option<String>,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<InstallationWorkOrder, String> {
    let (claims, tenant_id) = tenant_and_claims(&auth, &token).await?;
    customers
        .reject_work_order_reschedule_request(
            &claims.sub,
            &tenant_id,
            &id,
            WorkOrderRescheduleDecisionRequest {
                scheduled_at: None,
                notes,
            },
            Some("127.0.0.1"),
        )
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn work_order_commands_are_defined() {
        let source = include_str!("work_orders.rs");

        assert!(source.contains("pub async fn reopen_installation_work_order"));
        assert!(source.contains("pub async fn get_pending_work_order_reschedule_request"));
        assert!(source.contains("pub async fn approve_work_order_reschedule_request"));
        assert!(source.contains("pub async fn reject_work_order_reschedule_request"));
    }
}
