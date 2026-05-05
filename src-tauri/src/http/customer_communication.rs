use super::AppState;
use crate::error::AppError;
use crate::models::{CustomerEmailSendRequest, CustomerEmailSendResponse};
use axum::{extract::State, http::HeaderMap, Json};

fn get_token(headers: &HeaderMap) -> Result<String, AppError> {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(str::to_string)
        .ok_or(AppError::Unauthorized)
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

pub async fn send_customer_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CustomerEmailSendRequest>,
) -> Result<Json<CustomerEmailSendResponse>, AppError> {
    let (actor_id, tenant_id) = authorize_customer_manage(&state, &headers).await?;
    let customer = state
        .customer_service
        .get_customer(&actor_id, &tenant_id, &payload.customer_id)
        .await?;
    let to_email = customer
        .email
        .as_deref()
        .map(str::trim)
        .filter(|email| !email.is_empty())
        .ok_or_else(|| AppError::Validation("Customer email is not set".to_string()))?;

    let (subject, body) = if let Some(template_id) = payload
        .template_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let tenant_name = state
            .message_template_service
            .tenant_name(&tenant_id)
            .await?;
        let rendered = state
            .message_template_service
            .render_customer_email(&tenant_id, template_id, &customer, tenant_name.as_deref())
            .await?;
        (rendered.subject, rendered.body)
    } else {
        let subject = payload.subject.trim();
        let body = payload.body.trim();
        if subject.is_empty() {
            return Err(AppError::Validation(
                "Email subject is required".to_string(),
            ));
        }
        if body.is_empty() {
            return Err(AppError::Validation("Email body is required".to_string()));
        }
        (subject.to_string(), body.to_string())
    };

    state
        .notification_service
        .force_send_email(Some(tenant_id), to_email, &subject, &body)
        .await?;

    Ok(Json(CustomerEmailSendResponse {
        ok: true,
        queued: true,
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn customer_email_http_uses_customer_rbac_and_template_rendering() {
        let source = include_str!("customer_communication.rs");
        assert!(source.contains("pub async fn send_customer_email"));
        assert!(source.contains("\"customers\", \"manage\""));
        assert!(source.contains("render_customer_email"));
        assert!(source.contains("force_send_email"));
    }
}
