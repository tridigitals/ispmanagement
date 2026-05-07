use super::{
    models::RadiusRequestContext, ManagedRadiusRuntimeAccount, RadiusAccessDecision,
    RadiusNasClient, RadiusReplyAttributes, RadiusRepository,
};
use crate::error::AppResult;
use crate::models::RadiusAuthLog;
use crate::security::secret::decrypt_secret_opt_for;
use crate::services::pppoe_service::PURPOSE_PPPOE;
use chrono::Utc;
use des::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
use des::Des;
use md4::{Digest, Md4};
use md5::compute as md5_compute;
use sha1::Sha1;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use uuid::Uuid;

pub trait RadiusAuthRepository {
    fn resolve_nas_client<'a>(
        &'a self,
        context: &'a RadiusRequestContext,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<RadiusNasClient>>> + Send + 'a>>;

    fn load_managed_account<'a>(
        &'a self,
        tenant_id: &'a str,
        router_id: &'a str,
        username: &'a str,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<ManagedRadiusRuntimeAccount>>> + Send + 'a>>;

    fn insert_auth_log<'a>(
        &'a self,
        auth_log: &'a RadiusAuthLog,
    ) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + 'a>>;
}

impl RadiusAuthRepository for RadiusRepository {
    fn resolve_nas_client<'a>(
        &'a self,
        context: &'a RadiusRequestContext,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<RadiusNasClient>>> + Send + 'a>> {
        Box::pin(async move { self.resolve_nas_client(context).await })
    }

    fn load_managed_account<'a>(
        &'a self,
        tenant_id: &'a str,
        router_id: &'a str,
        username: &'a str,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<ManagedRadiusRuntimeAccount>>> + Send + 'a>>
    {
        Box::pin(async move {
            self.load_managed_account(tenant_id, router_id, username)
                .await
        })
    }

    fn insert_auth_log<'a>(
        &'a self,
        auth_log: &'a RadiusAuthLog,
    ) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + 'a>> {
        Box::pin(async move { self.insert_auth_log(auth_log).await })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusPapAuthRequest {
    pub source_ip: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusChapAuthRequest {
    pub source_ip: String,
    pub username: String,
    pub chap_identifier: u8,
    pub challenge: Vec<u8>,
    pub response: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusMschapV2AuthRequest {
    pub source_ip: String,
    pub username: String,
    pub ident: u8,
    pub peer_challenge: Vec<u8>,
    pub reserved: Vec<u8>,
    pub nt_response: Vec<u8>,
    pub authenticator_challenge: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusPapAuthResult {
    pub decision: RadiusAccessDecision,
    pub rejection_reason: Option<String>,
    pub reply_attributes: RadiusReplyAttributes,
    pub mschapv2_success: Option<RadiusMschapV2Success>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusMschapV2Success {
    pub ident: u8,
    pub message: String,
}

pub struct RadiusAuthService<R> {
    repository: R,
}

impl<R> RadiusAuthService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R> RadiusAuthService<R>
where
    R: RadiusAuthRepository,
{
    pub async fn authenticate_chap(
        &self,
        request: &RadiusChapAuthRequest,
    ) -> AppResult<RadiusPapAuthResult> {
        let started_at = Instant::now();
        let (nas, account, expected_password) = match self
            .load_auth_context(
                &request.source_ip,
                &request.username,
                request_log(&request.source_ip, &request.username),
                started_at,
                "chap",
            )
            .await?
        {
            AuthContextLoad::Ready(context) => context,
            AuthContextLoad::Rejected(result) => return Ok(result),
        };

        let expected_digest = md5_compute(
            [
                vec![request.chap_identifier],
                expected_password.into_bytes(),
                request.challenge.clone(),
            ]
            .concat(),
        );

        if expected_digest.0.as_slice() != request.response.as_slice() {
            let result = reject("invalid_password");
            self.log_auth_outcome(
                request_log(&request.source_ip, &request.username),
                Some(&nas.tenant_id),
                Some(&nas.router_id),
                &result,
                started_at.elapsed().as_millis() as i64,
                "chap",
            )
            .await?;
            return Ok(result);
        }

        let result = RadiusPapAuthResult {
            decision: RadiusAccessDecision::Accept,
            rejection_reason: None,
            reply_attributes: RadiusReplyAttributes::from_account(&account.account),
            mschapv2_success: None,
        };
        self.log_auth_outcome(
            request_log(&request.source_ip, &request.username),
            Some(&nas.tenant_id),
            Some(&nas.router_id),
            &result,
            started_at.elapsed().as_millis() as i64,
            "chap",
        )
        .await?;

        Ok(result)
    }

    pub async fn authenticate_pap(
        &self,
        request: &RadiusPapAuthRequest,
    ) -> AppResult<RadiusPapAuthResult> {
        let started_at = Instant::now();
        let auth_request = request_log(&request.source_ip, &request.username);
        let (nas, account, expected_password) = match self
            .load_auth_context(
                &request.source_ip,
                &request.username,
                auth_request,
                started_at,
                "pap",
            )
            .await?
        {
            AuthContextLoad::Ready(context) => context,
            AuthContextLoad::Rejected(result) => return Ok(result),
        };

        if expected_password != request.password {
            let result = reject("invalid_password");
            self.log_auth_outcome(
                auth_request,
                Some(&nas.tenant_id),
                Some(&nas.router_id),
                &result,
                started_at.elapsed().as_millis() as i64,
                "pap",
            )
            .await?;
            return Ok(result);
        }

        let result = RadiusPapAuthResult {
            decision: RadiusAccessDecision::Accept,
            rejection_reason: None,
            reply_attributes: RadiusReplyAttributes::from_account(&account.account),
            mschapv2_success: None,
        };
        self.log_auth_outcome(
            auth_request,
            Some(&nas.tenant_id),
            Some(&nas.router_id),
            &result,
            started_at.elapsed().as_millis() as i64,
            "pap",
        )
        .await?;

        Ok(result)
    }

    pub async fn authenticate_mschapv2(
        &self,
        request: &RadiusMschapV2AuthRequest,
    ) -> AppResult<RadiusPapAuthResult> {
        let started_at = Instant::now();
        let auth_request = request_log(&request.source_ip, &request.username);
        let (nas, account, expected_password) = match self
            .load_auth_context(
                &request.source_ip,
                &request.username,
                auth_request,
                started_at,
                "mschapv2",
            )
            .await?
        {
            AuthContextLoad::Ready(context) => context,
            AuthContextLoad::Rejected(result) => return Ok(result),
        };

        let expected_response = generate_nt_response(
            &request.authenticator_challenge,
            &request.peer_challenge,
            &request.username,
            &expected_password,
        );

        if expected_response.as_slice() != request.nt_response.as_slice() {
            let result = reject("invalid_password");
            self.log_auth_outcome(
                auth_request,
                Some(&nas.tenant_id),
                Some(&nas.router_id),
                &result,
                started_at.elapsed().as_millis() as i64,
                "mschapv2",
            )
            .await?;
            return Ok(result);
        }

        let result = RadiusPapAuthResult {
            decision: RadiusAccessDecision::Accept,
            rejection_reason: None,
            reply_attributes: RadiusReplyAttributes::from_account(&account.account),
            mschapv2_success: Some(RadiusMschapV2Success {
                ident: request.ident,
                message: generate_authenticator_response(
                    &expected_password,
                    &request.peer_challenge,
                    &request.authenticator_challenge,
                    &request.username,
                    &request.nt_response,
                ),
            }),
        };
        self.log_auth_outcome(
            auth_request,
            Some(&nas.tenant_id),
            Some(&nas.router_id),
            &result,
            started_at.elapsed().as_millis() as i64,
            "mschapv2",
        )
        .await?;

        Ok(result)
    }

    async fn load_auth_context(
        &self,
        source_ip: &str,
        username: &str,
        request: AuthLogRequest<'_>,
        started_at: Instant,
        auth_type: &str,
    ) -> AppResult<AuthContextLoad> {
        let context = RadiusRequestContext {
            source_ip: source_ip.to_string(),
        };
        let Some(nas) = self.repository.resolve_nas_client(&context).await? else {
            let result = reject("unknown_nas");
            self.log_auth_outcome(
                request,
                None,
                None,
                &result,
                started_at.elapsed().as_millis() as i64,
                auth_type,
            )
            .await?;
            return Ok(AuthContextLoad::Rejected(result));
        };

        let Some(account) = self
            .repository
            .load_managed_account(&nas.tenant_id, &nas.router_id, username)
            .await?
        else {
            let result = reject("unknown_user");
            self.log_auth_outcome(
                request,
                Some(&nas.tenant_id),
                Some(&nas.router_id),
                &result,
                started_at.elapsed().as_millis() as i64,
                auth_type,
            )
            .await?;
            return Ok(AuthContextLoad::Rejected(result));
        };

        if account.account.disabled {
            let result = reject("account_disabled");
            self.log_auth_outcome(
                request,
                Some(&nas.tenant_id),
                Some(&nas.router_id),
                &result,
                started_at.elapsed().as_millis() as i64,
                auth_type,
            )
            .await?;
            return Ok(AuthContextLoad::Rejected(result));
        }

        let Some(expected_password) =
            decrypt_secret_opt_for(PURPOSE_PPPOE, &account.account.password_enc)?
        else {
            let result = reject("password_unavailable");
            self.log_auth_outcome(
                request,
                Some(&nas.tenant_id),
                Some(&nas.router_id),
                &result,
                started_at.elapsed().as_millis() as i64,
                auth_type,
            )
            .await?;
            return Ok(AuthContextLoad::Rejected(result));
        };

        Ok(AuthContextLoad::Ready((nas, account, expected_password)))
    }

    async fn log_auth_outcome(
        &self,
        request: AuthLogRequest<'_>,
        tenant_id: Option<&str>,
        router_id: Option<&str>,
        result: &RadiusPapAuthResult,
        latency_ms: i64,
        auth_type: &str,
    ) -> AppResult<()> {
        let outcome = match result.decision {
            RadiusAccessDecision::Accept => "accept",
            RadiusAccessDecision::Reject => "reject",
        };

        let auth_log = RadiusAuthLog {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.map(ToOwned::to_owned),
            router_id: router_id.map(ToOwned::to_owned),
            source_ip: request.source_ip.to_string(),
            username: Some(request.username.to_string()),
            radius_identity: None,
            outcome: outcome.to_string(),
            reason: result.rejection_reason.clone(),
            auth_type: Some(auth_type.to_string()),
            latency_ms: Some(latency_ms),
            created_at: Utc::now(),
        };

        self.repository.insert_auth_log(&auth_log).await
    }
}

fn reject(reason: &str) -> RadiusPapAuthResult {
    RadiusPapAuthResult {
        decision: RadiusAccessDecision::Reject,
        rejection_reason: Some(reason.to_string()),
        reply_attributes: RadiusReplyAttributes { attributes: vec![] },
        mschapv2_success: None,
    }
}

const MSCHAPV2_MAGIC1: &[u8] = b"Magic server to client signing constant";
const MSCHAPV2_MAGIC2: &[u8] = b"Pad to make it do more than one iteration";

fn generate_nt_response(
    authenticator_challenge: &[u8],
    peer_challenge: &[u8],
    username: &str,
    password: &str,
) -> Vec<u8> {
    let challenge = challenge_hash(peer_challenge, authenticator_challenge, username);
    let password_hash = nt_password_hash(password);
    challenge_response(&challenge, &password_hash)
}

fn generate_authenticator_response(
    password: &str,
    peer_challenge: &[u8],
    authenticator_challenge: &[u8],
    username: &str,
    nt_response: &[u8],
) -> String {
    let password_hash = nt_password_hash(password);
    let password_hash_hash = md4_hash(&password_hash);

    let mut first = Sha1::new();
    first.update(password_hash_hash);
    first.update(nt_response);
    first.update(MSCHAPV2_MAGIC1);
    let digest = first.finalize();

    let challenge = challenge_hash(peer_challenge, authenticator_challenge, username);
    let mut second = Sha1::new();
    second.update(digest);
    second.update(challenge);
    second.update(MSCHAPV2_MAGIC2);

    format!("S={}", encode_hex_upper(&second.finalize()))
}

fn challenge_hash(
    peer_challenge: &[u8],
    authenticator_challenge: &[u8],
    username: &str,
) -> [u8; 8] {
    let mut sha1 = Sha1::new();
    sha1.update(peer_challenge);
    sha1.update(authenticator_challenge);
    sha1.update(normalize_mschap_username(username).as_bytes());

    let digest = sha1.finalize();
    let mut challenge = [0_u8; 8];
    challenge.copy_from_slice(&digest[..8]);
    challenge
}

fn nt_password_hash(password: &str) -> [u8; 16] {
    let utf16: Vec<u8> = password
        .encode_utf16()
        .flat_map(|unit| unit.to_le_bytes())
        .collect();
    md4_hash(&utf16)
}

fn md4_hash(bytes: &[u8]) -> [u8; 16] {
    let mut hasher = Md4::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut output = [0_u8; 16];
    output.copy_from_slice(&digest);
    output
}

fn challenge_response(challenge: &[u8; 8], password_hash: &[u8; 16]) -> Vec<u8> {
    let mut z_password_hash = [0_u8; 21];
    z_password_hash[..16].copy_from_slice(password_hash);

    let mut response = Vec::with_capacity(24);
    for chunk in z_password_hash.chunks(7) {
        let key = expand_des_key(chunk);
        let cipher = Des::new(GenericArray::from_slice(&key));
        let mut block = GenericArray::clone_from_slice(challenge);
        cipher.encrypt_block(&mut block);
        response.extend_from_slice(&block);
    }
    response
}

fn expand_des_key(key_7: &[u8]) -> [u8; 8] {
    let mut key = [0_u8; 8];
    key[0] = key_7[0] & 0xfe;
    key[1] = ((key_7[0] << 7) | (key_7[1] >> 1)) & 0xfe;
    key[2] = ((key_7[1] << 6) | (key_7[2] >> 2)) & 0xfe;
    key[3] = ((key_7[2] << 5) | (key_7[3] >> 3)) & 0xfe;
    key[4] = ((key_7[3] << 4) | (key_7[4] >> 4)) & 0xfe;
    key[5] = ((key_7[4] << 3) | (key_7[5] >> 5)) & 0xfe;
    key[6] = ((key_7[5] << 2) | (key_7[6] >> 6)) & 0xfe;
    key[7] = (key_7[6] << 1) & 0xfe;

    for byte in &mut key {
        *byte = set_odd_parity(*byte);
    }

    key
}

fn set_odd_parity(byte: u8) -> u8 {
    if byte.count_ones() % 2 == 0 {
        byte | 1
    } else {
        byte & 0xfe
    }
}

fn normalize_mschap_username(username: &str) -> &str {
    username.rsplit('\\').next().unwrap_or(username)
}

fn encode_hex_upper(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

enum AuthContextLoad {
    Ready((RadiusNasClient, ManagedRadiusRuntimeAccount, String)),
    Rejected(RadiusPapAuthResult),
}

#[derive(Clone, Copy)]
struct AuthLogRequest<'a> {
    source_ip: &'a str,
    username: &'a str,
}

fn request_log<'a>(source_ip: &'a str, username: &'a str) -> AuthLogRequest<'a> {
    AuthLogRequest {
        source_ip,
        username,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RadiusAuthRepository, RadiusAuthService, RadiusChapAuthRequest, RadiusMschapV2AuthRequest,
        RadiusPapAuthRequest,
    };
    use crate::error::AppResult;
    use crate::models::{PppoeAccount, PppoeAccountSource, RadiusAuthLog};
    use crate::services::radius_service::{
        ManagedRadiusRuntimeAccount, RadiusAccessDecision, RadiusNasClient,
    };
    use chrono::Utc;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct FakeRadiusAuthRepository {
        nas: Option<RadiusNasClient>,
        accounts: Vec<ManagedRadiusRuntimeAccount>,
        auth_logs: Arc<Mutex<Vec<RadiusAuthLog>>>,
    }

    impl RadiusAuthRepository for FakeRadiusAuthRepository {
        fn resolve_nas_client<'a>(
            &'a self,
            _context: &'a crate::services::radius_service::models::RadiusRequestContext,
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

    fn sample_nas(router_id: &str) -> RadiusNasClient {
        RadiusNasClient {
            id: format!("nas-{router_id}"),
            tenant_id: "tenant-1".into(),
            router_id: router_id.into(),
            nas_name: format!("NAS {router_id}"),
            nas_ip_or_cidr: "203.0.113.10/32".into(),
            shortname: Some(router_id.into()),
            shared_secret: "shared-secret".into(),
        }
    }

    fn sample_account(
        router_id: &str,
        username: &str,
        password: &str,
    ) -> ManagedRadiusRuntimeAccount {
        let now = Utc::now();
        ManagedRadiusRuntimeAccount {
            account: PppoeAccount {
                id: format!("acct-{router_id}-{username}"),
                tenant_id: "tenant-1".into(),
                router_id: router_id.into(),
                customer_id: "cust-1".into(),
                location_id: "loc-1".into(),
                username: username.into(),
                password_enc: password.into(),
                package_id: None,
                profile_id: None,
                router_profile_name: Some(format!("profile-{router_id}")),
                remote_address: Some(if router_id == "router-2" {
                    "10.20.20.2".into()
                } else {
                    "10.10.10.2".into()
                }),
                address_pool: Some(if router_id == "router-2" {
                    "pool-b".into()
                } else {
                    "pool-a".into()
                }),
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
    async fn pap_auth_accepts_valid_managed_radius_account() {
        let repository = FakeRadiusAuthRepository {
            nas: Some(sample_nas("router-1")),
            accounts: vec![sample_account("router-1", "alice", "secret-1")],
            ..Default::default()
        };
        let service = RadiusAuthService::new(repository);

        let result = service
            .authenticate_pap(&RadiusPapAuthRequest {
                source_ip: "203.0.113.10".into(),
                username: "alice".into(),
                password: "secret-1".into(),
            })
            .await
            .expect("pap auth should return result");

        assert_eq!(result.decision, RadiusAccessDecision::Accept);
        assert_eq!(result.rejection_reason, None);
        assert_eq!(
            result.reply_attributes.get("Mikrotik-Group"),
            Some("profile-router-1")
        );
        let logs = service.repository.auth_logs.lock().expect("auth log mutex");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].outcome, "accept");
        assert_eq!(logs[0].tenant_id.as_deref(), Some("tenant-1"));
        assert_eq!(logs[0].router_id.as_deref(), Some("router-1"));
    }

    #[tokio::test]
    async fn pap_auth_rejects_wrong_password() {
        let repository = FakeRadiusAuthRepository {
            nas: Some(sample_nas("router-1")),
            accounts: vec![sample_account("router-1", "alice", "secret-1")],
            ..Default::default()
        };
        let service = RadiusAuthService::new(repository);

        let result = service
            .authenticate_pap(&RadiusPapAuthRequest {
                source_ip: "203.0.113.10".into(),
                username: "alice".into(),
                password: "wrong".into(),
            })
            .await
            .expect("pap auth should return result");

        assert_eq!(result.decision, RadiusAccessDecision::Reject);
        assert_eq!(result.rejection_reason.as_deref(), Some("invalid_password"));
        let logs = service.repository.auth_logs.lock().expect("auth log mutex");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].outcome, "reject");
        assert_eq!(logs[0].reason.as_deref(), Some("invalid_password"));
    }

    #[tokio::test]
    async fn pap_auth_rejects_disabled_account() {
        let mut account = sample_account("router-1", "alice", "secret-1");
        account.account.disabled = true;

        let repository = FakeRadiusAuthRepository {
            nas: Some(sample_nas("router-1")),
            accounts: vec![account],
            ..Default::default()
        };
        let service = RadiusAuthService::new(repository);

        let result = service
            .authenticate_pap(&RadiusPapAuthRequest {
                source_ip: "203.0.113.10".into(),
                username: "alice".into(),
                password: "secret-1".into(),
            })
            .await
            .expect("pap auth should return result");

        assert_eq!(result.decision, RadiusAccessDecision::Reject);
        assert_eq!(result.rejection_reason.as_deref(), Some("account_disabled"));
    }

    #[tokio::test]
    async fn pap_auth_rejects_unknown_nas() {
        let service = RadiusAuthService::new(FakeRadiusAuthRepository::default());

        let result = service
            .authenticate_pap(&RadiusPapAuthRequest {
                source_ip: "198.51.100.7".into(),
                username: "alice".into(),
                password: "secret-1".into(),
            })
            .await
            .expect("pap auth should return result");

        assert_eq!(result.decision, RadiusAccessDecision::Reject);
        assert_eq!(result.rejection_reason.as_deref(), Some("unknown_nas"));
    }

    #[tokio::test]
    async fn pap_auth_isolates_duplicate_usernames_by_nas_router_context() {
        let repository = FakeRadiusAuthRepository {
            nas: Some(sample_nas("router-2")),
            accounts: vec![
                sample_account("router-1", "alice", "secret-1"),
                sample_account("router-2", "alice", "secret-2"),
            ],
            ..Default::default()
        };
        let service = RadiusAuthService::new(repository);

        let result = service
            .authenticate_pap(&RadiusPapAuthRequest {
                source_ip: "203.0.113.10".into(),
                username: "alice".into(),
                password: "secret-2".into(),
            })
            .await
            .expect("pap auth should return result");

        assert_eq!(result.decision, RadiusAccessDecision::Accept);
        assert_eq!(
            result.reply_attributes.get("Framed-IP-Address"),
            Some("10.20.20.2")
        );
        assert_eq!(result.reply_attributes.get("Framed-Pool"), Some("pool-b"));
    }

    #[tokio::test]
    async fn chap_auth_accepts_valid_managed_radius_account() {
        let repository = FakeRadiusAuthRepository {
            nas: Some(sample_nas("router-1")),
            accounts: vec![sample_account("router-1", "alice", "secret-1")],
            ..Default::default()
        };
        let service = RadiusAuthService::new(repository);
        let challenge = b"0123456789abcdef".to_vec();
        let digest = md5::compute([vec![7_u8], b"secret-1".to_vec(), challenge.clone()].concat());

        let result = service
            .authenticate_chap(&RadiusChapAuthRequest {
                source_ip: "203.0.113.10".into(),
                username: "alice".into(),
                chap_identifier: 7,
                challenge,
                response: digest.0.to_vec(),
            })
            .await
            .expect("chap auth should return result");

        assert_eq!(result.decision, RadiusAccessDecision::Accept);
        assert_eq!(result.rejection_reason, None);
    }

    #[tokio::test]
    async fn chap_auth_rejects_invalid_digest() {
        let repository = FakeRadiusAuthRepository {
            nas: Some(sample_nas("router-1")),
            accounts: vec![sample_account("router-1", "alice", "secret-1")],
            ..Default::default()
        };
        let service = RadiusAuthService::new(repository);

        let result = service
            .authenticate_chap(&RadiusChapAuthRequest {
                source_ip: "203.0.113.10".into(),
                username: "alice".into(),
                chap_identifier: 7,
                challenge: b"0123456789abcdef".to_vec(),
                response: vec![0; 16],
            })
            .await
            .expect("chap auth should return result");

        assert_eq!(result.decision, RadiusAccessDecision::Reject);
        assert_eq!(result.rejection_reason.as_deref(), Some("invalid_password"));
    }

    #[tokio::test]
    async fn mschapv2_auth_accepts_valid_managed_radius_account() {
        let repository = FakeRadiusAuthRepository {
            nas: Some(sample_nas("router-1")),
            accounts: vec![sample_account("router-1", "User", "clientPass")],
            ..Default::default()
        };
        let service = RadiusAuthService::new(repository);

        let result = service
            .authenticate_mschapv2(&RadiusMschapV2AuthRequest {
                source_ip: "203.0.113.10".into(),
                username: "User".into(),
                ident: 7,
                peer_challenge: vec![
                    0x21, 0x40, 0x23, 0x24, 0x25, 0x5E, 0x26, 0x2A, 0x28, 0x29, 0x5F, 0x2B, 0x3A,
                    0x33, 0x7C, 0x7E,
                ],
                reserved: vec![0; 8],
                nt_response: vec![
                    0x82, 0x30, 0x9E, 0xCD, 0x8D, 0x70, 0x8B, 0x5E, 0xA0, 0x8F, 0xAA, 0x39, 0x81,
                    0xCD, 0x83, 0x54, 0x42, 0x33, 0x11, 0x4A, 0x3D, 0x85, 0xD6, 0xDF,
                ],
                authenticator_challenge: vec![
                    0x5B, 0x5D, 0x7C, 0x7D, 0x7B, 0x3F, 0x2F, 0x3E, 0x3C, 0x2C, 0x60, 0x21, 0x32,
                    0x26, 0x26, 0x28,
                ],
            })
            .await
            .expect("mschapv2 auth should return result");

        assert_eq!(result.decision, RadiusAccessDecision::Accept);
        assert_eq!(
            result.mschapv2_success.as_ref().map(|value| value.ident),
            Some(7)
        );
        assert_eq!(
            result
                .mschapv2_success
                .as_ref()
                .map(|value| value.message.as_str()),
            Some("S=407A5589115FD0D6209F510FE9C04566932CDA56")
        );
    }
}
