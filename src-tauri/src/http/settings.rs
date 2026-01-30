use super::websocket::WsEvent;
use super::AppState;
use crate::http::auth::extract_ip;
use crate::models::{Setting, UpsertSettingDto};
use axum::{
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    Json,
};
use base64::{engine::general_purpose, Engine as _};
use serde_json::json;
use std::fs;
use std::net::SocketAddr;

// Helper to get token from header
fn get_token(headers: &HeaderMap) -> Result<String, crate::error::AppError> {
    headers
        .get("Authorization")
        .and_then(|h: &axum::http::HeaderValue| h.to_str().ok())
        .and_then(|h: &str| h.strip_prefix("Bearer "))
        .map(|s: &str| s.to_string())
        .ok_or(crate::error::AppError::Unauthorized)
}

#[derive(serde::Serialize)]
pub struct PublicSettings {
    pub app_name: Option<String>,
    pub app_description: Option<String>,
    pub default_locale: Option<String>,
    pub currency_code: Option<String>,
    pub maintenance_mode: bool,
    pub maintenance_message: Option<String>,
    // Payment Settings
    pub payment_midtrans_enabled: bool,
    pub payment_midtrans_client_key: Option<String>,
    pub payment_midtrans_is_production: bool,
    pub payment_manual_enabled: bool,
}

pub async fn get_public_settings(
    State(state): State<AppState>,
) -> Result<Json<PublicSettings>, crate::error::AppError> {
    // Public settings are always global (None tenant_id)
    let app_name = state.settings_service.get_value(None, "app_name").await?;
    let app_description = state
        .settings_service
        .get_value(None, "app_description")
        .await?;
    let default_locale = state
        .settings_service
        .get_value(None, "default_locale")
        .await?;
    let currency_code = state
        .settings_service
        .get_value(None, "currency_code")
        .await?
        .or_else(|| Some("IDR".to_string()));
    let maintenance_mode_str = state
        .settings_service
        .get_value(None, "maintenance_mode")
        .await?;
    let maintenance_message = state
        .settings_service
        .get_value(None, "maintenance_message")
        .await?;

    // Payment
    let midtrans_enabled_str = state
        .settings_service
        .get_value(None, "payment_midtrans_enabled")
        .await?;
    let midtrans_client_key = state
        .settings_service
        .get_value(None, "payment_midtrans_client_key")
        .await?;
    let midtrans_is_prod_str = state
        .settings_service
        .get_value(None, "payment_midtrans_is_production")
        .await?;
    let manual_enabled_str = state
        .settings_service
        .get_value(None, "payment_manual_enabled")
        .await?;

    let maintenance_mode = maintenance_mode_str.as_deref() == Some("true");
    let payment_midtrans_enabled = midtrans_enabled_str.as_deref() == Some("true");
    let payment_midtrans_is_production = midtrans_is_prod_str.as_deref() == Some("true");
    // Default manual to true if not set, or check explicit "true"? Usually better to default false if system managed, but user said "only bank transfer appears", implying manual works.
    // If manual_enabled_str is None, user might see it enabled if I default true.
    // In frontend: `sMap["payment_manual_enabled"] !== "false"; // Default true`.
    let payment_manual_enabled = manual_enabled_str.as_deref() != Some("false");

    Ok(Json(PublicSettings {
        app_name,
        app_description,
        default_locale,
        currency_code,
        maintenance_mode,
        maintenance_message,
        payment_midtrans_enabled,
        payment_midtrans_client_key: midtrans_client_key,
        payment_midtrans_is_production,
        payment_manual_enabled,
    }))
}

pub async fn get_logo(State(state): State<AppState>, headers: HeaderMap) -> Json<Option<String>> {
    let mut tenant_id: Option<String> = None;

    // Try to extract tenant_id from token if available
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if let Ok(claims) = state.auth_service.validate_token(token).await {
                    tenant_id = claims.tenant_id;
                }
            }
        }
    }

    let mut logo_path = state.app_data_dir.join("uploads");
    if let Some(tid) = tenant_id {
        logo_path = logo_path.join(tid);
    }

    logo_path = logo_path.join("logo.png");

    if logo_path.exists() {
        if let Ok(bytes) = fs::read(&logo_path) {
            let base64_str = general_purpose::STANDARD.encode(&bytes);
            return Json(Some(format!("data:image/png;base64,{}", base64_str)));
        }
    }
    // Fallback to global logo if tenant logo missing? Or just return None?
    // Let's fallback to global if specific tenant logo not found
    let global_path = state.app_data_dir.join("uploads").join("logo.png");
    if global_path.exists() {
        if let Ok(bytes) = fs::read(&global_path) {
            let base64_str = general_purpose::STANDARD.encode(&bytes);
            return Json(Some(format!("data:image/png;base64,{}", base64_str)));
        }
    }

    Json(None)
}

pub async fn get_all_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Setting>>, crate::error::AppError> {
    let token = get_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    // Check permission using RBAC
    if let Some(ref tenant_id) = claims.tenant_id {
        state
            .auth_service
            .check_permission(&claims.sub, tenant_id, "settings", "read")
            .await?;
    }

    // For superadmin, read GLOBAL settings (ignoring their personal tenant_id)
    // This aligns with upsert_setting which forces Global for superadmins
    let tenant_id = if claims.is_super_admin {
        None
    } else {
        claims.tenant_id.as_deref()
    };

    let settings = state.settings_service.get_all(tenant_id).await?;
    Ok(Json(settings))
}

pub async fn get_setting(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Option<Setting>>, crate::error::AppError> {
    let token = get_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    // Check permission using RBAC
    if let Some(ref tenant_id) = claims.tenant_id {
        state
            .auth_service
            .check_permission(&claims.sub, tenant_id, "settings", "read")
            .await?;
    }

    // For superadmin, read GLOBAL settings
    let tenant_id = if claims.is_super_admin {
        None
    } else {
        claims.tenant_id.as_deref()
    };

    let setting = state.settings_service.get_by_key(tenant_id, &key).await?;
    Ok(Json(setting))
}

pub async fn get_setting_value(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Option<String>>, crate::error::AppError> {
    let token = get_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    // Check permission using RBAC
    if let Some(ref tenant_id) = claims.tenant_id {
        state
            .auth_service
            .check_permission(&claims.sub, tenant_id, "settings", "read")
            .await?;
    }

    // For superadmin, read GLOBAL settings
    let tenant_id = if claims.is_super_admin {
        None
    } else {
        claims.tenant_id.as_deref()
    };

    let value = state.settings_service.get_value(tenant_id, &key).await?;
    Ok(Json(value))
}

#[derive(serde::Deserialize)]
pub struct UpsertSettingRequest {
    key: String,
    value: String,
    description: Option<String>,
}

pub async fn upsert_setting(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<UpsertSettingRequest>,
) -> Result<Json<Setting>, crate::error::AppError> {
    let token = get_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);

    // Check permission using RBAC
    if let Some(ref tenant_id) = claims.tenant_id {
        state
            .auth_service
            .check_permission(&claims.sub, tenant_id, "settings", "update")
            .await?;
    }

    let dto = UpsertSettingDto {
        key: payload.key,
        value: payload.value,
        description: payload.description,
    };

    // For superadmin, save settings as GLOBAL (tenant_id = None)
    // This is important for settings like maintenance_mode that need to be accessible via public endpoint
    let tenant_id_for_save = if claims.is_super_admin {
        None
    } else {
        claims.tenant_id
    };

    println!(
        "DEBUG: [HTTP] Upserting setting '{}' for Tenant ID: {:?} (is_super_admin: {})",
        dto.key, tenant_id_for_save, claims.is_super_admin
    );

    // Check if this is a maintenance_mode change to broadcast via WebSocket
    let is_maintenance_mode = dto.key == "maintenance_mode";
    let maintenance_enabled = dto.value == "true";

    let setting = state
        .settings_service
        .upsert(tenant_id_for_save, dto, Some(&claims.sub), Some(&ip))
        .await?;

    // Broadcast maintenance mode change to all connected clients
    if is_maintenance_mode {
        // Get maintenance message if exists
        let maintenance_message = state
            .settings_service
            .get_value(None, "maintenance_message")
            .await
            .ok()
            .flatten();

        state.ws_hub.broadcast(WsEvent::MaintenanceModeChanged {
            enabled: maintenance_enabled,
            message: maintenance_message,
        });
        println!(
            "DEBUG: [HTTP] Broadcasted MaintenanceModeChanged event (enabled: {})",
            maintenance_enabled
        );
    }

    Ok(Json(setting))
}

pub async fn delete_setting(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let token = get_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);

    // Check permission using RBAC
    if let Some(ref tenant_id) = claims.tenant_id {
        state
            .auth_service
            .check_permission(&claims.sub, tenant_id, "settings", "delete")
            .await?;
    }

    state
        .settings_service
        .delete(
            claims.tenant_id.as_deref(),
            &key,
            Some(&claims.sub),
            Some(&ip),
        )
        .await?;
    Ok(Json(json!({"message": "Setting deleted"})))
}

#[derive(serde::Deserialize)]
pub struct UploadLogoRequest {
    content: String, // Base64 content
}

pub async fn upload_logo(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<UploadLogoRequest>,
) -> Result<Json<String>, crate::error::AppError> {
    let token = get_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);

    // Check permission using RBAC
    if let Some(ref tenant_id) = claims.tenant_id {
        state
            .auth_service
            .check_permission(&claims.sub, tenant_id, "settings", "update")
            .await?;
    }

    let bytes = general_purpose::STANDARD
        .decode(&payload.content)
        .map_err(|e| crate::error::AppError::Validation(format!("Invalid base64: {}", e)))?;

    let mut uploads_dir = state.app_data_dir.join("uploads");

    // If tenant, create subdirectory
    if let Some(tid) = &claims.tenant_id {
        uploads_dir = uploads_dir.join(tid);
    }

    if !uploads_dir.exists() {
        fs::create_dir_all(&uploads_dir)
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    }

    let file_path = uploads_dir.join("logo.png");
    fs::write(&file_path, bytes).map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    let path_str = file_path.to_string_lossy().to_string();
    let dto = UpsertSettingDto {
        key: "app_logo_path".to_string(),
        value: path_str.clone(),
        description: Some("Path to application logo".to_string()),
    };
    state
        .settings_service
        .upsert(claims.tenant_id, dto, Some(&claims.sub), Some(&ip))
        .await?;

    Ok(Json(path_str))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestEmailRequest {
    to_email: String,
}

pub async fn send_test_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TestEmailRequest>,
) -> Result<Json<String>, crate::error::AppError> {
    let token = get_token(&headers)?;
    state.auth_service.check_admin(&token).await?;

    let email_service = crate::services::EmailService::new((*state.settings_service).clone());
    email_service.send_test_email(&payload.to_email).await?;

    Ok(Json(format!(
        "Test email sent successfully to {}",
        payload.to_email
    )))
}
