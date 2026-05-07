use super::{models::RadiusRequestContext, RadiusNasClient, RadiusRepository};
use crate::error::AppResult;
use crate::models::{RadiusAccountingSession, RadiusAccountingStatusType};
use chrono::{DateTime, Utc};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

pub trait RadiusAccountingRepository {
    fn resolve_nas_client<'a>(
        &'a self,
        context: &'a RadiusRequestContext,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<RadiusNasClient>>> + Send + 'a>>;

    fn upsert_accounting_session<'a>(
        &'a self,
        session: &'a RadiusAccountingSession,
    ) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + 'a>>;
}

impl RadiusAccountingRepository for RadiusRepository {
    fn resolve_nas_client<'a>(
        &'a self,
        context: &'a RadiusRequestContext,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<RadiusNasClient>>> + Send + 'a>> {
        Box::pin(async move { self.resolve_nas_client(context).await })
    }

    fn upsert_accounting_session<'a>(
        &'a self,
        session: &'a RadiusAccountingSession,
    ) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + 'a>> {
        Box::pin(async move { self.upsert_accounting_session(session).await })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusAccountingRequest {
    pub source_ip: String,
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
    pub occurred_at: Option<DateTime<Utc>>,
    pub raw_attributes_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusAccountingResult {
    pub acknowledged: bool,
    pub rejection_reason: Option<String>,
    pub session: Option<RadiusAccountingSession>,
}

pub struct RadiusAccountingService<R> {
    repository: R,
}

impl<R> RadiusAccountingService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R> RadiusAccountingService<R>
where
    R: RadiusAccountingRepository,
{
    pub async fn handle_accounting(
        &self,
        request: &RadiusAccountingRequest,
    ) -> AppResult<RadiusAccountingResult> {
        let context = RadiusRequestContext {
            source_ip: request.source_ip.clone(),
        };
        let Some(nas) = self.repository.resolve_nas_client(&context).await? else {
            return Ok(RadiusAccountingResult {
                acknowledged: false,
                rejection_reason: Some("unknown_nas".to_string()),
                session: None,
            });
        };

        let now = request.occurred_at.unwrap_or_else(Utc::now);
        let (started_at, last_update_at, ended_at) = status_timestamps(&request.status_type, now);

        let session = RadiusAccountingSession {
            id: Uuid::new_v4().to_string(),
            tenant_id: nas.tenant_id,
            router_id: nas.router_id,
            nas_ip_address: Some(request.source_ip.clone()),
            nas_ip_or_cidr: Some(nas.nas_ip_or_cidr),
            username: request.username.clone(),
            radius_identity: normalize_optional(request.radius_identity.as_deref()),
            acct_session_id: request.acct_session_id.clone(),
            status_type: request.status_type.clone(),
            framed_ip_address: normalize_optional(request.framed_ip_address.as_deref()),
            calling_station_id: normalize_optional(request.calling_station_id.as_deref()),
            session_time_seconds: request.session_time_seconds,
            input_octets: request.input_octets,
            output_octets: request.output_octets,
            terminate_cause: normalize_optional(request.terminate_cause.as_deref()),
            started_at,
            last_update_at,
            ended_at,
            raw_attributes_json: normalize_optional(request.raw_attributes_json.as_deref()),
            created_at: now,
            updated_at: now,
        };

        self.repository.upsert_accounting_session(&session).await?;

        Ok(RadiusAccountingResult {
            acknowledged: true,
            rejection_reason: None,
            session: Some(session),
        })
    }
}

fn status_timestamps(
    status_type: &RadiusAccountingStatusType,
    occurred_at: DateTime<Utc>,
) -> (
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
) {
    match status_type {
        RadiusAccountingStatusType::Start => (Some(occurred_at), Some(occurred_at), None),
        RadiusAccountingStatusType::InterimUpdate => (None, Some(occurred_at), None),
        RadiusAccountingStatusType::Stop => (None, Some(occurred_at), Some(occurred_at)),
        RadiusAccountingStatusType::AccountingOn | RadiusAccountingStatusType::AccountingOff => {
            (None, Some(occurred_at), None)
        }
    }
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{RadiusAccountingRepository, RadiusAccountingRequest, RadiusAccountingService};
    use crate::error::AppResult;
    use crate::models::{RadiusAccountingSession, RadiusAccountingStatusType};
    use crate::services::radius_service::RadiusNasClient;
    use chrono::{TimeZone, Utc};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct FakeRadiusAccountingRepository {
        nas: Option<RadiusNasClient>,
        sessions: Arc<Mutex<Vec<RadiusAccountingSession>>>,
    }

    impl RadiusAccountingRepository for FakeRadiusAccountingRepository {
        fn resolve_nas_client<'a>(
            &'a self,
            _context: &'a crate::services::radius_service::models::RadiusRequestContext,
        ) -> Pin<Box<dyn Future<Output = AppResult<Option<RadiusNasClient>>> + Send + 'a>> {
            Box::pin(async move { Ok(self.nas.clone()) })
        }

        fn upsert_accounting_session<'a>(
            &'a self,
            session: &'a RadiusAccountingSession,
        ) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + 'a>> {
            Box::pin(async move {
                self.sessions
                    .lock()
                    .expect("sessions mutex")
                    .push(session.clone());
                Ok(())
            })
        }
    }

    fn sample_nas() -> RadiusNasClient {
        RadiusNasClient {
            id: "nas-1".into(),
            tenant_id: "tenant-1".into(),
            router_id: "router-1".into(),
            nas_name: "NAS 1".into(),
            nas_ip_or_cidr: "203.0.113.10/32".into(),
            shortname: Some("router-1".into()),
            shared_secret: "shared-secret".into(),
        }
    }

    fn sample_request(status_type: RadiusAccountingStatusType) -> RadiusAccountingRequest {
        RadiusAccountingRequest {
            source_ip: "203.0.113.10".into(),
            username: "alice".into(),
            radius_identity: Some("alice@tenant-1".into()),
            acct_session_id: "session-1".into(),
            status_type,
            framed_ip_address: Some("10.10.10.2".into()),
            calling_station_id: Some("AA:BB:CC:DD:EE:FF".into()),
            session_time_seconds: Some(120),
            input_octets: Some(1024),
            output_octets: Some(2048),
            terminate_cause: Some("User-Request".into()),
            occurred_at: Some(Utc.with_ymd_and_hms(2026, 5, 5, 12, 0, 0).unwrap()),
            raw_attributes_json: Some("{\"Acct-Status-Type\":\"Start\"}".into()),
        }
    }

    #[tokio::test]
    async fn accounting_start_upserts_session_with_started_timestamp() {
        let repository = FakeRadiusAccountingRepository {
            nas: Some(sample_nas()),
            sessions: Arc::new(Mutex::new(Vec::new())),
        };
        let service = RadiusAccountingService::new(repository.clone());

        let result = service
            .handle_accounting(&sample_request(RadiusAccountingStatusType::Start))
            .await
            .expect("accounting start should succeed");

        assert!(result.acknowledged);
        let session = result.session.expect("session should exist");
        assert_eq!(
            session.started_at,
            sample_request(RadiusAccountingStatusType::Start).occurred_at
        );
        assert_eq!(
            session.last_update_at,
            sample_request(RadiusAccountingStatusType::Start).occurred_at
        );
        assert_eq!(session.ended_at, None);
        assert_eq!(repository.sessions.lock().expect("sessions mutex").len(), 1);
    }

    #[tokio::test]
    async fn accounting_interim_update_upserts_session_without_start_or_end_timestamp() {
        let repository = FakeRadiusAccountingRepository {
            nas: Some(sample_nas()),
            sessions: Arc::new(Mutex::new(Vec::new())),
        };
        let service = RadiusAccountingService::new(repository);

        let result = service
            .handle_accounting(&sample_request(RadiusAccountingStatusType::InterimUpdate))
            .await
            .expect("accounting interim should succeed");

        let session = result.session.expect("session should exist");
        assert_eq!(session.started_at, None);
        assert_eq!(
            session.last_update_at,
            sample_request(RadiusAccountingStatusType::InterimUpdate).occurred_at
        );
        assert_eq!(session.ended_at, None);
    }

    #[tokio::test]
    async fn accounting_stop_upserts_session_with_end_timestamp() {
        let repository = FakeRadiusAccountingRepository {
            nas: Some(sample_nas()),
            sessions: Arc::new(Mutex::new(Vec::new())),
        };
        let service = RadiusAccountingService::new(repository);

        let result = service
            .handle_accounting(&sample_request(RadiusAccountingStatusType::Stop))
            .await
            .expect("accounting stop should succeed");

        let session = result.session.expect("session should exist");
        assert_eq!(session.started_at, None);
        assert_eq!(
            session.ended_at,
            sample_request(RadiusAccountingStatusType::Stop).occurred_at
        );
    }

    #[tokio::test]
    async fn accounting_rejects_unknown_nas() {
        let service = RadiusAccountingService::new(FakeRadiusAccountingRepository::default());

        let result = service
            .handle_accounting(&sample_request(RadiusAccountingStatusType::Start))
            .await
            .expect("unknown nas should produce result");

        assert!(!result.acknowledged);
        assert_eq!(result.rejection_reason.as_deref(), Some("unknown_nas"));
        assert_eq!(result.session, None);
    }
}
