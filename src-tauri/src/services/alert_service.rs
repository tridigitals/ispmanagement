//! Alert Service - Error and Performance Alerting via Email
//!
//! Monitors metrics and sends email alerts when thresholds are exceeded.

use crate::error::AppResult;
use crate::services::email_service::EmailService;
use crate::services::metrics_service::RequestMetrics;
use crate::services::settings_service::SettingsService;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Alert types for cooldown tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertType {
    HighErrorRate,
    RateLimitSpike,
    SlowResponse,
}

impl AlertType {
    fn as_str(&self) -> &'static str {
        match self {
            AlertType::HighErrorRate => "high_error_rate",
            AlertType::RateLimitSpike => "rate_limit_spike",
            AlertType::SlowResponse => "slow_response",
        }
    }

    fn subject(&self) -> &'static str {
        match self {
            AlertType::HighErrorRate => "⚠️ High Error Rate Alert",
            AlertType::RateLimitSpike => "🛡️ Rate Limiting Spike Detected",
            AlertType::SlowResponse => "🐢 Slow Response Time Alert",
        }
    }
}

/// Alert service for monitoring and email notifications
#[derive(Clone)]
pub struct AlertService {
    email_service: EmailService,
    settings_service: SettingsService,
    /// Track last alert time per type to implement cooldown
    last_alert_times: std::sync::Arc<RwLock<HashMap<String, Instant>>>,
    /// Previous metrics snapshot for calculating rates
    #[allow(dead_code)]
    previous_metrics: std::sync::Arc<RwLock<Option<MetricsSnapshot>>>,
}

/// Snapshot of metrics for rate calculation
#[derive(Debug, Clone)]
struct MetricsSnapshot {
    total_requests: u64,
    error_count: u64,
    rate_limited_count: u64,
    #[allow(dead_code)]
    timestamp: Instant,
}

impl AlertService {
    /// Create a new alert service
    pub fn new(email_service: EmailService, settings_service: SettingsService) -> Self {
        Self {
            email_service,
            settings_service,
            last_alert_times: std::sync::Arc::new(RwLock::new(HashMap::new())),
            previous_metrics: std::sync::Arc::new(RwLock::new(None)),
        }
    }

    /// Helper to get setting value with default
    async fn get_setting(&self, key: &str, default: &str) -> String {
        self.settings_service
            .get_value(None, key)
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| default.to_string())
    }

    /// Check metrics and send alerts if thresholds exceeded
    pub async fn check_and_alert(&self, metrics: &RequestMetrics) {
        // Check if alerting is enabled
        let enabled = self.get_setting("alerting_enabled", "false").await;

        if enabled != "true" {
            return;
        }

        // Get thresholds from settings
        let error_threshold: f64 = self
            .get_setting("alerting_error_threshold", "5.0")
            .await
            .parse()
            .unwrap_or(5.0);

        let rate_limit_threshold: u64 = self
            .get_setting("alerting_rate_limit_threshold", "50")
            .await
            .parse()
            .unwrap_or(50);

        let response_time_threshold: f64 = self
            .get_setting("alerting_response_time_threshold", "3000.0")
            .await
            .parse()
            .unwrap_or(3000.0);

        // Calculate error rate
        let error_rate = if metrics.total_requests > 0 {
            (metrics.error_count as f64 / metrics.total_requests as f64) * 100.0
        } else {
            0.0
        };

        // Check High Error Rate
        if error_rate > error_threshold && metrics.total_requests > 10 {
            self.maybe_send_alert(
                AlertType::HighErrorRate,
                &format!(
                    "Error rate is {:.1}% (threshold: {:.1}%)\n\n\
                    Total Requests: {}\n\
                    Errors: {}\n\
                    Rate Limited: {}\n\n\
                    Please investigate the application logs.",
                    error_rate,
                    error_threshold,
                    metrics.total_requests,
                    metrics.error_count,
                    metrics.rate_limited_count
                ),
            )
            .await;
        }

        // Check Rate Limit Spike
        if metrics.rate_limited_count > rate_limit_threshold {
            self.maybe_send_alert(
                AlertType::RateLimitSpike,
                &format!(
                    "Rate limiting spike detected!\n\n\
                    Rate Limited Requests: {}\n\
                    Threshold: {}\n\n\
                    This may indicate a DDoS attack or misconfigured client.",
                    metrics.rate_limited_count, rate_limit_threshold
                ),
            )
            .await;
        }

        // Check Slow Response Time
        if metrics.p95_response_time_ms > response_time_threshold {
            self.maybe_send_alert(
                AlertType::SlowResponse,
                &format!(
                    "P95 response time is {:.0}ms (threshold: {:.0}ms)\n\n\
                    Average: {:.0}ms\n\
                    Min: {:.0}ms\n\
                    Max: {:.0}ms\n\n\
                    Consider optimizing database queries or scaling resources.",
                    metrics.p95_response_time_ms,
                    response_time_threshold,
                    metrics.avg_response_time_ms,
                    metrics.min_response_time_ms,
                    metrics.max_response_time_ms
                ),
            )
            .await;
        }

        // Store current metrics snapshot
        let snapshot = MetricsSnapshot {
            total_requests: metrics.total_requests,
            error_count: metrics.error_count,
            rate_limited_count: metrics.rate_limited_count,
            timestamp: Instant::now(),
        };
        *self.previous_metrics.write().unwrap() = Some(snapshot);
    }

    /// Send alert if cooldown period has passed
    async fn maybe_send_alert(&self, alert_type: AlertType, body: &str) {
        let cooldown_minutes: u64 = self
            .get_setting("alerting_cooldown_minutes", "15")
            .await
            .parse()
            .unwrap_or(15);

        let cooldown = Duration::from_secs(cooldown_minutes * 60);
        let key = alert_type.as_str().to_string();

        // Check cooldown
        {
            let times = self.last_alert_times.read().unwrap();
            if let Some(last_time) = times.get(&key) {
                if last_time.elapsed() < cooldown {
                    return; // Still in cooldown
                }
            }
        }

        // Send the alert
        if let Err(e) = self.send_alert(alert_type.subject(), body).await {
            warn!("Failed to send alert email: {}", e);
        } else {
            info!("Alert sent: {}", alert_type.subject());
            // Update last alert time
            let mut times = self.last_alert_times.write().unwrap();
            times.insert(key, Instant::now());
        }
    }

    /// Send alert email to configured recipient
    async fn send_alert(&self, subject: &str, body: &str) -> AppResult<()> {
        let recipient = self.get_setting("alerting_email", "").await;

        if recipient.is_empty() {
            warn!("Alerting email not configured, skipping alert");
            return Ok(());
        }

        let app_name = self.get_setting("app_name", "SaaS App").await;

        let full_subject = format!("[{}] {}", app_name, subject);
        let full_body = format!(
            "{}\n\n---\nThis is an automated alert from {}.\nTimestamp: {}",
            body,
            app_name,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        self.email_service
            .send_email(&recipient, &full_subject, &full_body)
            .await
    }
}
