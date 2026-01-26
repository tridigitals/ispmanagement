use tauri::State;
use crate::models::Tenant;
use crate::services::AuthService;

pub async fn get_tenant_by_slug(
    slug: String,
    auth_service: State<'_, AuthService>,
) -> Result<Tenant, String> {
    let tenant = sqlx::query_as("SELECT * FROM tenants WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

    match tenant {
        Some(t) => Ok(t),
        None => Err("Tenant not found".into()),
    }
}
