use super::AppState;
use crate::http::auth::extract_ip;
use crate::http::domain_resolver::normalize_custom_domain_input;
use crate::models::tenant::{
    apply_manual_custom_domain_status, resolve_custom_domain_lifecycle_transition,
    CUSTOM_DOMAIN_STATUS_PENDING,
};
use crate::models::Tenant;
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

#[derive(Debug, Serialize, sqlx::FromRow)]
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

#[derive(Debug, Serialize, sqlx::FromRow)]
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

#[derive(Debug, Serialize, sqlx::FromRow)]
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

#[derive(Debug, Serialize, sqlx::FromRow)]
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

#[derive(Serialize)]
pub struct SuperadminManagedRadiusServerListResponse {
    pub data: Vec<SuperadminManagedRadiusServer>,
    pub total: i64,
}

#[derive(Serialize)]
pub struct SuperadminManagedRadiusAssignmentListResponse {
    pub data: Vec<SuperadminManagedRadiusAssignment>,
    pub total: i64,
}

#[derive(Serialize)]
pub struct SuperadminManagedRadiusUserListResponse {
    pub data: Vec<SuperadminManagedRadiusUser>,
    pub total: i64,
}

#[derive(Serialize)]
pub struct SuperadminManagedRadiusSessionListResponse {
    pub data: Vec<SuperadminManagedRadiusSession>,
    pub total: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
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

#[derive(Serialize)]
pub struct SuperadminManagedRadiusMappingListResponse {
    pub data: Vec<SuperadminManagedRadiusMapping>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct SuperadminManagedRadiusSecretValue {
    pub shared_secret: String,
    pub shared_secret_masked: String,
}

#[derive(Debug, Serialize)]
pub struct SuperadminManagedRadiusRuntimeStatus {
    pub enabled: bool,
    pub running: bool,
    pub bind_addr: String,
    pub auth_port: i32,
    pub acct_port: i32,
    pub advertised_host: String,
    pub require_message_authenticator: bool,
}

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
    #[serde(alias = "db_host")]
    pub endpoint_host: String,
    #[serde(default, alias = "db_port")]
    pub endpoint_port: Option<i32>,
    #[serde(default, alias = "db_name")]
    pub runtime_label: Option<String>,
    #[serde(default, alias = "db_user")]
    pub runtime_user: Option<String>,
    #[serde(default, alias = "db_password")]
    pub runtime_secret: Option<String>,
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
    #[serde(alias = "radius_server_id")]
    pub radius_endpoint_id: String,
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
    #[serde(alias = "radius_server_id")]
    pub radius_endpoint_id: String,
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

async fn ensure_unique_custom_domain(
    state: &AppState,
    tenant_id: &str,
    custom_domain: Option<&str>,
) -> Result<(), crate::error::AppError> {
    let Some(custom_domain) = custom_domain else {
        return Ok(());
    };

    let owner: Option<String> =
        sqlx::query_scalar("SELECT id FROM tenants WHERE custom_domain = $1")
            .bind(custom_domain)
            .fetch_optional(&state.auth_service.pool)
            .await?;

    if let Some(owner_id) = owner {
        if owner_id != tenant_id {
            return Err(crate::error::AppError::Validation(
                "Custom domain already used by another tenant".to_string(),
            ));
        }
    }

    Ok(())
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

pub async fn get_managed_radius_runtime_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SuperadminManagedRadiusRuntimeStatus>, crate::error::AppError> {
    check_super_admin(&state, &headers).await?;

    let advertised_host = std::env::var("MANAGED_RADIUS_HOST")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("RADIUS_PUBLIC_HOST")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });
    let status = state.radius_service.status(advertised_host.as_deref());

    Ok(Json(SuperadminManagedRadiusRuntimeStatus {
        enabled: status.enabled,
        running: status.running,
        bind_addr: status.bind_addr,
        auth_port: i32::from(status.auth_port),
        acct_port: i32::from(status.acct_port),
        advertised_host: status.advertised_host,
        require_message_authenticator: status.require_message_authenticator,
    }))
}

#[cfg(test)]
mod tests {
    use crate::http::domain_resolver::normalize_custom_domain_input;
    use crate::models::tenant::{
        resolve_custom_domain_lifecycle_transition, CUSTOM_DOMAIN_STATUS_ACTIVE,
        CUSTOM_DOMAIN_STATUS_NONE,
    };

    #[test]
    fn superadmin_custom_domain_input_can_be_cleared_with_blank_value() {
        let normalized = normalize_custom_domain_input(Some("   ")).expect("blank should be valid");

        assert!(normalized.is_none());
    }

    #[test]
    fn superadmin_custom_domain_removal_resets_status_to_none() {
        let next = resolve_custom_domain_lifecycle_transition(
            Some("portal.customer.net"),
            Some(CUSTOM_DOMAIN_STATUS_ACTIVE),
            None,
            None,
            None,
        );

        assert_eq!(next.0, CUSTOM_DOMAIN_STATUS_NONE);
    }
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
    .fetch_all(&mut *tx)
    .await?;

    let total = users.len() as i64;
    tx.commit().await?;

    Ok(Json(SuperadminManagedRadiusUserListResponse {
        data: users,
        total,
    }))
}

pub async fn list_managed_radius_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SuperadminManagedRadiusSessionListResponse>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

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
    .fetch_all(&mut *tx)
    .await?;

    let total = sessions.len() as i64;
    tx.commit().await?;

    Ok(Json(SuperadminManagedRadiusSessionListResponse {
        data: sessions,
        total,
    }))
}

pub async fn list_managed_radius_mappings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SuperadminManagedRadiusMappingListResponse>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

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
            shared_secret_masked:
                crate::services::ManagedRadiusService::mask_shared_secret_for_display(
                    &shared_secret,
                ),
            is_active: row.14,
            updated_at: row.15,
        });
    }

    let total = data.len() as i64;
    Ok(Json(SuperadminManagedRadiusMappingListResponse {
        data,
        total,
    }))
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
        .create_endpoint(
            crate::services::managed_radius_service::ManagedRadiusEndpointUpsert {
                name: payload.name,
                endpoint_host: payload.endpoint_host,
                endpoint_port: payload.endpoint_port,
                runtime_label: payload.runtime_label,
                runtime_user: payload.runtime_user,
                runtime_secret: payload.runtime_secret,
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
            "MANAGED_RADIUS_SERVER_CREATED",
            "managed_radius_server",
            Some(&server.id),
            Some("Native RADIUS endpoint created by Superadmin"),
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
        .update_endpoint(
            &id,
            crate::services::managed_radius_service::ManagedRadiusEndpointUpsert {
                name: payload.name,
                endpoint_host: payload.endpoint_host,
                endpoint_port: payload.endpoint_port,
                runtime_label: payload.runtime_label,
                runtime_user: payload.runtime_user,
                runtime_secret: payload.runtime_secret,
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
            Some("Native RADIUS endpoint updated by Superadmin"),
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
        .set_endpoint_active(&id, payload.is_active)
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
            Some("Native RADIUS endpoint active state changed by Superadmin"),
            Some(&ip),
        )
        .await;

    Ok(Json(json!({"ok": true})))
}

pub async fn set_managed_radius_server_default(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    let server = state
        .managed_radius_service
        .set_endpoint_default(&id)
        .await?;

    state
        .audit_service
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
                radius_endpoint_id: payload.radius_endpoint_id,
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
                radius_endpoint_id: payload.radius_endpoint_id,
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
        .create_mapping(
            crate::services::managed_radius_service::ManagedRadiusNasUpsert {
                tenant_id: payload.tenant_id.clone(),
                radius_endpoint_id: payload.radius_endpoint_id,
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
                radius_endpoint_id: payload.radius_endpoint_id,
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
        shared_secret_masked: crate::services::ManagedRadiusService::mask_shared_secret_for_display(
            &secret,
        ),
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
        shared_secret_masked: crate::services::ManagedRadiusService::mask_shared_secret_for_display(
            &secret,
        ),
        shared_secret: secret,
    }))
}

pub async fn delete_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;

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
    .fetch_all(&state.auth_service.pool)
    .await?;

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
    .fetch_all(&state.auth_service.pool)
    .await?;

    let mut tx = state.auth_service.pool.begin().await?;

    // Delete tenant (cascades to tenant_members, roles, etc.)
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(&id)
        .execute(&mut *tx)
        .await?;

    // Delete orphaned users (no other tenant membership)
    for uid in &orphan_user_ids {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(uid)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    state
        .audit_service
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

    Ok(Json(json!({
        "message": "Tenant deleted successfully",
        "orphaned_users_deleted": orphan_user_ids.len()
    })))
}

pub async fn create_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;

    // 1. Create Tenant object
    let mut tenant = Tenant::new(payload.name, payload.slug);
    tenant.custom_domain = normalize_custom_domain_input(payload.custom_domain.as_deref())
        .map_err(crate::error::AppError::Validation)?;
    if tenant.custom_domain.is_some() {
        tenant.custom_domain_status = Some(CUSTOM_DOMAIN_STATUS_PENDING.to_string());
    }

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

    ensure_unique_custom_domain(&state, &tenant.id, tenant.custom_domain.as_deref()).await?;

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
        "INSERT INTO tenants (id, name, slug, custom_domain, custom_domain_status, custom_domain_verified_at, custom_domain_failure_reason, logo_url, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
    )
    .bind(&tenant.id)
    .bind(&tenant.name)
    .bind(&tenant.slug)
    .bind(&tenant.custom_domain)
    .bind(&tenant.custom_domain_status)
    .bind(&tenant.custom_domain_verified_at)
    .bind(&tenant.custom_domain_failure_reason)
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

    // Look up global 'Owner' role (seeded, tenant_id IS NULL)
    let now = chrono::Utc::now();
    let owner_role_id: String = sqlx::query_scalar(
        "SELECT id FROM roles WHERE name = 'Owner' AND tenant_id IS NULL LIMIT 1"
    )
    .fetch_one(&mut *tx)
    .await?;

    // Create Membership with role_id
    let membership_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO tenant_members (id, tenant_id, user_id, role_id, created_at) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(membership_id)
    .bind(&tenant.id)
    .bind(&user.id)
    .bind(&owner_role_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let plan_id_to_assign = if let Some(pid) = payload.plan_id.clone() {
        Some(pid)
    } else {
        sqlx::query_scalar("SELECT id FROM plans WHERE is_default = true LIMIT 1")
            .fetch_optional(&state.auth_service.pool)
            .await
            .unwrap_or(None)
    };

    if let Some(pid) = plan_id_to_assign {
        if let Err(err) = state
            .plan_service
            .assign_plan_to_tenant(&tenant.id, &pid)
            .await
        {
            tracing::error!(
                "Failed to assign plan {} to tenant {} via HTTP flow: {}",
                pid,
                tenant.id,
                err
            );
        } else {
            match state
                .plan_service
                .check_feature_access(&tenant.id, "managed_radius")
                .await
            {
                Ok(access) if access.has_access => {
                    if let Err(err) = state
                        .managed_radius_service
                        .auto_assign_default_endpoint_for_tenant(&tenant.id)
                        .await
                    {
                        tracing::error!(
                            "Failed to auto-assign default native RADIUS endpoint for tenant {} via HTTP flow: {}",
                            tenant.id,
                            err
                        );
                    }
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::error!(
                        "Failed to evaluate managed_radius feature access for tenant {} via HTTP flow: {}",
                        tenant.id,
                        err
                    );
                }
            }
        }
    }

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
    #[serde(default)]
    pub plan_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct UpdateTenantDomainStatusRequest {
    pub status: String,
    pub failure_reason: Option<String>,
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

    let normalized_custom_domain = normalize_custom_domain_input(payload.custom_domain.as_deref())
        .map_err(crate::error::AppError::Validation)?;

    if before.custom_domain != normalized_custom_domain && normalized_custom_domain.is_some() {
        let access = state
            .plan_service
            .check_feature_access(&id, "custom_domain")
            .await
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
        if !access.has_access {
            return Err(crate::error::AppError::Forbidden(
                "This tenant plan does not support Custom Domains".to_string(),
            ));
        }
    }

    ensure_unique_custom_domain(&state, &id, normalized_custom_domain.as_deref()).await?;

    let (next_domain_status, next_verified_at, next_failure_reason) =
        resolve_custom_domain_lifecycle_transition(
            before.custom_domain.as_deref(),
            before.custom_domain_status.as_deref(),
            before.custom_domain_verified_at,
            before.custom_domain_failure_reason.as_deref(),
            normalized_custom_domain.as_deref(),
        );

    // Update
    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    let tenant: Tenant = sqlx::query_as(
        "UPDATE tenants SET name = $1, slug = $2, custom_domain = $3, custom_domain_status = $4, custom_domain_verified_at = $5, custom_domain_failure_reason = $6, is_active = $7, updated_at = $8 WHERE id = $9 RETURNING *"
    )
    .bind(&payload.name)
    .bind(&payload.slug)
    .bind(&normalized_custom_domain)
    .bind(&next_domain_status)
    .bind(next_verified_at)
    .bind(next_failure_reason)
    .bind(payload.is_active)
    .bind(chrono::Utc::now())
    .bind(&id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    if let Some(ref new_plan_id) = payload.plan_id {
        // If plan is provided in the request, try to assign it to the tenant
        if let Err(err) = state
            .plan_service
            .assign_plan_to_tenant(&id, new_plan_id)
            .await
        {
            tracing::error!(
                "Failed to assign plan {} to tenant {} during update: {}",
                new_plan_id,
                id,
                err
            );
            return Err(crate::error::AppError::Internal(
                "Failed to assign plan to tenant".to_string(),
            ));
        }
    }

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
        "custom_domain_status_before": before.custom_domain_status,
        "custom_domain_status_after": tenant.custom_domain_status,
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

    // Notify superadmins when custom domain changes (superadmin path)
    let domain_changed = before.custom_domain != tenant.custom_domain;
    if domain_changed {
        #[cfg(feature = "postgres")]
        let super_admins: Vec<(String,)> =
            match sqlx::query_as("SELECT id FROM users WHERE is_super_admin = true AND is_active = true")
                .fetch_all(&state.auth_service.pool)
                .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!(error = %e, "failed to list superadmins for custom domain notif");
                    Vec::new()
                }
            };

        #[cfg(feature = "sqlite")]
        let super_admins: Vec<(String,)> =
            match sqlx::query_as("SELECT id FROM users WHERE is_super_admin = 1 AND is_active = 1")
                .fetch_all(&state.auth_service.pool)
                .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!(error = %e, "failed to list superadmins for custom domain notif");
                    Vec::new()
                }
            };

        let title = "Tenant Custom Domain Updated (by Superadmin)";
        let msg = format!(
            "{} updated tenant **{}** ({}) custom domain.\n• Before: {}\n• After: {}\n• Status: {}",
            claims.email,
            tenant.name,
            id,
            before.custom_domain.unwrap_or_else(|| "none".to_string()),
            tenant.custom_domain.clone().unwrap_or_else(|| "none".to_string()),
            tenant.custom_domain_status.clone().unwrap_or_else(|| "unknown".to_string()),
        );

        for (user_id,) in &super_admins {
            if let Err(e) = state
                .notification_service
                .create_notification(
                    user_id.clone(),
                    None,
                    title.to_string(),
                    msg.clone(),
                    "info".to_string(),
                    "system".to_string(),
                    None,
                )
                .await
            {
                tracing::error!(user_id = %user_id, error = %e, "failed to create custom-domain notification for superadmin");
            }
        }
    }

    Ok(Json(tenant))
}

pub async fn update_tenant_domain_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTenantDomainStatusRequest>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    let claims = check_super_admin(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

    let before: Tenant = sqlx::query_as("SELECT * FROM tenants WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.auth_service.pool)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Tenant not found".to_string()))?;

    let (next_status, next_verified_at, next_failure_reason) = apply_manual_custom_domain_status(
        before.custom_domain.as_deref(),
        &payload.status,
        payload.failure_reason.as_deref(),
    )
    .map_err(crate::error::AppError::Validation)?;

    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    let tenant: Tenant = sqlx::query_as(
        "UPDATE tenants SET custom_domain_status = $1, custom_domain_verified_at = $2, custom_domain_failure_reason = $3, updated_at = $4 WHERE id = $5 RETURNING *",
    )
    .bind(&next_status)
    .bind(next_verified_at)
    .bind(next_failure_reason)
    .bind(chrono::Utc::now())
    .bind(&id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    let details = serde_json::json!({
        "message": "Updated tenant custom domain status",
        "tenant_id": id,
        "custom_domain": tenant.custom_domain,
        "custom_domain_status_before": before.custom_domain_status,
        "custom_domain_status_after": tenant.custom_domain_status,
        "custom_domain_failure_reason_before": before.custom_domain_failure_reason,
        "custom_domain_failure_reason_after": tenant.custom_domain_failure_reason,
    })
    .to_string();
    state
        .audit_service
        .log(
            Some(&claims.sub),
            Some(&id),
            "update",
            "tenant_custom_domain_status",
            Some(&id),
            Some(details.as_str()),
            Some(&ip),
        )
        .await;

    Ok(Json(tenant))
}
