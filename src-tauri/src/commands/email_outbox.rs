//! Email Outbox (admin monitor + retry)

use crate::models::{EmailOutboxItem, EmailOutboxStats, PaginatedResponse};
use crate::services::AuthService;
use chrono::Utc;
use tauri::State;

#[tauri::command]
pub async fn list_email_outbox(
    token: String,
    page: Option<u32>,
    per_page: Option<u32>,
    status: Option<String>,
    search: Option<String>,
    auth_service: State<'_, AuthService>,
) -> Result<PaginatedResponse<EmailOutboxItem>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "email_outbox", "read")
        .await
        .map_err(|e| e.to_string())?;

    #[cfg(feature = "postgres")]
    let (rows, total) = {
        use sqlx::Postgres;
        use sqlx::QueryBuilder;

        let page = page.unwrap_or(1).max(1);
        let per_page = per_page.unwrap_or(25).clamp(1, 100);
        let offset: i64 = ((page - 1) * per_page) as i64;

        let status = status
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all");
        let search = search.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty());

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
            .fetch_one(&auth_service.pool)
            .await
            .map_err(|e| e.to_string())?;

        qb.push(" ORDER BY eo.created_at DESC");
        qb.push(" LIMIT ");
        qb.push_bind(per_page as i64);
        qb.push(" OFFSET ");
        qb.push_bind(offset);

        let rows: Vec<EmailOutboxItem> = qb
            .build_query_as()
            .fetch_all(&auth_service.pool)
            .await
            .map_err(|e| e.to_string())?;

        (rows, total)
    };

    #[cfg(not(feature = "postgres"))]
    let (rows, total): (Vec<EmailOutboxItem>, i64) = (Vec::new(), 0);

    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(25).clamp(1, 100);

    Ok(PaginatedResponse {
        data: rows,
        total,
        page,
        per_page,
    })
}

#[tauri::command]
pub async fn get_email_outbox_stats(
    token: String,
    auth_service: State<'_, AuthService>,
) -> Result<EmailOutboxStats, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "email_outbox", "read")
        .await
        .map_err(|e| e.to_string())?;

    #[cfg(feature = "postgres")]
    {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT status, COUNT(*)::bigint FROM email_outbox WHERE tenant_id = $1 GROUP BY status",
        )
        .bind(&tenant_id)
        .fetch_all(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

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

        return Ok(stats);
    }

    #[cfg(not(feature = "postgres"))]
    Ok(EmailOutboxStats {
        all: 0,
        queued: 0,
        sending: 0,
        sent: 0,
        failed: 0,
    })
}

#[tauri::command]
pub async fn retry_email_outbox(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
) -> Result<serde_json::Value, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "email_outbox", "retry")
        .await
        .map_err(|e| e.to_string())?;

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
        .execute(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

        if res.rows_affected() == 0 {
            return Err("Not found (or currently sending)".to_string());
        }
    }

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub async fn delete_email_outbox(
    token: String,
    id: String,
    auth_service: State<'_, AuthService>,
) -> Result<serde_json::Value, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "email_outbox", "delete")
        .await
        .map_err(|e| e.to_string())?;

    #[cfg(feature = "postgres")]
    {
        let res = sqlx::query(
            "DELETE FROM email_outbox WHERE id = $1 AND tenant_id = $2 AND status != 'sending'",
        )
        .bind(&id)
        .bind(&tenant_id)
        .execute(&auth_service.pool)
        .await
        .map_err(|e| e.to_string())?;

        if res.rows_affected() == 0 {
            return Err("Not found (or currently sending)".to_string());
        }
    }

    Ok(serde_json::json!({ "success": true }))
}
