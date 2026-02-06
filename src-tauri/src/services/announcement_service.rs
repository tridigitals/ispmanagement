use crate::db::DbPool;
use crate::models::Announcement;
use crate::services::NotificationService;
use chrono::Utc;
use std::collections::HashSet;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct AnnouncementScheduler {
    pool: DbPool,
    notification_service: NotificationService,
}

impl AnnouncementScheduler {
    pub fn new(pool: DbPool, notification_service: NotificationService) -> Self {
        Self {
            pool,
            notification_service,
        }
    }

    pub async fn start(&self) {
        let pool = self.pool.clone();
        let notification_service = self.notification_service.clone();

        tokio::spawn(async move {
            info!("Announcement Scheduler started.");
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            let mut warned_missing_schema = false;

            loop {
                interval.tick().await;

                if let Err(e) = Self::process_due(&pool, &notification_service).await {
                    if e.contains("relation \"announcements\" does not exist")
                        || e.contains("relation \"announcement_dismissals\" does not exist")
                        || e.contains("relation \"users\" does not exist")
                    {
                        if !warned_missing_schema {
                            warned_missing_schema = true;
                            warn!(
                                "Announcement scheduler paused: database schema not migrated yet (missing announcements tables)."
                            );
                        }
                    } else {
                        error!("Announcement scheduler failed: {}", e);
                    }
                }
            }
        });
    }

    #[cfg(feature = "postgres")]
    async fn tenant_admin_user_ids(
        pool: &sqlx::Pool<sqlx::Postgres>,
        tenant_id: &str,
    ) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT DISTINCT tm.user_id
            FROM tenant_members tm
            JOIN role_permissions rp ON rp.role_id = tm.role_id
            WHERE tm.tenant_id = $1
              AND tm.role_id IS NOT NULL
              AND rp.permission_id = ANY($2)
        "#,
        )
        .bind(tenant_id)
        .bind(&["admin:access", "admin:*", "*"])
        .fetch_all(pool)
        .await
    }

    #[cfg(feature = "postgres")]
    async fn tenant_user_ids(
        pool: &sqlx::Pool<sqlx::Postgres>,
        tenant_id: &str,
    ) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT DISTINCT user_id FROM tenant_members WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_all(pool)
            .await
    }

    async fn send_announcement_notifications(
        pool: &DbPool,
        notification_service: &NotificationService,
        announcement: &Announcement,
    ) {
        if !announcement.deliver_in_app {
            return;
        }

        let mut recipients: HashSet<String> = HashSet::new();

        #[cfg(feature = "postgres")]
        {
            if let Some(tid) = announcement.tenant_id.as_deref() {
                if announcement.audience == "admins" {
                    recipients.extend(
                        Self::tenant_admin_user_ids(pool, tid)
                            .await
                            .unwrap_or_default(),
                    );
                } else {
                    recipients.extend(Self::tenant_user_ids(pool, tid).await.unwrap_or_default());
                }
            } else {
                let ids: Vec<String> =
                    sqlx::query_scalar("SELECT id FROM users WHERE is_active = true")
                        .fetch_all(pool)
                        .await
                        .unwrap_or_default();
                recipients.extend(ids);
            }
        }

        let title = announcement.title.clone();
        let msg = if announcement.body.len() > 180 {
            format!("{}…", &announcement.body[..180])
        } else {
            announcement.body.clone()
        };

        for uid in recipients {
            let _ = notification_service
                .create_notification(
                    uid,
                    announcement.tenant_id.clone(),
                    title.clone(),
                    msg.clone(),
                    announcement.severity.clone(),
                    "announcement".to_string(),
                    Some(format!("/announcements/{}", announcement.id)),
                )
                .await;
        }
    }

    #[cfg(feature = "postgres")]
    async fn send_announcement_emails(
        pool: &DbPool,
        notification_service: &NotificationService,
        announcement: &Announcement,
    ) {
        if !announcement.deliver_email {
            return;
        }

        let mut recipients: HashSet<String> = HashSet::new();

        if let Some(tid) = announcement.tenant_id.as_deref() {
            if announcement.audience == "admins" {
                recipients.extend(Self::tenant_admin_user_ids(pool, tid).await.unwrap_or_default());
            } else {
                recipients.extend(Self::tenant_user_ids(pool, tid).await.unwrap_or_default());
            }
        } else {
            let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM users WHERE is_active = true")
                .fetch_all(pool)
                .await
                .unwrap_or_default();
            recipients.extend(ids);
        }

        let mut ids: Vec<String> = recipients.into_iter().collect();
        ids.sort();

        let subject = format!("[Announcement] {}", announcement.title);

        let mut body = String::new();
        body.push_str(&announcement.title);
        body.push_str("\n\n");
        body.push_str(&announcement.body);

        // Best-effort include a link if app_main_domain is configured and announcement is tenant-scoped.
        if let Some(tid) = announcement.tenant_id.as_deref() {
            let main_domain: Option<String> = sqlx::query_scalar(
                "SELECT value FROM settings WHERE tenant_id IS NULL AND key = 'app_main_domain' LIMIT 1",
            )
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

            let slug: Option<String> =
                sqlx::query_scalar("SELECT slug FROM tenants WHERE id = $1 LIMIT 1")
                    .bind(tid)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None);

            if let (Some(domain), Some(slug)) = (main_domain, slug) {
                body.push_str("\n\nOpen in app:\n");
                body.push_str(&format!(
                    "https://{}/{}{}",
                    domain,
                    slug,
                    format!("/announcements/{}", announcement.id)
                ));
                body.push('\n');
            }
        }

        let _ = notification_service
            .force_send_email_to_users(&ids, &subject, &body)
            .await;
    }

    pub async fn process_due(
        pool: &DbPool,
        notification_service: &NotificationService,
    ) -> Result<(), String> {
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let due: Vec<Announcement> = sqlx::query_as(
            r#"
            SELECT *
            FROM announcements
            WHERE starts_at <= $1
              AND notified_at IS NULL
              AND (ends_at IS NULL OR ends_at > $1)
              AND (deliver_in_app = true OR deliver_email = true)
            ORDER BY starts_at ASC
            LIMIT 50
        "#,
        )
        .bind(now)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

        #[cfg(not(feature = "postgres"))]
        let due: Vec<Announcement> = Vec::new();

        for ann in due {
            Self::send_announcement_notifications(pool, notification_service, &ann).await;

            #[cfg(feature = "postgres")]
            {
                Self::send_announcement_emails(pool, notification_service, &ann).await;
            }

            #[cfg(feature = "postgres")]
            {
                let _ = sqlx::query(
                    "UPDATE announcements SET notified_at = $1 WHERE id = $2 AND notified_at IS NULL",
                )
                .bind(now)
                .bind(&ann.id)
                .execute(pool)
                .await;
            }
        }

        Ok(())
    }
}
