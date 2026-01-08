//! Settings Commands

use crate::models::{Setting, UpsertSettingDto};
use crate::services::{AuthService, SettingsService};
use crate::services::auth_service::AuthSettings;
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use tauri::{AppHandle, Manager, State};

/// Get all settings
#[tauri::command]
pub async fn get_all_settings(
    token: String,
    settings_service: State<'_, SettingsService>,
    auth_service: State<'_, AuthService>,
) -> Result<Vec<Setting>, String> {
    auth_service.check_admin(&token).await.map_err(|e| e.to_string())?;
    settings_service.get_all().await.map_err(|e| e.to_string())
}

/// Get public auth settings (no token required)
#[tauri::command]
pub async fn get_auth_settings(
    auth_service: State<'_, AuthService>,
) -> Result<AuthSettings, String> {
    Ok(auth_service.get_auth_settings().await)
}

/// Get setting by key
#[tauri::command]
pub async fn get_setting(
    token: String,
    key: String,
    settings_service: State<'_, SettingsService>,
    auth_service: State<'_, AuthService>,
) -> Result<Option<Setting>, String> {
    auth_service.check_admin(&token).await.map_err(|e| e.to_string())?;
    settings_service.get_by_key(&key).await.map_err(|e| e.to_string())
}

/// Get setting value by key
#[tauri::command]
pub async fn get_setting_value(
    token: String,
    key: String,
    settings_service: State<'_, SettingsService>,
    auth_service: State<'_, AuthService>,
) -> Result<Option<String>, String> {
    auth_service.check_admin(&token).await.map_err(|e| e.to_string())?;
    settings_service.get_value(&key).await.map_err(|e| e.to_string())
}

/// Upsert (create or update) setting
#[tauri::command]
pub async fn upsert_setting(
    token: String,
    key: String,
    value: String,
    description: Option<String>,
    settings_service: State<'_, SettingsService>,
    auth_service: State<'_, AuthService>,
) -> Result<Setting, String> {
    auth_service.check_admin(&token).await.map_err(|e| e.to_string())?;
    let dto = UpsertSettingDto { key, value, description };
    settings_service.upsert(dto).await.map_err(|e| e.to_string())
}

/// Delete setting
#[tauri::command]
pub async fn delete_setting(
    token: String,
    key: String,
    settings_service: State<'_, SettingsService>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    auth_service.check_admin(&token).await.map_err(|e| e.to_string())?;
    settings_service.delete(&key).await.map_err(|e| e.to_string())
}

/// Upload Logo
#[tauri::command]
pub async fn upload_logo(
    token: String,
    content: String,
    auth_service: State<'_, AuthService>,
    settings_service: State<'_, SettingsService>,
    app_handle: AppHandle,
) -> Result<String, String> {
    auth_service.check_admin(&token).await.map_err(|e| e.to_string())?;

    // Decode Base64
    let bytes = general_purpose::STANDARD
        .decode(content)
        .map_err(|e| format!("Invalid base64: {}", e))?;

    // Get App Data Dir
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    
    let uploads_dir = app_dir.join("uploads");
    if !uploads_dir.exists() {
        fs::create_dir_all(&uploads_dir).map_err(|e| e.to_string())?;
    }

    // Save file (always as logo.png for simplicity, or handle extensions if needed)
    // For now we enforce PNG or handle client side conversion
    let file_path = uploads_dir.join("logo.png");
    fs::write(&file_path, bytes).map_err(|e| e.to_string())?;

    // Save path to settings
    let path_str = file_path.to_string_lossy().to_string();
    let dto = UpsertSettingDto { 
        key: "app_logo_path".to_string(), 
        value: path_str.clone(), 
        description: Some("Path to application logo".to_string()) 
    };
    settings_service.upsert(dto).await.map_err(|e| e.to_string())?;

    Ok(path_str)
}

/// Get Logo as Base64
#[tauri::command]
pub async fn get_logo(
    app_handle: AppHandle,
) -> Result<Option<String>, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    
    let logo_path = app_dir.join("uploads").join("logo.png");
    
    if logo_path.exists() {
        let bytes = fs::read(&logo_path).map_err(|e| e.to_string())?;
        let base64_str = general_purpose::STANDARD.encode(&bytes);
        // Assuming PNG for now, but could detect magic bytes
        Ok(Some(format!("data:image/png;base64,{}", base64_str)))
    } else {
        Ok(None)
    }
}

/// Send test email to verify SMTP settings
#[tauri::command]
pub async fn send_test_email(
    token: String,
    to_email: String,
    auth_service: State<'_, AuthService>,
    settings_service: State<'_, SettingsService>,
) -> Result<String, String> {
    auth_service.check_admin(&token).await.map_err(|e| e.to_string())?;
    
    // Create email service with cloned settings service
    let email_service = crate::services::EmailService::new((*settings_service).clone());
    
    email_service
        .send_test_email(&to_email)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(format!("Test email sent successfully to {}", to_email))
}

