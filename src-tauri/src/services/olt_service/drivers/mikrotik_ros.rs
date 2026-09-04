//! MikroTik RouterOS OLT Driver — communicates via RouterOS API (port 8728).
//!
//! Unlike the HIOSO web-scraping driver, this driver talks to a MikroTik router
//! that the OLT's SFP/uplink is connected to, providing:
//!   - Real-time bandwidth (via `/interface/monitor-traffic` with `once`)
//!   - SFP optical power (via `/interface/ethernet/monitor` with `once`)
//!   - System resource metrics (CPU, memory, uptime)
//!
//! ONU-level data (per-subscriber signal, status, distance) is NOT available
//! through RouterOS API and will return empty / default values.

use crate::error::{AppError, AppResult};
use crate::models::{OltGlobalStats, OltOnuDetail, OltSystemInfo, PonPortStats};
use async_trait::async_trait;
use mikrotik_rs::{protocol::command::CommandBuilder, protocol::CommandResponse, MikrotikDevice};

use super::OltDriver;

const DEFAULT_PORT: u16 = 8728;

pub struct MikrotikRosDriver {
    device: Option<MikrotikDevice>,
}

impl Default for MikrotikRosDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl MikrotikRosDriver {
    pub fn new() -> Self {
        Self { device: None }
    }

    /// Send a RouterOS command and collect all replies into a Vec of attribute maps.
    async fn send_command(
        &self,
        path: &str,
    ) -> AppResult<Vec<std::collections::HashMap<String, Option<String>>>> {
        let dev = self
            .device
            .as_ref()
            .ok_or_else(|| AppError::Validation("Not connected to MikroTik".into()))?;

        let cmd = CommandBuilder::new().command(path).build();
        let mut rx = dev
            .send_command(cmd)
            .await
            .map_err(|e| AppError::Internal(format!("RouterOS command failed: {}", e)))?;

        let mut rows: Vec<std::collections::HashMap<String, Option<String>>> = Vec::new();
        while let Some(res) = rx.recv().await {
            match res.map_err(|e| AppError::Internal(format!("RouterOS rx: {}", e)))? {
                CommandResponse::Reply(reply) => {
                    rows.push(reply.attributes);
                }
                _ => {}
            }
        }
        Ok(rows)
    }
}

#[async_trait]
impl OltDriver for MikrotikRosDriver {
    async fn connect(
        &mut self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> AppResult<()> {
        let port = if port == 0 { DEFAULT_PORT } else { port };
        let addr = format!("{}:{}", host, port);

        let dev = MikrotikDevice::connect(addr, username, Some(password))
            .await
            .map_err(|e| AppError::Internal(format!("MikroTik connect failed: {}", e)))?;

        self.device = Some(dev);
        Ok(())
    }

    async fn disconnect(&mut self) -> AppResult<()> {
        self.device = None;
        Ok(())
    }

    async fn get_system_info(&self) -> AppResult<OltSystemInfo> {
        // Identity
        let identity = self
            .send_command("/system/identity/print")
            .await
            .ok()
            .and_then(|rows| rows.first()?.get("name")?.clone())
            .unwrap_or_else(|| "MikroTik".to_string());

        // Resource (model, version)
        let resource = self.send_command("/system/resource/print").await.ok();
        let model = resource
            .as_ref()
            .and_then(|rows| rows.first()?.get("board-name")?.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        let version = resource
            .as_ref()
            .and_then(|rows| rows.first()?.get("version")?.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        Ok(OltSystemInfo {
            name: identity,
            model,
            version,
            address: String::new(), // RouterOS address already known by caller
        })
    }

    async fn get_global_stats(&self) -> AppResult<OltGlobalStats> {
        // Get all interfaces, filter for SFP-based ones (these represent PON uplinks / OLT connections)
        let interfaces = self.send_command("/interface/print").await?;

        let mut pon_ports: Vec<PonPortStats> = Vec::new();
        let mut total_onus: i32 = 0;
        let mut online_onus: i32 = 0;
        let mut offline_onus: i32 = 0;

        for iface in &interfaces {
            let name = iface
                .get("name")
                .and_then(|v| v.clone())
                .unwrap_or_default();

            // Only include physical interfaces (sfp, ether, combo)
            let running = iface
                .get("running")
                .and_then(|v| v.clone())
                .map(|v| v == "true")
                .unwrap_or(false);

            let disabled = iface
                .get("disabled")
                .and_then(|v| v.clone())
                .map(|v| v == "true")
                .unwrap_or(false);

            if disabled {
                continue;
            }

            // For SFP interfaces, poll real-time traffic
            let (_rx_bps, _tx_bps, _sfp_rx_power) = if running {
                self.poll_interface_metrics(&name).await.unwrap_or_default()
            } else {
                (0, 0, None)
            };

            let online = if running { 1 } else { 0 };
            let offline = if !running && !disabled { 1 } else { 0 };
            let total = online + offline;

            pon_ports.push(PonPortStats {
                name,
                total,
                online,
                offline,
            });

            total_onus += total;
            online_onus += online;
            offline_onus += offline;
        }

        if pon_ports.is_empty() {
            // Fallback: return at least one entry so the dashboard shows something
            pon_ports.push(PonPortStats {
                name: "all".to_string(),
                total: 0,
                online: 0,
                offline: 0,
            });
        }

        Ok(OltGlobalStats {
            name: Some("MikroTik RouterOS".into()),
            ip: None,
            pon_ports,
            total_onus,
            online_onus,
            offline_onus,
            low_onus: 0,
            risk_onus: 0,
        })
    }

    async fn get_pon_onu_details(&self, _pon: &str) -> AppResult<Vec<OltOnuDetail>> {
        // RouterOS API doesn't expose per-ONU subscriber details.
        // Use the HIOSO web driver for that data.
        Ok(Vec::new())
    }

    async fn get_onu_signal(&self, _mac: &str) -> AppResult<f64> {
        // RouterOS doesn't expose per-ONU signal by MAC.
        Ok(0.0)
    }

    async fn get_onu_status(&self, _mac: &str) -> AppResult<String> {
        Ok("unknown".into())
    }

    async fn reboot_onu(&self, _onu_id: &str, _onu_name: &str) -> AppResult<bool> {
        // RouterOS can't reboot individual ONUs on third-party OLTs
        Ok(false)
    }

    async fn update_onu_name(&self, _onu_id: &str, _pon: &str, _new_name: &str) -> AppResult<()> {
        Err(AppError::Validation(
            "ONU rename not supported via MikroTik RouterOS".into(),
        ))
    }
}

// ── Private helpers ──

impl MikrotikRosDriver {
    /// Poll real-time interface metrics (bandwidth + SFP power).
    /// Mirrors MiksTraffic's approach:
    ///   `/interface/monitor-traffic` → rx/tx-bits-per-second
    ///   `/interface/ethernet/monitor` → sfp-rx-power
    async fn poll_interface_metrics(&self, iface_name: &str) -> AppResult<(i64, i64, Option<f64>)> {
        let dev = self
            .device
            .as_ref()
            .ok_or_else(|| AppError::Validation("Not connected".into()))?;

        // ── Traffic ──
        let traffic = {
            let mut args = std::collections::HashMap::new();
            args.insert("interface".to_string(), iface_name.to_string());
            args.insert("once".to_string(), String::new());

            let cmd = CommandBuilder::new()
                .command("/interface/monitor-traffic")
                .build();
            let mut rx = dev
                .send_command(cmd)
                .await
                .map_err(|e| AppError::Internal(format!("monitor-traffic: {}", e)))?;

            let mut rx_bps: i64 = 0;
            let mut tx_bps: i64 = 0;
            while let Some(res) = rx.recv().await {
                if let Ok(CommandResponse::Reply(reply)) =
                    res.map_err(|e| AppError::Internal(e.to_string()))
                {
                    rx_bps = reply
                        .attributes
                        .get("rx-bits-per-second")
                        .and_then(|v| v.as_ref()?.parse::<i64>().ok())
                        .unwrap_or(0);
                    tx_bps = reply
                        .attributes
                        .get("tx-bits-per-second")
                        .and_then(|v| v.as_ref()?.parse::<i64>().ok())
                        .unwrap_or(0);
                }
            }
            (rx_bps, tx_bps)
        };

        // ── Ethernet/SFP monitor ──
        let sfp_rx: Option<f64> = {
            let mut args = std::collections::HashMap::new();
            args.insert("numbers".to_string(), iface_name.to_string());
            args.insert("once".to_string(), String::new());

            let cmd = CommandBuilder::new()
                .command("/interface/ethernet/monitor")
                .build();
            let mut rx = match dev.send_command(cmd).await {
                Ok(rx) => rx,
                Err(_) => return Ok((traffic.0, traffic.1, None)),
            };

            let mut power: Option<f64> = None;
            while let Some(res) = rx.recv().await {
                if let Ok(CommandResponse::Reply(reply)) =
                    res.map_err(|e| AppError::Internal(e.to_string()))
                {
                    power = reply
                        .attributes
                        .get("sfp-rx-power")
                        .and_then(|v| v.as_ref()?.parse::<f64>().ok());
                }
            }
            power
        };

        Ok((traffic.0, traffic.1, sfp_rx))
    }
}
