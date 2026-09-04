//! Email Service - HTTP API only (SMTP disabled until Windows restart)
//!
//! Providers: Resend, SendGrid, Custom Webhook
//! Note: SMTP support requires uncommenting lettre in Cargo.toml after Windows restart

use crate::error::{AppError, AppResult};
use crate::services::SettingsService;
use base64::Engine;
use lettre::message::{header::ContentType, Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::Tls;
use lettre::transport::smtp::client::TlsParameters;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Serialize;
use std::time::Instant;
use tracing::info;

/// Binary attachment carried alongside an outbound email.
///
/// `content_type` should be a valid MIME type (e.g. `application/pdf`).
/// `content` is the raw bytes — providers that need base64 (Resend,
/// SendGrid, Webhook) handle the encoding internally.
#[derive(Debug, Clone)]
pub struct EmailAttachment {
    pub filename: String,
    pub content_type: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SmtpConnectionTestResult {
    pub ok: bool,
    pub provider: String,
    pub host: String,
    pub port: u16,
    pub encryption: String,
    pub duration_ms: i64,
    pub message: String,
}

/// Email service for sending emails
#[derive(Clone)]
pub struct EmailService {
    settings_service: SettingsService,
}

/// Email configuration from settings  
#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub provider: String,
    pub from_email: String,
    pub from_name: String,
    pub api_key: String,
    pub webhook_url: String,
    // SMTP fields (for future use)
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_encryption: String,
}

/// Email request for Resend API
#[derive(Debug, Serialize)]
struct ResendRequest {
    from: String,
    to: Vec<String>,
    subject: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
}

/// Email request for SendGrid API
#[derive(Debug, Serialize)]
struct SendGridRequest {
    personalizations: Vec<SendGridPersonalization>,
    from: SendGridEmail,
    subject: String,
    content: Vec<SendGridContent>,
}

#[derive(Debug, Serialize)]
struct SendGridPersonalization {
    to: Vec<SendGridEmail>,
}

#[derive(Debug, Serialize)]
struct SendGridEmail {
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct SendGridContent {
    #[serde(rename = "type")]
    content_type: String,
    value: String,
}

/// Generic webhook request
#[derive(Debug, Serialize)]
struct WebhookRequest {
    to: String,
    from_email: String,
    from_name: String,
    subject: String,
    body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    body_html: Option<String>,
}

impl EmailService {
    pub fn new(settings_service: SettingsService) -> Self {
        Self { settings_service }
    }

    async fn get_value_fallback(&self, tenant_id: Option<&str>, key: &str) -> Option<String> {
        if let Some(tid) = tenant_id {
            if let Ok(Some(s)) = self.settings_service.get_value(Some(tid), key).await {
                if !s.trim().is_empty() {
                    return Some(s);
                }
            }
        }

        self.settings_service
            .get_value(None, key)
            .await
            .ok()
            .flatten()
    }

    /// Get email configuration from settings
    async fn get_config_for(&self, tenant_id: Option<&str>) -> AppResult<EmailConfig> {
        let provider = self
            .get_value_fallback(tenant_id, "email_provider")
            .await
            .unwrap_or_else(|| "resend".to_string());
        let from_email = self
            .get_value_fallback(tenant_id, "email_from_address")
            .await
            .unwrap_or_else(|| "noreply@example.com".to_string());
        let from_name = self
            .get_value_fallback(tenant_id, "email_from_name")
            .await
            .unwrap_or_else(|| "System".to_string());
        let api_key = self
            .get_value_fallback(tenant_id, "email_api_key")
            .await
            .unwrap_or_default();
        let webhook_url = self
            .get_value_fallback(tenant_id, "email_webhook_url")
            .await
            .unwrap_or_default();

        // SMTP fields (stored for future use)
        let smtp_host = self
            .get_value_fallback(tenant_id, "email_smtp_host")
            .await
            .unwrap_or_default();
        let smtp_port_str = self
            .get_value_fallback(tenant_id, "email_smtp_port")
            .await
            .unwrap_or_else(|| "587".to_string());
        let smtp_username = self
            .get_value_fallback(tenant_id, "email_smtp_username")
            .await
            .unwrap_or_default();
        let smtp_password = self
            .get_value_fallback(tenant_id, "email_smtp_password")
            .await
            .unwrap_or_default();
        let smtp_encryption = self
            .get_value_fallback(tenant_id, "email_smtp_encryption")
            .await
            .unwrap_or_else(|| "starttls".to_string());

        Ok(EmailConfig {
            provider,
            from_email,
            from_name,
            api_key,
            webhook_url,
            smtp_host,
            smtp_port: smtp_port_str.parse().unwrap_or(587),
            smtp_username,
            smtp_password,
            smtp_encryption,
        })
    }

    /// Send email via configured provider
    pub async fn send_email(&self, to: &str, subject: &str, body: &str) -> AppResult<()> {
        self.send_email_for_tenant(None, to, subject, body).await
    }

    /// Send email using tenant-scoped settings (falls back to global settings).
    pub async fn send_email_for_tenant(
        &self,
        tenant_id: Option<&str>,
        to: &str,
        subject: &str,
        body: &str,
    ) -> AppResult<()> {
        let config = self.get_config_for(tenant_id).await?;

        info!("Sending email to {} via {}", to, config.provider);

        match config.provider.as_str() {
            "resend" => self.send_via_resend(&config, to, subject, body, None).await,
            "smtp" => self.send_via_smtp(&config, to, subject, body).await,
            "sendgrid" => {
                self.send_via_sendgrid(&config, to, subject, body, None)
                    .await
            }
            "webhook" => {
                self.send_via_webhook(&config, to, subject, body, None)
                    .await
            }
            _ => Err(AppError::Validation(format!(
                "Unknown email provider: {}",
                config.provider
            ))),
        }
    }

    pub async fn send_email_with_optional_html_for_tenant(
        &self,
        tenant_id: Option<&str>,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: Option<&str>,
    ) -> AppResult<()> {
        if let Some(html) = body_html {
            self.send_email_with_html_for_tenant(tenant_id, to, subject, body_text, html)
                .await
        } else {
            self.send_email_for_tenant(tenant_id, to, subject, body_text)
                .await
        }
    }

    pub async fn send_email_with_html_for_tenant(
        &self,
        tenant_id: Option<&str>,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: &str,
    ) -> AppResult<()> {
        let config = self.get_config_for(tenant_id).await?;
        info!("Sending email to {} via {}", to, config.provider);

        match config.provider.as_str() {
            "resend" => {
                self.send_via_resend(&config, to, subject, body_text, Some(body_html.to_string()))
                    .await
            }
            "smtp" => {
                self.send_via_smtp_html(&config, to, subject, body_text, body_html)
                    .await
            }
            "sendgrid" => {
                self.send_via_sendgrid(&config, to, subject, body_text, Some(body_html.to_string()))
                    .await
            }
            "webhook" => {
                self.send_via_webhook(&config, to, subject, body_text, Some(body_html.to_string()))
                    .await
            }
            _ => Err(AppError::Validation(format!(
                "Unknown email provider: {}",
                config.provider
            ))),
        }
    }

    pub async fn test_smtp_connection_for_tenant(
        &self,
        tenant_id: Option<&str>,
    ) -> AppResult<SmtpConnectionTestResult> {
        let config = self.get_config_for(tenant_id).await?;
        if config.provider != "smtp" {
            return Err(AppError::Validation(
                "Email provider is not SMTP (set provider to smtp to test connection)".to_string(),
            ));
        }
        let mailer = self.build_smtp_transport(&config)?;
        let start = Instant::now();
        match mailer.test_connection().await {
            Ok(true) | Ok(false) => Ok(SmtpConnectionTestResult {
                ok: true,
                provider: config.provider,
                host: config.smtp_host,
                port: config.smtp_port,
                encryption: config.smtp_encryption,
                duration_ms: start.elapsed().as_millis() as i64,
                message: "SMTP connection verified".to_string(),
            }),
            Err(e) => Err(AppError::Internal(format!(
                "SMTP connection test failed ({}:{} / {}): {}",
                config.smtp_host, config.smtp_port, config.smtp_encryption, e
            ))),
        }
    }

    fn build_smtp_transport(
        &self,
        config: &EmailConfig,
    ) -> AppResult<AsyncSmtpTransport<Tokio1Executor>> {
        let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());

        let mailer_builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
            .map_err(|e| AppError::Validation(format!("Invalid SMTP host: {}", e)))?
            .port(config.smtp_port)
            .credentials(creds);

        let mailer = match config.smtp_encryption.as_str() {
            "ssl" => mailer_builder
                .tls(Tls::Wrapper(
                    TlsParameters::new(config.smtp_host.clone())
                        .map_err(|e| AppError::Internal(format!("TLS error: {}", e)))?,
                ))
                .build(),
            "starttls" | "tls" => mailer_builder
                .tls(Tls::Required(
                    TlsParameters::new(config.smtp_host.clone())
                        .map_err(|e| AppError::Internal(format!("TLS error: {}", e)))?,
                ))
                .build(),
            _ => mailer_builder.tls(Tls::None).build(),
        };

        Ok(mailer)
    }

    /// Send via SMTP
    async fn send_via_smtp(
        &self,
        config: &EmailConfig,
        to: &str,
        subject: &str,
        body: &str,
    ) -> AppResult<()> {
        let email = Message::builder()
            .from(
                format!("{} <{}>", config.from_name, config.from_email)
                    .parse()
                    .map_err(|e| AppError::Validation(format!("Invalid from address: {}", e)))?,
            )
            .to(to
                .parse()
                .map_err(|e| AppError::Validation(format!("Invalid to address: {}", e)))?)
            .subject(subject)
            .body(body.to_string())
            .map_err(|e| AppError::Internal(format!("Failed to build email: {}", e)))?;

        let mailer = self.build_smtp_transport(config)?;

        mailer
            .send(email)
            .await
            .map_err(|e| AppError::Internal(format!("SMTP sending failed: {}", e)))?;

        info!("Email sent via SMTP");
        Ok(())
    }

    async fn send_via_smtp_html(
        &self,
        config: &EmailConfig,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: &str,
    ) -> AppResult<()> {
        let builder = Message::builder()
            .from(
                format!("{} <{}>", config.from_name, config.from_email)
                    .parse()
                    .map_err(|e| AppError::Validation(format!("Invalid from address: {}", e)))?,
            )
            .to(to
                .parse()
                .map_err(|e| AppError::Validation(format!("Invalid to address: {}", e)))?)
            .subject(subject);

        let multipart = MultiPart::alternative()
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_PLAIN)
                    .body(body_text.to_string()),
            )
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(body_html.to_string()),
            );

        let email = builder
            .multipart(multipart)
            .map_err(|e| AppError::Internal(format!("Failed to build email: {}", e)))?;

        let mailer = self.build_smtp_transport(config)?;
        mailer
            .send(email)
            .await
            .map_err(|e| AppError::Internal(format!("SMTP sending failed: {}", e)))?;

        info!("Email sent via SMTP");
        Ok(())
    }

    /// Send via Resend API
    async fn send_via_resend(
        &self,
        config: &EmailConfig,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: Option<String>,
    ) -> AppResult<()> {
        if config.api_key.is_empty() {
            return Err(AppError::Validation(
                "Resend API key not configured".to_string(),
            ));
        }

        let client = reqwest::Client::new();
        let request = ResendRequest {
            from: format!("{} <{}>", config.from_name, config.from_email),
            to: vec![to.to_string()],
            subject: subject.to_string(),
            text: body_text.to_string(),
            html: body_html,
        };

        let response = client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", config.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("Resend error: {}", err)));
        }

        info!("Email sent via Resend");
        Ok(())
    }

    /// Send via SendGrid API
    async fn send_via_sendgrid(
        &self,
        config: &EmailConfig,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: Option<String>,
    ) -> AppResult<()> {
        if config.api_key.is_empty() {
            return Err(AppError::Validation(
                "SendGrid API key not configured".to_string(),
            ));
        }

        let client = reqwest::Client::new();
        let mut content = vec![SendGridContent {
            content_type: "text/plain".to_string(),
            value: body_text.to_string(),
        }];
        if let Some(html) = body_html {
            content.push(SendGridContent {
                content_type: "text/html".to_string(),
                value: html,
            });
        }

        let request = SendGridRequest {
            personalizations: vec![SendGridPersonalization {
                to: vec![SendGridEmail {
                    email: to.to_string(),
                    name: None,
                }],
            }],
            from: SendGridEmail {
                email: config.from_email.clone(),
                name: Some(config.from_name.clone()),
            },
            subject: subject.to_string(),
            content,
        };

        let response = client
            .post("https://api.sendgrid.com/v3/mail/send")
            .header("Authorization", format!("Bearer {}", config.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("SendGrid error: {}", err)));
        }

        info!("Email sent via SendGrid");
        Ok(())
    }

    /// Send via Webhook
    async fn send_via_webhook(
        &self,
        config: &EmailConfig,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: Option<String>,
    ) -> AppResult<()> {
        if config.webhook_url.is_empty() {
            return Err(AppError::Validation(
                "Webhook URL not configured".to_string(),
            ));
        }

        let client = reqwest::Client::new();
        let request = WebhookRequest {
            to: to.to_string(),
            from_email: config.from_email.clone(),
            from_name: config.from_name.clone(),
            subject: subject.to_string(),
            body: body_text.to_string(),
            body_html,
        };

        let response = client
            .post(&config.webhook_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("Webhook error: {}", err)));
        }

        info!("Email sent via Webhook");
        Ok(())
    }

    /// Send a test email
    pub async fn send_test_email(&self, to: &str) -> AppResult<()> {
        self.send_email(
            to,
            "Test Email - Configuration Verified",
            "Hello!\n\nThis is a test email. Your email configuration is working correctly.\n\nBest regards,\nYour Application",
        ).await
    }

    // ---------------- attachments ----------------

    /// Send an email with binary attachments through the tenant-configured provider.
    ///
    /// Phase 2 of bulk-send-invoice. Backward-compatible: when `attachments`
    /// is empty, falls back to `send_email_with_optional_html_for_tenant`.
    pub async fn send_email_with_attachments_for_tenant(
        &self,
        tenant_id: Option<&str>,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: Option<&str>,
        attachments: &[EmailAttachment],
    ) -> AppResult<()> {
        if attachments.is_empty() {
            return self
                .send_email_with_optional_html_for_tenant(
                    tenant_id, to, subject, body_text, body_html,
                )
                .await;
        }

        let config = self.get_config_for(tenant_id).await?;
        info!(
            "Sending email to {} via {} with {} attachment(s)",
            to,
            config.provider,
            attachments.len()
        );

        match config.provider.as_str() {
            "resend" => {
                self.send_via_resend_with_attachments(
                    &config,
                    to,
                    subject,
                    body_text,
                    body_html,
                    attachments,
                )
                .await
            }
            "smtp" => {
                self.send_via_smtp_with_attachments(
                    &config,
                    to,
                    subject,
                    body_text,
                    body_html,
                    attachments,
                )
                .await
            }
            "sendgrid" => {
                self.send_via_sendgrid_with_attachments(
                    &config,
                    to,
                    subject,
                    body_text,
                    body_html,
                    attachments,
                )
                .await
            }
            "webhook" => {
                self.send_via_webhook_with_attachments(
                    &config,
                    to,
                    subject,
                    body_text,
                    body_html,
                    attachments,
                )
                .await
            }
            _ => Err(AppError::Validation(format!(
                "Unknown email provider: {}",
                config.provider
            ))),
        }
    }

    /// SMTP with attachments — uses lettre's `MultiPart::mixed` with an
    /// alternative text/html part plus one binary part per attachment.
    async fn send_via_smtp_with_attachments(
        &self,
        config: &EmailConfig,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: Option<&str>,
        attachments: &[EmailAttachment],
    ) -> AppResult<()> {
        let builder = Message::builder()
            .from(
                format!("{} <{}>", config.from_name, config.from_email)
                    .parse()
                    .map_err(|e| AppError::Validation(format!("Invalid from address: {}", e)))?,
            )
            .to(to
                .parse()
                .map_err(|e| AppError::Validation(format!("Invalid to address: {}", e)))?)
            .subject(subject);

        // Body part: alternative(text, html) when html present; otherwise plain.
        let body_part = if let Some(html) = body_html {
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(body_text.to_string()),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html.to_string()),
                )
        } else {
            MultiPart::alternative().singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_PLAIN)
                    .body(body_text.to_string()),
            )
        };

        let mut mixed = MultiPart::mixed().multipart(body_part);
        for att in attachments {
            let ct = att.content_type.parse::<ContentType>().map_err(|e| {
                AppError::Validation(format!(
                    "Invalid attachment content_type '{}': {}",
                    att.content_type, e
                ))
            })?;
            mixed = mixed
                .singlepart(Attachment::new(att.filename.clone()).body(att.content.clone(), ct));
        }

        let email = builder
            .multipart(mixed)
            .map_err(|e| AppError::Internal(format!("Failed to build email: {}", e)))?;

        let mailer = self.build_smtp_transport(config)?;
        mailer
            .send(email)
            .await
            .map_err(|e| AppError::Internal(format!("SMTP sending failed: {}", e)))?;

        info!("Email sent via SMTP (with attachments)");
        Ok(())
    }

    /// Resend with attachments — Resend accepts `attachments: [{filename, content}]`
    /// where `content` is a base64-encoded string.
    async fn send_via_resend_with_attachments(
        &self,
        config: &EmailConfig,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: Option<&str>,
        attachments: &[EmailAttachment],
    ) -> AppResult<()> {
        if config.api_key.is_empty() {
            return Err(AppError::Validation(
                "Resend API key not configured".to_string(),
            ));
        }

        #[derive(Serialize)]
        struct ResendAttachment {
            filename: String,
            content: String, // base64
            #[serde(rename = "content_type", skip_serializing_if = "Option::is_none")]
            content_type: Option<String>,
        }
        #[derive(Serialize)]
        struct ResendRequestWithAttachments<'a> {
            from: String,
            to: Vec<String>,
            subject: &'a str,
            text: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            html: Option<&'a str>,
            attachments: Vec<ResendAttachment>,
        }

        let b64 = base64::engine::general_purpose::STANDARD;
        let req = ResendRequestWithAttachments {
            from: format!("{} <{}>", config.from_name, config.from_email),
            to: vec![to.to_string()],
            subject,
            text: body_text,
            html: body_html,
            attachments: attachments
                .iter()
                .map(|a| ResendAttachment {
                    filename: a.filename.clone(),
                    content: b64.encode(&a.content),
                    content_type: Some(a.content_type.clone()),
                })
                .collect(),
        };

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", config.api_key))
            .json(&req)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("Resend error: {}", err)));
        }

        info!("Email sent via Resend (with attachments)");
        Ok(())
    }

    /// SendGrid v3 with attachments.
    async fn send_via_sendgrid_with_attachments(
        &self,
        config: &EmailConfig,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: Option<&str>,
        attachments: &[EmailAttachment],
    ) -> AppResult<()> {
        if config.api_key.is_empty() {
            return Err(AppError::Validation(
                "SendGrid API key not configured".to_string(),
            ));
        }

        #[derive(Serialize)]
        struct SgAttachment {
            content: String, // base64
            #[serde(rename = "type")]
            kind: String,
            filename: String,
            disposition: &'static str,
        }
        #[derive(Serialize)]
        struct SgRequest<'a> {
            personalizations: Vec<SendGridPersonalization>,
            from: SendGridEmail,
            subject: &'a str,
            content: Vec<SendGridContent>,
            attachments: Vec<SgAttachment>,
        }

        let mut content = vec![SendGridContent {
            content_type: "text/plain".to_string(),
            value: body_text.to_string(),
        }];
        if let Some(html) = body_html {
            content.push(SendGridContent {
                content_type: "text/html".to_string(),
                value: html.to_string(),
            });
        }

        let b64 = base64::engine::general_purpose::STANDARD;
        let req = SgRequest {
            personalizations: vec![SendGridPersonalization {
                to: vec![SendGridEmail {
                    email: to.to_string(),
                    name: None,
                }],
            }],
            from: SendGridEmail {
                email: config.from_email.clone(),
                name: Some(config.from_name.clone()),
            },
            subject,
            content,
            attachments: attachments
                .iter()
                .map(|a| SgAttachment {
                    content: b64.encode(&a.content),
                    kind: a.content_type.clone(),
                    filename: a.filename.clone(),
                    disposition: "attachment",
                })
                .collect(),
        };

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.sendgrid.com/v3/mail/send")
            .header("Authorization", format!("Bearer {}", config.api_key))
            .json(&req)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("SendGrid error: {}", err)));
        }

        info!("Email sent via SendGrid (with attachments)");
        Ok(())
    }

    /// Webhook with attachments — passes a JSON payload with base64-encoded
    /// attachments in `attachments: [{filename, content_type, content_base64}]`.
    async fn send_via_webhook_with_attachments(
        &self,
        config: &EmailConfig,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: Option<&str>,
        attachments: &[EmailAttachment],
    ) -> AppResult<()> {
        if config.webhook_url.is_empty() {
            return Err(AppError::Validation(
                "Webhook URL not configured".to_string(),
            ));
        }

        #[derive(Serialize)]
        struct WhAttachment {
            filename: String,
            content_type: String,
            content_base64: String,
        }
        #[derive(Serialize)]
        struct WhRequest<'a> {
            to: &'a str,
            from_email: &'a str,
            from_name: &'a str,
            subject: &'a str,
            body: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            body_html: Option<&'a str>,
            attachments: Vec<WhAttachment>,
        }

        let b64 = base64::engine::general_purpose::STANDARD;
        let req = WhRequest {
            to,
            from_email: &config.from_email,
            from_name: &config.from_name,
            subject,
            body: body_text,
            body_html,
            attachments: attachments
                .iter()
                .map(|a| WhAttachment {
                    filename: a.filename.clone(),
                    content_type: a.content_type.clone(),
                    content_base64: b64.encode(&a.content),
                })
                .collect(),
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&config.webhook_url)
            .json(&req)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("Webhook error: {}", err)));
        }

        info!("Email sent via Webhook (with attachments)");
        Ok(())
    }
}
