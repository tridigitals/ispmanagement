use super::AppState;
use crate::error::AppError;
use crate::models::whatsapp::{
    WhatsappEventDefinition, WhatsappTestSendRequest, WhatsappTestSendResponse,
};
use axum::{extract::State, http::HeaderMap, Json};

fn get_token(headers: &HeaderMap) -> Result<String, AppError> {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(str::to_string)
        .ok_or(AppError::Unauthorized)
}

async fn authorize_settings_access(
    state: &AppState,
    headers: &HeaderMap,
    action: &str,
) -> Result<Option<String>, AppError> {
    let token = get_token(headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    if claims.is_super_admin {
        return Ok(None);
    }

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| AppError::Forbidden("Tenant context required".to_string()))?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "settings", action)
        .await?;
    Ok(Some(tenant_id))
}

pub async fn list_events(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<WhatsappEventDefinition>>, AppError> {
    let _ = authorize_settings_access(&state, &headers, "read").await?;
    Ok(Json(state.whatsapp_gateway_service.events()))
}

pub async fn test_send(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<WhatsappTestSendRequest>,
) -> Result<Json<WhatsappTestSendResponse>, AppError> {
    let tenant_id = authorize_settings_access(&state, &headers, "update").await?;
    let phone = payload.phone.trim();
    let message = payload.message.trim();

    if phone.is_empty() {
        return Err(AppError::Validation(
            "WhatsApp phone is required".to_string(),
        ));
    }
    if message.is_empty() {
        return Err(AppError::Validation(
            "WhatsApp test message is required".to_string(),
        ));
    }

    let result = state
        .whatsapp_gateway_service
        .test_send(tenant_id.as_deref(), phone, message)
        .await?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    #[test]
    fn whatsapp_http_source_exposes_test_send_and_events_handlers() {
        let source = include_str!("whatsapp.rs");
        assert!(source.contains("pub async fn test_send"));
        assert!(source.contains("pub async fn list_events"));
        assert!(source.contains("\"settings\", action"));
    }
}
