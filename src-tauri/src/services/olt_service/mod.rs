//! OLT (Optical Line Terminal) Monitoring Service
//!
//! Multi-vendor OLT management with driver abstraction.
//! Provides CRUD inventory, real-time monitoring with caching,
//! ONU detail queries, reboot, and history tracking.

pub mod drivers;

use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    AllOnusResponse, CreateOltRequest, Olt, OltAllDetailsResponse, OltOnuDetail,
    OltOnuHistoryRecord, OltStatsResponse, RebootOnuRequest, TestConnectionResponse,
    UpdateOltRequest,
};
use crate::security::secret::{decrypt_secret_opt, encrypt_secret};
use crate::services::audit_service::AuditService;
use crate::services::notification_service::NotificationService;
use chrono::Utc;
use drivers::create_driver;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use uuid::Uuid;

pub fn pool_ref(pool: &DbPool) -> &DbPool {
    pool
}

pub struct OltService {
    pool: DbPool,
    notification_service: NotificationService,
    audit_service: AuditService,
}

impl OltService {
    pub fn new(
        pool: DbPool,
        notification_service: NotificationService,
        audit_service: AuditService,
    ) -> Self {
        Self {
            pool,
            notification_service,
            audit_service,
        }
    }

    // ── CRUD ──────────────────────────────────────────────

    pub async fn list_olts(&self, tenant_id: &str) -> AppResult<Vec<Olt>> {
        let olts = sqlx::query_as::<_, Olt>(
            "SELECT * FROM public.olts WHERE tenant_id = $1 ORDER BY name",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(olts)
    }

    pub async fn get_olt(&self, id: &str, tenant_id: &str) -> AppResult<Olt> {
        let olt = sqlx::query_as::<_, Olt>(
            "SELECT * FROM public.olts WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("OLT not found".into()))?;
        Ok(olt)
    }

    pub async fn create_olt(
        &self,
        tenant_id: &str,
        req: CreateOltRequest,
    ) -> AppResult<Olt> {
        let olt = Olt::new(
            tenant_id.to_string(),
            req.name,
            req.description,
            req.olt_type,
            req.host,
            req.port,
            req.username,
            Some(encrypt_secret(&req.password)?),
        );

        sqlx::query(
            "INSERT INTO public.olts (id, tenant_id, name, description, olt_type, host, port, username, password_enc, is_online, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&olt.id)
        .bind(&olt.tenant_id)
        .bind(&olt.name)
        .bind(&olt.description)
        .bind(&olt.olt_type)
        .bind(&olt.host)
        .bind(olt.port)
        .bind(&olt.username)
        .bind(&olt.password_enc)
        .bind(olt.is_online)
        .bind(olt.created_at)
        .bind(olt.updated_at)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                None,
                Some(tenant_id),
                "olt_created",
                "olt",
                Some(&olt.id),
                Some(&format!("Created OLT: {}", olt.name)),
                None,
            )
            .await;

        // ── Auto-create NetworkAsset ──────────────────────
        let asset_type = format!("olt_{}", olt.olt_type);
        let vendor = match olt.olt_type.as_str() {
            "hioso_ha7302cst" => "HIOSO",
            "vsol_epon" => "VSOL",
            _ => "Unknown",
        };
        let now = Utc::now();
        let asset_id = Uuid::new_v4().to_string();
        let metadata = serde_json::json!({
            "olt_id": olt.id,
            "host": olt.host,
            "olt_type": olt.olt_type,
        });

        // Use raw SQL to avoid circular dependency with NetworkAssetService
        let _ = sqlx::query(
            "INSERT INTO public.network_assets
             (id, tenant_id, asset_group, asset_type, name, vendor, status, notes, metadata, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&asset_id)
        .bind(tenant_id)
        .bind("olt")
        .bind(&asset_type)
        .bind(&olt.name)
        .bind(vendor)
        .bind("active")
        .bind(&olt.description)
        .bind(metadata)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await;

        Ok(olt)
    }

    pub async fn update_olt(
        &self,
        id: &str,
        tenant_id: &str,
        req: UpdateOltRequest,
    ) -> AppResult<Olt> {
        let existing = self.get_olt(id, tenant_id).await?;

        let name = req.name.unwrap_or(existing.name);
        let description = req.description.or(existing.description);
        let host = req.host.unwrap_or(existing.host);
        let port = req.port.unwrap_or(existing.port);
        let username = req.username.unwrap_or(existing.username);
        let password_enc = if let Some(ref pw) = req.password {
            Some(encrypt_secret(pw)?)
        } else {
            existing.password_enc
        };

        sqlx::query(
            "UPDATE public.olts SET name = $1, description = $2, host = $3, port = $4, username = $5, password_enc = $6, updated_at = now()
             WHERE id = $7 AND tenant_id = $8",
        )
        .bind(&name)
        .bind(&description)
        .bind(&host)
        .bind(port)
        .bind(&username)
        .bind(&password_enc)
        .bind(id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                None,
                Some(tenant_id),
                "olt_updated",
                "olt",
                Some(id),
                Some(&format!("Updated OLT: {}", name)),
                None,
            )
            .await;

        self.get_olt(id, tenant_id).await
    }

    pub async fn delete_olt(&self, id: &str, tenant_id: &str) -> AppResult<()> {
        let olt = self.get_olt(id, tenant_id).await?;

        sqlx::query("DELETE FROM public.olts WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;

        self.audit_service
            .log(
                None,
                Some(tenant_id),
                "olt_deleted",
                "olt",
                Some(id),
                Some(&format!("Deleted OLT: {}", olt.name)),
                None,
            )
            .await;

        Ok(())
    }

    // ── Connection & Monitoring ───────────────────────────

    /// Test connection to OLT without saving anything
    pub async fn test_connection(
        &self,
        _tenant_id: &str,
        host: &str,
        port: i32,
        username: &str,
        password: &str,
        olt_type: &str,
    ) -> AppResult<TestConnectionResponse> {
        let mut driver = create_driver(olt_type)?;
        match driver
            .connect(host, port as u16, username, password)
            .await
        {
            Ok(()) => {
                let info = driver.get_system_info().await.ok();
                driver.disconnect().await.ok();
                Ok(TestConnectionResponse {
                    success: true,
                    info,
                    error: None,
                })
            }
            Err(e) => Ok(TestConnectionResponse {
                success: false,
                info: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get OLT statistics (cache-aware)
    pub async fn get_olt_stats(
        &self,
        id: &str,
        tenant_id: &str,
        force_refresh: bool,
    ) -> AppResult<OltStatsResponse> {
        let olt = self.get_olt(id, tenant_id).await?;

        // Return cached if available and not forcing refresh
        if !force_refresh && olt.last_stats.is_some() {
            let stats: crate::models::OltGlobalStats =
                serde_json::from_value(olt.last_stats.clone().unwrap())
                    .map_err(|e| AppError::Internal(format!("Cache parse: {}", e)))?;
            return Ok(OltStatsResponse {
                status: "success".into(),
                data: stats,
                info: None,
                cached: true,
                is_online: olt.is_online,
                updated_at: olt.last_updated.map(|d| d.to_rfc3339()),
            });
        }

        // Decrypt password
        let password = decrypt_secret_opt(&olt.password_enc.unwrap_or_default())
            .unwrap_or_default()
            .unwrap_or_default();

        // Connect and fetch
        let mut driver = create_driver(&olt.olt_type)?;
        match driver
            .connect(&olt.host, olt.port as u16, &olt.username, &password)
            .await
        {
            Ok(()) => {
                let stats = driver.get_global_stats().await?;
                let info = driver.get_system_info().await.ok();
                driver.disconnect().await.ok();

                let stats_json = serde_json::to_value(&stats)
                    .map_err(|e| AppError::Internal(format!("Serialize: {}", e)))?;

                // Update cache in DB
                sqlx::query(
                    "UPDATE public.olts SET last_stats = $1, last_updated = now(), is_online = true, last_polled_at = now(), last_error = NULL WHERE id = $2",
                )
                .bind(&stats_json)
                .bind(id)
                .execute(&self.pool)
                .await?;

                Ok(OltStatsResponse {
                    status: "success".into(),
                    data: stats,
                    info,
                    cached: false,
                    is_online: true,
                    updated_at: Some(Utc::now().to_rfc3339()),
                })
            }
            Err(e) => {
                let err_msg = e.to_string();
                sqlx::query(
                    "UPDATE public.olts SET is_online = false, last_error = $1, last_polled_at = now() WHERE id = $2",
                )
                .bind(&err_msg)
                .bind(id)
                .execute(&self.pool)
                .await?;

                Err(AppError::Internal(format!(
                    "OLT connection failed: {}",
                    err_msg
                )))
            }
        }
    }

    /// Get all ONU details from a specific OLT
    pub async fn get_olt_all_details(
        &self,
        id: &str,
        tenant_id: &str,
    ) -> AppResult<OltAllDetailsResponse> {
        let olt = self.get_olt(id, tenant_id).await?;
        let password = decrypt_secret_opt(&olt.password_enc.unwrap_or_default())
            .unwrap_or_default()
            .unwrap_or_default();

        let mut driver = create_driver(&olt.olt_type)?;
        driver
            .connect(&olt.host, olt.port as u16, &olt.username, &password)
            .await
            .map_err(|e| AppError::Internal(format!("Connection failed: {}", e)))?;

        let stats = driver.get_global_stats().await?;
        let info = driver.get_system_info().await?;

        // Collect all ONUs across PON ports
        let mut all_onus = Vec::new();
        for p in &stats.pon_ports {
            if let Ok(onus) = driver.get_pon_onu_details(&p.name).await {
                all_onus.extend(onus.into_iter().map(|mut o| {
                    o.olt_id = Some(id.to_string());
                    o.olt_name = Some(olt.name.clone());
                    o
                }));
            }
        }

        driver.disconnect().await.ok();

        // Save ONU history
        self.save_onu_history(id, tenant_id, &all_onus).await.ok();

        // Update cache
        if let Ok(stats_json) = serde_json::to_value(&stats) {
            sqlx::query(
                "UPDATE public.olts SET last_stats = $1, last_updated = now(), is_online = true WHERE id = $2",
            )
            .bind(&stats_json)
            .bind(id)
            .execute(&self.pool)
            .await
            .ok();
        }

        Ok(OltAllDetailsResponse {
            status: "success".into(),
            info,
            onus: all_onus,
            stats,
        })
    }

    /// Get ALL ONUs across ALL OLTs (for global search)
    pub async fn get_all_onus(&self, tenant_id: &str) -> AppResult<AllOnusResponse> {
        let olts = self.list_olts(tenant_id).await?;
        let mut all_onus = Vec::new();

        for olt in &olts {
            let password = decrypt_secret_opt(&olt.password_enc.clone().unwrap_or_default())
                .unwrap_or_default()
                .unwrap_or_default();

            let mut driver = match create_driver(&olt.olt_type) {
                Ok(d) => d,
                Err(_) => continue,
            };

            if driver
                .connect(&olt.host, olt.port as u16, &olt.username, &password)
                .await
                .is_err()
            {
                continue;
            }

            if let Ok(stats) = driver.get_global_stats().await {
                for p in &stats.pon_ports {
                    if let Ok(onus) = driver.get_pon_onu_details(&p.name).await {
                        all_onus.extend(onus.into_iter().map(|mut o| {
                            o.olt_id = Some(olt.id.clone());
                            o.olt_name = Some(olt.name.clone());
                            o
                        }));
                    }
                }
            }

            driver.disconnect().await.ok();
        }

        Ok(AllOnusResponse {
            status: "success".into(),
            data: all_onus,
        })
    }

    /// Reboot an ONU on a specific OLT
    pub async fn reboot_onu(
        &self,
        id: &str,
        tenant_id: &str,
        req: RebootOnuRequest,
    ) -> AppResult<serde_json::Value> {
        let olt = self.get_olt(id, tenant_id).await?;
        let password = decrypt_secret_opt(&olt.password_enc.unwrap_or_default())
            .unwrap_or_default()
            .unwrap_or_default();

        let mut driver = create_driver(&olt.olt_type)?;
        driver
            .connect(&olt.host, olt.port as u16, &olt.username, &password)
            .await
            .map_err(|e| AppError::Internal(format!("Connection failed: {}", e)))?;

        let ok = driver.reboot_onu(&req.onu_id, &req.onu_name).await?;
        driver.disconnect().await.ok();

        if ok {
            self.audit_service
                .log(
                    None,
                    Some(tenant_id),
                    "onu_reboot",
                    "olt_onu",
                    Some(&req.onu_id),
                    Some(&format!(
                        "Rebooted ONU {} on OLT {}",
                        req.onu_name, olt.name
                    )),
                    None,
                )
                .await;

            Ok(serde_json::json!({"status": "success", "message": "Reboot command sent"}))
        } else {
            Err(AppError::Internal(
                "Failed to send reboot command".into(),
            ))
        }
    }

    // ── ONU History ───────────────────────────────────────

    /// Save ONU signal/status history to the database
    pub async fn save_onu_history(
        &self,
        olt_id: &str,
        tenant_id: &str,
        onus: &[OltOnuDetail],
    ) -> AppResult<()> {
        for onu in onus {
            let rx: Option<f64> = onu.rx.replace("dBm", "").trim().parse().ok();
            let tx: Option<f64> = onu
                .tx
                .as_ref()
                .and_then(|s| s.replace("dBm", "").trim().parse().ok());

            sqlx::query(
                "INSERT INTO public.olt_onu_history (id, olt_id, tenant_id, onu_id, pon, mac, name, status, rx_power, tx_power, distance, temperature)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(olt_id)
            .bind(tenant_id)
            .bind(&onu.onu_id)
            .bind(&onu.pon)
            .bind(&onu.mac)
            .bind(&onu.name)
            .bind(&onu.status)
            .bind(rx)
            .bind(tx)
            .bind(onu.distance.as_ref().and_then(|s| s.replace("km", "").trim().parse::<f64>().ok()))
            .bind(onu.temperature.as_ref().and_then(|s| s.trim().parse::<f64>().ok()))
            .execute(&self.pool)
            .await?;
        }

        // ── Low-signal alert pipeline ─────────────────────
        // Check for ONUs with dangerously low RX power and notify tenant admins
        const LOW_SIGNAL_DBM: f64 = -24.0;
        const LOW_SIGNAL_FLOOR_DBM: f64 = -50.0;

        for onu in onus {
            if onu.status != "Online" {
                continue;
            }
            let rx: f64 = match onu.rx.replace("dBm", "").trim().parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            if rx > LOW_SIGNAL_DBM || rx < LOW_SIGNAL_FLOOR_DBM {
                continue;
            }

            // Find admin users for this tenant to receive the alert
            let admin_users: Vec<(String,)> = sqlx::query_as(
                "SELECT u.id::text FROM users u
                 WHERE u.tenant_id = $1::uuid
                   AND u.role IN ('owner', 'admin')
                 LIMIT 3",
            )
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            let title = "⚠️ Sinyal ONU Rendah".to_string();
            let message = format!(
                "ONU {} (MAC: {}) di OLT — sinyal rendah: {:.1} dBm",
                onu.name, onu.mac, rx,
            );

            for (user_id,) in &admin_users {
                let _ = self
                    .notification_service
                    .create_notification(
                        user_id.clone(),
                        Some(tenant_id.to_string()),
                        title.clone(),
                        message.clone(),
                        "warning".to_string(),
                        "olt_alert".to_string(),
                        Some(format!("/admin/olts/{}", olt_id)),
                    )
                    .await;
            }

            // Throttle: only alert for the first low-signal ONU per save cycle
            break;
        }

        Ok(())
    }

    /// Get recent ONU history records
    pub async fn get_onu_history(
        &self,
        olt_id: &str,
        tenant_id: &str,
        limit: i64,
    ) -> AppResult<Vec<OltOnuHistoryRecord>> {
        let records = sqlx::query_as::<_, OltOnuHistoryRecord>(
            "SELECT * FROM public.olt_onu_history WHERE olt_id = $1 AND tenant_id = $2 ORDER BY recorded_at DESC LIMIT $3",
        )
        .bind(olt_id)
        .bind(tenant_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    // ── Background Poller ────────────────────────────────

    /// Start background OLT poller. Returns immediately, runs forever.
    /// Polls all OLTs every 30 seconds and pushes stats via WebSocket.
    pub fn start_poller(self: Arc<Self>, ws_hub: Arc<crate::http::WsHub>) {
        tokio::spawn(async move {
            // Wait 5 seconds before first poll to let the server fully start
            tokio::time::sleep(Duration::from_secs(5)).await;

            let mut tick = interval(Duration::from_secs(30));
            loop {
                tick.tick().await;
                if let Err(e) = self.poll_all_olts(&ws_hub).await {
                    tracing::warn!("OLT poller cycle error: {}", e);
                }
            }
        });
    }

    /// Poll all OLTs across all tenants and push updates via WebSocket
    async fn poll_all_olts(
        &self,
        ws_hub: &crate::http::WsHub,
    ) -> AppResult<()> {
        // Query all tenants with OLTs
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT tenant_id::text FROM public.olts WHERE tenant_id IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;

        for (tenant_id,) in &rows {
            let olts = self.list_olts(tenant_id).await.unwrap_or_default();

            for olt in &olts {
                match self.get_olt_stats(&olt.id, tenant_id, true).await {
                    Ok(resp) => {
                        // Push to WebSocket for real-time UI updates
                        ws_hub.broadcast(crate::http::WsEvent::OltStatsUpdate {
                            tenant_id: tenant_id.clone(),
                            olt_id: olt.id.clone(),
                            olt_name: olt.name.clone(),
                            is_online: resp.is_online,
                            stats: serde_json::to_value(&resp.data)
                                .unwrap_or(serde_json::json!({})),
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Poll failed for OLT {}: {}", olt.name, e);
                    }
                }
            }
        }

        Ok(())
    }
}
