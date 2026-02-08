//! Support Tickets (Tenant scoped)

use crate::http::{WsEvent, WsHub};
use crate::models::{
    FileRecord, PaginatedResponse, SupportTicket, SupportTicketDetail, SupportTicketListItem,
    SupportTicketMessage, SupportTicketMessageWithAttachments,
};
use crate::services::{AuditService, AuthService, NotificationService};
use chrono::Utc;
use std::collections::HashMap;
use std::collections::HashSet;
use tauri::State;
use uuid::Uuid;

#[cfg(feature = "postgres")]
async fn support_admin_user_ids(
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
    .bind(["support:read_all", "support:reply"])
    .fetch_all(pool)
    .await
}

#[cfg(feature = "postgres")]
async fn notify_support_admins_new_ticket(
    pool: &sqlx::Pool<sqlx::Postgres>,
    notification_service: &NotificationService,
    tenant_id: &str,
    ticket_id: &str,
    created_by: &str,
    subject: &str,
) {
    let admins = support_admin_user_ids(pool, tenant_id).await.unwrap_or_default();
    let creator_name: Option<String> = sqlx::query_scalar("SELECT name FROM users WHERE id = $1")
        .bind(created_by)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

    let title = "New support ticket".to_string();
    let message = match creator_name {
        Some(n) if !n.trim().is_empty() => format!("{n}: {subject}"),
        _ => subject.to_string(),
    };

    for uid in admins {
        if uid == created_by {
            continue;
        }
        let _ = notification_service
            .create_notification(
                uid,
                Some(tenant_id.to_string()),
                title.clone(),
                message.clone(),
                "info".to_string(),
                "support".to_string(),
                Some(format!("/admin/support/{ticket_id}")),
            )
            .await;
    }
}

#[cfg(feature = "postgres")]
async fn notify_support_ticket_reply(
    pool: &sqlx::Pool<sqlx::Postgres>,
    auth_service: &AuthService,
    notification_service: &NotificationService,
    tenant_id: &str,
    ticket: &SupportTicket,
    author_id: &str,
    is_internal: bool,
) {
    if is_internal {
        return;
    }

    let is_staff = auth_service
        .has_permission(author_id, tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);

    let mut sent: HashSet<String> = HashSet::new();

    if let Some(owner) = ticket.created_by.clone() {
        sent.insert(owner.clone());
        if owner != author_id {
            let title = if is_staff {
                "Support reply"
            } else {
                "Ticket updated"
            };
            let _ = notification_service
                .create_notification(
                    owner,
                    Some(tenant_id.to_string()),
                    title.to_string(),
                    ticket.subject.clone(),
                    "info".to_string(),
                    "support".to_string(),
                    Some(format!("/support/{}", ticket.id)),
                )
                .await;
        }
    }

    let admins = support_admin_user_ids(pool, tenant_id).await.unwrap_or_default();
    for uid in admins {
        if uid == author_id {
            continue;
        }
        if sent.contains(&uid) {
            continue;
        }
        sent.insert(uid.clone());

        let title = if is_staff {
            "Staff replied"
        } else {
            "Customer replied"
        };
        let _ = notification_service
            .create_notification(
                uid,
                Some(tenant_id.to_string()),
                title.to_string(),
                ticket.subject.clone(),
                "info".to_string(),
                "support".to_string(),
                Some(format!("/admin/support/{}", ticket.id)),
            )
            .await;
    }
}

#[cfg(feature = "postgres")]
async fn broadcast_support_ticket_message_created(
    pool: &sqlx::Pool<sqlx::Postgres>,
    ws_hub: &std::sync::Arc<WsHub>,
    tenant_id: &str,
    ticket: &SupportTicket,
    author_id: &str,
    is_internal: bool,
    message_id: &str,
) {
    let mut recipients: HashSet<String> = HashSet::new();

    if !is_internal {
        if let Some(owner) = ticket.created_by.clone() {
            if owner != author_id {
                recipients.insert(owner);
            }
        }
    }

    let admins = support_admin_user_ids(pool, tenant_id).await.unwrap_or_default();
    for uid in admins {
        if uid == author_id {
            continue;
        }
        recipients.insert(uid);
    }

    for uid in recipients {
        ws_hub.broadcast(WsEvent::SupportTicketMessageCreated {
            user_id: uid,
            tenant_id: Some(tenant_id.to_string()),
            ticket_id: ticket.id.clone(),
            message_id: message_id.to_string(),
        });
    }
}

#[derive(serde::Serialize)]
pub struct SupportTicketStats {
    pub all: i64,
    pub open: i64,
    pub pending: i64,
    pub closed: i64,
}

fn normalize_priority(p: Option<String>) -> String {
    match p.as_deref() {
        Some("low") | Some("normal") | Some("high") | Some("urgent") => p.unwrap(),
        _ => "normal".to_string(),
    }
}

fn normalize_status(s: Option<String>) -> Option<String> {
    match s.as_deref() {
        Some("open") | Some("pending") | Some("closed") => s,
        _ => None,
    }
}

#[tauri::command]
pub async fn list_support_tickets(
    token: String,
    status: Option<String>,
    search: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    auth_service: State<'_, AuthService>,
) -> Result<PaginatedResponse<SupportTicketListItem>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    // Permission model:
    // - support:read_all -> list all tickets in tenant
    // - support:read -> list only own tickets
    let can_all = auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);

    if !can_all {
        auth_service
            .check_permission(&claims.sub, &tenant_id, "support", "read")
            .await
            .map_err(|e| e.to_string())?;
    } else {
        // Still require at least read permission to avoid weird role setups.
        let _ = auth_service
            .check_permission(&claims.sub, &tenant_id, "support", "read_all")
            .await;
    }

    let st = normalize_status(status);

    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page.saturating_sub(1) * per_page) as i64;

    let search = search
        .as_deref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let (rows, total): (Vec<SupportTicketListItem>, i64) = if can_all {
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM support_tickets t
            LEFT JOIN users u ON u.id = t.created_by
            WHERE t.tenant_id = $1
              AND ($2::text IS NULL OR t.status = $2)
              AND (
                $3::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($3) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($3) || '%'
              )
        "#,
        )
        .bind(&tenant_id)
        .bind(st.clone())
        .bind(search.clone())
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

        let rows: Vec<SupportTicketListItem> = sqlx::query_as(
            r#"
            SELECT
                t.*,
                u.name AS created_by_name,
                (SELECT COUNT(*) FROM support_ticket_messages m WHERE m.ticket_id = t.id) AS message_count,
                (SELECT MAX(created_at) FROM support_ticket_messages m WHERE m.ticket_id = t.id) AS last_message_at
            FROM support_tickets t
            LEFT JOIN users u ON u.id = t.created_by
            WHERE t.tenant_id = $1
              AND ($2::text IS NULL OR t.status = $2)
              AND (
                $3::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($3) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($3) || '%'
              )
            ORDER BY COALESCE((SELECT MAX(created_at) FROM support_ticket_messages m WHERE m.ticket_id = t.id), t.updated_at) DESC
            LIMIT $4 OFFSET $5
        "#,
        )
        .bind(&tenant_id)
        .bind(st)
        .bind(search)
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

        (rows, total)
    } else {
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM support_tickets t
            LEFT JOIN users u ON u.id = t.created_by
            WHERE t.tenant_id = $1
              AND t.created_by = $2
              AND ($3::text IS NULL OR t.status = $3)
              AND (
                $4::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($4) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($4) || '%'
              )
        "#,
        )
        .bind(&tenant_id)
        .bind(&claims.sub)
        .bind(st.clone())
        .bind(search.clone())
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

        let rows: Vec<SupportTicketListItem> = sqlx::query_as(
            r#"
            SELECT
                t.*,
                u.name AS created_by_name,
                (SELECT COUNT(*) FROM support_ticket_messages m WHERE m.ticket_id = t.id) AS message_count,
                (SELECT MAX(created_at) FROM support_ticket_messages m WHERE m.ticket_id = t.id) AS last_message_at
            FROM support_tickets t
            LEFT JOIN users u ON u.id = t.created_by
            WHERE t.tenant_id = $1
              AND t.created_by = $2
              AND ($3::text IS NULL OR t.status = $3)
              AND (
                $4::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($4) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($4) || '%'
              )
            ORDER BY COALESCE((SELECT MAX(created_at) FROM support_ticket_messages m WHERE m.ticket_id = t.id), t.updated_at) DESC
            LIMIT $5 OFFSET $6
        "#,
        )
        .bind(&tenant_id)
        .bind(&claims.sub)
        .bind(st)
        .bind(search)
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

        (rows, total)
    };

    Ok(PaginatedResponse {
        data: rows,
        total,
        page,
        per_page,
    })
}

#[tauri::command]
pub async fn get_support_ticket_stats(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<SupportTicketStats, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    let can_all = auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);

    if !can_all {
        auth_service
            .check_permission(&claims.sub, &tenant_id, "support", "read")
            .await
            .map_err(|e| e.to_string())?;
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        all: i64,
        open: i64,
        pending: i64,
        closed: i64,
    }

    let row: Row = if can_all {
        sqlx::query_as(
            r#"
            SELECT
              COUNT(*) AS all,
              COALESCE(SUM(CASE WHEN status = 'open' THEN 1 ELSE 0 END), 0) AS open,
              COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0) AS pending,
              COALESCE(SUM(CASE WHEN status = 'closed' THEN 1 ELSE 0 END), 0) AS closed
            FROM support_tickets
            WHERE tenant_id = $1
        "#,
        )
        .bind(&tenant_id)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query_as(
            r#"
            SELECT
              COUNT(*) AS all,
              COALESCE(SUM(CASE WHEN status = 'open' THEN 1 ELSE 0 END), 0) AS open,
              COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0) AS pending,
              COALESCE(SUM(CASE WHEN status = 'closed' THEN 1 ELSE 0 END), 0) AS closed
            FROM support_tickets
            WHERE tenant_id = $1 AND created_by = $2
        "#,
        )
        .bind(&tenant_id)
        .bind(&claims.sub)
        .fetch_one(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?
    };

    Ok(SupportTicketStats {
        all: row.all,
        open: row.open,
        pending: row.pending,
        closed: row.closed,
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_support_ticket(
    token: String,
    subject: String,
    message: String,
    priority: Option<String>,
    attachment_ids: Option<Vec<String>>,
    auth_service: State<'_, AuthService>,
    notification_service: State<'_, NotificationService>,
    audit_service: State<'_, AuditService>,
) -> Result<SupportTicketDetail, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "support", "create")
        .await
        .map_err(|e| e.to_string())?;

    if subject.trim().is_empty() || message.trim().is_empty() {
        return Err("Subject and message are required".to_string());
    }

    let now = Utc::now();
    let ticket_id = Uuid::new_v4().to_string();
    let msg_id = Uuid::new_v4().to_string();
    let priority = normalize_priority(priority);

    let mut tx = auth_service.pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query(
        r#"
        INSERT INTO support_tickets (
            id, tenant_id, created_by, subject, status, priority, assigned_to,
            created_at, updated_at, closed_at
        )
        VALUES ($1,$2,$3,$4,'open',$5,NULL,$6,$7,NULL)
    "#,
    )
    .bind(&ticket_id)
    .bind(&tenant_id)
    .bind(&claims.sub)
    .bind(subject.trim())
    .bind(&priority)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        r#"
        INSERT INTO support_ticket_messages (id, ticket_id, author_id, body, is_internal, created_at)
        VALUES ($1,$2,$3,$4,false,$5)
    "#,
    )
    .bind(&msg_id)
    .bind(&ticket_id)
    .bind(&claims.sub)
    .bind(message.trim())
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    #[cfg(feature = "postgres")]
    if let Some(file_ids) = attachment_ids.as_ref().filter(|v| !v.is_empty()) {
        attach_files_pg(&mut tx, &tenant_id, &msg_id, file_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    let ticket: SupportTicket = sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1")
        .bind(&ticket_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let messages: Vec<SupportTicketMessage> = sqlx::query_as(
        "SELECT * FROM support_ticket_messages WHERE ticket_id = $1 ORDER BY created_at ASC",
    )
    .bind(&ticket_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    // Audit (best-effort)
    let audit_details = serde_json::json!({
        "subject": ticket.subject,
        "priority": ticket.priority,
        "message_id": msg_id,
        "attachments": attachment_ids.as_ref().map(|v| v.len()).unwrap_or(0),
    })
    .to_string();
    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "create",
            "support_ticket",
            Some(&ticket_id),
            Some(audit_details.as_str()),
            None,
        )
        .await;

    #[cfg(feature = "postgres")]
    notify_support_admins_new_ticket(
        &auth_service.pool,
        &notification_service,
        &tenant_id,
        &ticket_id,
        &claims.sub,
        &ticket.subject,
    )
    .await;

    // Always notify creator as well (useful for bell history and single-user tenants).
    let _ = notification_service
        .create_notification(
            claims.sub.clone(),
            Some(tenant_id.clone()),
            "Ticket created".to_string(),
            ticket.subject.clone(),
            "success".to_string(),
            "support".to_string(),
            Some(format!("/support/{ticket_id}")),
        )
        .await;

    let message_ids: Vec<String> = messages.iter().map(|m| m.id.clone()).collect();
    let att_map: HashMap<String, Vec<FileRecord>> = {
        #[cfg(feature = "postgres")]
        {
            fetch_attachments_map_pg(&auth_service.pool, &tenant_id, &ticket_id, &message_ids)
                .await
                .unwrap_or_default()
        }
        #[cfg(not(feature = "postgres"))]
        {
            HashMap::new()
        }
    };

    let messages = messages
        .into_iter()
        .map(|m| SupportTicketMessageWithAttachments {
            id: m.id.clone(),
            ticket_id: m.ticket_id,
            author_id: m.author_id,
            body: m.body,
            is_internal: m.is_internal,
            created_at: m.created_at,
            attachments: att_map.get(&m.id).cloned().unwrap_or_default(),
        })
        .collect();

    Ok(SupportTicketDetail { ticket, messages })
}

#[tauri::command]
pub async fn get_support_ticket(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
) -> Result<SupportTicketDetail, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    let can_all = auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);

    if !can_all {
        auth_service
            .check_permission(&claims.sub, &tenant_id, "support", "read")
            .await
            .map_err(|e| e.to_string())?;
    }

    let ticket: SupportTicket =
        sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1 AND tenant_id = $2")
            .bind(&id)
            .bind(&tenant_id)
            .fetch_one(&auth_service.pool)
            .await
            .map_err(|e| e.to_string())?;

    if !can_all && ticket.created_by.as_deref() != Some(claims.sub.as_str()) {
        return Err("Forbidden".to_string());
    }

    // Non-admins should not see internal notes.
    let can_internal = auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "internal")
        .await
        .unwrap_or(false);

    let messages: Vec<SupportTicketMessage> = if can_internal {
        sqlx::query_as(
            "SELECT * FROM support_ticket_messages WHERE ticket_id = $1 ORDER BY created_at ASC",
        )
        .bind(&id)
        .fetch_all(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query_as(
            "SELECT * FROM support_ticket_messages WHERE ticket_id = $1 AND is_internal = false ORDER BY created_at ASC",
        )
        .bind(&id)
        .fetch_all(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?
    };

    let message_ids: Vec<String> = messages.iter().map(|m| m.id.clone()).collect();
    let att_map: HashMap<String, Vec<FileRecord>> = {
        #[cfg(feature = "postgres")]
        {
            fetch_attachments_map_pg(&auth_service.pool, &tenant_id, &id, &message_ids)
                .await
                .unwrap_or_default()
        }
        #[cfg(not(feature = "postgres"))]
        {
            HashMap::new()
        }
    };

    let messages = messages
        .into_iter()
        .map(|m| SupportTicketMessageWithAttachments {
            id: m.id.clone(),
            ticket_id: m.ticket_id,
            author_id: m.author_id,
            body: m.body,
            is_internal: m.is_internal,
            created_at: m.created_at,
            attachments: att_map.get(&m.id).cloned().unwrap_or_default(),
        })
        .collect();

    Ok(SupportTicketDetail { ticket, messages })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn reply_support_ticket(
    token: String,
    id: String,
    message: String,
    is_internal: Option<bool>,
    attachment_ids: Option<Vec<String>>,
    auth_service: State<'_, AuthService>,
    notification_service: State<'_, NotificationService>,
    audit_service: State<'_, AuditService>,
    ws_hub: State<'_, std::sync::Arc<WsHub>>,
) -> Result<SupportTicketMessageWithAttachments, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "support", "reply")
        .await
        .map_err(|e| e.to_string())?;

    if message.trim().is_empty() {
        return Err("Message is required".to_string());
    }

    let ticket: SupportTicket =
        sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1 AND tenant_id = $2")
            .bind(&id)
            .bind(&tenant_id)
            .fetch_one(&auth_service.pool)
            .await
            .map_err(|e| e.to_string())?;

    if ticket.status == "closed" {
        return Err("Ticket is closed".to_string());
    }

    let can_all = auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);

    if !can_all && ticket.created_by.as_deref() != Some(claims.sub.as_str()) {
        return Err("Forbidden".to_string());
    }

    let is_internal = is_internal.unwrap_or(false);
    if is_internal {
        auth_service
            .check_permission(&claims.sub, &tenant_id, "support", "internal")
            .await
            .map_err(|e| e.to_string())?;
    }

    let now = Utc::now();
    let msg_id = Uuid::new_v4().to_string();

    let mut tx = auth_service.pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query(
        r#"
        INSERT INTO support_ticket_messages (id, ticket_id, author_id, body, is_internal, created_at)
        VALUES ($1,$2,$3,$4,$5,$6)
    "#,
    )
    .bind(&msg_id)
    .bind(&id)
    .bind(&claims.sub)
    .bind(message.trim())
    .bind(is_internal)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    #[cfg(feature = "postgres")]
    if let Some(file_ids) = attachment_ids.as_ref().filter(|v| !v.is_empty()) {
        attach_files_pg(&mut tx, &tenant_id, &msg_id, file_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    sqlx::query("UPDATE support_tickets SET updated_at = $1 WHERE id = $2")
        .bind(now)
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let msg: SupportTicketMessage =
        sqlx::query_as("SELECT * FROM support_ticket_messages WHERE id = $1")
            .bind(&msg_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    // Audit (best-effort)
    let audit_details = serde_json::json!({
        "message_id": msg_id,
        "internal": is_internal,
        "attachments": attachment_ids.as_ref().map(|v| v.len()).unwrap_or(0),
    })
    .to_string();
    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "reply",
            "support_ticket",
            Some(&id),
            Some(audit_details.as_str()),
            None,
        )
        .await;

    #[cfg(feature = "postgres")]
    notify_support_ticket_reply(
        &auth_service.pool,
        &auth_service,
        &notification_service,
        &tenant_id,
        &ticket,
        &claims.sub,
        is_internal,
    )
    .await;

    #[cfg(feature = "postgres")]
    broadcast_support_ticket_message_created(
        &auth_service.pool,
        ws_hub.inner(),
        &tenant_id,
        &ticket,
        &claims.sub,
        is_internal,
        &msg_id,
    )
    .await;

    let att_map: HashMap<String, Vec<FileRecord>> = {
        #[cfg(feature = "postgres")]
        {
            fetch_attachments_map_pg(
                &auth_service.pool,
                &tenant_id,
                &id,
                std::slice::from_ref(&msg.id),
            )
                .await
                .unwrap_or_default()
        }
        #[cfg(not(feature = "postgres"))]
        {
            HashMap::new()
        }
    };

    Ok(SupportTicketMessageWithAttachments {
        id: msg.id.clone(),
        ticket_id: msg.ticket_id,
        author_id: msg.author_id,
        body: msg.body,
        is_internal: msg.is_internal,
        created_at: msg.created_at,
        attachments: att_map.get(&msg.id).cloned().unwrap_or_default(),
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_support_ticket(
    token: String,
    id: String,
    status: Option<String>,
    priority: Option<String>,
    assigned_to: Option<String>,
    auth_service: State<'_, AuthService>,
    audit_service: State<'_, AuditService>,
    notification_service: State<'_, NotificationService>,
) -> Result<SupportTicket, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    // Update is admin-only (read_all + update is typical).
    auth_service
        .check_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .map_err(|e| e.to_string())?;

    let now = Utc::now();
    let status = normalize_status(status);
    let priority = priority.and_then(|p| {
        let p = p.to_lowercase();
        match p.as_str() {
            "low" | "normal" | "high" | "urgent" => Some(p),
            _ => None,
        }
    });

    if status.is_some() || priority.is_some() {
        auth_service
            .check_permission(&claims.sub, &tenant_id, "support", "update")
            .await
            .map_err(|e| e.to_string())?;
    }

    if assigned_to.is_some() {
        auth_service
            .check_permission(&claims.sub, &tenant_id, "support", "assign")
            .await
            .map_err(|e| e.to_string())?;
    }

    // Fetch ticket to ensure tenant scope.
    let existing: SupportTicket =
        sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1 AND tenant_id = $2")
            .bind(&id)
            .bind(&tenant_id)
            .fetch_one(&auth_service.pool)
            .await
            .map_err(|e| e.to_string())?;

    let old_status = existing.status.clone();
    let old_priority = existing.priority.clone();
    let old_assigned_to = existing.assigned_to.clone();

    let new_status = status.unwrap_or(existing.status);
    let new_priority = priority.unwrap_or(existing.priority);
    let assigned_to = assigned_to.or(existing.assigned_to);
    let closed_at = if new_status == "closed" {
        Some(now)
    } else {
        None
    };

    let ticket: SupportTicket = sqlx::query_as(
        r#"
        UPDATE support_tickets
        SET status = $1,
            priority = $2,
            assigned_to = $3,
            updated_at = $4,
            closed_at = $5
        WHERE id = $6 AND tenant_id = $7
        RETURNING *
    "#,
    )
    .bind(new_status)
    .bind(new_priority)
    .bind(assigned_to)
    .bind(now)
    .bind(closed_at)
    .bind(&id)
    .bind(&tenant_id)
    .fetch_one(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    let status_changed = ticket.status != old_status;
    let assigned_changed = ticket.assigned_to != old_assigned_to;
    let owner_id = ticket.created_by.clone();
    let assignee_id = ticket.assigned_to.clone();

    // Audit (best-effort)
    let action = if old_status != "closed" && ticket.status == "closed" {
        "close"
    } else if old_status == "closed" && ticket.status != "closed" {
        "reopen"
    } else {
        "update"
    };
    let audit_details = serde_json::json!({
        "from": {
            "status": old_status,
            "priority": old_priority,
            "assigned_to": old_assigned_to,
        },
        "to": {
            "status": ticket.status,
            "priority": ticket.priority,
            "assigned_to": ticket.assigned_to,
        }
    })
    .to_string();
    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            action,
            "support_ticket",
            Some(&id),
            Some(audit_details.as_str()),
            None,
        )
        .await;

    // Notifications (best-effort)
    // - Owner gets notified on status/assignment changes.
    // - Assignee gets notified on status changes and when assigned.
    // (UI will prefix tenant slug via resolveActionUrl)
    if status_changed {
        if let Some(owner) = owner_id.clone() {
            if owner != claims.sub {
                let _ = notification_service
                    .create_notification(
                        owner,
                        Some(tenant_id.to_string()),
                        "Ticket status updated".to_string(),
                        format!("{} ({})", ticket.subject, ticket.status),
                        "info".to_string(),
                        "support".to_string(),
                        Some(format!("/support/{}", ticket.id)),
                    )
                    .await;
            }
        }

        if let Some(assignee) = assignee_id.clone() {
            if assignee != claims.sub {
                let _ = notification_service
                    .create_notification(
                        assignee,
                        Some(tenant_id.to_string()),
                        "Ticket status updated".to_string(),
                        format!("{} ({})", ticket.subject, ticket.status),
                        "info".to_string(),
                        "support".to_string(),
                        Some(format!("/admin/support/{}", ticket.id)),
                    )
                    .await;
            }
        }
    }

    if assigned_changed {
        if let Some(new_assignee) = assignee_id.clone() {
            if new_assignee != claims.sub {
                let _ = notification_service
                    .create_notification(
                        new_assignee,
                        Some(tenant_id.to_string()),
                        "Ticket assigned".to_string(),
                        ticket.subject.clone(),
                        "info".to_string(),
                        "support".to_string(),
                        Some(format!("/admin/support/{}", ticket.id)),
                    )
                    .await;
            }
        }

        if let Some(owner) = owner_id {
            if owner != claims.sub {
                let _ = notification_service
                    .create_notification(
                        owner,
                        Some(tenant_id.to_string()),
                        "Ticket updated".to_string(),
                        "Your ticket assignment was updated.".to_string(),
                        "info".to_string(),
                        "support".to_string(),
                        Some(format!("/support/{}", ticket.id)),
                    )
                    .await;
            }
        }
    }

    Ok(ticket)
}

#[cfg(feature = "postgres")]
async fn attach_files_pg(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    message_id: &str,
    file_ids: &[String],
) -> Result<(), sqlx::Error> {
    let now = Utc::now();
    for fid in file_ids {
        let exists: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM file_records WHERE id = $1 AND tenant_id = $2",
        )
        .bind(fid)
        .bind(tenant_id)
        .fetch_one(&mut **tx)
        .await
        .unwrap_or(false);

        if !exists {
            return Err(sqlx::Error::RowNotFound);
        }

        let aid = Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT INTO support_ticket_attachments (id, message_id, file_id, created_at) VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",
        )
        .bind(aid)
        .bind(message_id)
        .bind(fid)
        .bind(now)
        .execute(&mut **tx)
        .await;
    }

    Ok(())
}

#[cfg(feature = "postgres")]
async fn fetch_attachments_map_pg(
    pool: &sqlx::Pool<sqlx::Postgres>,
    tenant_id: &str,
    ticket_id: &str,
    message_ids: &[String],
) -> Result<HashMap<String, Vec<FileRecord>>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        message_id: String,
        id: String,
        tenant_id: String,
        name: String,
        original_name: String,
        path: String,
        size: i64,
        content_type: String,
        storage_provider: String,
        uploaded_by: Option<String>,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
    }

    if message_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT
            a.message_id,
            f.id, f.tenant_id, f.name, f.original_name, f.path, f.size, f.content_type,
            f.storage_provider, f.uploaded_by, f.created_at, f.updated_at
        FROM support_ticket_attachments a
        JOIN support_ticket_messages m ON m.id = a.message_id
        JOIN support_tickets t ON t.id = m.ticket_id
        JOIN file_records f ON f.id = a.file_id
        WHERE t.tenant_id = $1
          AND m.ticket_id = $2
          AND a.message_id = ANY($3)
        ORDER BY a.created_at ASC
    "#,
    )
    .bind(tenant_id)
    .bind(ticket_id)
    .bind(message_ids)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut map: HashMap<String, Vec<FileRecord>> = HashMap::new();
    for r in rows {
        let fr = FileRecord {
            id: r.id,
            tenant_id: r.tenant_id,
            name: r.name,
            original_name: r.original_name,
            path: r.path,
            size: r.size,
            content_type: r.content_type,
            storage_provider: r.storage_provider,
            uploaded_by: r.uploaded_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
        };
        map.entry(r.message_id).or_default().push(fr);
    }

    Ok(map)
}
