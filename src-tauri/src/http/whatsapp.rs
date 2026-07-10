use super::AppState;
use crate::error::AppError;
use crate::models::whatsapp::{
    WhatsappCustomerSendRequest, WhatsappEventDefinition, WhatsappGatewayReadiness,
    WhatsappTestSendRequest, WhatsappTestSendResponse,
};
use axum::{extract::State, http::HeaderMap, Json};

fn get_token(headers: &HeaderMap) -> Result<String, AppError> {
    crate::http::extract_token(headers)
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

async fn authorize_readiness_access(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<String>, AppError> {
    let token = get_token(headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    if claims.is_super_admin {
        return Ok(None);
    }

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| AppError::Forbidden("Tenant context required".to_string()))?;

    if state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "settings", "read")
        .await
        .is_ok()
        || state
            .auth_service
            .check_permission(&claims.sub, &tenant_id, "customers", "manage")
            .await
            .is_ok()
    {
        return Ok(Some(tenant_id));
    }

    Err(AppError::Forbidden("Permission denied".to_string()))
}

async fn authorize_customer_manage(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(String, String), AppError> {
    let token = get_token(headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| AppError::Forbidden("Tenant context required".to_string()))?;
    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "customers", "manage")
        .await?;
    Ok((claims.sub, tenant_id))
}

pub async fn list_events(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<WhatsappEventDefinition>>, AppError> {
    let _ = authorize_settings_access(&state, &headers, "read").await?;
    Ok(Json(state.whatsapp_gateway_service.events()))
}

pub async fn readiness(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<WhatsappGatewayReadiness>, AppError> {
    let tenant_id = authorize_readiness_access(&state, &headers).await?;
    let readiness = state
        .whatsapp_gateway_service
        .readiness(tenant_id.as_deref())
        .await?;
    Ok(Json(readiness))
}

pub async fn customer_send(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<WhatsappCustomerSendRequest>,
) -> Result<Json<WhatsappTestSendResponse>, AppError> {
    let (actor_id, tenant_id) = authorize_customer_manage(&state, &headers).await?;
    let readiness = state
        .whatsapp_gateway_service
        .readiness(Some(&tenant_id))
        .await?;
    if !readiness.ready {
        return Err(AppError::Validation(
            readiness
                .reason
                .unwrap_or_else(|| "WhatsApp gateway is not ready".to_string()),
        ));
    }

    let customer = state
        .customer_service
        .get_customer(&actor_id, &tenant_id, &payload.customer_id)
        .await?;
    let phone = customer
        .phone
        .as_deref()
        .map(str::trim)
        .filter(|phone| !phone.is_empty())
        .ok_or_else(|| AppError::Validation("Customer phone is not set".to_string()))?;
    let rendered_message = if let Some(template_id) = payload
        .template_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let tenant_name = state
            .message_template_service
            .tenant_name(&tenant_id)
            .await?;
        state
            .message_template_service
            .render_customer_whatsapp(&tenant_id, template_id, &customer, tenant_name.as_deref())
            .await?
    } else {
        let message = payload.message.trim();
        if message.is_empty() {
            return Err(AppError::Validation(
                "WhatsApp message is required".to_string(),
            ));
        }
        message.to_string()
    };
    let event_code = payload
        .template
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("customer_manual_message:{value}"))
        .unwrap_or_else(|| "customer_manual_message".to_string());

    let result = state
        .whatsapp_gateway_service
        .send_text_response(
            Some(&tenant_id),
            &event_code,
            None,
            phone,
            &rendered_message,
        )
        .await?;
    Ok(Json(result))
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
        assert!(source.contains("pub async fn readiness"));
        assert!(source.contains("pub async fn customer_send"));
        assert!(source.contains("\"settings\", action"));
        assert!(source.contains("\"customers\", \"manage\""));
        assert!(source.contains("authorize_readiness_access"));
    }
}
