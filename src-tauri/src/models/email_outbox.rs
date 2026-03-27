use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmailOutboxItem {
    pub id: String,
    pub tenant_id: Option<String>,
    pub to_email: String,
    pub subject: String,
    pub body: String,
    pub body_html: Option<String>,
    pub status: String, // queued | sending | sent | failed
    pub attempts: i32,
    pub max_attempts: i32,
    pub scheduled_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    #[sqlx(default)]
    pub last_attempted_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[sqlx(default)]
    pub next_retry_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[sqlx(default)]
    pub retryable: bool,
    #[serde(default)]
    #[sqlx(default)]
    pub delivery_status_summary: String,
}

impl EmailOutboxItem {
    pub fn with_retry_visibility(mut self) -> Self {
        self.last_attempted_at = self.derive_last_attempted_at();
        self.next_retry_at = self.derive_next_retry_at();
        self.retryable = self.derive_retryable();
        self.delivery_status_summary = self.derive_delivery_status_summary();
        self
    }

    fn derive_last_attempted_at(&self) -> Option<DateTime<Utc>> {
        if self.attempts <= 0 {
            return None;
        }

        match self.status.as_str() {
            "queued" => self.last_error.as_ref().map(|_| self.updated_at),
            "sending" | "sent" | "failed" => Some(self.updated_at),
            _ => Some(self.updated_at),
        }
    }

    fn derive_next_retry_at(&self) -> Option<DateTime<Utc>> {
        if self.status == "queued" && self.attempts > 0 && self.last_error.is_some() && self.retryable {
            Some(self.scheduled_at)
        } else {
            None
        }
    }

    fn derive_retryable(&self) -> bool {
        matches!(self.status.as_str(), "queued" | "failed") && self.attempts < self.max_attempts
    }

    fn derive_delivery_status_summary(&self) -> String {
        match self.status.as_str() {
            "sent" => format!("Delivered after {} attempt{}", self.attempts.max(1), if self.attempts.max(1) == 1 { "" } else { "s" }),
            "sending" => format!("Sending (attempt {} of {})", self.attempts.max(1), self.max_attempts.max(1)),
            "failed" if self.retryable => format!(
                "Failed on attempt {} of {}; retry available",
                self.attempts.max(1),
                self.max_attempts.max(1)
            ),
            "failed" => format!(
                "Failed after {} of {} attempts",
                self.attempts.max(1),
                self.max_attempts.max(1)
            ),
            "queued" if self.attempts > 0 && self.last_error.is_some() => format!(
                "Retry scheduled after failed attempt {} of {}",
                self.attempts,
                self.max_attempts.max(1)
            ),
            "queued" => format!("Queued for initial delivery (0 of {} attempts used)", self.max_attempts.max(1)),
            other => format!("{} ({} of {} attempts used)", other, self.attempts.max(0), self.max_attempts.max(1)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailOutboxStats {
    pub all: i64,
    pub queued: i64,
    pub sending: i64,
    pub sent: i64,
    pub failed: i64,
}

#[cfg(test)]
mod tests {
    use super::EmailOutboxItem;
    use chrono::{Duration, Utc};

    fn sample_item(status: &str, attempts: i32, max_attempts: i32, has_error: bool) -> EmailOutboxItem {
        let now = Utc::now();
        EmailOutboxItem {
            id: "outbox-1".to_string(),
            tenant_id: Some("tenant-1".to_string()),
            to_email: "ops@example.com".to_string(),
            subject: "Subject".to_string(),
            body: "Body".to_string(),
            body_html: None,
            status: status.to_string(),
            attempts,
            max_attempts,
            scheduled_at: now + Duration::minutes(5),
            last_error: has_error.then(|| "smtp timeout".to_string()),
            sent_at: None,
            created_at: now - Duration::minutes(10),
            updated_at: now,
            last_attempted_at: None,
            next_retry_at: None,
            retryable: false,
            delivery_status_summary: String::new(),
        }
    }

    #[test]
    fn derives_retry_visibility_for_retry_scheduled_item() {
        let item = sample_item("queued", 2, 5, true).with_retry_visibility();

        assert_eq!(item.last_attempted_at, Some(item.updated_at));
        assert_eq!(item.next_retry_at, Some(item.scheduled_at));
        assert!(item.retryable);
        assert_eq!(item.delivery_status_summary, "Retry scheduled after failed attempt 2 of 5");
    }

    #[test]
    fn derives_terminal_failure_visibility() {
        let item = sample_item("failed", 5, 5, true).with_retry_visibility();

        assert_eq!(item.last_attempted_at, Some(item.updated_at));
        assert_eq!(item.next_retry_at, None);
        assert!(!item.retryable);
        assert_eq!(item.delivery_status_summary, "Failed after 5 of 5 attempts");
    }

    #[test]
    fn derives_sent_visibility() {
        let item = sample_item("sent", 1, 5, false).with_retry_visibility();

        assert_eq!(item.last_attempted_at, Some(item.updated_at));
        assert_eq!(item.next_retry_at, None);
        assert!(!item.retryable);
        assert_eq!(item.delivery_status_summary, "Delivered after 1 attempt");
    }
}
