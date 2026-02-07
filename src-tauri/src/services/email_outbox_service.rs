use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::services::{EmailService, SettingsService};
use chrono::{DateTime, Utc};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct EmailOutboxService {
    pool: DbPool,
    settings_service: SettingsService,
    email_service: EmailService,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct EmailOutboxRow {
    pub id: String,
    pub to_email: String,
    pub subject: String,
    pub body: String,
    pub max_attempts: i32,
}

impl EmailOutboxService {
    pub fn new(pool: DbPool, settings_service: SettingsService, email_service: EmailService) -> Self {
        Self {
            pool,
            settings_service,
            email_service,
        }
    }

    async fn enabled(&self) -> bool {
        self.settings_service
            .get_value(None, "email_outbox_enabled")
            .await
            .ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(true)
    }

    async fn max_attempts_default(&self) -> i32 {
        self.settings_service
            .get_value(None, "email_outbox_max_attempts")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(5)
            .clamp(1, 25)
    }

    async fn base_delay_seconds(&self) -> i64 {
        self.settings_service
            .get_value(None, "email_outbox_base_delay_seconds")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(30)
            .clamp(5, 3600)
    }

    #[cfg(feature = "postgres")]
    async fn try_advisory_lock(&self, key: &str) -> bool {
        // Hash a string to a stable i64 lock id. This prevents collisions in typical setups.
        let locked: Result<bool, _> = sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtext($1))")
            .bind(key)
            .fetch_one(&self.pool)
            .await;
        locked.unwrap_or(false)
    }

    #[cfg(feature = "postgres")]
    async fn advisory_unlock(&self, key: &str) {
        let _ = sqlx::query_scalar::<_, bool>("SELECT pg_advisory_unlock(hashtext($1))")
            .bind(key)
            .fetch_one(&self.pool)
            .await;
    }

    #[cfg(not(feature = "postgres"))]
    async fn try_advisory_lock(&self, _key: &str) -> bool {
        true
    }

    #[cfg(not(feature = "postgres"))]
    async fn advisory_unlock(&self, _key: &str) {}

    pub async fn enqueue(
        &self,
        tenant_id: Option<String>,
        to_email: String,
        subject: String,
        body: String,
        max_attempts: Option<i32>,
        scheduled_at: Option<DateTime<Utc>>,
    ) -> AppResult<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let max_attempts = max_attempts.unwrap_or(self.max_attempts_default().await).clamp(1, 25);
        let scheduled_at = scheduled_at.unwrap_or(now);

        #[cfg(feature = "postgres")]
        {
            sqlx::query(
                r#"
                INSERT INTO email_outbox
                  (id, tenant_id, to_email, subject, body, status, attempts, max_attempts, scheduled_at, last_error, sent_at, created_at, updated_at)
                VALUES
                  ($1,$2,$3,$4,$5,'queued',0,$6,$7,NULL,NULL,$8,$9)
            "#,
            )
            .bind(&id)
            .bind(tenant_id.as_deref())
            .bind(&to_email)
            .bind(&subject)
            .bind(&body)
            .bind(max_attempts)
            .bind(scheduled_at)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        }

        Ok(id)
    }

    /// Send email using outbox if enabled, otherwise send directly.
    pub async fn send_or_enqueue(
        &self,
        tenant_id: Option<String>,
        to: &str,
        subject: &str,
        body: &str,
    ) -> AppResult<()> {
        if self.enabled().await {
            let _ = self
                .enqueue(
                    tenant_id,
                    to.to_string(),
                    subject.to_string(),
                    body.to_string(),
                    None,
                    None,
                )
                .await?;
            Ok(())
        } else {
            self.email_service.send_email(to, subject, body).await
        }
    }

    /// Send email to users (by user id). Uses outbox when enabled.
    #[cfg(feature = "postgres")]
    pub async fn send_or_enqueue_to_users(
        &self,
        tenant_id: Option<String>,
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

        for email in emails {
            // Keep it simple: enqueue each recipient separately.
            let _ = self
                .send_or_enqueue(tenant_id.clone(), &email, subject, body)
                .await;
        }

        Ok(())
    }

    pub async fn start_sender(&self) {
        let svc = self.clone();
        tokio::spawn(async move {
            info!("Email outbox sender started.");
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            let lock_key = "email_outbox_sender";
            let mut warned_missing_schema = false;

            loop {
                interval.tick().await;

                if !svc.enabled().await {
                    continue;
                }

                if !svc.try_advisory_lock(lock_key).await {
                    continue;
                }

                let res = svc.process_batch().await;
                svc.advisory_unlock(lock_key).await;

                if let Err(e) = res {
                    let msg = e.to_string();
                    if msg.contains("relation \"email_outbox\" does not exist") {
                        if !warned_missing_schema {
                            warned_missing_schema = true;
                            warn!("Email outbox paused: database schema not migrated yet (missing email_outbox table).");
                        }
                    } else {
                        error!("Email outbox sender failed: {}", msg);
                    }
                }
            }
        });
    }

    async fn process_batch(&self) -> AppResult<()> {
        #[cfg(not(feature = "postgres"))]
        {
            return Ok(());
        }

        #[cfg(feature = "postgres")]
        {
            let now = Utc::now();
            let base_delay = self.base_delay_seconds().await;

            let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

            let rows: Vec<EmailOutboxRow> = sqlx::query_as(
                r#"
                SELECT id::text, to_email, subject, body, max_attempts
                FROM email_outbox
                WHERE status = 'queued'
                  AND scheduled_at <= $1
                ORDER BY scheduled_at ASC, created_at ASC
                LIMIT 50
                FOR UPDATE SKIP LOCKED
            "#,
            )
            .bind(now)
            .fetch_all(&mut *tx)
            .await
            .map_err(AppError::Database)?;

            if rows.is_empty() {
                tx.commit().await.map_err(AppError::Database)?;
                return Ok(());
            }

            // Mark as sending + increment attempts (claim in this transaction).
            let ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
            sqlx::query(
                "UPDATE email_outbox SET status = 'sending', attempts = attempts + 1, updated_at = $1 WHERE id = ANY($2)",
            )
            .bind(now)
            .bind(&ids)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

            tx.commit().await.map_err(AppError::Database)?;

            for r in rows {
                // Re-read attempts/max_attempts best-effort
                let attempts: i32 = sqlx::query_scalar("SELECT attempts FROM email_outbox WHERE id = $1")
                    .bind(&r.id)
                    .fetch_one(&self.pool)
                    .await
                    .unwrap_or(1);

                let max_attempts: i32 = sqlx::query_scalar("SELECT max_attempts FROM email_outbox WHERE id = $1")
                    .bind(&r.id)
                    .fetch_one(&self.pool)
                    .await
                    .unwrap_or(r.max_attempts);

                match self
                    .email_service
                    .send_email(&r.to_email, &r.subject, &r.body)
                    .await
                {
                    Ok(_) => {
                        let _ = sqlx::query(
                            "UPDATE email_outbox SET status = 'sent', sent_at = $1, updated_at = $1, last_error = NULL WHERE id = $2",
                        )
                        .bind(now)
                        .bind(&r.id)
                        .execute(&self.pool)
                        .await;
                    }
                    Err(e) => {
                        let err_msg = format!("{}", e);
                        let is_final = attempts >= max_attempts;
                        let next_delay = (base_delay * (2_i64.saturating_pow((attempts - 1).max(0) as u32)))
                            .min(60 * 60);
                        let next_at = now + chrono::Duration::seconds(next_delay);

                        if is_final {
                            let _ = sqlx::query(
                                "UPDATE email_outbox SET status = 'failed', last_error = $1, updated_at = $2 WHERE id = $3",
                            )
                            .bind(&err_msg)
                            .bind(now)
                            .bind(&r.id)
                            .execute(&self.pool)
                            .await;
                        } else {
                            let _ = sqlx::query(
                                "UPDATE email_outbox SET status = 'queued', scheduled_at = $1, last_error = $2, updated_at = $3 WHERE id = $4",
                            )
                            .bind(next_at)
                            .bind(&err_msg)
                            .bind(now)
                            .bind(&r.id)
                            .execute(&self.pool)
                            .await;
                        }
                    }
                }
            }

            Ok(())
        }
    }
}
