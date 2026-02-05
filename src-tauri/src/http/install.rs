//! Install HTTP Handlers

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::models::{CreateUserDto, UpdateUserDto, UpsertSettingDto, UserResponse};
use crate::models::{Tenant, TenantMember};
use uuid::Uuid;

#[derive(Serialize)]
pub struct IsInstalledResponse {
    installed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
    admin_name: String,
    admin_email: String,
    admin_password: String,
    app_name: Option<String>,
    app_url: Option<String>,
}

#[derive(Serialize)]
pub struct InstallResponse {
    success: bool,
    user: UserResponse,
    tenant: Option<Tenant>,
    message: String,
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in input.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Check if application is installed (has any users)
pub async fn check_installed(State(state): State<AppState>) -> Json<IsInstalledResponse> {
    // If DB is freshly created but not migrated yet, treat as "not installed"
    // instead of returning an error which would spam the UI.
    let count = state.user_service.count().await.unwrap_or(0);
    Json(IsInstalledResponse {
        installed: count > 0,
    })
}

/// Install the application (create first admin user and configure settings)
pub async fn install_app(
    State(state): State<AppState>,
    Json(payload): Json<InstallRequest>,
) -> Result<Json<InstallResponse>, (StatusCode, String)> {
    // Check if already installed
    let count = state
        .user_service
        .count()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if count > 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Application is already installed".to_string(),
        ));
    }

    // 1. Save App Settings
    if let Some(app_name) = payload.app_name.clone() {
        let _ = state
            .settings_service
            .upsert(
                None,
                UpsertSettingDto {
                    key: "app_name".to_string(),
                    value: app_name,
                    description: Some("Application name".to_string()),
                },
                None,
                None,
            )
            .await;
    }

    if let Some(app_url) = payload.app_url {
        let _ = state
            .settings_service
            .upsert(
                None,
                UpsertSettingDto {
                    // Used across the app (e.g. payment callbacks)
                    key: "app_public_url".to_string(),
                    value: app_url,
                    description: Some("Public URL of the application".to_string()),
                },
                None,
                None,
            )
            .await;
    }

    // 2. Create admin user
    let dto = CreateUserDto {
        email: payload.admin_email,
        password: payload.admin_password,
        name: payload.admin_name,
    };

    let user_res = state
        .user_service
        .create(dto, None, None)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // 3. Update role to admin and set as super admin
    let update_dto = UpdateUserDto {
        email: None,
        name: None,
        role: Some("admin".to_string()),
        is_super_admin: Some(true),
        is_active: Some(true),
    };

    let admin_user = state
        .user_service
        .update(&user_res.id, update_dto, None, None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 4. Create default tenant and membership (so the app is usable immediately)
    let mut created_tenant: Option<Tenant> = None;
    // Best-effort: if tenant creation fails, keep install successful (superadmin can create tenant later).
    let tenant_name = payload
        .app_name
        .clone()
        .unwrap_or_else(|| "Default Tenant".to_string());
    let mut tenant = Tenant::new(tenant_name.clone(), slugify(&tenant_name));

    let slug_exists: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM tenants WHERE slug = $1")
        .bind(&tenant.slug)
        .fetch_one(&state.auth_service.pool)
        .await
        .unwrap_or(false);
    if slug_exists {
        tenant.slug = format!("{}-{}", tenant.slug, Uuid::new_v4().simple());
    }

    if sqlx::query(
        "INSERT INTO tenants (id, name, slug, custom_domain, logo_url, is_active, enforce_2fa, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&tenant.id)
    .bind(&tenant.name)
    .bind(&tenant.slug)
    .bind(&tenant.custom_domain)
    .bind(&tenant.logo_url)
    .bind(tenant.is_active)
    .bind(tenant.enforce_2fa)
    .bind(tenant.created_at)
    .bind(tenant.updated_at)
    .execute(&state.auth_service.pool)
    .await
    .is_ok()
    {
        let member = TenantMember::new(tenant.id.clone(), admin_user.id.clone(), "owner".to_string(), None);
        let _ = sqlx::query(
            "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&member.id)
        .bind(&member.tenant_id)
        .bind(&member.user_id)
        .bind(&member.role)
        .bind(&member.role_id)
        .bind(member.created_at)
        .execute(&state.auth_service.pool)
        .await;

        // Assign default plan subscription if possible
        if let Ok(Some(plan_id)) = sqlx::query_scalar::<_, String>(
            "SELECT id FROM plans WHERE is_default = true ORDER BY sort_order ASC LIMIT 1",
        )
        .fetch_optional(&state.auth_service.pool)
        .await
        {
            let now = chrono::Utc::now();
            let sub_id = Uuid::new_v4().to_string();
            let _ = sqlx::query(
                "INSERT INTO tenant_subscriptions (id, tenant_id, plan_id, status, created_at, updated_at) VALUES ($1, $2, $3, 'active', $4, $5)",
            )
            .bind(sub_id)
            .bind(&tenant.id)
            .bind(plan_id)
            .bind(now)
            .bind(now)
            .execute(&state.auth_service.pool)
            .await;
        }

        created_tenant = Some(tenant);
    }

    Ok(Json(InstallResponse {
        success: true,
        user: admin_user,
        tenant: created_tenant,
        message: "Installation complete! Settings configured, admin created, and default tenant prepared.".to_string(),
    }))
}
