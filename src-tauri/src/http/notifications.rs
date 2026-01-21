use crate::error::AppResult;
use crate::http::AppState;
use crate::models::{CreatePushSubscriptionRequest, UpdatePreferenceRequest, UserResponse};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ListNotificationsQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_notifications))
        .route("/unread-count", get(get_unread_count))
        .route("/{id}/read", post(mark_as_read))
        .route("/read-all", post(mark_all_as_read))
        .route("/{id}", delete(delete_notification))
        .route("/preferences", get(get_preferences).put(update_preference))
        .route("/push/subscribe", post(subscribe_push))
        .route("/push/unsubscribe", post(unsubscribe_push))
        .route("/test", post(send_test_notification))
}

// Helper to get current user from headers
async fn get_current_user(state: &AppState, headers: &HeaderMap) -> AppResult<UserResponse> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    let user_response = state
        .auth_service
        .get_enriched_user(&claims.sub, claims.tenant_id)
        .await?;
    Ok(user_response)
}

// GET /api/notifications
async fn list_notifications(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Query<ListNotificationsQuery>,
) -> AppResult<Json<crate::models::PaginatedResponse<crate::models::Notification>>> {
    let user = get_current_user(&state, &headers).await?;
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);

    let result = state
        .notification_service
        .list_notifications(&user.id, page, per_page)
        .await?;
    Ok(Json(result))
}

// GET /api/notifications/unread-count
async fn get_unread_count(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user = get_current_user(&state, &headers).await?;
    let count = state
        .notification_service
        .get_unread_count(&user.id)
        .await?;
    Ok(Json(serde_json::json!({ "count": count })))
}

// POST /api/notifications/:id/read
async fn mark_as_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let user = get_current_user(&state, &headers).await?;
    state
        .notification_service
        .mark_as_read(&id, &user.id)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

// POST /api/notifications/read-all
async fn mark_all_as_read(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user = get_current_user(&state, &headers).await?;
    state
        .notification_service
        .mark_all_as_read(&user.id)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

// DELETE /api/notifications/:id
async fn delete_notification(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let user = get_current_user(&state, &headers).await?;
    state
        .notification_service
        .delete_notification(&id, &user.id)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

// GET /api/notifications/preferences
async fn get_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<crate::models::NotificationPreference>>> {
    let user = get_current_user(&state, &headers).await?;
    let prefs = state
        .notification_service
        .get_user_preferences(&user.id)
        .await?;
    Ok(Json(prefs))
}

// PUT /api/notifications/preferences
async fn update_preference(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdatePreferenceRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let user = get_current_user(&state, &headers).await?;
    state
        .notification_service
        .update_user_preference(&user.id, payload)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

// POST /api/notifications/push/subscribe
async fn subscribe_push(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreatePushSubscriptionRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let user = get_current_user(&state, &headers).await?;
    state
        .notification_service
        .subscribe_push(&user.id, payload)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

// POST /api/notifications/push/unsubscribe
async fn unsubscribe_push(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>, // Expecting { "endpoint": "..." }
) -> AppResult<Json<serde_json::Value>> {
    let _user = get_current_user(&state, &headers).await?;

    let endpoint = payload
        .get("endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::error::AppError::Validation("Endpoint is required".to_string()))?;

    state
        .notification_service
        .unsubscribe_push(endpoint)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

// POST /api/notifications/test
async fn send_test_notification(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user = get_current_user(&state, &headers).await?;

    state
        .notification_service
        .create_notification(
            user.id.clone(),
            None, // tenant_id not available in UserResponse
            "Test Notification".to_string(),
            "This is a test notification to verify delivery channels.".to_string(),
            "info".to_string(),
            "system".to_string(),
            Some("/profile".to_string()),
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
