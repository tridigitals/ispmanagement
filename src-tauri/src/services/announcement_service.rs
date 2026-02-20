use crate::db::DbPool;
use crate::models::Announcement;
use crate::services::encode_unsubscribe_token;
use crate::services::AuditService;
use crate::services::NotificationService;
use chrono::Utc;
use std::collections::HashSet;
use tracing::{error, info, warn};

fn strip_html_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn ann_snapshot_json(ann: &Announcement) -> serde_json::Value {
    serde_json::json!({
        "id": ann.id,
        "tenant_id": ann.tenant_id,
        "created_by": ann.created_by,
        "cover_file_id": ann.cover_file_id,
        "title": ann.title,
        "severity": ann.severity,
        "audience": ann.audience,
        "mode": ann.mode,
        "format": ann.format,
        "deliver_in_app": ann.deliver_in_app,
        "deliver_email": ann.deliver_email,
        "deliver_email_force": ann.deliver_email_force,
        "starts_at": ann.starts_at.to_rfc3339(),
        "ends_at": ann.ends_at.map(|d| d.to_rfc3339()),
        "notified_at": ann.notified_at.map(|d| d.to_rfc3339()),
        "created_at": ann.created_at.to_rfc3339(),
        "updated_at": ann.updated_at.to_rfc3339(),
    })
}

#[derive(Clone)]
pub struct AnnouncementScheduler {
    pool: DbPool,
    notification_service: NotificationService,
    audit_service: AuditService,
}

impl AnnouncementScheduler {
    pub fn new(
        pool: DbPool,
        notification_service: NotificationService,
        audit_service: AuditService,
    ) -> Self {
        Self {
            pool,
            notification_service,
            audit_service,
        }
    }

    pub async fn start(&self) {
        let pool = self.pool.clone();
        let notification_service = self.notification_service.clone();
        let audit_service = self.audit_service.clone();

        tokio::spawn(async move {
            info!("Announcement Scheduler started.");
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            let mut warned_missing_schema = false;

            loop {
                interval.tick().await;

                #[cfg(feature = "postgres")]
                {
                    // Prevent duplicate processing when running multiple instances.
                    let mut advisory_conn = match pool.acquire().await {
                        Ok(c) => c,
                        Err(e) => {
                            warn!("Announcement scheduler skipped: failed to acquire DB connection: {}", e);
                            continue;
                        }
                    };

                    let locked: bool =
                        sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtext($1))")
                            .bind("announcement_scheduler")
                            .fetch_one(&mut *advisory_conn)
                            .await
                            .unwrap_or(false);
                    if !locked {
                        continue;
                    }

                    if let Err(e) =
                        Self::process_due(&pool, &notification_service, &audit_service).await
                    {
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

                    let _ =
                        sqlx::query_scalar::<_, bool>("SELECT pg_advisory_unlock(hashtext($1))")
                            .bind("announcement_scheduler")
                            .fetch_one(&mut *advisory_conn)
                            .await;
                    continue;
                }

                #[cfg(not(feature = "postgres"))]
                if let Err(e) =
                    Self::process_due(&pool, &notification_service, &audit_service).await
                {
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
        .bind(["admin:access", "admin:*", "*"])
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
        let plain = if announcement.format == "html" {
            strip_html_tags(&announcement.body)
        } else {
            announcement.body.clone()
        };
        let msg = if plain.chars().count() > 180 {
            let short: String = plain.chars().take(180).collect();
            format!("{}…", short)
        } else {
            plain
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

        let mut ids: Vec<String> = recipients.into_iter().collect();
        ids.sort();

        if !announcement.deliver_email_force && !ids.is_empty() {
            let disabled: Vec<String> = sqlx::query_scalar(
                r#"
                SELECT user_id
                FROM notification_preferences
                WHERE user_id = ANY($1)
                  AND channel = 'email'
                  AND category = 'announcement'
                  AND enabled = false
            "#,
            )
            .bind(&ids)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
            if !disabled.is_empty() {
                let disabled_set: std::collections::HashSet<String> =
                    disabled.into_iter().collect();
                ids.retain(|u| !disabled_set.contains(u));
            }
        }

        if ids.is_empty() {
            return;
        }

        let subject = format!("[Announcement] {}", announcement.title);

        let main_domain: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE tenant_id IS NULL AND key = 'app_main_domain' LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        let slug: Option<String> = if let Some(tid) = announcement.tenant_id.as_deref() {
            sqlx::query_scalar("SELECT slug FROM tenants WHERE id = $1 LIMIT 1")
                .bind(tid)
                .fetch_optional(pool)
                .await
                .unwrap_or(None)
        } else {
            None
        };

        let users: Vec<(String, String)> =
            sqlx::query_as("SELECT id, email FROM users WHERE id = ANY($1) AND is_active = true")
                .bind(&ids)
                .fetch_all(pool)
                .await
                .unwrap_or_default();

        for (user_id, email) in users {
            let open_url = match (main_domain.as_deref(), slug.as_deref()) {
                (Some(domain), Some(sl)) => Some(format!(
                    "https://{}/{}/announcements/{}",
                    domain, sl, announcement.id
                )),
                (Some(domain), None) => Some(format!(
                    "https://{}/announcements/{}",
                    domain, announcement.id
                )),
                _ => None,
            };

            let unsub_url = if let Some(domain) = main_domain.as_deref() {
                if let Ok(tok) = encode_unsubscribe_token(
                    pool,
                    &user_id,
                    announcement.tenant_id.clone(),
                    "announcement",
                    "email",
                    365,
                )
                .await
                {
                    // Public endpoint serves a minimal HTML confirmation page.
                    Some(format!("https://{}/api/public/unsubscribe/{}", domain, tok))
                } else {
                    None
                }
            } else {
                None
            };

            let plain_body = {
                let mut b = String::new();
                b.push_str(&announcement.title);
                b.push_str("\n\n");
                if announcement.format == "html" {
                    b.push_str(&strip_html_tags(&announcement.body));
                } else {
                    b.push_str(&announcement.body);
                }
                if let Some(url) = open_url.as_deref() {
                    b.push_str("\n\nOpen in app:\n");
                    b.push_str(url);
                    b.push('\n');
                }
                if let Some(url) = unsub_url.as_deref() {
                    b.push_str("\n\nUnsubscribe:\n");
                    b.push_str(url);
                    b.push('\n');
                }
                b
            };

            let html_body = {
                let content = if announcement.format == "html" {
                    announcement.body.clone()
                } else {
                    let esc = announcement
                        .body
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;");
                    format!("<pre style=\"white-space:pre-wrap\">{}</pre>", esc)
                };

                let open = open_url
                    .as_deref()
                    .map(|u| format!("<p><a href=\"{u}\">Open in app</a></p>"))
                    .unwrap_or_default();
                let unsub = unsub_url
                    .as_deref()
                    .map(|u| format!("<p style=\"color:#6b7280;font-size:12px\">Unsubscribe: <a href=\"{u}\">{u}</a></p>"))
                    .unwrap_or_default();

                format!(
                    r#"<!doctype html>
<html>
<body style="font-family:ui-sans-serif,system-ui,-apple-system,Segoe UI,Roboto,Arial;line-height:1.5;color:#111827">
  <div style="max-width:640px;margin:0 auto;padding:20px">
    <div style="border:1px solid #e5e7eb;border-radius:14px;padding:18px">
      <div style="font-size:12px;letter-spacing:.12em;text-transform:uppercase;color:#6b7280">Announcement</div>
      <h1 style="margin:10px 0 0;font-size:20px">{}</h1>
      <div style="margin-top:12px">{}</div>
      {}
    </div>
    {}
  </div>
</body>
</html>"#,
                    announcement.title, content, open, unsub
                )
            };

            let _ = notification_service
                .force_send_email_with_html(
                    announcement.tenant_id.clone(),
                    &email,
                    &subject,
                    &plain_body,
                    Some(html_body),
                )
                .await;
        }
    }

    pub async fn process_due(
        pool: &DbPool,
        notification_service: &NotificationService,
        audit_service: &AuditService,
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

            // Audit best-effort: scheduler-driven publish (no user context).
            let publish_details = serde_json::json!({
                "cause": "scheduler",
                "scope": if ann.tenant_id.is_some() { "tenant" } else { "global" },
                "announcement": ann_snapshot_json(&ann),
            })
            .to_string();
            audit_service
                .log(
                    None,
                    ann.tenant_id.as_deref(),
                    "publish",
                    "announcements",
                    Some(&ann.id),
                    Some(publish_details.as_str()),
                    None,
                )
                .await;
        }

        Ok(())
    }
}
