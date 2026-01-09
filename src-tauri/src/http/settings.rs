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

pub async fn get_logo(
    State(state): State<AppState>,
) -> Json<Option<String>> {
    let logo_path = state.app_data_dir.join("uploads").join("logo.png");
    
    if logo_path.exists() {
        if let Ok(bytes) = fs::read(&logo_path) {
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
    state.auth_service.check_admin(&token).await?;
    let settings = state.settings_service.get_all().await?;
    Ok(Json(settings))
}

pub async fn get_setting(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Option<Setting>>, crate::error::AppError> {
    let token = get_token(&headers)?;
    state.auth_service.check_admin(&token).await?;
    let setting = state.settings_service.get_by_key(&key).await?;
    Ok(Json(setting))
}

pub async fn get_setting_value(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Option<String>>, crate::error::AppError> {
    let token = get_token(&headers)?;
    state.auth_service.check_admin(&token).await?;
    let value = state.settings_service.get_value(&key).await?;
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
    state.auth_service.check_admin(&token).await?;
    
    let dto = UpsertSettingDto {
        key: payload.key,
        value: payload.value,
        description: payload.description,
    };
    
    let setting = state.settings_service.upsert(dto).await?;
    Ok(Json(setting))
}

pub async fn delete_setting(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let token = get_token(&headers)?;
    state.auth_service.check_admin(&token).await?;
    state.settings_service.delete(&key).await?;
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
    state.auth_service.check_admin(&token).await?;

    let bytes = general_purpose::STANDARD
        .decode(&payload.content)
        .map_err(|e| crate::error::AppError::Validation(format!("Invalid base64: {}", e)))?;

    let uploads_dir = state.app_data_dir.join("uploads");
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
    state.settings_service.upsert(dto).await?;

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
