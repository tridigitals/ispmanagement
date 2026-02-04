//! Email Service - HTTP API only (SMTP disabled until Windows restart)
//!
//! Providers: Resend, SendGrid, Custom Webhook
//! Note: SMTP support requires uncommenting lettre in Cargo.toml after Windows restart

use crate::error::{AppError, AppResult};
use crate::services::SettingsService;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::Tls;
use lettre::transport::smtp::client::TlsParameters;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Serialize;
use tracing::info;

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
}

impl EmailService {
    pub fn new(settings_service: SettingsService) -> Self {
        Self { settings_service }
    }

    /// Get email configuration from settings
    async fn get_config(&self) -> AppResult<EmailConfig> {
        let provider = self
            .settings_service
            .get_value(None, "email_provider")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "resend".to_string());
        let from_email = self
            .settings_service
            .get_value(None, "email_from_address")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "noreply@example.com".to_string());
        let from_name = self
            .settings_service
            .get_value(None, "email_from_name")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "System".to_string());
        let api_key = self
            .settings_service
            .get_value(None, "email_api_key")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let webhook_url = self
            .settings_service
            .get_value(None, "email_webhook_url")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();

        // SMTP fields (stored for future use)
        let smtp_host = self
            .settings_service
            .get_value(None, "email_smtp_host")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let smtp_port_str = self
            .settings_service
            .get_value(None, "email_smtp_port")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "587".to_string());
        let smtp_username = self
            .settings_service
            .get_value(None, "email_smtp_username")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let smtp_password = self
            .settings_service
            .get_value(None, "email_smtp_password")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let smtp_encryption = self
            .settings_service
            .get_value(None, "email_smtp_encryption")
            .await
            .ok()
            .flatten()
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
        let config = self.get_config().await?;

        info!("Sending email to {} via {}", to, config.provider);

        match config.provider.as_str() {
            "resend" => self.send_via_resend(&config, to, subject, body).await,
            "smtp" => self.send_via_smtp(&config, to, subject, body).await,
            "sendgrid" => self.send_via_sendgrid(&config, to, subject, body).await,
            "webhook" => self.send_via_webhook(&config, to, subject, body).await,
            _ => Err(AppError::Validation(format!(
                "Unknown email provider: {}",
                config.provider
            ))),
        }
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

        let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());

        // Determine TLS security based on encryption setting
        // "tls" -> Wrapper/StartTLS (Port 587 usually)
        // "ssl" -> Implicit TLS (Port 465 usually)
        // "none" -> No encryption

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
        body: &str,
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
            text: body.to_string(),
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
        body: &str,
    ) -> AppResult<()> {
        if config.api_key.is_empty() {
            return Err(AppError::Validation(
                "SendGrid API key not configured".to_string(),
            ));
        }

        let client = reqwest::Client::new();
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
            content: vec![SendGridContent {
                content_type: "text/plain".to_string(),
                value: body.to_string(),
            }],
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
        body: &str,
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
            body: body.to_string(),
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
}
