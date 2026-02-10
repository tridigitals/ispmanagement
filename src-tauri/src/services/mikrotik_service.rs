//! MikroTik integration (tenant-scoped): inventory + basic health/metrics polling.
//!
//! Current scope:
//! - CRUD routers (host/port/username/password)
//! - Test connection (identity/version)
//! - Background poller to update online/offline + store snapshots
//!
//! Notes:
//! - Passwords are stored encrypted-at-rest in DB (never returned via API).
//!   Encryption uses `MIKROTIK_CRED_KEY` (see `crate::security::secret`).

use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    CreateMikrotikRouterRequest, MikrotikHealthSnapshot, MikrotikInterfaceSnapshot,
    MikrotikInterfaceMetric, MikrotikIpAddressSnapshot, MikrotikRouter, MikrotikRouterMetric,
    MikrotikRouterNocRow, MikrotikRouterSnapshot, MikrotikTestResult, UpdateMikrotikRouterRequest,
};
use crate::security::secret::{decrypt_secret_opt, encrypt_secret};
use crate::services::{AuditService, NotificationService};
use chrono::Utc;
use mikrotik_rs::{protocol::command::CommandBuilder, protocol::CommandResponse, MikrotikDevice};
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};
use chrono::DateTime;

#[derive(Clone)]
pub struct MikrotikService {
    pool: DbPool,
    notification_service: NotificationService,
    audit_service: AuditService,
}

impl MikrotikService {
    pub fn new(pool: DbPool, notification_service: NotificationService, audit_service: AuditService) -> Self {
        Self {
            pool,
            notification_service,
            audit_service,
        }
    }

    pub async fn list_routers(&self, tenant_id: &str) -> AppResult<Vec<MikrotikRouter>> {
        let routers = sqlx::query_as::<_, MikrotikRouter>(
            r#"
            SELECT * FROM mikrotik_routers
            WHERE tenant_id = $1
            ORDER BY updated_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(routers)
    }

    pub async fn list_noc(&self, tenant_id: &str) -> AppResult<Vec<MikrotikRouterNocRow>> {
        // Portable SQL: correlated subqueries for "latest" metric columns per router.
        let rows = sqlx::query_as::<_, MikrotikRouterNocRow>(
            r#"
            SELECT
              r.id, r.tenant_id, r.name, r.host, r.port, r.username, r.use_tls, r.enabled,
              r.identity, r.ros_version, r.is_online, r.last_seen_at, r.latency_ms, r.last_error,
              r.created_at, r.updated_at,

              (SELECT m.cpu_load FROM mikrotik_router_metrics m WHERE m.router_id = r.id ORDER BY m.ts DESC LIMIT 1) AS cpu_load,
              (SELECT m.total_memory_bytes FROM mikrotik_router_metrics m WHERE m.router_id = r.id ORDER BY m.ts DESC LIMIT 1) AS total_memory_bytes,
              (SELECT m.free_memory_bytes FROM mikrotik_router_metrics m WHERE m.router_id = r.id ORDER BY m.ts DESC LIMIT 1) AS free_memory_bytes,
              (SELECT m.total_hdd_bytes FROM mikrotik_router_metrics m WHERE m.router_id = r.id ORDER BY m.ts DESC LIMIT 1) AS total_hdd_bytes,
              (SELECT m.free_hdd_bytes FROM mikrotik_router_metrics m WHERE m.router_id = r.id ORDER BY m.ts DESC LIMIT 1) AS free_hdd_bytes,
              (SELECT m.uptime_seconds FROM mikrotik_router_metrics m WHERE m.router_id = r.id ORDER BY m.ts DESC LIMIT 1) AS uptime_seconds,
              (SELECT m.rx_bps FROM mikrotik_router_metrics m WHERE m.router_id = r.id ORDER BY m.ts DESC LIMIT 1) AS rx_bps,
              (SELECT m.tx_bps FROM mikrotik_router_metrics m WHERE m.router_id = r.id ORDER BY m.ts DESC LIMIT 1) AS tx_bps
            FROM mikrotik_routers r
            WHERE r.tenant_id = $1
            ORDER BY r.updated_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows)
    }

    pub async fn get_router(&self, tenant_id: &str, id: &str) -> AppResult<Option<MikrotikRouter>> {
        let router = sqlx::query_as::<_, MikrotikRouter>(
            r#"
            SELECT * FROM mikrotik_routers
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(router)
    }

    pub async fn create_router(
        &self,
        tenant_id: &str,
        req: CreateMikrotikRouterRequest,
    ) -> AppResult<MikrotikRouter> {
        let encrypted_password = encrypt_secret(req.password.as_str())?;
        let router = MikrotikRouter::new(
            tenant_id.to_string(),
            req.name,
            req.host,
            req.port.unwrap_or(8728),
            req.username,
            encrypted_password,
            req.use_tls.unwrap_or(false),
            req.enabled.unwrap_or(true),
        );

        sqlx::query(
            r#"
            INSERT INTO mikrotik_routers
            (id, tenant_id, name, host, port, username, password, use_tls, enabled,
             identity, ros_version, is_online, last_seen_at, latency_ms, last_error,
             created_at, updated_at)
            VALUES
            ($1,$2,$3,$4,$5,$6,$7,$8,$9,
             $10,$11,$12,$13,$14,$15,
             $16,$17)
            "#,
        )
        .bind(&router.id)
        .bind(&router.tenant_id)
        .bind(&router.name)
        .bind(&router.host)
        .bind(router.port)
        .bind(&router.username)
        .bind(&router.password)
        .bind(router.use_tls)
        .bind(router.enabled)
        .bind(&router.identity)
        .bind(&router.ros_version)
        .bind(router.is_online)
        .bind(&router.last_seen_at)
        .bind(&router.latency_ms)
        .bind(&router.last_error)
        .bind(router.created_at)
        .bind(router.updated_at)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(router)
    }

    pub async fn update_router(
        &self,
        tenant_id: &str,
        id: &str,
        req: UpdateMikrotikRouterRequest,
    ) -> AppResult<MikrotikRouter> {
        let existing = self
            .get_router(tenant_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".to_string()))?;

        let now = Utc::now();
        let name = req.name.unwrap_or(existing.name);
        let host = req.host.unwrap_or(existing.host);
        let port = req.port.unwrap_or(existing.port);
        let username = req.username.unwrap_or(existing.username);
        let password = match req.password {
            Some(p) if !p.trim().is_empty() => encrypt_secret(p.as_str())?,
            _ => existing.password,
        };
        let use_tls = req.use_tls.unwrap_or(existing.use_tls);
        let enabled = req.enabled.unwrap_or(existing.enabled);

        sqlx::query(
            r#"
            UPDATE mikrotik_routers SET
              name = $1,
              host = $2,
              port = $3,
              username = $4,
              password = $5,
              use_tls = $6,
              enabled = $7,
              updated_at = $8
            WHERE id = $9 AND tenant_id = $10
            "#,
        )
        .bind(&name)
        .bind(&host)
        .bind(port)
        .bind(&username)
        .bind(&password)
        .bind(use_tls)
        .bind(enabled)
        .bind(now)
        .bind(id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let updated = self
            .get_router(tenant_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".to_string()))?;

        Ok(updated)
    }

    pub async fn delete_router(&self, tenant_id: &str, id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn list_metrics(
        &self,
        tenant_id: &str,
        router_id: &str,
        limit: u32,
    ) -> AppResult<Vec<MikrotikRouterMetric>> {
        // Ensure router belongs to tenant
        let exists: Option<String> = sqlx::query_scalar(
            "SELECT id FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2",
        )
        .bind(router_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if exists.is_none() {
            return Err(AppError::Forbidden("No access to router".into()));
        }

        let rows = sqlx::query_as::<_, MikrotikRouterMetric>(
            r#"
            SELECT * FROM mikrotik_router_metrics
            WHERE router_id = $1
            ORDER BY ts DESC
            LIMIT $2
            "#,
        )
        .bind(router_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows)
    }

    pub async fn list_interface_metrics(
        &self,
        tenant_id: &str,
        router_id: &str,
        interface_name: Option<&str>,
        limit: u32,
    ) -> AppResult<Vec<MikrotikInterfaceMetric>> {
        // Ensure router belongs to tenant
        let exists: Option<String> = sqlx::query_scalar(
            "SELECT id FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2",
        )
        .bind(router_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if exists.is_none() {
            return Err(AppError::Forbidden("No access to router".into()));
        }

        let rows = if let Some(ifname) = interface_name {
            sqlx::query_as::<_, MikrotikInterfaceMetric>(
                r#"
                SELECT * FROM mikrotik_interface_metrics
                WHERE router_id = $1 AND interface_name = $2
                ORDER BY ts DESC
                LIMIT $3
                "#,
            )
            .bind(router_id)
            .bind(ifname)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?
        } else {
            sqlx::query_as::<_, MikrotikInterfaceMetric>(
                r#"
                SELECT * FROM mikrotik_interface_metrics
                WHERE router_id = $1
                ORDER BY ts DESC
                LIMIT $2
                "#,
            )
            .bind(router_id)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?
        };

        Ok(rows)
    }

    pub async fn list_latest_interface_metrics(
        &self,
        tenant_id: &str,
        router_id: &str,
    ) -> AppResult<Vec<MikrotikInterfaceMetric>> {
        // Ensure router belongs to tenant
        let exists: Option<String> = sqlx::query_scalar(
            "SELECT id FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2",
        )
        .bind(router_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if exists.is_none() {
            return Err(AppError::Forbidden("No access to router".into()));
        }

        // Portable approach: order by interface_name + ts desc, then de-dupe in Rust.
        let mut rows = sqlx::query_as::<_, MikrotikInterfaceMetric>(
            r#"
            SELECT * FROM mikrotik_interface_metrics
            WHERE router_id = $1
            ORDER BY interface_name ASC, ts DESC
            "#,
        )
        .bind(router_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut out: Vec<MikrotikInterfaceMetric> = vec![];
        let mut last: Option<String> = None;
        for r in rows.drain(..) {
            if last.as_deref() == Some(r.interface_name.as_str()) {
                continue;
            }
            last = Some(r.interface_name.clone());
            out.push(r);
        }

        Ok(out)
    }

    /// Fetch a "live" snapshot from the router (best-effort).
    ///
    /// This is used by the admin detail UI to show richer data without forcing
    /// the background poller to store huge payloads.
    pub async fn get_snapshot(&self, tenant_id: &str, id: &str) -> AppResult<MikrotikRouterSnapshot> {
        let mut router = self
            .get_router(tenant_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".to_string()))?;

        let addr = format!("{}:{}", router.host, router.port);
        let password = decrypt_secret_opt(router.password.as_str())?;

        let started = Instant::now();

        let dev = match timeout(
            Duration::from_secs(5),
            MikrotikDevice::connect(addr, router.username.as_str(), password.as_deref()),
        )
        .await
        {
            Ok(Ok(dev)) => dev,
            Ok(Err(e)) => {
                let now = Utc::now();
                let latency_ms = Some(started.elapsed().as_millis().min(i32::MAX as u128) as i32);
                let msg = e.to_string();

                let _ = sqlx::query(
                    r#"
                    UPDATE mikrotik_routers SET
                      is_online = false,
                      latency_ms = $1,
                      last_error = $2,
                      updated_at = $3
                    WHERE id = $4 AND tenant_id = $5
                    "#,
                )
                .bind(latency_ms)
                .bind(&msg)
                .bind(now)
                .bind(&router.id)
                .bind(&router.tenant_id)
                .execute(&self.pool)
                .await;

                router.is_online = false;
                router.latency_ms = latency_ms;
                router.last_error = Some(msg);
                router.last_seen_at = None;
                router.updated_at = now;

                return Ok(MikrotikRouterSnapshot {
                    router,
                    cpu_load: None,
                    total_memory_bytes: None,
                    free_memory_bytes: None,
                    total_hdd_bytes: None,
                    free_hdd_bytes: None,
                    uptime_seconds: None,
                    board_name: None,
                    architecture: None,
                    cpu: None,
                    interfaces: vec![],
                    ip_addresses: vec![],
                    health: None,
                });
            }
            Err(_) => {
                let now = Utc::now();
                let latency_ms = Some(started.elapsed().as_millis().min(i32::MAX as u128) as i32);
                let msg = "Connection timed out".to_string();

                let _ = sqlx::query(
                    r#"
                    UPDATE mikrotik_routers SET
                      is_online = false,
                      latency_ms = $1,
                      last_error = $2,
                      updated_at = $3
                    WHERE id = $4 AND tenant_id = $5
                    "#,
                )
                .bind(latency_ms)
                .bind(&msg)
                .bind(now)
                .bind(&router.id)
                .bind(&router.tenant_id)
                .execute(&self.pool)
                .await;

                router.is_online = false;
                router.latency_ms = latency_ms;
                router.last_error = Some(msg);
                router.last_seen_at = None;
                router.updated_at = now;

                return Ok(MikrotikRouterSnapshot {
                    router,
                    cpu_load: None,
                    total_memory_bytes: None,
                    free_memory_bytes: None,
                    total_hdd_bytes: None,
                    free_hdd_bytes: None,
                    uptime_seconds: None,
                    board_name: None,
                    architecture: None,
                    cpu: None,
                    interfaces: vec![],
                    ip_addresses: vec![],
                    health: None,
                });
            }
        };

        let latency_ms = Some(started.elapsed().as_millis().min(i32::MAX as u128) as i32);

        // identity + version (best-effort)
        let identity = self.fetch_identity_snapshot(&dev).await.ok().flatten();

        // Resource
        let (
            cpu_load,
            total_memory_bytes,
            free_memory_bytes,
            total_hdd_bytes,
            free_hdd_bytes,
            uptime_seconds,
            board_name,
            architecture,
            cpu,
            version,
        ) = self.fetch_resource_snapshot(&dev).await.unwrap_or_default();

        // Interfaces
        let interfaces = self.fetch_interfaces_snapshot(&dev).await.unwrap_or_default();

        // IP addresses
        let ip_addresses = self.fetch_ip_addresses_snapshot(&dev).await.unwrap_or_default();

        // Health (optional on some devices)
        let health = match self.fetch_health_snapshot(&dev).await {
            Ok(v) => Some(v),
            Err(e) if e.to_string().contains("health_not_supported") => None,
            Err(_) => None,
        };

        // Treat successful snapshot as an explicit online signal.
        let now = Utc::now();
        let _ = sqlx::query(
            r#"
            UPDATE mikrotik_routers SET
              is_online = true,
              last_seen_at = $1,
              latency_ms = $2,
              last_error = NULL,
              identity = COALESCE($3, identity),
              ros_version = COALESCE($4, ros_version),
              updated_at = $5
            WHERE id = $6 AND tenant_id = $7
            "#,
        )
        .bind(now)
        .bind(latency_ms)
        .bind(identity.clone())
        .bind(version.clone())
        .bind(now)
        .bind(&router.id)
        .bind(&router.tenant_id)
        .execute(&self.pool)
        .await;

        router.is_online = true;
        router.last_seen_at = Some(now);
        router.latency_ms = latency_ms;
        router.last_error = None;
        router.identity = identity.or(router.identity);
        router.ros_version = version.or(router.ros_version);
        router.updated_at = now;

        Ok(MikrotikRouterSnapshot {
            router,
            cpu_load,
            total_memory_bytes,
            free_memory_bytes,
            total_hdd_bytes,
            free_hdd_bytes,
            uptime_seconds,
            board_name,
            architecture,
            cpu,
            interfaces,
            ip_addresses,
            health,
        })
    }

    pub async fn test_connection(&self, tenant_id: &str, id: &str) -> AppResult<MikrotikTestResult> {
        let router = self
            .get_router(tenant_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".to_string()))?;

        let started = Instant::now();
        let latency_ms = Some(started.elapsed().as_millis().min(i32::MAX as u128) as i32);

        match self.connect_and_probe(&router).await {
            Ok((identity, version)) => {
                // Treat a successful test as an explicit "online" signal.
                let now = Utc::now();
                let _ = sqlx::query(
                    r#"
                    UPDATE mikrotik_routers SET
                      is_online = true,
                      last_seen_at = $1,
                      latency_ms = $2,
                      last_error = NULL,
                      identity = $3,
                      ros_version = $4,
                      updated_at = $5
                    WHERE id = $6 AND tenant_id = $7
                    "#,
                )
                .bind(now)
                .bind(latency_ms)
                .bind(identity.clone())
                .bind(version.clone())
                .bind(now)
                .bind(&router.id)
                .bind(&router.tenant_id)
                .execute(&self.pool)
                .await;

                Ok(MikrotikTestResult {
                    ok: true,
                    identity,
                    ros_version: version,
                    latency_ms,
                    error: None,
                })
            }
            Err(e) => {
                // Store last error so UI can surface it.
                let now = Utc::now();
                let msg = e.to_string();
                let _ = sqlx::query(
                    r#"
                    UPDATE mikrotik_routers SET
                      is_online = false,
                      latency_ms = $1,
                      last_error = $2,
                      updated_at = $3
                    WHERE id = $4 AND tenant_id = $5
                    "#,
                )
                .bind(latency_ms)
                .bind(&msg)
                .bind(now)
                .bind(&router.id)
                .bind(&router.tenant_id)
                .execute(&self.pool)
                .await;

                Ok(MikrotikTestResult {
                    ok: false,
                    identity: None,
                    ros_version: None,
                    latency_ms,
                    error: Some(msg),
                })
            }
        }
    }

    async fn connect_and_probe(
        &self,
        router: &MikrotikRouter,
    ) -> Result<(Option<String>, Option<String>), anyhow::Error> {
        // RouterOS API is plain TCP by default (8728). TLS is optional and not implemented here.
        let addr = format!("{}:{}", router.host, router.port);
        let password = decrypt_secret_opt(router.password.as_str())
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let dev = timeout(
            Duration::from_secs(5),
            MikrotikDevice::connect(addr, router.username.as_str(), password.as_deref()),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Connection timed out"))?
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        // identity
        let cmd_id = CommandBuilder::new()
            .command("/system/identity/print")
            .build();
        let mut rx = dev
            .send_command(cmd_id)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let mut identity: Option<String> = None;
        while let Some(res) = rx.recv().await {
            let r = res.map_err(|e| anyhow::anyhow!(e.to_string()))?;
            if let CommandResponse::Reply(reply) = r {
                identity = reply.attributes.get("name").and_then(|v| v.clone());
            }
        }

        // version
        let cmd_res = CommandBuilder::new()
            .command("/system/resource/print")
            .build();
        let mut rx2 = dev
            .send_command(cmd_res)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let mut version: Option<String> = None;
        while let Some(res) = rx2.recv().await {
            let r = res.map_err(|e| anyhow::anyhow!(e.to_string()))?;
            if let CommandResponse::Reply(reply) = r {
                version = reply.attributes.get("version").and_then(|v| v.clone());
            }
        }

        Ok((identity, version))
    }

    /// Background poller (best-effort).
    ///
    /// Default interval: 300s. Can be overridden by `MIKROTIK_POLL_INTERVAL_SECS`.
    pub fn start_poller(self: Arc<Self>) {
        tokio::spawn(async move {
            let interval_secs = std::env::var("MIKROTIK_POLL_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|v| *v >= 30 && *v <= 3600)
                .unwrap_or(300);

            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                if let Err(e) = self.poll_once().await {
                    warn!("[MikrotikPoller] Poll failed: {}", e);
                }
            }
        });
    }

    async fn poll_once(&self) -> AppResult<()> {
        let routers = sqlx::query_as::<_, MikrotikRouter>(
            r#"
            SELECT * FROM mikrotik_routers
            WHERE enabled = true
            ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        for router in routers {
            let _ = self.poll_router(router).await;
        }
        Ok(())
    }

    async fn poll_router(&self, router: MikrotikRouter) -> AppResult<()> {
        let started = Instant::now();
        let prev_online = router.is_online;
        let tenant_id = router.tenant_id.clone();

        let probe = self.connect_and_probe(&router).await;
        let now = Utc::now();
        let latency_ms = Some(started.elapsed().as_millis().min(i32::MAX as u128) as i32);

        match probe {
            Ok((identity, version)) => {
                // Basic resource snapshot
                let metric = self.fetch_resource_metric(&router).await.unwrap_or_else(|_| {
                    let mut m = MikrotikRouterMetric::new(router.id.clone());
                    m.ts = now;
                    m
                });

                // Update router status
                sqlx::query(
                    r#"
                    UPDATE mikrotik_routers SET
                      is_online = true,
                      last_seen_at = $1,
                      latency_ms = $2,
                      last_error = NULL,
                      identity = $3,
                      ros_version = $4,
                      updated_at = $5
                    WHERE id = $6
                    "#,
                )
                .bind(now)
                .bind(latency_ms)
                .bind(identity.clone())
                .bind(version.clone())
                .bind(now)
                .bind(&router.id)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;

                // Insert metric (best-effort)
                let _ = sqlx::query(
                    r#"
                    INSERT INTO mikrotik_router_metrics
                    (id, router_id, ts, cpu_load, total_memory_bytes, free_memory_bytes,
                     total_hdd_bytes, free_hdd_bytes, uptime_seconds, rx_bps, tx_bps)
                    VALUES
                    ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
                    "#,
                )
                .bind(&metric.id)
                .bind(&metric.router_id)
                .bind(metric.ts)
                .bind(metric.cpu_load)
                .bind(metric.total_memory_bytes)
                .bind(metric.free_memory_bytes)
                .bind(metric.total_hdd_bytes)
                .bind(metric.free_hdd_bytes)
                .bind(metric.uptime_seconds)
                .bind(metric.rx_bps)
                .bind(metric.tx_bps)
                .execute(&self.pool)
                .await;

                // Per-interface metrics (best-effort). Also compute aggregate rx/tx bps.
                if let Ok((rx_bps, tx_bps)) = self.poll_interface_metrics(&router, now).await {
                    if rx_bps.is_some() || tx_bps.is_some() {
                        let _ = sqlx::query(
                            r#"
                            UPDATE mikrotik_router_metrics
                            SET rx_bps = $1, tx_bps = $2
                            WHERE id = $3
                            "#,
                        )
                        .bind(rx_bps)
                        .bind(tx_bps)
                        .bind(&metric.id)
                        .execute(&self.pool)
                        .await;
                    }
                }

                if !prev_online {
                    self.notify_tenant(
                        &tenant_id,
                        "Router online",
                        format!("{} is back online.", router.name),
                        Some(format!("/admin/network/routers/{}", router.id)),
                        "success",
                    )
                    .await;

                    self.audit_service
                        .log(
                            None,
                            Some(&tenant_id),
                            "status_online",
                            "mikrotik_router",
                            Some(&router.id),
                            Some(&format!("{} is back online", router.name)),
                            None,
                        )
                        .await;
                }
            }
            Err(e) => {
                let msg = e.to_string();
                sqlx::query(
                    r#"
                    UPDATE mikrotik_routers SET
                      is_online = false,
                      latency_ms = $1,
                      last_error = $2,
                      updated_at = $3
                    WHERE id = $4
                    "#,
                )
                .bind(latency_ms)
                .bind(&msg)
                .bind(now)
                .bind(&router.id)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;

                if prev_online {
                    self.notify_tenant(
                        &tenant_id,
                        "Router offline",
                        format!("{} is unreachable.", router.name),
                        Some(format!("/admin/network/routers/{}", router.id)),
                        "error",
                    )
                    .await;

                    self.audit_service
                        .log(
                            None,
                            Some(&tenant_id),
                            "status_offline",
                            "mikrotik_router",
                            Some(&router.id),
                            Some(&format!("{} became unreachable: {}", router.name, msg)),
                            None,
                        )
                        .await;
                }
            }
        }

        info!(
            "[MikrotikPoller] {} ({}) polled in {}ms",
            router.name,
            router.host,
            started.elapsed().as_millis()
        );

        Ok(())
    }

    async fn poll_interface_metrics(
        &self,
        router: &MikrotikRouter,
        ts: DateTime<Utc>,
    ) -> Result<(Option<i64>, Option<i64>), anyhow::Error> {
        #[derive(sqlx::FromRow, Debug)]
        struct PrevIfaceRow {
            interface_name: String,
            ts: DateTime<Utc>,
            rx_byte: Option<i64>,
            tx_byte: Option<i64>,
        }

        let password = decrypt_secret_opt(router.password.as_str())?;
        let addr = format!("{}:{}", router.host, router.port);
        let dev = timeout(
            Duration::from_secs(5),
            MikrotikDevice::connect(addr, router.username.as_str(), password.as_deref()),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Connection timed out"))?
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let interfaces = self.fetch_interfaces_snapshot(&dev).await?;

        // Fetch last metrics for all interfaces in one shot.
        let mut prev_rows = sqlx::query_as::<_, PrevIfaceRow>(
            r#"
            SELECT interface_name, ts, rx_byte, tx_byte
            FROM mikrotik_interface_metrics
            WHERE router_id = $1
            ORDER BY interface_name ASC, ts DESC
            "#,
        )
        .bind(&router.id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mut prev_map: std::collections::HashMap<String, PrevIfaceRow> = std::collections::HashMap::new();
        for r in prev_rows.drain(..) {
            if prev_map.contains_key(&r.interface_name) {
                continue;
            }
            prev_map.insert(r.interface_name.clone(), r);
        }

        let mut sum_rx: Option<i64> = None;
        let mut sum_tx: Option<i64> = None;

        for it in interfaces {
            let prev = prev_map.get(&it.name);
            let mut m = MikrotikInterfaceMetric::new(router.id.clone(), it.name.clone());
            m.ts = ts;
            m.rx_byte = it.rx_byte;
            m.tx_byte = it.tx_byte;
            m.running = it.running;
            m.disabled = it.disabled;
            m.link_downs = it.link_downs;

            if let (Some(prev_row), Some(cur_rx), Some(prev_rx)) = (prev, it.rx_byte, prev.and_then(|p| p.rx_byte)) {
                let dt = (ts - prev_row.ts).num_milliseconds() as f64 / 1000.0;
                if dt > 0.0 {
                    let delta = cur_rx - prev_rx;
                    if delta >= 0 {
                        let bps = ((delta as f64) * 8.0 / dt).round() as i64;
                        m.rx_bps = Some(bps);
                        sum_rx = Some(sum_rx.unwrap_or(0) + bps);
                    }
                }
            }

            if let (Some(prev_row), Some(cur_tx), Some(prev_tx)) = (prev, it.tx_byte, prev.and_then(|p| p.tx_byte)) {
                let dt = (ts - prev_row.ts).num_milliseconds() as f64 / 1000.0;
                if dt > 0.0 {
                    let delta = cur_tx - prev_tx;
                    if delta >= 0 {
                        let bps = ((delta as f64) * 8.0 / dt).round() as i64;
                        m.tx_bps = Some(bps);
                        sum_tx = Some(sum_tx.unwrap_or(0) + bps);
                    }
                }
            }

            let _ = sqlx::query(
                r#"
                INSERT INTO mikrotik_interface_metrics
                (id, router_id, interface_name, ts, rx_byte, tx_byte, rx_bps, tx_bps, running, disabled, link_downs)
                VALUES
                ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
                "#,
            )
            .bind(&m.id)
            .bind(&m.router_id)
            .bind(&m.interface_name)
            .bind(m.ts)
            .bind(m.rx_byte)
            .bind(m.tx_byte)
            .bind(m.rx_bps)
            .bind(m.tx_bps)
            .bind(m.running)
            .bind(m.disabled)
            .bind(m.link_downs)
            .execute(&self.pool)
            .await;
        }

        Ok((sum_rx, sum_tx))
    }

    async fn fetch_resource_metric(
        &self,
        router: &MikrotikRouter,
    ) -> Result<MikrotikRouterMetric, anyhow::Error> {
        let addr = format!("{}:{}", router.host, router.port);
        let password = decrypt_secret_opt(router.password.as_str())
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let dev = timeout(
            Duration::from_secs(5),
            MikrotikDevice::connect(addr, router.username.as_str(), password.as_deref()),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Connection timed out"))?
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let cmd = CommandBuilder::new()
            .command("/system/resource/print")
            .build();
        let mut rx = dev
            .send_command(cmd)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mut metric = MikrotikRouterMetric::new(router.id.clone());
        metric.ts = Utc::now();

        while let Some(res) = rx.recv().await {
            let r = res.map_err(|e| anyhow::anyhow!(e.to_string()))?;
            if let CommandResponse::Reply(reply) = r {
                metric.cpu_load = reply
                    .attributes
                    .get("cpu-load")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<i32>().ok()));
                metric.total_memory_bytes = reply
                    .attributes
                    .get("total-memory")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok()));
                metric.free_memory_bytes = reply
                    .attributes
                    .get("free-memory")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok()));
                metric.total_hdd_bytes = reply
                    .attributes
                    .get("total-hdd-space")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok()));
                metric.free_hdd_bytes = reply
                    .attributes
                    .get("free-hdd-space")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok()));
                metric.uptime_seconds = reply
                    .attributes
                    .get("uptime")
                    .and_then(|v| v.as_deref().map(parse_uptime_to_secs));
            }
        }

        Ok(metric)
    }

    async fn fetch_resource_snapshot(
        &self,
        dev: &MikrotikDevice,
    ) -> Result<
        (
            Option<i32>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
        anyhow::Error,
    > {
        let cmd = CommandBuilder::new()
            .command("/system/resource/print")
            .build();
        let mut rx = dev
            .send_command(cmd)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mut cpu_load: Option<i32> = None;
        let mut total_memory_bytes: Option<i64> = None;
        let mut free_memory_bytes: Option<i64> = None;
        let mut total_hdd_bytes: Option<i64> = None;
        let mut free_hdd_bytes: Option<i64> = None;
        let mut uptime_seconds: Option<i64> = None;
        let mut board_name: Option<String> = None;
        let mut architecture: Option<String> = None;
        let mut cpu: Option<String> = None;
        let mut version: Option<String> = None;

        while let Some(res) = rx.recv().await {
            let r = res.map_err(|e| anyhow::anyhow!(e.to_string()))?;
            if let CommandResponse::Reply(reply) = r {
                cpu_load = reply
                    .attributes
                    .get("cpu-load")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<i32>().ok()));
                total_memory_bytes = reply
                    .attributes
                    .get("total-memory")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok()));
                free_memory_bytes = reply
                    .attributes
                    .get("free-memory")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok()));
                total_hdd_bytes = reply
                    .attributes
                    .get("total-hdd-space")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok()));
                free_hdd_bytes = reply
                    .attributes
                    .get("free-hdd-space")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok()));
                uptime_seconds = reply
                    .attributes
                    .get("uptime")
                    .and_then(|v| v.as_deref().map(parse_uptime_to_secs));

                board_name = reply.attributes.get("board-name").and_then(|v| v.clone());
                architecture = reply
                    .attributes
                    .get("architecture-name")
                    .and_then(|v| v.clone());
                cpu = reply.attributes.get("cpu").and_then(|v| v.clone());
                version = reply.attributes.get("version").and_then(|v| v.clone());
            }
        }

        Ok((
            cpu_load,
            total_memory_bytes,
            free_memory_bytes,
            total_hdd_bytes,
            free_hdd_bytes,
            uptime_seconds,
            board_name,
            architecture,
            cpu,
            version,
        ))
    }

    async fn fetch_identity_snapshot(
        &self,
        dev: &MikrotikDevice,
    ) -> Result<Option<String>, anyhow::Error> {
        let cmd = CommandBuilder::new()
            .command("/system/identity/print")
            .build();
        let mut rx = dev
            .send_command(cmd)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mut identity: Option<String> = None;
        while let Some(res) = rx.recv().await {
            let r = res.map_err(|e| anyhow::anyhow!(e.to_string()))?;
            if let CommandResponse::Reply(reply) = r {
                identity = reply.attributes.get("name").and_then(|v| v.clone());
            }
        }

        Ok(identity)
    }

    async fn fetch_interfaces_snapshot(
        &self,
        dev: &MikrotikDevice,
    ) -> Result<Vec<MikrotikInterfaceSnapshot>, anyhow::Error> {
        let cmd = CommandBuilder::new()
            .command("/interface/print")
            .build();
        let mut rx = dev
            .send_command(cmd)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mut out: Vec<MikrotikInterfaceSnapshot> = vec![];
        while let Some(res) = rx.recv().await {
            let r = res.map_err(|e| anyhow::anyhow!(e.to_string()))?;
            if let CommandResponse::Reply(reply) = r {
                let name = reply
                    .attributes
                    .get("name")
                    .and_then(|v| v.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let running = reply
                    .attributes
                    .get("running")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<bool>().ok()));
                let disabled = reply
                    .attributes
                    .get("disabled")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<bool>().ok()));
                let mtu = reply
                    .attributes
                    .get("mtu")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<i32>().ok()));

                out.push(MikrotikInterfaceSnapshot {
                    name,
                    interface_type: reply.attributes.get("type").and_then(|v| v.clone()),
                    running,
                    disabled,
                    mtu,
                    mac_address: reply
                        .attributes
                        .get("mac-address")
                        .and_then(|v| v.clone()),
                    rx_byte: reply
                        .attributes
                        .get("rx-byte")
                        .and_then(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok())),
                    tx_byte: reply
                        .attributes
                        .get("tx-byte")
                        .and_then(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok())),
                    rx_packet: reply
                        .attributes
                        .get("rx-packet")
                        .and_then(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok())),
                    tx_packet: reply
                        .attributes
                        .get("tx-packet")
                        .and_then(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok())),
                    link_downs: reply
                        .attributes
                        .get("link-downs")
                        .and_then(|v| v.as_ref().and_then(|s| s.parse::<i64>().ok())),
                });
            }
        }

        // Stable sort for UX
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(out)
    }

    async fn fetch_ip_addresses_snapshot(
        &self,
        dev: &MikrotikDevice,
    ) -> Result<Vec<MikrotikIpAddressSnapshot>, anyhow::Error> {
        let cmd = CommandBuilder::new()
            .command("/ip/address/print")
            .build();
        let mut rx = dev
            .send_command(cmd)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mut out: Vec<MikrotikIpAddressSnapshot> = vec![];
        while let Some(res) = rx.recv().await {
            let r = res.map_err(|e| anyhow::anyhow!(e.to_string()))?;
            if let CommandResponse::Reply(reply) = r {
                let address = reply
                    .attributes
                    .get("address")
                    .and_then(|v| v.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                let disabled = reply
                    .attributes
                    .get("disabled")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<bool>().ok()));
                let dynamic = reply
                    .attributes
                    .get("dynamic")
                    .and_then(|v| v.as_ref().and_then(|s| s.parse::<bool>().ok()));

                out.push(MikrotikIpAddressSnapshot {
                    address,
                    network: reply.attributes.get("network").and_then(|v| v.clone()),
                    interface: reply.attributes.get("interface").and_then(|v| v.clone()),
                    disabled,
                    dynamic,
                });
            }
        }

        Ok(out)
    }

    async fn fetch_health_snapshot(&self, dev: &MikrotikDevice) -> Result<MikrotikHealthSnapshot, anyhow::Error> {
        let cmd = CommandBuilder::new()
            .command("/system/health/print")
            .build();
        let mut rx = dev
            .send_command(cmd)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mut temperature_c: Option<f64> = None;
        let mut voltage_v: Option<f64> = None;
        let mut cpu_temperature_c: Option<f64> = None;

        while let Some(res) = rx.recv().await {
            let r = res.map_err(|e| anyhow::anyhow!(e.to_string()))?;
            match r {
                CommandResponse::Reply(reply) => {
                    // RouterOS returns varying keys depending on hardware.
                    temperature_c = reply
                        .attributes
                        .get("temperature")
                        .and_then(|v| v.as_ref().and_then(|s| s.parse::<f64>().ok()))
                        .or_else(|| {
                            reply
                                .attributes
                                .get("board-temperature1")
                                .and_then(|v| v.as_ref().and_then(|s| s.parse::<f64>().ok()))
                        });
                    cpu_temperature_c = reply
                        .attributes
                        .get("cpu-temperature")
                        .and_then(|v| v.as_ref().and_then(|s| s.parse::<f64>().ok()));
                    voltage_v = reply
                        .attributes
                        .get("voltage")
                        .and_then(|v| v.as_ref().and_then(|s| s.parse::<f64>().ok()));
                }
                CommandResponse::Trap(_trap) => {
                    // Command not supported on this device; treat as absent.
                    return Err(anyhow::anyhow!("health_not_supported"));
                }
                _ => {}
            }
        }

        Ok(MikrotikHealthSnapshot {
            temperature_c,
            voltage_v,
            cpu_temperature_c,
        })
    }

    async fn notify_tenant(
        &self,
        tenant_id: &str,
        title: &str,
        message: String,
        action_url: Option<String>,
        notification_type: &str,
    ) {
        // Send to all tenant members who have manage/read access to routers.
        let user_ids: Result<Vec<String>, sqlx::Error> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT tm.user_id
            FROM tenant_members tm
            JOIN role_permissions rp ON rp.role_id = tm.role_id
            JOIN permissions p ON p.id = rp.permission_id
            WHERE tm.tenant_id = $1
              AND p.resource = 'network_routers'
              AND p.action IN ('read','manage')
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await;

        let user_ids = match user_ids {
            Ok(v) => v,
            Err(_) => return,
        };

        for uid in user_ids {
            let _ = self
                .notification_service
                .create_notification(
                    uid,
                    Some(tenant_id.to_string()),
                    title.to_string(),
                    message.clone(),
                    notification_type.to_string(),
                    "network".to_string(),
                    action_url.clone(),
                )
                .await;
        }
    }
}

fn parse_uptime_to_secs(s: &str) -> i64 {
    // RouterOS uptime string example: "1w2d3h4m5s" or "3h12m" etc.
    let mut total: i64 = 0;
    let mut num = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            num.push(ch);
            continue;
        }
        let val: i64 = num.parse().unwrap_or(0);
        num.clear();
        match ch {
            'w' => total += val * 7 * 24 * 3600,
            'd' => total += val * 24 * 3600,
            'h' => total += val * 3600,
            'm' => total += val * 60,
            's' => total += val,
            _ => {}
        }
    }
    total
}
