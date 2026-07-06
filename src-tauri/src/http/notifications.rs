use crate::error::AppResult;
use crate::http::AppState;
use crate::models::{
    CreatePushSubscriptionRequest, Notification, PaginatedResponse, RegisterDeviceRequest,
    UnregisterDeviceRequest, UnsubscribePushRequest, UpdatePreferenceRequest, UserResponse,
};
use crate::services::Claims;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
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
        .route("/", delete(delete_all_notifications))
        .route("/{id}", delete(delete_notification))
        .route("/preferences", get(get_preferences).put(update_preference))
        .route("/push/subscribe", post(subscribe_push))
        .route("/push/unsubscribe", post(unsubscribe_push))
        .route("/devices", post(register_device).delete(unregister_device))
        .route("/test", post(send_test_notification))
}

// Helper to get current user from headers
async fn get_current_user_and_claims(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<(UserResponse, Claims)> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    let user_response = state
        .auth_service
        .get_enriched_user(&claims.sub, claims.tenant_id.clone())
        .await?;
    Ok((user_response, claims))
}

async fn get_current_user(state: &AppState, headers: &HeaderMap) -> AppResult<UserResponse> {
    let (user, _) = get_current_user_and_claims(state, headers).await?;
    Ok(user)
}

// GET /api/notifications
async fn list_notifications(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Query<ListNotificationsQuery>,
) -> AppResult<Json<crate::models::PaginatedResponse<crate::models::Notification>>> {
    let (user, claims) = get_current_user_and_claims(&state, &headers).await?;
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);

    let result = if has_internal_app_access(&user) {
        state
            .notification_service
            .list_notifications(&user.id, page, per_page)
            .await?
    } else {
        let filtered = list_filtered_portal_notifications(&state, &user, &claims).await?;
        paginate_notifications(&filtered, page, per_page)
    };
    Ok(Json(result))
}

// GET /api/notifications/unread-count
async fn get_unread_count(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let (user, claims) = get_current_user_and_claims(&state, &headers).await?;
    let count = if has_internal_app_access(&user) {
        state
            .notification_service
            .get_unread_count(&user.id)
            .await?
    } else {
        list_filtered_portal_notifications(&state, &user, &claims)
            .await?
            .into_iter()
            .filter(|notification| !notification.is_read)
            .count() as i64
    };
    Ok(Json(serde_json::json!({ "count": count })))
}

fn has_internal_app_access(user: &UserResponse) -> bool {
    if user.is_super_admin {
        return true;
    }

    let role = user.role.trim().to_ascii_lowercase();
    if role == "owner" || role == "admin" {
        return true;
    }

    user.permissions.iter().any(|permission| {
        permission == "*"
            || permission == "admin:access"
            || (permission != "customers:read_own"
                && [
                    "admin:",
                    "team:",
                    "roles:",
                    "settings:",
                    "customers:",
                    "customer_locations:",
                    "billing:",
                    "work_orders:",
                    "pppoe:",
                    "network_",
                    "router_inventory:",
                    "ppp_profiles:",
                    "ip_pools:",
                    "isp_packages:",
                    "audit_logs:",
                    "email_outbox:",
                    "storage_console:",
                    "backups:",
                ]
                .iter()
                .any(|prefix| permission.starts_with(prefix)))
    })
}

async fn list_filtered_portal_notifications(
    state: &AppState,
    user: &UserResponse,
    claims: &Claims,
) -> AppResult<Vec<Notification>> {
    let tenant_id = claims.tenant_id.as_deref().ok_or_else(|| {
        crate::error::AppError::Validation(
            "Tenant context required for portal notifications".to_string(),
        )
    })?;
    let customer_id: Option<String> = sqlx::query_scalar(
        "SELECT customer_id FROM customer_users WHERE tenant_id = $1 AND user_id = $2 LIMIT 1",
    )
    .bind(tenant_id)
    .bind(&user.id)
    .fetch_optional(&state.auth_service.pool)
    .await?;
    let customer_id = customer_id.ok_or_else(|| {
        crate::error::AppError::Forbidden("You are not linked to any customer".to_string())
    })?;

    let notifications = state
        .notification_service
        .list_all_notifications(&user.id)
        .await?;
    let accessible_invoice_ids = state
        .payment_service
        .list_customer_portal_invoices(tenant_id, &customer_id, None)
        .await?
        .into_iter()
        .map(|invoice| invoice.id)
        .collect::<Vec<_>>();

    Ok(filter_portal_notifications(
        notifications,
        accessible_invoice_ids.as_slice(),
    ))
}

fn is_legacy_portal_invoice_reminder(notification: &Notification) -> bool {
    if notification.category != "billing" {
        return false;
    }
    if notification.action_url.as_deref() != Some("/dashboard/invoices") {
        return false;
    }

    let title = notification.title.trim().to_ascii_lowercase();
    title.starts_with("invoice due") || title.starts_with("invoice overdue")
}

fn invoice_id_from_action_url(action_url: Option<&str>) -> Option<String> {
    let raw = action_url?.trim();
    let suffix = raw.strip_prefix("/pay/")?;
    let invoice_id = suffix
        .split(['?', '#', '/'])
        .next()
        .map(str::trim)
        .unwrap_or_default();
    if invoice_id.is_empty() {
        return None;
    }
    Some(invoice_id.to_string())
}

fn filter_portal_notifications(
    notifications: Vec<Notification>,
    accessible_invoice_ids: &[String],
) -> Vec<Notification> {
    let accessible_invoice_ids = accessible_invoice_ids
        .iter()
        .cloned()
        .collect::<HashSet<_>>();

    notifications
        .into_iter()
        .filter(|notification| {
            if is_legacy_portal_invoice_reminder(notification) {
                return false;
            }

            let Some(invoice_id) = invoice_id_from_action_url(notification.action_url.as_deref())
            else {
                return true;
            };

            accessible_invoice_ids.contains(&invoice_id)
        })
        .collect()
}

fn paginate_notifications(
    notifications: &[Notification],
    page: u32,
    per_page: u32,
) -> PaginatedResponse<Notification> {
    let safe_page = page.max(1);
    let safe_per_page = per_page.max(1);
    let offset = ((safe_page - 1) * safe_per_page) as usize;
    let data = notifications
        .iter()
        .skip(offset)
        .take(safe_per_page as usize)
        .cloned()
        .collect::<Vec<_>>();

    PaginatedResponse {
        data,
        total: notifications.len() as i64,
        page: safe_page,
        per_page: safe_per_page,
    }
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

// DELETE /api/notifications — clear all notifications for the current user
async fn delete_all_notifications(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user = get_current_user(&state, &headers).await?;
    state
        .notification_service
        .delete_all_user_notifications(&user.id)
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
    Json(payload): Json<UnsubscribePushRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let user = get_current_user(&state, &headers).await?;

    state
        .notification_service
        .unsubscribe_push_for_user(&payload.endpoint, &user.id)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

// POST /api/notifications/devices
async fn register_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterDeviceRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let user = get_current_user(&state, &headers).await?;
    state
        .notification_service
        .register_device(&user.id, payload)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

// DELETE /api/notifications/devices
async fn unregister_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UnregisterDeviceRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let user = get_current_user(&state, &headers).await?;
    state
        .notification_service
        .unregister_device(&user.id, &payload.fcm_token)
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

#[cfg(test)]
mod tests {
    use super::{filter_portal_notifications, paginate_notifications};
    use crate::models::Notification;
    use chrono::Utc;

    fn notification(
        id: &str,
        category: &str,
        title: &str,
        action_url: Option<&str>,
        is_read: bool,
    ) -> Notification {
        Notification {
            id: id.to_string(),
            user_id: "user-1".to_string(),
            tenant_id: Some("tenant-1".to_string()),
            title: title.to_string(),
            message: "Message".to_string(),
            notification_type: "info".to_string(),
            category: category.to_string(),
            action_url: action_url.map(str::to_string),
            is_read,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn portal_filter_hides_legacy_billing_notification_routes() {
        let rows = vec![
            notification(
                "legacy",
                "billing",
                "Invoice overdue by 2 day(s)",
                Some("/dashboard/invoices"),
                false,
            ),
            notification(
                "valid",
                "billing",
                "Invoice created",
                Some("/pay/invoice-1"),
                false,
            ),
            notification(
                "announcement",
                "announcement",
                "Maintenance",
                Some("/announcements/ann-1"),
                true,
            ),
        ];

        let filtered = filter_portal_notifications(rows, &["invoice-1".to_string()]);

        assert_eq!(
            filtered.into_iter().map(|item| item.id).collect::<Vec<_>>(),
            vec!["valid".to_string(), "announcement".to_string()]
        );
    }

    #[test]
    fn portal_filter_hides_foreign_pay_links() {
        let rows = vec![
            notification(
                "foreign",
                "billing",
                "Invoice created",
                Some("/pay/foreign-invoice"),
                false,
            ),
            notification(
                "owned",
                "billing",
                "Invoice created",
                Some("/pay/owned-invoice"),
                false,
            ),
        ];

        let filtered = filter_portal_notifications(rows, &["owned-invoice".to_string()]);

        assert_eq!(
            filtered.into_iter().map(|item| item.id).collect::<Vec<_>>(),
            vec!["owned".to_string()]
        );
    }

    #[test]
    fn portal_filter_hides_all_pay_links_when_customer_has_no_accessible_invoices() {
        let rows = vec![notification(
            "foreign",
            "billing",
            "Invoice created",
            Some("/pay/foreign-invoice"),
            false,
        )];

        let filtered = filter_portal_notifications(rows, &[]);

        assert!(filtered.is_empty());
    }

    #[test]
    fn portal_pagination_runs_after_filtering() {
        let rows = vec![
            notification(
                "legacy-1",
                "billing",
                "Invoice due today",
                Some("/dashboard/invoices"),
                false,
            ),
            notification("keep-1", "support", "Support update", None, false),
            notification(
                "foreign",
                "billing",
                "Invoice created",
                Some("/pay/foreign"),
                false,
            ),
            notification(
                "keep-2",
                "billing",
                "Invoice created",
                Some("/pay/owned"),
                true,
            ),
        ];

        let filtered = filter_portal_notifications(rows, &["owned".to_string()]);
        let page = paginate_notifications(&filtered, 1, 2);

        assert_eq!(page.total, 2);
        assert_eq!(
            page.data
                .into_iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec!["keep-1".to_string(), "keep-2".to_string()]
        );
    }
}
