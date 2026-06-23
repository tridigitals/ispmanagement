use super::AppState;
use crate::models::{
    CreateSupportTicketDto, FileRecord, PaginatedResponse, ReplySupportTicketDto,
    SupportTicket, SupportTicketDetail, SupportTicketListItem, SupportTicketMessage,
    SupportTicketMessageWithAttachments, SatisfactionDto, TeamMemberWithUser,
    UpdateSupportTicketDto,
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use uuid::Uuid;

use super::announcements_support_common::{
    normalize_priority, normalize_priority_optional_lowercase, normalize_status,
    support_admin_user_ids,
};

/// Internal field workers (technicians + field staff) see tickets **assigned**
/// to them. Customers see tickets they **created**. Admins/staff see all.
fn is_field_worker_role(role: &str) -> bool {
    matches!(role, "technician" | "staff")
}

#[cfg(feature = "postgres")]
async fn notify_support_admins_new_ticket(
    state: &AppState,
    tenant_id: &str,
    ticket_id: &str,
    created_by: &str,
    subject: &str,
) {
    let admins = support_admin_user_ids(&state.auth_service.pool, tenant_id)
        .await
        .unwrap_or_default();

    let creator_name: Option<String> = sqlx::query_scalar("SELECT name FROM users WHERE id = $1")
        .bind(created_by)
        .fetch_optional(&state.auth_service.pool)
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
        let _ = state
            .notification_service
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
    state: &AppState,
    tenant_id: &str,
    ticket: &SupportTicket,
    author_id: &str,
    is_internal: bool,
) {
    if is_internal {
        // Internal notes are staff-only; don't notify the customer.
        return;
    }

    let is_staff = state
        .auth_service
        .has_permission(author_id, tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);

    // We notify both "sides" so the bell updates in both user and admin areas:
    // - ticket owner gets a /support/{id} action
    // - support admins get an /admin/support/{id} action
    // Dedupe recipients so an owner who is also staff doesn't get two entries.
    let mut sent: HashSet<String> = HashSet::new();

    if let Some(owner) = ticket.created_by.clone() {
        // Ticket owner gets an in-app notification, unless they are the author.
        // Don't `return` here: we still want to notify support admins.
        sent.insert(owner.clone());
        if owner != author_id {
            let title = if is_staff {
                "Support reply"
            } else {
                "Ticket updated"
            };
            let _ = state
                .notification_service
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

    let admins = support_admin_user_ids(&state.auth_service.pool, tenant_id)
        .await
        .unwrap_or_default();

    for uid in admins {
        if uid == author_id {
            continue;
        }
        if sent.contains(&uid) {
            continue; // owner already got a notification
        }
        sent.insert(uid.clone());

        let title = if is_staff {
            "Staff replied"
        } else {
            "Customer replied"
        };
        let _ = state
            .notification_service
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
    state: &AppState,
    tenant_id: &str,
    ticket: &SupportTicket,
    author_id: &str,
    is_internal: bool,
    message_id: &str,
) {
    let mut recipients: HashSet<String> = HashSet::new();

    // Customer should see staff replies, staff should see customer replies (and internal notes are staff-only).
    // We mirror the same "sides" logic as notifications, but send a light WS event so pages can refresh.
    if !is_internal {
        if let Some(owner) = ticket.created_by.clone() {
            if owner != author_id {
                recipients.insert(owner);
            }
        }
    }

    let admins = support_admin_user_ids(&state.auth_service.pool, tenant_id)
        .await
        .unwrap_or_default();
    for uid in admins {
        if uid == author_id {
            continue;
        }
        recipients.insert(uid);
    }

    for uid in recipients {
        state
            .ws_hub
            .broadcast(crate::http::WsEvent::SupportTicketMessageCreated {
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

#[derive(Deserialize)]
pub struct ListParams {
    pub status: Option<String>,
    pub category: Option<String>,
    pub search: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub assigned: Option<String>, // "all" | "assigned" | "unassigned"
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

pub async fn list_support_tickets(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ListParams>,
) -> Result<Json<PaginatedResponse<SupportTicketListItem>>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    let can_all = state
        .auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);

    if !can_all {
        state
            .auth_service
            .check_permission(&claims.sub, &tenant_id, "support", "read")
            .await?;
    }

    let st = normalize_status(params.status);

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page.saturating_sub(1) * per_page) as i64;

    let search = params
        .search
        .as_deref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let category = params
        .category
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
              AND ($3::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($3) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($3) || '%')
              AND ($4::text IS NULL OR t.category = $4)
              AND (
                $5::text IS NULL
                OR ($5::text = 'assigned' AND t.assigned_to IS NOT NULL)
                OR ($5::text = 'unassigned' AND t.assigned_to IS NULL)
              )
        "#,
        )
        .bind(&tenant_id)
        .bind(st.clone())
        .bind(search.clone())
        .bind(category.clone())
        .bind(params.assigned.clone())
        .fetch_one(&state.auth_service.pool)
        .await?;

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
              AND ($3::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($3) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($3) || '%')
              AND ($4::text IS NULL OR t.category = $4)
              AND (
                $5::text IS NULL
                OR ($5::text = 'assigned' AND t.assigned_to IS NOT NULL)
                OR ($5::text = 'unassigned' AND t.assigned_to IS NULL)
              )
            ORDER BY
              CASE WHEN $5::text = 'unassigned' THEN 0 ELSE 1 END ASC,
              COALESCE((SELECT MAX(created_at) FROM support_ticket_messages m WHERE m.ticket_id = t.id), t.updated_at) DESC
            LIMIT $6 OFFSET $7
        "#,
        )
        .bind(&tenant_id)
        .bind(st)
        .bind(search)
        .bind(category)
        .bind(params.assigned.clone())
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&state.auth_service.pool)
        .await?;

        (rows, total)
    } else if is_field_worker_role(&claims.role) {
        // Technician / field staff: show tickets ASSIGNED to them OR unassigned tickets.
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM support_tickets t
            LEFT JOIN users u ON u.id = t.created_by
            WHERE t.tenant_id = $1
              AND (t.assigned_to = $2 OR t.assigned_to IS NULL)
              AND ($3::text IS NULL OR t.status = $3)
              AND ($4::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($4) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($4) || '%')
              AND ($5::text IS NULL OR t.category = $5)
        "#,
        )
        .bind(&tenant_id)
        .bind(&claims.sub)
        .bind(st.clone())
        .bind(search.clone())
        .bind(category.clone())
        .fetch_one(&state.auth_service.pool)
        .await?;

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
              AND (t.assigned_to = $2 OR t.assigned_to IS NULL)
              AND ($3::text IS NULL OR t.status = $3)
              AND ($4::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($4) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($4) || '%')
              AND ($5::text IS NULL OR t.category = $5)
            ORDER BY
              CASE
                WHEN t.assigned_to IS NULL THEN 0
                ELSE 1
              END ASC,
              COALESCE((SELECT MAX(created_at) FROM support_ticket_messages m WHERE m.ticket_id = t.id), t.updated_at) DESC
            LIMIT $6 OFFSET $7
        "#,
        )
        .bind(&tenant_id)
        .bind(&claims.sub)
        .bind(st.clone())
        .bind(search.clone())
        .bind(category.clone())
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&state.auth_service.pool)
        .await?;

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
              AND ($4::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($4) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($4) || '%')
              AND ($5::text IS NULL OR t.category = $5)
        "#,
        )
        .bind(&tenant_id)
        .bind(&claims.sub)
        .bind(st.clone())
        .bind(search.clone())
        .bind(category.clone())
        .fetch_one(&state.auth_service.pool)
        .await?;

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
              AND ($4::text IS NULL
                OR LOWER(t.subject) LIKE '%' || LOWER($4) || '%'
                OR LOWER(COALESCE(u.name, '')) LIKE '%' || LOWER($4) || '%')
              AND ($5::text IS NULL OR t.category = $5)
            ORDER BY COALESCE((SELECT MAX(created_at) FROM support_ticket_messages m WHERE m.ticket_id = t.id), t.updated_at) DESC
            LIMIT $6 OFFSET $7
        "#,
        )
        .bind(&tenant_id)
        .bind(&claims.sub)
        .bind(st)
        .bind(search)
        .bind(category)
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&state.auth_service.pool)
        .await?;

        (rows, total)
    };

    Ok(Json(PaginatedResponse {
        data: rows,
        total,
        page,
        per_page,
    }))
}

pub async fn get_support_ticket_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SupportTicketStats>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    let can_all = state
        .auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);

    if !can_all {
        state
            .auth_service
            .check_permission(&claims.sub, &tenant_id, "support", "read")
            .await?;
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
        .fetch_one(&state.auth_service.pool)
        .await?
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
        .fetch_one(&state.auth_service.pool)
        .await?
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
        .fetch_one(&state.auth_service.pool)
        .await?
    };

    Ok(Json(SupportTicketStats {
        all: row.all,
        open: row.open,
        pending: row.pending,
        closed: row.closed,
    }))
}

pub async fn create_support_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<CreateSupportTicketDto>,
) -> Result<Json<SupportTicketDetail>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "support", "create")
        .await?;

    if dto.subject.trim().is_empty() || dto.message.trim().is_empty() {
        return Err(crate::error::AppError::Validation(
            "Subject and message are required".to_string(),
        ));
    }

    let now = Utc::now();
    let ticket_id = Uuid::new_v4().to_string();
    let msg_id = Uuid::new_v4().to_string();
    let priority = normalize_priority(dto.priority);

    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO support_tickets (
            id, tenant_id, created_by, subject, status, priority, assigned_to,
            category, subscription_id, created_at, updated_at, closed_at
        )
        VALUES ($1,$2,$3,$4,'open',$5,NULL,$6,$7,$8,$9,NULL)
    "#,
    )
    .bind(&ticket_id)
    .bind(&tenant_id)
    .bind(&claims.sub)
    .bind(dto.subject.trim())
    .bind(&priority)
    .bind(dto.category.as_deref())
    .bind(dto.subscription_id.as_deref())
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // Look up the creator's display name once so the very first message
    // (which IS the ticket body) carries it for the UI to render.
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
    .bind(dto.message.trim())
    .bind(now)
    .execute(&mut *tx)
    .await?;

    #[cfg(feature = "postgres")]
    if let Some(file_ids) = dto.attachment_ids.as_ref().filter(|v| !v.is_empty()) {
        attach_files_pg(&mut tx, &tenant_id, &msg_id, file_ids).await?;
    }

    let ticket: SupportTicket = sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1")
        .bind(&ticket_id)
        .fetch_one(&mut *tx)
        .await?;

    let messages: Vec<SupportTicketMessage> = sqlx::query_as(
        "SELECT * FROM support_ticket_messages WHERE ticket_id = $1 ORDER BY created_at ASC",
    )
    .bind(&ticket_id)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    // Audit (best-effort; does not fail request on error)
    let audit_details = serde_json::json!({
        "subject": ticket.subject,
        "priority": ticket.priority,
        "message_id": msg_id,
        "attachments": dto.attachment_ids.as_ref().map(|v| v.len()).unwrap_or(0),
    })
    .to_string();
    state
        .audit_service
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
    notify_support_admins_new_ticket(&state, &tenant_id, &ticket_id, &claims.sub, &ticket.subject)
        .await;

    // Always notify creator as well (useful for bell history and single-user tenants).
    let _ = state
        .notification_service
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
            fetch_attachments_map_pg(
                &state.auth_service.pool,
                &tenant_id,
                &ticket_id,
                &message_ids,
            )
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

    Ok(Json(SupportTicketDetail { ticket, messages }))
}

pub async fn get_support_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<SupportTicketDetail>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    let can_all = state
        .auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);

    if !can_all {
        state
            .auth_service
            .check_permission(&claims.sub, &tenant_id, "support", "read")
            .await?;
    }

    let ticket: SupportTicket =
        sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1 AND tenant_id = $2")
            .bind(&id)
            .bind(&tenant_id)
            .fetch_one(&state.auth_service.pool)
            .await?;

    if !can_all && ticket.created_by.as_deref() != Some(claims.sub.as_str()) {
        return Err(crate::error::AppError::Forbidden("Forbidden".to_string()));
    }

    let can_internal = state
        .auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "internal")
        .await
        .unwrap_or(false);

    let messages: Vec<SupportTicketMessage> = if can_internal {
        sqlx::query_as(
            "SELECT * FROM support_ticket_messages WHERE ticket_id = $1 ORDER BY created_at ASC",
        )
        .bind(&id)
        .fetch_all(&state.auth_service.pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT * FROM support_ticket_messages WHERE ticket_id = $1 AND is_internal = false ORDER BY created_at ASC",
        )
        .bind(&id)
        .fetch_all(&state.auth_service.pool)
        .await?
    };

    let message_ids: Vec<String> = messages.iter().map(|m| m.id.clone()).collect();
    let att_map: HashMap<String, Vec<FileRecord>> = {
        #[cfg(feature = "postgres")]
        {
            fetch_attachments_map_pg(&state.auth_service.pool, &tenant_id, &id, &message_ids)
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

    Ok(Json(SupportTicketDetail { ticket, messages }))
}

pub async fn reply_support_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(dto): Json<ReplySupportTicketDto>,
) -> Result<Json<SupportTicketMessageWithAttachments>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "support", "reply")
        .await?;

    if dto.message.trim().is_empty() {
        return Err(crate::error::AppError::Validation(
            "Message is required".to_string(),
        ));
    }

    let ticket: SupportTicket =
        sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1 AND tenant_id = $2")
            .bind(&id)
            .bind(&tenant_id)
            .fetch_one(&state.auth_service.pool)
            .await?;

    if ticket.status == "closed" {
        return Err(crate::error::AppError::Validation(
            "Ticket is closed".to_string(),
        ));
    }

    let can_all = state
        .auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);

    if !can_all && ticket.created_by.as_deref() != Some(claims.sub.as_str()) {
        return Err(crate::error::AppError::Forbidden("Forbidden".to_string()));
    }

    let is_internal = dto.is_internal.unwrap_or(false);
    if is_internal {
        state
            .auth_service
            .check_permission(&claims.sub, &tenant_id, "support", "internal")
            .await?;
    }

    let now = Utc::now();
    let msg_id = Uuid::new_v4().to_string();

    let mut tx = state.auth_service.pool.begin().await?;
    state
        .auth_service
        .apply_rls_context_tx(&mut tx, &claims)
        .await?;

    // Look up the author's display name at reply time. We snapshot the
    // name into the row so historical messages keep their sender label
    // even if the user later renames or is deleted.
    let author_name: Option<String> = sqlx::query_scalar(
        "SELECT name FROM users WHERE id = $1",
    )
    .bind(&claims.sub)
    .fetch_optional(&state.auth_service.pool)
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
    .bind(dto.message.trim())
    .bind(is_internal)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    #[cfg(feature = "postgres")]
    if let Some(file_ids) = dto.attachment_ids.as_ref().filter(|v| !v.is_empty()) {
        attach_files_pg(&mut tx, &tenant_id, &msg_id, file_ids).await?;
    }

    sqlx::query("UPDATE support_tickets SET updated_at = $1 WHERE id = $2")
        .bind(now)
        .bind(&id)
        .execute(&mut *tx)
        .await?;

    let msg: SupportTicketMessage =
        sqlx::query_as("SELECT * FROM support_ticket_messages WHERE id = $1")
            .bind(&msg_id)
            .fetch_one(&mut *tx)
            .await?;

    tx.commit().await?;

    // Audit (best-effort)
    let audit_details = serde_json::json!({
        "message_id": msg_id,
        "internal": is_internal,
        "attachments": dto.attachment_ids.as_ref().map(|v| v.len()).unwrap_or(0),
    })
    .to_string();
    state
        .audit_service
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
    notify_support_ticket_reply(&state, &tenant_id, &ticket, &claims.sub, is_internal).await;

    #[cfg(feature = "postgres")]
    broadcast_support_ticket_message_created(
        &state,
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
                &state.auth_service.pool,
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

    Ok(Json(SupportTicketMessageWithAttachments {
        id: msg.id.clone(),
        ticket_id: msg.ticket_id,
        author_id: msg.author_id,
        author_name: msg.author_name,
        body: msg.body,
        is_internal: msg.is_internal,
        created_at: msg.created_at,
        attachments: att_map.get(&msg.id).cloned().unwrap_or_default(),
    }))
}

/// List all messages on a support ticket.
/// Same authorization as get_support_ticket: admin OR creator OR assignee.
/// Internal notes hidden from non-admin field workers / customers.
pub async fn list_support_ticket_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Vec<SupportTicketMessageWithAttachments>>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "support", "read")
        .await?;

    let ticket: SupportTicket =
        sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1 AND tenant_id = $2")
            .bind(&id)
            .bind(&tenant_id)
            .fetch_one(&state.auth_service.pool)
            .await?;

    let can_all = state
        .auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);

    let is_creator = ticket.created_by.as_deref() == Some(claims.sub.as_str());
    let is_assignee = ticket.assigned_to.as_deref() == Some(claims.sub.as_str());
    if !can_all && !is_creator && !is_assignee {
        return Err(crate::error::AppError::Forbidden(
            "Ticket is not assigned to you".to_string(),
        ));
    }

    let can_internal = state
        .auth_service
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
        .fetch_all(&state.auth_service.pool)
        .await?
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
        .fetch_all(&state.auth_service.pool)
        .await?
    };

    // Fetch attachments for these messages (same pattern as get_support_ticket).
    let message_ids: Vec<String> = messages.iter().map(|m| m.id.clone()).collect();
    let att_map: HashMap<String, Vec<FileRecord>> = {
        #[cfg(feature = "postgres")]
        {
            fetch_attachments_map_pg(&state.auth_service.pool, &tenant_id, &id, &message_ids)
                .await
                .unwrap_or_default()
        }
        #[cfg(not(feature = "postgres"))]
        {
            HashMap::new()
        }
    };

    let messages_with_attachments: Vec<SupportTicketMessageWithAttachments> = messages
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

    Ok(Json(messages_with_attachments))
}

pub async fn update_support_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(dto): Json<UpdateSupportTicketDto>,
) -> Result<Json<SupportTicket>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await?;

    let now = Utc::now();
    let status = normalize_status(dto.status);
    let priority = normalize_priority_optional_lowercase(dto.priority);

    if status.is_some() || priority.is_some() {
        state
            .auth_service
            .check_permission(&claims.sub, &tenant_id, "support", "update")
            .await?;
    }

    if dto.assigned_to.is_some() {
        state
            .auth_service
            .check_permission(&claims.sub, &tenant_id, "support", "assign")
            .await?;
    }

    let existing: SupportTicket =
        sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1 AND tenant_id = $2")
            .bind(&id)
            .bind(&tenant_id)
            .fetch_one(&state.auth_service.pool)
            .await?;

    let old_status = existing.status.clone();
    let old_priority = existing.priority.clone();
    let old_assigned_to = existing.assigned_to.clone();

    let new_status = status.unwrap_or(existing.status);
    let new_priority = priority.unwrap_or(existing.priority);
    let assigned_to = dto.assigned_to.or(existing.assigned_to);
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
    .fetch_one(&state.auth_service.pool)
    .await?;

    let status_changed = ticket.status != old_status;
    let assigned_changed = ticket.assigned_to != old_assigned_to;

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
    state
        .audit_service
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
    // 1) Status changes -> notify ticket owner (and assignee if present)
    if status_changed {
        if let Some(owner) = ticket.created_by.clone() {
            if owner != claims.sub {
                let title = match ticket.status.as_str() {
                    "closed" => "Ticket closed",
                    "pending" => "Ticket updated",
                    "open" => "Ticket reopened",
                    _ => "Ticket updated",
                }
                .to_string();

                let _ = state
                    .notification_service
                    .create_notification(
                        owner,
                        Some(tenant_id.clone()),
                        title,
                        ticket.subject.clone(),
                        "info".to_string(),
                        "support".to_string(),
                        Some(format!("/support/{id}")),
                    )
                    .await;
            }
        }

        if let Some(assignee) = ticket.assigned_to.clone() {
            if assignee != claims.sub {
                let title = match ticket.status.as_str() {
                    "closed" => "Ticket closed",
                    "pending" => "Ticket needs review",
                    "open" => "Ticket reopened",
                    _ => "Ticket updated",
                }
                .to_string();

                let _ = state
                    .notification_service
                    .create_notification(
                        assignee,
                        Some(tenant_id.clone()),
                        title,
                        ticket.subject.clone(),
                        "info".to_string(),
                        "support".to_string(),
                        Some(format!("/admin/support/{id}")),
                    )
                    .await;
            }
        }
    }

    // 2) Assignment changes -> notify new assignee
    if assigned_changed {
        if let Some(assignee) = ticket.assigned_to.clone() {
            if assignee != claims.sub {
                let _ = state
                    .notification_service
                    .create_notification(
                        assignee,
                        Some(tenant_id.clone()),
                        "Ticket assigned".to_string(),
                        ticket.subject.clone(),
                        "info".to_string(),
                        "support".to_string(),
                        Some(format!("/admin/support/{id}")),
                    )
                    .await;
            }
        }
    }

    Ok(Json(ticket))
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

/// Submit satisfaction rating for a closed ticket (customer only).
pub async fn submit_ticket_satisfaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<SatisfactionDto>,
) -> Result<(), crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;

    if body.rating < 1 || body.rating > 5 {
        return Err(crate::error::AppError::Validation(
            "Rating must be between 1 and 5".to_string(),
        ));
    }

    // Verify ticket exists, is closed, and belongs to this user
    let exists = sqlx::query_scalar::<_, String>(
        "SELECT id FROM support_tickets WHERE id = $1 AND created_by = $2 AND status = 'closed'",
    )
    .bind(&id)
    .bind(&claims.sub)
    .fetch_optional(&state.auth_service.pool)
    .await
    .map_err(crate::error::AppError::from)?;

    if exists.is_none() {
        return Err(crate::error::AppError::NotFound(
            "Ticket not found or not eligible for rating".to_string(),
        ));
    }

    sqlx::query(
        "UPDATE support_tickets SET satisfaction_rating = $1, satisfaction_comment = $2, updated_at = NOW() WHERE id = $3",
    )
    .bind(body.rating)
    .bind(body.comment.as_deref())
    .bind(&id)
    .execute(&state.auth_service.pool)
    .await
    .map_err(crate::error::AppError::from)?;

    Ok(())
}

// =============================================================================
// Sprint 3: Ticket action HTTP endpoints (mobile-technician app)
// =============================================================================

use axum::extract::Multipart;

/// POST /api/support/tickets/:id/start
/// Marks the ticket as in_progress. Only the assignee or an admin can call.
pub async fn start_support_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<SupportTicket>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    let ticket: SupportTicket =
        sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1 AND tenant_id = $2")
            .bind(&id)
            .bind(&tenant_id)
            .fetch_one(&state.auth_service.pool)
            .await?;

    let can_all = state
        .auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);
    let is_assignee = ticket.assigned_to.as_deref() == Some(claims.sub.as_str());
    if !can_all && !is_assignee {
        return Err(crate::error::AppError::Forbidden(
            "Only the assignee can start this ticket".to_string(),
        ));
    }

    if ticket.status == "closed" || ticket.status == "resolved" {
        return Err(crate::error::AppError::Validation(format!(
            "Cannot start a {} ticket",
            ticket.status
        )));
    }

    let now = Utc::now();
    let updated: SupportTicket = sqlx::query_as(
        r#"
        UPDATE support_tickets
        SET status = 'in_progress', started_at = $1, updated_at = $1
        WHERE id = $2 AND tenant_id = $3
        RETURNING *
        "#,
    )
    .bind(now)
    .bind(&id)
    .bind(&tenant_id)
    .fetch_one(&state.auth_service.pool)
    .await?;

    // Notify ticket owner that work has started.
    if let Some(ref owner) = updated.created_by {
        if owner != &claims.sub {
            let subject = format!("Teknisi mulai mengerjakan tiket: {}", updated.subject);
            let body = format!(
                "Teknisi telah mulai mengerjakan tiket Anda ({}). Pantau progresnya di aplikasi.",
                updated.id
            );
            let _ = state
                .notification_service
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

    state.ws_hub.broadcast(crate::http::WsEvent::SupportTicketUpdated {
        ticket_id: updated.id.clone(),
        status: updated.status.clone(),
        actor_id: claims.sub.clone(),
    });

    Ok(Json(updated))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolveTicketDto {
    pub completion_notes: Option<String>,
    pub signature_file_id: Option<String>,
    pub photo_file_ids: Option<Vec<String>>,
}

/// POST /api/support/tickets/:id/resolve
/// Marks the ticket as resolved with completion proof.
pub async fn resolve_support_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ResolveTicketDto>,
) -> Result<Json<SupportTicket>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    let ticket: SupportTicket =
        sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1 AND tenant_id = $2")
            .bind(&id)
            .bind(&tenant_id)
            .fetch_one(&state.auth_service.pool)
            .await?;

    if ticket.status == "closed" {
        return Err(crate::error::AppError::Validation(
            "Ticket is closed".to_string(),
        ));
    }
    if ticket.status == "resolved" {
        return Err(crate::error::AppError::Validation(
            "Ticket already resolved".to_string(),
        ));
    }

    let can_all = state
        .auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);
    let is_assignee = ticket.assigned_to.as_deref() == Some(claims.sub.as_str());
    if !can_all && !is_assignee {
        return Err(crate::error::AppError::Forbidden(
            "Only the assignee can resolve this ticket".to_string(),
        ));
    }

    let photos_json = serde_json::to_value(body.photo_file_ids.unwrap_or_default())
        .map_err(|e| crate::error::AppError::Internal(format!("photo list: {e}")))?;

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
    .bind(body.completion_notes.as_deref())
    .bind(body.signature_file_id.as_deref())
    .bind(photos_json)
    .bind(&id)
    .bind(&tenant_id)
    .fetch_one(&state.auth_service.pool)
    .await?;

    let audit_details = serde_json::json!({
        "action": "resolve",
        "completion_notes": body.completion_notes,
        "photo_count": updated.completion_photos.len(),
        "has_signature": body.signature_file_id.is_some(),
    })
    .to_string();
    state
        .audit_service
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

    if let Some(ref owner) = updated.created_by {
        if owner != &claims.sub {
            let subject = format!("Tiket Anda telah selesai: {}", updated.subject);
            let body2 = format!(
                "Teknisi telah menyelesaikan tiket ({}). Silakan cek aplikasi untuk konfirmasi.",
                updated.id
            );
            let _ = state
                .notification_service
                .create_notification(
                    owner.clone(),
                    Some(tenant_id.clone()),
                    subject,
                    body2,
                    "info".to_string(),
                    "support".to_string(),
                    Some(format!("/support/tickets/{}", updated.id)),
                )
                .await;
        }
    }

    state.ws_hub.broadcast(crate::http::WsEvent::SupportTicketUpdated {
        ticket_id: updated.id.clone(),
        status: updated.status.clone(),
        actor_id: claims.sub.clone(),
    });

    Ok(Json(updated))
}

/// POST /api/support/tickets/:id/photos
/// Multipart upload of a proof-of-work photo for the given ticket.
/// Returns the file_record ID; the technician app includes this ID in
/// the resolve call's `photo_file_ids` array.
pub async fn upload_ticket_photo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<FileRecord>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    // Verify ticket exists and belongs to tenant.
    let ticket: SupportTicket =
        sqlx::query_as("SELECT * FROM support_tickets WHERE id = $1 AND tenant_id = $2")
            .bind(&id)
            .bind(&tenant_id)
            .fetch_one(&state.auth_service.pool)
            .await?;

    let can_all = state
        .auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);
    let is_assignee = ticket.assigned_to.as_deref() == Some(claims.sub.as_str());
    if !can_all && !is_assignee {
        return Err(crate::error::AppError::Forbidden(
            "Only the assignee can upload photos".to_string(),
        ));
    }

    // Read the first multipart field as the photo bytes.
    let mut file_name = format!("ticket-{}-photo.jpg", id);
    let mut content_type = "image/jpeg".to_string();
    let mut data: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        crate::error::AppError::Internal(format!("multipart read: {e}"))
    })? {
        let name = field.name().unwrap_or("").to_string();
        if name == "photo" || name == "file" {
            if let Some(fname) = field.file_name() {
                file_name = fname.to_string();
            }
            if let Some(ct) = field.content_type() {
                content_type = ct.to_string();
            }
            let bytes = field
                .bytes()
                .await
                .map_err(|e| crate::error::AppError::Internal(format!("photo bytes: {e}")))?;
            data = Some(bytes.to_vec());
        }
    }

    let data = data.ok_or_else(|| {
        crate::error::AppError::Validation("Missing 'photo' field".to_string())
    })?;
    if data.is_empty() {
        return Err(crate::error::AppError::Validation(
            "Empty photo upload".to_string(),
        ));
    }
    // Hard cap 10 MB to prevent OOM from mobile uploads.
    if data.len() > 10 * 1024 * 1024 {
        return Err(crate::error::AppError::Validation(
            "Photo too large (max 10 MB)".to_string(),
        ));
    }

    let (file_path, safe_name, file_id) = state
        .storage_service
        .prepare_upload_path(&tenant_id, &file_name)
        .await?;

    tokio::fs::write(&file_path, &data)
        .await
        .map_err(|e| crate::error::AppError::Internal(format!("disk write: {e}")))?;

    let file_record = state
        .storage_service
        .register_upload(
            &tenant_id,
            &file_id,
            &file_name,
            &safe_name,
            file_path.to_string_lossy().as_ref(),
            &content_type,
            data.len() as i64,
            "local",
            Some(&claims.sub),
            false,
        )
        .await?;

    Ok(Json(file_record))
}

/// List team members eligible for ticket assignment.
/// Returns users with support permission OR owner/admin/technician/noc/planner/staff roles.
/// Excludes customers.
pub async fn list_support_assignees(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamMemberWithUser>>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await?;

    #[cfg(feature = "postgres")]
    let eligible: Vec<TeamMemberWithUser> = sqlx::query_as(
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
    .fetch_all(&state.auth_service.pool)
    .await?;

    #[cfg(not(feature = "postgres"))]
    let eligible: Vec<TeamMemberWithUser> = sqlx::query_as(
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
    .fetch_all(&state.auth_service.pool)
    .await?;

    Ok(Json(eligible))
}

#[cfg(test)]
mod tests {
    use super::ListParams;
    use axum::extract::Query;
    use axum::http::Uri;

    #[test]
    fn list_params_query_parsing_characterizes_http_request_shape() {
        let uri: Uri = "/?status=open&search=router&page=2&per_page=50"
            .parse()
            .expect("valid uri");
        let Query(params) = Query::<ListParams>::try_from_uri(&uri).expect("params parse");

        assert_eq!(params.status.as_deref(), Some("open"));
        assert_eq!(params.search.as_deref(), Some("router"));
        assert_eq!(params.page, Some(2));
        assert_eq!(params.per_page, Some(50));
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
