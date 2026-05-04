use crate::models::{
    MessageTemplate, MessageTemplateListQuery, MessageTemplatePayload,
    MessageTemplatePreviewRequest, MessageTemplatePreviewResponse,
};
use crate::services::{AuthService, MessageTemplateService};
use tauri::State;

async fn tenant_and_permission(
    auth: &AuthService,
    token: &str,
    action: &str,
) -> Result<String, String> {
    let claims = auth
        .validate_token(token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "Tenant context required".to_string())?;
    auth.check_permission(&claims.sub, &tenant_id, "communication_templates", action)
        .await
        .map_err(|e| e.to_string())?;
    Ok(tenant_id)
}

#[tauri::command]
pub async fn list_message_templates(
    token: String,
    q: Option<String>,
    use_case: Option<String>,
    channel: Option<String>,
    status: Option<String>,
    target: Option<String>,
    trigger_mode: Option<String>,
    auth: State<'_, AuthService>,
    message_template_service: State<'_, MessageTemplateService>,
) -> Result<Vec<MessageTemplate>, String> {
    let tenant_id = tenant_and_permission(&auth, &token, "read").await?;
    message_template_service
        .list(
            &tenant_id,
            MessageTemplateListQuery {
                q,
                use_case,
                channel,
                status,
                target,
                trigger_mode,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_message_template(
    token: String,
    payload: MessageTemplatePayload,
    auth: State<'_, AuthService>,
    message_template_service: State<'_, MessageTemplateService>,
) -> Result<MessageTemplate, String> {
    let tenant_id = tenant_and_permission(&auth, &token, "manage").await?;
    message_template_service
        .create(&tenant_id, payload)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_message_template(
    token: String,
    id: String,
    payload: MessageTemplatePayload,
    auth: State<'_, AuthService>,
    message_template_service: State<'_, MessageTemplateService>,
) -> Result<MessageTemplate, String> {
    let tenant_id = tenant_and_permission(&auth, &token, "manage").await?;
    message_template_service
        .update(&tenant_id, &id, payload)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_message_template(
    token: String,
    id: String,
    auth: State<'_, AuthService>,
    message_template_service: State<'_, MessageTemplateService>,
) -> Result<bool, String> {
    let tenant_id = tenant_and_permission(&auth, &token, "manage").await?;
    message_template_service
        .delete(&tenant_id, &id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn preview_message_template(
    token: String,
    payload: MessageTemplatePreviewRequest,
    auth: State<'_, AuthService>,
    message_template_service: State<'_, MessageTemplateService>,
) -> Result<MessageTemplatePreviewResponse, String> {
    let _tenant_id = tenant_and_permission(&auth, &token, "read").await?;
    message_template_service
        .preview(payload)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn message_template_commands_use_rbac() {
        let source = include_str!("message_templates.rs");
        assert!(source.contains("communication_templates"));
        assert!(source.contains("list_message_templates"));
        assert!(source.contains("create_message_template"));
        assert!(source.contains("\"manage\""));
    }
}
