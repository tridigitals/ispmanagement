//! MikroTik router inventory + monitoring commands (tenant admin).

use crate::models::{
    CreateMikrotikIpPoolRequest, CreateMikrotikPppProfileRequest, CreateMikrotikRouterRequest,
    ManagedRadiusRouterSetup, MikrotikAlert, MikrotikIncident, MikrotikInterfaceCounter,
    MikrotikInterfaceMetric, MikrotikIpPool, MikrotikIpPoolDeleteResult,
    MikrotikIpPoolDependencyStatus, MikrotikLogClearResult, MikrotikLogEntry,
    MikrotikLogRetentionSettings, MikrotikLogSyncResult, MikrotikPppProfile,
    MikrotikPppProfileDeleteResult, MikrotikPppProfileDependencyStatus, MikrotikRouter,
    MikrotikRouterMetric, MikrotikRouterNocRow, MikrotikTestResult, PaginatedResponse,
    SimulateMikrotikIncidentRequest, UpdateMikrotikIncidentRequest,
    UpdateMikrotikPppProfileRequest, UpdateMikrotikRouterRequest,
};
use crate::services::mikrotik_service::{
    MIKROTIK_LOGS_DEFAULT_INCLUDE_TOTAL, MIKROTIK_LOGS_DEFAULT_PAGE, MIKROTIK_LOGS_DEFAULT_PER_PAGE,
};
use crate::services::{
    AuditService, AuthService, ManagedRadiusService, MikrotikService, PlanService,
};
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

    auth.check_permission(&claims.sub, &tenant_id, "router_inventory", "read")
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

    auth.check_permission(&claims.sub, &tenant_id, "network_noc", "read")
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

    auth.check_permission(&claims.sub, &tenant_id, "network_alerts", "read")
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
pub async fn list_mikrotik_incidents(
    token: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
    active_only: Option<bool>,
    limit: Option<u32>,
) -> Result<Vec<MikrotikIncident>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_incidents", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .list_incidents(
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
    month: Option<u32>,
    year: Option<i32>,
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

    auth.check_permission(&claims.sub, &tenant_id, "network_logs", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .list_logs(
            &tenant_id,
            router_id,
            level,
            topic,
            q,
            month,
            year,
            page.unwrap_or(MIKROTIK_LOGS_DEFAULT_PAGE),
            per_page.unwrap_or(MIKROTIK_LOGS_DEFAULT_PER_PAGE),
            include_total.unwrap_or(MIKROTIK_LOGS_DEFAULT_INCLUDE_TOTAL),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mikrotik_log_retention(
    token: String,
    router_id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikLogRetentionSettings, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_logs", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .get_router_log_retention(&tenant_id, &router_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_mikrotik_log_retention(
    token: String,
    router_id: String,
    retention_days: Option<i64>,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikLogRetentionSettings, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_logs", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .update_router_log_retention_days(&tenant_id, &router_id, retention_days)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_mikrotik_logs(
    token: String,
    router_id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikLogClearResult, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_logs", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .clear_logs_for_router(&tenant_id, &router_id)
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

    auth.check_permission(&claims.sub, &tenant_id, "network_logs", "manage")
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

    auth.check_permission(&claims.sub, &tenant_id, "network_alerts", "manage")
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

    auth.check_permission(&claims.sub, &tenant_id, "network_alerts", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .resolve_alert_by_id(&tenant_id, &id, &claims.sub)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ack_mikrotik_incident(
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

    auth.check_permission(&claims.sub, &tenant_id, "network_incidents", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .ack_incident(&tenant_id, &id, &claims.sub)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resolve_mikrotik_incident(
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

    auth.check_permission(&claims.sub, &tenant_id, "network_incidents", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .resolve_incident_by_id(&tenant_id, &id, &claims.sub)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_mikrotik_incident(
    token: String,
    id: String,
    owner_user_id: Option<String>,
    notes: Option<String>,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikIncident, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_incidents", "manage")
        .await
        .map_err(|e| e.to_string())?;

    let req = UpdateMikrotikIncidentRequest {
        owner_user_id,
        notes,
    };

    mikrotik
        .update_incident(&tenant_id, &id, req.owner_user_id, req.notes, &claims.sub)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn simulate_mikrotik_incident(
    token: String,
    router_id: String,
    incident_type: String,
    severity: Option<String>,
    interface_name: Option<String>,
    message: Option<String>,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikIncident, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_incidents", "manage")
        .await
        .map_err(|e| e.to_string())?;

    let req = SimulateMikrotikIncidentRequest {
        router_id,
        incident_type,
        severity,
        interface_name,
        message,
    };

    mikrotik
        .simulate_incident(
            &tenant_id,
            &claims.sub,
            &req.router_id,
            &req.incident_type,
            req.severity,
            req.interface_name,
            req.message,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_mikrotik_incident_auto_escalation(
    token: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<serde_json::Value, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "network_incidents", "manage")
        .await
        .map_err(|e| e.to_string())?;

    let escalated = mikrotik
        .trigger_auto_escalation_now(&tenant_id, &claims.sub)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "ok": true, "escalated": escalated }))
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

    auth.check_permission(&claims.sub, &tenant_id, "router_inventory", "read")
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

    auth.check_permission(&claims.sub, &tenant_id, "router_inventory", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .get_snapshot(&tenant_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mikrotik_router_managed_radius_setup(
    token: String,
    router_id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
    managed_radius: State<'_, ManagedRadiusService>,
    plans: State<'_, PlanService>,
) -> Result<ManagedRadiusRouterSetup, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    let can_manage_routers = auth
        .has_permission(&claims.sub, &tenant_id, "router_inventory", "manage")
        .await
        .map_err(|e| e.to_string())?;
    let can_manage_work_orders = auth
        .has_permission(&claims.sub, &tenant_id, "work_orders", "manage")
        .await
        .map_err(|e| e.to_string())?;
    if !can_manage_routers && !can_manage_work_orders {
        return Err("Permission denied".to_string());
    }

    let router = mikrotik
        .get_router(&tenant_id, &router_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Router not found".to_string())?;

    let plan_allows_managed_radius = plans
        .check_feature_access(&tenant_id, "managed_radius")
        .await
        .map(|access| access.has_access)
        .unwrap_or(false);

    let mut setup = managed_radius
        .get_router_setup(&tenant_id, &router, plan_allows_managed_radius)
        .await
        .map_err(|e| e.to_string())?;

    let can_reveal_secret = auth
        .check_permission(
            &claims.sub,
            &tenant_id,
            "router_inventory",
            "manage_radius_secret",
        )
        .await
        .is_ok();

    if !can_reveal_secret {
        setup.shared_secret = None;
    }

    Ok(setup)
}

#[tauri::command]
pub async fn assign_mikrotik_router_managed_radius_default(
    token: String,
    router_id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
    managed_radius: State<'_, ManagedRadiusService>,
    plans: State<'_, PlanService>,
    audit: State<'_, AuditService>,
) -> Result<ManagedRadiusRouterSetup, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "router_inventory", "manage")
        .await
        .map_err(|e| e.to_string())?;

    let router = mikrotik
        .get_router(&tenant_id, &router_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Router not found".to_string())?;

    let plan_allows_managed_radius = plans
        .check_feature_access(&tenant_id, "managed_radius")
        .await
        .map(|access| access.has_access)
        .unwrap_or(false);

    if !plan_allows_managed_radius {
        return Err("Managed RADIUS is not included in the current plan".to_string());
    }

    if managed_radius
        .get_active_assignment_for_tenant_optional(&tenant_id)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("Managed RADIUS assignment is already active for this tenant".to_string());
    }

    let assigned = managed_radius
        .auto_assign_default_server_for_tenant(&tenant_id)
        .await
        .map_err(|e| e.to_string())?;

    if assigned.is_none() {
        return Err("No default Managed RADIUS server is configured".to_string());
    }

    audit
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "MIKROTIK_ROUTER_MANAGED_RADIUS_ASSIGNED_DEFAULT",
            "mikrotik_router",
            Some(&router.id),
            Some(&format!(
                "Assigned tenant {} to the default Managed RADIUS server from router {}",
                tenant_id, router.name
            )),
            None,
        )
        .await;

    let mut setup = managed_radius
        .get_router_setup(&tenant_id, &router, true)
        .await
        .map_err(|e| e.to_string())?;

    let can_reveal_secret = auth
        .check_permission(
            &claims.sub,
            &tenant_id,
            "router_inventory",
            "manage_radius_secret",
        )
        .await
        .is_ok();

    if !can_reveal_secret {
        setup.shared_secret = None;
    }

    Ok(setup)
}

#[tauri::command]
pub async fn create_mikrotik_router_managed_radius_mapping(
    token: String,
    router_id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
    managed_radius: State<'_, ManagedRadiusService>,
    plans: State<'_, PlanService>,
    audit: State<'_, AuditService>,
) -> Result<ManagedRadiusRouterSetup, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "router_inventory", "manage")
        .await
        .map_err(|e| e.to_string())?;

    let router = mikrotik
        .get_router(&tenant_id, &router_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Router not found".to_string())?;

    let plan_allows_managed_radius = plans
        .check_feature_access(&tenant_id, "managed_radius")
        .await
        .map(|access| access.has_access)
        .unwrap_or(false);

    if !plan_allows_managed_radius {
        return Err("Managed RADIUS is not included in the current plan".to_string());
    }

    managed_radius
        .auto_create_mapping_for_router(&tenant_id, &router)
        .await
        .map_err(|e| e.to_string())?;

    audit
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "MIKROTIK_ROUTER_MANAGED_RADIUS_MAPPING_CREATED",
            "mikrotik_router",
            Some(&router.id),
            Some(&format!(
                "Created Managed RADIUS NAS mapping automatically for router {}",
                router.name
            )),
            None,
        )
        .await;

    let mut setup = managed_radius
        .get_router_setup(&tenant_id, &router, true)
        .await
        .map_err(|e| e.to_string())?;

    let can_reveal_secret = auth
        .check_permission(
            &claims.sub,
            &tenant_id,
            "router_inventory",
            "manage_radius_secret",
        )
        .await
        .is_ok();

    if !can_reveal_secret {
        setup.shared_secret = None;
    }

    Ok(setup)
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

    auth.check_permission(&claims.sub, &tenant_id, "ppp_profiles", "read")
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

    auth.check_permission(&claims.sub, &tenant_id, "ppp_profiles", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .sync_ppp_profiles(&tenant_id, &router_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_mikrotik_ppp_profile(
    token: String,
    router_id: String,
    payload: CreateMikrotikPppProfileRequest,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikPppProfile, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "ppp_profiles", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .create_ppp_profile(&tenant_id, &router_id, payload)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_mikrotik_ppp_profile(
    token: String,
    router_id: String,
    id: String,
    payload: UpdateMikrotikPppProfileRequest,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikPppProfile, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "ppp_profiles", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .update_ppp_profile(&tenant_id, &router_id, &id, payload)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_mikrotik_ppp_profile(
    token: String,
    router_id: String,
    id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikPppProfileDeleteResult, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "ppp_profiles", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .delete_ppp_profile(&tenant_id, &router_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mikrotik_ppp_profile_dependencies(
    token: String,
    router_id: String,
    id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikPppProfileDependencyStatus, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "ppp_profiles", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .get_ppp_profile_dependencies(&tenant_id, &router_id, &id)
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

    auth.check_permission(&claims.sub, &tenant_id, "ip_pools", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .list_ip_pools(&tenant_id, &router_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_mikrotik_ip_pool(
    token: String,
    router_id: String,
    name: String,
    ranges: Option<String>,
    next_pool: Option<String>,
    comment: Option<String>,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikIpPool, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "ip_pools", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .create_ip_pool(
            &tenant_id,
            &router_id,
            CreateMikrotikIpPoolRequest {
                name,
                ranges,
                next_pool,
                comment,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_mikrotik_ip_pool(
    token: String,
    router_id: String,
    id: String,
    name: Option<String>,
    ranges: Option<String>,
    next_pool: Option<String>,
    comment: Option<String>,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikIpPool, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "ip_pools", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .update_ip_pool(
            &tenant_id,
            &router_id,
            &id,
            crate::models::UpdateMikrotikIpPoolRequest {
                name,
                ranges,
                next_pool,
                comment,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_mikrotik_ip_pool(
    token: String,
    router_id: String,
    id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikIpPoolDeleteResult, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "ip_pools", "manage")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .delete_ip_pool(&tenant_id, &router_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mikrotik_ip_pool_dependencies(
    token: String,
    router_id: String,
    id: String,
    auth: State<'_, AuthService>,
    mikrotik: State<'_, MikrotikService>,
) -> Result<MikrotikIpPoolDependencyStatus, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "ip_pools", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .get_ip_pool_dependencies(&tenant_id, &router_id, &id)
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

    auth.check_permission(&claims.sub, &tenant_id, "ip_pools", "manage")
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
    latitude: Option<f64>,
    longitude: Option<f64>,
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

    auth.check_permission(&claims.sub, &tenant_id, "router_inventory", "manage")
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
                latitude,
                longitude,
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
    latitude: Option<f64>,
    longitude: Option<f64>,
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

    auth.check_permission(&claims.sub, &tenant_id, "router_inventory", "manage")
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
                latitude,
                longitude,
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

    auth.check_permission(&claims.sub, &tenant_id, "router_inventory", "manage")
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

    auth.check_permission(&claims.sub, &tenant_id, "router_inventory", "read")
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

    auth.check_permission(&claims.sub, &tenant_id, "router_inventory", "read")
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

    auth.check_permission(&claims.sub, &tenant_id, "router_inventory", "read")
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

    auth.check_permission(&claims.sub, &tenant_id, "router_inventory", "read")
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

    auth.check_permission(&claims.sub, &tenant_id, "router_inventory", "read")
        .await
        .map_err(|e| e.to_string())?;

    mikrotik
        .get_live_interface_counters(&tenant_id, &router_id, names)
        .await
        .map_err(|e| e.to_string())
}
