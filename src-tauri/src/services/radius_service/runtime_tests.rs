use super::{
    accounting::RadiusAccountingRepository,
    auth::RadiusAuthRepository,
    packet::{
        build_access_response, build_accounting_response, decode_request,
        extract_accounting_request, extract_chap_auth_request, extract_mschapv2_auth_request,
        extract_pap_auth_request,
    },
    ManagedRadiusRuntimeAccount, RadiusAccessDecision, RadiusAccountingService, RadiusAuthService,
    RadiusNasClient, RadiusPapAuthResult, RadiusReplyAttribute, RadiusReplyAttributes,
};
use crate::error::AppResult;
use crate::models::{PppoeAccount, PppoeAccountSource, RadiusAccountingSession, RadiusAuthLog};
use chrono::Utc;
use radius::core::avp::AVP;
use radius::core::code::Code;
use radius::core::packet::Packet;
use radius::core::{rfc2865, rfc2866, rfc2869};
use std::future::Future;
use std::net::Ipv4Addr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

const SHARED_SECRET: &[u8] = b"radius-secret";

#[derive(Clone, Default)]
struct FakeRuntimeRepository {
    nas: Option<RadiusNasClient>,
    accounts: Vec<ManagedRadiusRuntimeAccount>,
    auth_logs: Arc<Mutex<Vec<RadiusAuthLog>>>,
    accounting_sessions: Arc<Mutex<Vec<RadiusAccountingSession>>>,
}

impl RadiusAuthRepository for FakeRuntimeRepository {
    fn resolve_nas_client<'a>(
        &'a self,
        _context: &'a super::models::RadiusRequestContext,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<RadiusNasClient>>> + Send + 'a>> {
        Box::pin(async move { Ok(self.nas.clone()) })
    }

    fn load_managed_account<'a>(
        &'a self,
        tenant_id: &'a str,
        router_id: &'a str,
        username: &'a str,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<ManagedRadiusRuntimeAccount>>> + Send + 'a>>
    {
        Box::pin(async move {
            Ok(self
                .accounts
                .iter()
                .find(|account| {
                    account.account.tenant_id == tenant_id
                        && account.account.router_id == router_id
                        && account.account.username == username
                })
                .cloned())
        })
    }

    fn insert_auth_log<'a>(
        &'a self,
        auth_log: &'a RadiusAuthLog,
    ) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + 'a>> {
        Box::pin(async move {
            self.auth_logs
                .lock()
                .expect("auth log mutex")
                .push(auth_log.clone());
            Ok(())
        })
    }
}

impl RadiusAccountingRepository for FakeRuntimeRepository {
    fn resolve_nas_client<'a>(
        &'a self,
        _context: &'a super::models::RadiusRequestContext,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<RadiusNasClient>>> + Send + 'a>> {
        Box::pin(async move { Ok(self.nas.clone()) })
    }

    fn upsert_accounting_session<'a>(
        &'a self,
        session: &'a RadiusAccountingSession,
    ) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + 'a>> {
        Box::pin(async move {
            self.accounting_sessions
                .lock()
                .expect("accounting session mutex")
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
        shared_secret: String::from_utf8_lossy(SHARED_SECRET).to_string(),
    }
}

fn sample_account() -> ManagedRadiusRuntimeAccount {
    let now = Utc::now();
    ManagedRadiusRuntimeAccount {
        account: PppoeAccount {
            id: "acct-1".into(),
            tenant_id: "tenant-1".into(),
            router_id: "router-1".into(),
            customer_id: "cust-1".into(),
            location_id: "loc-1".into(),
            username: "alice".into(),
            password_enc: "secret-1".into(),
            package_id: None,
            profile_id: None,
            router_profile_name: Some("basic".into()),
            remote_address: Some("10.10.10.2".into()),
            address_pool: Some("pool-a".into()),
            disabled: false,
            comment: None,
            account_source: PppoeAccountSource::ManagedRadius,
            router_present: false,
            router_secret_id: None,
            last_sync_at: None,
            last_error: None,
            is_provisioned: false,
            radius_identity: None,
            provisioned_at: None,
            provisioning_error: None,
            created_at: now,
            updated_at: now,
        },
    }
}

#[tokio::test]
async fn runtime_smoke_auth_flow_builds_access_accept_and_persists_auth_log() {
    let repository = FakeRuntimeRepository {
        nas: Some(sample_nas()),
        accounts: vec![sample_account()],
        ..Default::default()
    };
    let auth_service = RadiusAuthService::new(repository.clone());

    let mut request = Packet::new(Code::AccessRequest, SHARED_SECRET);
    rfc2865::add_user_name(&mut request, "alice");
    rfc2865::add_user_password(&mut request, b"secret-1").expect("user password");
    let request_bytes = request.encode().expect("encode access request");
    let decoded = decode_request(&request_bytes, SHARED_SECRET).expect("decode request");
    let auth_request =
        extract_pap_auth_request(&decoded, "203.0.113.10").expect("extract auth request");

    let result = auth_service
        .authenticate_pap(&auth_request)
        .await
        .expect("authenticate pap");
    let response_bytes =
        build_access_response(&decoded, &result).expect("build access accept response");
    let response = Packet::decode(&response_bytes, SHARED_SECRET).expect("decode response");

    assert_eq!(result.decision, RadiusAccessDecision::Accept);
    assert_eq!(response.get_code(), Code::AccessAccept);
    assert_eq!(
        rfc2865::lookup_framed_ip_address(&response)
            .transpose()
            .expect("framed ip"),
        Some(Ipv4Addr::new(10, 10, 10, 2))
    );
    assert_eq!(
        rfc2869::lookup_framed_pool(&response)
            .transpose()
            .expect("framed pool"),
        Some("pool-a".to_string())
    );
    let logs = repository.auth_logs.lock().expect("auth log mutex");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].outcome, "accept");
}

#[tokio::test]
async fn runtime_smoke_accounting_flow_builds_response_and_persists_session() {
    let repository = FakeRuntimeRepository {
        nas: Some(sample_nas()),
        ..Default::default()
    };
    let accounting_service = RadiusAccountingService::new(repository.clone());

    let mut request = Packet::new(Code::AccountingRequest, SHARED_SECRET);
    rfc2865::add_user_name(&mut request, "alice");
    rfc2866::add_acct_status_type(&mut request, rfc2866::ACCT_STATUS_TYPE_START);
    rfc2866::add_acct_session_id(&mut request, "sess-1");
    rfc2866::add_acct_session_time(&mut request, 120);
    rfc2865::add_framed_ip_address(&mut request, &Ipv4Addr::new(10, 10, 10, 2));
    let request_bytes = request.encode().expect("encode accounting request");
    let decoded = decode_request(&request_bytes, SHARED_SECRET).expect("decode request");
    let accounting_request =
        extract_accounting_request(&decoded, "203.0.113.10").expect("extract accounting request");

    let result = accounting_service
        .handle_accounting(&accounting_request)
        .await
        .expect("handle accounting");
    let response_bytes = build_accounting_response(&decoded).expect("build accounting response");
    let response = Packet::decode(&response_bytes, SHARED_SECRET).expect("decode response");

    assert!(result.acknowledged);
    assert_eq!(response.get_code(), Code::AccountingResponse);
    let sessions = repository
        .accounting_sessions
        .lock()
        .expect("accounting session mutex");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].acct_session_id, "sess-1");
}

#[tokio::test]
async fn runtime_smoke_chap_flow_builds_access_accept_and_persists_auth_log() {
    let repository = FakeRuntimeRepository {
        nas: Some(sample_nas()),
        accounts: vec![sample_account()],
        ..Default::default()
    };
    let auth_service = RadiusAuthService::new(repository.clone());

    let mut request = Packet::new(Code::AccessRequest, SHARED_SECRET);
    rfc2865::add_user_name(&mut request, "alice");
    rfc2865::add_chap_challenge(&mut request, b"0123456789abcdef");
    let digest = md5::compute(
        [
            vec![7_u8],
            b"secret-1".to_vec(),
            b"0123456789abcdef".to_vec(),
        ]
        .concat(),
    );
    let chap_password = [vec![7_u8], digest.0.to_vec()].concat();
    rfc2865::add_chap_password(&mut request, &chap_password);
    let request_bytes = request.encode().expect("encode access request");
    let decoded = decode_request(&request_bytes, SHARED_SECRET).expect("decode request");
    let auth_request = extract_chap_auth_request(&decoded, "203.0.113.10").expect("extract auth");

    let result = auth_service
        .authenticate_chap(&auth_request)
        .await
        .expect("authenticate chap");
    let response_bytes =
        build_access_response(&decoded, &result).expect("build access accept response");
    let response = Packet::decode(&response_bytes, SHARED_SECRET).expect("decode response");

    assert_eq!(result.decision, RadiusAccessDecision::Accept);
    assert_eq!(response.get_code(), Code::AccessAccept);
    let logs = repository.auth_logs.lock().expect("auth log mutex");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].outcome, "accept");
}

#[tokio::test]
async fn runtime_smoke_mschapv2_flow_builds_access_accept_and_persists_auth_log() {
    let repository = FakeRuntimeRepository {
        nas: Some(sample_nas()),
        accounts: vec![ManagedRadiusRuntimeAccount {
            account: {
                let mut account = sample_account().account;
                account.username = "User".into();
                account.password_enc = "clientPass".into();
                account.router_profile_name = None;
                account
            },
        }],
        ..Default::default()
    };
    let auth_service = RadiusAuthService::new(repository.clone());

    let mut request = Packet::new(Code::AccessRequest, SHARED_SECRET);
    rfc2865::add_user_name(&mut request, "User");
    request.add(AVP::from_bytes(
        26,
        &[
            &[0, 0, 1, 55][..],
            &[11, 18],
            &[
                0x5B, 0x5D, 0x7C, 0x7D, 0x7B, 0x3F, 0x2F, 0x3E, 0x3C, 0x2C, 0x60, 0x21, 0x32, 0x26,
                0x26, 0x28,
            ],
        ]
        .concat(),
    ));
    request.add(AVP::from_bytes(
        26,
        &[
            &[0, 0, 1, 55][..],
            &[25, 52],
            &[
                7, 0, 0x21, 0x40, 0x23, 0x24, 0x25, 0x5E, 0x26, 0x2A, 0x28, 0x29, 0x5F, 0x2B, 0x3A,
                0x33, 0x7C, 0x7E, 0, 0, 0, 0, 0, 0, 0, 0, 0x82, 0x30, 0x9E, 0xCD, 0x8D, 0x70, 0x8B,
                0x5E, 0xA0, 0x8F, 0xAA, 0x39, 0x81, 0xCD, 0x83, 0x54, 0x42, 0x33, 0x11, 0x4A, 0x3D,
                0x85, 0xD6, 0xDF,
            ],
        ]
        .concat(),
    ));
    let request_bytes = request.encode().expect("encode access request");
    let decoded = decode_request(&request_bytes, SHARED_SECRET).expect("decode request");
    let auth_request =
        extract_mschapv2_auth_request(&decoded, "203.0.113.10").expect("extract mschapv2");

    let result = auth_service
        .authenticate_mschapv2(&auth_request)
        .await
        .expect("authenticate mschapv2");
    let response_bytes =
        build_access_response(&decoded, &result).expect("build access accept response");
    let response = Packet::decode(&response_bytes, SHARED_SECRET).expect("decode response");

    assert_eq!(result.decision, RadiusAccessDecision::Accept);
    assert_eq!(response.get_code(), Code::AccessAccept);
    let success_value = response
        .lookup(26)
        .expect("ms-chap2-success should exist")
        .encode_bytes();
    assert_eq!(success_value[..5], [0, 0, 1, 55, 26]);
    assert_eq!(success_value[6], 7);
    assert_eq!(success_value[7], b'S');
    let logs = repository.auth_logs.lock().expect("auth log mutex");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].outcome, "accept");
}

#[test]
fn runtime_smoke_reject_shape_matches_packet_builder_contract() {
    let result = RadiusPapAuthResult {
        decision: RadiusAccessDecision::Reject,
        rejection_reason: Some("invalid_password".into()),
        reply_attributes: RadiusReplyAttributes {
            attributes: vec![RadiusReplyAttribute {
                name: "Framed-Pool",
                value: "pool-a".into(),
            }],
        },
        mschapv2_success: None,
    };

    assert_eq!(result.decision, RadiusAccessDecision::Reject);
    assert_eq!(result.rejection_reason.as_deref(), Some("invalid_password"));
    assert_eq!(result.reply_attributes.get("Framed-Pool"), Some("pool-a"));
}
