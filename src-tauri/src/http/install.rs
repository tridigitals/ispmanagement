//! Install HTTP Handlers

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::models::{CreateUserDto, UpdateUserDto, UpsertSettingDto, UserResponse};

#[derive(Serialize)]
pub struct IsInstalledResponse {
    installed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
    admin_name: String,
    admin_email: String,
    admin_password: String,
    app_name: Option<String>,
    app_url: Option<String>,
}

#[derive(Serialize)]
pub struct InstallResponse {
    success: bool,
    user: UserResponse,
    message: String,
}

/// Check if application is installed (has any users)
pub async fn check_installed(State(state): State<AppState>) -> Json<IsInstalledResponse> {
    let count = state.user_service.count().await.unwrap_or(0);
    Json(IsInstalledResponse {
        installed: count > 0,
    })
}

/// Install the application (create first admin user and configure settings)
pub async fn install_app(
    State(state): State<AppState>,
    Json(payload): Json<InstallRequest>,
) -> Result<Json<InstallResponse>, (StatusCode, String)> {
    // Check if already installed
    let count = state
        .user_service
        .count()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if count > 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Application is already installed".to_string(),
        ));
    }

    // 1. Save App Settings
    if let Some(app_name) = payload.app_name {
        let _ = state
            .settings_service
            .upsert(
                None,
                UpsertSettingDto {
                    key: "app_name".to_string(),
                    value: app_name,
                    description: Some("Application name".to_string()),
                },
                None,
                None,
            )
            .await;
    }

    if let Some(app_url) = payload.app_url {
        let _ = state
            .settings_service
            .upsert(
                None,
                UpsertSettingDto {
                    key: "app_url".to_string(),
                    value: app_url,
                    description: Some("Application Base URL".to_string()),
                },
                None,
                None,
            )
            .await;
    }

    // 2. Create admin user
    let dto = CreateUserDto {
        email: payload.admin_email,
        password: payload.admin_password,
        name: payload.admin_name,
    };

    let user_res = state
        .user_service
        .create(dto, None, None)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // 3. Update role to admin and set as super admin
    let update_dto = UpdateUserDto {
        email: None,
        name: None,
        role: Some("admin".to_string()),
        is_super_admin: Some(true),
        is_active: Some(true),
    };

    let admin_user = state
        .user_service
        .update(&user_res.id, update_dto, None, None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(InstallResponse {
        success: true,
        user: admin_user,
        message: "Installation complete! Settings configured and admin created.".to_string(),
    }))
}
