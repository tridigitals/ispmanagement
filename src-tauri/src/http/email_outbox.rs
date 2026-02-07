use crate::error::{AppError, AppResult};
use crate::http::AppState;
use crate::models::{EmailOutboxItem, EmailOutboxStats, PaginatedResponse};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListEmailOutboxQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub status: Option<String>,
    pub search: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_email_outbox))
        .route("/stats", get(get_email_outbox_stats))
        .route("/{id}/retry", post(retry_email_outbox))
        .route("/{id}", delete(delete_email_outbox))
}

fn bearer_token(headers: &HeaderMap) -> AppResult<String> {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or(AppError::Unauthorized)
}

// GET /api/email-outbox
async fn list_email_outbox(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Query<ListEmailOutboxQuery>,
) -> AppResult<Json<PaginatedResponse<EmailOutboxItem>>> {
    let token = bearer_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims.tenant_id.clone().ok_or(AppError::Unauthorized)?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "email_outbox", "read")
        .await?;

    #[cfg(feature = "postgres")]
    let (rows, total) = {
        use sqlx::Postgres;
        use sqlx::QueryBuilder;

        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(25).clamp(1, 100);
        let offset: i64 = ((page - 1) * per_page) as i64;

        let status = query
            .status
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all");
        let search = query.search.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty());

        let mut qb_count: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM email_outbox eo WHERE 1=1");
        let mut qb: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT eo.* FROM email_outbox eo WHERE 1=1");

        qb_count.push(" AND eo.tenant_id = ");
        qb_count.push_bind(&tenant_id);
        qb.push(" AND eo.tenant_id = ");
        qb.push_bind(&tenant_id);

        if let Some(st) = status.as_deref() {
            qb_count.push(" AND eo.status = ");
            qb_count.push_bind(st);
            qb.push(" AND eo.status = ");
            qb.push_bind(st);
        }

        if let Some(q) = search {
            let like = format!("%{}%", q);
            qb_count.push(" AND (eo.to_email ILIKE ");
            qb_count.push_bind(like.clone());
            qb_count.push(" OR eo.subject ILIKE ");
            qb_count.push_bind(like.clone());
            qb_count.push(")");

            qb.push(" AND (eo.to_email ILIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR eo.subject ILIKE ");
            qb.push_bind(like);
            qb.push(")");
        }

        let total: i64 = qb_count
            .build_query_scalar()
            .fetch_one(&state.auth_service.pool)
            .await
            .map_err(AppError::Database)?;

        qb.push(" ORDER BY eo.created_at DESC");
        qb.push(" LIMIT ");
        qb.push_bind(per_page as i64);
        qb.push(" OFFSET ");
        qb.push_bind(offset);

        let rows: Vec<EmailOutboxItem> = qb
            .build_query_as()
            .fetch_all(&state.auth_service.pool)
            .await
            .map_err(AppError::Database)?;

        (rows, total)
    };

    #[cfg(not(feature = "postgres"))]
    let (rows, total): (Vec<EmailOutboxItem>, i64) = (Vec::new(), 0);

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(25).clamp(1, 100);

    Ok(Json(PaginatedResponse {
        data: rows,
        total,
        page,
        per_page,
    }))
}

// GET /api/email-outbox/stats
async fn get_email_outbox_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<EmailOutboxStats>> {
    let token = bearer_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims.tenant_id.clone().ok_or(AppError::Unauthorized)?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "email_outbox", "read")
        .await?;

    #[cfg(feature = "postgres")]
    {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT status, COUNT(*)::bigint FROM email_outbox WHERE tenant_id = $1 GROUP BY status",
        )
        .bind(&tenant_id)
        .fetch_all(&state.auth_service.pool)
        .await
        .map_err(AppError::Database)?;

        let mut stats = EmailOutboxStats {
            all: 0,
            queued: 0,
            sending: 0,
            sent: 0,
            failed: 0,
        };

        for (st, c) in rows {
            stats.all += c;
            match st.as_str() {
                "queued" => stats.queued = c,
                "sending" => stats.sending = c,
                "sent" => stats.sent = c,
                "failed" => stats.failed = c,
                _ => {}
            }
        }

        return Ok(Json(stats));
    }

    #[cfg(not(feature = "postgres"))]
    Ok(Json(EmailOutboxStats {
        all: 0,
        queued: 0,
        sending: 0,
        sent: 0,
        failed: 0,
    }))
}

// POST /api/email-outbox/:id/retry
async fn retry_email_outbox(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let token = bearer_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims.tenant_id.clone().ok_or(AppError::Unauthorized)?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "email_outbox", "retry")
        .await?;

    #[cfg(feature = "postgres")]
    {
        let now = Utc::now();
        let res = sqlx::query(
            r#"
            UPDATE email_outbox
            SET status = 'queued',
                attempts = 0,
                scheduled_at = $1,
                last_error = NULL,
                sent_at = NULL,
                updated_at = $1
            WHERE id = $2
              AND tenant_id = $3
              AND status != 'sending'
        "#,
        )
        .bind(now)
        .bind(&id)
        .bind(&tenant_id)
        .execute(&state.auth_service.pool)
        .await
        .map_err(AppError::Database)?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Outbox item not found (or currently sending)".into()));
        }
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

// DELETE /api/email-outbox/:id
async fn delete_email_outbox(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let token = bearer_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims.tenant_id.clone().ok_or(AppError::Unauthorized)?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "email_outbox", "delete")
        .await?;

    #[cfg(feature = "postgres")]
    {
        let res = sqlx::query(
            "DELETE FROM email_outbox WHERE id = $1 AND tenant_id = $2 AND status != 'sending'",
        )
        .bind(&id)
        .bind(&tenant_id)
        .execute(&state.auth_service.pool)
        .await
        .map_err(AppError::Database)?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Outbox item not found (or currently sending)".into()));
        }
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

