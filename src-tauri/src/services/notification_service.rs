use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::http::WsHub;
use crate::models::{
    CreatePushSubscriptionRequest, Notification, NotificationPreference, PaginatedResponse,
    PushSubscription, UpdatePreferenceRequest,
};
use crate::services::EmailService;
use axum::http::Uri;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

// Web Push (pure Rust implementation)
use base64ct::{Base64UrlUnpadded, Encoding};
use web_push_native::{
    jwt_simple::algorithms::ES256KeyPair, p256::PublicKey, Auth, WebPushBuilder,
};

#[derive(Clone)]
pub struct NotificationService {
    pool: DbPool,
    ws_hub: Arc<WsHub>,
    email_service: EmailService,
}

impl NotificationService {
    pub fn new(pool: DbPool, ws_hub: Arc<WsHub>, email_service: EmailService) -> Self {
        Self {
            pool,
            ws_hub,
            email_service,
        }
    }

    /// Send an email immediately, bypassing notification preferences.
    ///
    /// Used for "forced" deliveries such as admin-triggered broadcasts.
    pub async fn force_send_email(&self, to: &str, subject: &str, body: &str) -> AppResult<()> {
        self.email_service.send_email(to, subject, body).await
    }

    /// Send an email to a set of users (by user_id), bypassing preferences.
    #[cfg(feature = "postgres")]
    pub async fn force_send_email_to_users(
        &self,
        user_ids: &[String],
        subject: &str,
        body: &str,
    ) -> AppResult<()> {
        if user_ids.is_empty() {
            return Ok(());
        }

        let emails: Vec<String> = sqlx::query_scalar(
            "SELECT email FROM users WHERE id = ANY($1) AND is_active = true",
        )
        .bind(user_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let email_service = self.email_service.clone();
        let subject = subject.to_string();
        let body = body.to_string();

        tokio::spawn(async move {
            for email in emails {
                let _ = email_service.send_email(&email, &subject, &body).await;
            }
        });

        Ok(())
    }

    /// Create and send a notification
    pub async fn create_notification(
        &self,
        user_id: String,
        tenant_id: Option<String>,
        title: String,
        message: String,
        notification_type: String,
        category: String,
        action_url: Option<String>,
    ) -> AppResult<Notification> {
        let notification = Notification::new(
            user_id.clone(),
            tenant_id,
            title,
            message,
            notification_type,
            category,
            action_url,
        );

        #[cfg(feature = "postgres")]
        sqlx::query(r#"
            INSERT INTO notifications 
            (id, user_id, tenant_id, title, message, notification_type, category, action_url, is_read, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#)
        .bind(&notification.id)
        .bind(&notification.user_id)
        .bind(&notification.tenant_id)
        .bind(&notification.title)
        .bind(&notification.message)
        .bind(&notification.notification_type)
        .bind(&notification.category)
        .bind(&notification.action_url)
        .bind(notification.is_read)
        .bind(notification.created_at)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        #[cfg(feature = "sqlite")]
        sqlx::query(r#"
            INSERT INTO notifications 
            (id, user_id, tenant_id, title, message, notification_type, category, action_url, is_read, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#)
        .bind(&notification.id)
        .bind(&notification.user_id)
        .bind(&notification.tenant_id)
        .bind(&notification.title)
        .bind(&notification.message)
        .bind(&notification.notification_type)
        .bind(&notification.category)
        .bind(&notification.action_url)
        .bind(if notification.is_read { 1 } else { 0 })
        .bind(notification.created_at)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let _ = self.deliver_notification(&notification).await;

        Ok(notification)
    }

    /// List notifications for a user
    pub async fn list_notifications(
        &self,
        user_id: &str,
        page: u32,
        per_page: u32,
    ) -> AppResult<PaginatedResponse<Notification>> {
        let offset = (page - 1) * per_page;

        let notifications = sqlx::query_as::<_, Notification>(
            r#"
            SELECT * FROM notifications 
            WHERE user_id = $1 
            ORDER BY created_at DESC 
            LIMIT $2 OFFSET $3
        "#,
        )
        .bind(user_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(AppError::Database)?;

        Ok(PaginatedResponse {
            data: notifications,
            total,
            page,
            per_page,
        })
    }

    /// Get unread count
    pub async fn get_unread_count(&self, user_id: &str) -> AppResult<i64> {
        #[cfg(feature = "postgres")]
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = false",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        #[cfg(feature = "sqlite")]
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = 0",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(count)
    }

    /// Mark as read
    pub async fn mark_as_read(&self, id: &str, user_id: &str) -> AppResult<()> {
        #[cfg(feature = "postgres")]
        sqlx::query("UPDATE notifications SET is_read = true WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE notifications SET is_read = 1 WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    /// Mark all as read
    pub async fn mark_all_as_read(&self, user_id: &str) -> AppResult<()> {
        #[cfg(feature = "postgres")]
        sqlx::query("UPDATE notifications SET is_read = true WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        #[cfg(feature = "sqlite")]
        sqlx::query("UPDATE notifications SET is_read = 1 WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    /// Delete notification
    pub async fn delete_notification(&self, id: &str, user_id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM notifications WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    // ================= Preference Methods =================

    pub async fn get_user_preferences(
        &self,
        user_id: &str,
    ) -> AppResult<Vec<NotificationPreference>> {
        let prefs = sqlx::query_as::<_, NotificationPreference>(
            "SELECT * FROM notification_preferences WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(prefs)
    }

    pub async fn update_user_preference(
        &self,
        user_id: &str,
        req: UpdatePreferenceRequest,
    ) -> AppResult<()> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        sqlx::query(r#"
            INSERT INTO notification_preferences (id, user_id, channel, category, enabled, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id, channel, category) 
            DO UPDATE SET enabled = $5, updated_at = $6
        "#)
        .bind(&id)
        .bind(user_id)
        .bind(&req.channel)
        .bind(&req.category)
        .bind(req.enabled)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        #[cfg(feature = "sqlite")]
        sqlx::query(r#"
            INSERT INTO notification_preferences (id, user_id, channel, category, enabled, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id, channel, category) 
            DO UPDATE SET enabled = $5, updated_at = $6
        "#)
        .bind(&id)
        .bind(user_id)
        .bind(&req.channel)
        .bind(&req.category)
        .bind(if req.enabled { 1 } else { 0 })
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    // ================= Push Subscriptions =================

    pub async fn subscribe_push(
        &self,
        user_id: &str,
        req: CreatePushSubscriptionRequest,
    ) -> AppResult<()> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM push_subscriptions WHERE endpoint = $1)",
        )
        .bind(&req.endpoint)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(false);

        if exists {
            return Ok(());
        }

        sqlx::query(
            r#"
            INSERT INTO push_subscriptions (id, user_id, endpoint, p256dh, auth, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(req.endpoint)
        .bind(req.p256dh)
        .bind(req.auth)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    pub async fn unsubscribe_push(&self, endpoint: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM push_subscriptions WHERE endpoint = $1")
            .bind(endpoint)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// Send Push Notification using web-push-native
    pub async fn send_push_notification(
        &self,
        notif: &Notification,
        user_id: &str,
    ) -> AppResult<()> {
        let subscriptions = sqlx::query_as::<_, PushSubscription>(
            "SELECT * FROM push_subscriptions WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if subscriptions.is_empty() {
            return Ok(());
        }

        // Get VAPID private key from env
        let vapid_private_key = std::env::var("VAPID_PRIVATE_KEY").unwrap_or_default();
        if vapid_private_key.is_empty() {
            tracing::warn!("VAPID_PRIVATE_KEY not set. Skipping push notification.");
            return Ok(());
        }

        // Parse keypair
        let key_bytes = match Base64UrlUnpadded::decode_vec(&vapid_private_key) {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Invalid VAPID key encoding: {}", e);
                return Ok(());
            }
        };
        let key_pair = match ES256KeyPair::from_bytes(&key_bytes) {
            Ok(kp) => kp,
            Err(e) => {
                tracing::error!("Invalid VAPID key: {}", e);
                return Ok(());
            }
        };

        // Create payload
        let payload = serde_json::json!({
            "title": notif.title,
            "message": notif.message,
            "action_url": notif.action_url
        });
        let payload_bytes = payload.to_string().into_bytes();

        // Create HTTP client
        let client = reqwest::Client::new();

        for sub in subscriptions {
            // Decode subscription keys (Handle both URL-safe and Standard Base64 for backward compatibility)
            let p256dh_safe = sub.p256dh.replace('+', "-").replace('/', "_");
            let p256dh_safe = p256dh_safe.trim_end_matches('=');
            let p256dh_bytes = match Base64UrlUnpadded::decode_vec(p256dh_safe) {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("Invalid p256dh for {}: {}", sub.endpoint, e);
                    continue;
                }
            };

            let auth_safe = sub.auth.replace('+', "-").replace('/', "_");
            let auth_safe = auth_safe.trim_end_matches('=');
            let auth_bytes = match Base64UrlUnpadded::decode_vec(auth_safe) {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("Invalid auth for {}: {}", sub.endpoint, e);
                    continue;
                }
            };

            let public_key = match PublicKey::from_sec1_bytes(&p256dh_bytes) {
                Ok(pk) => pk,
                Err(e) => {
                    tracing::error!("Invalid public key for {}: {}", sub.endpoint, e);
                    continue;
                }
            };

            // Auth must be exactly 16 bytes
            if auth_bytes.len() != 16 {
                tracing::error!(
                    "Invalid auth length for {}: got {} bytes, expected 16",
                    sub.endpoint,
                    auth_bytes.len()
                );
                continue;
            }
            let auth = Auth::clone_from_slice(&auth_bytes);

            // Build push request
            let endpoint: Uri = match sub.endpoint.parse() {
                Ok(u) => u,
                Err(e) => {
                    tracing::error!("Invalid endpoint URL {}: {}", sub.endpoint, e);
                    continue;
                }
            };

            let builder = WebPushBuilder::new(endpoint, public_key, auth)
                .with_vapid(&key_pair, "mailto:admin@example.com");

            let request = match builder.build(payload_bytes.clone()) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Failed to build push request: {:?}", e);
                    continue;
                }
            };

            // Convert http::Request to reqwest and send
            let (parts, body) = request.into_parts();
            let url = format!("{}", parts.uri);

            let mut req_builder = client.post(&url);
            for (name, value) in parts.headers.iter() {
                if let Ok(v) = value.to_str() {
                    req_builder = req_builder.header(name.as_str(), v);
                }
            }
            req_builder = req_builder.body(body);

            match req_builder.send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        tracing::info!("Push sent to {}", sub.endpoint);
                    } else if response.status() == 410 {
                        let _ = self.unsubscribe_push(&sub.endpoint).await;
                        tracing::info!("Removed expired subscription: {}", sub.endpoint);
                    } else {
                        tracing::warn!(
                            "Push failed with status {}: {}",
                            response.status(),
                            sub.endpoint
                        );
                    }
                }
                Err(e) => {
                    tracing::error!("Push request failed: {}", e);
                }
            }
        }

        Ok(())
    }

    // ================= Delivery Logic =================

    async fn deliver_notification(&self, notif: &Notification) -> AppResult<()> {
        let prefs = self.get_user_preferences(&notif.user_id).await?;

        let should_send = |channel: &str, category: &str| -> bool {
            if category == "security" && channel == "email" {
                return true;
            }

            if !prefs
                .iter()
                .any(|p| p.channel == channel && p.category == category)
            {
                if channel == "in_app" {
                    return true;
                }

                return false;
            }

            prefs
                .iter()
                .any(|p| p.channel == channel && p.category == category && p.enabled)
        };

        // 1. In-App: Send WS Event
        if should_send("in_app", &notif.category) {
            let event = crate::http::WsEvent::NotificationReceived {
                user_id: notif.user_id.clone(),
                tenant_id: notif.tenant_id.clone(),
                id: notif.id.clone(),
                title: notif.title.clone(),
                message: notif.message.clone(),
                notification_type: notif.notification_type.clone(),
                category: notif.category.clone(),
                action_url: notif.action_url.clone(),
                created_at: notif.created_at.to_rfc3339(),
            };
            self.ws_hub.broadcast(event);

            if let Ok(count) = self.get_unread_count(&notif.user_id).await {
                self.ws_hub
                    .broadcast(crate::http::WsEvent::UnreadCountUpdated {
                        user_id: notif.user_id.clone(),
                        count,
                    });
            }
        }

        // 2. Email
        if should_send("email", &notif.category) {
            let user_email: Option<String> =
                sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
                    .bind(&notif.user_id)
                    .fetch_optional(&self.pool)
                    .await
                    .unwrap_or(None);

            if let Some(email) = user_email {
                let prefix = match notif.notification_type.as_str() {
                    "error" => "[Error] ",
                    "warning" => "[Alert] ",
                    "success" => "[Success] ",
                    _ => "",
                };
                let subject = format!("{}{}", prefix, notif.title);

                let email_service = self.email_service.clone();
                let message = notif.message.clone();
                tokio::spawn(async move {
                    let _ = email_service.send_email(&email, &subject, &message).await;
                });
            }
        }

        // 3. Push
        if should_send("push", &notif.category) {
            let _ = self.send_push_notification(notif, &notif.user_id).await;
        }

        Ok(())
    }
}
