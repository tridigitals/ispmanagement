use super::AppState;
use crate::http::auth::extract_ip;
use crate::models::Tenant;
use crate::commands::superadmin::{
    SuperadminManagedRadiusAssignment, SuperadminManagedRadiusAssignmentListResponse,
    SuperadminManagedRadiusMapping, SuperadminManagedRadiusMappingListResponse,
    SuperadminManagedRadiusServer, SuperadminManagedRadiusServerListResponse,
    SuperadminManagedRadiusSecretValue,
    SuperadminManagedRadiusUser, SuperadminManagedRadiusUserListResponse,
};
use axum::{
    extract::ConnectInfo,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;

const DEFAULT_RADIUS_AUTH_PORT: i32 = 1812;
const DEFAULT_RADIUS_ACCT_PORT: i32 = 1813;

#[derive(Serialize)]
pub struct TenantListResponse {
    pub data: Vec<Tenant>,
    pub total: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
    pub custom_domain: Option<String>,
    pub owner_email: String,
    pub owner_password: String,
    pub plan_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ManagedRadiusServerRequest {
    pub name: String,
    pub db_host: String,
    pub db_port: Option<i32>,
    pub db_name: String,
    pub db_user: String,
    pub db_password: Option<String>,
    pub is_active: bool,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ManagedRadiusServerActiveRequest {
    pub is_active: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ManagedRadiusAssignmentRequest {
    pub tenant_id: String,
    pub radius_server_id: String,
    pub is_active: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ManagedRadiusAssignmentActiveRequest {
    pub tenant_id: String,
    pub is_active: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ManagedRadiusMappingRequest {
    pub tenant_id: String,
    pub radius_server_id: String,
    pub router_id: String,
    pub nas_name: String,
    pub nas_ip_or_cidr: String,
    pub shortname: Option<String>,
    pub shared_secret: Option<String>,
    pub is_active: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ManagedRadiusMappingActiveRequest {
    pub tenant_id: String,
    pub is_active: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ManagedRadiusSecretRequest {
    pub tenant_id: String,
    pub shared_secret: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ManagedRadiusRevealSecretRequest {
    pub tenant_id: String,
}

// ...

// Helper to check super admin permission
async fn check_super_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::services::auth_service::Claims, crate::error::AppError> {
    let token = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(token).await?;

    if !claims.is_super_admin {
        return Err(crate::error::AppError::Unauthorized);
    }

    Ok(claims)
}

pub async fn list_tenants(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<TenantListResponse>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    let tenants: Vec<Tenant> = sqlx::query_as("SELECT * FROM tenants ORDER BY created_at DESC")
        .fetch_all(&mut *tx)
        .await?;

    let total = tenants.len() as i64;
    tx.commit().await?;

    Ok(Json(TenantListResponse {
        data: tenants,
        total,
    }))
}

pub async fn list_managed_radius_servers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SuperadminManagedRadiusServerListResponse>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

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
          s.id, s.name, s.db_host, s.db_port, s.db_name, s.is_active, s.notes, s.updated_at
        ORDER BY s.updated_at DESC, s.name ASC
        "#,
    )
    .bind(DEFAULT_RADIUS_AUTH_PORT)
    .bind(DEFAULT_RADIUS_ACCT_PORT)
    .fetch_all(&mut *tx)
    .await?;

    let total = servers.len() as i64;
    tx.commit().await?;

    Ok(Json(SuperadminManagedRadiusServerListResponse {
        data: servers,
        total,
    }))
}

pub async fn list_managed_radius_assignments(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SuperadminManagedRadiusAssignmentListResponse>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

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
    .fetch_all(&mut *tx)
    .await?;

    let total = assignments.len() as i64;
    tx.commit().await?;

    Ok(Json(SuperadminManagedRadiusAssignmentListResponse {
        data: assignments,
        total,
    }))
}

pub async fn list_managed_radius_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SuperadminManagedRadiusUserListResponse>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

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
    .fetch_all(&mut *tx)
    .await?;

    let total = users.len() as i64;
    tx.commit().await?;

    Ok(Json(SuperadminManagedRadiusUserListResponse {
        data: users,
        total,
    }))
}

pub async fn list_managed_radius_mappings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SuperadminManagedRadiusMappingListResponse>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let mut tx = state.auth_service.pool.begin().await?;
    state.auth_service.apply_rls_context_tx(&mut tx, &claims).await?;

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
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    let mut data = Vec::with_capacity(rows.len());
    for row in rows {
        let shared_secret = state
            .managed_radius_service
            .reveal_mapping_secret(&row.1, &row.0)
            .await?;
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
            shared_secret_masked:
                crate::services::ManagedRadiusService::mask_shared_secret_for_display(
                    &shared_secret,
                ),
            is_active: row.14,
            updated_at: row.15,
        });
    }

    let total = data.len() as i64;
    Ok(Json(SuperadminManagedRadiusMappingListResponse { data, total }))
}

pub async fn create_managed_radius_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<ManagedRadiusServerRequest>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    let server = state
        .managed_radius_service
        .create_server(crate::services::managed_radius_service::ManagedRadiusServerUpsert {
            name: payload.name,
            db_host: payload.db_host,
            db_port: payload.db_port,
            db_name: payload.db_name,
            db_user: payload.db_user,
            db_password: payload.db_password,
            is_active: payload.is_active,
            notes: payload.notes,
        })
        .await?;

    state
        .audit_service
        .log(
            Some(&claims.sub),
            None,
            "MANAGED_RADIUS_SERVER_CREATED",
            "managed_radius_server",
            Some(&server.id),
            Some("Managed RADIUS server created by Superadmin"),
            Some(&ip),
        )
        .await;

    Ok(Json(json!({"ok": true, "id": server.id})))
}

pub async fn update_managed_radius_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<ManagedRadiusServerRequest>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    state
        .managed_radius_service
        .update_server(
            &id,
            crate::services::managed_radius_service::ManagedRadiusServerUpsert {
                name: payload.name,
                db_host: payload.db_host,
                db_port: payload.db_port,
                db_name: payload.db_name,
                db_user: payload.db_user,
                db_password: payload.db_password,
                is_active: payload.is_active,
                notes: payload.notes,
            },
        )
        .await?;

    state
        .audit_service
        .log(
            Some(&claims.sub),
            None,
            "MANAGED_RADIUS_SERVER_UPDATED",
            "managed_radius_server",
            Some(&id),
            Some("Managed RADIUS server updated by Superadmin"),
            Some(&ip),
        )
        .await;

    Ok(Json(json!({"ok": true})))
}

pub async fn set_managed_radius_server_active(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<ManagedRadiusServerActiveRequest>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    state
        .managed_radius_service
        .set_server_active(&id, payload.is_active)
        .await?;

    state
        .audit_service
        .log(
            Some(&claims.sub),
            None,
            if payload.is_active {
                "MANAGED_RADIUS_SERVER_ACTIVATED"
            } else {
                "MANAGED_RADIUS_SERVER_DEACTIVATED"
            },
            "managed_radius_server",
            Some(&id),
            Some("Managed RADIUS server active state changed by Superadmin"),
            Some(&ip),
        )
        .await;

    Ok(Json(json!({"ok": true})))
}

pub async fn create_managed_radius_assignment(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<ManagedRadiusAssignmentRequest>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    let assignment = state
        .managed_radius_service
        .create_assignment(
            crate::services::managed_radius_service::TenantRadiusAssignmentUpsert {
                tenant_id: payload.tenant_id.clone(),
                radius_server_id: payload.radius_server_id,
                is_active: payload.is_active,
            },
        )
        .await?;

    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&payload.tenant_id),
            "MANAGED_RADIUS_ASSIGNMENT_CREATED",
            "managed_radius_assignment",
            Some(&assignment.id),
            Some("Managed RADIUS tenant assignment created by Superadmin"),
            Some(&ip),
        )
        .await;

    Ok(Json(json!({"ok": true, "id": assignment.id})))
}

pub async fn update_managed_radius_assignment(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<ManagedRadiusAssignmentRequest>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    state
        .managed_radius_service
        .update_assignment(
            &id,
            crate::services::managed_radius_service::TenantRadiusAssignmentUpsert {
                tenant_id: payload.tenant_id.clone(),
                radius_server_id: payload.radius_server_id,
                is_active: payload.is_active,
            },
        )
        .await?;

    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&payload.tenant_id),
            "MANAGED_RADIUS_ASSIGNMENT_UPDATED",
            "managed_radius_assignment",
            Some(&id),
            Some("Managed RADIUS tenant assignment updated by Superadmin"),
            Some(&ip),
        )
        .await;

    Ok(Json(json!({"ok": true})))
}

pub async fn set_managed_radius_assignment_active(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<ManagedRadiusAssignmentActiveRequest>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    state
        .managed_radius_service
        .set_assignment_active(&payload.tenant_id, &id, payload.is_active)
        .await?;

    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&payload.tenant_id),
            if payload.is_active {
                "MANAGED_RADIUS_ASSIGNMENT_ACTIVATED"
            } else {
                "MANAGED_RADIUS_ASSIGNMENT_DEACTIVATED"
            },
            "managed_radius_assignment",
            Some(&id),
            Some("Managed RADIUS tenant assignment active state changed by Superadmin"),
            Some(&ip),
        )
        .await;

    Ok(Json(json!({"ok": true})))
}

pub async fn create_managed_radius_mapping(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<ManagedRadiusMappingRequest>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    let mapping = state
        .managed_radius_service
        .create_mapping(crate::services::managed_radius_service::ManagedRadiusNasUpsert {
            tenant_id: payload.tenant_id.clone(),
            radius_server_id: payload.radius_server_id,
            router_id: payload.router_id,
            nas_name: payload.nas_name,
            nas_ip_or_cidr: payload.nas_ip_or_cidr,
            shortname: payload.shortname,
            shared_secret: payload.shared_secret,
            is_active: payload.is_active,
        })
        .await?;

    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&payload.tenant_id),
            "MANAGED_RADIUS_MAPPING_CREATED",
            "managed_radius_mapping",
            Some(&mapping.id),
            Some("Managed RADIUS mapping created by Superadmin"),
            Some(&ip),
        )
        .await;

    Ok(Json(json!({"ok": true, "id": mapping.id})))
}

pub async fn update_managed_radius_mapping(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<ManagedRadiusMappingRequest>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    state
        .managed_radius_service
        .update_mapping(
            &id,
            crate::services::managed_radius_service::ManagedRadiusNasUpsert {
                tenant_id: payload.tenant_id.clone(),
                radius_server_id: payload.radius_server_id,
                router_id: payload.router_id,
                nas_name: payload.nas_name,
                nas_ip_or_cidr: payload.nas_ip_or_cidr,
                shortname: payload.shortname,
                shared_secret: payload.shared_secret,
                is_active: payload.is_active,
            },
        )
        .await?;

    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&payload.tenant_id),
            "MANAGED_RADIUS_MAPPING_UPDATED",
            "managed_radius_mapping",
            Some(&id),
            Some("Managed RADIUS mapping updated by Superadmin"),
            Some(&ip),
        )
        .await;

    Ok(Json(json!({"ok": true})))
}

pub async fn set_managed_radius_mapping_active(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<ManagedRadiusMappingActiveRequest>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    state
        .managed_radius_service
        .set_mapping_active(&payload.tenant_id, &id, payload.is_active)
        .await?;

    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&payload.tenant_id),
            if payload.is_active {
                "MANAGED_RADIUS_MAPPING_ACTIVATED"
            } else {
                "MANAGED_RADIUS_MAPPING_DEACTIVATED"
            },
            "managed_radius_mapping",
            Some(&id),
            Some("Managed RADIUS mapping active state changed by Superadmin"),
            Some(&ip),
        )
        .await;

    Ok(Json(json!({"ok": true})))
}

pub async fn rotate_managed_radius_mapping_secret(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<ManagedRadiusSecretRequest>,
) -> Result<Json<SuperadminManagedRadiusSecretValue>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    let secret = state
        .managed_radius_service
        .rotate_mapping_secret(&payload.tenant_id, &id, payload.shared_secret)
        .await?;

    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&payload.tenant_id),
            "MANAGED_RADIUS_MAPPING_SECRET_ROTATED",
            "managed_radius_mapping",
            Some(&id),
            Some("Managed RADIUS mapping secret rotated by Superadmin"),
            Some(&ip),
        )
        .await;

    Ok(Json(SuperadminManagedRadiusSecretValue {
        shared_secret_masked:
            crate::services::ManagedRadiusService::mask_shared_secret_for_display(&secret),
        shared_secret: secret,
    }))
}

pub async fn reveal_managed_radius_mapping_secret(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<ManagedRadiusRevealSecretRequest>,
) -> Result<Json<SuperadminManagedRadiusSecretValue>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    let secret = state
        .managed_radius_service
        .reveal_mapping_secret(&payload.tenant_id, &id)
        .await?;

    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&payload.tenant_id),
            "MANAGED_RADIUS_MAPPING_SECRET_REVEALED",
            "managed_radius_mapping",
            Some(&id),
            Some("Managed RADIUS mapping secret revealed by Superadmin"),
            Some(&ip),
        )
        .await;

    Ok(Json(SuperadminManagedRadiusSecretValue {
        shared_secret_masked:
            crate::services::ManagedRadiusService::mask_shared_secret_for_display(&secret),
        shared_secret: secret,
    }))
}

pub async fn delete_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(json!({"message": "Tenant deleted successfully"})))
}

pub async fn create_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;

    // 1. Create Tenant object
    let mut tenant = Tenant::new(payload.name, payload.slug);
    tenant.custom_domain = payload.custom_domain;

    // Check if slug exists
    let exists: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM tenants WHERE slug = $1")
        .bind(&tenant.slug)
        .fetch_one(&state.auth_service.pool)
        .await?;

    if exists {
        return Err(crate::error::AppError::Validation(
            "Slug already exists".to_string(),
        ));
    }

    // 2. Hash owner password
    let password_hash = crate::services::AuthService::hash_password(&payload.owner_password)?;
    let user = crate::models::User::new(payload.owner_email, password_hash, "Admin".to_string());

    // Check if email exists
    let user_exists: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&state.auth_service.pool)
        .await?;

    if user_exists {
        return Err(crate::error::AppError::Validation(
            "User email already exists".to_string(),
        ));
    }

    // 3. Start Transaction
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    // Insert Tenant
    sqlx::query(
        "INSERT INTO tenants (id, name, slug, custom_domain, logo_url, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(&tenant.id)
    .bind(&tenant.name)
    .bind(&tenant.slug)
    .bind(&tenant.custom_domain)
    .bind(&tenant.logo_url)
    .bind(tenant.is_active)
    .bind(tenant.created_at)
    .bind(tenant.updated_at)
    .execute(&mut *tx)
    .await?;

    // Insert User
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
    )
    .bind(&user.id)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&user.name)
    .bind("admin") // Tenant admin
    .bind(false)
    .bind(true)
    .bind(user.created_at)
    .bind(user.updated_at)
    .execute(&mut *tx)
    .await?;

    // Create Membership
    let membership_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO tenant_members (id, tenant_id, user_id, role, created_at) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(membership_id)
    .bind(&tenant.id)
    .bind(&user.id)
    .bind("owner")
    .bind(chrono::Utc::now())
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(tenant))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct UpdateTenantRequest {
    pub name: String,
    pub slug: String,
    pub custom_domain: Option<String>,
    pub is_active: bool,
}

pub async fn update_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTenantRequest>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    // Check if tenant exists
    let before: Option<Tenant> = sqlx::query_as("SELECT * FROM tenants WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.auth_service.pool)
        .await?;

    let before = match before {
        Some(t) => t,
        None => {
            return Err(crate::error::AppError::NotFound(
                "Tenant not found".to_string(),
            ));
        }
    };

    // Check if slug exists (if changed)
    let slug_owner: Option<String> = sqlx::query_scalar("SELECT id FROM tenants WHERE slug = $1")
        .bind(&payload.slug)
        .fetch_optional(&state.auth_service.pool)
        .await?;

    if let Some(slug_owner_id) = slug_owner {
        if slug_owner_id != id {
            return Err(crate::error::AppError::Validation(
                "Slug already taken".to_string(),
            ));
        }
    }

    // Update
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    let tenant: Tenant = sqlx::query_as(
        "UPDATE tenants SET name = $1, slug = $2, custom_domain = $3, is_active = $4, updated_at = $5 WHERE id = $6 RETURNING *"
    )
    .bind(&payload.name)
    .bind(&payload.slug)
    .bind(&payload.custom_domain)
    .bind(payload.is_active)
    .bind(chrono::Utc::now())
    .bind(&id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    // Audit
    let details = serde_json::json!({
        "message": "Updated tenant",
        "tenant_id": id,
        "name_before": before.name,
        "name_after": tenant.name,
        "slug_before": before.slug,
        "slug_after": tenant.slug,
        "custom_domain_before": before.custom_domain,
        "custom_domain_after": tenant.custom_domain,
        "is_active_before": before.is_active,
        "is_active_after": tenant.is_active,
    })
    .to_string();
    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&id),
            "update",
            "tenant",
            Some(&id),
            Some(details.as_str()),
            Some(&ip),
        )
        .await;

    Ok(Json(tenant))
}
