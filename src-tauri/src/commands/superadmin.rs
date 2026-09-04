use crate::models::Tenant;
use crate::services::{AuditService, AuthService, ManagedRadiusService, PlanService};
use tauri::State;

const DEFAULT_RADIUS_AUTH_PORT: i32 = 1812;
const DEFAULT_RADIUS_ACCT_PORT: i32 = 1813;

#[derive(serde::Serialize)]
pub struct TenantListResponse {
    pub data: Vec<Tenant>,
    pub total: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SuperadminManagedRadiusServer {
    pub id: String,
    pub name: String,
    pub endpoint_host: String,
    pub auth_port: i32,
    pub acct_port: i32,
    pub is_active: bool,
    pub is_default: bool,
    pub notes: Option<String>,
    pub tenant_count: i64,
    pub router_count: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SuperadminManagedRadiusAssignment {
    pub id: String,
    pub tenant_id: String,
    pub tenant_name: String,
    pub radius_endpoint_id: String,
    pub endpoint_name: String,
    pub radius_host: String,
    pub auth_port: i32,
    pub acct_port: i32,
    pub is_active: bool,
    pub router_count: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SuperadminManagedRadiusUser {
    pub id: String,
    pub tenant_id: String,
    pub tenant_name: String,
    pub router_id: String,
    pub router_name: Option<String>,
    pub username: String,
    pub radius_identity: Option<String>,
    pub account_source: String,
    pub is_provisioned: bool,
    pub provisioned_at: Option<chrono::DateTime<chrono::Utc>>,
    pub provisioning_error: Option<String>,
    pub router_profile_name: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SuperadminManagedRadiusSession {
    pub id: String,
    pub tenant_id: String,
    pub tenant_name: String,
    pub router_id: String,
    pub router_name: Option<String>,
    pub username: String,
    pub radius_identity: Option<String>,
    pub acct_session_id: String,
    pub status_type: String,
    pub framed_ip_address: Option<String>,
    pub calling_station_id: Option<String>,
    pub session_time_seconds: Option<i64>,
    pub input_octets: Option<i64>,
    pub output_octets: Option<i64>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_update_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize)]
pub struct SuperadminManagedRadiusServerListResponse {
    pub data: Vec<SuperadminManagedRadiusServer>,
    pub total: i64,
}

#[derive(serde::Serialize)]
pub struct SuperadminManagedRadiusAssignmentListResponse {
    pub data: Vec<SuperadminManagedRadiusAssignment>,
    pub total: i64,
}

#[derive(serde::Serialize)]
pub struct SuperadminManagedRadiusUserListResponse {
    pub data: Vec<SuperadminManagedRadiusUser>,
    pub total: i64,
}

#[derive(serde::Serialize)]
pub struct SuperadminManagedRadiusSessionListResponse {
    pub data: Vec<SuperadminManagedRadiusSession>,
    pub total: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SuperadminManagedRadiusMapping {
    pub id: String,
    pub tenant_id: String,
    pub tenant_name: String,
    pub radius_endpoint_id: String,
    pub endpoint_name: String,
    pub radius_host: String,
    pub auth_port: i32,
    pub acct_port: i32,
    pub router_id: String,
    pub router_name: Option<String>,
    pub nas_name: String,
    pub nas_ip_or_cidr: String,
    pub shortname: Option<String>,
    pub shared_secret_masked: String,
    pub is_active: bool,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize)]
pub struct SuperadminManagedRadiusMappingListResponse {
    pub data: Vec<SuperadminManagedRadiusMapping>,
    pub total: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct SuperadminManagedRadiusSecretValue {
    pub shared_secret: String,
    pub shared_secret_masked: String,
}

#[tauri::command]
pub async fn list_tenants(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<TenantListResponse, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    // Access pool directly (or via a new TenantService if refactored)
    let tenants: Vec<Tenant> = sqlx::query_as("SELECT * FROM tenants ORDER BY created_at DESC")
        .fetch_all(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    let total = tenants.len() as i64;

    Ok(TenantListResponse {
        data: tenants,
        total,
    })
}

#[tauri::command]
pub async fn list_managed_radius_servers(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<SuperadminManagedRadiusServerListResponse, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let servers: Vec<SuperadminManagedRadiusServer> = sqlx::query_as(
        r#"
        SELECT
          s.id,
          s.name,
          s.db_host AS endpoint_host,
          $1::integer AS auth_port,
          $2::integer AS acct_port,
          s.is_active,
          s.is_default,
          s.notes,
          COUNT(DISTINCT a.tenant_id)::bigint AS tenant_count,
          COUNT(DISTINCT n.id)::bigint AS router_count,
          s.updated_at
        FROM radius_servers s
        LEFT JOIN tenant_radius_assignments a
          ON a.radius_server_id = s.id
         AND a.is_active = true
        LEFT JOIN managed_radius_nas n
          ON n.radius_server_id = s.id
         AND n.is_active = true
        GROUP BY
          s.id, s.name, s.db_host, s.is_active, s.is_default, s.notes, s.updated_at
        ORDER BY s.is_default DESC, s.updated_at DESC, s.name ASC
        "#,
    )
    .bind(DEFAULT_RADIUS_AUTH_PORT)
    .bind(DEFAULT_RADIUS_ACCT_PORT)
    .fetch_all(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    let total = servers.len() as i64;

    Ok(SuperadminManagedRadiusServerListResponse {
        data: servers,
        total,
    })
}

#[tauri::command]
pub async fn list_managed_radius_assignments(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<SuperadminManagedRadiusAssignmentListResponse, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let assignments: Vec<SuperadminManagedRadiusAssignment> = sqlx::query_as(
        r#"
        SELECT
          a.id,
          a.tenant_id,
          t.name AS tenant_name,
          a.radius_server_id AS radius_endpoint_id,
          s.name AS endpoint_name,
          s.db_host AS radius_host,
          $1::integer AS auth_port,
          $2::integer AS acct_port,
          a.is_active,
          COUNT(n.id)::bigint AS router_count,
          a.updated_at
        FROM tenant_radius_assignments a
        INNER JOIN tenants t
          ON t.id = a.tenant_id
        INNER JOIN radius_servers s
          ON s.id = a.radius_server_id
        LEFT JOIN managed_radius_nas n
          ON n.tenant_id = a.tenant_id
         AND n.radius_server_id = a.radius_server_id
         AND n.is_active = true
        GROUP BY
          a.id, a.tenant_id, t.name, a.radius_server_id, s.name, s.db_host, a.is_active, a.updated_at
        ORDER BY a.updated_at DESC, t.name ASC
        "#,
    )
    .bind(DEFAULT_RADIUS_AUTH_PORT)
    .bind(DEFAULT_RADIUS_ACCT_PORT)
    .fetch_all(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    let total = assignments.len() as i64;

    Ok(SuperadminManagedRadiusAssignmentListResponse {
        data: assignments,
        total,
    })
}

#[tauri::command]
pub async fn list_managed_radius_users(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<SuperadminManagedRadiusUserListResponse, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let users: Vec<SuperadminManagedRadiusUser> = sqlx::query_as(
        r#"
        SELECT
          p.id,
          p.tenant_id,
          t.name AS tenant_name,
          p.router_id,
          r.name AS router_name,
          p.username,
          p.radius_identity,
          p.account_source,
          p.is_provisioned,
          p.provisioned_at,
          p.provisioning_error,
          p.router_profile_name,
          p.updated_at
        FROM pppoe_accounts p
        INNER JOIN tenants t
          ON t.id = p.tenant_id
        LEFT JOIN mikrotik_routers r
          ON r.id = p.router_id
         AND r.tenant_id = p.tenant_id
        WHERE p.account_source = 'managed_radius'
        ORDER BY p.updated_at DESC, p.username ASC
        "#,
    )
    .fetch_all(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    let total = users.len() as i64;

    Ok(SuperadminManagedRadiusUserListResponse { data: users, total })
}

#[tauri::command]
pub async fn list_managed_radius_sessions(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<SuperadminManagedRadiusSessionListResponse, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let sessions: Vec<SuperadminManagedRadiusSession> = sqlx::query_as(
        r#"
        SELECT
          s.id,
          s.tenant_id,
          t.name AS tenant_name,
          s.router_id,
          r.name AS router_name,
          s.username,
          s.radius_identity,
          s.acct_session_id,
          s.status_type::text AS status_type,
          s.framed_ip_address,
          s.calling_station_id,
          s.session_time_seconds,
          s.input_octets,
          s.output_octets,
          s.started_at,
          s.last_update_at,
          s.ended_at,
          s.updated_at
        FROM radius_accounting_sessions s
        INNER JOIN tenants t
          ON t.id = s.tenant_id
        LEFT JOIN mikrotik_routers r
          ON r.id = s.router_id
        ORDER BY COALESCE(s.last_update_at, s.updated_at) DESC
        LIMIT 200
        "#,
    )
    .fetch_all(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    let total = sessions.len() as i64;

    Ok(SuperadminManagedRadiusSessionListResponse {
        data: sessions,
        total,
    })
}

#[tauri::command]
pub async fn list_managed_radius_mappings(
    token: String,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
) -> Result<SuperadminManagedRadiusMappingListResponse, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            String,
            String,
            i32,
            i32,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
            String,
            bool,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT
          n.id,
          n.tenant_id,
          t.name AS tenant_name,
          n.radius_server_id AS radius_endpoint_id,
          s.name AS endpoint_name,
          s.db_host AS radius_host,
          $1::integer AS auth_port,
          $2::integer AS acct_port,
          n.router_id,
          r.name AS router_name,
          n.nas_name,
          n.nas_ip_or_cidr,
          n.shortname,
          n.shared_secret_enc,
          n.is_active,
          n.updated_at
        FROM managed_radius_nas n
        INNER JOIN tenants t
          ON t.id = n.tenant_id
        INNER JOIN radius_servers s
          ON s.id = n.radius_server_id
        LEFT JOIN mikrotik_routers r
          ON r.id = n.router_id
         AND r.tenant_id = n.tenant_id
        ORDER BY n.updated_at DESC, n.nas_name ASC
        "#,
    )
    .bind(DEFAULT_RADIUS_AUTH_PORT)
    .bind(DEFAULT_RADIUS_ACCT_PORT)
    .fetch_all(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut data = Vec::with_capacity(rows.len());
    for row in rows {
        let shared_secret = managed_radius_service
            .reveal_mapping_secret(&row.1, &row.0)
            .await
            .map_err(|e| e.to_string())?;
        data.push(SuperadminManagedRadiusMapping {
            id: row.0,
            tenant_id: row.1,
            tenant_name: row.2,
            radius_endpoint_id: row.3,
            endpoint_name: row.4,
            radius_host: row.5,
            auth_port: row.6,
            acct_port: row.7,
            router_id: row.8,
            router_name: row.9,
            nas_name: row.10,
            nas_ip_or_cidr: row.11,
            shortname: row.12,
            shared_secret_masked: ManagedRadiusService::mask_shared_secret_for_display(
                &shared_secret,
            ),
            is_active: row.14,
            updated_at: row.15,
        });
    }

    let total = data.len() as i64;
    Ok(SuperadminManagedRadiusMappingListResponse { data, total })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_managed_radius_server(
    token: String,
    name: String,
    endpoint_host: String,
    endpoint_port: Option<i32>,
    runtime_label: Option<String>,
    runtime_user: Option<String>,
    runtime_secret: Option<String>,
    is_active: bool,
    notes: Option<String>,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let server = managed_radius_service
        .create_endpoint(
            crate::services::managed_radius_service::ManagedRadiusEndpointUpsert {
                name,
                endpoint_host,
                endpoint_port,
                runtime_label,
                runtime_user,
                runtime_secret,
                is_active,
                notes,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            None,
            "MANAGED_RADIUS_SERVER_CREATED",
            "managed_radius_server",
            Some(&server.id),
            Some("Native RADIUS endpoint created by Superadmin"),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_managed_radius_server(
    token: String,
    id: String,
    name: String,
    endpoint_host: String,
    endpoint_port: Option<i32>,
    runtime_label: Option<String>,
    runtime_user: Option<String>,
    runtime_secret: Option<String>,
    is_active: bool,
    notes: Option<String>,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    managed_radius_service
        .update_endpoint(
            &id,
            crate::services::managed_radius_service::ManagedRadiusEndpointUpsert {
                name,
                endpoint_host,
                endpoint_port,
                runtime_label,
                runtime_user,
                runtime_secret,
                is_active,
                notes,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            None,
            "MANAGED_RADIUS_SERVER_UPDATED",
            "managed_radius_server",
            Some(&id),
            Some("Native RADIUS endpoint updated by Superadmin"),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
pub async fn set_managed_radius_server_active(
    token: String,
    id: String,
    is_active: bool,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    managed_radius_service
        .set_endpoint_active(&id, is_active)
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            None,
            if is_active {
                "MANAGED_RADIUS_SERVER_ACTIVATED"
            } else {
                "MANAGED_RADIUS_SERVER_DEACTIVATED"
            },
            "managed_radius_server",
            Some(&id),
            Some("Native RADIUS endpoint active state changed by Superadmin"),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
pub async fn set_managed_radius_server_default(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let server = managed_radius_service
        .set_endpoint_default(&id)
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            None,
            "MANAGED_RADIUS_SERVER_SET_DEFAULT",
            "managed_radius_server",
            Some(&server.id),
            Some(&format!(
                "Set native RADIUS endpoint {} as default",
                server.name
            )),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
pub async fn create_managed_radius_assignment(
    token: String,
    tenant_id: String,
    radius_endpoint_id: String,
    is_active: bool,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let assignment = managed_radius_service
        .create_assignment(
            crate::services::managed_radius_service::TenantRadiusAssignmentUpsert {
                tenant_id: tenant_id.clone(),
                radius_endpoint_id,
                is_active,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "MANAGED_RADIUS_ASSIGNMENT_CREATED",
            "managed_radius_assignment",
            Some(&assignment.id),
            Some("Managed RADIUS tenant assignment created by Superadmin"),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
pub async fn update_managed_radius_assignment(
    token: String,
    id: String,
    tenant_id: String,
    radius_endpoint_id: String,
    is_active: bool,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    managed_radius_service
        .update_assignment(
            &id,
            crate::services::managed_radius_service::TenantRadiusAssignmentUpsert {
                tenant_id: tenant_id.clone(),
                radius_endpoint_id,
                is_active,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "MANAGED_RADIUS_ASSIGNMENT_UPDATED",
            "managed_radius_assignment",
            Some(&id),
            Some("Managed RADIUS tenant assignment updated by Superadmin"),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
pub async fn set_managed_radius_assignment_active(
    token: String,
    tenant_id: String,
    id: String,
    is_active: bool,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    managed_radius_service
        .set_assignment_active(&tenant_id, &id, is_active)
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            if is_active {
                "MANAGED_RADIUS_ASSIGNMENT_ACTIVATED"
            } else {
                "MANAGED_RADIUS_ASSIGNMENT_DEACTIVATED"
            },
            "managed_radius_assignment",
            Some(&id),
            Some("Managed RADIUS tenant assignment active state changed by Superadmin"),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_managed_radius_mapping(
    token: String,
    tenant_id: String,
    radius_endpoint_id: String,
    router_id: String,
    nas_name: String,
    nas_ip_or_cidr: String,
    shortname: Option<String>,
    shared_secret: Option<String>,
    is_active: bool,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let mapping = managed_radius_service
        .create_mapping(
            crate::services::managed_radius_service::ManagedRadiusNasUpsert {
                tenant_id: tenant_id.clone(),
                radius_endpoint_id,
                router_id,
                nas_name,
                nas_ip_or_cidr,
                shortname,
                shared_secret,
                is_active,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "MANAGED_RADIUS_MAPPING_CREATED",
            "managed_radius_mapping",
            Some(&mapping.id),
            Some("Managed RADIUS mapping created by Superadmin"),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_managed_radius_mapping(
    token: String,
    id: String,
    tenant_id: String,
    radius_endpoint_id: String,
    router_id: String,
    nas_name: String,
    nas_ip_or_cidr: String,
    shortname: Option<String>,
    shared_secret: Option<String>,
    is_active: bool,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    managed_radius_service
        .update_mapping(
            &id,
            crate::services::managed_radius_service::ManagedRadiusNasUpsert {
                tenant_id: tenant_id.clone(),
                radius_endpoint_id,
                router_id,
                nas_name,
                nas_ip_or_cidr,
                shortname,
                shared_secret,
                is_active,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "MANAGED_RADIUS_MAPPING_UPDATED",
            "managed_radius_mapping",
            Some(&id),
            Some("Managed RADIUS mapping updated by Superadmin"),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
pub async fn set_managed_radius_mapping_active(
    token: String,
    tenant_id: String,
    id: String,
    is_active: bool,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    managed_radius_service
        .set_mapping_active(&tenant_id, &id, is_active)
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            if is_active {
                "MANAGED_RADIUS_MAPPING_ACTIVATED"
            } else {
                "MANAGED_RADIUS_MAPPING_DEACTIVATED"
            },
            "managed_radius_mapping",
            Some(&id),
            Some("Managed RADIUS mapping active state changed by Superadmin"),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
pub async fn rotate_managed_radius_mapping_secret(
    token: String,
    tenant_id: String,
    id: String,
    shared_secret: Option<String>,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<SuperadminManagedRadiusSecretValue, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let next_secret = managed_radius_service
        .rotate_mapping_secret(&tenant_id, &id, shared_secret)
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "MANAGED_RADIUS_MAPPING_SECRET_ROTATED",
            "managed_radius_mapping",
            Some(&id),
            Some("Managed RADIUS mapping secret rotated by Superadmin"),
            None,
        )
        .await;

    Ok(SuperadminManagedRadiusSecretValue {
        shared_secret_masked: ManagedRadiusService::mask_shared_secret_for_display(&next_secret),
        shared_secret: next_secret,
    })
}

#[tauri::command]
pub async fn reveal_managed_radius_mapping_secret(
    token: String,
    tenant_id: String,
    id: String,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<SuperadminManagedRadiusSecretValue, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let secret = managed_radius_service
        .reveal_mapping_secret(&tenant_id, &id)
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "MANAGED_RADIUS_MAPPING_SECRET_REVEALED",
            "managed_radius_mapping",
            Some(&id),
            Some("Managed RADIUS mapping secret revealed by Superadmin"),
            None,
        )
        .await;

    Ok(SuperadminManagedRadiusSecretValue {
        shared_secret_masked: ManagedRadiusService::mask_shared_secret_for_display(&secret),
        shared_secret: secret,
    })
}

#[tauri::command]
pub async fn delete_tenant(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    // Find users who ONLY belong to this tenant (will be orphaned after delete)
    #[cfg(feature = "postgres")]
    let orphan_user_ids: Vec<String> = sqlx::query_scalar(
        r#"SELECT tm.user_id FROM tenant_members tm
           WHERE tm.tenant_id = $1
           AND NOT EXISTS (
             SELECT 1 FROM tenant_members tm2
             WHERE tm2.user_id = tm.user_id AND tm2.tenant_id != $1
           )"#,
    )
    .bind(&id)
    .fetch_all(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    #[cfg(not(feature = "postgres"))]
    let orphan_user_ids: Vec<String> = sqlx::query_scalar(
        r#"SELECT tm.user_id FROM tenant_members tm
           WHERE tm.tenant_id = ?
           AND NOT EXISTS (
             SELECT 1 FROM tenant_members tm2
             WHERE tm2.user_id = tm.user_id AND tm2.tenant_id != ?
           )"#,
    )
    .bind(&id)
    .bind(&id)
    .fetch_all(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut tx = auth_service.pool.begin().await.map_err(|e| e.to_string())?;

    // Delete tenant (cascades to tenant_members, roles, etc.)
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    // Delete orphaned users (no other tenant membership)
    for uid in &orphan_user_ids {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(uid)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            None,
            "TENANT_DELETED",
            "tenant",
            Some(&id),
            Some(&format!(
                "Tenant deleted with {} orphaned user(s)",
                orphan_user_ids.len()
            )),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_tenant(
    token: String,
    name: String,
    slug: String,
    custom_domain: Option<String>,
    owner_email: String,
    owner_password: String,
    plan_id: Option<String>,
    auth_service: State<'_, AuthService>,
    audit_service: State<'_, AuditService>,
    plan_service: State<'_, PlanService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
) -> Result<Tenant, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let mut tenant = Tenant::new(name, slug);
    tenant.custom_domain = custom_domain;

    // Check if slug exists
    let exists: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM tenants WHERE slug = $1")
        .bind(&tenant.slug)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    if exists {
        return Err("Slug already exists".to_string());
    }

    // Hash owner password
    let password_hash =
        crate::services::AuthService::hash_password(&owner_password).map_err(|e| e.to_string())?;
    let user = crate::models::User::new(owner_email.clone(), password_hash, "Admin".to_string());

    // Check if email exists
    let user_exists: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    if user_exists {
        return Err("User email already exists".to_string());
    }

    // Start Transaction
    let mut tx = auth_service.pool.begin().await.map_err(|e| e.to_string())?;

    // 1. Create Tenant
    #[cfg(feature = "postgres")]
    let sql_t = "INSERT INTO tenants (id, name, slug, custom_domain, logo_url, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
    #[cfg(feature = "sqlite")]
    let sql_t = "INSERT INTO tenants (id, name, slug, custom_domain, logo_url, is_active, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";

    let q_t = sqlx::query(sql_t)
        .bind(&tenant.id)
        .bind(&tenant.name)
        .bind(&tenant.slug)
        .bind(&tenant.custom_domain)
        .bind(&tenant.logo_url);

    #[cfg(feature = "postgres")]
    let q_t = q_t
        .bind(tenant.is_active)
        .bind(tenant.created_at)
        .bind(tenant.updated_at);
    #[cfg(feature = "sqlite")]
    let q_t = q_t
        .bind(if tenant.is_active { 1 } else { 0 })
        .bind(tenant.created_at.to_rfc3339())
        .bind(tenant.updated_at.to_rfc3339());

    q_t.execute(&mut *tx).await.map_err(|e| e.to_string())?;

    // 2. Create User
    #[cfg(feature = "postgres")]
    let sql_u = "INSERT INTO users (id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)";
    #[cfg(feature = "sqlite")]
    let sql_u = "INSERT INTO users (id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";

    let q_u = sqlx::query(sql_u)
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.name)
        .bind("admin") // Global role (legacy)
        .bind(false)
        .bind(true);

    #[cfg(feature = "postgres")]
    let q_u = q_u.bind(user.created_at).bind(user.updated_at);
    #[cfg(feature = "sqlite")]
    let q_u = q_u
        .bind(user.created_at.to_rfc3339())
        .bind(user.updated_at.to_rfc3339());

    q_u.execute(&mut *tx).await.map_err(|e| e.to_string())?;

    // 3. Look up global 'Owner' role (seeded, tenant_id IS NULL)
    let now = chrono::Utc::now();

    #[cfg(feature = "postgres")]
    let role_id: String = sqlx::query_scalar(
        "SELECT id FROM roles WHERE name = 'Owner' AND tenant_id IS NULL LIMIT 1",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    #[cfg(feature = "sqlite")]
    let role_id: String = sqlx::query_scalar(
        "SELECT id FROM roles WHERE name = 'Owner' AND tenant_id IS NULL LIMIT 1",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    // 4. Assign 'Owner' Role to User (Tenant Membership)
    let membership_id = uuid::Uuid::new_v4().to_string();
    #[cfg(feature = "postgres")]
    let sql_m = "INSERT INTO tenant_members (id, tenant_id, user_id, role_id, created_at) VALUES ($1, $2, $3, $4, $5)";
    #[cfg(feature = "sqlite")]
    let sql_m = "INSERT INTO tenant_members (id, tenant_id, user_id, role_id, created_at) VALUES (?, ?, ?, ?, ?)";

    let q_m = sqlx::query(sql_m)
        .bind(membership_id)
        .bind(&tenant.id)
        .bind(&user.id)
        .bind(&role_id);

    #[cfg(feature = "postgres")]
    let q_m = q_m.bind(now);
    #[cfg(feature = "sqlite")]
    let q_m = q_m.bind(now.to_rfc3339());

    q_m.execute(&mut *tx).await.map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    // 5. Assign Plan
    let plan_id_to_assign = if let Some(pid) = plan_id {
        Some(pid)
    } else {
        // Try to find default plan
        let default_plan_id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM plans WHERE is_default = 1 OR is_default = true LIMIT 1",
        )
        .fetch_optional(&auth_service.pool)
        .await
        .unwrap_or(None);

        default_plan_id
    };

    if let Some(pid) = plan_id_to_assign {
        if let Err(e) = plan_service.assign_plan_to_tenant(&tenant.id, &pid).await {
            // Log error but don't fail the request since tenant is created
            tracing::error!(
                "Failed to assign plan {} to tenant {}: {}",
                pid,
                tenant.id,
                e
            );
        } else {
            match plan_service
                .check_feature_access(&tenant.id, "managed_radius")
                .await
            {
                Ok(access) if access.has_access => {
                    if let Err(e) = managed_radius_service
                        .auto_assign_default_endpoint_for_tenant(&tenant.id)
                        .await
                    {
                        tracing::error!(
                            "Failed to auto-assign default native RADIUS endpoint for tenant {}: {}",
                            tenant.id,
                            e
                        );
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!(
                        "Failed to evaluate managed_radius feature access for tenant {}: {}",
                        tenant.id,
                        e
                    );
                }
            }
        }
    }

    audit_service
        .log(
            Some(&claims.sub),
            None,
            "TENANT_CREATED",
            "tenant",
            Some(&tenant.id),
            Some(&format!(
                "Created tenant {} with owner {}",
                tenant.name, owner_email
            )),
            None,
        )
        .await;

    Ok(tenant)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_tenant(
    token: String,
    id: String,
    name: String,
    slug: String,
    custom_domain: Option<String>,
    is_active: bool,
    auth_service: State<'_, AuthService>,
    audit_service: State<'_, AuditService>,
) -> Result<Tenant, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    // Check if tenant exists
    let existing: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM tenants WHERE id = $1")
        .bind(&id)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    if !existing {
        return Err("Tenant not found".to_string());
    }

    // Check if slug exists (if changed)
    let slug_owner: Option<String> = sqlx::query_scalar("SELECT id FROM tenants WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(slug_owner_id) = slug_owner {
        if slug_owner_id != id {
            return Err("Slug already taken".to_string());
        }
    }

    #[cfg(feature = "postgres")]
    let sql = "UPDATE tenants SET name = $1, slug = $2, custom_domain = $3, is_active = $4, updated_at = $5 WHERE id = $6 RETURNING *";
    #[cfg(feature = "sqlite")]
    let sql = "UPDATE tenants SET name = ?, slug = ?, custom_domain = ?, is_active = ?, updated_at = ? WHERE id = ? RETURNING *";

    let q = sqlx::query_as::<_, Tenant>(sql)
        .bind(&name)
        .bind(&slug)
        .bind(&custom_domain);

    #[cfg(feature = "postgres")]
    let q = q.bind(is_active).bind(chrono::Utc::now());
    #[cfg(feature = "sqlite")]
    let q = q
        .bind(if is_active { 1 } else { 0 })
        .bind(chrono::Utc::now().to_rfc3339());

    let tenant = q
        .bind(&id)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            None,
            "TENANT_UPDATED",
            "tenant",
            Some(&id),
            Some(&format!("Updated tenant {}, active: {}", name, is_active)),
            None,
        )
        .await;

    Ok(tenant)
}

#[cfg(test)]
fn superadmin_commands_source() -> &'static str {
    include_str!("superadmin.rs")
}

#[cfg(test)]
mod tests {
    use super::superadmin_commands_source;

    #[test]
    fn managed_radius_commands_use_endpoint_param_names() {
        let source = superadmin_commands_source();
        let legacy_assignment_param = [
            "create_managed_radius_assignment(\n",
            "    token: String,\n",
            "    tenant_id: String,\n",
            "    radius_server_id: String,",
        ]
        .concat();
        let legacy_update_assignment_param = [
            "update_managed_radius_assignment(\n",
            "    token: String,\n",
            "    id: String,\n",
            "    tenant_id: String,\n",
            "    radius_server_id: String,",
        ]
        .concat();
        let legacy_mapping_param = [
            "create_managed_radius_mapping(\n",
            "    token: String,\n",
            "    tenant_id: String,\n",
            "    radius_server_id: String,",
        ]
        .concat();
        let legacy_update_mapping_param = [
            "update_managed_radius_mapping(\n",
            "    token: String,\n",
            "    id: String,\n",
            "    tenant_id: String,\n",
            "    radius_server_id: String,",
        ]
        .concat();

        assert!(!source.contains(&legacy_assignment_param));
        assert!(!source.contains(&legacy_update_assignment_param));
        assert!(!source.contains(&legacy_mapping_param));
        assert!(!source.contains(&legacy_update_mapping_param));
    }
}
