//! Announcements / Broadcasts (tenant + global)

use crate::http::{WsEvent, WsHub};
use crate::models::{Announcement, CreateAnnouncementDto, PaginatedResponse, UpdateAnnouncementDto};
use crate::services::{AuditService, AuthService, NotificationService};
use chrono::Utc;
use std::collections::HashSet;
use tauri::State;
use uuid::Uuid;

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

fn norm_severity(s: Option<String>) -> String {
    match s.as_deref() {
        Some("info") | Some("success") | Some("warning") | Some("error") => s.unwrap(),
        _ => "info".to_string(),
    }
}

fn norm_audience(a: Option<String>) -> String {
    match a.as_deref() {
        Some("all") | Some("admins") => a.unwrap(),
        _ => "all".to_string(),
    }
}

fn norm_mode(m: Option<String>) -> String {
    match m.as_deref() {
        Some("post") | Some("banner") => m.unwrap(),
        _ => "post".to_string(),
    }
}

fn norm_format(f: Option<String>) -> String {
    match f.as_deref() {
        Some("plain") | Some("markdown") | Some("html") => f.unwrap(),
        _ => "plain".to_string(),
    }
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
            if announcement.audience == "admins" {
                recipients.extend(tenant_admin_user_ids(pool, tid).await.unwrap_or_default());
            } else {
                recipients.extend(tenant_user_ids(pool, tid).await.unwrap_or_default());
            }
        } else {
            let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM users WHERE is_active = true")
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
    pool: &crate::db::DbPool,
    notification_service: &NotificationService,
    announcement: &Announcement,
) {
    if !announcement.deliver_email {
        return;
    }

    let mut recipients: HashSet<String> = HashSet::new();

    if let Some(tid) = announcement.tenant_id.as_deref() {
        if announcement.audience == "admins" {
            recipients.extend(tenant_admin_user_ids(pool, tid).await.unwrap_or_default());
        } else {
            recipients.extend(tenant_user_ids(pool, tid).await.unwrap_or_default());
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
    if announcement.format == "html" {
        body.push_str(&strip_html_tags(&announcement.body));
    } else {
        body.push_str(&announcement.body);
    }

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
                "https://{}/{}/announcements/{}",
                domain, slug, announcement.id
            ));
            body.push('\n');
        }
    }

    let _ = notification_service
        .force_send_email_to_users(announcement.tenant_id.clone(), &ids, &subject, &body)
        .await;
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
        auth_service
            .has_permission(&user_id, tid, "admin", "access")
            .await
            .unwrap_or(false)
    } else {
        false
    } || claims.is_super_admin;

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
            OR (a.audience = 'admins' AND $4 = true)
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
        auth_service
            .has_permission(&user_id, tid, "admin", "access")
            .await
            .unwrap_or(false)
    } else {
        false
    } || claims.is_super_admin;

    let now = Utc::now();

    #[cfg(feature = "postgres")]
    let (rows, total) = {
        use sqlx::Postgres;
        use sqlx::QueryBuilder;

        let page = page.unwrap_or(1).max(1);
        let per_page = per_page.unwrap_or(20).clamp(1, 100);
        let offset: i64 = ((page - 1) * per_page) as i64;

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
        qb_count.push(" AND (a.audience = 'all' OR (a.audience = 'admins' AND ");
        qb_count.push_bind(is_admin);
        qb_count.push(" = true))");

        qb.push(" AND a.deliver_in_app = true AND a.starts_at <= ");
        qb.push_bind(now);
        qb.push(" AND (a.audience = 'all' OR (a.audience = 'admins' AND ");
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

    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(20).clamp(1, 100);

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
        auth_service
            .has_permission(&user_id, tid, "admin", "access")
            .await
            .unwrap_or(false)
    } else {
        false
    } || claims.is_super_admin;

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
                OR (audience = 'admins' AND $4 = true)
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

        let page = page.unwrap_or(1).max(1);
        let per_page = per_page.unwrap_or(20).clamp(1, 100);
        let offset: i64 = ((page - 1) * per_page) as i64;

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

    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(20).clamp(1, 100);

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
    let cover_file_id = dto.cover_file_id.clone();

    #[cfg(feature = "postgres")]
    let mut ann: Announcement = sqlx::query_as(
        r#"
        INSERT INTO announcements
          (id, tenant_id, created_by, cover_file_id, title, body, severity, audience, mode, format, deliver_in_app, deliver_email, starts_at, ends_at, notified_at, created_at, updated_at)
        VALUES
          ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,NULL,$15,$16)
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
            ann = sqlx::query_as("UPDATE announcements SET notified_at = $1 WHERE id = $2 RETURNING *")
                .bind(now)
                .bind(&ann.id)
                .fetch_one(&auth_service.pool)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    audit_service
        .log(
            Some(&claims.sub),
            target_tenant_id.as_deref(),
            "create",
            "announcements",
            Some(&ann.id),
            Some(&format!("Created announcement: {}", ann.title)),
            None,
        )
        .await;

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
    let cover_file_id = dto.cover_file_id.unwrap_or(existing.cover_file_id);
    let starts_at = dto.starts_at.unwrap_or(existing.starts_at);
    let ends_at = dto.ends_at.or(existing.ends_at);
    if let Some(e) = ends_at {
        if e <= starts_at {
            return Err("ends_at must be after starts_at".to_string());
        }
    }

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
            starts_at = $10,
            ends_at = $11,
            updated_at = $12
        WHERE id = $13
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
    .bind(starts_at)
    .bind(ends_at)
    .bind(now)
    .bind(&id)
    .fetch_one(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "update",
            "announcements",
            Some(&id),
            Some("Updated announcement"),
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
        let _ = sqlx::query(
            "DELETE FROM announcements WHERE id = $1 AND (tenant_id = $2 OR ($3 = true AND tenant_id IS NULL))",
        )
        .bind(&id)
        .bind(&tenant_id)
        .bind(claims.is_super_admin)
        .execute(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "delete",
            "announcements",
            Some(&id),
            Some("Deleted announcement"),
            None,
        )
        .await;

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
