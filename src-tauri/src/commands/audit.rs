//! Audit Logs Commands

use crate::models::PaginatedResponse;
use crate::services::{AuditService, AuthService};
use tauri::State;

fn has_permission(perms: &[String], resource: &str, action: &str) -> bool {
    let perm = format!("{}:{}", resource, action);
    let wildcard = format!("{}:*", resource);
    perms
        .iter()
        .any(|p| p == "*" || p == &perm || p == &wildcard)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn list_audit_logs(
    token: String,
    page: Option<u32>,
    per_page: Option<u32>,
    user_id: Option<String>,
    tenant_id: Option<String>,
    action: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    search: Option<String>,
    audit_service: State<'_, AuditService>,
    auth_service: State<'_, AuthService>,
) -> Result<PaginatedResponse<crate::models::AuditLogResponse>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let date_from_parsed = date_from.and_then(|d| {
        chrono::DateTime::parse_from_rfc3339(&d)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });
    let date_to_parsed = date_to.and_then(|d| {
        chrono::DateTime::parse_from_rfc3339(&d)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });

    let filter = crate::models::AuditLogFilter {
        page,
        per_page,
        user_id,
        tenant_id,
        action,
        date_from: date_from_parsed,
        date_to: date_to_parsed,
        search,
    };

    let (logs, total) = audit_service
        .list(filter)
        .await
        .map_err(|e| e.to_string())?;

    Ok(PaginatedResponse {
        data: logs,
        total,
        page: page.unwrap_or(1),
        per_page: per_page.unwrap_or(20),
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn list_tenant_audit_logs(
    token: String,
    page: Option<u32>,
    per_page: Option<u32>,
    user_id: Option<String>,
    action: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    search: Option<String>,
    audit_service: State<'_, AuditService>,
    auth_service: State<'_, AuthService>,
) -> Result<PaginatedResponse<crate::models::AuditLogResponse>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "Tenant context missing".to_string())?;

    let perms = auth_service
        .get_user_permissions(&claims.sub, &tenant_id)
        .await
        .map_err(|e| e.to_string())?;
    if !has_permission(&perms, "audit_logs", "read") {
        return Err("Missing permission audit_logs:read".to_string());
    }

    let date_from_parsed = date_from.and_then(|d| {
        chrono::DateTime::parse_from_rfc3339(&d)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });
    let date_to_parsed = date_to.and_then(|d| {
        chrono::DateTime::parse_from_rfc3339(&d)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });

    let filter = crate::models::AuditLogFilter {
        page,
        per_page,
        user_id,
        tenant_id: Some(tenant_id),
        action,
        date_from: date_from_parsed,
        date_to: date_to_parsed,
        search,
    };

    let (logs, total) = audit_service
        .list(filter)
        .await
        .map_err(|e| e.to_string())?;

    Ok(PaginatedResponse {
        data: logs,
        total,
        page: page.unwrap_or(1),
        per_page: per_page.unwrap_or(20),
    })
}
