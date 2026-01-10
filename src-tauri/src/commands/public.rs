use tauri::State;
use crate::models::Tenant;
use crate::services::AuthService;

#[tauri::command]
pub async fn get_tenant_by_slug(
    slug: String,
    auth_service: State<'_, AuthService>,
) -> Result<Tenant, String> {
    println!("DEBUG: Fetching tenant for slug: '{}'", slug);

    let tenant: Option<Tenant> = sqlx::query_as("SELECT * FROM tenants WHERE slug = $1 AND is_active = true")
        .bind(&slug)
        .fetch_optional(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    match &tenant {
        Some(t) => println!("DEBUG: Found tenant: {}", t.name),
        None => println!("DEBUG: Tenant not found for slug: '{}'", slug),
    }

    let mut tenant = tenant.ok_or_else(|| "Tenant not found".to_string())?;

    // Fetch public settings for this tenant (app_name, app_logo_path)
    let settings: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM settings WHERE tenant_id = $1 AND key IN ('app_name', 'app_logo_path')"
    )
    .bind(&tenant.id)
    .fetch_all(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    for (key, value) in settings {
        match key.as_str() {
            "app_name" => tenant.name = value,
            "app_logo_path" => tenant.logo_url = Some(value),
            _ => {}
        }
    }

    Ok(tenant)
}
