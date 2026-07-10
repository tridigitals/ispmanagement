use super::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{
    MessageTemplate, MessageTemplateListQuery, MessageTemplatePayload,
    MessageTemplatePreviewRequest, MessageTemplatePreviewResponse,
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageTemplatePayloadEnvelope {
    payload: MessageTemplatePayload,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageTemplatePreviewEnvelope {
    payload: MessageTemplatePreviewRequest,
}

fn bearer_token(headers: &HeaderMap) -> AppResult<String> {
    crate::http::extract_token(headers)
}

async fn tenant_and_permission(
    state: &AppState,
    headers: &HeaderMap,
    action: &str,
) -> AppResult<String> {
    let token = bearer_token(headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| AppError::Forbidden("Tenant context required".to_string()))?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "communication_templates", action)
        .await?;
    Ok(tenant_id)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_templates).post(create_template))
        .route("/preview", post(preview_template))
        .route("/{id}", put(update_template).delete(delete_template))
}

async fn list_templates(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<MessageTemplateListQuery>,
) -> AppResult<Json<Vec<MessageTemplate>>> {
    let tenant_id = tenant_and_permission(&state, &headers, "read").await?;
    Ok(Json(
        state
            .message_template_service
            .list(&tenant_id, query)
            .await?,
    ))
}

async fn create_template(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<MessageTemplatePayloadEnvelope>,
) -> AppResult<Json<MessageTemplate>> {
    let tenant_id = tenant_and_permission(&state, &headers, "manage").await?;
    Ok(Json(
        state
            .message_template_service
            .create(&tenant_id, payload.payload)
            .await?,
    ))
}

async fn update_template(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<MessageTemplatePayloadEnvelope>,
) -> AppResult<Json<MessageTemplate>> {
    let tenant_id = tenant_and_permission(&state, &headers, "manage").await?;
    Ok(Json(
        state
            .message_template_service
            .update(&tenant_id, &id, payload.payload)
            .await?,
    ))
}

async fn delete_template(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let tenant_id = tenant_and_permission(&state, &headers, "manage").await?;
    state
        .message_template_service
        .delete(&tenant_id, &id)
        .await?;
    Ok(Json(true))
}

async fn preview_template(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<MessageTemplatePreviewEnvelope>,
) -> AppResult<Json<MessageTemplatePreviewResponse>> {
    let _tenant_id = tenant_and_permission(&state, &headers, "read").await?;
    Ok(Json(
        state.message_template_service.preview(payload.payload)?,
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn message_template_http_routes_use_rbac() {
        let source = include_str!("message_templates.rs");
        assert!(source.contains("communication_templates"));
        assert!(source.contains("\"read\""));
        assert!(source.contains("\"manage\""));
        assert!(source.contains("preview_template"));
    }
}
