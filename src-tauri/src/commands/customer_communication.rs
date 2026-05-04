use crate::models::{CustomerEmailSendRequest, CustomerEmailSendResponse};
use crate::services::{AuthService, CustomerService, MessageTemplateService, NotificationService};
use tauri::State;

async fn authorize_customer_manage(
    auth_service: &AuthService,
    token: &str,
) -> Result<(String, String), String> {
    let claims = auth_service
        .validate_token(token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "customers", "manage")
        .await
        .map_err(|e| e.to_string())?;

    Ok((claims.sub, tenant_id))
}

#[tauri::command]
pub async fn send_customer_email(
    token: String,
    payload: CustomerEmailSendRequest,
    auth_service: State<'_, AuthService>,
    customer_service: State<'_, CustomerService>,
    message_template_service: State<'_, MessageTemplateService>,
    notification_service: State<'_, NotificationService>,
) -> Result<CustomerEmailSendResponse, String> {
    let (actor_id, tenant_id) = authorize_customer_manage(&auth_service, &token).await?;
    let customer = customer_service
        .get_customer(&actor_id, &tenant_id, &payload.customer_id)
        .await
        .map_err(|e| e.to_string())?;
    let to_email = customer
        .email
        .as_deref()
        .map(str::trim)
        .filter(|email| !email.is_empty())
        .ok_or_else(|| "Customer email is not set".to_string())?;

    let (subject, body) = if let Some(template_id) = payload
        .template_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let tenant_name = message_template_service
            .tenant_name(&tenant_id)
            .await
            .map_err(|e| e.to_string())?;
        let rendered = message_template_service
            .render_customer_email(&tenant_id, template_id, &customer, tenant_name.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        (rendered.subject, rendered.body)
    } else {
        let subject = payload.subject.trim();
        let body = payload.body.trim();
        if subject.is_empty() {
            return Err("Email subject is required".to_string());
        }
        if body.is_empty() {
            return Err("Email body is required".to_string());
        }
        (subject.to_string(), body.to_string())
    };

    notification_service
        .force_send_email(Some(tenant_id), to_email, &subject, &body)
        .await
        .map_err(|e| e.to_string())?;

    Ok(CustomerEmailSendResponse {
        ok: true,
        queued: true,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn customer_email_command_uses_customer_rbac_and_template_rendering() {
        let source = include_str!("customer_communication.rs");
        assert!(source.contains("pub async fn send_customer_email"));
        assert!(source.contains("\"customers\", \"manage\""));
        assert!(source.contains("render_customer_email"));
        assert!(source.contains("force_send_email"));
    }
}
