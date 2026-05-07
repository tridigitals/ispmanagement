use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum RadiusAccountingStatusType {
    Start,
    Stop,
    InterimUpdate,
    AccountingOn,
    AccountingOff,
}

impl RadiusAccountingStatusType {
    pub fn from_radius_value(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "start" => Some(Self::Start),
            "stop" => Some(Self::Stop),
            "interim-update" | "interim_update" => Some(Self::InterimUpdate),
            "accounting-on" | "accounting_on" => Some(Self::AccountingOn),
            "accounting-off" | "accounting_off" => Some(Self::AccountingOff),
            _ => None,
        }
    }

    pub fn as_radius_value(&self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::Stop => "Stop",
            Self::InterimUpdate => "Interim-Update",
            Self::AccountingOn => "Accounting-On",
            Self::AccountingOff => "Accounting-Off",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct RadiusAccountingSession {
    pub id: String,
    pub tenant_id: String,
    pub router_id: String,
    pub nas_ip_address: Option<String>,
    pub nas_ip_or_cidr: Option<String>,
    pub username: String,
    pub radius_identity: Option<String>,
    pub acct_session_id: String,
    pub status_type: RadiusAccountingStatusType,
    pub framed_ip_address: Option<String>,
    pub calling_station_id: Option<String>,
    pub session_time_seconds: Option<i64>,
    pub input_octets: Option<i64>,
    pub output_octets: Option<i64>,
    pub terminate_cause: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub last_update_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub raw_attributes_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RadiusAccountingSession {
    pub fn display_identity(&self) -> &str {
        self.radius_identity
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(self.username.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct RadiusAuthLog {
    pub id: String,
    pub tenant_id: Option<String>,
    pub router_id: Option<String>,
    pub source_ip: String,
    pub username: Option<String>,
    pub radius_identity: Option<String>,
    pub outcome: String,
    pub reason: Option<String>,
    pub auth_type: Option<String>,
    pub latency_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
}

impl RadiusAuthLog {
    pub fn display_identity(&self) -> Option<&str> {
        self.radius_identity
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .or(self.username.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::{RadiusAccountingSession, RadiusAccountingStatusType, RadiusAuthLog};
    use chrono::Utc;

    #[test]
    fn radius_accounting_status_type_parses_radius_and_snake_case_values() {
        assert_eq!(
            RadiusAccountingStatusType::from_radius_value("Start"),
            Some(RadiusAccountingStatusType::Start)
        );
        assert_eq!(
            RadiusAccountingStatusType::from_radius_value("interim-update"),
            Some(RadiusAccountingStatusType::InterimUpdate)
        );
        assert_eq!(
            RadiusAccountingStatusType::from_radius_value("accounting_off"),
            Some(RadiusAccountingStatusType::AccountingOff)
        );
        assert_eq!(
            RadiusAccountingStatusType::from_radius_value("unknown"),
            None
        );
    }

    #[test]
    fn radius_accounting_session_prefers_radius_identity_for_display() {
        let session = RadiusAccountingSession {
            id: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            router_id: "router-1".to_string(),
            nas_ip_address: Some("10.0.0.1".to_string()),
            nas_ip_or_cidr: Some("10.0.0.1/32".to_string()),
            username: "pppoe-user".to_string(),
            radius_identity: Some("radius-user".to_string()),
            acct_session_id: "acct-1".to_string(),
            status_type: RadiusAccountingStatusType::Start,
            framed_ip_address: None,
            calling_station_id: None,
            session_time_seconds: None,
            input_octets: None,
            output_octets: None,
            terminate_cause: None,
            started_at: None,
            last_update_at: None,
            ended_at: None,
            raw_attributes_json: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(session.display_identity(), "radius-user");
    }

    #[test]
    fn radius_auth_log_falls_back_to_username_for_display_identity() {
        let log = RadiusAuthLog {
            id: "auth-1".to_string(),
            tenant_id: Some("tenant-1".to_string()),
            router_id: Some("router-1".to_string()),
            source_ip: "203.0.113.10".to_string(),
            username: Some("pppoe-user".to_string()),
            radius_identity: None,
            outcome: "accept".to_string(),
            reason: None,
            auth_type: Some("pap".to_string()),
            latency_ms: Some(5),
            created_at: Utc::now(),
        };

        assert_eq!(log.display_identity(), Some("pppoe-user"));
    }
}
