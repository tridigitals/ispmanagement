use crate::models::{
    CreatePushSubscriptionRequest, Notification, NotificationPreference, PaginatedResponse,
    UpdatePreferenceRequest,
};
use crate::services::{AuthService, NotificationService};
use tauri::State;

/// List notifications with pagination
#[tauri::command]
pub async fn list_notifications(
    token: String,
    page: Option<u32>,
    per_page: Option<u32>,
    notification_service: State<'_, NotificationService>,
    auth_service: State<'_, AuthService>,
) -> Result<PaginatedResponse<Notification>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let page = page.unwrap_or(1);
    let per_page = per_page.unwrap_or(20);

    notification_service
        .list_notifications(&claims.sub, page, per_page)
        .await
        .map_err(|e| e.to_string())
}

/// Get unread notification count
#[tauri::command]
pub async fn get_unread_count(
    token: String,
    notification_service: State<'_, NotificationService>,
    auth_service: State<'_, AuthService>,
) -> Result<serde_json::Value, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let count = notification_service
        .get_unread_count(&claims.sub)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "count": count }))
}

/// Mark notification as read
#[tauri::command]
pub async fn mark_as_read(
    token: String,
    id: String,
    notification_service: State<'_, NotificationService>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    notification_service
        .mark_as_read(&id, &claims.sub)
        .await
        .map_err(|e| e.to_string())
}

/// Mark all notifications as read
#[tauri::command]
pub async fn mark_all_as_read(
    token: String,
    notification_service: State<'_, NotificationService>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    notification_service
        .mark_all_as_read(&claims.sub)
        .await
        .map_err(|e| e.to_string())
}

/// Delete notification
#[tauri::command]
pub async fn delete_notification(
    token: String,
    id: String,
    notification_service: State<'_, NotificationService>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    notification_service
        .delete_notification(&id, &claims.sub)
        .await
        .map_err(|e| e.to_string())
}

/// Get notification preferences
#[tauri::command]
pub async fn get_preferences(
    token: String,
    notification_service: State<'_, NotificationService>,
    auth_service: State<'_, AuthService>,
) -> Result<Vec<NotificationPreference>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    notification_service
        .get_user_preferences(&claims.sub)
        .await
        .map_err(|e| e.to_string())
}

/// Update notification preference
#[tauri::command]
pub async fn update_preference(
    token: String,
    channel: String,
    category: String,
    enabled: bool,
    notification_service: State<'_, NotificationService>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let req = UpdatePreferenceRequest {
        channel,
        category,
        enabled,
    };

    notification_service
        .update_user_preference(&claims.sub, req)
        .await
        .map_err(|e| e.to_string())
}

/// Subscribe to push notifications
#[tauri::command]
pub async fn subscribe_push(
    token: String,
    endpoint: String,
    p256dh: String,
    auth: String,
    notification_service: State<'_, NotificationService>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let req = CreatePushSubscriptionRequest {
        endpoint,
        p256dh,
        auth,
    };

    notification_service
        .subscribe_push(&claims.sub, req)
        .await
        .map_err(|e| e.to_string())
}

/// Unsubscribe from push notifications
#[tauri::command]
pub async fn unsubscribe_push(
    token: String,
    endpoint: String,
    notification_service: State<'_, NotificationService>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    notification_service
        .unsubscribe_push_for_user(&endpoint, &claims.sub)
        .await
        .map_err(|e| e.to_string())
}

/// Send test notification to self
#[tauri::command]
pub async fn send_test(
    token: String,
    notification_service: State<'_, NotificationService>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    // Create a test notification
    notification_service
        .create_notification(
            claims.sub.clone(),
            None,
            "Test Notification".to_string(),
            "This is a test notification sent from your profile.".to_string(),
            "info".to_string(),
            "system".to_string(),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
