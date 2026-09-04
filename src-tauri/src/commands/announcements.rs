//! Announcements / Broadcasts (tenant + global)

use crate::http::{WsEvent, WsHub};
use crate::models::{
    Announcement, CreateAnnouncementDto, PaginatedResponse, UpdateAnnouncementDto,
};
use crate::services::{encode_unsubscribe_token, AuditService, AuthService, NotificationService};
use chrono::Utc;
use std::collections::HashSet;
use tauri::State;
use uuid::Uuid;

use super::announcements_support_common::{
    active_subscriber_portal_user_ids, ann_changed_fields, ann_snapshot_json,
    can_access_admin_audience, customer_portal_user_ids, global_recipient_ids,
    is_internal_tenant_member, norm_audience, norm_format, norm_mode, norm_severity,
    package_subscriber_portal_user_ids, should_reschedule_delivery, strip_html_tags,
    suspended_subscriber_portal_user_ids, tenant_admin_user_ids, tenant_user_ids,
};

async fn send_announcement_notifications(
    pool: &crate::db::DbPool,
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
            match announcement.audience.as_str() {
                "admins" => {
                    recipients.extend(tenant_admin_user_ids(pool, tid).await.unwrap_or_default());
                }
                "customers" => {
                    recipients.extend(
                        customer_portal_user_ids(pool, tid)
                            .await
                            .unwrap_or_default(),
                    );
                }
                "active_subscribers" => {
                    recipients.extend(
                        active_subscriber_portal_user_ids(pool, tid)
                            .await
                            .unwrap_or_default(),
                    );
                }
                "suspended_subscribers" => {
                    recipients.extend(
                        suspended_subscriber_portal_user_ids(pool, tid)
                            .await
                            .unwrap_or_default(),
                    );
                }
                _ => {
                    // "all" — tenant members + customer portal users
                    recipients.extend(tenant_user_ids(pool, tid).await.unwrap_or_default());
                    recipients.extend(
                        customer_portal_user_ids(pool, tid)
                            .await
                            .unwrap_or_default(),
                    );
                }
            }

            // Optional: narrow down to specific package subscribers
            if let Some(pkg_id) = announcement.target_package_id.as_deref() {
                let pkg_users: HashSet<String> =
                    package_subscriber_portal_user_ids(pool, tid, pkg_id)
                        .await
                        .unwrap_or_default()
                        .into_iter()
                        .collect();
                recipients.retain(|u| pkg_users.contains(u));
            }
        } else {
            // Global: hormati `audience`. Lihat `global_recipient_ids` — dulu
            // baris ini mengirim ke semua user aktif dan mengabaikan audiens.
            let ids: Vec<String> = global_recipient_ids(pool, announcement.audience.as_str())
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
    pool: &crate::db::DbPool,
    notification_service: &NotificationService,
    announcement: &Announcement,
) {
    if !announcement.deliver_email {
        return;
    }

    let mut recipients: HashSet<String> = HashSet::new();

    if let Some(tid) = announcement.tenant_id.as_deref() {
        match announcement.audience.as_str() {
            "admins" => {
                recipients.extend(tenant_admin_user_ids(pool, tid).await.unwrap_or_default());
            }
            "customers" => {
                recipients.extend(
                    customer_portal_user_ids(pool, tid)
                        .await
                        .unwrap_or_default(),
                );
            }
            "active_subscribers" => {
                recipients.extend(
                    active_subscriber_portal_user_ids(pool, tid)
                        .await
                        .unwrap_or_default(),
                );
            }
            "suspended_subscribers" => {
                recipients.extend(
                    suspended_subscriber_portal_user_ids(pool, tid)
                        .await
                        .unwrap_or_default(),
                );
            }
            _ => {
                recipients.extend(tenant_user_ids(pool, tid).await.unwrap_or_default());
                recipients.extend(
                    customer_portal_user_ids(pool, tid)
                        .await
                        .unwrap_or_default(),
                );
            }
        }

        if let Some(pkg_id) = announcement.target_package_id.as_deref() {
            let pkg_users: HashSet<String> = package_subscriber_portal_user_ids(pool, tid, pkg_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .collect();
            recipients.retain(|u| pkg_users.contains(u));
        }
    } else {
        // Jalur email global: audiens juga dihormati di sini.
        let ids: Vec<String> = global_recipient_ids(pool, announcement.audience.as_str())
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
            let disabled_set: std::collections::HashSet<String> = disabled.into_iter().collect();
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

#[tauri::command]
pub async fn list_active_announcements(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<Vec<Announcement>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims.tenant_id.clone();
    let user_id = claims.sub.clone();

    let is_admin = if let Some(tid) = tenant_id.as_deref() {
        let is_internal_member = is_internal_tenant_member(&auth_service.pool, tid, &user_id)
            .await
            .unwrap_or(false);
        can_access_admin_audience(is_internal_member, claims.is_super_admin)
    } else {
        claims.is_super_admin
    };

    let now = Utc::now();

    #[cfg(feature = "postgres")]
    let rows: Vec<Announcement> = sqlx::query_as(
        r#"
        SELECT a.*
        FROM announcements a
        LEFT JOIN announcement_dismissals d
          ON d.announcement_id = a.id AND d.user_id = $1
        WHERE d.id IS NULL
          AND ($2::text IS NULL OR a.tenant_id IS NULL OR a.tenant_id = $2)
          AND a.deliver_in_app = true
          AND a.starts_at <= $3
          AND (a.ends_at IS NULL OR a.ends_at > $3)
          AND (
            a.audience = 'all'
            OR a.audience = 'customers'
            OR a.audience = 'active_subscribers'
            OR a.audience = 'suspended_subscribers'
            OR (a.audience = 'admins' AND $4 = true)
            OR (a.audience = 'target_package' AND EXISTS (
              SELECT 1 FROM customer_users cu
              JOIN customer_subscriptions cs ON cs.customer_id = cu.customer_id AND cs.tenant_id = cu.tenant_id
              WHERE cu.user_id = $1 AND cs.package_id = a.target_package_id AND cs.status IN ('active','suspended')
            ))
          )
        ORDER BY a.starts_at DESC
        LIMIT 5
    "#,
    )
    .bind(&user_id)
    .bind(tenant_id.as_deref())
    .bind(now)
    .bind(is_admin)
    .fetch_all(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    #[cfg(not(feature = "postgres"))]
    let rows: Vec<Announcement> = Vec::new();

    Ok(rows)
}

#[tauri::command]
pub async fn list_recent_announcements(
    token: String,
    page: Option<u32>,
    per_page: Option<u32>,
    search: Option<String>,
    severity: Option<String>,
    mode: Option<String>,
    auth_service: State<'_, AuthService>,
) -> Result<PaginatedResponse<Announcement>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims.tenant_id.clone();
    let user_id = claims.sub.clone();

    let is_admin = if let Some(tid) = tenant_id.as_deref() {
        let is_internal_member = is_internal_tenant_member(&auth_service.pool, tid, &user_id)
            .await
            .unwrap_or(false);
        can_access_admin_audience(is_internal_member, claims.is_super_admin)
    } else {
        claims.is_super_admin
    };

    let now = Utc::now();

    #[cfg(feature = "postgres")]
    let (rows, total) = {
        use sqlx::Postgres;
        use sqlx::QueryBuilder;

        let pg = crate::services::pagination::normalize(page.unwrap_or(1), per_page.unwrap_or(20));
        let page = pg.page;
        let per_page = pg.per_page;
        let offset: i64 = pg.offset;

        let search = search.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty());
        let severity = severity
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all");
        let mode = mode
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all");

        let mut qb_count: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM announcements a \
             LEFT JOIN announcement_dismissals d ON d.announcement_id = a.id AND d.user_id = ",
        );
        qb_count.push_bind(&user_id);
        qb_count.push(" WHERE d.id IS NULL");

        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT a.* FROM announcements a \
             LEFT JOIN announcement_dismissals d ON d.announcement_id = a.id AND d.user_id = ",
        );
        qb.push_bind(&user_id);
        qb.push(" WHERE d.id IS NULL");

        if let Some(tid) = tenant_id.as_deref() {
            qb_count.push(" AND (a.tenant_id IS NULL OR a.tenant_id = ");
            qb_count.push_bind(tid);
            qb_count.push(")");

            qb.push(" AND (a.tenant_id IS NULL OR a.tenant_id = ");
            qb.push_bind(tid);
            qb.push(")");
        } else {
            qb_count.push(" AND a.tenant_id IS NULL");
            qb.push(" AND a.tenant_id IS NULL");
        }

        qb_count.push(" AND a.deliver_in_app = true AND a.starts_at <= ");
        qb_count.push_bind(now);
        qb_count.push(" AND (a.audience IN ('all','customers','active_subscribers','suspended_subscribers') OR (a.audience = 'admins' AND ");
        qb_count.push_bind(is_admin);
        qb_count.push(" = true))");

        qb.push(" AND a.deliver_in_app = true AND a.starts_at <= ");
        qb.push_bind(now);
        qb.push(" AND (a.audience IN ('all','customers','active_subscribers','suspended_subscribers') OR (a.audience = 'admins' AND ");
        qb.push_bind(is_admin);
        qb.push(" = true))");

        if let Some(sev) = severity.as_deref() {
            qb_count.push(" AND a.severity = ");
            qb_count.push_bind(sev);
            qb.push(" AND a.severity = ");
            qb.push_bind(sev);
        }

        if let Some(m) = mode.as_deref() {
            qb_count.push(" AND a.mode = ");
            qb_count.push_bind(m);
            qb.push(" AND a.mode = ");
            qb.push_bind(m);
        }

        if let Some(q) = search {
            let like = format!("%{}%", q);
            qb_count.push(" AND (a.title ILIKE ");
            qb_count.push_bind(like.clone());
            qb_count.push(" OR a.body ILIKE ");
            qb_count.push_bind(like.clone());
            qb_count.push(")");

            qb.push(" AND (a.title ILIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR a.body ILIKE ");
            qb.push_bind(like);
            qb.push(")");
        }

        let total: i64 = qb_count
            .build_query_scalar()
            .fetch_one(&auth_service.pool)
            .await
            .map_err(|e| e.to_string())?;

        qb.push(" ORDER BY a.starts_at DESC");
        qb.push(" LIMIT ");
        qb.push_bind(per_page as i64);
        qb.push(" OFFSET ");
        qb.push_bind(offset);

        let rows: Vec<Announcement> = qb
            .build_query_as()
            .fetch_all(&auth_service.pool)
            .await
            .map_err(|e| e.to_string())?;
        (rows, total)
    };

    #[cfg(not(feature = "postgres"))]
    let (rows, total): (Vec<Announcement>, i64) = (Vec::new(), 0);

    // Metadata respons harus cocok dengan normalisasi query di atas.
    let pg = crate::services::pagination::normalize(page.unwrap_or(1), per_page.unwrap_or(20));
    let page = pg.page;
    let per_page = pg.per_page;

    Ok(PaginatedResponse {
        data: rows,
        total,
        page,
        per_page,
    })
}

#[tauri::command]
pub async fn get_announcement(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
) -> Result<Announcement, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims.tenant_id.clone();
    let user_id = claims.sub.clone();

    let is_admin = if let Some(tid) = tenant_id.as_deref() {
        let is_internal_member = is_internal_tenant_member(&auth_service.pool, tid, &user_id)
            .await
            .unwrap_or(false);
        can_access_admin_audience(is_internal_member, claims.is_super_admin)
    } else {
        claims.is_super_admin
    };

    let can_manage = if let Some(tid) = tenant_id.as_deref() {
        auth_service
            .has_permission(&user_id, tid, "announcements", "manage")
            .await
            .unwrap_or(false)
    } else {
        false
    } || claims.is_super_admin;

    let now = Utc::now();

    #[cfg(feature = "postgres")]
    let row: Announcement = if can_manage {
        sqlx::query_as(
            r#"
            SELECT *
            FROM announcements
            WHERE id = $1
              AND ($2::text IS NULL OR tenant_id IS NULL OR tenant_id = $2)
        "#,
        )
        .bind(&id)
        .bind(tenant_id.as_deref())
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query_as(
            r#"
            SELECT *
            FROM announcements
            WHERE id = $1
              AND deliver_in_app = true
              AND ($2::text IS NULL OR tenant_id IS NULL OR tenant_id = $2)
              AND starts_at <= $3
              AND (ends_at IS NULL OR ends_at > $3 OR notified_at IS NOT NULL)
              AND (
                audience = 'all'
                OR audience = 'customers'
                OR audience = 'active_subscribers'
                OR audience = 'suspended_subscribers'
                OR (audience = 'admins' AND $4 = true)
                OR (audience = 'target_package' AND EXISTS (
                  SELECT 1 FROM customer_users cu
                  JOIN customer_subscriptions cs ON cs.customer_id = cu.customer_id AND cs.tenant_id = cu.tenant_id
                  WHERE cu.user_id = $1 AND cs.package_id = a.target_package_id AND cs.status IN ('active','suspended')
                ))
              )
        "#,
        )
        .bind(&id)
        .bind(tenant_id.as_deref())
        .bind(now)
        .bind(is_admin)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?
    };

    #[cfg(not(feature = "postgres"))]
    let row: Announcement = Announcement {
        id,
        tenant_id,
        created_by: None,
        cover_file_id: None,
        title: "".into(),
        body: "".into(),
        severity: "info".into(),
        audience: "all".into(),
        mode: "post".into(),
        format: "plain".into(),
        deliver_in_app: true,
        deliver_email: false,
        deliver_email_force: true,
        starts_at: now,
        ends_at: None,
        notified_at: None,
        created_at: now,
        updated_at: now,
    };

    Ok(row)
}

#[tauri::command]
pub async fn dismiss_announcement(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let now = Utc::now();
    let did = Uuid::new_v4().to_string();

    #[cfg(feature = "postgres")]
    {
        let _ = sqlx::query(
            r#"
            INSERT INTO announcement_dismissals (id, announcement_id, user_id, dismissed_at)
            VALUES ($1,$2,$3,$4)
            ON CONFLICT (user_id, announcement_id) DO NOTHING
        "#,
        )
        .bind(&did)
        .bind(&id)
        .bind(&claims.sub)
        .bind(now)
        .execute(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn list_announcements_admin(
    token: String,
    scope: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    search: Option<String>,
    severity: Option<String>,
    mode: Option<String>,
    status: Option<String>,
    auth_service: State<'_, AuthService>,
) -> Result<PaginatedResponse<Announcement>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "announcements", "manage")
        .await
        .map_err(|e| e.to_string())?;

    let scope = scope.unwrap_or_else(|| "tenant".to_string());
    let now = Utc::now();

    #[cfg(feature = "postgres")]
    let (rows, total) = {
        use sqlx::Postgres;
        use sqlx::QueryBuilder;

        let pg = crate::services::pagination::normalize(page.unwrap_or(1), per_page.unwrap_or(20));
        let page = pg.page;
        let per_page = pg.per_page;
        let offset: i64 = pg.offset;

        let search = search.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty());
        let severity = severity
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all");
        let mode = mode
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all");
        let status = status
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all");

        let mut qb_count: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM announcements a WHERE 1=1");
        let mut qb: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT a.* FROM announcements a WHERE 1=1");

        match scope.as_str() {
            "global" if claims.is_super_admin => {
                qb_count.push(" AND a.tenant_id IS NULL");
                qb.push(" AND a.tenant_id IS NULL");
            }
            "all" if claims.is_super_admin => {
                // no tenant filter
            }
            _ => {
                qb_count.push(" AND a.tenant_id = ");
                qb_count.push_bind(&tenant_id);
                qb.push(" AND a.tenant_id = ");
                qb.push_bind(&tenant_id);
            }
        }

        if let Some(sev) = severity.as_deref() {
            qb_count.push(" AND a.severity = ");
            qb_count.push_bind(sev);
            qb.push(" AND a.severity = ");
            qb.push_bind(sev);
        }

        if let Some(m) = mode.as_deref() {
            qb_count.push(" AND a.mode = ");
            qb_count.push_bind(m);
            qb.push(" AND a.mode = ");
            qb.push_bind(m);
        }

        if let Some(st) = status.as_deref() {
            match st {
                "scheduled" => {
                    qb_count.push(" AND a.starts_at > ");
                    qb_count.push_bind(now);
                    qb.push(" AND a.starts_at > ");
                    qb.push_bind(now);
                }
                "expired" => {
                    qb_count.push(" AND a.ends_at IS NOT NULL AND a.ends_at <= ");
                    qb_count.push_bind(now);
                    qb.push(" AND a.ends_at IS NOT NULL AND a.ends_at <= ");
                    qb.push_bind(now);
                }
                "active" => {
                    qb_count.push(" AND a.starts_at <= ");
                    qb_count.push_bind(now);
                    qb_count.push(" AND (a.ends_at IS NULL OR a.ends_at > ");
                    qb_count.push_bind(now);
                    qb_count.push(")");

                    qb.push(" AND a.starts_at <= ");
                    qb.push_bind(now);
                    qb.push(" AND (a.ends_at IS NULL OR a.ends_at > ");
                    qb.push_bind(now);
                    qb.push(")");
                }
                _ => {}
            }
        }

        if let Some(q) = search {
            let like = format!("%{}%", q);
            qb_count.push(" AND (a.title ILIKE ");
            qb_count.push_bind(like.clone());
            qb_count.push(" OR a.body ILIKE ");
            qb_count.push_bind(like.clone());
            qb_count.push(")");

            qb.push(" AND (a.title ILIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR a.body ILIKE ");
            qb.push_bind(like);
            qb.push(")");
        }

        let total: i64 = qb_count
            .build_query_scalar()
            .fetch_one(&auth_service.pool)
            .await
            .map_err(|e| e.to_string())?;

        qb.push(" ORDER BY a.created_at DESC");
        qb.push(" LIMIT ");
        qb.push_bind(per_page as i64);
        qb.push(" OFFSET ");
        qb.push_bind(offset);

        let rows: Vec<Announcement> = qb
            .build_query_as()
            .fetch_all(&auth_service.pool)
            .await
            .map_err(|e| e.to_string())?;
        (rows, total)
    };

    #[cfg(not(feature = "postgres"))]
    let (rows, total): (Vec<Announcement>, i64) = (Vec::new(), 0);

    // Metadata respons harus cocok dengan normalisasi query di atas.
    let pg = crate::services::pagination::normalize(page.unwrap_or(1), per_page.unwrap_or(20));
    let page = pg.page;
    let per_page = pg.per_page;

    Ok(PaginatedResponse {
        data: rows,
        total,
        page,
        per_page,
    })
}

#[tauri::command]
pub async fn create_announcement_admin(
    token: String,
    dto: CreateAnnouncementDto,
    auth_service: State<'_, AuthService>,
    notification_service: State<'_, NotificationService>,
    audit_service: State<'_, AuditService>,
) -> Result<Announcement, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims.tenant_id.clone();
    let scope = dto.scope.clone().unwrap_or_else(|| "tenant".to_string());
    let target_tenant_id = if scope == "global" {
        if !claims.is_super_admin {
            return Err("Forbidden".to_string());
        }
        None
    } else {
        let tid = tenant_id.ok_or_else(|| "Tenant context required".to_string())?;
        Some(tid)
    };

    if let Some(tid) = target_tenant_id.as_deref() {
        auth_service
            .check_permission(&claims.sub, tid, "announcements", "manage")
            .await
            .map_err(|e| e.to_string())?;
    }

    if dto.title.trim().is_empty() || dto.body.trim().is_empty() {
        return Err("Title and body are required".to_string());
    }

    let now = Utc::now();
    let starts_at = dto.starts_at.unwrap_or(now);
    let ends_at = dto.ends_at;
    if let Some(e) = ends_at {
        if e <= starts_at {
            return Err("ends_at must be after starts_at".to_string());
        }
    }

    let id = Uuid::new_v4().to_string();
    let severity = norm_severity(dto.severity);
    let audience = norm_audience(dto.audience);
    let mode = norm_mode(dto.mode);
    let format = norm_format(dto.format);
    let deliver_in_app = dto.deliver_in_app.unwrap_or(true);
    let deliver_email = dto.deliver_email.unwrap_or(false);
    let deliver_email_force = dto.deliver_email_force.unwrap_or(true);
    let cover_file_id = dto.cover_file_id.clone();

    #[cfg(feature = "postgres")]
    let mut ann: Announcement = sqlx::query_as(
        r#"
        INSERT INTO announcements
          (id, tenant_id, created_by, cover_file_id, title, body, severity, audience, mode, format, deliver_in_app, deliver_email, deliver_email_force, starts_at, ends_at, notified_at, created_at, updated_at)
        VALUES
          ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,NULL,$16,$17)
        RETURNING *
    "#,
    )
    .bind(&id)
    .bind(target_tenant_id.clone())
    .bind(Some(claims.sub.clone()))
    .bind(cover_file_id.clone())
    .bind(dto.title.trim())
    .bind(dto.body.trim())
    .bind(&severity)
    .bind(&audience)
    .bind(&mode)
    .bind(&format)
    .bind(deliver_in_app)
    .bind(deliver_email)
    .bind(deliver_email_force)
    .bind(starts_at)
    .bind(ends_at)
    .bind(now)
    .bind(now)
    .fetch_one(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    #[cfg(not(feature = "postgres"))]
    let mut ann: Announcement = Announcement {
        id,
        tenant_id: target_tenant_id.clone(),
        created_by: Some(claims.sub.clone()),
        cover_file_id,
        title: dto.title,
        body: dto.body,
        severity,
        audience,
        mode,
        format,
        deliver_in_app,
        deliver_email,
        deliver_email_force,
        starts_at,
        ends_at,
        notified_at: None,
        created_at: now,
        updated_at: now,
    };

    if starts_at <= now
        && ends_at.map(|e| e > now).unwrap_or(true)
        && (deliver_in_app || deliver_email)
    {
        send_announcement_notifications(&auth_service.pool, &notification_service, &ann).await;

        #[cfg(feature = "postgres")]
        {
            send_announcement_emails(&auth_service.pool, &notification_service, &ann).await;
            ann = sqlx::query_as(
                "UPDATE announcements SET notified_at = $1 WHERE id = $2 RETURNING *",
            )
            .bind(now)
            .bind(&ann.id)
            .fetch_one(&auth_service.pool)
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    // Audit (best-effort)
    let create_details = serde_json::json!({
        "scope": scope,
        "delivered_immediately": ann.notified_at.is_some(),
        "announcement": ann_snapshot_json(&ann),
    })
    .to_string();
    audit_service
        .log(
            Some(&claims.sub),
            ann.tenant_id.as_deref(),
            "create",
            "announcements",
            Some(&ann.id),
            Some(create_details.as_str()),
            None,
        )
        .await;

    // If we delivered right away, log an explicit "publish" action as well (useful for filtering).
    if ann.notified_at.is_some() {
        let publish_details = serde_json::json!({
            "cause": "immediate",
            "scope": scope,
            "announcement": ann_snapshot_json(&ann),
        })
        .to_string();
        audit_service
            .log(
                Some(&claims.sub),
                ann.tenant_id.as_deref(),
                "publish",
                "announcements",
                Some(&ann.id),
                Some(publish_details.as_str()),
                None,
            )
            .await;
    }

    Ok(ann)
}

#[tauri::command]
pub async fn update_announcement_admin(
    token: String,
    id: String,
    dto: UpdateAnnouncementDto,
    auth_service: State<'_, AuthService>,
    audit_service: State<'_, AuditService>,
) -> Result<Announcement, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "announcements", "manage")
        .await
        .map_err(|e| e.to_string())?;

    #[cfg(feature = "postgres")]
    let existing: Announcement = sqlx::query_as(
        "SELECT * FROM announcements WHERE id = $1 AND (tenant_id = $2 OR ($3 = true AND tenant_id IS NULL))",
    )
    .bind(&id)
    .bind(&tenant_id)
    .bind(claims.is_super_admin)
    .fetch_one(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    let before = existing.clone();
    let now = Utc::now();
    let title = dto.title.unwrap_or(existing.title);
    let body = dto.body.unwrap_or(existing.body);
    let severity = if dto.severity.is_some() {
        norm_severity(dto.severity)
    } else {
        existing.severity
    };
    let audience = if dto.audience.is_some() {
        norm_audience(dto.audience)
    } else {
        existing.audience
    };
    let mode = if dto.mode.is_some() {
        norm_mode(dto.mode)
    } else {
        existing.mode
    };
    let format = if dto.format.is_some() {
        norm_format(dto.format)
    } else {
        existing.format
    };
    let deliver_in_app = dto.deliver_in_app.unwrap_or(existing.deliver_in_app);
    let deliver_email = dto.deliver_email.unwrap_or(existing.deliver_email);
    let deliver_email_force = dto
        .deliver_email_force
        .unwrap_or(existing.deliver_email_force);
    let cover_file_id = dto.cover_file_id.unwrap_or(existing.cover_file_id);
    let starts_at = dto.starts_at.unwrap_or(existing.starts_at);
    let ends_at = match dto.ends_at {
        Some(Some(dt)) => Some(dt),
        Some(None) => None,
        None => existing.ends_at,
    };
    if let Some(e) = ends_at {
        if e <= starts_at {
            return Err("ends_at must be after starts_at".to_string());
        }
    }

    // Jadwalkan ulang pengiriman bila `starts_at` digeser ke masa depan pada
    // pengumuman yang sudah terkirim; lihat `should_reschedule_delivery`.
    let jadwalkan_ulang =
        should_reschedule_delivery(existing.notified_at, existing.starts_at, starts_at, now);
    let notified_at = if jadwalkan_ulang {
        None
    } else {
        existing.notified_at
    };

    #[cfg(feature = "postgres")]
    let ann: Announcement = sqlx::query_as(
        r#"
        UPDATE announcements
        SET cover_file_id = $1,
            title = $2,
            body = $3,
            severity = $4,
            audience = $5,
            mode = $6,
            format = $7,
            deliver_in_app = $8,
            deliver_email = $9,
            deliver_email_force = $10,
            starts_at = $11,
            ends_at = $12,
            notified_at = $13,
            updated_at = $14
        WHERE id = $15
        RETURNING *
    "#,
    )
    .bind(cover_file_id)
    .bind(title.trim())
    .bind(body.trim())
    .bind(severity)
    .bind(audience)
    .bind(mode)
    .bind(format)
    .bind(deliver_in_app)
    .bind(deliver_email)
    .bind(deliver_email_force)
    .bind(starts_at)
    .bind(ends_at)
    .bind(notified_at)
    .bind(now)
    .bind(&id)
    .fetch_one(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Audit (best-effort)
    let changed = ann_changed_fields(&before, &ann);
    let update_details = serde_json::json!({
        "changed": changed,
        "from": ann_snapshot_json(&before),
        "to": ann_snapshot_json(&ann),
    })
    .to_string();
    audit_service
        .log(
            Some(&claims.sub),
            ann.tenant_id.as_deref(),
            "update",
            "announcements",
            Some(&id),
            Some(update_details.as_str()),
            None,
        )
        .await;

    Ok(ann)
}

#[tauri::command]
pub async fn delete_announcement_admin(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
    audit_service: State<'_, AuditService>,
) -> Result<(), String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "announcements", "manage")
        .await
        .map_err(|e| e.to_string())?;

    #[cfg(feature = "postgres")]
    {
        // Load for audit snapshot (also ensures row exists).
        let existing: Announcement = sqlx::query_as(
            "SELECT * FROM announcements WHERE id = $1 AND (tenant_id = $2 OR ($3 = true AND tenant_id IS NULL))",
        )
        .bind(&id)
        .bind(&tenant_id)
        .bind(claims.is_super_admin)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

        let _ = sqlx::query(
            "DELETE FROM announcements WHERE id = $1 AND (tenant_id = $2 OR ($3 = true AND tenant_id IS NULL))",
        )
        .bind(&id)
        .bind(&tenant_id)
        .bind(claims.is_super_admin)
        .execute(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

        let delete_details = serde_json::json!({
            "announcement": ann_snapshot_json(&existing),
        })
        .to_string();
        audit_service
            .log(
                Some(&claims.sub),
                existing.tenant_id.as_deref(),
                "delete",
                "announcements",
                Some(&id),
                Some(delete_details.as_str()),
                None,
            )
            .await;
    }

    Ok(())
}

#[cfg(feature = "postgres")]
#[tauri::command]
pub async fn process_due_announcements_command(
    auth_service: State<'_, AuthService>,
    notification_service: State<'_, NotificationService>,
    ws_hub: State<'_, std::sync::Arc<WsHub>>,
) -> Result<(), String> {
    let now = Utc::now();
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
    .fetch_all(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    for ann in due {
        send_announcement_notifications(&auth_service.pool, &notification_service, &ann).await;
        send_announcement_emails(&auth_service.pool, &notification_service, &ann).await;
        let _ = sqlx::query(
            "UPDATE announcements SET notified_at = $1 WHERE id = $2 AND notified_at IS NULL",
        )
        .bind(now)
        .bind(&ann.id)
        .execute(&auth_service.pool)
        .await;

        // Nudge clients via WS so banner can refresh quickly (client-side filter still applies).
        // We only send a broad hint; individual users will refresh via NotificationReceived too.
        ws_hub.broadcast(WsEvent::PermissionsChanged);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ann_changed_fields, ann_snapshot_json, norm_audience, norm_format, norm_mode,
        norm_severity, strip_html_tags,
    };
    use crate::models::Announcement;
    use chrono::{TimeZone, Utc};

    fn sample_announcement() -> Announcement {
        let ts = Utc
            .with_ymd_and_hms(2026, 3, 27, 0, 0, 0)
            .single()
            .expect("valid UTC timestamp");
        Announcement {
            id: "ann-1".to_string(),
            tenant_id: Some("tenant-1".to_string()),
            created_by: Some("user-1".to_string()),
            cover_file_id: Some("file-1".to_string()),
            title: "Maintenance".to_string(),
            body: "<p>Hello</p>   world".to_string(),
            severity: "info".to_string(),
            audience: "all".to_string(),
            mode: "post".to_string(),
            format: "html".to_string(),
            deliver_in_app: true,
            deliver_email: false,
            deliver_email_force: true,
            deliver_whatsapp: false,
            deliver_push: false,
            target_package_id: None,
            starts_at: ts,
            ends_at: Some(ts),
            notified_at: Some(ts),
            created_at: ts,
            updated_at: ts,
        }
    }

    #[test]
    fn strip_html_tags_collapses_whitespace_and_removes_tags() {
        let input = "<div>hello <b>rust</b>\n\t world</div>";
        assert_eq!(strip_html_tags(input), "hello rust world");
    }

    #[test]
    fn normalization_helpers_preserve_valid_and_default_invalid_values() {
        assert_eq!(norm_severity(Some("warning".to_string())), "warning");
        assert_eq!(norm_severity(Some("oops".to_string())), "info");

        assert_eq!(norm_audience(Some("admins".to_string())), "admins");
        assert_eq!(norm_audience(None), "all");

        assert_eq!(norm_mode(Some("banner".to_string())), "banner");
        assert_eq!(norm_mode(Some("invalid".to_string())), "post");

        assert_eq!(norm_format(Some("markdown".to_string())), "markdown");
        assert_eq!(norm_format(None), "plain");
    }

    #[test]
    fn ann_snapshot_json_and_changed_fields_characterize_mapping_shape() {
        let before = sample_announcement();
        let mut after = before.clone();
        after.title = "Maintenance window".to_string();
        after.deliver_email = true;
        after.ends_at = None;

        let changed = ann_changed_fields(&before, &after);
        assert_eq!(changed, vec!["title", "deliver_email", "ends_at"]);

        let snapshot = ann_snapshot_json(&before);
        assert_eq!(snapshot["id"], "ann-1");
        assert_eq!(snapshot["tenant_id"], "tenant-1");
        assert_eq!(snapshot["deliver_in_app"], true);
        assert_eq!(snapshot["deliver_email"], false);
        assert_eq!(snapshot["starts_at"], "2026-03-27T00:00:00+00:00");
    }
}
