//! Mock OLT Driver — for testing without physical OLT hardware
//!
//! Returns synthetic data with realistic-looking values.

use super::OltDriver;
use crate::error::AppResult;
use crate::models::{OltGlobalStats, OltOnuDetail, OltSystemInfo, PonPortStats};
use async_trait::async_trait;

pub struct MockOltDriver {
    connected: bool,
}

impl MockOltDriver {
    pub fn new() -> Self {
        Self { connected: false }
    }
}

impl Default for MockOltDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OltDriver for MockOltDriver {
    async fn connect(
        &mut self,
        _host: &str,
        _port: u16,
        _username: &str,
        _password: &str,
    ) -> AppResult<()> {
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> AppResult<()> {
        self.connected = false;
        Ok(())
    }

    async fn get_system_info(&self) -> AppResult<OltSystemInfo> {
        Ok(OltSystemInfo {
            name: "Mock OLT".into(),
            model: "MOCK-V1".into(),
            version: "1.0.0".into(),
            address: "127.0.0.1".into(),
        })
    }

    async fn get_global_stats(&self) -> AppResult<OltGlobalStats> {
        Ok(OltGlobalStats {
            name: Some("Mock OLT".into()),
            ip: Some("127.0.0.1".into()),
            pon_ports: vec![
                PonPortStats {
                    name: "0/1/1".into(),
                    total: 32,
                    online: 30,
                    offline: 2,
                },
                PonPortStats {
                    name: "0/1/2".into(),
                    total: 32,
                    online: 28,
                    offline: 4,
                },
            ],
            total_onus: 64,
            online_onus: 58,
            offline_onus: 6,
            low_onus: 3,
            risk_onus: 0,
        })
    }

    async fn get_pon_onu_details(&self, pon: &str) -> AppResult<Vec<OltOnuDetail>> {
        Ok(vec![
            OltOnuDetail {
                onu_id: "1".into(),
                name: "Pelanggan A".into(),
                mac: "AA:BB:CC:DD:EE:01".into(),
                status: "Online".into(),
                rx: "-19.20".into(),
                tx: Some("2.10".into()),
                distance: Some("1.5".into()),
                temperature: Some("45".into()),
                pon: pon.into(),
                olt_id: None,
                olt_name: None,
            },
            OltOnuDetail {
                onu_id: "2".into(),
                name: "Pelanggan B".into(),
                mac: "AA:BB:CC:DD:EE:02".into(),
                status: "Offline".into(),
                rx: "--".into(),
                tx: Some("--".into()),
                distance: Some("3.2".into()),
                temperature: Some("0".into()),
                pon: pon.into(),
                olt_id: None,
                olt_name: None,
            },
        ])
    }

    async fn get_onu_signal(&self, _mac: &str) -> AppResult<f64> {
        Ok(-19.5)
    }

    async fn get_onu_status(&self, _mac: &str) -> AppResult<String> {
        Ok("online".into())
    }

    async fn reboot_onu(&self, _onu_id: &str, _onu_name: &str) -> AppResult<bool> {
        Ok(true)
    }

    async fn update_onu_name(&self, _onu_id: &str, _pon: &str, _new_name: &str) -> AppResult<()> {
        Ok(())
    }
}
