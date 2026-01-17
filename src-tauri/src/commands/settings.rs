//! Settings Commands

use crate::models::{Setting, UpsertSettingDto};
use crate::services::{AuthService, SettingsService};
use crate::services::auth_service::AuthSettings;
use crate::http::websocket::{WsHub, WsEvent};
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

#[derive(serde::Serialize)]
pub struct PublicSettings {
    pub app_name: Option<String>,
    pub app_description: Option<String>,
    pub default_locale: Option<String>,
    pub maintenance_mode: bool,
    pub maintenance_message: Option<String>,
}

#[tauri::command]
pub async fn get_public_settings(
    settings_service: State<'_, SettingsService>,
) -> Result<PublicSettings, String> {
    let app_name = settings_service.get_value(None, "app_name").await.map_err(|e| e.to_string())?;
    let app_description = settings_service.get_value(None, "app_description").await.map_err(|e| e.to_string())?;
    let default_locale = settings_service.get_value(None, "default_locale").await.map_err(|e| e.to_string())?;
    let maintenance_mode_str = settings_service.get_value(None, "maintenance_mode").await.map_err(|e| e.to_string())?;
    let maintenance_message = settings_service.get_value(None, "maintenance_message").await.map_err(|e| e.to_string())?;
    
    let maintenance_mode = maintenance_mode_str.as_deref() == Some("true");
    
    Ok(PublicSettings {
        app_name,
        app_description,
        default_locale,
        maintenance_mode,
        maintenance_message,
    })
}

/// Get all settings
#[tauri::command]
pub async fn get_all_settings(
    token: String,
    settings_service: State<'_, SettingsService>,
    auth_service: State<'_, AuthService>,
) -> Result<Vec<Setting>, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    
    // Security check: must be admin or superadmin
    // (Actual role names might vary, usually 'admin' or 'owner')
    // Ideally use permission check, but for now checking role or super_admin flag
    if !claims.is_super_admin && claims.role != "admin" {
        return Err("Unauthorized".to_string());
    }

    // If Super Admin, fetch GLOBAL settings (tenant_id = None)
    // If Tenant Admin, fetch TENANT settings
    let target_tenant_id = if claims.is_super_admin {
        None
    } else {
        claims.tenant_id.as_deref()
    };

    settings_service.get_all(target_tenant_id).await.map_err(|e| e.to_string())
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
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    
    if !claims.is_super_admin && claims.role != "admin" {
        return Err("Unauthorized".to_string());
    }

    let target_tenant_id = if claims.is_super_admin {
        None
    } else {
        claims.tenant_id.as_deref()
    };

    settings_service.get_by_key(target_tenant_id, &key).await.map_err(|e| e.to_string())
}

/// Get setting value by key
#[tauri::command]
pub async fn get_setting_value(
    token: String,
    key: String,
    settings_service: State<'_, SettingsService>,
    auth_service: State<'_, AuthService>,
) -> Result<Option<String>, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    
    if !claims.is_super_admin && claims.role != "admin" {
        return Err("Unauthorized".to_string());
    }

    let target_tenant_id = if claims.is_super_admin {
        None
    } else {
        claims.tenant_id.as_deref()
    };

    settings_service.get_value(target_tenant_id, &key).await.map_err(|e| e.to_string())
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
    ws_hub: State<'_, Arc<WsHub>>,
) -> Result<Setting, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    if claims.role != "admin" {
        return Err("Unauthorized".to_string());
    }
    
    // For superadmin, save settings as GLOBAL (tenant_id = None)
    let tenant_id_for_save = if claims.is_super_admin {
        None
    } else {
        claims.tenant_id
    };
    
    let is_maintenance_mode = key == "maintenance_mode";
    let maintenance_enabled = value == "true";

    let dto = UpsertSettingDto { key, value, description };
    let setting = settings_service.upsert(tenant_id_for_save, dto, Some(&claims.sub), Some("127.0.0.1")).await.map_err(|e| e.to_string())?;

    // Broadcast maintenance mode change to all connected clients
    if is_maintenance_mode {
        // Get maintenance message if exists
        let maintenance_message = settings_service.get_value(None, "maintenance_message").await.ok().flatten();
        
        ws_hub.broadcast(WsEvent::MaintenanceModeChanged { 
            enabled: maintenance_enabled,
            message: maintenance_message,
        });
        println!("DEBUG: [Tauri] Broadcasted MaintenanceModeChanged event (enabled: {})", maintenance_enabled);
    }

    Ok(setting)
}

/// Delete setting
#[tauri::command]
pub async fn delete_setting(
    token: String,
    key: String,
    settings_service: State<'_, SettingsService>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    
    if !claims.is_super_admin && claims.role != "admin" {
        return Err("Unauthorized".to_string());
    }

    let target_tenant_id = if claims.is_super_admin {
        None
    } else {
        claims.tenant_id.as_deref()
    };

    settings_service.delete(target_tenant_id, &key, Some(&claims.sub), Some("127.0.0.1")).await.map_err(|e| e.to_string())
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
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;
    if claims.role != "admin" {
        return Err("Unauthorized".to_string());
    }

    // Decode Base64
    let bytes = general_purpose::STANDARD
        .decode(content)
        .map_err(|e| format!("Invalid base64: {}", e))?;

    // Get App Data Dir
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    
    let mut uploads_dir = app_dir.join("uploads");
    if let Some(tid) = &claims.tenant_id {
        uploads_dir = uploads_dir.join(tid);
    }

    if !uploads_dir.exists() {
        fs::create_dir_all(&uploads_dir).map_err(|e| e.to_string())?;
    }

    // Save file
    let file_path = uploads_dir.join("logo.png");
    fs::write(&file_path, bytes).map_err(|e| e.to_string())?;

    // Save path to settings
    let path_str = file_path.to_string_lossy().to_string();
    let dto = UpsertSettingDto { 
        key: "app_logo_path".to_string(), 
        value: path_str.clone(), 
        description: Some("Path to application logo".to_string()) 
    };
    settings_service.upsert(claims.tenant_id, dto, Some(&claims.sub), Some("127.0.0.1")).await.map_err(|e| e.to_string())?;

    Ok(path_str)
}

/// Get Logo as Base64
#[tauri::command]
pub async fn get_logo(
    token: Option<String>,
    app_handle: AppHandle,
    auth_service: State<'_, AuthService>,
) -> Result<Option<String>, String> {
    let mut tenant_id = None;

    if let Some(t) = token {
        if let Ok(claims) = auth_service.validate_token(&t).await {
            tenant_id = claims.tenant_id;
        }
    }

    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    
    let mut logo_path = app_dir.join("uploads");
    if let Some(tid) = tenant_id {
        logo_path = logo_path.join(tid);
    }
    logo_path = logo_path.join("logo.png");
    
    if logo_path.exists() {
        let bytes = fs::read(&logo_path).map_err(|e| e.to_string())?;
        let base64_str = general_purpose::STANDARD.encode(&bytes);
        Ok(Some(format!("data:image/png;base64,{}", base64_str)))
    } else {
        // Fallback to global if tenant logo not found
        let global_path = app_dir.join("uploads").join("logo.png");
        if global_path.exists() {
            let bytes = fs::read(&global_path).map_err(|e| e.to_string())?;
            let base64_str = general_purpose::STANDARD.encode(&bytes);
            Ok(Some(format!("data:image/png;base64,{}", base64_str)))
        } else {
            Ok(None)
        }
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

