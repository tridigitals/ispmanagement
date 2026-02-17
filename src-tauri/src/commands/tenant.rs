use crate::models::Tenant;
use crate::services::{AuthService, PlanService};
use chrono::Utc;
use tauri::State;

#[tauri::command]
pub async fn get_current_tenant(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<Tenant, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims.tenant_id.ok_or("Not a tenant user")?;

    let tenant: Tenant = sqlx::query_as("SELECT * FROM tenants WHERE id = $1")
        .bind(&tenant_id)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(tenant)
}

#[tauri::command]
pub async fn update_current_tenant(
    token: String,
    name: Option<String>,
    custom_domain: Option<String>,
    enforce_2fa: Option<bool>,
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
) -> Result<Tenant, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims.tenant_id.ok_or("Not a tenant user")?;
    auth_service
        .check_permission(&claims.sub, &tenant_id, "settings", "update")
        .await
        .map_err(|e| e.to_string())?;

    // 1. Get Current Tenant
    let current: Tenant = sqlx::query_as("SELECT * FROM tenants WHERE id = $1")
        .bind(&tenant_id)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    // 2. Check Feature Access for Custom Domain
    if let Some(ref domain) = custom_domain {
        // If changing or setting domain
        if current.custom_domain.as_ref() != Some(domain) {
            let access = plan_service
                .check_feature_access(&tenant_id, "custom_domain")
                .await
                .map_err(|e| e.to_string())?;
            if !access.has_access {
                return Err(
                    "Your plan does not support Custom Domains. Please upgrade.".to_string()
                );
            }
        }
    }

    // 3. Update
    #[cfg(feature = "postgres")]
    let sql = "UPDATE tenants SET name = $1, custom_domain = $2, enforce_2fa = $3, updated_at = $4 WHERE id = $5 RETURNING *";
    #[cfg(feature = "sqlite")]
    let sql = "UPDATE tenants SET name = ?, custom_domain = ?, enforce_2fa = ?, updated_at = ? WHERE id = ? RETURNING *";

    let new_name = name.unwrap_or(current.name);
    let new_domain = custom_domain.or(current.custom_domain);
    let new_enforce = enforce_2fa.unwrap_or(current.enforce_2fa);
    let now = Utc::now();

    let q = sqlx::query_as::<_, Tenant>(sql)
        .bind(new_name)
        .bind(new_domain)
        .bind(new_enforce);

    #[cfg(feature = "postgres")]
    let q = q.bind(now);
    #[cfg(feature = "sqlite")]
    let q = q.bind(now.to_rfc3339());

    let tenant = q
        .bind(&tenant_id)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(tenant)
}
