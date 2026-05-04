use crate::models::whatsapp::{
    WhatsappEventDefinition, WhatsappGatewayReadiness, WhatsappTestSendResponse,
};
use crate::services::{
    AuthService, CustomerService, MessageTemplateService, WhatsappGatewayService,
};
use tauri::State;

async fn authorize_settings_access(
    auth_service: &AuthService,
    token: &str,
    action: &str,
) -> Result<Option<String>, String> {
    let claims = auth_service
        .validate_token(token)
        .await
        .map_err(|e| e.to_string())?;

    if claims.is_super_admin {
        return Ok(None);
    }

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "Tenant context required".to_string())?;
    auth_service
        .check_permission(&claims.sub, &tenant_id, "settings", action)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Some(tenant_id))
}

async fn authorize_readiness_access(
    auth_service: &AuthService,
    token: &str,
) -> Result<Option<String>, String> {
    let claims = auth_service
        .validate_token(token)
        .await
        .map_err(|e| e.to_string())?;

    if claims.is_super_admin {
        return Ok(None);
    }

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "Tenant context required".to_string())?;

    if auth_service
        .check_permission(&claims.sub, &tenant_id, "settings", "read")
        .await
        .is_ok()
        || auth_service
            .check_permission(&claims.sub, &tenant_id, "customers", "manage")
            .await
            .is_ok()
    {
        return Ok(Some(tenant_id));
    }

    Err("Permission denied".to_string())
}

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
pub async fn list_whatsapp_events(
    token: String,
    whatsapp_gateway_service: State<'_, WhatsappGatewayService>,
    auth_service: State<'_, AuthService>,
) -> Result<Vec<WhatsappEventDefinition>, String> {
    let _ = authorize_settings_access(&auth_service, &token, "read").await?;
    Ok(whatsapp_gateway_service.events())
}

#[tauri::command]
pub async fn get_whatsapp_gateway_readiness(
    token: String,
    whatsapp_gateway_service: State<'_, WhatsappGatewayService>,
    auth_service: State<'_, AuthService>,
) -> Result<WhatsappGatewayReadiness, String> {
    let tenant_id = authorize_readiness_access(&auth_service, &token).await?;
    whatsapp_gateway_service
        .readiness(tenant_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_customer_whatsapp(
    token: String,
    customer_id: String,
    message: String,
    template: Option<String>,
    template_id: Option<String>,
    whatsapp_gateway_service: State<'_, WhatsappGatewayService>,
    auth_service: State<'_, AuthService>,
    customer_service: State<'_, CustomerService>,
    message_template_service: State<'_, MessageTemplateService>,
) -> Result<WhatsappTestSendResponse, String> {
    let (actor_id, tenant_id) = authorize_customer_manage(&auth_service, &token).await?;
    let readiness = whatsapp_gateway_service
        .readiness(Some(&tenant_id))
        .await
        .map_err(|e| e.to_string())?;
    if !readiness.ready {
        return Err(readiness
            .reason
            .unwrap_or_else(|| "WhatsApp gateway is not ready".to_string()));
    }

    let customer = customer_service
        .get_customer(&actor_id, &tenant_id, &customer_id)
        .await
        .map_err(|e| e.to_string())?;
    let phone = customer
        .phone
        .as_deref()
        .map(str::trim)
        .filter(|phone| !phone.is_empty())
        .ok_or_else(|| "Customer phone is not set".to_string())?;
    let rendered_message = if let Some(template_id) = template_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        message_template_service
            .render_customer_whatsapp(&tenant_id, template_id, &customer, None)
            .await
            .map_err(|e| e.to_string())?
    } else {
        let message = message.trim();
        if message.is_empty() {
            return Err("WhatsApp message is required".to_string());
        }
        message.to_string()
    };

    let event_code = template
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("customer_manual_message:{value}"))
        .unwrap_or_else(|| "customer_manual_message".to_string());

    whatsapp_gateway_service
        .send_text_response(
            Some(&tenant_id),
            &event_code,
            None,
            phone,
            &rendered_message,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_test_whatsapp(
    token: String,
    phone: String,
    message: String,
    event_code: Option<String>,
    whatsapp_gateway_service: State<'_, WhatsappGatewayService>,
    auth_service: State<'_, AuthService>,
) -> Result<WhatsappTestSendResponse, String> {
    let tenant_id = authorize_settings_access(&auth_service, &token, "update").await?;
    let phone = phone.trim();
    let message = message.trim();

    if phone.is_empty() {
        return Err("WhatsApp phone is required".to_string());
    }
    if message.is_empty() {
        return Err("WhatsApp test message is required".to_string());
    }

    let _ = event_code;
    whatsapp_gateway_service
        .test_send(tenant_id.as_deref(), phone, message)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn whatsapp_commands_expose_event_list_and_test_send() {
        let source = include_str!("whatsapp.rs");
        assert!(source.contains("pub async fn list_whatsapp_events"));
        assert!(source.contains("pub async fn get_whatsapp_gateway_readiness"));
        assert!(source.contains("pub async fn send_customer_whatsapp"));
        assert!(source.contains("pub async fn send_test_whatsapp"));
        assert!(source.contains("\"settings\", action"));
        assert!(source.contains("\"customers\", \"manage\""));
        assert!(source.contains("authorize_readiness_access"));
    }
}
