use crate::models::{
    CreateUserDto, Tenant, TenantMember, UpdateUserDto, UpsertSettingDto, UserResponse,
};
use crate::services::{AuthService, SettingsService, UserService};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn is_installed(user_service: State<'_, UserService>) -> Result<bool, String> {
    // Check if any users exist using the count method
    // If the schema isn't migrated yet (e.g. fresh DB), treat as "not installed" instead of error-looping the UI.
    match user_service.count().await {
        Ok(count) => Ok(count > 0),
        Err(_) => Ok(false),
    }
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

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn install_app(
    admin_name: String,
    admin_email: String,
    admin_password: String,
    app_name: Option<String>,
    app_url: Option<String>,
    user_service: State<'_, UserService>,
    settings_service: State<'_, SettingsService>,
    auth_service: State<'_, AuthService>,
) -> Result<UserResponse, String> {
    // Double check if installed
    let count = user_service.count().await.map_err(|e| e.to_string())?;
    if count > 0 {
        return Err("Application already installed".to_string());
    }

    // Save global app settings (best-effort).
    if let Some(name) = app_name.clone() {
        let _ = settings_service
            .upsert(
                None,
                UpsertSettingDto {
                    key: "app_name".to_string(),
                    value: name,
                    description: Some("Application name".to_string()),
                },
                None,
                Some("127.0.0.1"),
            )
            .await;
    }

    if let Some(url) = app_url.clone() {
        let _ = settings_service
            .upsert(
                None,
                UpsertSettingDto {
                    key: "app_public_url".to_string(),
                    value: url,
                    description: Some("Public URL of the application".to_string()),
                },
                None,
                Some("127.0.0.1"),
            )
            .await;
    }

    let dto = CreateUserDto {
        email: admin_email,
        password: admin_password,
        name: admin_name,
    };

    // Create the user (defaults to role="user")
    let user_res = user_service
        .create(dto, None, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())?;

    // Update role to admin
    let update_dto = UpdateUserDto {
        email: None,
        name: None,
        role: Some("admin".to_string()),
        is_super_admin: Some(true),
        is_active: Some(true),
    };

    user_service
        .update(&user_res.id, update_dto, None, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())?;

    // Create default tenant + membership so the app is usable immediately.
    // Best-effort: install should still succeed even if tenant setup fails.
    let tenant_name = app_name.unwrap_or_else(|| "Default Tenant".to_string());
    let mut tenant = Tenant::new(tenant_name.clone(), slugify(&tenant_name));
    let slug_exists: bool = sqlx::query_scalar("SELECT count(*) > 0 FROM tenants WHERE slug = $1")
        .bind(&tenant.slug)
        .fetch_one(&auth_service.pool)
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
    .execute(&auth_service.pool)
    .await
    .is_ok()
    {
        let member = TenantMember::new(
            tenant.id.clone(),
            user_res.id.clone(),
            "owner".to_string(),
            None,
        );
        let _ = sqlx::query(
            "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&member.id)
        .bind(&member.tenant_id)
        .bind(&member.user_id)
        .bind(&member.role)
        .bind(&member.role_id)
        .bind(member.created_at)
        .execute(&auth_service.pool)
        .await;

        if let Ok(Some(plan_id)) = sqlx::query_scalar::<_, String>(
            "SELECT id FROM plans WHERE is_default = true ORDER BY sort_order ASC LIMIT 1",
        )
        .fetch_optional(&auth_service.pool)
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
            .execute(&auth_service.pool)
            .await;
        }
    }

    Ok(user_res)
}
