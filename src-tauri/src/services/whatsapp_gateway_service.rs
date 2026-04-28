use crate::error::{AppError, AppResult};
use crate::models::whatsapp::{
    WhatsappCustomHttpConfig, WhatsappEventDefinition, WhatsappEventScope, WhatsappGatewayConfig,
    WhatsappHttpMethod, WhatsappProvider, WhatsappTestSendResponse,
};
use crate::services::SettingsService;
use std::collections::HashMap;
use uuid::Uuid;

const FONNTE_DEFAULT_BASE_URL: &str = "https://api.fonnte.com/send";

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
        WhatsappProvider::CustomHttp => {
            let custom = config.custom_http.as_ref().ok_or_else(|| {
                AppError::Validation(
                    "Custom HTTP configuration is required for custom_http provider".to_string(),
                )
            })?;

            validate_custom_http_config(custom)
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
        WhatsappProvider::CustomHttp => build_custom_http_request(config, phone, message),
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

fn build_custom_http_request(
    config: &WhatsappGatewayConfig,
    phone: &str,
    message: &str,
) -> AppResult<WhatsappGatewayRequest> {
    let custom = config
        .custom_http
        .as_ref()
        .expect("validated custom config");
    let headers = parse_custom_headers(custom.headers_json.as_deref())?;
    let normalized_phone = normalize_phone(phone);
    let body = custom.body_template.as_ref().map(|template| {
        template
            .replace("{{phone}}", &normalized_phone)
            .replace("{{message}}", message)
    });

    Ok(WhatsappGatewayRequest {
        url: custom.url.trim().to_string(),
        method: custom.method,
        headers,
        body,
        success_statuses: if custom.success_statuses.is_empty() {
            vec![200, 201, 202]
        } else {
            custom.success_statuses.clone()
        },
    })
}

fn validate_custom_http_config(config: &WhatsappCustomHttpConfig) -> AppResult<()> {
    if config.url.trim().is_empty() {
        return Err(AppError::Validation(
            "Custom HTTP URL is required".to_string(),
        ));
    }

    if !matches!(
        config.method,
        WhatsappHttpMethod::Get
            | WhatsappHttpMethod::Post
            | WhatsappHttpMethod::Put
            | WhatsappHttpMethod::Patch
    ) {
        return Err(AppError::Validation(
            "Custom HTTP method must be GET, POST, PUT, or PATCH".to_string(),
        ));
    }

    Ok(())
}

fn parse_custom_headers(headers_json: Option<&str>) -> AppResult<HashMap<String, String>> {
    let Some(headers_json) = headers_json
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(HashMap::new());
    };

    let value: serde_json::Value = serde_json::from_str(headers_json)
        .map_err(|err| AppError::Validation(format!("Invalid custom headers JSON: {err}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| AppError::Validation("Custom headers JSON must be an object".to_string()))?;

    let mut headers = HashMap::new();
    for (key, value) in object {
        let header_value = value.as_str().ok_or_else(|| {
            AppError::Validation("Custom headers JSON values must be strings".to_string())
        })?;
        headers.insert(key.to_string(), header_value.to_string());
    }

    Ok(headers)
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
        async fn setting(
            svc: &SettingsService,
            tenant_id: Option<&str>,
            key: &str,
        ) -> AppResult<String> {
            Ok(svc.get_value(tenant_id, key).await?.unwrap_or_default())
        }

        let provider = match setting(&self.settings_service, tenant_id, "wa_gateway_provider")
            .await?
            .as_str()
        {
            "fonnte" => WhatsappProvider::Fonnte,
            "custom_http" => WhatsappProvider::CustomHttp,
            _ => WhatsappProvider::Disabled,
        };

        let method = match setting(
            &self.settings_service,
            tenant_id,
            "wa_gateway_custom_method",
        )
        .await?
        .to_ascii_uppercase()
        .as_str()
        {
            "GET" => WhatsappHttpMethod::Get,
            "PUT" => WhatsappHttpMethod::Put,
            "PATCH" => WhatsappHttpMethod::Patch,
            "DELETE" => WhatsappHttpMethod::Delete,
            _ => WhatsappHttpMethod::Post,
        };
        let success_statuses = setting(
            &self.settings_service,
            tenant_id,
            "wa_gateway_custom_success_statuses",
        )
        .await?
        .split(',')
        .filter_map(|item| item.trim().parse::<u16>().ok())
        .collect();

        Ok(WhatsappGatewayConfig {
            provider,
            fonnte_base_url: Some(
                setting(
                    &self.settings_service,
                    tenant_id,
                    "wa_gateway_fonnte_base_url",
                )
                .await?,
            ),
            fonnte_token: Some(
                setting(&self.settings_service, tenant_id, "wa_gateway_fonnte_token").await?,
            ),
            fonnte_sender: Some(
                setting(
                    &self.settings_service,
                    tenant_id,
                    "wa_gateway_fonnte_sender",
                )
                .await?,
            ),
            custom_http: Some(WhatsappCustomHttpConfig {
                url: setting(&self.settings_service, tenant_id, "wa_gateway_custom_url").await?,
                method,
                headers_json: Some(
                    setting(
                        &self.settings_service,
                        tenant_id,
                        "wa_gateway_custom_headers",
                    )
                    .await?,
                ),
                body_template: Some(
                    setting(
                        &self.settings_service,
                        tenant_id,
                        "wa_gateway_custom_body_template",
                    )
                    .await?,
                ),
                success_statuses,
            }),
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
        let (status, error) = match result {
            Ok(response) => {
                let status = response.status().as_u16();
                if request.success_statuses.contains(&status) {
                    ("sent", None)
                } else {
                    ("failed", Some(format!("HTTP {status}")))
                }
            }
            Err(err) => ("failed", Some(err.to_string())),
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
            status,
            error,
        )
        .await
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
        WhatsappProvider::CustomHttp => "custom_http",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    fn custom_config() -> WhatsappGatewayConfig {
        WhatsappGatewayConfig {
            provider: WhatsappProvider::CustomHttp,
            fonnte_base_url: None,
            fonnte_token: None,
            fonnte_sender: None,
            custom_http: Some(WhatsappCustomHttpConfig {
                url: "https://gateway.example/send".to_string(),
                method: WhatsappHttpMethod::Post,
                headers_json: None,
                body_template: Some("{\"to\":\"{{phone}}\",\"text\":\"{{message}}\"}".to_string()),
                success_statuses: vec![200, 201, 202],
            }),
        }
    }

    #[test]
    fn validate_enabled_fonnte_requires_token() {
        let config = WhatsappGatewayConfig {
            provider: WhatsappProvider::Fonnte,
            fonnte_base_url: None,
            fonnte_token: None,
            fonnte_sender: None,
            custom_http: None,
        };

        let err = validate_gateway_config(&config).unwrap_err();

        assert!(matches!(err, AppError::Validation(message) if message.contains("Fonnte token")));
    }

    #[test]
    fn validate_custom_http_requires_supported_method() {
        let mut config = custom_config();
        config.custom_http.as_mut().unwrap().method = WhatsappHttpMethod::Delete;

        let err = validate_gateway_config(&config).unwrap_err();

        assert!(
            matches!(err, AppError::Validation(message) if message.contains("GET, POST, PUT, or PATCH"))
        );
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
            custom_http: None,
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
            custom_http: None,
        };

        let request = build_gateway_request(&config, "08123456789", "Payment due").unwrap();

        assert_eq!(request.url, "https://fonnte.local/send");
        assert_eq!(
            request.body.as_deref(),
            Some("target=628123456789&message=Payment due&device=device-a")
        );
    }

    #[test]
    fn builds_custom_request_with_template_substitution_and_headers() {
        let mut config = custom_config();
        config.custom_http.as_mut().unwrap().headers_json =
            Some("{\"X-Api-Key\":\"abc\",\"Content-Type\":\"application/json\"}".to_string());

        let request = build_gateway_request(&config, "+628123456789", "Hello").unwrap();

        assert_eq!(request.url, "https://gateway.example/send");
        assert_eq!(request.method, WhatsappHttpMethod::Post);
        assert_eq!(request.headers.get("X-Api-Key"), Some(&"abc".to_string()));
        assert_eq!(
            request.body.as_deref(),
            Some("{\"to\":\"628123456789\",\"text\":\"Hello\"}")
        );
    }

    #[test]
    fn malformed_custom_headers_json_returns_validation_error() {
        let mut config = custom_config();
        config.custom_http.as_mut().unwrap().headers_json = Some("{bad json".to_string());

        let err = build_gateway_request(&config, "08123456789", "Hello").unwrap_err();

        assert!(matches!(err, AppError::Validation(message) if message.contains("headers JSON")));
    }
}
