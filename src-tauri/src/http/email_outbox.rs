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
    pub scope: Option<String>, // tenant | global | all (super admin)
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub status: Option<String>,
    pub search: Option<String>,
}

#[derive(Deserialize)]
pub struct EmailOutboxStatsQuery {
    pub scope: Option<String>, // tenant | global | all (super admin)
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

    let scope = query.scope.clone().unwrap_or_else(|| "tenant".to_string());
    if scope != "tenant" && !claims.is_super_admin {
        return Err(AppError::Forbidden("Forbidden".to_string()));
    }

    let tenant_id = claims.tenant_id.clone();
    if scope == "tenant" && tenant_id.is_none() {
        return Err(AppError::Unauthorized);
    }

    if let Some(tid) = tenant_id.as_deref() {
        state
            .auth_service
            .check_permission(&claims.sub, tid, "email_outbox", "read")
            .await?;
    } else if !claims.is_super_admin {
        return Err(AppError::Forbidden("Forbidden".to_string()));
    }

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
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            r#"
            SELECT
              eo.id::text AS id,
              eo.tenant_id::text AS tenant_id,
              eo.to_email,
              eo.subject,
              eo.body,
              eo.body_html,
              eo.status,
              eo.attempts,
              eo.max_attempts,
              eo.scheduled_at,
              eo.last_error,
              eo.sent_at,
              eo.created_at,
              eo.updated_at
            FROM email_outbox eo
            WHERE 1=1
        "#,
        );

        match scope.as_str() {
            "global" => {
                qb_count.push(" AND eo.tenant_id IS NULL");
                qb.push(" AND eo.tenant_id IS NULL");
            }
            "all" => {
                // no tenant filter
            }
            _ => {
                // tenant (default) - cast to text to be compatible with legacy schemas that used UUID columns.
                qb_count.push(" AND eo.tenant_id::text = ");
                qb_count.push_bind(tenant_id.as_deref().unwrap_or_default());
                qb.push(" AND eo.tenant_id::text = ");
                qb.push_bind(tenant_id.as_deref().unwrap_or_default());
            }
        }

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
    query: Query<EmailOutboxStatsQuery>,
) -> AppResult<Json<EmailOutboxStats>> {
    let token = bearer_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    // allow scope via query param: /stats?scope=global
    let scope = query.scope.clone().unwrap_or_else(|| "tenant".to_string());
    if scope != "tenant" && !claims.is_super_admin {
        return Err(AppError::Forbidden("Forbidden".to_string()));
    }

    let tenant_id = claims.tenant_id.clone();
    if scope == "tenant" && tenant_id.is_none() {
        return Err(AppError::Unauthorized);
    }

    if let Some(tid) = tenant_id.as_deref() {
        state
            .auth_service
            .check_permission(&claims.sub, tid, "email_outbox", "read")
            .await?;
    } else if !claims.is_super_admin {
        return Err(AppError::Forbidden("Forbidden".to_string()));
    }

    #[cfg(feature = "postgres")]
    {
        let rows: Vec<(String, i64)> = match scope.as_str() {
            "global" => {
                sqlx::query_as(
                    "SELECT status, COUNT(*)::bigint FROM email_outbox WHERE tenant_id IS NULL GROUP BY status",
                )
                .fetch_all(&state.auth_service.pool)
                .await
                .map_err(AppError::Database)?
            }
            "all" => {
                sqlx::query_as("SELECT status, COUNT(*)::bigint FROM email_outbox GROUP BY status")
                    .fetch_all(&state.auth_service.pool)
                    .await
                    .map_err(AppError::Database)?
            }
            _ => {
                sqlx::query_as(
                    "SELECT status, COUNT(*)::bigint FROM email_outbox WHERE tenant_id::text = $1 GROUP BY status",
                )
                .bind(tenant_id.as_deref().unwrap_or_default())
                .fetch_all(&state.auth_service.pool)
                .await
                .map_err(AppError::Database)?
            }
        };

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

    if let Some(tid) = claims.tenant_id.as_deref() {
        state
            .auth_service
            .check_permission(&claims.sub, tid, "email_outbox", "retry")
            .await?;
    } else if !claims.is_super_admin {
        return Err(AppError::Unauthorized);
    }

    #[cfg(feature = "postgres")]
    {
        let now = Utc::now();
        let res = if claims.is_super_admin {
            sqlx::query(
                r#"
                UPDATE email_outbox
                SET status = 'queued',
                    attempts = 0,
                    scheduled_at = $1,
                    last_error = NULL,
                    sent_at = NULL,
                    updated_at = $1
                WHERE id::text = $2
                  AND status != 'sending'
            "#,
            )
            .bind(now)
            .bind(&id)
            .execute(&state.auth_service.pool)
            .await
            .map_err(AppError::Database)?
        } else {
            let tid = claims.tenant_id.clone().unwrap_or_default();
            sqlx::query(
                r#"
                UPDATE email_outbox
                SET status = 'queued',
                    attempts = 0,
                    scheduled_at = $1,
                    last_error = NULL,
                    sent_at = NULL,
                    updated_at = $1
                WHERE id::text = $2
                  AND tenant_id::text = $3
                  AND status != 'sending'
            "#,
            )
            .bind(now)
            .bind(&id)
            .bind(&tid)
            .execute(&state.auth_service.pool)
            .await
            .map_err(AppError::Database)?
        };

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

    if let Some(tid) = claims.tenant_id.as_deref() {
        state
            .auth_service
            .check_permission(&claims.sub, tid, "email_outbox", "delete")
            .await?;
    } else if !claims.is_super_admin {
        return Err(AppError::Unauthorized);
    }

    #[cfg(feature = "postgres")]
    {
        let res = if claims.is_super_admin {
            sqlx::query("DELETE FROM email_outbox WHERE id::text = $1 AND status != 'sending'")
                .bind(&id)
                .execute(&state.auth_service.pool)
                .await
                .map_err(AppError::Database)?
        } else {
            let tid = claims.tenant_id.clone().unwrap_or_default();
            sqlx::query(
                "DELETE FROM email_outbox WHERE id::text = $1 AND tenant_id::text = $2 AND status != 'sending'",
            )
            .bind(&id)
            .bind(&tid)
            .execute(&state.auth_service.pool)
            .await
            .map_err(AppError::Database)?
        };

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Outbox item not found (or currently sending)".into()));
        }
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
