use super::accounting::RadiusAccountingService;
use super::auth::RadiusAuthService;
use super::config::RadiusRuntimeConfig;
use super::models::RadiusRequestContext;
use super::packet::{
    build_access_response, build_accounting_response, decode_request, extract_accounting_request,
    extract_chap_auth_request, extract_mschapv2_auth_request, extract_pap_auth_request,
    is_chap_access_request, is_mschapv2_access_request, validate_message_authenticator,
};
use super::repository::RadiusRepository;
use crate::db::DbPool;
use crate::error::AppResult;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;
use tracing::{debug, warn};

#[derive(Default)]
struct RadiusRuntimeState {
    started: AtomicBool,
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

#[derive(Debug, Clone, Copy)]
enum RadiusTrafficKind {
    Auth,
    Accounting,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusRuntimeStatus {
    pub enabled: bool,
    pub running: bool,
    pub bind_addr: String,
    pub auth_port: u16,
    pub acct_port: u16,
    pub advertised_host: String,
    pub require_message_authenticator: bool,
}

#[derive(Clone)]
pub struct RadiusService {
    pool: DbPool,
    config: RadiusRuntimeConfig,
    runtime: Arc<RadiusRuntimeState>,
}

impl RadiusService {
    pub fn new(pool: DbPool, config: RadiusRuntimeConfig) -> Self {
        Self {
            pool,
            config,
            runtime: Arc::new(RadiusRuntimeState::default()),
        }
    }

    pub async fn start(&self) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        if self.runtime.started.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        match self.bind_and_spawn().await {
            Ok(tasks) => {
                let mut handles = self
                    .runtime
                    .tasks
                    .lock()
                    .map_err(|_| "radius runtime task state poisoned".to_string())?;
                handles.extend(tasks);
                Ok(())
            }
            Err(error) => {
                self.runtime.started.store(false, Ordering::SeqCst);
                Err(error)
            }
        }
    }

    pub async fn stop(&self) {
        self.runtime.started.store(false, Ordering::SeqCst);
        if let Ok(mut tasks) = self.runtime.tasks.lock() {
            for task in tasks.drain(..) {
                task.abort();
            }
        }
    }

    async fn bind_and_spawn(&self) -> Result<Vec<JoinHandle<()>>, String> {
        let auth_addr = format!("{}:{}", self.config.bind_addr, self.config.auth_port);
        let acct_addr = format!("{}:{}", self.config.bind_addr, self.config.acct_port);

        let auth_socket = UdpSocket::bind(&auth_addr).await.map_err(|error| {
            format!("failed to bind radius auth socket on {auth_addr}: {error}")
        })?;
        let acct_socket = UdpSocket::bind(&acct_addr).await.map_err(|error| {
            format!("failed to bind radius accounting socket on {acct_addr}: {error}")
        })?;

        debug!("radius auth socket bound on {auth_addr}");
        debug!("radius accounting socket bound on {acct_addr}");

        let auth_service = self.clone();
        let auth_packet_size = self.config.max_packet_size;
        let auth_task = tokio::spawn(async move {
            auth_service
                .serve_socket(auth_socket, auth_packet_size, RadiusTrafficKind::Auth)
                .await;
        });

        let acct_service = self.clone();
        let acct_packet_size = self.config.max_packet_size;
        let acct_task = tokio::spawn(async move {
            acct_service
                .serve_socket(acct_socket, acct_packet_size, RadiusTrafficKind::Accounting)
                .await;
        });

        Ok(vec![auth_task, acct_task])
    }

    async fn serve_socket(&self, socket: UdpSocket, packet_size: usize, kind: RadiusTrafficKind) {
        let mut buffer = vec![0_u8; packet_size.max(512)];

        loop {
            let (size, peer) = match socket.recv_from(&mut buffer).await {
                Ok(result) => result,
                Err(error) => {
                    warn!("radius {:?} recv_from failed: {error}", kind);
                    continue;
                }
            };

            let response = match kind {
                RadiusTrafficKind::Auth => {
                    self.handle_access_request(&peer.ip().to_string(), &buffer[..size])
                        .await
                }
                RadiusTrafficKind::Accounting => {
                    self.handle_accounting_request(&peer.ip().to_string(), &buffer[..size])
                        .await
                }
            };

            match response {
                Ok(Some(bytes)) => {
                    if let Err(error) = socket.send_to(&bytes, peer).await {
                        warn!("radius {:?} send_to {peer} failed: {error}", kind);
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    warn!(
                        "radius {:?} request handling failed for {peer}: {error}",
                        kind
                    );
                }
            }
        }
    }

    fn repository(&self) -> RadiusRepository {
        RadiusRepository::new(self.pool.clone())
    }

    pub async fn handle_access_request(
        &self,
        source_ip: &str,
        bytes: &[u8],
    ) -> AppResult<Option<Vec<u8>>> {
        let repository = self.repository();
        let context = RadiusRequestContext {
            source_ip: source_ip.to_string(),
        };
        let Some(nas) = repository.resolve_nas_client(&context).await? else {
            return Ok(None);
        };

        let packet = decode_request(bytes, nas.shared_secret.as_bytes())?;
        validate_message_authenticator(bytes, &packet, self.config.require_message_authenticator)?;
        let auth_service = RadiusAuthService::new(repository);
        let result = if is_mschapv2_access_request(&packet) {
            let request = extract_mschapv2_auth_request(&packet, source_ip)?;
            auth_service.authenticate_mschapv2(&request).await?
        } else if is_chap_access_request(&packet) {
            let request = extract_chap_auth_request(&packet, source_ip)?;
            auth_service.authenticate_chap(&request).await?
        } else {
            let request = extract_pap_auth_request(&packet, source_ip)?;
            auth_service.authenticate_pap(&request).await?
        };
        let response = build_access_response(&packet, &result)?;

        Ok(Some(response))
    }

    pub async fn handle_accounting_request(
        &self,
        source_ip: &str,
        bytes: &[u8],
    ) -> AppResult<Option<Vec<u8>>> {
        let repository = self.repository();
        let context = RadiusRequestContext {
            source_ip: source_ip.to_string(),
        };
        let Some(nas) = repository.resolve_nas_client(&context).await? else {
            return Ok(None);
        };

        let packet = decode_request(bytes, nas.shared_secret.as_bytes())?;
        validate_message_authenticator(bytes, &packet, self.config.require_message_authenticator)?;
        let request = extract_accounting_request(&packet, source_ip)?;
        let accounting_service = RadiusAccountingService::new(repository);
        let result = accounting_service.handle_accounting(&request).await?;
        if !result.acknowledged {
            return Ok(None);
        }

        let response = build_accounting_response(&packet)?;
        Ok(Some(response))
    }

    pub fn config(&self) -> &RadiusRuntimeConfig {
        &self.config
    }

    pub fn status(&self, advertised_host_override: Option<&str>) -> RadiusRuntimeStatus {
        RadiusRuntimeStatus {
            enabled: self.config.enabled,
            running: self.runtime.started.load(Ordering::SeqCst),
            bind_addr: self.config.bind_addr.clone(),
            auth_port: self.config.auth_port,
            acct_port: self.config.acct_port,
            advertised_host: advertised_host_override
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(self.config.bind_addr.as_str())
                .to_string(),
            require_message_authenticator: self.config.require_message_authenticator,
        }
    }

    #[cfg(test)]
    fn is_running(&self) -> bool {
        self.runtime.started.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    fn runtime_task_count(&self) -> usize {
        self.runtime
            .tasks
            .lock()
            .map(|tasks| tasks.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::{RadiusRuntimeConfig, RadiusService};
    use sqlx::postgres::PgPoolOptions;
    use tokio::net::UdpSocket;

    fn test_pool() -> sqlx::PgPool {
        PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@127.0.0.1/test_db")
            .expect("lazy pool")
    }

    async fn free_udp_port() -> u16 {
        let socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral udp port");
        let port = socket.local_addr().expect("local addr").port();
        drop(socket);
        port
    }

    fn enabled_config(auth_port: u16, acct_port: u16) -> RadiusRuntimeConfig {
        RadiusRuntimeConfig {
            enabled: true,
            bind_addr: "127.0.0.1".into(),
            auth_port,
            acct_port,
            worker_concurrency: 2,
            request_timeout_ms: 3_000,
            max_packet_size: 4096,
            require_message_authenticator: true,
        }
    }

    #[tokio::test]
    async fn radius_service_status_reflects_config_before_runtime_start() {
        let service = RadiusService::new(test_pool(), enabled_config(1812, 1813));

        let status = service.status(None);

        assert!(status.enabled);
        assert!(!status.running);
        assert_eq!(status.bind_addr, "127.0.0.1");
        assert_eq!(status.auth_port, 1812);
        assert_eq!(status.acct_port, 1813);
        assert_eq!(status.advertised_host, "127.0.0.1");
        assert!(status.require_message_authenticator);
    }

    #[tokio::test]
    async fn radius_service_start_binds_udp_sockets_when_enabled() {
        let service = RadiusService::new(
            test_pool(),
            enabled_config(free_udp_port().await, free_udp_port().await),
        );

        service.start().await.expect("radius service should start");

        assert!(service.is_running());
        assert_eq!(service.runtime_task_count(), 2);

        service.stop().await;
        assert!(!service.is_running());
    }

    #[tokio::test]
    async fn radius_service_start_is_idempotent_after_first_bind() {
        let service = RadiusService::new(
            test_pool(),
            enabled_config(free_udp_port().await, free_udp_port().await),
        );

        service.start().await.expect("first start should succeed");
        service.start().await.expect("second start should be no-op");

        assert_eq!(service.runtime_task_count(), 2);

        service.stop().await;
    }

    #[tokio::test]
    async fn radius_service_start_returns_error_when_auth_port_is_already_bound() {
        let conflict_socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("bind conflict socket");
        let conflict_port = conflict_socket
            .local_addr()
            .expect("conflict local addr")
            .port();

        let service = RadiusService::new(
            test_pool(),
            enabled_config(conflict_port, free_udp_port().await),
        );

        let error = service
            .start()
            .await
            .expect_err("bind conflict should fail");

        assert!(error.contains("failed to bind radius auth socket"));
        assert!(!service.is_running());
        assert_eq!(service.runtime_task_count(), 0);
    }
}
