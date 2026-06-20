use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::http::WsHub;
use crate::models::{
    CreatePushSubscriptionRequest, Notification, NotificationPreference, PaginatedResponse,
    PushSubscription, RegisterDeviceRequest, UpdatePreferenceRequest, UserDevice,
};
use crate::services::EmailOutboxService;
use crate::services::WhatsappGatewayService;
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
    email_outbox: EmailOutboxService,
    whatsapp_gateway: Option<WhatsappGatewayService>,
}

impl NotificationService {
    pub fn new(pool: DbPool, ws_hub: Arc<WsHub>, email_outbox: EmailOutboxService) -> Self {
        Self {
            pool,
            ws_hub,
            email_outbox,
            whatsapp_gateway: None,
        }
    }

    pub fn new_with_whatsapp(
        pool: DbPool,
        ws_hub: Arc<WsHub>,
        email_outbox: EmailOutboxService,
        whatsapp_gateway: WhatsappGatewayService,
    ) -> Self {
        Self {
            pool,
            ws_hub,
            email_outbox,
            whatsapp_gateway: Some(whatsapp_gateway),
        }
    }

    /// Send an email immediately, bypassing notification preferences.
    ///
    /// Used for "forced" deliveries such as admin-triggered broadcasts.
    pub async fn force_send_email(
        &self,
        tenant_id: Option<String>,
        to: &str,
        subject: &str,
        body: &str,
    ) -> AppResult<()> {
        self.email_outbox
            .send_or_enqueue(tenant_id, to, subject, body)
            .await
    }

    /// Force send/enqueue an email to a single address, optionally with HTML body.
    pub async fn force_send_email_with_html(
        &self,
        tenant_id: Option<String>,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: Option<String>,
    ) -> AppResult<()> {
        self.email_outbox
            .send_or_enqueue_with_html(tenant_id, to, subject, body_text, body_html)
            .await
    }

    /// Force send/enqueue an email with binary attachments.
    ///
    /// Phase 2 of bulk-send-invoice. Routes through `email_outbox` so the
    /// attachments persist with the queued row and survive retries. When
    /// `attachments` is empty, falls through to the no-attachment path.
    pub async fn force_send_email_with_attachments(
        &self,
        tenant_id: Option<String>,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: Option<&str>,
        attachments: Vec<crate::services::email_service::EmailAttachment>,
    ) -> AppResult<()> {
        self.email_outbox
            .send_or_enqueue_with_attachments(
                tenant_id,
                to,
                subject,
                body_text,
                body_html,
                attachments,
            )
            .await
    }

    /// Explicit WhatsApp send for admin-triggered actions (e.g. bulk invoice send).
    ///
    /// Unlike `deliver_whatsapp_notification`, this does NOT gate on the per-event
    /// WhatsApp toggle (`is_event_whatsapp_enabled`) — an explicit admin action
    /// should always attempt delivery regardless of the auto-notification settings.
    /// Returns `Ok(false)` when the gateway is not configured or the provider
    /// reports a non-success delivery; `Ok(true)` only on confirmed send.
    pub async fn force_send_whatsapp(
        &self,
        tenant_id: Option<&str>,
        event_code: &str,
        recipient_user_id: Option<&str>,
        phone: &str,
        message: &str,
    ) -> AppResult<bool> {
        let Some(gateway) = &self.whatsapp_gateway else {
            return Ok(false);
        };
        // Use the *_response variant so we can read the actual delivery outcome.
        // `send_text` swallows provider-level failures into Ok(()), which would
        // make whatsapp_sent inaccurate.
        match gateway
            .send_text_response(tenant_id, event_code, recipient_user_id, phone, message)
            .await
        {
            Ok(resp) => Ok(resp.ok),
            Err(e) => {
                tracing::warn!("force_send_whatsapp failed for {phone}: {e}");
                Ok(false)
            }
        }
    }

    /// Send an email to a set of users (by user_id), bypassing preferences.
    #[cfg(feature = "postgres")]
    pub async fn force_send_email_to_users(
        &self,
        tenant_id: Option<String>,
        user_ids: &[String],
        subject: &str,
        body: &str,
    ) -> AppResult<()> {
        self.email_outbox
            .send_or_enqueue_to_users(tenant_id, user_ids, subject, body)
            .await
    }

    /// Send an email to a set of users (by user_id), optionally with HTML body.
    /// Bypasses notification preferences (caller controls the recipient list).
    #[cfg(feature = "postgres")]
    pub async fn force_send_email_to_users_with_html(
        &self,
        tenant_id: Option<String>,
        user_ids: &[String],
        subject: &str,
        body_text: &str,
        body_html: Option<String>,
    ) -> AppResult<()> {
        self.email_outbox
            .send_or_enqueue_to_users_with_html(tenant_id, user_ids, subject, body_text, body_html)
            .await
    }

    /// Create and send a notification
    #[allow(clippy::too_many_arguments)]
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

    /// List all notifications for a user without pagination.
    pub async fn list_all_notifications(&self, user_id: &str) -> AppResult<Vec<Notification>> {
        let notifications = sqlx::query_as::<_, Notification>(
            r#"
            SELECT * FROM notifications
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(notifications)
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

    /// Delete ALL notifications for a user (used by mobile app "clear all")
    pub async fn delete_all_user_notifications(&self, user_id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM notifications WHERE user_id = $1")
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

    pub async fn unsubscribe_push_for_user(&self, endpoint: &str, user_id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM push_subscriptions WHERE endpoint = $1 AND user_id = $2")
            .bind(endpoint)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    // ================= FCM Device Registration =================

    pub async fn register_device(
        &self,
        user_id: &str,
        req: RegisterDeviceRequest,
    ) -> AppResult<()> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let token_preview: String = req.fcm_token.chars().take(20).collect();
        tracing::info!(
            "[FCM] register_device user_id={} platform={} token={}...",
            user_id,
            req.platform,
            token_preview
        );

        sqlx::query(
            r#"
            INSERT INTO user_devices (id, user_id, fcm_token, platform, device_info, updated_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (fcm_token) DO UPDATE SET
                user_id = $2, platform = $4, device_info = $5, updated_at = $6
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(&req.fcm_token)
        .bind(&req.platform)
        .bind(&req.device_info)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        tracing::info!("[FCM] register_device OK user_id={}", user_id);
        Ok(())
    }

    pub async fn unregister_device(&self, user_id: &str, fcm_token: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM user_devices WHERE fcm_token = $1 AND user_id = $2")
            .bind(fcm_token)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn send_fcm_push(
        &self,
        notif: &Notification,
        user_id: &str,
    ) -> AppResult<()> {
        // Firebase v1 API requires a service account JSON file
        let sa_path = match std::env::var("FIREBASE_SERVICE_ACCOUNT_PATH") {
            Ok(p) if !p.is_empty() => p,
            _ => {
                tracing::warn!("FIREBASE_SERVICE_ACCOUNT_PATH not set, skipping FCM push");
                return Ok(());
            }
        };

        let devices = sqlx::query_as::<_, UserDevice>(
            "SELECT * FROM user_devices WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if devices.is_empty() {
            return Ok(());
        }

        // Get OAuth2 access token from service account
        let access_token = match Self::get_firebase_access_token(&sa_path).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Failed to get Firebase access token: {}", e);
                return Ok(());
            }
        };

        // Extract project_id from service account
        let sa_json: serde_json::Value = match std::fs::read_to_string(&sa_path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            _ => return Ok(()),
        };
        let project_id = sa_json["project_id"].as_str().unwrap_or("");

        let client = reqwest::Client::new();
        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            project_id
        );

        for device in &devices {
            let payload = serde_json::json!({
                "message": {
                    "token": device.fcm_token,
                    "notification": {
                        "title": notif.title,
                        "body": notif.message,
                    },
                    "data": {
                        "notification_id": &notif.id,
                        "category": &notif.category,
                        "action_url": notif.action_url.as_deref().unwrap_or(""),
                    },
                    "android": {
                        "priority": "high",
                        "notification": {
                            "channel_id": "high_importance_channel",
                            "sound": "default",
                        }
                    }
                }
            });

            match client
                .post(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        tracing::info!("FCM v1 push sent to device {}", device.id);
                    } else {
                        let body = resp.text().await.unwrap_or_default();
                        tracing::warn!("FCM v1 push failed ({}): {}", status, body);
                        // If token expired, clear cache and retry once
                        if status.as_u16() == 401 {
                            tracing::info!("Token expired, will refresh on next attempt");
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("FCM v1 request error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Get Firebase OAuth2 access token from service account JSON.
    /// Uses JWT assertion flow (no extra deps beyond jsonwebtoken).
    async fn get_firebase_access_token(sa_path: &str) -> Result<String, String> {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        let sa_str = std::fs::read_to_string(sa_path)
            .map_err(|e| format!("read service account: {}", e))?;
        let sa: serde_json::Value = serde_json::from_str(&sa_str)
            .map_err(|e| format!("parse service account: {}", e))?;

        let client_email = sa["client_email"].as_str()
            .ok_or("missing client_email")?;
        let private_key_pem = sa["private_key"].as_str()
            .ok_or("missing private_key")?;

        let now = chrono::Utc::now().timestamp();

        // JWT claims for Google OAuth2
        #[derive(serde::Serialize)]
        struct Claims {
            iss: String,
            scope: String,
            aud: String,
            iat: i64,
            exp: i64,
        }

        let claims = Claims {
            iss: client_email.to_string(),
            scope: "https://www.googleapis.com/auth/firebase.messaging".to_string(),
            aud: "https://oauth2.googleapis.com/token".to_string(),
            iat: now,
            exp: now + 3600,
        };

        let header = Header::new(Algorithm::RS256);
        let key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
            .map_err(|e| format!("invalid RSA key: {}", e))?;
        let jwt = encode(&header, &claims, &key)
            .map_err(|e| format!("JWT encode: {}", e))?;

        // Exchange JWT for access token
        let client = reqwest::Client::new();
        let resp = client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt),
            ])
            .send()
            .await
            .map_err(|e| format!("token request: {}", e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("token exchange failed: {}", body));
        }

        let token_resp: serde_json::Value = resp.json().await
            .map_err(|e| format!("parse token response: {}", e))?;

        token_resp["access_token"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "no access_token in response".to_string())
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
                        let _ = self.unsubscribe_push_for_user(&sub.endpoint, user_id).await;
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
                if channel == "in_app" || channel == "push" {
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

                // Use outbox to ensure reliable delivery with retries.
                let _ = self
                    .email_outbox
                    .send_or_enqueue(notif.tenant_id.clone(), &email, &subject, &notif.message)
                    .await;
            }
        }

        // 3. Push (Web Push + FCM)
        if should_send("push", &notif.category) {
            let _ = self.send_push_notification(notif, &notif.user_id).await;
            let _ = self.send_fcm_push(notif, &notif.user_id).await;
        }

        // 4. WhatsApp
        if should_send("whatsapp", &notif.category) {
            let _ = self.deliver_whatsapp_notification(notif).await;
        }

        Ok(())
    }

    async fn deliver_whatsapp_notification(&self, notif: &Notification) -> AppResult<()> {
        let Some(whatsapp_gateway) = &self.whatsapp_gateway else {
            return Ok(());
        };
        let event_code = whatsapp_event_code_for_category(&notif.category);
        if !whatsapp_gateway
            .is_event_whatsapp_enabled(notif.tenant_id.as_deref(), event_code)
            .await?
        {
            return Ok(());
        }

        let phone = self.lookup_whatsapp_phone(notif).await?;
        let Some(phone) = phone.filter(|value| !value.trim().is_empty()) else {
            return Ok(());
        };

        whatsapp_gateway
            .send_text(
                notif.tenant_id.as_deref(),
                event_code,
                Some(&notif.user_id),
                &phone,
                &format!("{}\n\n{}", notif.title, notif.message),
            )
            .await
    }

    async fn lookup_whatsapp_phone(&self, notif: &Notification) -> AppResult<Option<String>> {
        if let Some(tenant_id) = &notif.tenant_id {
            let customer_phone: Option<String> = sqlx::query_scalar(
                r#"
                SELECT c.phone
                FROM customer_users cu
                JOIN customers c ON c.id = cu.customer_id
                WHERE cu.tenant_id = $1 AND cu.user_id = $2
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(&notif.user_id)
            .fetch_optional(&self.pool)
            .await?;
            if customer_phone
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .is_some()
            {
                return Ok(customer_phone);
            }
        }

        let address_phone: Option<String> = sqlx::query_scalar(
            r#"
            SELECT phone
            FROM user_addresses
            WHERE user_id = $1 AND phone IS NOT NULL AND phone != ''
            ORDER BY is_default_billing DESC, is_default_shipping DESC, created_at DESC
            LIMIT 1
            "#,
        )
        .bind(&notif.user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(address_phone)
    }
}

fn whatsapp_event_code_for_category(category: &str) -> &'static str {
    match category {
        "billing" | "payment" => "customer_invoice_due",
        "support" => "support_ticket_replied",
        "network" => "network_router_down",
        "operations" => "installation_scheduled",
        _ => "system_alert",
    }
}
