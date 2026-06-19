//! OLT (Optical Line Terminal) Monitoring Service
//!
//! Multi-vendor OLT management with driver abstraction.
//! Provides CRUD inventory, real-time monitoring with caching,
//! ONU detail queries, reboot, and history tracking.

pub mod drivers;

use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    AllOnusResponse, CreateNetworkAssetRequest, CreateOltRequest, Olt, OltAllDetailsResponse,
    OltOnuDetail, OltOnuHistoryRecord, OltPublicToken, OltStatsResponse, RebootOnuRequest,
    TestConnectionResponse, UpdateNetworkAssetRequest, UpdateOltRequest,
};
use crate::security::secret::{decrypt_secret_opt, encrypt_secret};
use crate::services::audit_service::AuditService;
use crate::services::network_asset_service::NetworkAssetService;
use crate::services::network_mapping_service::NetworkMappingService;
use crate::services::notification_service::NotificationService;
use crate::services::onu_linker::OnuLinker;
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
    network_asset_service: Arc<NetworkAssetService>,
    network_mapping_service: Arc<NetworkMappingService>,
    onu_linker: OnuLinker,
}

impl OltService {
    pub fn new(
        pool: DbPool,
        notification_service: NotificationService,
        audit_service: AuditService,
        network_asset_service: Arc<NetworkAssetService>,
        network_mapping_service: Arc<NetworkMappingService>,
        onu_linker: OnuLinker,
    ) -> Self {
        Self {
            pool,
            notification_service,
            audit_service,
            network_asset_service,
            network_mapping_service,
            onu_linker,
        }
    }

    /// Sprint C: best-effort sync to network_mapping_service. Failures are
    /// logged at warn but do not bubble up — the OLT CRUD has already succeeded.
    async fn sync_to_topology(
        &self,
        actor_id: &str,
        tenant_id: &str,
        context: &str,
    ) {
        match self
            .network_mapping_service
            .sync_topology_asset_nodes(actor_id, tenant_id)
            .await
        {
            Ok(resp) => tracing::debug!(
                "Sprint C: topology sync after {}: touched={} assets_created={} assets_updated={}",
                context, resp.total_nodes_touched, resp.asset_nodes_created, resp.asset_nodes_updated
            ),
            Err(e) => tracing::warn!(
                "Sprint C: topology sync after {} failed: {}",
                context, e
            ),
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
        actor_id: &str,
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
            req.latitude,
            req.longitude,
            req.address_line,
            req.uplink_router_id,
            req.uplink_port,
        );

        sqlx::query(
            "INSERT INTO public.olts (id, tenant_id, name, description, olt_type, host, port, username, password_enc, is_online, latitude, longitude, address_line, uplink_router_id, uplink_port, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
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
        .bind(olt.latitude)
        .bind(olt.longitude)
        .bind(&olt.address_line)
        .bind(olt.uplink_router_id)
        .bind(&olt.uplink_port)
        .bind(olt.created_at)
        .bind(olt.updated_at)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "olt_created",
                "olt",
                Some(&olt.id),
                Some(&format!("Created OLT: {}", olt.name)),
                None,
            )
            .await;

        // ── Auto-create NetworkAsset via NetworkAssetService ──
        let vendor = match olt.olt_type.as_str() {
            "hioso_ha7302cst" => "HIOSO",
            "vsol_epon" => "VSOL",
            _ => "Unknown",
        };
        let metadata = serde_json::json!({
            "olt_id": olt.id,
            "host": olt.host,
            "olt_type": olt.olt_type,
        });

        let asset_input = CreateNetworkAssetRequest {
            asset_type: "olt".into(),
            name: olt.name.clone(),
            code: None,
            vendor: Some(vendor.to_string()),
            model: None,
            serial_number: None,
            status: Some("available".into()),
            customer_id: None,
            location_id: None,
            work_order_id: None,
            parent_asset_id: None,
            olt_id: None,
            pon_port: None,
            latitude: olt.latitude,
            longitude: olt.longitude,
            notes: olt.description.clone(),
            metadata: Some(metadata),
        };
        match self
            .network_asset_service
            .create_asset(actor_id, tenant_id, asset_input)
            .await
        {
            Ok(asset) => tracing::info!(
                "Created NetworkAsset {} for OLT {}",
                asset.id,
                olt.id
            ),
            Err(e) => tracing::warn!(
                "Failed to create NetworkAsset for OLT {}: {}",
                olt.id,
                e
            ),
        }

        // Sprint C: propagate OLT → network_nodes so it appears on the map.
        self.sync_to_topology(actor_id, tenant_id, "olt_created").await;

        Ok(olt)
    }

    pub async fn update_olt(
        &self,
        actor_id: &str,
        id: &str,
        tenant_id: &str,
        req: UpdateOltRequest,
    ) -> AppResult<Olt> {
        let existing = self.get_olt(id, tenant_id).await?;

        let name = req.name.unwrap_or(existing.name.clone());
        let description = req.description.or_else(|| existing.description.clone());
        let host = req.host.unwrap_or(existing.host.clone());
        let port = req.port.unwrap_or(existing.port);
        let username = req.username.unwrap_or(existing.username.clone());
        let password_enc = if let Some(ref pw) = req.password {
            Some(encrypt_secret(pw)?)
        } else {
            existing.password_enc.clone()
        };
        // Sprint C: triple-state — req.latitude: Option<Option<f64>>
        //   None → leave unchanged, Some(None) → clear, Some(Some(v)) → set
        let latitude = match req.latitude {
            Some(v) => v,
            None => existing.latitude,
        };
        let longitude = match req.longitude {
            Some(v) => v,
            None => existing.longitude,
        };
        let address_line = match req.address_line {
            Some(v) => v,
            None => existing.address_line.clone(),
        };
        // Sprint D: triple-state for uplink fields
        let uplink_router_id = match req.uplink_router_id {
            Some(v) => v,
            None => existing.uplink_router_id,
        };
        let uplink_port = match req.uplink_port {
            Some(v) => v,
            None => existing.uplink_port.clone(),
        };

        sqlx::query(
            "UPDATE public.olts SET name = $1, description = $2, host = $3, port = $4, username = $5, password_enc = $6, latitude = $7, longitude = $8, address_line = $9, uplink_router_id = $10, uplink_port = $11, updated_at = now()
             WHERE id = $12 AND tenant_id = $13",
        )
        .bind(&name)
        .bind(&description)
        .bind(&host)
        .bind(port)
        .bind(&username)
        .bind(&password_enc)
        .bind(latitude)
        .bind(longitude)
        .bind(&address_line)
        .bind(uplink_router_id)
        .bind(&uplink_port)
        .bind(id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "olt_updated",
                "olt",
                Some(id),
                Some(&format!("Updated OLT: {}", name)),
                None,
            )
            .await;

        // ── Sync NetworkAsset (if exists) ─────────────────
        if let Some(asset_id) = self.find_asset_id_by_olt_id(tenant_id, id).await? {
            let update_dto = UpdateNetworkAssetRequest {
                asset_type: None,
                name: Some(name.clone()),
                code: None,
                vendor: None,
                model: None,
                serial_number: None,
                status: None,
                customer_id: None,
                location_id: None,
                work_order_id: None,
                parent_asset_id: None,
                olt_id: None,
                pon_port: None,
                // Sprint C: forward lat/lng to NetworkAsset so map stays in sync.
                // Pass raw values (network_asset_service.update_asset treats None as "keep existing"
                // and Some(v) as "set to v"). We always set here using our resolved values.
                latitude: if latitude.is_some() { latitude } else { None },
                longitude: if longitude.is_some() { longitude } else { None },
                notes: description.clone(),
                metadata: None,
            };
            if let Err(e) = self
                .network_asset_service
                .update_asset(actor_id, tenant_id, &asset_id, update_dto)
                .await
            {
                tracing::warn!(
                    "Failed to sync NetworkAsset {} for OLT {}: {}",
                    asset_id,
                    id,
                    e
                );
            }
        }

        // Sprint C: propagate updated lat/lng to network_nodes so the map marker moves.
        self.sync_to_topology(actor_id, tenant_id, "olt_updated").await;

        self.get_olt(id, tenant_id).await
    }

    pub async fn delete_olt(
        &self,
        actor_id: &str,
        id: &str,
        tenant_id: &str,
    ) -> AppResult<()> {
        let olt = self.get_olt(id, tenant_id).await?;

        sqlx::query("DELETE FROM public.olts WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "olt_deleted",
                "olt",
                Some(id),
                Some(&format!("Deleted OLT: {}", olt.name)),
                None,
            )
            .await;

        // ── Cascade delete NetworkAsset (if exists) ───────
        if let Some(asset_id) = self.find_asset_id_by_olt_id(tenant_id, id).await? {
            if let Err(e) = self
                .network_asset_service
                .delete_asset(actor_id, tenant_id, &asset_id)
                .await
            {
                tracing::warn!(
                    "Failed to delete NetworkAsset {} for OLT {}: {}",
                    asset_id,
                    id,
                    e
                );
            }
        }

        // Sprint C: propagate OLT removal to network_nodes.
        self.sync_to_topology(actor_id, tenant_id, "olt_deleted").await;

        Ok(())
    }

    /// Find the NetworkAsset ID linked to an OLT (via metadata->>'olt_id').
    /// Returns None if no asset exists.
    async fn find_asset_id_by_olt_id(
        &self,
        tenant_id: &str,
        olt_id: &str,
    ) -> AppResult<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM public.network_assets
             WHERE tenant_id = $1 AND asset_type = 'olt' AND metadata->>'olt_id' = $2
             LIMIT 1",
        )
        .bind(tenant_id)
        .bind(olt_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(id,)| id))
    }

    // ── Connection & Monitoring ───────────────────────────

    /// Test connection to OLT without saving anything.
    /// If `olt_id` is provided, the stored (decrypted) password is used —
    /// the `password` arg is ignored. This lets the UI test an existing OLT
    /// without making the user re-enter credentials.
    pub async fn test_connection(
        &self,
        tenant_id: &str,
        olt_id: Option<&str>,
        host: &str,
        port: i32,
        username: &str,
        password: &str,
        olt_type: &str,
    ) -> AppResult<TestConnectionResponse> {
        // If an existing OLT is referenced, use its stored password
        let password = if let Some(id) = olt_id {
            let olt = self.get_olt(id, tenant_id).await?;
            match olt.password_enc.as_deref() {
                Some(enc) => decrypt_secret_opt(enc)?.unwrap_or_default(),
                None => String::new(),
            }
        } else {
            password.to_string()
        };

        let mut driver = create_driver(olt_type)?;
        match driver
            .connect(host, port as u16, username, &password)
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

    /// Get all ONUs on a single OLT, enriched with linked network_asset
    /// + customer_id via OnuLinker. Used by `GET /api/admin/olts/{id}/onu-customer`.
    pub async fn get_olt_onus_with_customer(
        &self,
        olt_id: &str,
        tenant_id: &str,
    ) -> AppResult<Vec<crate::models::OnuWithCustomer>> {
        use crate::models::OnuWithCustomer;

        let olt = self.get_olt(olt_id, tenant_id).await?;
        let password = decrypt_secret_opt(&olt.password_enc.clone().unwrap_or_default())
            .unwrap_or_default()
            .unwrap_or_default();

        let mut driver = create_driver(&olt.olt_type)?;
        driver
            .connect(&olt.host, olt.port as u16, &olt.username, &password)
            .await
            .map_err(|e| AppError::Internal(format!("OLT connection failed: {}", e)))?;

        let stats = driver.get_global_stats().await?;
        let mut all_onus: Vec<OltOnuDetail> = Vec::new();
        for p in &stats.pon_ports {
            if let Ok(onus) = driver.get_pon_onu_details(&p.name).await {
                all_onus.extend(onus);
            }
        }
        driver.disconnect().await.ok();

        let mut result = Vec::with_capacity(all_onus.len());
        for onu in all_onus {
            let link = self
                .onu_linker
                .lookup_by_mac(tenant_id, &onu.mac)
                .await
                .ok()
                .flatten();
            result.push(OnuWithCustomer {
                onu_id: onu.onu_id,
                name: onu.name,
                mac: onu.mac,
                status: onu.status,
                rx: onu.rx,
                tx: onu.tx,
                distance: onu.distance,
                temperature: onu.temperature,
                pon: onu.pon,
                olt_id: olt.id.clone(),
                asset_id: link.as_ref().map(|l| l.asset_id.clone()),
                customer_id: link.as_ref().and_then(|l| l.customer_id.clone()),
                linked_at: None,
            });
        }

        Ok(result)
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
        // ── Auto-link discovered ONUs to network_assets by MAC ──
        let mac_pon_pairs: Vec<(String, String)> = onus
            .iter()
            .filter(|o| !o.mac.is_empty())
            .map(|o| (o.mac.clone(), o.pon.clone()))
            .collect();
        if !mac_pon_pairs.is_empty() {
            match self
                .onu_linker
                .link_batch(tenant_id, olt_id, &mac_pon_pairs)
                .await
            {
                Ok(stats) => {
                    if stats.linked > 0 || stats.errors > 0 {
                        tracing::info!(
                            "OnuLinker batch for OLT {}: linked={}, skipped={}, errors={}",
                            olt_id,
                            stats.linked,
                            stats.skipped,
                            stats.errors
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "OnuLinker batch failed for OLT {}: {}",
                        olt_id,
                        e
                    );
                }
            }
        }

        for onu in onus {
            // Parse signal values as f32 (DB column type is `real` = float4)
            let rx: Option<f32> = onu.rx.replace("dBm", "").trim().parse().ok();
            let tx: Option<f32> = onu
                .tx
                .as_ref()
                .and_then(|s| s.replace("dBm", "").trim().parse().ok());
            let dist: Option<f32> = onu
                .distance
                .as_ref()
                .and_then(|s| s.replace("km", "").trim().parse().ok());
            let temp: Option<f32> = onu
                .temperature
                .as_ref()
                .and_then(|s| s.trim().parse().ok());

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
            .bind(dist)
            .bind(temp)
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

// ── Token + Public Stats (Phase 7) ──────────────────────────

use crate::models::CreatePublicTokenRequest;

impl OltService {
    /// List all public tokens for an OLT
    pub async fn list_public_tokens(
        &self,
        olt_id: &str,
        tenant_id: &str,
    ) -> AppResult<Vec<OltPublicToken>> {
        // Verify OLT exists and belongs to tenant
        self.get_olt(olt_id, tenant_id).await?;

        let tokens = sqlx::query_as::<_, OltPublicToken>(
            "SELECT * FROM public.olt_public_tokens WHERE olt_id = $1 AND tenant_id = $2 ORDER BY created_at DESC",
        )
        .bind(olt_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(tokens)
    }

    /// Create a new public token for an OLT
    pub async fn create_public_token(
        &self,
        olt_id: &str,
        tenant_id: &str,
        req: CreatePublicTokenRequest,
    ) -> AppResult<OltPublicToken> {
        self.get_olt(olt_id, tenant_id).await?;

        let id = Uuid::new_v4().to_string();
        let token = format!("{:016x}", rand::random::<u128>());
        let enabled = req.enabled;
        let expires_at: Option<chrono::NaiveDateTime> = req
            .expires_at
            .as_deref()
            .and_then(|s| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok());
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO public.olt_public_tokens (id, olt_id, tenant_id, token, description, enabled, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&id)
        .bind(olt_id)
        .bind(tenant_id)
        .bind(&token)
        .bind(&req.description)
        .bind(enabled)
        .bind(now)
        .bind(expires_at.map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)))
        .execute(&self.pool)
        .await?;

        // Return the created token
        let token_row = sqlx::query_as::<_, OltPublicToken>(
            "SELECT * FROM public.olt_public_tokens WHERE id = $1",
        )
        .bind(&id)
        .fetch_one(&self.pool)
        .await?;

        Ok(token_row)
    }

    /// Delete a public token
    pub async fn delete_public_token(
        &self,
        token_id: &str,
        tenant_id: &str,
    ) -> AppResult<()> {
        let result = sqlx::query(
            "DELETE FROM public.olt_public_tokens WHERE id = $1 AND tenant_id = $2",
        )
        .bind(token_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Public token not found".into()));
        }
        Ok(())
    }

    /// Get OLT stats by public token (no auth required)
    pub async fn get_stats_by_token(
        &self,
        token: &str,
    ) -> AppResult<serde_json::Value> {
        // Lookup token
        let token_row = sqlx::query_as::<_, OltPublicToken>(
            "SELECT * FROM public.olt_public_tokens WHERE token = $1 AND enabled = true",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Invalid or disabled token".into()))?;

        // Check expiry
        if let Some(expires) = token_row.expires_at {
            if Utc::now() > expires {
                return Err(AppError::Forbidden("Token has expired".into()));
            }
        }

        // Get OLT info (without tenant check — public access)
        let olt = sqlx::query_as::<_, Olt>(
            "SELECT * FROM public.olts WHERE id = $1",
        )
        .bind(&token_row.olt_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("OLT not found".into()))?;

        // Get stats via driver (with fallback to cached)
        let password = decrypt_secret_opt(&olt.password_enc.unwrap_or_default())
            .unwrap_or_default()
            .unwrap_or_default();
        let mut driver = create_driver(&olt.olt_type)?;
        let (is_online, stats_json) = match driver
            .connect(&olt.host, olt.port as u16, &olt.username, &password)
            .await
        {
            Ok(()) => {
                let stats = driver.get_global_stats().await?;
                driver.disconnect().await.ok();
                let json = serde_json::to_value(&stats)
                    .unwrap_or(serde_json::json!({}));
                (true, json)
            }
            Err(_) => {
                // Fallback to cached stats
                let cached: Option<serde_json::Value> = sqlx::query_scalar(
                    "SELECT last_stats FROM public.olts WHERE id = $1",
                )
                .bind(&olt.id)
                .fetch_optional(&self.pool)
                .await
                .ok()
                .flatten();
                (olt.is_online, cached.unwrap_or(serde_json::json!({})))
            }
        };

        Ok(serde_json::json!({
            "status": "success",
            "olt_name": olt.name,
            "olt_host": olt.host,
            "is_online": is_online,
            "data": stats_json,
        }))
    }

    /// Get OLT signal graph data by public token (no auth required)
    pub async fn get_signal_by_token(
        &self,
        token: &str,
    ) -> AppResult<serde_json::Value> {
        // Lookup token
        let token_row = sqlx::query_as::<_, OltPublicToken>(
            "SELECT * FROM public.olt_public_tokens WHERE token = $1 AND enabled = true",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Invalid or disabled token".into()))?;

        // Check expiry
        if let Some(expires) = token_row.expires_at {
            if Utc::now() > expires {
                return Err(AppError::Forbidden("Token has expired".into()));
            }
        }

        // Get OLT
        let olt = sqlx::query_as::<_, Olt>(
            "SELECT * FROM public.olts WHERE id = $1",
        )
        .bind(&token_row.olt_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("OLT not found".into()))?;

        // Get ONU details via driver
        let password = decrypt_secret_opt(&olt.password_enc.unwrap_or_default())
            .unwrap_or_default()
            .unwrap_or_default();
        let mut driver = create_driver(&olt.olt_type)?;
        let details = match driver
            .connect(&olt.host, olt.port as u16, &olt.username, &password)
            .await
        {
            Ok(()) => {
                let stats = driver.get_global_stats().await;
                // Collect ONUs from all PON ports
                let mut all_onus = Vec::new();
                if let Ok(stats) = &stats {
                    for pon in &stats.pon_ports {
                        if let Ok(onus) = driver.get_pon_onu_details(&pon.name).await {
                            all_onus.extend(onus);
                        }
                    }
                }
                driver.disconnect().await.ok();
                all_onus
            }
            Err(_) => Vec::new(),
        };

        // Calculate signal distribution for charting
        let mut excellent = 0i32; // > -20 dBm
        let mut good = 0i32;     // -20 to -24 dBm
        let mut fair = 0i32;     // -24 to -27 dBm
        let mut poor = 0i32;     // < -27 dBm

        for onu in &details {
            if onu.status != "Online" { continue; }
            let rx: f64 = onu.rx.replace("dBm", "").trim().parse().unwrap_or(-999.0);
            if rx > -20.0 { excellent += 1; }
            else if rx > -24.0 { good += 1; }
            else if rx > -27.0 { fair += 1; }
            else { poor += 1; }
        }

        Ok(serde_json::json!({
            "status": "success",
            "olt_name": olt.name,
            "signal_distribution": {
                "excellent": excellent,
                "good": good,
                "fair": fair,
                "poor": poor,
            }
        }))
    }
}
