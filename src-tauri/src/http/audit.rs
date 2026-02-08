use crate::http::AppState;
use crate::models::PaginatedResponse;
use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};

#[derive(serde::Deserialize)]
pub struct AuditLogQuery {
    page: Option<u32>,
    #[serde(rename = "perPage")]
    per_page: Option<u32>,
    user_id: Option<String>,
    tenant_id: Option<String>,
    action: Option<String>,
    date_from: Option<String>, // Query params are strings, we need to parse dates? Axum handles parsing if type is valid, but ISO strings might need handling. better to verify.
    // If we use AuditLogFilter directly, we need to make sure serde parses it correctly from Query.
    // Axum Query uses serde_urlencoded, which handles basic types.
    // If AuditLogFilter uses DateTime<Utc>, we need feature = "serde" in chrono.
    // It should work if JSON format is standard.
    // However, safest for query is String and parse manually or use chrono defaults.
    date_to: Option<String>,
    search: Option<String>,
}

// Map Query to Filter
impl From<AuditLogQuery> for crate::models::AuditLogFilter {
    fn from(val: AuditLogQuery) -> Self {
        let date_from = val.date_from.and_then(|d| {
            chrono::DateTime::parse_from_rfc3339(&d)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });
        let date_to = val.date_to.and_then(|d| {
            chrono::DateTime::parse_from_rfc3339(&d)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });

        crate::models::AuditLogFilter {
            page: val.page,
            per_page: val.per_page,
            user_id: val.user_id,
            tenant_id: val.tenant_id,
            action: val.action,
            date_from,
            date_to,
            search: val.search,
        }
    }
}

fn extract_token(headers: &HeaderMap) -> Result<String, (axum::http::StatusCode, String)> {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or((
            axum::http::StatusCode::UNAUTHORIZED,
            "Missing or invalid Authorization header".to_string(),
        ))
}

pub async fn list_audit_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuditLogQuery>,
) -> Result<
    Json<PaginatedResponse<crate::models::AuditLogResponse>>,
    (axum::http::StatusCode, String),
> {
    let auth_service = &state.auth_service;
    let audit_service = &state.audit_service;

    let token = extract_token(&headers)?;
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| (axum::http::StatusCode::UNAUTHORIZED, e.to_string()))?;

    if !claims.is_super_admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            "Unauthorized".to_string(),
        ));
    }

    let filter: crate::models::AuditLogFilter = query.into();
    let page = filter.page.unwrap_or(1); // Keep for response
    let per_page = filter.per_page.unwrap_or(20);

    let (logs, total) = audit_service.list(filter).await.map_err(|e| {
        tracing::error!("Failed to list audit logs: {}", e);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(PaginatedResponse {
        data: logs,
        total,
        page,
        per_page,
    }))
}
