use super::AppState;
use crate::models::{Announcement, CreateAnnouncementDto, PaginatedResponse, UpdateAnnouncementDto};
use crate::services::encode_unsubscribe_token;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashSet;
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

fn ann_changed_fields(before: &Announcement, after: &Announcement) -> Vec<&'static str> {
    let mut out = Vec::new();
    if before.cover_file_id != after.cover_file_id {
        out.push("cover_file_id");
    }
    if before.title != after.title {
        out.push("title");
    }
    if before.body != after.body {
        out.push("body");
    }
    if before.severity != after.severity {
        out.push("severity");
    }
    if before.audience != after.audience {
        out.push("audience");
    }
    if before.mode != after.mode {
        out.push("mode");
    }
    if before.format != after.format {
        out.push("format");
    }
    if before.deliver_in_app != after.deliver_in_app {
        out.push("deliver_in_app");
    }
    if before.deliver_email != after.deliver_email {
        out.push("deliver_email");
    }
    if before.deliver_email_force != after.deliver_email_force {
        out.push("deliver_email_force");
    }
    if before.starts_at != after.starts_at {
        out.push("starts_at");
    }
    if before.ends_at != after.ends_at {
        out.push("ends_at");
    }
    out
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

async fn auth_claims(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::services::auth_service::Claims, crate::error::AppError> {
    let token = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(crate::error::AppError::Unauthorized)?;

    state.auth_service.validate_token(token).await
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

#[derive(Deserialize)]
pub struct ListAdminParams {
    pub scope: Option<String>, // "tenant" | "global" | "all"
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub search: Option<String>,
    pub severity: Option<String>,
    pub mode: Option<String>,
    pub status: Option<String>, // "active" | "scheduled" | "expired"
}

#[derive(Deserialize)]
pub struct ListRecentParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub search: Option<String>,
    pub severity: Option<String>,
    pub mode: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/active", get(list_active))
        .route("/recent", get(list_recent))
        .route("/{id}", get(get_one))
        .route("/{id}/dismiss", post(dismiss))
        .route(
            "/admin",
            get(list_admin).post(create_announcement),
        )
        .route(
            "/admin/{id}",
            put(update_announcement).delete(delete_announcement),
        )
}

pub async fn get_one(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Announcement>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims.tenant_id.clone();
    let user_id = claims.sub.clone();

    let is_admin = if let Some(tid) = tenant_id.as_deref() {
        state
            .auth_service
            .has_permission(&user_id, tid, "admin", "access")
            .await
            .unwrap_or(false)
    } else {
        false
    } || claims.is_super_admin;

    let can_manage = if let Some(tid) = tenant_id.as_deref() {
        state
            .auth_service
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
        .fetch_one(&state.auth_service.pool)
        .await?
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
        .fetch_one(&state.auth_service.pool)
        .await?
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

    Ok(Json(row))
}

pub async fn list_active(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Announcement>>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims.tenant_id.clone();
    let user_id = claims.sub.clone();

    let is_admin = if let Some(tid) = tenant_id.as_deref() {
        state
            .auth_service
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
    .fetch_all(&state.auth_service.pool)
    .await?;

    #[cfg(not(feature = "postgres"))]
    let rows: Vec<Announcement> = Vec::new();

    Ok(Json(rows))
}

pub async fn list_recent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ListRecentParams>,
) -> Result<Json<PaginatedResponse<Announcement>>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims.tenant_id.clone();
    let user_id = claims.sub.clone();

    let is_admin = if let Some(tid) = tenant_id.as_deref() {
        state
            .auth_service
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

        let page = params.page.unwrap_or(1).max(1);
        let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
        let offset: i64 = ((page - 1) * per_page) as i64;

        let search = params.search.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty());
        let severity = params
            .severity
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all");
        let mode = params
            .mode
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

        // Tenant scoping: tenant users see global + their tenant. No tenant context sees global only.
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
            .fetch_one(&state.auth_service.pool)
            .await?;

        qb.push(" ORDER BY a.starts_at DESC");
        qb.push(" LIMIT ");
        qb.push_bind(per_page as i64);
        qb.push(" OFFSET ");
        qb.push_bind(offset);

        let rows: Vec<Announcement> = qb.build_query_as().fetch_all(&state.auth_service.pool).await?;
        (rows, total)
    };

    #[cfg(not(feature = "postgres"))]
    let (rows, total): (Vec<Announcement>, i64) = (Vec::new(), 0);

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);

    Ok(Json(PaginatedResponse {
        data: rows,
        total,
        page,
        per_page,
    }))
}

pub async fn dismiss(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
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
        .execute(&state.auth_service.pool)
        .await?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn list_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ListAdminParams>,
) -> Result<Json<PaginatedResponse<Announcement>>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "announcements", "manage")
        .await?;

    let scope = params.scope.unwrap_or_else(|| "tenant".to_string());
    let now = Utc::now();

    #[cfg(feature = "postgres")]
    let (rows, total) = {
        use sqlx::Postgres;
        use sqlx::QueryBuilder;

        let page = params.page.unwrap_or(1).max(1);
        let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
        let offset: i64 = ((page - 1) * per_page) as i64;

        let search = params.search.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty());
        let severity = params
            .severity
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all");
        let mode = params
            .mode
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all");
        let status = params
            .status
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all");

        let mut qb_count: QueryBuilder<Postgres> = QueryBuilder::new("SELECT COUNT(*) FROM announcements a WHERE 1=1");
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT a.* FROM announcements a WHERE 1=1");

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
            .fetch_one(&state.auth_service.pool)
            .await?;

        qb.push(" ORDER BY a.created_at DESC");
        qb.push(" LIMIT ");
        qb.push_bind(per_page as i64);
        qb.push(" OFFSET ");
        qb.push_bind(offset);

        let rows: Vec<Announcement> = qb.build_query_as().fetch_all(&state.auth_service.pool).await?;
        (rows, total)
    };

    #[cfg(not(feature = "postgres"))]
    let (rows, total): (Vec<Announcement>, i64) = (Vec::new(), 0);

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);

    Ok(Json(PaginatedResponse {
        data: rows,
        total,
        page,
        per_page,
    }))
}

async fn send_announcement_notifications(
    state: &AppState,
    announcement: &Announcement,
) -> Result<(), crate::error::AppError> {
    if !announcement.deliver_in_app {
        return Ok(());
    }

    let mut recipients: HashSet<String> = HashSet::new();

    #[cfg(feature = "postgres")]
    {
        if let Some(tid) = announcement.tenant_id.as_deref() {
            if announcement.audience == "admins" {
                recipients.extend(tenant_admin_user_ids(&state.auth_service.pool, tid).await?);
            } else {
                recipients.extend(tenant_user_ids(&state.auth_service.pool, tid).await?);
            }
        } else {
            // Global: notify all users (simple baseline)
            let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM users WHERE is_active = true")
                .fetch_all(&state.auth_service.pool)
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
        let _ = state
            .notification_service
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

    Ok(())
}

#[cfg(feature = "postgres")]
async fn send_announcement_emails(
    state: &AppState,
    announcement: &Announcement,
) -> Result<(), crate::error::AppError> {
    if !announcement.deliver_email {
        return Ok(());
    }

    let mut recipients: HashSet<String> = HashSet::new();

    if let Some(tid) = announcement.tenant_id.as_deref() {
        if announcement.audience == "admins" {
            recipients.extend(tenant_admin_user_ids(&state.auth_service.pool, tid).await?);
        } else {
            recipients.extend(tenant_user_ids(&state.auth_service.pool, tid).await?);
        }
    } else {
        let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM users WHERE is_active = true")
            .fetch_all(&state.auth_service.pool)
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
        .fetch_all(&state.auth_service.pool)
        .await
        .unwrap_or_default();
        if !disabled.is_empty() {
            let disabled_set: std::collections::HashSet<String> = disabled.into_iter().collect();
            ids.retain(|u| !disabled_set.contains(u));
        }
    }

    if ids.is_empty() {
        return Ok(());
    }

    let subject = format!("[Announcement] {}", announcement.title);

    let main_domain: Option<String> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE tenant_id IS NULL AND key = 'app_main_domain' LIMIT 1",
    )
    .fetch_optional(&state.auth_service.pool)
    .await
    .unwrap_or(None);

    let slug: Option<String> = if let Some(tid) = announcement.tenant_id.as_deref() {
        sqlx::query_scalar("SELECT slug FROM tenants WHERE id = $1 LIMIT 1")
            .bind(tid)
            .fetch_optional(&state.auth_service.pool)
            .await
            .unwrap_or(None)
    } else {
        None
    };

    let users: Vec<(String, String)> =
        sqlx::query_as("SELECT id, email FROM users WHERE id = ANY($1) AND is_active = true")
            .bind(&ids)
            .fetch_all(&state.auth_service.pool)
            .await
            .unwrap_or_default();

    for (user_id, email) in users {
        let open_url = match (main_domain.as_deref(), slug.as_deref()) {
            (Some(domain), Some(sl)) => {
                Some(format!("https://{}/{}/announcements/{}", domain, sl, announcement.id))
            }
            (Some(domain), None) => Some(format!("https://{}/announcements/{}", domain, announcement.id)),
            _ => None,
        };

        let unsub_url = if let Some(domain) = main_domain.as_deref() {
            if let Ok(tok) = encode_unsubscribe_token(
                &state.auth_service.pool,
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

        let _ = state
            .notification_service
            .force_send_email_with_html(
                announcement.tenant_id.clone(),
                &email,
                &subject,
                &plain_body,
                Some(html_body),
            )
            .await;
    }

    Ok(())
}

pub async fn create_announcement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<CreateAnnouncementDto>,
) -> Result<Json<Announcement>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims.tenant_id.clone();

    let scope = dto.scope.clone().unwrap_or_else(|| "tenant".to_string());
    let target_tenant_id = if scope == "global" {
        if !claims.is_super_admin {
            return Err(crate::error::AppError::Forbidden(
                "Forbidden".to_string(),
            ));
        }
        None
    } else {
        let tid = tenant_id.ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;
        Some(tid)
    };

    if let Some(tid) = target_tenant_id.as_deref() {
        state
            .auth_service
            .check_permission(&claims.sub, tid, "announcements", "manage")
            .await?;
    }

    if dto.title.trim().is_empty() || dto.body.trim().is_empty() {
        return Err(crate::error::AppError::Validation(
            "Title and body are required".to_string(),
        ));
    }

    let now = Utc::now();
    let starts_at = dto.starts_at.unwrap_or(now);
    let ends_at = dto.ends_at;
    if let Some(e) = ends_at {
        if e <= starts_at {
            return Err(crate::error::AppError::Validation(
                "ends_at must be after starts_at".to_string(),
            ));
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
    .fetch_one(&state.auth_service.pool)
    .await?;

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

    // If active immediately, deliver now and set notified_at.
    if starts_at <= now && ends_at.map(|e| e > now).unwrap_or(true) && (deliver_in_app || deliver_email) {
        let _ = send_announcement_notifications(&state, &ann).await;

        #[cfg(feature = "postgres")]
        {
            let _ = send_announcement_emails(&state, &ann).await;
        }

        #[cfg(feature = "postgres")]
        {
            ann = sqlx::query_as("UPDATE announcements SET notified_at = $1 WHERE id = $2 RETURNING *")
                .bind(now)
                .bind(&ann.id)
                .fetch_one(&state.auth_service.pool)
                .await?;
        }
    }

    // Audit (best-effort)
    let create_details = serde_json::json!({
        "scope": scope,
        "delivered_immediately": ann.notified_at.is_some(),
        "announcement": ann_snapshot_json(&ann),
    })
    .to_string();
    state
        .audit_service
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
        state
            .audit_service
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

    Ok(Json(ann))
}

pub async fn update_announcement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(dto): Json<UpdateAnnouncementDto>,
) -> Result<Json<Announcement>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "announcements", "manage")
        .await?;

    #[cfg(feature = "postgres")]
    let existing: Announcement = sqlx::query_as(
        "SELECT * FROM announcements WHERE id = $1 AND (tenant_id = $2 OR ($3 = true AND tenant_id IS NULL))",
    )
    .bind(&id)
    .bind(&tenant_id)
    .bind(claims.is_super_admin)
    .fetch_one(&state.auth_service.pool)
    .await?;

    #[cfg(not(feature = "postgres"))]
    let existing: Announcement = Announcement {
        id: id.clone(),
        tenant_id: Some(tenant_id.clone()),
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
        starts_at: Utc::now(),
        ends_at: None,
        notified_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let before = existing.clone();
    let now = Utc::now();
    let cover_file_id = dto.cover_file_id.unwrap_or(existing.cover_file_id);
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
    let starts_at = dto.starts_at.unwrap_or(existing.starts_at);
    let ends_at = dto.ends_at.or(existing.ends_at);
    if let Some(e) = ends_at {
        if e <= starts_at {
            return Err(crate::error::AppError::Validation(
                "ends_at must be after starts_at".to_string(),
            ));
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
            deliver_email_force = $10,
            starts_at = $11,
            ends_at = $12,
            updated_at = $13
        WHERE id = $14
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
    .bind(now)
    .bind(&id)
    .fetch_one(&state.auth_service.pool)
    .await?;

    #[cfg(not(feature = "postgres"))]
    let ann: Announcement = existing;

    // Audit (best-effort)
    let changed = ann_changed_fields(&before, &ann);
    let update_details = serde_json::json!({
        "changed": changed,
        "from": ann_snapshot_json(&before),
        "to": ann_snapshot_json(&ann),
    })
    .to_string();
    state
        .audit_service
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

    Ok(Json(ann))
}

pub async fn delete_announcement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "announcements", "manage")
        .await?;

    #[cfg(feature = "postgres")]
    {
        let existing: Announcement = sqlx::query_as(
            "SELECT * FROM announcements WHERE id = $1 AND (tenant_id = $2 OR ($3 = true AND tenant_id IS NULL))",
        )
        .bind(&id)
        .bind(&tenant_id)
        .bind(claims.is_super_admin)
        .fetch_one(&state.auth_service.pool)
        .await?;

        let _ = sqlx::query(
            "DELETE FROM announcements WHERE id = $1 AND (tenant_id = $2 OR ($3 = true AND tenant_id IS NULL))",
        )
        .bind(&id)
        .bind(&tenant_id)
        .bind(claims.is_super_admin)
        .execute(&state.auth_service.pool)
        .await?;

        let delete_details = serde_json::json!({
            "announcement": ann_snapshot_json(&existing),
        })
        .to_string();
        state
            .audit_service
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

    Ok(Json(serde_json::json!({ "ok": true })))
}

// --- Scheduler support ---

#[cfg(feature = "postgres")]
pub async fn process_due_announcements(state: &AppState) -> Result<(), String> {
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
    .fetch_all(&state.auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    for ann in due {
        let _ = send_announcement_notifications(state, &ann).await;
        let _ = send_announcement_emails(state, &ann).await;
        let _ = sqlx::query("UPDATE announcements SET notified_at = $1 WHERE id = $2 AND notified_at IS NULL")
            .bind(now)
            .bind(&ann.id)
            .execute(&state.auth_service.pool)
            .await;
    }

    Ok(())
}
