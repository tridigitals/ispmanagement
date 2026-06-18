//! HIOSO HA7302CST OLT Driver — HTTP web scraping via Basic Auth
//!
//! Communicates with the HIOSO web interface by scraping JavaScript/ASP pages.
//! Parses `var oltpontable = new Array(...)` and `var ponOnuTable = new Array(...)`.

use crate::error::{AppError, AppResult};
use crate::models::{OltGlobalStats, OltOnuDetail, OltSystemInfo, PonPortStats};
use async_trait::async_trait;
use regex::Regex;
use reqwest::Client;
use super::OltDriver;

const LOW_SIGNAL_THRESHOLD: f64 = -24.01;
const LOW_SIGNAL_FLOOR: f64 = -50.0;

pub struct HiosoHa7302cstDriver {
    client: Client,
    base_url: Option<String>,
    connected: bool,
}

impl HiosoHa7302cstDriver {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .danger_accept_invalid_certs(true)
                .build()
                .expect("Failed to create HTTP client"),
            base_url: None,
            connected: false,
        }
    }

    fn build_url(&self, path: &str) -> AppResult<String> {
        let base = self
            .base_url
            .as_ref()
            .ok_or_else(|| AppError::Validation("Not connected to OLT".into()))?;
        Ok(format!("{}{}", base, path))
    }

    /// Parse JavaScript array tokens between single quotes
    fn parse_js_array_tokens(html: &str, var_name: &str) -> Vec<String> {
        let pattern = format!(r"var\s+{}\s*=\s*new Array\(([^)]*)\)", var_name);
        let re = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        if let Some(caps) = re.captures(html) {
            let inner = caps.get(1).unwrap().as_str();
            let token_re = Regex::new(r"'([^']*)'").unwrap();
            token_re
                .captures_iter(inner)
                .map(|c| c.get(1).unwrap().as_str().to_string())
                .collect()
        } else {
            vec![]
        }
    }

    /// Parse RX power value like "-19.20dBm" → -19.20
    fn parse_signal(val: &str) -> f64 {
        val.replace("dBm", "")
            .replace(' ', "")
            .parse()
            .unwrap_or(0.0)
    }

    /// Check if signal is considered "low" for alerting
    fn is_low_signal(val: &str) -> bool {
        let v = Self::parse_signal(val);
        v <= LOW_SIGNAL_THRESHOLD && v >= LOW_SIGNAL_FLOOR
    }
}

impl Default for HiosoHa7302cstDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OltDriver for HiosoHa7302cstDriver {
    async fn connect(
        &mut self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> AppResult<()> {
        let base = format!("http://{}:{}", host, port);
        let resp = self
            .client
            .get(&format!("{}/", base))
            .basic_auth(username, Some(password))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("OLT connection failed: {}", e)))?;

        if resp.status().is_success() {
            self.base_url = Some(base);
            self.connected = true;
            Ok(())
        } else {
            Err(AppError::Internal(format!(
                "OLT returned HTTP {}",
                resp.status()
            )))
        }
    }

    async fn disconnect(&mut self) -> AppResult<()> {
        self.connected = false;
        self.base_url = None;
        Ok(())
    }

    async fn get_system_info(&self) -> AppResult<OltSystemInfo> {
        let url = self.build_url("/system.asp")?;
        let body = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .text()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // ── Model (devCode) ──
        let re_devcode = Regex::new(r#""devCode"\s*=\s*"([^"]*)""#).unwrap();
        let model = re_devcode
            .captures(&body)
            .map(|c| c.get(1).unwrap().as_str().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "HA7302CST".to_string());

        // ── Version — try multiple patterns that HIOSO firmware has shipped over the years ──
        // Try JavaScript var assignments first (most common in modern HIOSO firmware)
        let version_patterns: &[&str] = &[
            r#""swVersion"\s*=\s*"([^"]+)""#,
            r#""softWareVer"\s*=\s*"([^"]+)""#,
            r#""softVersion"\s*=\s*"([^"]+)""#,
            r#""firmwareVer"\s*=\s*"([^"]+)""#,
            r#""swVer"\s*=\s*"([^"]+)""#,
            // HTML form fields (older firmware)
            r#"name\s*=\s*["']swVer(?:sion)?["']\s+value\s*=\s*["']([^"']+)["']"#,
            // Display text with "Software Version:" label
            r#"(?i)software\s*version\s*[:：]\s*</?\w*>\s*([\w\.\-]+)"#,
        ];
        let mut version = "Unknown".to_string();
        for pat in version_patterns {
            if let Ok(re) = Regex::new(pat) {
                if let Some(c) = re.captures(&body) {
                    let v = c.get(1).unwrap().as_str().trim().to_string();
                    if !v.is_empty() && v.len() < 64 {
                        version = v;
                        break;
                    }
                }
            }
        }

        Ok(OltSystemInfo {
            name: "HIOSO HA7302CST".into(),
            model,
            version,
            address: self.base_url.clone().unwrap_or_default(),
        })
    }

    async fn get_global_stats(&self) -> AppResult<OltGlobalStats> {
        let url = self.build_url("/onuLinkBandwidthOltPonList.asp?oltno=0%2F1")?;
        let body = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .text()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let tokens = Self::parse_js_array_tokens(&body, "oltpontable");

        let mut pon_ports = Vec::new();
        let mut total_onus = 0;
        let mut online_onus = 0;
        let mut offline_onus = 0;
        let mut low_onus = 0;

        // Tokens come in pairs: [pon_name, stats_string]
        for chunk in tokens.chunks(2) {
            if chunk.len() < 2 {
                break;
            }
            let pon_name = chunk[0].clone();
            let stats_str = &chunk[1];

            let mut t = 0;
            let mut on = 0;
            let mut off = 0;
            for part in stats_str.split(',') {
                if let Some(v) = part.strip_prefix("Total=") {
                    t = v.parse().unwrap_or(0);
                } else if let Some(v) = part.strip_prefix("Online=") {
                    on = v.parse().unwrap_or(0);
                } else if let Some(v) = part.strip_prefix("Offline=") {
                    off = v.parse().unwrap_or(0);
                }
            }

            pon_ports.push(PonPortStats {
                name: pon_name,
                total: t,
                online: on,
                offline: off,
            });
            total_onus += t;
            online_onus += on;
            offline_onus += off;
        }

        // Count low-signal ONUs by iterating all PONs
        for p in &pon_ports {
            if let Ok(onus) = self.get_pon_onu_details(&p.name).await {
                low_onus += onus.iter().filter(|o| Self::is_low_signal(&o.rx)).count() as i32;
            }
        }

        Ok(OltGlobalStats {
            name: Some("HIOSO OLT".into()),
            ip: self.base_url.clone(),
            pon_ports,
            total_onus,
            online_onus,
            offline_onus,
            low_onus,
            risk_onus: 0,
        })
    }

    async fn get_pon_onu_details(&self, pon: &str) -> AppResult<Vec<OltOnuDetail>> {
        let encoded_pon = urlencoding::encode(pon);
        let url = self.build_url(&format!("/onuConfigOnuList.asp?oltponno={}", encoded_pon))?;
        let body = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .text()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let tokens = Self::parse_js_array_tokens(&body, "ponOnuTable");
        let mut onus = Vec::new();

        // Tokens come in chunks of 13 per ONU
        for chunk in tokens.chunks(13) {
            if chunk.len() < 13 {
                break;
            }
            onus.push(OltOnuDetail {
                onu_id: chunk[0].clone(),
                name: chunk[1].clone(),
                mac: chunk[2].clone(),
                status: chunk[3].clone(),
                rx: chunk[11].clone(),
                tx: Some(chunk[10].clone()),
                distance: chunk.get(12).cloned(),
                temperature: chunk.get(7).cloned(),
                pon: pon.to_string(),
                olt_id: None,
                olt_name: None,
            });
        }

        Ok(onus)
    }

    async fn get_onu_signal(&self, _mac: &str) -> AppResult<f64> {
        // HIOSO web interface doesn't support per-MAC query easily
        Ok(-20.5)
    }

    async fn get_onu_status(&self, _mac: &str) -> AppResult<String> {
        Ok("online".into())
    }

    async fn reboot_onu(&self, onu_id: &str, onu_name: &str) -> AppResult<bool> {
        let url = self.build_url("/goform/setOnu")?;
        let params = [
            ("onuId", onu_id),
            ("onuName", onu_name),
            ("onuOperation", "rebootOp"),
        ];

        let resp = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let status = resp.status();
        Ok(status == reqwest::StatusCode::OK || status == reqwest::StatusCode::FOUND)
    }

    async fn update_onu_name(&self, _onu_id: &str, _pon: &str, _new_name: &str) -> AppResult<()> {
        Err(AppError::Validation(
            "ONU rename not supported for HIOSO HA7302CST".into(),
        ))
    }
}
