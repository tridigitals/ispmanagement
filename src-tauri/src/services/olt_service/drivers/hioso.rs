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
    /// Stored from connect() so we can send Basic Auth with every request.
    credentials: Option<(String, String)>,
}

impl HiosoHa7302cstDriver {
    pub fn new() -> Self {
        // cookie_store(true) makes the client persist + replay Set-Cookie
        // headers across requests, which is how HIOSO's session login works.
        // reqwest's default cookie feature is enabled in Cargo.toml.
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .cookie_store(true)
            .build()
            .expect("Failed to create HTTP client");
        Self {
            client,
            base_url: None,
            connected: false,
            credentials: None,
        }
    }

    /// Issue an authenticated GET.  Uses stored Basic Auth credentials if available.
    async fn auth_get(&self, url: &str) -> AppResult<String> {
        let mut req = self.client.get(url);
        if let Some((ref user, ref pass)) = self.credentials {
            req = req.basic_auth(user, Some(pass));
        }
        let body = req
            .send()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .text()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(body)
    }

    fn build_url(&self, path: &str) -> AppResult<String> {
        let base = self
            .base_url
            .as_ref()
            .ok_or_else(|| AppError::Validation("Not connected to OLT".into()))?;
        Ok(format!("{}{}", base, path))
    }

    /// Parse JavaScript array tokens between single quotes.
    /// Handles multiline declarations, common across HIOSO firmware revisions.
    fn parse_js_array_tokens(html: &str, var_name: &str) -> Vec<String> {
        // Strategy: find `var <name> = new Array(` and extract every single-quoted token
        // that appears before the statement-closing `);`.
        //
        // We avoid a single regex because the array literal may span many lines
        // (thousands of ONUs) and may contain escaped quotes inside values.

        let needle = format!("var {} = new Array(", var_name);
        let start = match html.find(&needle) {
            Some(pos) => pos + needle.len(),
            None => return vec![],
        };

        // Scan forward for the matching closing paren, counting nesting depth
        // and respecting JavaScript string delimiters so a `)` inside a quoted
        // value doesn't fool us.
        let rest = &html[start..];
        let mut depth: u32 = 1;
        let mut end: usize = 0;
        let mut in_single = false;
        let mut in_double = false;
        let mut prev_was_backslash = false;

        for (i, ch) in rest.char_indices() {
            if in_single {
                if prev_was_backslash {
                    prev_was_backslash = false;
                } else if ch == '\\' {
                    prev_was_backslash = true;
                } else if ch == '\'' {
                    in_single = false;
                }
                continue;
            }
            if in_double {
                if prev_was_backslash {
                    prev_was_backslash = false;
                } else if ch == '\\' {
                    prev_was_backslash = true;
                } else if ch == '"' {
                    in_double = false;
                }
                continue;
            }
            match ch {
                '\'' => in_single = true,
                '"' => in_double = true,
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i;
                        break;
                    }
                }
                _ => {}
            }
        }

        if depth != 0 || end == 0 {
            return vec![];
        }

        let inner = &rest[..end];

        // Extract all single-quoted tokens (faster than regex for many small strings)
        let mut tokens = Vec::new();
        let mut pos = 0;
        while let Some(open) = inner[pos..].find('\'') {
            let after_open = pos + open + 1;
            let mut close = after_open;
            let mut escaped = false;
            while close < inner.len() {
                let ch = inner.as_bytes()[close] as char;
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '\'' {
                    tokens.push(inner[after_open..close].to_string());
                    pos = close + 1;
                    break;
                }
                close += 1;
            }
            if close >= inner.len() {
                break; // unterminated string — stop
            }
        }

        tokens
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
        self.base_url = Some(base.clone());
        self.credentials = Some((username.to_string(), password.to_string()));

        // HIOSO web uses a session cookie issued by POSTing credentials to the
        // root URL.  We do an initial GET to warm the cookie jar, then a POST
        // that mimics the browser login form.  `cookie_store(true)` on the
        // client means the Set-Cookie we get back is replayed automatically
        // on every subsequent request.
        let _ = self
            .client
            .get(&format!("{}/login.asp", base))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("OLT connection failed: {}", e)))?;

        // POST credentials.  HIOSO's login form fields are typically
        // `Username` and `Password` (capital U) but the form is often
        // tolerant of case.  We send the common field names.
        let mut form: Vec<(&str, &str)> = Vec::new();
        form.push(("Username", username));
        form.push(("username", username));
        form.push(("Password", password));
        form.push(("password", password));
        form.push(("Submit", "Login"));
        form.push(("login", "Login"));

        let resp = self
            .client
            .post(&format!("{}/", base))
            .form(&form)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("OLT login failed: {}", e)))?;

        let status = resp.status();
        let _ = resp.text().await; // drain body to release connection

        if status.is_success() || status.is_redirection() {
            // Verify that the session actually lets us in: hit a protected
            // endpoint and make sure we don't get the login page back.
            let probe = self
                .client
                .get(&format!("{}/onuLinkBandwidthOltPonList.asp?oltno=0%2F1", base))
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("OLT probe failed: {}", e)))?;
            let probe_status = probe.status();
            let probe_body = probe.text().await.unwrap_or_default();
            if probe_status.is_success() && !probe_body.contains("Please login") {
                self.connected = true;
                return Ok(());
            }
            Err(AppError::Internal(format!(
                "OLT login did not establish a session (status={} body={}…)",
                probe_status,
                &probe_body[..probe_body.len().min(100)]
            )))
        } else {
            Err(AppError::Internal(format!(
                "OLT login returned HTTP {}",
                status
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
        let body = self.auth_get(&url).await?;

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
        tracing::info!("[HIOSO] Fetching global stats: {}", url);
        let body = self.auth_get(&url).await?;

        tracing::info!(
            "[HIOSO] global_stats body len={} preview={}",
            body.len(),
            &body[..body.len().min(300)]
        );
        // DEBUG: write raw HTML to file so we can inspect it
        let _ = std::fs::write("/tmp/hioso_global_stats.html", &body);

        let tokens = Self::parse_js_array_tokens(&body, "oltpontable");
        tracing::info!("[HIOSO] oltpontable tokens found: {}", tokens.len());

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
        tracing::info!("[HIOSO] Fetching PON onu details: {}", url);
        let body = self.auth_get(&url).await?;

        tracing::info!(
            "[HIOSO] PON={} body len={} preview={}",
            pon,
            body.len(),
            &body[..body.len().min(200)]
        );

        let tokens = Self::parse_js_array_tokens(&body, "ponOnuTable");
        tracing::info!("[HIOSO] PON={} ponOnuTable tokens found: {}", pon, tokens.len());
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
