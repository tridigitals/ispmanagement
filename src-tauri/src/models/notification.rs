use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Notification entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub tenant_id: Option<String>,
    pub title: String,
    pub message: String,
    pub notification_type: String, // "info", "success", "warning", "error"
    pub category: String,          // "system", "team", "payment", "security"
    pub action_url: Option<String>, // URL to navigate when clicked
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

impl Notification {
    pub fn new(
        user_id: String,
        tenant_id: Option<String>,
        title: String,
        message: String,
        notification_type: String,
        category: String,
        action_url: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            tenant_id,
            title,
            message,
            notification_type,
            category,
            action_url,
            is_read: false,
            created_at: Utc::now(),
        }
    }
}

/// User notification preferences
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationPreference {
    pub id: String,
    pub user_id: String,
    pub channel: String,  // "in_app", "email", "push"
    pub category: String, // "system", "team", "payment", "security"
    pub enabled: bool,
    pub updated_at: DateTime<Utc>,
}

impl NotificationPreference {
    #[allow(dead_code)]
    pub fn new(user_id: String, channel: String, category: String, enabled: bool) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            channel,
            category,
            enabled,
            updated_at: Utc::now(),
        }
    }
}

/// Push subscription (Web Push API)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PushSubscription {
    pub id: String,
    pub user_id: String,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub created_at: DateTime<Utc>,
}

impl PushSubscription {
    #[allow(dead_code)]
    pub fn new(user_id: String, endpoint: String, p256dh: String, auth: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            endpoint,
            p256dh,
            auth,
            created_at: Utc::now(),
        }
    }
}

// Request DTOs

#[derive(Debug, Deserialize)]
pub struct UpdatePreferenceRequest {
    pub channel: String,
    pub category: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreatePushSubscriptionRequest {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}
