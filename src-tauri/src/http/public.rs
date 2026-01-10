use axum::{
    extract::{Path, State},
    Json,
};
use crate::models::Tenant;
use base64::Engine as _;
use super::AppState;

pub async fn get_tenant_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    println!("DEBUG: [HTTP] Fetching tenant for slug: '{}'", slug);

    let tenant: Option<Tenant> = sqlx::query_as("SELECT * FROM tenants WHERE slug = $1 AND is_active = true")
        .bind(&slug)
        .fetch_optional(&state.auth_service.pool)
        .await?;

    match &tenant {
        Some(t) => println!("DEBUG: [HTTP] Found tenant: {}", t.name),
        None => {
            println!("DEBUG: [HTTP] Tenant not found for slug: '{}'", slug);
            return Err(crate::error::AppError::NotFound("Tenant not found".to_string()));
        }
    }

    let mut tenant = tenant.unwrap();

    // Fetch public settings for this tenant (app_name, app_logo_path)
    let settings: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM settings WHERE tenant_id = $1 AND key IN ('app_name', 'app_logo_path')"
    )
    .bind(&tenant.id)
    .fetch_all(&state.auth_service.pool)
    .await
    .map_err(crate::error::AppError::Database)?;

    for (key, value) in settings {
        match key.as_str() {
            "app_name" => tenant.name = value,
            "app_logo_path" => {
                if std::path::Path::new(&value).exists() {
                    if let Ok(bytes) = std::fs::read(&value) {
                        let base64_str = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        tenant.logo_url = Some(format!("data:image/png;base64,{}", base64_str));
                    } else {
                        // Keep path if read fails (debugging)
                         tenant.logo_url = Some(value);
                    }
                } else {
                     tenant.logo_url = Some(value);
                }
            },
            _ => {}
        }
    }
    
    Ok(Json(tenant))
}

pub async fn get_tenant_by_domain(
    State(state): State<AppState>,
    Path(domain): Path<String>,
) -> Result<Json<Tenant>, crate::error::AppError> {
    println!("DEBUG: [HTTP] Fetching tenant for domain: '{}'", domain);

    let tenant: Option<Tenant> = sqlx::query_as("SELECT * FROM tenants WHERE custom_domain = $1 AND is_active = true")
        .bind(&domain)
        .fetch_optional(&state.auth_service.pool)
        .await?;

    match &tenant {
        Some(t) => println!("DEBUG: [HTTP] Found tenant by domain: {}", t.name),
        None => {
            println!("DEBUG: [HTTP] Tenant not found for domain: '{}'", domain);
            return Err(crate::error::AppError::NotFound("Tenant not found".to_string()));
        }
    }

    let mut tenant = tenant.unwrap();

    // Fetch public settings for this tenant (app_name, app_logo_path)
    let settings: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM settings WHERE tenant_id = $1 AND key IN ('app_name', 'app_logo_path')"
    )
    .bind(&tenant.id)
    .fetch_all(&state.auth_service.pool)
    .await
    .map_err(crate::error::AppError::Database)?;

    for (key, value) in settings {
        match key.as_str() {
            "app_name" => tenant.name = value,
            "app_logo_path" => {
                if std::path::Path::new(&value).exists() {
                     if let Ok(bytes) = std::fs::read(&value) {
                        let base64_str = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        tenant.logo_url = Some(format!("data:image/png;base64,{}", base64_str));
                    } else {
                         tenant.logo_url = Some(value);
                    }
                } else {
                     tenant.logo_url = Some(value);
                }
            },
            _ => {}
        }
    }
    
    Ok(Json(tenant))
}
