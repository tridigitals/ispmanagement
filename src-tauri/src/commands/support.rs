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

use super::announcements_support_common::{
    normalize_category, normalize_priority, normalize_status, support_admin_user_ids,
};

/// Internal field workers (technicians + field staff) see tickets **assigned**
/// to them. Customers see tickets they **created**. Admins/staff see all.
/// Returns true when the role is a field-worker (not a customer/admin).
fn is_field_worker_role(role: &str) -> bool {
    matches!(role, "technician" | "staff")
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
    let admins = support_admin_user_ids(pool, tenant_id)
        .await
        .unwrap_or_default();
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

    let admins = support_admin_user_ids(pool, tenant_id)
        .await
        .unwrap_or_default();
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

    let admins = support_admin_user_ids(pool, tenant_id)
        .await
        .unwrap_or_default();
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

#[tauri::command]
pub async fn list_support_tickets(
    token: String,
    status: Option<String>,
    search: Option<String>,
    category: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    assigned: Option<String>, // "all" | "assigned" | "unassigned"
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
    let category = normalize_category(category);

    let (rows, total): (Vec<SupportTicketListItem>, i64) = if can_all {
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM support_tickets t
            LEFT JOIN users u ON u.id = t.created_by
            WHERE t.tenant_id = $1
              AND ($2::text IS NULL OR t.status = $2)
              AND ($4::text IS NULL OR t.category = $4)
              AND (
                $3::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($3) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($3) || '%'
              )
              AND (
                $7::text IS NULL
                OR ($7::text = 'assigned' AND t.assigned_to IS NOT NULL)
                OR ($7::text = 'unassigned' AND t.assigned_to IS NULL)
              )
        "#,
        )
        .bind(&tenant_id)
        .bind(st.clone())
        .bind(search.clone())
        .bind(category.clone())
        .bind(assigned.clone())
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
              AND ($4::text IS NULL OR t.category = $4)
              AND (
                $3::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($3) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($3) || '%'
              )
              AND (
                $7::text IS NULL
                OR ($7::text = 'assigned' AND t.assigned_to IS NOT NULL)
                OR ($7::text = 'unassigned' AND t.assigned_to IS NULL)
              )
            ORDER BY
              CASE WHEN $7::text = 'unassigned' THEN 0 ELSE 1 END ASC,
              COALESCE((SELECT MAX(created_at) FROM support_ticket_messages m WHERE m.ticket_id = t.id), t.updated_at) DESC
            LIMIT $5 OFFSET $6
        "#,
        )
        .bind(&tenant_id)
        .bind(st)
        .bind(search)
        .bind(category)
        .bind(per_page as i64)
        .bind(offset)
        .bind(assigned)
        .fetch_all(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

        (rows, total)
    } else if is_field_worker_role(&claims.role) {
        // Technician / field staff: show tickets ASSIGNED to them OR unassigned tickets.
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM support_tickets t
            LEFT JOIN users u ON u.id = t.created_by
            WHERE t.tenant_id = $1
              AND ($2::text IS NULL OR t.status = $2)
              AND ($4::text IS NULL OR t.category = $4)
              AND (
                $3::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($3) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($3) || '%'
              )
              AND (t.assigned_to = $5 OR t.assigned_to IS NULL)
        "#,
        )
        .bind(&tenant_id)
        .bind(st.clone())
        .bind(search.clone())
        .bind(category.clone())
        .bind(&claims.sub)
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
              AND ($4::text IS NULL OR t.category = $4)
              AND (
                $3::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($3) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($3) || '%'
              )
              AND (t.assigned_to = $7 OR t.assigned_to IS NULL)
            ORDER BY
              CASE
                WHEN t.assigned_to IS NULL THEN 0
                ELSE 1
              END ASC,
              COALESCE((SELECT MAX(created_at) FROM support_ticket_messages m WHERE m.ticket_id = t.id), t.updated_at) DESC
            LIMIT $5 OFFSET $6
        "#,
        )
        .bind(&tenant_id)
        .bind(st)
        .bind(search)
        .bind(category)
        .bind(per_page as i64)
        .bind(offset)
        .bind(&claims.sub)
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
              AND ($2::text IS NULL OR t.status = $2)
              AND ($4::text IS NULL OR t.category = $4)
              AND (
                $3::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($3) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($3) || '%'
              )
              AND t.created_by = $5
        "#,
        )
        .bind(&tenant_id)
        .bind(st.clone())
        .bind(search.clone())
        .bind(category.clone())
        .bind(&claims.sub)
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
              AND ($4::text IS NULL OR t.category = $4)
              AND (
                $3::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($3) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($3) || '%'
              )
              AND t.created_by = $7
            ORDER BY COALESCE((SELECT MAX(created_at) FROM support_ticket_messages m WHERE m.ticket_id = t.id), t.updated_at) DESC
            LIMIT $5 OFFSET $6
        "#,
        )
        .bind(&tenant_id)
        .bind(st)
        .bind(search)
        .bind(category)
        .bind(per_page as i64)
        .bind(offset)
        .bind(&claims.sub)
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
    } else if is_field_worker_role(&claims.role) {
        // Technician / staff: stats over tickets ASSIGNED to them OR unassigned tickets.
        sqlx::query_as(
            r#"
            SELECT
              COUNT(*) AS all,
              COALESCE(SUM(CASE WHEN status = 'open' THEN 1 ELSE 0 END), 0) AS open,
              COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0) AS pending,
              COALESCE(SUM(CASE WHEN status = 'closed' THEN 1 ELSE 0 END), 0) AS closed
            FROM support_tickets
            WHERE tenant_id = $1 AND (assigned_to = $2 OR assigned_to IS NULL)
        "#,
        )
        .bind(&tenant_id)
        .bind(&claims.sub)
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
    category: Option<String>,
    subscription_id: Option<String>,
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
    let category = normalize_category(category);

    let mut tx = auth_service.pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query(
        r#"
        INSERT INTO support_tickets (
            id, tenant_id, created_by, subject, status, priority, category, subscription_id,
            assigned_to, created_at, updated_at, closed_at
        )
        VALUES ($1,$2,$3,$4,'open',$5,$6,$7,NULL,$8,$9,NULL)
    "#,
    )
    .bind(&ticket_id)
    .bind(&tenant_id)
    .bind(&claims.sub)
    .bind(subject.trim())
    .bind(&priority)
    .bind(&category)
    .bind(&subscription_id)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    // Resolve the creator's display name at create time so the first
    // message (which IS the ticket body) carries it for the UI.
    let creator_name: Option<String> = sqlx::query_scalar(
        "SELECT name FROM users WHERE id = $1",
    )
    .bind(&claims.sub)
    .fetch_optional(&mut *tx)
    .await
    .ok()
    .flatten()
    .filter(|s: &String| !s.trim().is_empty());

    sqlx::query(
        r#"
        INSERT INTO support_ticket_messages (id, ticket_id, author_id, author_name, body, is_internal, created_at)
        VALUES ($1,$2,$3,$4,$5,false,$6)
    "#,
    )
    .bind(&msg_id)
    .bind(&ticket_id)
    .bind(&claims.sub)
    .bind(creator_name.as_deref())
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
            author_name: m.author_name,
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

    // Non-admins can see tickets they created OR (for field workers) tickets
    // assigned to them. Technicians don't create tickets — admins assign to them.
    let is_creator = ticket.created_by.as_deref() == Some(claims.sub.as_str());
    let is_assignee = ticket.assigned_to.as_deref() == Some(claims.sub.as_str());
    if !can_all && !is_creator && !is_assignee {
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
            author_name: m.author_name,
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

    // Non-admins can reply on tickets they created OR (for field workers) tickets
    // assigned to them.
    let is_creator = ticket.created_by.as_deref() == Some(claims.sub.as_str());
    let is_assignee = ticket.assigned_to.as_deref() == Some(claims.sub.as_str());
    if !can_all && !is_creator && !is_assignee {
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

    // Resolve the author's display name at reply time so historical
    // messages keep their sender label even if the user renames/deletes.
    let author_name: Option<String> = sqlx::query_scalar(
        "SELECT name FROM users WHERE id = $1",
    )
    .bind(&claims.sub)
    .fetch_optional(&mut *tx)
    .await
    .ok()
    .flatten()
    .filter(|s: &String| !s.trim().is_empty());

    sqlx::query(
        r#"
        INSERT INTO support_ticket_messages (id, ticket_id, author_id, author_name, body, is_internal, created_at)
        VALUES ($1,$2,$3,$4,$5,$6,$7)
    "#,
    )
    .bind(&msg_id)
    .bind(&id)
    .bind(&claims.sub)
    .bind(author_name.as_deref())
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
        author_name: msg.author_name,
        body: msg.body,
        is_internal: msg.is_internal,
        created_at: msg.created_at,
        attachments: att_map.get(&msg.id).cloned().unwrap_or_default(),
    })
}

/// List all messages on a support ticket.
/// Same authorization as get_support_ticket: admin OR creator OR assignee.
/// Internal notes are hidden from non-admin field workers / customers.
#[tauri::command]
pub async fn list_support_ticket_messages(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
) -> Result<Vec<SupportTicketMessage>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "support", "read")
        .await
        .map_err(|e| e.to_string())?;

    // Verify the user can see this ticket (admin OR creator OR assignee).
    let ticket: SupportTicket =
        sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1 AND tenant_id = $2")
            .bind(&id)
            .bind(&tenant_id)
            .fetch_one(&auth_service.pool)
            .await
            .map_err(|e| e.to_string())?;

    let can_all = auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);

    let is_creator = ticket.created_by.as_deref() == Some(claims.sub.as_str());
    let is_assignee = ticket.assigned_to.as_deref() == Some(claims.sub.as_str());
    if !can_all && !is_creator && !is_assignee {
        return Err("Forbidden".to_string());
    }

    // Internal notes only visible to those with `support:internal` permission.
    let can_internal = auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "internal")
        .await
        .unwrap_or(false);

    let messages: Vec<SupportTicketMessage> = if can_internal {
        sqlx::query_as(
            r#"
            SELECT id, ticket_id, author_id, author_name, body, is_internal, created_at
            FROM support_ticket_messages
            WHERE ticket_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(&id)
        .fetch_all(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query_as(
            r#"
            SELECT id, ticket_id, author_id, author_name, body, is_internal, created_at
            FROM support_ticket_messages
            WHERE ticket_id = $1 AND is_internal = false
            ORDER BY created_at ASC
            "#,
        )
        .bind(&id)
        .fetch_all(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?
    };

    Ok(messages)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_support_ticket(
    token: String,
    id: String,
    status: Option<String>,
    priority: Option<String>,
    category: Option<String>,
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
    let new_category = category.or(existing.category);
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
            category = $4,
            updated_at = $5,
            closed_at = $6
        WHERE id = $7 AND tenant_id = $8
        RETURNING *
    "#,
    )
    .bind(new_status)
    .bind(new_priority)
    .bind(assigned_to)
    .bind(new_category)
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
    if file_ids.is_empty() {
        return Ok(());
    }

    let expected_count = file_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>()
        .len();
    let existing_ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM file_records WHERE tenant_id = $1 AND id = ANY($2)")
            .bind(tenant_id)
            .bind(file_ids)
            .fetch_all(&mut **tx)
            .await?;
    if existing_ids.len() != expected_count {
        return Err(sqlx::Error::RowNotFound);
    }

    let now = Utc::now();
    use sqlx::{Postgres, QueryBuilder};
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO support_ticket_attachments (id, message_id, file_id, created_at) ",
    );
    qb.push_values(file_ids, |mut b, fid| {
        b.push_bind(Uuid::new_v4().to_string())
            .push_bind(message_id)
            .push_bind(fid)
            .push_bind(now);
    });
    qb.push(" ON CONFLICT DO NOTHING");
    qb.build().execute(&mut **tx).await?;

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

/// Submit a satisfaction rating for a closed ticket.
/// Customer-only — the ticket must be closed and belong to the caller.
#[tauri::command]
pub async fn submit_ticket_satisfaction(
    token: String,
    ticket_id: String,
    rating: i32,
    comment: Option<String>,
    auth_service: State<'_, AuthService>,
) -> Result<(), String> {
    if rating < 1 || rating > 5 {
        return Err("Rating must be between 1 and 5".to_string());
    }

    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims.tenant_id.as_deref().ok_or("Tenant context required")?;
    let customer_id: String = sqlx::query_scalar(
        "SELECT customer_id FROM customer_users WHERE tenant_id = $1 AND user_id = $2 LIMIT 1",
    )
    .bind(tenant_id)
    .bind(&claims.sub)
    .fetch_optional(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Customer context required".to_string())?;

    // Verify ticket exists, is closed, and belongs to this customer
    let ticket = sqlx::query_scalar::<_, String>(
        "SELECT id FROM support_tickets WHERE id = $1 AND created_by = $2 AND status = 'closed'",
    )
    .bind(&ticket_id)
    .bind(&customer_id)
    .fetch_optional(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    if ticket.is_none() {
        return Err("Ticket not found or not eligible for rating".to_string());
    }

    sqlx::query(
        "UPDATE support_tickets SET satisfaction_rating = $1, satisfaction_comment = $2, updated_at = NOW() WHERE id = $3",
    )
    .bind(rating)
    .bind(comment.as_deref())
    .bind(&ticket_id)
    .execute(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

// =============================================================================
// Sprint 3: Ticket action commands (start / resolve / upload proof)
// =============================================================================

/// Mark a ticket as in_progress. Only the assigned technician/staff/admin can
/// start work. Sets `started_at` to now() and status to `in_progress`.
#[tauri::command]
pub async fn start_support_ticket(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
    notification_service: State<'_, NotificationService>,
    ws_hub: State<'_, std::sync::Arc<WsHub>>,
) -> Result<SupportTicket, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    let ticket: SupportTicket =
        sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1 AND tenant_id = $2")
            .bind(&id)
            .bind(&tenant_id)
            .fetch_one(&auth_service.pool)
            .await
            .map_err(|e| e.to_string())?;

    // Only assignee or admin can start work.
    let can_all = auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);
    let is_assignee = ticket.assigned_to.as_deref() == Some(claims.sub.as_str());
    if !can_all && !is_assignee {
        return Err("Forbidden: only the assignee can start this ticket".to_string());
    }

    if ticket.status == "closed" || ticket.status == "resolved" {
        return Err(format!("Cannot start a {} ticket", ticket.status));
    }

    let now = Utc::now();
    let updated: SupportTicket = sqlx::query_as(
        r#"
        UPDATE support_tickets
        SET status = 'in_progress',
            started_at = $1,
            updated_at = $1
        WHERE id = $2 AND tenant_id = $3
        RETURNING *
        "#,
    )
    .bind(now)
    .bind(&id)
    .bind(&tenant_id)
    .fetch_one(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Notify ticket owner that work has started.
    if let Some(ref owner) = updated.created_by {
        if owner != &claims.sub {
            let subject = format!("Teknisi mulai mengerjakan tiket: {}", updated.subject);
            let body = format!(
                "Teknisi telah mulai mengerjakan tiket Anda ({}). Pantau progresnya di aplikasi.",
                updated.id
            );
            let _ = notification_service
                .create_notification(
                    owner.clone(),
                    Some(tenant_id.clone()),
                    subject,
                    body,
                    "info".to_string(),
                    "support".to_string(),
                    Some(format!("/support/tickets/{}", updated.id)),
                )
                .await;
        }
    }

    ws_hub.broadcast(WsEvent::SupportTicketUpdated {
        ticket_id: updated.id.clone(),
        status: updated.status.clone(),
        actor_id: claims.sub.clone(),
    });

    Ok(updated)
}

/// Resolve a ticket with completion notes + optional photo IDs + optional signature.
/// Sets status='resolved', resolved_at=now, completion_notes, signature_url,
/// and stores the list of attached photo file_record IDs in completion_photos.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn resolve_support_ticket(
    token: String,
    id: String,
    completion_notes: Option<String>,
    signature_file_id: Option<String>,
    photo_file_ids: Option<Vec<String>>,
    auth_service: State<'_, AuthService>,
    notification_service: State<'_, NotificationService>,
    audit_service: State<'_, AuditService>,
    ws_hub: State<'_, std::sync::Arc<WsHub>>,
) -> Result<SupportTicket, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

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
    if ticket.status == "resolved" {
        return Err("Ticket already resolved".to_string());
    }

    let can_all = auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);
    let is_assignee = ticket.assigned_to.as_deref() == Some(claims.sub.as_str());
    if !can_all && !is_assignee {
        return Err("Forbidden: only the assignee can resolve this ticket".to_string());
    }

    let photos_json = serde_json::to_value(photo_file_ids.unwrap_or_default())
        .map_err(|e| format!("photo list serialization failed: {e}"))?;

    let now = Utc::now();
    let updated: SupportTicket = sqlx::query_as(
        r#"
        UPDATE support_tickets
        SET status           = 'resolved',
            resolved_at      = $1,
            updated_at       = $1,
            completion_notes = $2,
            signature_url    = $3,
            completion_photos = $4,
            started_at       = COALESCE(started_at, $1)
        WHERE id = $5 AND tenant_id = $6
        RETURNING *
        "#,
    )
    .bind(now)
    .bind(completion_notes.as_deref())
    .bind(signature_file_id.as_deref())
    .bind(photos_json)
    .bind(&id)
    .bind(&tenant_id)
    .fetch_one(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Audit log.
    let audit_details = serde_json::json!({
        "action": "resolve",
        "completion_notes": completion_notes,
        "photo_count": updated.completion_photos.len(),
        "has_signature": signature_file_id.is_some(),
    })
    .to_string();
    audit_service
        .log(
            Some(&claims.sub),
            Some(&tenant_id),
            "resolve",
            "support_ticket",
            Some(&updated.id),
            Some(&audit_details),
            None,
        )
        .await;

    // Notify ticket owner that the ticket was resolved.
    if let Some(ref owner) = updated.created_by {
        if owner != &claims.sub {
            let subject = format!("Tiket Anda telah selesai: {}", updated.subject);
            let body = format!(
                "Teknisi telah menyelesaikan tiket ({}). Silakan cek aplikasi untuk konfirmasi.",
                updated.id
            );
            let _ = notification_service
                .create_notification(
                    owner.clone(),
                    Some(tenant_id.clone()),
                    subject,
                    body,
                    "info".to_string(),
                    "support".to_string(),
                    Some(format!("/support/tickets/{}", updated.id)),
                )
                .await;
        }
    }

    ws_hub.broadcast(WsEvent::SupportTicketUpdated {
        ticket_id: updated.id.clone(),
        status: updated.status.clone(),
        actor_id: claims.sub.clone(),
    });

    Ok(updated)
}

/// List team members eligible for ticket assignment.
/// Returns users with support permission OR owner/admin/technician/noc/planner/staff roles.
/// Excludes customers.
#[tauri::command]
pub async fn list_support_assignees(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<Vec<crate::models::TeamMemberWithUser>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .map_err(|e| e.to_string())?;

    #[cfg(feature = "postgres")]
    let eligible: Vec<crate::models::TeamMemberWithUser> = sqlx::query_as(
        r#"
        SELECT
          tm.id, tm.user_id, u.name, u.email,
          tm.role, tm.role_id, r.name AS role_name,
          u.is_active, tm.created_at, r.level AS role_level
        FROM tenant_members tm
        JOIN users u ON tm.user_id = u.id
        LEFT JOIN roles r ON tm.role_id = r.id
        WHERE tm.tenant_id = $1
          AND u.is_active = TRUE
          AND LOWER(COALESCE(r.name, tm.role, '')) NOT IN ('customer', 'pelanggan')
          AND (
            EXISTS(
              SELECT 1 FROM role_permissions rp
              JOIN permissions p ON p.id = rp.permission_id
              WHERE rp.role_id = tm.role_id AND p.resource = 'support'
            )
            OR LOWER(COALESCE(r.name, tm.role, '')) IN ('owner','admin','noc','planner','staff','technician','teknisi')
          )
        ORDER BY LOWER(u.name), LOWER(u.email)
        "#,
    )
    .bind(&tenant_id)
    .fetch_all(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    #[cfg(not(feature = "postgres"))]
    let eligible: Vec<crate::models::TeamMemberWithUser> = sqlx::query_as(
        r#"
        SELECT
          tm.id, tm.user_id, u.name, u.email,
          tm.role, tm.role_id, r.name AS role_name,
          u.is_active, tm.created_at, r.level AS role_level
        FROM tenant_members tm
        JOIN users u ON tm.user_id = u.id
        LEFT JOIN roles r ON tm.role_id = r.id
        WHERE tm.tenant_id = ?
          AND u.is_active = 1
          AND LOWER(COALESCE(r.name, tm.role, '')) NOT IN ('customer', 'pelanggan')
          AND (
            EXISTS(
              SELECT 1 FROM role_permissions rp
              JOIN permissions p ON p.id = rp.permission_id
              WHERE rp.role_id = tm.role_id AND p.resource = 'support'
            )
            OR LOWER(COALESCE(r.name, tm.role, '')) IN ('owner','admin','noc','planner','staff','technician','teknisi')
          )
        ORDER BY LOWER(u.name), LOWER(u.email)
        "#,
    )
    .bind(&tenant_id)
    .fetch_all(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(eligible)
}

#[cfg(test)]
mod tests {
    use super::{normalize_priority, normalize_status};

    #[test]
    fn normalize_priority_preserves_allowed_values_and_defaults_invalid() {
        assert_eq!(normalize_priority(Some("low".to_string())), "low");
        assert_eq!(normalize_priority(Some("urgent".to_string())), "urgent");
        assert_eq!(normalize_priority(Some("invalid".to_string())), "normal");
        assert_eq!(normalize_priority(None), "normal");
    }

    #[test]
    fn normalize_status_accepts_known_and_rejects_unknown() {
        assert_eq!(
            normalize_status(Some("open".to_string())),
            Some("open".to_string())
        );
        assert_eq!(
            normalize_status(Some("pending".to_string())),
            Some("pending".to_string())
        );
        assert_eq!(normalize_status(Some("other".to_string())), None);
        assert_eq!(normalize_status(None), None);
    }

    #[cfg(feature = "postgres")]
    #[tokio::test]
    async fn fetch_attachments_map_pg_empty_message_ids_returns_empty_without_db_hit() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@127.0.0.1/test_db")
            .expect("lazy postgres pool should be constructible");

        let map = super::fetch_attachments_map_pg(&pool, "tenant-1", "ticket-1", &[])
            .await
            .expect("empty message ids should short-circuit");

        assert!(map.is_empty());
    }
}
