use crate::error::{AppError, AppResult};
use crate::models::whatsapp::{
    WhatsappEventDefinition, WhatsappEventScope, WhatsappGatewayConfig, WhatsappGatewayReadiness,
    WhatsappHttpMethod, WhatsappProvider, WhatsappTestSendResponse,
};
use crate::services::SettingsService;
use std::collections::HashMap;
use uuid::Uuid;

const FONNTE_DEFAULT_BASE_URL: &str = "https://api.fonnte.com/send";
const TRIWAX_SEND_URL: &str = "https://api.triwax.com/api/external/v1/send";

async fn setting_value(
    svc: &SettingsService,
    tenant_id: Option<&str>,
    key: &str,
) -> AppResult<String> {
    Ok(svc.get_value(tenant_id, key).await?.unwrap_or_default())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhatsappGatewayRequest {
    pub url: String,
    pub method: WhatsappHttpMethod,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub success_statuses: Vec<u16>,
}

#[derive(Debug, Clone)]
pub struct WhatsappEventRegistry {
    platform_events: Vec<WhatsappEventDefinition>,
    tenant_events: Vec<WhatsappEventDefinition>,
}

#[derive(Clone)]
pub struct WhatsappGatewayService {
    pool: crate::db::DbPool,
    settings_service: SettingsService,
    client: reqwest::Client,
}

impl Default for WhatsappEventRegistry {
    fn default() -> Self {
        fn event(
            scope: WhatsappEventScope,
            code: &str,
            label: &str,
            description: &str,
        ) -> WhatsappEventDefinition {
            WhatsappEventDefinition {
                scope,
                code: code.to_string(),
                label: label.to_string(),
                description: Some(description.to_string()),
            }
        }

        Self {
            platform_events: vec![
                event(
                    WhatsappEventScope::Platform,
                    "tenant_invoice_created",
                    "Tenant invoice created",
                    "A platform invoice has been issued to a tenant.",
                ),
                event(
                    WhatsappEventScope::Platform,
                    "tenant_invoice_due",
                    "Tenant invoice due",
                    "A tenant platform invoice is approaching or past due.",
                ),
                event(
                    WhatsappEventScope::Platform,
                    "tenant_subscription_expiring",
                    "Tenant subscription expiring",
                    "A tenant subscription is close to expiry.",
                ),
                event(
                    WhatsappEventScope::Platform,
                    "system_alert",
                    "System alert",
                    "A platform-level system alert was raised.",
                ),
                event(
                    WhatsappEventScope::Platform,
                    "backup_failed",
                    "Backup failed",
                    "A platform backup job failed.",
                ),
            ],
            tenant_events: vec![
                event(
                    WhatsappEventScope::Tenant,
                    "customer_invoice_created",
                    "Customer invoice created",
                    "A customer invoice was created.",
                ),
                event(
                    WhatsappEventScope::Tenant,
                    "customer_invoice_due",
                    "Customer invoice due",
                    "A customer invoice is approaching or past due.",
                ),
                event(
                    WhatsappEventScope::Tenant,
                    "payment_received",
                    "Payment received",
                    "A customer payment has been recorded.",
                ),
                event(
                    WhatsappEventScope::Tenant,
                    "installation_scheduled",
                    "Installation scheduled",
                    "A customer installation has been scheduled.",
                ),
                event(
                    WhatsappEventScope::Tenant,
                    "installation_completed",
                    "Installation completed",
                    "A customer installation has been completed.",
                ),
                event(
                    WhatsappEventScope::Tenant,
                    "support_ticket_replied",
                    "Support ticket replied",
                    "A support ticket received a reply.",
                ),
                event(
                    WhatsappEventScope::Tenant,
                    "network_router_down",
                    "Router down",
                    "A tenant router is offline or unreachable.",
                ),
            ],
        }
    }
}

impl WhatsappEventRegistry {
    pub fn events_for_scope(&self, scope: WhatsappEventScope) -> &[WhatsappEventDefinition] {
        match scope {
            WhatsappEventScope::Platform => &self.platform_events,
            WhatsappEventScope::Tenant => &self.tenant_events,
        }
    }

    pub fn all_events(&self) -> Vec<WhatsappEventDefinition> {
        self.platform_events
            .iter()
            .chain(self.tenant_events.iter())
            .cloned()
            .collect()
    }
}

pub fn normalize_phone(phone: &str) -> String {
    let compact: String = phone
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_digit() || *ch == '+')
        .collect();
    let without_plus = compact.strip_prefix('+').unwrap_or(&compact);

    if let Some(rest) = without_plus.strip_prefix('0') {
        format!("62{rest}")
    } else {
        without_plus.to_string()
    }
}

pub fn validate_gateway_config(config: &WhatsappGatewayConfig) -> AppResult<()> {
    match config.provider {
        WhatsappProvider::Disabled => Ok(()),
        WhatsappProvider::Fonnte => {
            if config
                .fonnte_token
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
            {
                return Err(AppError::Validation(
                    "Fonnte token is required when WhatsApp gateway is enabled".to_string(),
                ));
            }

            Ok(())
        }
        WhatsappProvider::Triwax => {
            if config
                .triwax_api_key
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
            {
                return Err(AppError::Validation(
                    "Triwax API key is required when WhatsApp gateway is enabled".to_string(),
                ));
            }

            Ok(())
        }
    }
}

pub fn build_gateway_request(
    config: &WhatsappGatewayConfig,
    phone: &str,
    message: &str,
) -> AppResult<WhatsappGatewayRequest> {
    validate_gateway_config(config)?;

    match config.provider {
        WhatsappProvider::Disabled => Err(AppError::Validation(
            "WhatsApp gateway provider is disabled".to_string(),
        )),
        WhatsappProvider::Fonnte => build_fonnte_request(config, phone, message),
        WhatsappProvider::Triwax => build_triwax_request(config, phone, message),
    }
}

fn build_fonnte_request(
    config: &WhatsappGatewayConfig,
    phone: &str,
    message: &str,
) -> AppResult<WhatsappGatewayRequest> {
    let mut headers = HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        config.fonnte_token.clone().unwrap_or_default(),
    );
    headers.insert(
        "Content-Type".to_string(),
        "application/x-www-form-urlencoded".to_string(),
    );

    let url = config
        .fonnte_base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(FONNTE_DEFAULT_BASE_URL)
        .to_string();

    let mut pairs = vec![
        format!("target={}", normalize_phone(phone)),
        format!("message={message}"),
    ];
    if let Some(sender) = config
        .fonnte_sender
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        pairs.push(format!("device={sender}"));
    }

    Ok(WhatsappGatewayRequest {
        url,
        method: WhatsappHttpMethod::Post,
        headers,
        body: Some(pairs.join("&")),
        success_statuses: vec![200, 201, 202],
    })
}

fn build_triwax_request(
    config: &WhatsappGatewayConfig,
    phone: &str,
    message: &str,
) -> AppResult<WhatsappGatewayRequest> {
    let mut headers = HashMap::new();
    headers.insert(
        "X-API-Key".to_string(),
        config.triwax_api_key.clone().unwrap_or_default(),
    );
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    let body = serde_json::json!({
        "phone": normalize_phone(phone),
        "message": message,
    })
    .to_string();

    Ok(WhatsappGatewayRequest {
        url: TRIWAX_SEND_URL.to_string(),
        method: WhatsappHttpMethod::Post,
        headers,
        body: Some(body),
        success_statuses: vec![200, 201, 202],
    })
}

impl WhatsappGatewayService {
    pub fn new(pool: crate::db::DbPool, settings_service: SettingsService) -> Self {
        Self {
            pool,
            settings_service,
            client: reqwest::Client::new(),
        }
    }

    pub fn events(&self) -> Vec<WhatsappEventDefinition> {
        WhatsappEventRegistry::default().all_events()
    }

    pub async fn load_config(&self, tenant_id: Option<&str>) -> AppResult<WhatsappGatewayConfig> {
        let enabled =
            setting_value(&self.settings_service, tenant_id, "wa_gateway_enabled").await? == "true";
        let provider = match setting_value(&self.settings_service, tenant_id, "wa_gateway_provider")
            .await?
            .as_str()
        {
            "fonnte" if enabled => WhatsappProvider::Fonnte,
            "triwax" if enabled => WhatsappProvider::Triwax,
            _ => WhatsappProvider::Disabled,
        };

        Ok(WhatsappGatewayConfig {
            provider,
            fonnte_base_url: Some(
                setting_value(
                    &self.settings_service,
                    tenant_id,
                    "wa_gateway_fonnte_base_url",
                )
                .await?,
            ),
            fonnte_token: Some(
                setting_value(&self.settings_service, tenant_id, "wa_gateway_fonnte_token").await?,
            ),
            fonnte_sender: Some(
                setting_value(
                    &self.settings_service,
                    tenant_id,
                    "wa_gateway_fonnte_sender",
                )
                .await?,
            ),
            triwax_api_key: Some(
                setting_value(
                    &self.settings_service,
                    tenant_id,
                    "wa_gateway_triwax_api_key",
                )
                .await?,
            ),
        })
    }

    pub async fn readiness(&self, tenant_id: Option<&str>) -> AppResult<WhatsappGatewayReadiness> {
        let enabled =
            setting_value(&self.settings_service, tenant_id, "wa_gateway_enabled").await? == "true";
        let configured_provider =
            setting_value(&self.settings_service, tenant_id, "wa_gateway_provider").await?;
        let provider = match configured_provider.as_str() {
            "fonnte" => WhatsappProvider::Fonnte,
            "triwax" => WhatsappProvider::Triwax,
            _ => WhatsappProvider::Disabled,
        };
        let provider_label = provider_name(provider).to_string();

        if !enabled {
            return Ok(WhatsappGatewayReadiness {
                ready: false,
                provider: provider_label,
                reason: Some("WhatsApp gateway is disabled".to_string()),
            });
        }

        let reason = match provider {
            WhatsappProvider::Disabled => Some("WhatsApp gateway provider is not selected"),
            WhatsappProvider::Fonnte => {
                let token =
                    setting_value(&self.settings_service, tenant_id, "wa_gateway_fonnte_token")
                        .await?;
                token
                    .trim()
                    .is_empty()
                    .then_some("Fonnte token is not configured")
            }
            WhatsappProvider::Triwax => {
                let api_key = setting_value(
                    &self.settings_service,
                    tenant_id,
                    "wa_gateway_triwax_api_key",
                )
                .await?;
                api_key
                    .trim()
                    .is_empty()
                    .then_some("Triwax API key is not configured")
            }
        };

        Ok(WhatsappGatewayReadiness {
            ready: reason.is_none(),
            provider: provider_label,
            reason: reason.map(str::to_string),
        })
    }

    pub async fn test_send(
        &self,
        tenant_id: Option<&str>,
        phone: &str,
        message: &str,
    ) -> AppResult<WhatsappTestSendResponse> {
        let config = self.load_config(tenant_id).await?;
        let request = build_gateway_request(&config, phone, message)?;
        let provider = provider_name(config.provider);

        let method = match request.method {
            WhatsappHttpMethod::Get => reqwest::Method::GET,
            WhatsappHttpMethod::Post => reqwest::Method::POST,
            WhatsappHttpMethod::Put => reqwest::Method::PUT,
            WhatsappHttpMethod::Patch => reqwest::Method::PATCH,
            WhatsappHttpMethod::Delete => reqwest::Method::DELETE,
        };

        let mut req = self.client.request(method, &request.url);
        for (key, value) in &request.headers {
            req = req.header(key, value);
        }
        if let Some(body) = &request.body {
            req = req.body(body.clone());
        }

        let response = req
            .send()
            .await
            .map_err(|err| AppError::Internal(format!("WhatsApp gateway request failed: {err}")))?;
        let status = response.status().as_u16();
        let ok = request.success_statuses.contains(&status);
        let error = if ok {
            None
        } else {
            Some(format!("HTTP {status}"))
        };

        self.log_delivery(
            tenant_id,
            if tenant_id.is_some() {
                "tenant"
            } else {
                "platform"
            },
            "test_send",
            provider,
            None,
            &normalize_phone(phone),
            if ok { "sent" } else { "failed" },
            error.clone(),
        )
        .await?;

        Ok(WhatsappTestSendResponse {
            ok,
            provider: provider.to_string(),
            status: Some(status),
            error,
        })
    }

    pub async fn send_text(
        &self,
        tenant_id: Option<&str>,
        event_code: &str,
        recipient_user_id: Option<&str>,
        phone: &str,
        message: &str,
    ) -> AppResult<()> {
        self.send_text_response(tenant_id, event_code, recipient_user_id, phone, message)
            .await
            .map(|_| ())
    }

    pub async fn send_text_response(
        &self,
        tenant_id: Option<&str>,
        event_code: &str,
        recipient_user_id: Option<&str>,
        phone: &str,
        message: &str,
    ) -> AppResult<WhatsappTestSendResponse> {
        let config = self.load_config(tenant_id).await?;
        let request = build_gateway_request(&config, phone, message)?;
        let provider = provider_name(config.provider);

        let method = match request.method {
            WhatsappHttpMethod::Get => reqwest::Method::GET,
            WhatsappHttpMethod::Post => reqwest::Method::POST,
            WhatsappHttpMethod::Put => reqwest::Method::PUT,
            WhatsappHttpMethod::Patch => reqwest::Method::PATCH,
            WhatsappHttpMethod::Delete => reqwest::Method::DELETE,
        };

        let mut req = self.client.request(method, &request.url);
        for (key, value) in &request.headers {
            req = req.header(key, value);
        }
        if let Some(body) = &request.body {
            req = req.body(body.clone());
        }

        let result = req.send().await;
        let (delivery_status, http_status, error) = match result {
            Ok(response) => {
                let status = response.status().as_u16();
                if request.success_statuses.contains(&status) {
                    ("sent", Some(status), None)
                } else {
                    ("failed", Some(status), Some(format!("HTTP {status}")))
                }
            }
            Err(err) => ("failed", None, Some(err.to_string())),
        };

        self.log_delivery(
            tenant_id,
            if tenant_id.is_some() {
                "tenant"
            } else {
                "platform"
            },
            event_code,
            provider,
            recipient_user_id,
            &normalize_phone(phone),
            delivery_status,
            error.clone(),
        )
        .await?;

        Ok(WhatsappTestSendResponse {
            ok: delivery_status == "sent",
            provider: provider.to_string(),
            status: http_status,
            error,
        })
    }

    pub async fn is_event_whatsapp_enabled(
        &self,
        tenant_id: Option<&str>,
        event_code: &str,
    ) -> AppResult<bool> {
        let key = if tenant_id.is_some() {
            "wa_events_tenant"
        } else {
            "wa_events_platform"
        };
        let raw = self
            .settings_service
            .get_value(tenant_id, key)
            .await?
            .unwrap_or_default();
        if raw.trim().is_empty() {
            return Ok(false);
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
            return Ok(false);
        };
        Ok(value
            .get(event_code)
            .and_then(|entry| entry.get("whatsapp"))
            .and_then(|enabled| enabled.as_bool())
            .unwrap_or(false))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn log_delivery(
        &self,
        tenant_id: Option<&str>,
        scope: &str,
        event_code: &str,
        provider: &str,
        recipient_user_id: Option<&str>,
        recipient_phone: &str,
        status: &str,
        error_summary: Option<String>,
    ) -> AppResult<()> {
        let tenant_uuid = tenant_id.map(Uuid::parse_str).transpose().map_err(|err| {
            AppError::Internal(format!("Invalid tenant id for WhatsApp log: {err}"))
        })?;
        let recipient_user_uuid =
            recipient_user_id
                .map(Uuid::parse_str)
                .transpose()
                .map_err(|err| {
                    AppError::Internal(format!("Invalid recipient user id for WhatsApp log: {err}"))
                })?;

        sqlx::query(
            r#"
            INSERT INTO whatsapp_delivery_logs
              (id, tenant_id, scope, event_code, provider, recipient_user_id, recipient_phone, status, error_summary, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(tenant_uuid)
        .bind(scope)
        .bind(event_code)
        .bind(provider)
        .bind(recipient_user_uuid)
        .bind(recipient_phone)
        .bind(status)
        .bind(error_summary)
        .bind(chrono::Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

fn provider_name(provider: WhatsappProvider) -> &'static str {
    match provider {
        WhatsappProvider::Disabled => "disabled",
        WhatsappProvider::Fonnte => "fonnte",
        WhatsappProvider::Triwax => "triwax",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    fn triwax_config() -> WhatsappGatewayConfig {
        WhatsappGatewayConfig {
            provider: WhatsappProvider::Triwax,
            fonnte_base_url: None,
            fonnte_token: None,
            fonnte_sender: None,
            triwax_api_key: Some("triwax-secret".to_string()),
        }
    }

    #[test]
    fn validate_enabled_fonnte_requires_token() {
        let config = WhatsappGatewayConfig {
            provider: WhatsappProvider::Fonnte,
            fonnte_base_url: None,
            fonnte_token: None,
            fonnte_sender: None,
            triwax_api_key: None,
        };

        let err = validate_gateway_config(&config).unwrap_err();

        assert!(matches!(err, AppError::Validation(message) if message.contains("Fonnte token")));
    }

    #[test]
    fn validate_enabled_triwax_requires_api_key() {
        let config = WhatsappGatewayConfig {
            provider: WhatsappProvider::Triwax,
            fonnte_base_url: None,
            fonnte_token: None,
            fonnte_sender: None,
            triwax_api_key: None,
        };

        let err = validate_gateway_config(&config).unwrap_err();

        assert!(matches!(err, AppError::Validation(message) if message.contains("Triwax API key")));
    }

    #[test]
    fn normalize_phone_converts_local_and_plus_sixty_two_numbers() {
        assert_eq!(normalize_phone("08123456789"), "628123456789");
        assert_eq!(normalize_phone("+628123456789"), "628123456789");
    }

    #[test]
    fn event_registry_separates_platform_and_tenant_definitions() {
        let registry = WhatsappEventRegistry::default();

        assert!(registry
            .events_for_scope(WhatsappEventScope::Platform)
            .iter()
            .any(|event| event.code == "tenant_invoice_due"));
        assert!(registry
            .events_for_scope(WhatsappEventScope::Tenant)
            .iter()
            .any(|event| event.code == "customer_invoice_due"));
        assert!(!registry
            .events_for_scope(WhatsappEventScope::Platform)
            .iter()
            .any(|event| event.code == "customer_invoice_due"));
    }

    #[test]
    fn builds_fonnte_request_with_default_base_url() {
        let config = WhatsappGatewayConfig {
            provider: WhatsappProvider::Fonnte,
            fonnte_base_url: None,
            fonnte_token: Some("secret-token".to_string()),
            fonnte_sender: None,
            triwax_api_key: None,
        };

        let request = build_gateway_request(&config, "08123456789", "Payment due").unwrap();

        assert_eq!(request.url, "https://api.fonnte.com/send");
        assert_eq!(request.method, WhatsappHttpMethod::Post);
        assert_eq!(
            request.headers.get("Authorization"),
            Some(&"secret-token".to_string())
        );
        assert_eq!(
            request.body.as_deref(),
            Some("target=628123456789&message=Payment due")
        );
    }

    #[test]
    fn builds_fonnte_request_with_custom_base_url_and_sender() {
        let config = WhatsappGatewayConfig {
            provider: WhatsappProvider::Fonnte,
            fonnte_base_url: Some("https://fonnte.local/send".to_string()),
            fonnte_token: Some("secret-token".to_string()),
            fonnte_sender: Some("device-a".to_string()),
            triwax_api_key: None,
        };

        let request = build_gateway_request(&config, "08123456789", "Payment due").unwrap();

        assert_eq!(request.url, "https://fonnte.local/send");
        assert_eq!(
            request.body.as_deref(),
            Some("target=628123456789&message=Payment due&device=device-a")
        );
    }

    #[test]
    fn builds_triwax_request_with_expected_headers_and_body() {
        let config = triwax_config();

        let request = build_gateway_request(&config, "+628123456789", "Hello").unwrap();

        assert_eq!(request.url, "https://api.triwax.com/api/external/v1/send");
        assert_eq!(request.method, WhatsappHttpMethod::Post);
        assert_eq!(
            request.headers.get("X-API-Key"),
            Some(&"triwax-secret".to_string())
        );
        assert_eq!(
            request.headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            request.body.as_deref(),
            Some("{\"message\":\"Hello\",\"phone\":\"628123456789\"}")
        );
    }
}
