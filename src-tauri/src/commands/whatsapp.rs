use crate::models::whatsapp::{WhatsappEventDefinition, WhatsappTestSendResponse};
use crate::services::{AuthService, WhatsappGatewayService};
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
        assert!(source.contains("pub async fn send_test_whatsapp"));
        assert!(source.contains("\"settings\", action"));
    }
}
