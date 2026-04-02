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
    pub host: String,
    pub auth_port: i32,
    pub acct_port: i32,
    pub db_host: String,
    pub db_port: i32,
    pub db_name: String,
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
    pub radius_server_id: String,
    pub server_name: String,
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
    pub radius_present: bool,
    pub radius_last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    pub radius_last_error: Option<String>,
    pub router_profile_name: Option<String>,
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

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SuperadminManagedRadiusMapping {
    pub id: String,
    pub tenant_id: String,
    pub tenant_name: String,
    pub radius_server_id: String,
    pub server_name: String,
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
          s.db_host AS host,
          $1::integer AS auth_port,
          $2::integer AS acct_port,
          s.db_host,
          s.db_port,
          s.db_name,
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
          s.id, s.name, s.db_host, s.db_port, s.db_name, s.is_active, s.is_default, s.notes, s.updated_at
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
          a.radius_server_id,
          s.name AS server_name,
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
          p.radius_present,
          p.radius_last_sync_at,
          p.radius_last_error,
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

    let rows = sqlx::query_as::<_, (
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
    )>(
        r#"
        SELECT
          n.id,
          n.tenant_id,
          t.name AS tenant_name,
          n.radius_server_id,
          s.name AS server_name,
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
            radius_server_id: row.3,
            server_name: row.4,
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
    db_host: String,
    db_port: Option<i32>,
    db_name: String,
    db_user: String,
    db_password: String,
    is_active: bool,
    notes: Option<String>,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let server = managed_radius_service
        .create_server(crate::services::managed_radius_service::ManagedRadiusServerUpsert {
            name,
            db_host,
            db_port,
            db_name,
            db_user,
            db_password: Some(db_password),
            is_active,
            notes,
        })
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            None,
            "MANAGED_RADIUS_SERVER_CREATED",
            "managed_radius_server",
            Some(&server.id),
            Some("Managed RADIUS server created by Superadmin"),
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
    db_host: String,
    db_port: Option<i32>,
    db_name: String,
    db_user: String,
    db_password: Option<String>,
    is_active: bool,
    notes: Option<String>,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    managed_radius_service
        .update_server(
            &id,
            crate::services::managed_radius_service::ManagedRadiusServerUpsert {
                name,
                db_host,
                db_port,
                db_name,
                db_user,
                db_password,
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
            Some("Managed RADIUS server updated by Superadmin"),
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
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    managed_radius_service
        .set_server_active(&id, is_active)
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
            Some("Managed RADIUS server active state changed by Superadmin"),
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
        .set_server_default(&id)
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            None,
            "MANAGED_RADIUS_SERVER_SET_DEFAULT",
            "managed_radius_server",
            Some(&server.id),
            Some(&format!("Set managed RADIUS server {} as default", server.name)),
            None,
        )
        .await;

    Ok(())
}

#[tauri::command]
pub async fn create_managed_radius_assignment(
    token: String,
    tenant_id: String,
    radius_server_id: String,
    is_active: bool,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let assignment = managed_radius_service
        .create_assignment(
            crate::services::managed_radius_service::TenantRadiusAssignmentUpsert {
                tenant_id: tenant_id.clone(),
                radius_server_id,
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
    radius_server_id: String,
    is_active: bool,
    auth_service: State<'_, AuthService>,
    managed_radius_service: State<'_, ManagedRadiusService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    managed_radius_service
        .update_assignment(
            &id,
            crate::services::managed_radius_service::TenantRadiusAssignmentUpsert {
                tenant_id: tenant_id.clone(),
                radius_server_id,
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
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
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
    radius_server_id: String,
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
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let mapping = managed_radius_service
        .create_mapping(crate::services::managed_radius_service::ManagedRadiusNasUpsert {
            tenant_id: tenant_id.clone(),
            radius_server_id,
            router_id,
            nas_name,
            nas_ip_or_cidr,
            shortname,
            shared_secret,
            is_active,
        })
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
    radius_server_id: String,
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
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    managed_radius_service
        .update_mapping(
            &id,
            crate::services::managed_radius_service::ManagedRadiusNasUpsert {
                tenant_id: tenant_id.clone(),
                radius_server_id,
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
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
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
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
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
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
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

    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(&id)
        .execute(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            None,
            "TENANT_DELETED",
            "tenant",
            Some(&id),
            Some("Tenant deleted by Superadmin"),
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

    // 3. Create 'Owner' Role for this Tenant
    let role_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    #[cfg(feature = "postgres")]
    let sql_r = "INSERT INTO roles (id, tenant_id, name, description, is_system, level, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
    #[cfg(feature = "sqlite")]
    let sql_r = "INSERT INTO roles (id, tenant_id, name, description, is_system, level, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";

    let q_r = sqlx::query(sql_r)
        .bind(&role_id)
        .bind(&tenant.id)
        .bind("Owner") // Role Name MUST be "Owner" for default logic in auth_service
        .bind("Tenant Owner with full access")
        .bind(true)
        .bind(100); // High level

    #[cfg(feature = "postgres")]
    let q_r = q_r.bind(now).bind(now);
    #[cfg(feature = "sqlite")]
    let q_r = q_r.bind(now.to_rfc3339()).bind(now.to_rfc3339());

    q_r.execute(&mut *tx).await.map_err(|e| e.to_string())?;

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
        .bind(&role_id); // Use the Role ID we just created

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
                        .auto_assign_default_server_for_tenant(&tenant.id)
                        .await
                    {
                        tracing::error!(
                            "Failed to auto-assign default Managed RADIUS server for tenant {}: {}",
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
