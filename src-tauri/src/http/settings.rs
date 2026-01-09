use axum::{
    extract::{State, Path},
    Json,
    http::HeaderMap,
};
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use super::AppState;
use crate::models::{Setting, UpsertSettingDto};
use serde_json::json;

// Helper to get token from header
fn get_token(headers: &HeaderMap) -> Result<String, crate::error::AppError> {
    headers.get("Authorization")
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
}

pub async fn get_public_settings(
    State(state): State<AppState>,
) -> Result<Json<PublicSettings>, crate::error::AppError> {
    // Public settings are always global (None tenant_id)
    let app_name = state.settings_service.get_value(None, "app_name").await?;
    let app_description = state.settings_service.get_value(None, "app_description").await?;
    let default_locale = state.settings_service.get_value(None, "default_locale").await?;
    
    Ok(Json(PublicSettings {
        app_name,
        app_description,
        default_locale,
    }))
}

pub async fn get_logo(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Option<String>> {
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
    if claims.role != "admin" {
        return Err(crate::error::AppError::Unauthorized);
    }
    
    let settings = state.settings_service.get_all(claims.tenant_id.as_deref()).await?;
    Ok(Json(settings))
}

pub async fn get_setting(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Option<Setting>>, crate::error::AppError> {
    let token = get_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    if claims.role != "admin" {
        return Err(crate::error::AppError::Unauthorized);
    }

    let setting = state.settings_service.get_by_key(claims.tenant_id.as_deref(), &key).await?;
    Ok(Json(setting))
}

pub async fn get_setting_value(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Option<String>>, crate::error::AppError> {
    let token = get_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    if claims.role != "admin" {
        return Err(crate::error::AppError::Unauthorized);
    }

    let value = state.settings_service.get_value(claims.tenant_id.as_deref(), &key).await?;
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
    Json(payload): Json<UpsertSettingRequest>,
) -> Result<Json<Setting>, crate::error::AppError> {
    let token = get_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    if claims.role != "admin" {
        return Err(crate::error::AppError::Unauthorized);
    }
    
    let dto = UpsertSettingDto {
        key: payload.key,
        value: payload.value,
        description: payload.description,
    };
    
    let setting = state.settings_service.upsert(claims.tenant_id, dto).await?;
    Ok(Json(setting))
}

pub async fn delete_setting(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let token = get_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    if claims.role != "admin" {
        return Err(crate::error::AppError::Unauthorized);
    }

    state.settings_service.delete(claims.tenant_id.as_deref(), &key).await?;
    Ok(Json(json!({"message": "Setting deleted"})))
}

#[derive(serde::Deserialize)]
pub struct UploadLogoRequest {
    content: String, // Base64 content
}

pub async fn upload_logo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UploadLogoRequest>,
) -> Result<Json<String>, crate::error::AppError> {
    let token = get_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    
    if claims.role != "admin" {
        return Err(crate::error::AppError::Unauthorized);
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
        fs::create_dir_all(&uploads_dir).map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    }

    let file_path = uploads_dir.join("logo.png");
    fs::write(&file_path, bytes).map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    let path_str = file_path.to_string_lossy().to_string();
    let dto = UpsertSettingDto { 
        key: "app_logo_path".to_string(), 
        value: path_str.clone(), 
        description: Some("Path to application logo".to_string()) 
    };
    state.settings_service.upsert(claims.tenant_id, dto).await?;

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

    Ok(Json(format!("Test email sent successfully to {}", payload.to_email)))
}
