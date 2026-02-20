//! MikroTik router inventory + monitoring commands (tenant admin).

use crate::models::{
    CreateMikrotikRouterRequest, MikrotikAlert, MikrotikInterfaceCounter, MikrotikInterfaceMetric,
    MikrotikIpPool, MikrotikLogEntry, MikrotikLogSyncResult, MikrotikPppProfile, MikrotikRouter,
    MikrotikRouterMetric, MikrotikRouterNocRow, MikrotikTestResult, PaginatedResponse,
    UpdateMikrotikRouterRequest,
};
use crate::services::{AuditService, AuthService, MikrotikService};
use tauri::State;

#[tauri::command]
pub async fn list_mikrotik_routers(
    token: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<Vec<MikrotikRouter>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .list_routers(&tenant_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mikrotik_noc(
    token: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<Vec<MikrotikRouterNocRow>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .list_noc(&tenant_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mikrotik_alerts(
    token: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
    active_only: Option<bool>,
    limit: Option<u32>,
) -> Result<Vec<MikrotikAlert>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .list_alerts(
            &tenant_id,
            active_only.unwrap_or(true),
            limit.unwrap_or(200),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mikrotik_logs(
    token: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
    router_id: Option<String>,
    level: Option<String>,
    topic: Option<String>,
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    include_total: Option<bool>,
) -> Result<PaginatedResponse<MikrotikLogEntry>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .list_logs(
            &tenant_id,
            router_id,
            level,
            topic,
            q,
            page.unwrap_or(1),
            per_page.unwrap_or(25),
            include_total.unwrap_or(false),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_mikrotik_logs(
    token: String,
    router_id: String,
    fetch_limit: Option<u32>,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikLogSyncResult, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .sync_logs_for_router(
            &tenant_id,
            &router_id,
            fetch_limit.unwrap_or(500).clamp(50, 50_000),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ack_mikrotik_alert(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .ack_alert(&tenant_id, &id, &claims.sub)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resolve_mikrotik_alert(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .resolve_alert_by_id(&tenant_id, &id, &claims.sub)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mikrotik_router(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikRouter, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .get_router(&tenant_id, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Router not found".to_string())
}

#[tauri::command]
pub async fn get_mikrotik_router_snapshot(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<crate::models::MikrotikRouterSnapshot, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .get_snapshot(&tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mikrotik_ppp_profiles(
    token: String,
    router_id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<Vec<MikrotikPppProfile>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .list_ppp_profiles(&tenant_id, &router_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_mikrotik_ppp_profiles(
    token: String,
    router_id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<Vec<MikrotikPppProfile>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .sync_ppp_profiles(&tenant_id, &router_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mikrotik_ip_pools(
    token: String,
    router_id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<Vec<MikrotikIpPool>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .list_ip_pools(&tenant_id, &router_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_mikrotik_ip_pools(
    token: String,
    router_id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<Vec<MikrotikIpPool>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .sync_ip_pools(&tenant_id, &router_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_mikrotik_router(
    token: String,
    name: String,
    host: String,
    port: Option<i32>,
    username: String,
    password: String,
    use_tls: Option<bool>,
    enabled: Option<bool>,
    maintenance_until: Option<String>,
    maintenance_reason: Option<String>,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
    audit: State<'_, AuditService>,
) -> Result<MikrotikRouter, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await
        .map_err(|e| e.to_string())?;

    let router = mikrotik
        .create_router(
            &tenant_id,
            CreateMikrotikRouterRequest {
                name,
                host,
                port,
                username,
                password,
                use_tls,
                enabled,
                maintenance_until: maintenance_until
                    .as_deref()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                maintenance_reason,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    audit
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "create",
            "mikrotik_router",
            Some(&router.id),
            Some(&format!(
                "Created router '{}' ({})",
                router.name, router.host
            )),
            None,
        )
        .await;

    Ok(router)
}

#[tauri::command]
pub async fn update_mikrotik_router(
    token: String,
    id: String,
    name: Option<String>,
    host: Option<String>,
    port: Option<i32>,
    username: Option<String>,
    password: Option<String>,
    use_tls: Option<bool>,
    enabled: Option<bool>,
    maintenance_until: Option<String>,
    maintenance_reason: Option<String>,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
    audit: State<'_, AuditService>,
) -> Result<MikrotikRouter, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await
        .map_err(|e| e.to_string())?;

    let router = mikrotik
        .update_router(
            &tenant_id,
            &id,
            UpdateMikrotikRouterRequest {
                name,
                host,
                port,
                username,
                password,
                use_tls,
                enabled,
                maintenance_until: maintenance_until
                    .as_deref()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                maintenance_reason,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    audit
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "update",
            "mikrotik_router",
            Some(&router.id),
            Some(&format!(
                "Updated router '{}' ({})",
                router.name, router.host
            )),
            None,
        )
        .await;

    Ok(router)
}

#[tauri::command]
pub async fn delete_mikrotik_router(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
    audit: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "manage")
        .await
        .map_err(|e| e.to_string())?;

    let existing = mikrotik
        .get_router(&tenant_id, &id)
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .delete_router(&tenant_id, &id)
        .await
        .map_err(|e| e.to_string())?;

    let details = existing
        .as_ref()
        .map(|r| format!("Deleted router '{}' ({})", r.name, r.host));
    audit
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "delete",
            "mikrotik_router",
            Some(&id),
            details.as_deref(),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
pub async fn test_mikrotik_router(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
    audit: State<'_, AuditService>,
) -> Result<MikrotikTestResult, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    let res = mikrotik
        .test_connection(&tenant_id, &id)
        .await
        .map_err(|e| e.to_string())?;

    let details = if res.ok {
        format!(
            "Tested router connection: ok identity={:?} version={:?} latency_ms={:?}",
            res.identity, res.ros_version, res.latency_ms
        )
    } else {
        format!("Tested router connection: failed error={:?}", res.error)
    };
    audit
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "test_connection",
            "mikrotik_router",
            Some(&id),
            Some(&details),
            None,
        )
        .await;

    Ok(res)
}

#[tauri::command]
pub async fn list_mikrotik_router_metrics(
    token: String,
    router_id: Option<String>,
    #[allow(non_snake_case)] routerId: Option<String>,
    limit: Option<u32>,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<Vec<MikrotikRouterMetric>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    let rid = router_id
        .or(routerId)
        .ok_or_else(|| "Missing routerId".to_string())?;

    mikrotik
        .list_metrics(&tenant_id, &rid, limit.unwrap_or(120))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mikrotik_interface_metrics(
    token: String,
    router_id: Option<String>,
    #[allow(non_snake_case)] routerId: Option<String>,
    interface: Option<String>,
    limit: Option<u32>,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<Vec<MikrotikInterfaceMetric>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    let rid = router_id
        .or(routerId)
        .ok_or_else(|| "Missing routerId".to_string())?;

    mikrotik
        .list_interface_metrics(&tenant_id, &rid, interface.as_deref(), limit.unwrap_or(120))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mikrotik_interface_latest(
    token: String,
    router_id: Option<String>,
    #[allow(non_snake_case)] routerId: Option<String>,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<Vec<MikrotikInterfaceMetric>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    let rid = router_id
        .or(routerId)
        .ok_or_else(|| "Missing routerId".to_string())?;

    mikrotik
        .list_latest_interface_metrics(&tenant_id, &rid)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mikrotik_live_interface_counters(
    token: String,
    router_id: String,
    names: Vec<String>,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<Vec<MikrotikInterfaceCounter>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_routers", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .get_live_interface_counters(&tenant_id, &router_id, names)
        .await
        .map_err(|e| e.to_string())
}
