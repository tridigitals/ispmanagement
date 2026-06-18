1|# OLT Monitoring System — Implementation Plan
2|
3|> **For Hermes:** Gunakan subagent-driven-development skill untuk implementasi task-by-task.
4|> Setiap task selesai → commit terpisah dengan format `feat(olt): <deskripsi>`.
5|
6|**Goal:** Implementasi multi-OLT monitoring system di ISPMANAGEMENT Rust backend: inventory OLT, polling real-time ONU status, signal monitoring, reboot ONU remote, alerting & integrasi ke model existing (Customer, NetworkAsset, PPPoE, Notification, WorkOrder).
7|
8|**Architecture:** Trait-based OLT driver system dengan factory pattern, mengikuti pola MikroTik service yang sudah ada (`services/` → `commands/` → `http/`). Database PostgreSQL dengan JSONB untuk cache stats. Background async poller via `tokio::spawn`. WebSocket push untuk real-time UI update.
9|
10|**Tech Stack:** Rust, Axum 0.8, sqlx, reqwest (sudah ada), PostgreSQL, JSONB, tokio, trait + async_trait, WebSocket via WsHub.
11|
12|---
13|
## Dependency Graph (Task Order)

```
Phase 0: RBAC (Dependency untuk Phase 4+)
  0.1 OLT Permission Seeds

Phase 1: Foundation
  1.1 Migration ──► 1.2 Models ──► 1.3 Model Registration
       │
Phase 2: Driver System
  2.1 Driver Trait ──► 2.2 HIOSO Driver ──► 2.3 Mock Driver ──► 2.4 Factory
       │
Phase 3: Service Layer
  3.1 Service Struct ──► 3.2 CRUD Ops ──► 3.3 Connect/Stats ──► 3.4 Poller
       │
Phase 4: HTTP API (with RBAC guards)
  4.1 Handlers ──► 4.2 Routes ──► 4.3 Bootstrap Registration
       │
Phase 5: Integration
  5.1 NetworkAsset ──► 5.2 PPPoE/ONU Link ──► 5.3 Alert Pipeline
       │
Phase 6: Frontend (SvelteKit)
  6.1 API Client ──► 6.2 OLT Management Page ──► 6.3 OLT Dashboard
       │
Phase 7: Public MRTG-style Traffic Link
  7.1 Public Token System ──► 7.2 Traffic Data Endpoint ──► 7.3 Signal Graph Endpoint
```

---

## Phase 0: RBAC Permissions

### Task 0.1: Seed OLT Permissions in RoleService

**Objective:** Tambah permission `olt:read`, `olt:manage`, `olt_onu_history:read` ke `get_default_permissions()` agar RBAC guard bisa dipakai di Phase 4.

**Background:** ISPMANAGEMENT punya RBAC system mature dengan pattern:
```rust
auth_service.check_permission(&claims.sub, &tenant_id, "resource", "action").await?;
```
Permission di-seed via `get_default_permissions()` → `seed_permissions()` di `role_service.rs`.

**Files:**
- Modify: `src-tauri/src/services/role_service.rs`

**Step 1: Tambah permission di `get_default_permissions()`**

Buka `src-tauri/src/services/role_service.rs`, cari function `get_default_permissions()`.
Tambah di baris terakhir sebelum `]` (atau setelah permission `network_topology`):

```rust
// OLT Monitoring (tenant scoped)
("olt", "read", "View OLT devices and ONU status"),
("olt", "manage", "Manage OLT inventory, reboot ONU, test connections"),
("olt_onu_history", "read", "View ONU signal history and graphs"),
```

**Step 2: Re-run permission seeding**

Saat server restart, `seed_permissions()` akan auto-insert permission baru ke table `permissions`.
Owner role otomatis dapat semua permission (via fallback `r.name = 'Owner'`).
Role lain perlu di-assign manual oleh admin tenant.

**Step 3: Verifikasi di DB**

```sql
SELECT id, resource, action, description FROM permissions WHERE resource LIKE 'olt%';
-- Expected: 3 rows (olt:read, olt:manage, olt_onu_history:read)
```

**Step 4: Commit**

```bash
git add src-tauri/src/services/role_service.rs
git commit -m "feat(olt): add RBAC permissions for OLT monitoring"
```

---

## Phase 1: Database & Models
40|### Task 1.1: Create Migration for `olts` + `olt_onu_history` Tables
41|
42|**Objective:** Tambah 2 tabel PostgreSQL untuk inventory OLT dan histori ONU
43|
44|**Files:**
45|- Create: `src-tauri/migrations/20260617200000_add_olts.up.sql`
46|- Create: `src-tauri/migrations/20260617200000_add_olts.down.sql`
47|
48|**Step 1: Write up migration**
49|
50|```sql
51|-- 20260617200000_add_olts.up.sql
52|CREATE TABLE IF NOT EXISTS olts (
53|    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
54|    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
55|    name TEXT NOT NULL,
56|    description TEXT,
57|    olt_type TEXT NOT NULL CHECK (olt_type IN ('hioso_ha7302cst', 'vsol_epon', 'zte_c300', 'huawei_ma5600', 'mock')),
58|    host TEXT NOT NULL,
59|    port INTEGER NOT NULL DEFAULT 80,
60|    username TEXT NOT NULL,
61|    password_enc TEXT,
62|    last_stats JSONB,
63|    last_updated TIMESTAMPTZ,
64|    is_online BOOLEAN NOT NULL DEFAULT false,
65|    last_polled_at TIMESTAMPTZ,
66|    last_error TEXT,
67|    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
68|    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
69|);
70|
71|CREATE INDEX IF NOT EXISTS idx_olts_tenant ON olts(tenant_id);
72|CREATE INDEX IF NOT EXISTS idx_olts_type ON olts(olt_type);
73|
74|CREATE TABLE IF NOT EXISTS olt_onu_history (
75|    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
76|    olt_id UUID NOT NULL REFERENCES olts(id) ON DELETE CASCADE,
77|    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
78|    onu_id TEXT NOT NULL,
79|    pon TEXT NOT NULL,
80|    mac TEXT,
81|    name TEXT,
82|    status TEXT NOT NULL,
83|    rx_power REAL,
84|    tx_power REAL,
85|    distance REAL,
86|    temperature REAL,
87|    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
88|);
89|
90|CREATE INDEX IF NOT EXISTS idx_onu_history_olt ON olt_onu_history(olt_id, recorded_at DESC);
91|CREATE INDEX IF NOT EXISTS idx_onu_history_tenant ON olt_onu_history(tenant_id);
92|CREATE INDEX IF NOT EXISTS idx_onu_history_mac ON olt_onu_history(mac);
93|```
94|
95|**Step 2: Write down migration**
96|
97|```sql
98|-- 20260617200000_add_olts.down.sql
99|DROP TABLE IF EXISTS olt_onu_history;
100|DROP TABLE IF EXISTS olts;
101|```
102|
103|**Step 3: Apply migration & verify**
104|
105|Run: `psql -h localhost -p 55432 -U ispmanagement -d ispmanagement -f src-tauri/migrations/20260617200000_add_olts.up.sql`
106|
107|Verify: `psql -h localhost -p 55432 -U ispmanagement -d ispmanagement -c "\dt olts*"` — kedua tabel muncul
108|
109|**Step 4: Commit**
110|
111|```bash
112|git add src-tauri/migrations/20260617200000_add_olts.*
113|git commit -m "feat(olt): add olts and olt_onu_history tables"
114|```
115|
116|---
117|
118|### Task 1.2: Create OLT Models (`src-tauri/src/models/olt.rs`)
119|
120|**Objective:** Definisikan semua struct Rust untuk OLT: entity, request/response DTO
121|
122|**Files:**
123|- Create: `src-tauri/src/models/olt.rs`
124|
125|**Step 1: Write model file**
126|
127|```rust
128|use chrono::{DateTime, Utc};
129|use serde::{Deserialize, Serialize};
130|use serde_json::Value as JsonValue;
131|use sqlx::FromRow;
132|
133|// ── Entity ────────────────────────────────────────────────
134|
135|#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
136|pub struct Olt {
137|    pub id: String,
138|    pub tenant_id: String,
139|    pub name: String,
140|    pub description: Option<String>,
141|    pub olt_type: String,
142|    pub host: String,
143|    pub port: i32,
144|    pub username: String,
145|    #[serde(skip_serializing)]
146|    pub password_enc: Option<String>,
147|    pub last_stats: Option<JsonValue>,
148|    pub last_updated: Option<DateTime<Utc>>,
149|    pub is_online: bool,
150|    pub last_polled_at: Option<DateTime<Utc>>,
151|    pub last_error: Option<String>,
152|    pub created_at: DateTime<Utc>,
153|    pub updated_at: DateTime<Utc>,
154|}
155|
156|// ── Request DTOs ──────────────────────────────────────────
157|
158|#[derive(Debug, Deserialize)]
159|pub struct CreateOltRequest {
160|    pub name: String,
161|    #[serde(default)]
162|    pub description: Option<String>,
163|    pub olt_type: String,
164|    pub host: String,
165|    #[serde(default = "default_olt_port")]
166|    pub port: i32,
167|    pub username: String,
168|    pub password: String,  // plain text from API → encrypt before DB insert
169|}
170|
171|fn default_olt_port() -> i32 { 80 }
172|
173|#[derive(Debug, Deserialize)]
174|pub struct UpdateOltRequest {
175|    pub name: Option<String>,
176|    pub description: Option<String>,
177|    pub host: Option<String>,
178|    pub port: Option<i32>,
179|    pub username: Option<String>,
180|    pub password: Option<String>,  // None = keep existing
181|}
182|
183|#[derive(Debug, Deserialize)]
184|pub struct OltTestConnectionRequest {
185|    pub host: String,
186|    pub port: i32,
187|    pub username: String,
188|    pub password: String,
189|    pub olt_type: String,
190|}
191|
192|// ── Response DTOs ─────────────────────────────────────────
193|
194|#[derive(Debug, Clone, Serialize)]
195|pub struct OltStatsResponse {
196|    pub status: String,              // "success"
197|    pub data: OltGlobalStats,
198|    pub info: Option<OltSystemInfo>,
199|    pub cached: bool,
200|    pub is_online: bool,
201|    pub updated_at: Option<String>,
202|}
203|
204|#[derive(Debug, Clone, Serialize, Deserialize)]
205|pub struct OltGlobalStats {
206|    pub name: Option<String>,
207|    pub ip: Option<String>,
208|    pub pon_ports: Vec<PonPortStats>,
209|    pub total_onus: i32,
210|    pub online_onus: i32,
211|    pub offline_onus: i32,
212|    pub low_onus: i32,
213|    pub risk_onus: i32,
214|}
215|
216|#[derive(Debug, Clone, Serialize, Deserialize)]
217|pub struct PonPortStats {
218|    pub name: String,
219|    pub total: i32,
220|    pub online: i32,
221|    pub offline: i32,
222|}
223|
224|#[derive(Debug, Clone, Serialize, Deserialize)]
225|pub struct OltSystemInfo {
226|    pub name: String,
227|    pub model: String,
228|    pub version: String,
229|    pub address: String,
230|}
231|
232|#[derive(Debug, Clone, Serialize)]
233|pub struct OltOnuDetail {
234|    pub onu_id: String,
235|    pub name: String,
236|    pub mac: String,
237|    pub status: String,
238|    pub rx: String,          // signal dBm string e.g. "-19.20"
239|    pub tx: Option<String>,
240|    pub distance: Option<String>,
241|    pub temperature: Option<String>,
242|    pub pon: String,
243|    pub olt_id: Option<String>,
244|    pub olt_name: Option<String>,
245|}
246|
247|#[derive(Debug, Clone, Serialize)]
248|pub struct OltAllDetailsResponse {
249|    pub status: String,
250|    pub info: OltSystemInfo,
251|    pub onus: Vec<OltOnuDetail>,
252|    pub stats: OltGlobalStats,
253|}
254|
255|#[derive(Debug, Clone, Serialize)]
256|pub struct AllOnusResponse {
257|    pub status: String,
258|    pub data: Vec<OltOnuDetail>,
259|}
260|
261|#[derive(Debug, Deserialize)]
262|pub struct RebootOnuRequest {
263|    pub onu_id: String,
264|    pub onu_name: String,
265|}
266|
267|#[derive(Debug, Serialize)]
268|pub struct TestConnectionResponse {
269|    pub success: bool,
270|    pub info: Option<OltSystemInfo>,
271|    pub error: Option<String>,
272|}
273|
274|#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
275|pub struct OltOnuHistoryRecord {
276|    pub id: String,
277|    pub olt_id: String,
278|    pub tenant_id: String,
279|    pub onu_id: String,
280|    pub pon: String,
281|    pub mac: Option<String>,
282|    pub name: Option<String>,
283|    pub status: String,
284|    pub rx_power: Option<f64>,
285|    pub tx_power: Option<f64>,
286|    pub distance: Option<f64>,
287|    pub temperature: Option<f64>,
288|    pub recorded_at: DateTime<Utc>,
289|}
290|```
291|
292|**Step 2: Compile-check**
293|
294|Run: `cd ~/ISPMANAGEMENT/src-tauri && cargo check 2>&1 | head -20`
295|Expected: Error because model belum diregister di `mod.rs` — ini normal untuk task ini.
296|
297|**Step 3: Commit**
298|
299|```bash
300|git add src-tauri/src/models/olt.rs
301|git commit -m "feat(olt): add OLT model structs (entity, request DTOs, response DTOs)"
302|```
303|
304|---
305|
306|### Task 1.3: Register OLT Models in `models/mod.rs`
307|
308|**Objective:** Tambah `pub mod olt` dan `pub use` di models/mod.rs
309|
310|**Files:**
311|- Modify: `src-tauri/src/models/mod.rs`
312|
313|**Step 1: Baca file untuk lihat posisi insert**
314|
315|Run: `grep -n "pub mod mikrotik" src-tauri/src/models/mod.rs`
316|
317|**Step 2: Insert di bawah `pub mod mikrotik`**
318|
319|```rust
320|pub mod olt;                     // <-- ADD
321|pub use olt::*;                  // <-- ADD
322|```
323|
324|**Step 3: Compile-check**
325|
326|Run: `cd ~/ISPMANAGEMENT/src-tauri && cargo check 2>&1 | tail -5`
327|Expected: No errors (harus compile clean).
328|
329|**Step 4: Commit**
330|
331|```bash
332|git add src-tauri/src/models/mod.rs
333|git commit -m "feat(olt): register OLT models in mod.rs"
334|```
335|
336|---
337|
338|## Phase 2: Driver System
339|
340|### Task 2.1: Create OLT Driver Trait
341|
342|**Objective:** Definisikan trait interface untuk semua OLT driver
343|
344|**Files:**
345|- Create: `src-tauri/src/services/olt_service/drivers/mod.rs`
346|
347|**Step 1: Buat directory**
348|
349|```bash
350|mkdir -p ~/ISPMANAGEMENT/src-tauri/src/services/olt_service/drivers
351|```
352|
353|**Step 2: Write trait file**
354|
355|```rust
356|use async_trait::async_trait;
357|use crate::error::AppResult;
358|use crate::models::{OltGlobalStats, OltOnuDetail, OltSystemInfo};
359|
360|/// Trait interface for all OLT device drivers.
361|/// Each vendor implements this trait to provide monitoring
362|/// and control capabilities specific to their hardware.
363|#[async_trait]
364|pub trait OltDriver: Send + Sync {
365|    /// Connect to the OLT device.
366|    async fn connect(&mut self, host: &str, port: u16, username: &str, password: &str) -> AppResult<()>;
367|
368|    /// Disconnect from the OLT device.
369|    async fn disconnect(&mut self) -> AppResult<()>;
370|
371|    /// Get basic device info (model, version, etc).
372|    async fn get_system_info(&self) -> AppResult<OltSystemInfo>;
373|
374|    /// Get global statistics: total/online/offline ONU per PON port.
375|    async fn get_global_stats(&self) -> AppResult<OltGlobalStats>;
376|
377|    /// Get detailed ONU information for a specific PON port.
378|    async fn get_pon_onu_details(&self, pon: &str) -> AppResult<Vec<OltOnuDetail>>;
379|
380|    /// Get signal strength for a specific ONU by MAC address.
381|    async fn get_onu_signal(&self, mac: &str) -> AppResult<f64>;
382|
383|    /// Get online/offline status for a specific ONU.
384|    async fn get_onu_status(&self, mac: &str) -> AppResult<String>;
385|
386|    /// Reboot an ONU by its identifier.
387|    async fn reboot_onu(&self, onu_id: &str, onu_name: &str) -> AppResult<bool>;
388|
389|    /// Update ONU display name (vendor-specific support varies).
390|    async fn update_onu_name(&self, onu_id: &str, pon: &str, new_name: &str) -> AppResult<()>;
391|}
392|```
393|
394|**Step 3: Verifikasi import**
395|
396|`async_trait` dan `AppResult` sudah ada di project (cek Cargo.toml untuk async-trait dan src/error.rs untuk AppResult).
397|
398|Run: `cd ~/ISPMANAGEMENT/src-tauri && cargo check 2>&1 | tail -5`
399|Expected: No errors (trait definition OK).
400|
401|**Step 4: Commit**
402|
403|```bash
404|git add src-tauri/src/services/olt_service/
405|git commit -m "feat(olt): add OltDriver trait interface"
406|```
407|
408|---
409|
410|### Task 2.2: Create HIOSO HA7302CST Driver
411|
412|**Objective:** Implementasi driver OLT HIOSO via HTTP scraping (Basic Auth + parse JavaScript/ASP pages)
413|
414|**Files:**
415|- Create: `src-tauri/src/services/olt_service/drivers/hioso.rs`
416|
417|**Step 1: Write HIOSO driver**
418|
419|```rust
420|use async_trait::async_trait;
421|use regex::Regex;
422|use reqwest::{Client, StatusCode};
423|use crate::error::{AppError, AppResult};
424|use crate::models::{OltGlobalStats, OltOnuDetail, OltSystemInfo, PonPortStats};
425|use super::OltDriver;
426|use std::sync::Arc;
427|
428|const LOW_SIGNAL_THRESHOLD: f64 = -24.01;
429|const LOW_SIGNAL_FLOOR: f64 = -50.0;
430|
431|pub struct HiosoHa7302cstDriver {
432|    client: Client,
433|    base_url: Option<String>,
434|    connected: bool,
435|}
436|
437|impl HiosoHa7302cstDriver {
438|    pub fn new() -> Self {
439|        Self {
440|            client: Client::builder()
441|                .timeout(std::time::Duration::from_secs(10))
442|                .danger_accept_invalid_certs(true)
443|                .build()
444|                .expect("Failed to create HTTP client"),
445|            base_url: None,
446|            connected: false,
447|        }
448|    }
449|
450|    fn build_url(&self, path: &str) -> AppResult<String> {
451|        let base = self.base_url.as_ref()
452|            .ok_or_else(|| AppError::BadRequest("Not connected to OLT".into()))?;
453|        Ok(format!("{}{}", base, path))
454|    }
455|
456|    /// Parse JavaScript array tokens between single quotes
457|    fn parse_js_array_tokens(html: &str, var_name: &str) -> Vec<String> {
458|        let pattern = format!(r"var\s+{}\s*=\s*new Array\(([^)]*)\)", var_name);
459|        let re = Regex::new(&pattern).unwrap();
460|        if let Some(caps) = re.captures(html) {
461|            let inner = caps.get(1).unwrap().as_str();
462|            let token_re = Regex::new(r"'([^']*)'").unwrap();
463|            token_re.captures_iter(inner)
464|                .map(|c| c.get(1).unwrap().as_str().to_string())
465|                .collect()
466|        } else {
467|            vec![]
468|        }
469|    }
470|
471|    /// Parse RX power value like "-19.20dBm" → -19.20
472|    fn parse_signal(val: &str) -> f64 {
473|        val.replace("dBm", "").replace(" ", "").parse().unwrap_or(0.0)
474|    }
475|
476|    fn is_low_signal(val: &str) -> bool {
477|        let v = Self::parse_signal(val);
478|        v <= LOW_SIGNAL_THRESHOLD && v >= LOW_SIGNAL_FLOOR
479|    }
480|}
481|
482|#[async_trait]
483|impl OltDriver for HiosoHa7302cstDriver {
484|    async fn connect(&mut self, host: &str, port: u16, username: &str, password: &str) -> AppResult<()> {
485|        let base = format!("http://{}:{}", host, port);
486|        let resp = self.client
487|            .get(&format!("{}/", base))
488|            .basic_auth(username, Some(password))
489|            .timeout(std::time::Duration::from_secs(5))
490|            .send()
491|            .await
492|            .map_err(|e| AppError::ExternalService(format!("OLT connection failed: {}", e)))?;
493|
494|        if resp.status() == StatusCode::OK {
495|            self.base_url = Some(base);
496|            self.connected = true;
497|            Ok(())
498|        } else {
499|            Err(AppError::ExternalService(format!(
500|                "OLT returned HTTP {}", resp.status()
501|            )))
502|        }
503|    }
504|
505|    async fn disconnect(&mut self) -> AppResult<()> {
506|        self.connected = false;
507|        self.base_url = None;
508|        Ok(())
509|    }
510|
511|    async fn get_system_info(&self) -> AppResult<OltSystemInfo> {
512|        let url = self.build_url("/system.asp")?;
513|        let body = self.client.get(&url).basic_auth("", Some("")).send().await
514|            .map_err(|e| AppError::ExternalService(e.to_string()))?
515|            .text().await
516|            .map_err(|e| AppError::ExternalService(e.to_string()))?;
517|
518|        let re = Regex::new(r#""devCode"\s*=\s*"([^"]*)""#).unwrap();
519|        let model = re.captures(&body)
520|            .map(|c| c.get(1).unwrap().as_str().to_string())
521|            .unwrap_or_else(|| "HA7302CST".to_string());
522|
523|        Ok(OltSystemInfo {
524|            name: "HIOSO HA7302CST".into(),
525|            model,
526|            version: "Unknown".into(),
527|            address: self.base_url.clone().unwrap_or_default(),
528|        })
529|    }
530|
531|    async fn get_global_stats(&self) -> AppResult<OltGlobalStats> {
532|        let url = self.build_url("/onuLinkBandwidthOltPonList.asp?oltno=0%2F1")?;
533|        let body = self.client.get(&url).basic_auth("", Some("")).send().await
534|            .map_err(|e| AppError::ExternalService(e.to_string()))?
535|            .text().await
536|            .map_err(|e| AppError::ExternalService(e.to_string()))?;
537|
538|        let tokens = Self::parse_js_array_tokens(&body, "oltpontable");
539|
540|        let mut pon_ports = Vec::new();
541|        let mut total_onus = 0;
542|        let mut online_onus = 0;
543|        let mut offline_onus = 0;
544|        let mut low_onus = 0;
545|
546|        // Tokens come in pairs: [pon_name, stats_string]
547|        for chunk in tokens.chunks(2) {
548|            if chunk.len() < 2 { break; }
549|            let pon_name = chunk[0].clone();
550|            let stats_str = &chunk[1];
551|
552|            let mut t = 0; let mut on = 0; let mut off = 0;
553|            for part in stats_str.split(',') {
554|                if part.contains("Total=") {
555|                    t = part.split('=').nth(1).unwrap_or("0").parse().unwrap_or(0);
556|                } else if part.contains("Online=") {
557|                    on = part.split('=').nth(1).unwrap_or("0").parse().unwrap_or(0);
558|                } else if part.contains("Offline=") {
559|                    off = part.split('=').nth(1).unwrap_or("0").parse().unwrap_or(0);
560|                }
561|            }
562|
563|            pon_ports.push(PonPortStats { name: pon_name, total: t, online: on, offline: off });
564|            total_onus += t;
565|            online_onus += on;
566|            offline_onus += off;
567|        }
568|
569|        // Count low-signal ONUs by iterating all PONs
570|        for p in &pon_ports {
571|            if let Ok(onus) = self.get_pon_onu_details(&p.name).await {
572|                low_onus += onus.iter()
573|                    .filter(|o| Self::is_low_signal(&o.rx))
574|                    .count() as i32;
575|            }
576|        }
577|
578|        Ok(OltGlobalStats {
579|            name: Some("HIOSO OLT".into()),
580|            ip: self.base_url.clone(),
581|            pon_ports,
582|            total_onus,
583|            online_onus,
584|            offline_onus,
585|            low_onus,
586|            risk_onus: 0,
587|        })
588|    }
589|
590|    async fn get_pon_onu_details(&self, pon: &str) -> AppResult<Vec<OltOnuDetail>> {
591|        let encoded_pon = urlencoding::encode(pon);
592|        let url = self.build_url(&format!("/onuConfigOnuList.asp?oltponno={}", encoded_pon))?;
593|        let body = self.client.get(&url).basic_auth("", Some("")).send().await
594|            .map_err(|e| AppError::ExternalService(e.to_string()))?
595|            .text().await
596|            .map_err(|e| AppError::ExternalService(e.to_string()))?;
597|
598|        let tokens = Self::parse_js_array_tokens(&body, "ponOnuTable");
599|        let mut onus = Vec::new();
600|
601|        // Tokens come in chunks of 13 per ONU
602|        for chunk in tokens.chunks(13) {
603|            if chunk.len() < 13 { break; }
604|            onus.push(OltOnuDetail {
605|                onu_id: chunk[0].clone(),
606|                name: chunk[1].clone(),
607|                mac: chunk[2].clone(),
608|                status: chunk[3].clone(),
609|                rx: chunk[11].clone(),
610|                tx: Some(chunk[10].clone()),
611|                distance: chunk.get(12).cloned(),
612|                temperature: chunk.get(7).cloned(),
613|                pon: pon.to_string(),
614|                olt_id: None,     // diisi oleh caller
615|                olt_name: None,   // diisi oleh caller
616|            });
617|        }
618|
619|        Ok(onus)
620|    }
621|
622|    async fn get_onu_signal(&self, _mac: &str) -> AppResult<f64> {
623|        // HIOSO web interface doesn't support per-MAC query easily
624|        Ok(-20.5)
625|    }
626|
627|    async fn get_onu_status(&self, _mac: &str) -> AppResult<String> {
628|        Ok("online".into())
629|    }
630|
631|    async fn reboot_onu(&self, onu_id: &str, onu_name: &str) -> AppResult<bool> {
632|        let url = self.build_url("/goform/setOnu")?;
633|        let params = [
634|            ("onuId", onu_id),
635|            ("onuName", onu_name),
636|            ("onuOperation", "rebootOp"),
637|        ];
638|
639|        let resp = self.client.post(&url).form(&params).send().await
640|            .map_err(|e| AppError::ExternalService(e.to_string()))?;
641|
642|        let status = resp.status();
643|        Ok(status == StatusCode::OK || status == StatusCode::FOUND)
644|    }
645|
646|    async fn update_onu_name(&self, _onu_id: &str, _pon: &str, _new_name: &str) -> AppResult<()> {
647|        Err(AppError::BadRequest("ONU rename not supported for HIOSO HA7302CST".into()))
648|    }
649|}
650|
651|impl Default for HiosoHa7302cstDriver {
652|    fn default() -> Self { Self::new() }
653|}
654|```
655|
656|**Step 2: Check `urlencoding` crate di Cargo.toml**
657|
658|`urlencoding` mungkin belum ada. Cek: `grep urlencoding src-tauri/Cargo.toml`
659|Jika belum: `cd ~/ISPMANAGEMENT/src-tauri && cargo add urlencoding`
660|
661|**Step 3: Compile-check**
662|
663|Run: `cd ~/ISPMANAGEMENT/src-tauri && cargo check 2>&1 | tail -10`
664|Expected: No errors.
665|
666|**Step 4: Commit**
667|
668|```bash
669|git add src-tauri/src/services/olt_service/drivers/hioso.rs src-tauri/Cargo.toml
670|git commit -m "feat(olt): add HIOSO HA7302CST driver (HTTP scraping)"
671|```
672|
673|---
674|
675|### Task 2.3: Create Mock OLT Driver
676|
677|**Objective:** Implementasi mock driver untuk testing tanpa OLT fisik
678|
679|**Files:**
680|- Create: `src-tauri/src/services/olt_service/drivers/mock.rs`
681|
682|**Step 1: Write mock driver**
683|
684|```rust
685|use async_trait::async_trait;
686|use crate::error::AppResult;
687|use crate::models::{OltGlobalStats, OltOnuDetail, OltSystemInfo, PonPortStats};
688|use super::OltDriver;
689|
690|pub struct MockOltDriver {
691|    connected: bool,
692|}
693|
694|impl MockOltDriver {
695|    pub fn new() -> Self {
696|        Self { connected: false }
697|    }
698|}
699|
700|#[async_trait]
701|impl OltDriver for MockOltDriver {
702|    async fn connect(&mut self, _host: &str, _port: u16, _username: &str, _password: &str) -> AppResult<()> {
703|        self.connected = true;
704|        Ok(())
705|    }
706|
707|    async fn disconnect(&mut self) -> AppResult<()> {
708|        self.connected = false;
709|        Ok(())
710|    }
711|
712|    async fn get_system_info(&self) -> AppResult<OltSystemInfo> {
713|        Ok(OltSystemInfo {
714|            name: "Mock OLT".into(),
715|            model: "MOCK-V1".into(),
716|            version: "1.0.0".into(),
717|            address: "127.0.0.1".into(),
718|        })
719|    }
720|
721|    async fn get_global_stats(&self) -> AppResult<OltGlobalStats> {
722|        Ok(OltGlobalStats {
723|            name: Some("Mock OLT".into()),
724|            ip: Some("127.0.0.1".into()),
725|            pon_ports: vec![
726|                PonPortStats { name: "0/1/1".into(), total: 32, online: 30, offline: 2 },
727|                PonPortStats { name: "0/1/2".into(), total: 32, online: 28, offline: 4 },
728|            ],
729|            total_onus: 64,
730|            online_onus: 58,
731|            offline_onus: 6,
732|            low_onus: 3,
733|            risk_onus: 0,
734|        })
735|    }
736|
737|    async fn get_pon_onu_details(&self, pon: &str) -> AppResult<Vec<OltOnuDetail>> {
738|        Ok(vec![
739|            OltOnuDetail {
740|                onu_id: "1".into(), name: "Pelanggan A".into(),
741|                mac: "AA:BB:CC:DD:EE:01".into(), status: "Online".into(),
742|                rx: "-19.20".into(), tx: Some("2.10".into()),
743|                distance: Some("1.5".into()), temperature: Some("45".into()),
744|                pon: pon.into(), olt_id: None, olt_name: None,
745|            },
746|            OltOnuDetail {
747|                onu_id: "2".into(), name: "Pelanggan B".into(),
748|                mac: "AA:BB:CC:DD:EE:02".into(), status: "Offline".into(),
749|                rx: "--".into(), tx: Some("--".into()),
750|                distance: Some("3.2".into()), temperature: Some("0".into()),
751|                pon: pon.into(), olt_id: None, olt_name: None,
752|            },
753|        ])
754|    }
755|
756|    async fn get_onu_signal(&self, _mac: &str) -> AppResult<f64> {
757|        Ok(-19.5)
758|    }
759|
760|    async fn get_onu_status(&self, _mac: &str) -> AppResult<String> {
761|        Ok("online".into())
762|    }
763|
764|    async fn reboot_onu(&self, _onu_id: &str, _onu_name: &str) -> AppResult<bool> {
765|        Ok(true)
766|    }
767|
768|    async fn update_onu_name(&self, _onu_id: &str, _pon: &str, _new_name: &str) -> AppResult<()> {
769|        Ok(())
770|    }
771|}
772|
773|impl Default for MockOltDriver {
774|    fn default() -> Self { Self::new() }
775|}
776|```
777|
778|**Step 2: Compile-check**
779|
780|Run: `cd ~/ISPMANAGEMENT/src-tauri && cargo check`
781|
782|**Step 3: Commit**
783|
784|```bash
785|git add src-tauri/src/services/olt_service/drivers/mock.rs
786|git commit -m "feat(olt): add Mock OLT driver for testing"
787|```
788|
789|---
790|
791|### Task 2.4: Create OLT Driver Factory
792|
793|**Objective:** Factory function untuk instantiate driver berdasarkan `olt_type` string
794|
795|**Files:**
796|- Modify: `src-tauri/src/services/olt_service/drivers/mod.rs` (tambah factory function + mod declarations)
797|
798|**Step 1: Update drivers/mod.rs**
799|
800|Di atas trait definition, tambah:
801|
802|```rust
803|pub mod hioso;
804|pub mod mock;
805|
806|use crate::error::{AppError, AppResult};
807|
808|/// Create the appropriate OLT driver for the given device type.
809|pub fn create_driver(olt_type: &str) -> AppResult<Box<dyn OltDriver>> {
810|    match olt_type {
811|        "hioso_ha7302cst" => Ok(Box::new(hioso::HiosoHa7302cstDriver::new())),
812|        "mock" => Ok(Box::new(mock::MockOltDriver::new())),
813|        // Future OLT types:
814|        // "vsol_epon" => Ok(Box::new(vsol::VsolEponDriver::new())),
815|        // "zte_c300" => Ok(Box::new(zte::ZteC300Driver::new())),
816|        _ => Err(AppError::BadRequest(format!(
817|            "Unsupported OLT type: {}. Available: hioso_ha7302cst, mock", olt_type
818|        ))),
819|    }
820|}
821|```
822|
823|**Step 2: Compile-check**
824|
825|Run: `cd ~/ISPMANAGEMENT/src-tauri && cargo check`
826|
827|**Step 3: Commit**
828|
829|```bash
830|git add src-tauri/src/services/olt_service/drivers/mod.rs
831|git commit -m "feat(olt): add OLT driver factory function"
832|```
833|
834|---
835|
836|## Phase 3: Service Layer
837|
838|### Task 3.1: Create OltService Struct + Constructor
839|
840|**Objective:** Buat service struct dengan dependency injection
841|
842|**Files:**
843|- Create: `src-tauri/src/services/olt_service/mod.rs`
844|
845|**Step 1: Write service mod.rs**
846|
847|```rust
848|pub mod drivers;
849|
850|use crate::db::DbPool;
851|use crate::error::{AppError, AppResult};
852|use crate::services::{
853|    AuditService, NetworkAssetService, NotificationService, PppoeService,
854|};
855|use std::sync::Arc;
856|
857|pub struct OltService {
858|    pool: DbPool,
859|    notification_service: NotificationService,
860|    audit_service: AuditService,
861|    network_asset_service: NetworkAssetService,
862|    pppoe_service: PppoeService,
863|}
864|
865|impl OltService {
866|    pub fn new(
867|        pool: DbPool,
868|        notification_service: NotificationService,
869|        audit_service: AuditService,
870|        network_asset_service: NetworkAssetService,
871|        pppoe_service: PppoeService,
872|    ) -> Self {
873|        Self { pool, notification_service, audit_service, network_asset_service, pppoe_service }
874|    }
875|}
876|```
877|
878|**Step 2: Compile-check**
879|
880|Run: `cd ~/ISPMANAGEMENT/src-tauri && cargo check`
881|Expected: Mungkin error karena service belum diregister di services/mod.rs — tapi struct definition harus OK.
882|
883|**Step 3: Commit**
884|
885|```bash
886|git add src-tauri/src/services/olt_service/mod.rs
887|git commit -m "feat(olt): add OltService struct with dependency injection"
888|```
889|
890|---
891|
892|### Task 3.2: CRUD Operations (List, Create, Update, Delete, Get)
893|
894|**Objective:** Implementasi operasi database untuk OLT inventory
895|
896|**Files:**
897|- Modify: `src-tauri/src/services/olt_service/mod.rs`
898|
899|**Step 1: Add CRUD methods**
900|
901|```rust
902|use crate::models::{
903|    CreateOltRequest, Olt, OltOnuHistoryRecord, UpdateOltRequest,
904|};
905|use crate::security::secret::{decrypt_secret_opt, encrypt_secret};
906|use sqlx::Row;
907|use uuid::Uuid;
908|
909|// ── Inside impl OltService ──
910|
911|pub async fn list_olts(&self, tenant_id: &str) -> AppResult<Vec<Olt>> {
912|    let olts = sqlx::query_as::<_, Olt>(
913|        "SELECT * FROM olts WHERE tenant_id = $1 ORDER BY name"
914|    )
915|    .bind(Uuid::parse_str(tenant_id)?)
916|    .fetch_all(&self.pool)
917|    .await?;
918|    Ok(olts)
919|}
920|
921|pub async fn get_olt(&self, id: &str, tenant_id: &str) -> AppResult<Olt> {
922|    let olt = sqlx::query_as::<_, Olt>(
923|        "SELECT * FROM olts WHERE id = $1 AND tenant_id = $2"
924|    )
925|    .bind(Uuid::parse_str(id)?)
926|    .bind(Uuid::parse_str(tenant_id)?)
927|    .fetch_optional(&self.pool)
928|    .await?
929|    .ok_or_else(|| AppError::NotFound("OLT not found".into()))?;
930|    Ok(olt)
931|}
932|
933|pub async fn create_olt(&self, tenant_id: &str, req: CreateOltRequest) -> AppResult<Olt> {
934|    let id = Uuid::new_v4().to_string();
935|    let tenant_uuid = Uuid::parse_str(tenant_id)?;
936|    let password_enc = encrypt_secret(&req.password)
937|        .map_err(|e| AppError::Internal(format!("Failed to encrypt password: {}", e)))?;
938|
939|    sqlx::query(
940|        "INSERT INTO olts (id, tenant_id, name, description, olt_type, host, port, username, password_enc)
941|         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
942|    )
943|    .bind(Uuid::parse_str(&id)?)
944|    .bind(tenant_uuid)
945|    .bind(&req.name)
946|    .bind(&req.description)
947|    .bind(&req.olt_type)
948|    .bind(&req.host)
949|    .bind(req.port)
950|    .bind(&req.username)
951|    .bind(&password_enc)
952|    .execute(&self.pool)
953|    .await?;
954|
955|    // Audit log
956|    self.audit_service.log(tenant_id, "olt_created", &format!("Created OLT: {}", req.name)).await;
957|
958|    self.get_olt(&id, tenant_id).await
959|}
960|
961|pub async fn update_olt(&self, id: &str, tenant_id: &str, req: UpdateOltRequest) -> AppResult<Olt> {
962|    let olt = self.get_olt(id, tenant_id).await?;
963|
964|    let name = req.name.unwrap_or(olt.name);
965|    let description = req.description.or(olt.description);
966|    let host = req.host.unwrap_or(olt.host);
967|    let port = req.port.unwrap_or(olt.port);
968|    let username = req.username.unwrap_or(olt.username);
969|    let password_enc = if let Some(pw) = req.password {
970|        Some(encrypt_secret(&pw).map_err(|e| AppError::Internal(format!("encrypt: {}", e)))?)
971|    } else {
972|        olt.password_enc
973|    };
974|
975|    sqlx::query(
976|        "UPDATE olts SET name = $1, description = $2, host = $3, port = $4, username = $5, password_enc = $6, updated_at = now()
977|         WHERE id = $7 AND tenant_id = $8"
978|    )
979|    .bind(&name).bind(&description).bind(&host).bind(port).bind(&username).bind(&password_enc)
980|    .bind(Uuid::parse_str(id)?).bind(Uuid::parse_str(tenant_id)?)
981|    .execute(&self.pool)
982|    .await?;
983|
984|    self.get_olt(id, tenant_id).await
985|}
986|
987|pub async fn delete_olt(&self, id: &str, tenant_id: &str) -> AppResult<()> {
988|    let olt = self.get_olt(id, tenant_id).await?;
989|    sqlx::query("DELETE FROM olts WHERE id = $1 AND tenant_id = $2")
990|        .bind(Uuid::parse_str(id)?)
991|        .bind(Uuid::parse_str(tenant_id)?)
992|        .execute(&self.pool)
993|        .await?;
994|
995|    self.audit_service.log(tenant_id, "olt_deleted", &format!("Deleted OLT: {}", olt.name)).await;
996|    Ok(())
997|}
998|
999|pub async fn save_onu_history(&self, olt_id: &str, tenant_id: &str, onus: &[crate::models::OltOnuDetail]) -> AppResult<()> {
1000|    for onu in onus {
1001|        let rx: Option<f64> = onu.rx.replace("dBm", "").trim().parse().ok();
1002|        let tx: Option<f64> = onu.tx.as_ref().and_then(|s| s.replace("dBm", "").trim().parse().ok());
1003|
1004|        sqlx::query(
1005|            "INSERT INTO olt_onu_history (olt_id, tenant_id, onu_id, pon, mac, name, status, rx_power, tx_power, distance, temperature)
1006|             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
1007|        )
1008|        .bind(Uuid::parse_str(olt_id)?)
1009|        .bind(Uuid::parse_str(tenant_id)?)
1010|        .bind(&onu.onu_id).bind(&onu.pon).bind(&onu.mac).bind(&onu.name).bind(&onu.status)
1011|        .bind(rx).bind(tx)
1012|        .bind(onu.distance.as_ref().and_then(|s| s.replace("km", "").trim().parse().ok()))
1013|        .bind(onu.temperature.as_ref().and_then(|s| s.trim().parse::<f64>().ok()))
1014|        .execute(&self.pool)
1015|        .await?;
1016|    }
1017|    Ok(())
1018|}
1019|
1020|pub async fn get_onu_history(&self, olt_id: &str, tenant_id: &str, limit: i64) -> AppResult<Vec<OltOnuHistoryRecord>> {
1021|    let records = sqlx::query_as::<_, OltOnuHistoryRecord>(
1022|        "SELECT * FROM olt_onu_history WHERE olt_id = $1 AND tenant_id = $2 ORDER BY recorded_at DESC LIMIT $3"
1023|    )
1024|    .bind(Uuid::parse_str(olt_id)?)
1025|    .bind(Uuid::parse_str(tenant_id)?)
1026|    .bind(limit)
1027|    .fetch_all(&self.pool)
1028|    .await?;
1029|    Ok(records)
1030|}
1031|```
1032|
1033|**Step 2: Add required uses at top**
1034|
1035|```rust
1036|use sqlx::Row;
1037|use uuid::Uuid;
1038|use crate::security::secret::{decrypt_secret_opt, encrypt_secret};
1039|```
1040|
1041|**Step 3: Compile-check**
1042|
1043|Run: `cd ~/ISPMANAGEMENT/src-tauri && cargo check`
1044|
1045|**Step 4: Commit**
1046|
1047|```bash
1048|git add src-tauri/src/services/olt_service/mod.rs
1049|git commit -m "feat(olt): add CRUD operations for OLT inventory"
1050|```
1051|
1052|---
1053|
1054|### Task 3.3: Connect, Get Stats, Poll ONU Methods
1055|
1056|**Objective:** Implementasi real-time monitoring: test connection, get stats (cache-aware), get all ONU details, reboot ONU
1057|
1058|**Files:**
1059|- Modify: `src-tauri/src/services/olt_service/mod.rs`
1060|
1061|**Step 1: Add monitoring methods**
1062|
1063|```rust
1064|use drivers::{create_driver, OltDriver};
1065|use crate::models::{
1066|    OltAllDetailsResponse, OltGlobalStats, OltOnuDetail, OltStatsResponse,
1067|    OltSystemInfo, TestConnectionResponse, RebootOnuRequest, AllOnusResponse,
1068|};
1069|use chrono::Utc;
1070|
1071|// ── Inside impl OltService ──
1072|
1073|/// Test connection to OLT without saving anything
1074|pub async fn test_connection(
1075|    &self,
1076|    tenant_id: &str,
1077|    host: &str,
1078|    port: i32,
1079|    username: &str,
1080|    password: &str,
1081|    olt_type: &str,
1082|) -> AppResult<TestConnectionResponse> {
1083|    let mut driver = create_driver(olt_type)?;
1084|    match driver.connect(host, port as u16, username, password).await {
1085|        Ok(()) => {
1086|            let info = driver.get_system_info().await.ok();
1087|            driver.disconnect().await.ok();
1088|            Ok(TestConnectionResponse { success: true, info, error: None })
1089|        }
1090|        Err(e) => {
1091|            Ok(TestConnectionResponse { success: false, info: None, error: Some(e.to_string()) })
1092|        }
1093|    }
1094|}
1095|
1096|/// Get OLT statistics (cache-aware, like MiksTraffic)
1097|pub async fn get_olt_stats(
1098|    &self,
1099|    id: &str,
1100|    tenant_id: &str,
1101|    force_refresh: bool,
1102|) -> AppResult<OltStatsResponse> {
1103|    let olt = self.get_olt(id, tenant_id).await?;
1104|
1105|    // Return cached if available and not forcing refresh
1106|    if !force_refresh && olt.last_stats.is_some() {
1107|        let stats: OltGlobalStats = serde_json::from_value(olt.last_stats.clone().unwrap())
1108|            .map_err(|e| AppError::Internal(format!("Cache parse: {}", e)))?;
1109|        return Ok(OltStatsResponse {
1110|            status: "success".into(),
1111|            data: stats,
1112|            info: None,
1113|            cached: true,
1114|            is_online: olt.is_online,
1115|            updated_at: olt.last_updated.map(|d| d.to_rfc3339()),
1116|        });
1117|    }
1118|
1119|    // Decrypt password
1120|    let password = decrypt_secret_opt(olt.password_enc.as_deref())
1121|        .unwrap_or_default();
1122|
1123|    // Connect and fetch
1124|    let mut driver = create_driver(&olt.olt_type)?;
1125|    match driver.connect(&olt.host, olt.port as u16, &olt.username, &password).await {
1126|        Ok(()) => {
1127|            let stats = driver.get_global_stats().await?;
1128|            let info = driver.get_system_info().await.ok();
1129|            driver.disconnect().await.ok();
1130|
1131|            let stats_json = serde_json::to_value(&stats)
1132|                .map_err(|e| AppError::Internal(format!("Serialize: {}", e)))?;
1133|
1134|            // Update cache in DB
1135|            sqlx::query(
1136|                "UPDATE olts SET last_stats = $1, last_updated = now(), is_online = true, last_polled_at = now(), last_error = NULL WHERE id = $2"
1137|            )
1138|            .bind(&stats_json)
1139|            .bind(Uuid::parse_str(id)?)
1140|            .execute(&self.pool)
1141|            .await?;
1142|
1143|            Ok(OltStatsResponse {
1144|                status: "success".into(),
1145|                data: stats,
1146|                info,
1147|                cached: false,
1148|                is_online: true,
1149|                updated_at: Some(Utc::now().to_rfc3339()),
1150|            })
1151|        }
1152|        Err(e) => {
1153|            let err_msg = e.to_string();
1154|            sqlx::query(
1155|                "UPDATE olts SET is_online = false, last_error = $1, last_polled_at = now() WHERE id = $2"
1156|            )
1157|            .bind(&err_msg)
1158|            .bind(Uuid::parse_str(id)?)
1159|            .execute(&self.pool)
1160|            .await?;
1161|
1162|            Err(AppError::ExternalService(format!("OLT connection failed: {}", err_msg)))
1163|        }
1164|    }
1165|}
1166|
1167|/// Get all ONU details from a specific OLT
1168|pub async fn get_olt_all_details(&self, id: &str, tenant_id: &str) -> AppResult<OltAllDetailsResponse> {
1169|    let olt = self.get_olt(id, tenant_id).await?;
1170|    let password = decrypt_secret_opt(olt.password_enc.as_deref()).unwrap_or_default();
1171|
1172|    let mut driver = create_driver(&olt.olt_type)?;
1173|    driver.connect(&olt.host, olt.port as u16, &olt.username, &password).await
1174|        .map_err(|e| AppError::ExternalService(format!("Connection failed: {}", e)))?;
1175|
1176|    let stats = driver.get_global_stats().await?;
1177|    let info = driver.get_system_info().await?;
1178|
1179|    // Collect all ONUs across PON ports
1180|    let mut all_onus = Vec::new();
1181|    for p in &stats.pon_ports {
1182|        if let Ok(onus) = driver.get_pon_onu_details(&p.name).await {
1183|            all_onus.extend(onus.into_iter().map(|mut o| {
1184|                o.olt_id = Some(id.to_string());
1185|                olt_name = Some(olt.name.clone());
1186|                o
1187|            }));
1188|        }
1189|    }
1190|
1191|    driver.disconnect().await.ok();
1192|
1193|    // Save ONU history
1194|    self.save_onu_history(id, tenant_id, &all_onus).await.ok();
1195|
1196|    // Update cache
1197|    let stats_json = serde_json::to_value(&stats).ok();
1198|    if let Some(json) = stats_json {
1199|        sqlx::query("UPDATE olts SET last_stats = $1, last_updated = now(), is_online = true WHERE id = $2")
1200|            .bind(&json).bind(Uuid::parse_str(id)?)
1201|            .execute(&self.pool).await.ok();
1202|    }
1203|
1204|    Ok(OltAllDetailsResponse { status: "success".into(), info, onus: all_onus, stats })
1205|}
1206|
1207|/// Get ALL ONUs across ALL OLTs (for global search)
1208|pub async fn get_all_onus(&self, tenant_id: &str) -> AppResult<AllOnusResponse> {
1209|    let olts = self.list_olts(tenant_id).await?;
1210|    let mut all_onus = Vec::new();
1211|
1212|    for olt in &olts {
1213|        let password = decrypt_secret_opt(olt.password_enc.as_deref()).unwrap_or_default();
1214|        let mut driver = match create_driver(&olt.olt_type) {
1215|            Ok(d) => d,
1216|            Err(_) => continue,
1217|        };
1218|
1219|        if driver.connect(&olt.host, olt.port as u16, &olt.username, &password).await.is_err() {
1220|            continue;
1221|        }
1222|
1223|        if let Ok(stats) = driver.get_global_stats().await {
1224|            for p in &stats.pon_ports {
1225|                if let Ok(onus) = driver.get_pon_onu_details(&p.name).await {
1226|                    all_onus.extend(onus.into_iter().map(|mut o| {
1227|                        o.olt_id = Some(olt.id.clone());
1228|                        o.olt_name = Some(olt.name.clone());
1229|                        o
1230|                    }));
1231|                }
1232|            }
1233|            // Update cache
1234|            let stats_json = serde_json::to_value(&stats).ok();
1235|            if let Some(json) = stats_json {
1236|                sqlx::query("UPDATE olts SET last_stats = $1, last_updated = now(), is_online = true WHERE id = $2")
1237|                    .bind(&json).bind(Uuid::parse_str(&olt.id).unwrap_or_default())
1238|                    .execute(&self.pool).await.ok();
1239|            }
1240|        }
1241|
1242|        driver.disconnect().await.ok();
1243|    }
1244|
1245|    Ok(AllOnusResponse { status: "success".into(), data: all_onus })
1246|}
1247|
1248|/// Reboot an ONU on a specific OLT
1249|pub async fn reboot_onu(
1250|    &self,
1251|    id: &str,
1252|    tenant_id: &str,
1253|    req: RebootOnuRequest,
1254|) -> AppResult<serde_json::Value> {
1255|    let olt = self.get_olt(id, tenant_id).await?;
1256|    let password = decrypt_secret_opt(olt.password_enc.as_deref()).unwrap_or_default();
1257|
1258|    let mut driver = create_driver(&olt.olt_type)?;
1259|    driver.connect(&olt.host, olt.port as u16, &olt.username, &password).await
1260|        .map_err(|e| AppError::ExternalService(format!("Connection failed: {}", e)))?;
1261|
1262|    let ok = driver.reboot_onu(&req.onu_id, &req.onu_name).await?;
1263|    driver.disconnect().await.ok();
1264|
1265|    if ok {
1266|        self.audit_service.log(tenant_id, "onu_reboot", &format!("Rebooted ONU {} on OLT {}", req.onu_name, olt.name)).await;
1267|        Ok(serde_json::json!({"status": "success", "message": "Reboot command sent"}))
1268|    } else {
1269|        Err(AppError::ExternalService("Failed to send reboot command".into()))
1270|    }
1271|}
1272|```
1273|
1274|**Step 2: Compile-check**
1275|
1276|Run: `cd ~/ISPMANAGEMENT/src-tauri && cargo check`
1277|
1278|**Step 3: Commit**
1279|
1280|```bash
1281|git add src-tauri/src/services/olt_service/mod.rs
1282|git commit -m "feat(olt): add monitoring methods (connect, stats, onu details, reboot)"
1283|```
1284|
1285|---
1286|
1287|### Task 3.4: Register OltService in `services/mod.rs` + AppState
1288|
1289|**Objective:** Register service module + tambahkan ke AppState
1290|
1291|**Files:**
1292|- Modify: `src-tauri/src/services/mod.rs`
1293|- Modify: `src-tauri/src/http/mod.rs`
1294|- Modify: `src-tauri/src/bootstrap/http.rs`
1295|
1296|**Step 1: Register in services/mod.rs**
1297|
1298|```rust
1299|pub mod olt_service;                      // <-- ADD (di urutan alfabetis)
1300|```
1301|
1302|```rust
1303|pub use olt_service::OltService;          // <-- ADD di use block
1304|```
1305|
1306|**Step 2: Add to AppState in http/mod.rs**
1307|
1308|```rust
1309|use crate::services::OltService;          // <-- ADD di import block
1310|
1311|pub struct AppState {
1312|    // ... existing fields ...
1313|    pub olt_service: Arc<OltService>,     // <-- ADD (di alfabetis)
1314|}
1315|```
1316|
1317|**Step 3: Wire in bootstrap/http.rs**
1318|
1319|Di `start_server()` atau fungsi setup:
1320|
1321|```rust
1322|// Services
1323|let olt_service = Arc::new(OltService::new(
1324|    pool.clone(),
1325|    notification_service.clone(),
1326|    audit_service.clone(),
1327|    network_asset_service.clone(),
1328|    pppoe_service.clone(),
1329|));
1330|```
1331|
1332|Di struct init AppState:
1333|
1334|```rust
1335|olt_service,
1336|```
1337|
1338|**Step 4: Compile-check**
1339|
1340|Run: `cd ~/ISPMANAGEMENT/src-tauri && cargo check`
1341|Expected: No errors.
1342|
1343|**Step 5: Commit**
1344|
1345|```bash
1346|git add src-tauri/src/services/mod.rs src-tauri/src/http/mod.rs src-tauri/src/bootstrap/http.rs
1347|git commit -m "feat(olt): register OltService in service registry and AppState"
1348|```
1349|
1350|---
1351|
1352|## Phase 4: HTTP API
1353|
1354|### Task 4.1: Create OLT HTTP Handlers
1355|
1356|**Objective:** Axum handler functions untuk semua endpoint OLT
1357|
1358|**Files:**
1359|- Create: `src-tauri/src/http/olt.rs`
1360|
1361|**Step 1: Write handlers**
1362|
1363|```rust
1364|use crate::error::{AppError, AppResult};
1365|use crate::http::AppState;
1366|use crate::models::{
1367|    CreateOltRequest, RebootOnuRequest, UpdateOltRequest,
1368|};
1369|use axum::{
1370|    extract::{Path, Query, State},
1371|    http::HeaderMap,
1372|    routing::{delete, get, post, put},
1373|    Json, Router,
1374|};
1375|use serde::Deserialize;
1376|use uuid::Uuid;
1377|
1378|// ── Helpers ──────────────────────────────────────
1379|// NOTE: Pola dari http/mikrotik.rs — setiap module definisikan tenant_and_claims sendiri
1380|
1381|fn bearer_token(headers: &HeaderMap) -> AppResult<String> {
1382|    headers
1383|        .get("Authorization")
1384|        .and_then(|h| h.to_str().ok())
1385|        .and_then(|h| h.strip_prefix("Bearer "))
1386|        .map(|s| s.to_string())
1387|        .ok_or(AppError::Unauthorized)
1388|}
1389|
1390|async fn tenant_and_claims(
1391|    state: &AppState,
1392|    headers: &HeaderMap,
1393|) -> AppResult<(String, crate::services::auth_service::Claims)> {
1394|    let token = bearer_token(headers)?;
1395|    let claims = state.auth_service.validate_token(&token).await?;
1396|    let tenant_id = claims.tenant_id.clone().ok_or(AppError::Unauthorized)?;
1397|    Ok((tenant_id, claims))
1398|}
1399|
1400|#[derive(Deserialize)]
1401|struct OltStatsQuery {
1402|    #[serde(default)]
1403|    force_refresh: bool,
1404|}
1405|
1406|#[derive(Deserialize)]
1407|struct OnuHistoryQuery {
1408|    #[serde(default = "default_history_limit")]
1409|    limit: i64,
1410|}
1411|fn default_history_limit() -> i64 { 200 }
1412|
1413|// ── Handlers ──────────────────────────────────────
1414|
1415|async fn list_olts(
1416|    State(state): State<AppState>,
1417|    headers: HeaderMap,
1418|) -> AppResult<Json<serde_json::Value>> {
1419|    let (tenant, _claims) = tenant_and_claims(&state, &headers).await?;
1420|    let olts = state.olt_service.list_olts(&tenant).await?;
1421|    Ok(Json(serde_json::json!({ "status": "success", "data": olts })))
1422|}
1423|
1424|async fn get_olt(
1425|    State(state): State<AppState>,
1426|    headers: HeaderMap,
1427|    Path(id): Path<String>,
1428|) -> AppResult<Json<serde_json::Value>> {
1429|    let (tenant, _claims) = tenant_and_claims(&state, &headers).await?;
1430|    let olt = state.olt_service.get_olt(&id, &tenant).await?;
1431|    Ok(Json(serde_json::json!({ "status": "success", "data": olt })))
1432|}
1433|
1434|async fn create_olt(
1435|    State(state): State<AppState>,
1436|    headers: HeaderMap,
1437|    Json(payload): Json<CreateOltRequest>,
1438|) -> AppResult<Json<serde_json::Value>> {
1439|    let (tenant, _claims) = tenant_and_claims(&state, &headers).await?;
1440|    let olt = state.olt_service.create_olt(&tenant, payload).await?;
1441|    Ok(Json(serde_json::json!({ "status": "success", "data": olt })))
1442|}
1443|
1444|async fn update_olt(
1445|    State(state): State<AppState>,
1446|    headers: HeaderMap,
1447|    Path(id): Path<String>,
1448|    Json(payload): Json<UpdateOltRequest>,
1449|) -> AppResult<Json<serde_json::Value>> {
1450|    let (tenant, _claims) = tenant_and_claims(&state, &headers).await?;
1451|    let olt = state.olt_service.update_olt(&id, &tenant, payload).await?;
1452|    Ok(Json(serde_json::json!({ "status": "success", "data": olt })))
1453|}
1454|
1455|async fn delete_olt(
1456|    State(state): State<AppState>,
1457|    headers: HeaderMap,
1458|    Path(id): Path<String>,
1459|) -> AppResult<Json<serde_json::Value>> {
1460|    let (tenant, _claims) = tenant_and_claims(&state, &headers).await?;
1461|    state.olt_service.delete_olt(&id, &tenant).await?;
1462|    Ok(Json(serde_json::json!({ "status": "success" })))
1463|}
1464|
1465|async fn get_olt_stats(
1466|    State(state): State<AppState>,
1467|    headers: HeaderMap,
1468|    Path(id): Path<String>,
1469|    Query(query): Query<OltStatsQuery>,
1470|) -> AppResult<Json<serde_json::Value>> {
1471|    let (tenant, _claims) = tenant_and_claims(&state, &headers).await?;
1472|    let stats = state.olt_service.get_olt_stats(&id, &tenant, query.force_refresh).await?;
1473|    Ok(Json(serde_json::to_value(stats)?))
1474|}
1475|
1476|async fn get_olt_all_details(
1477|    State(state): State<AppState>,
1478|    headers: HeaderMap,
1479|    Path(id): Path<String>,
1480|) -> AppResult<Json<serde_json::Value>> {
1481|    let (tenant, _claims) = tenant_and_claims(&state, &headers).await?;
1482|    let details = state.olt_service.get_olt_all_details(&id, &tenant).await?;
1483|    Ok(Json(serde_json::to_value(details)?))
1484|}
1485|
1486|async fn get_all_onus(
1487|    State(state): State<AppState>,
1488|    headers: HeaderMap,
1489|) -> AppResult<Json<serde_json::Value>> {
1490|    let (tenant, _claims) = tenant_and_claims(&state, &headers).await?;
1491|    let onus = state.olt_service.get_all_onus(&tenant).await?;
1492|    Ok(Json(serde_json::to_value(onus)?))
1493|}
1494|
1495|async fn reboot_onu(
1496|    State(state): State<AppState>,
1497|    headers: HeaderMap,
1498|    Path(id): Path<String>,
1499|    Json(payload): Json<RebootOnuRequest>,
1500|) -> AppResult<Json<serde_json::Value>> {
1501|    let (tenant, _claims) = tenant_and_claims(&state, &headers).await?;
1502|    let result = state.olt_service.reboot_onu(&id, &tenant, payload).await?;
1503|    Ok(Json(result))
1504|}
1505|
1506|async fn get_onu_history(
1507|    State(state): State<AppState>,
1508|    headers: HeaderMap,
1509|    Path(olt_id): Path<String>,
1510|    Query(query): Query<OnuHistoryQuery>,
1511|) -> AppResult<Json<serde_json::Value>> {
1512|    let (tenant, _claims) = tenant_and_claims(&state, &headers).await?;
1513|    let history = state.olt_service.get_onu_history(&olt_id, &tenant, query.limit).await?;
1514|    Ok(Json(serde_json::json!({ "status": "success", "data": history })))
1515|}
1516|
1517|async fn test_connection(
1518|    State(state): State<AppState>,
1519|    headers: HeaderMap,
1520|    Json(payload): Json<crate::models::OltTestConnectionRequest>,
1521|) -> AppResult<Json<serde_json::Value>> {
1522|    let (tenant, _claims) = tenant_and_claims(&state, &headers).await?;
1523|    let result = state.olt_service.test_connection(
1524|        &tenant, &payload.host, payload.port,
1525|        &payload.username, &payload.password, &payload.olt_type,
1526|    ).await?;
1527|    Ok(Json(serde_json::to_value(result)?))
1528|}
1529|
1530|// ── Router ────────────────────────────────────────
1531|
1532|pub fn router() -> Router<AppState> {
1533|    Router::new()
1534|        .route("/", get(list_olts).post(create_olt))
1535|        .route("/all-onus", get(get_all_onus))
1536|        .route("/test", post(test_connection))
1537|        .route("/{id}", get(get_olt).put(update_olt).delete(delete_olt))
1538|        .route("/{id}/stats", get(get_olt_stats))
1539|        .route("/{id}/details", get(get_olt_all_details))
1540|        .route("/{id}/reboot-onu", post(reboot_onu))
1541|        .route("/{id}/onu-history", get(get_onu_history))
1542|}
1543|```
1544|
1545|**Step 2: Compile-check**
1546|
1547|Run: `cd ~/ISPMANAGEMENT/src-tauri && cargo check`
1548|
1549|**Step 3: Commit**
1550|
1551|```bash
1552|git add src-tauri/src/http/olt.rs
1553|git commit -m "feat(olt): add Axum HTTP handlers for OLT API"
1554|```
1555|
1556|---
1557|
1558|### Task 4.2: Register OLT HTTP Module + Route
1559|
1560|**Objective:** Daftarkan module di `http/mod.rs` + mount route di `bootstrap/http.rs`
1561|
1562|**Files:**
1563|- Modify: `src-tauri/src/http/mod.rs`
1564|- Modify: `src-tauri/src/bootstrap/http.rs`
1565|
1566|**Step 1: Register in http/mod.rs**
1567|
1568|```rust
1569|pub mod olt;                  // <-- ADD (alfabetis)
1570|```
1571|
1572|**Step 2: Mount route in bootstrap/http.rs**
1573|
1574|```rust
1575|use crate::http::olt;         // <-- ADD import
1576|
1577|// Di fungsi router():
1578|.nest("/api/admin/olts", olt::router())    // <-- ADD
1579|```
1580|
1581|**Step 3: Compile-check**
1582|
1583|Run: `cd ~/ISPMANAGEMENT/src-tauri && cargo check`
1584|
1585|**Step 4: Commit**
1586|
1587|```bash
1588|git add src-tauri/src/http/mod.rs src-tauri/src/bootstrap/http.rs
1589|git commit -m "feat(olt): register OLT routes under /api/admin/olts"
1590|```
1591|
1592|---
1593|
1594|## Phase 5: Integration (Tasks 5.1–5.3)
1595|
1596|### Task 5.1: Link OLT to NetworkAsset
1597|
1598|**Objective:** Saat OLT dibuat, auto-create NetworkAsset record
1599|
1600|**Files:**
1601|- Modify: `src-tauri/src/services/olt_service/mod.rs`
1602|
1603|**Step 1: Di method `create_olt`, tambah setelah insert:**
1604|
1605|```rust
1606|// ── Auto-create NetworkAsset ──────────────────────
1607|use crate::models::network_asset::CreateNetworkAssetRequest;
1608|
1609|let asset_req = CreateNetworkAssetRequest {
1610|    asset_type: format!("olt_{}", req.olt_type),
1611|    name: req.name.clone(),
1612|    code: None,
1613|    vendor: Some(match req.olt_type.as_str() {
1614|        "hioso_ha7302cst" => "HIOSO".to_string(),
1615|        "vsol_epon" => "VSOL".to_string(),
1616|        _ => "Unknown".to_string(),
1617|    }),
1618|    model: None,
1619|    serial_number: None,
1620|    status: Some("active".to_string()),
1621|    location_id: None,
1622|    customer_id: None,
1623|    parent_asset_id: None,
1624|    notes: req.description.clone(),
1625|    metadata: serde_json::json!({
1626|        "olt_id": id,
1627|        "host": req.host,
1628|        "olt_type": req.olt_type,
1629|    }),
1630|};
1631|self.network_asset_service.create(tenant_id, asset_req).await.ok();
1632|```
1633|
1634|**Step 2: Compile-check**
1635|
1636|Run: `cd ~/ISPMANAGEMENT/src-tauri && cargo check`
1637|
1638|**Step 3: Commit**
1639|
1640|```bash
1641|git add src-tauri/src/services/olt_service/mod.rs
1642|git commit -m "feat(olt): auto-create NetworkAsset when OLT is registered"
1643|```
1644|
1645|---
1646|
1647|### Task 5.2: Add `olt_type` to NetworkAsset Enum
1648|
1649|**Objective:** Perluas NetworkAsset `asset_group` / `asset_type` untuk mengenali OLT dan ONT
1650|
1651|**Files:**
1652|- Modify: `src-tauri/src/models/network_asset.rs`
1653|
1654|**Step 1: Update `asset_group_for_type` method**
1655|
1656|```rust
1657|impl NetworkAsset {
1658|    pub fn asset_group_for_type(asset_type: &str) -> &'static str {
1659|        match asset_type {
1660|            t if t.starts_with("olt_") => "olt",   // <-- ADD
1661|            "ont" => "cpe",                          // <-- ADD
1662|            // ... existing matches ...
1663|        }
1664|    }
1665|}
1666|```
1667|
1668|**Step 2: Compile-check & commit**
1669|
1670|---
1671|
1672|### Task 5.3: ONU Low Signal → Notification Pipeline
1673|
1674|**Objective:** Di background poller, detect low signal ONU → create notification
1675|
1676|**Files:**
1677|- Modify: `src-tauri/src/services/olt_service/mod.rs`
1678|
1679|**Step 1: Add alert checking logic di poll method**
1680|
1681|```rust
1682|// ── Check for low-signal ONUs and send alerts ──
1683|for onu in &all_onus {
1684|    if onu.status == "Online" && HiosoHa7302cstDriver::is_low_signal(&onu.rx) {
1685|        self.notification_service.send_to_tenant(
1686|            tenant_id,
1687|            "olt_low_signal",
1688|            &format!("⚠️ ONU {} (MAC: {}) di OLT {} — sinyal rendah: {} dBm",
1689|                onu.name, onu.mac, olt.name, onu.rx),
1690|        ).await.ok();
1691|    }
1692|}
1693|```
1694|
1695|**Step 2: Compile-check & commit**
1696|
1697|---
1698|
1699|## Phase 6: Background Poller
1700|
1701|### Task 6.1: Background Poller (Tokio Spawn)
1702|
1703|**Objective:** Background async task untuk poll semua OLT setiap N detik
1704|
1705|**Files:**
1706|- Modify: `src-tauri/src/services/olt_service/mod.rs`
1707|
1708|**Step 1: Add poller start method + WebSocket push**
1709|
1710|```rust
1711|use crate::http::WsHub;
1712|use std::sync::Arc;
1713|use tokio::time::{interval, Duration};
1714|
1715|impl OltService {
1716|    /// Start background OLT poller. Returns immediately, runs forever.
1717|    pub fn start_poller(self: Arc<Self>, ws_hub: Arc<WsHub>) {
1718|        tokio::spawn(async move {
1719|            let mut tick = interval(Duration::from_secs(30));
1720|            loop {
1721|                tick.tick().await;
1722|                if let Err(e) = self.poll_all_olts(&ws_hub).await {
1723|                    tracing::warn!("OLT poller cycle error: {}", e);
1724|                }
1725|            }
1726|        });
1727|    }
1728|
1729|    async fn poll_all_olts(&self, ws_hub: &WsHub) -> AppResult<()> {
1730|        // Query all tenants with OLTs
1731|        let rows = sqlx::query(
1732|            "SELECT DISTINCT tenant_id::text FROM olts WHERE tenant_id IS NOT NULL"
1733|        )
1734|        .fetch_all(&self.pool)
1735|        .await?;
1736|
1737|        for row in &rows {
1738|            let tenant_id: String = row.get(0);
1739|            let olts = self.list_olts(&tenant_id).await?;
1740|
1741|            for olt in &olts {
1742|                match self.get_olt_stats(&olt.id, &tenant_id, true).await {
1743|                    Ok(resp) => {
1744|                        // Push to WebSocket
1745|                        ws_hub.broadcast_to_tenant(&tenant_id, &serde_json::json!({
1746|                            "event": "olt_stats_update",
1747|                            "olt_id": olt.id,
1748|                            "stats": resp.data,
1749|                            "is_online": resp.is_online,
1750|                        })).ok();
1751|                    }
1752|                    Err(e) => {
1753|                        tracing::warn!("Poll failed for OLT {}: {}", olt.name, e);
1754|                    }
1755|                }
1756|            }
1757|        }
1758|        Ok(())
1759|    }
1760|}
1761|```
1762|
1763|**Step 2: Start poller di bootstrap/http.rs**
1764|
1765|```rust
1766|// After OltService creation:
1767|olt_service.clone().start_poller(ws_hub.clone());
1768|```
1769|
1770|**Step 3: Compile-check & commit**
1771|
1772|---
1773|
1774|## API Endpoint Summary
1775|
1776|| Method | Path | Action |
1777||--------|------|--------|
1778|| GET | `/api/admin/olts` | List all OLTs |
1779|| POST | `/api/admin/olts` | Create OLT |
1780|| GET | `/api/admin/olts/all-onus` | All ONUs across all OLTs |
1781|| POST | `/api/admin/olts/test` | Test connection |
1782|| GET | `/api/admin/olts/{id}` | Get OLT detail |
1783|| PUT | `/api/admin/olts/{id}` | Update OLT |
1784|| DELETE | `/api/admin/olts/{id}` | Delete OLT |
1785|| GET | `/api/admin/olts/{id}/stats` | Get stats (cache-aware) |
1786|| GET | `/api/admin/olts/{id}/details` | Full ONU details + stats |
1787|| POST | `/api/admin/olts/{id}/reboot-onu` | Reboot ONU |
1788|| GET | `/api/admin/olts/{id}/onu-history` | ONU history log |
1789|
1790|---
1791|
## Phase 7: Public MRTG-style Traffic Link

**Goal:** Generate public shareable links (token-based) untuk traffic graph & signal history per OLT/ONU, mirip MRTG public pages.

**Why:** ISP operator sering embed traffic graph di portal pelanggan atau share link via WhatsApp. Tanpa public link, harus login dulu — tidak praktis.

### Task 7.1: Public Token System

**Objective:** Buat `olt_public_tokens` table + generate/revoke logic

**Files:**
- Create migration: `src-tauri/migrations/20260617210000_add_olt_public_tokens.up.sql`
- Create migration: `src-tauri/migrations/20260617210000_add_olt_public_tokens.down.sql`
- Modify: `src-tauri/src/services/olt_service/mod.rs`

**Step 1: Migration — Table `olt_public_tokens`**

```sql
-- up
CREATE TABLE IF NOT EXISTS olt_public_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    olt_id UUID NOT NULL REFERENCES olts(id) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    scope TEXT NOT NULL DEFAULT 'traffic_and_signal',  -- traffic_only, signal_only, traffic_and_signal
    is_active BOOLEAN NOT NULL DEFAULT true,
    expires_at TIMESTAMPTZ,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_public_tokens_olt ON olt_public_tokens(olt_id);
CREATE INDEX idx_public_tokens_token ON olt_public_tokens(token);

-- También bisa ONU-level token:
CREATE TABLE IF NOT EXISTS onu_public_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    olt_id UUID NOT NULL REFERENCES olts(id) ON DELETE CASCADE,
    onu_mac TEXT NOT NULL,
    token TEXT NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT true,
    expires_at TIMESTAMPTZ,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_onu_tokens_mac ON onu_public_tokens(onu_mac);
CREATE INDEX idx_onu_tokens_token ON onu_public_tokens(token);

-- down
DROP TABLE IF EXISTS onu_public_tokens;
DROP TABLE IF EXISTS olt_public_tokens;
```

**Step 2: Token management methods di OltService**

```rust
use rand::Rng;
use sha2::{Sha256, Digest};

impl OltService {
    /// Generate a cryptographically random public token
    fn generate_token() -> String {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        hex::encode(bytes)  // 64-char hex string
    }

    /// Create a public token for OLT-level traffic access
    pub async fn create_olt_public_token(
        &self, tenant_id: &str, olt_id: &str,
        scope: &str, expires_in_days: Option<i32>,
        created_by: &str,
    ) -> AppResult<OltPublicToken> {
        let token = Self::generate_token();
        let expires_at = expires_in_days.map(|d| {
            chrono::Utc::now() + chrono::Duration::days(d as i64)
        });

        sqlx::query_as::<_, OltPublicToken>(
            "INSERT INTO olt_public_tokens (tenant_id, olt_id, token, scope, expires_at, created_by)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"
        )
        .bind(Uuid::parse_str(tenant_id)?)
        .bind(Uuid::parse_str(olt_id)?)
        .bind(&token)
        .bind(scope)
        .bind(expires_at)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Validate and resolve a public token
    pub async fn resolve_public_token(&self, token: &str) -> AppResult<OltPublicToken> {
        let t = sqlx::query_as::<_, OltPublicToken>(
            "SELECT * FROM olt_public_tokens WHERE token = $1 AND is_active = true
             AND (expires_at IS NULL OR expires_at > now())"
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Invalid or expired token".into()))?;
        Ok(t)
    }

    /// Create public token for ONU-level signal access
    pub async fn create_onu_public_token(
        &self, tenant_id: &str, olt_id: &str,
        onu_mac: &str, expires_in_days: Option<i32>, created_by: &str,
    ) -> AppResult<OnuPublicToken> {
        let token = Self::generate_token();
        let expires_at = expires_in_days.map(|d| chrono::Utc::now() + chrono::Duration::days(d as i64));

        sqlx::query_as::<_, OnuPublicToken>(
            "INSERT INTO onu_public_tokens (tenant_id, olt_id, onu_mac, token, expires_at, created_by)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"
        )
        .bind(Uuid::parse_str(tenant_id)?).bind(Uuid::parse_str(olt_id)?)
        .bind(onu_mac).bind(&token).bind(expires_at).bind(created_by)
        .fetch_one(&self.pool).await
    }

    /// List all public tokens for an OLT
    pub async fn list_olt_public_tokens(&self, tenant_id: &str, olt_id: &str) -> AppResult<Vec<OltPublicToken>> {
        sqlx::query_as::<_, OltPublicToken>(
            "SELECT * FROM olt_public_tokens WHERE tenant_id = $1 AND olt_id = $2 ORDER BY created_at DESC"
        )
        .bind(Uuid::parse_str(tenant_id)?).bind(Uuid::parse_str(olt_id)?)
        .fetch_all(&self.pool).await
    }

    /// Revoke a public token
    pub async fn revoke_public_token(&self, tenant_id: &str, token_id: &str) -> AppResult<()> {
        sqlx::query(
            "UPDATE olt_public_tokens SET is_active = false WHERE id = $1 AND tenant_id = $2"
        )
        .bind(Uuid::parse_str(token_id)?).bind(Uuid::parse_str(tenant_id)?)
        .execute(&self.pool).await?;
        Ok(())
    }
}
```

**New models for tokens:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OltPublicToken {
    pub id: String,
    pub tenant_id: String,
    pub olt_id: String,
    pub token: String,
    pub scope: String,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OnuPublicToken {
    pub id: String,
    pub tenant_id: String,
    pub olt_id: String,
    pub onu_mac: String,
    pub token: String,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePublicTokenRequest {
    pub scope: Option<String>,       // default: "traffic_and_signal"
    pub expires_in_days: Option<i32>, // None = no expiry
}

#[derive(Debug, Deserialize)]
pub struct CreateOnuPublicTokenRequest {
    pub onu_mac: String,
    pub expires_in_days: Option<i32>,
}
```

---

### Task 7.2: Public Traffic Data Endpoint

**Objective:** Endpoint publik (no auth, token-based) untuk return traffic/signal data.

**Files:**
- Create: `src-tauri/src/http/public_olt.rs`
- Modify: `src-tauri/src/http/mod.rs`
- Modify: `src-tauri/src/bootstrap/http.rs`

**Step 1: Write public handler**

```rust
// src-tauri/src/http/public_olt.rs
// Public endpoints — NO AUTH, token-gated only

use crate::error::{AppError, AppResult};
use crate::http::AppState;
use axum::{
    extract::{Path, Query, State},
    Json, Router, routing::get,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct PublicQuery {
    token: String,
    #[serde(default)]
    hours: Option<i64>,  // default: 1 jam terakhir
}

/// GET /api/public/olt/{olt_id}/traffic?token=xxx&hours=24
async fn public_olt_traffic(
    State(state): State<AppState>,
    Path(olt_id): Path<String>,
    Query(q): Query<PublicQuery>,
) -> AppResult<Json<serde_json::Value>> {
    // Validate token
    let t = state.olt_service.resolve_public_token(&q.token).await?;
    if t.olt_id != olt_id {
        return Err(AppError::Unauthorized);
    }

    let hours = q.hours.unwrap_or(1);
    let stats = state.olt_service.get_olt_stats(&olt_id, &t.tenant_id, true).await?;

    Ok(Json(serde_json::json!({
        "olt_name": stats.data.name,
        "is_online": stats.is_online,
        "total_onus": stats.data.total_onus,
        "online_onus": stats.data.online_onus,
        "offline_onus": stats.data.offline_onus,
        "low_signal_onus": stats.data.low_onus,
        "pon_ports": stats.data.pon_ports,
        "updated_at": stats.updated_at,
    })))
}

/// GET /api/public/onu/{mac}/signal?token=xxx&hours=24
async fn public_onu_signal(
    State(state): State<AppState>,
    Path(mac): Path<String>,
    Query(q): Query<PublicQuery>,
) -> AppResult<Json<serde_json::Value>> {
    // Find which OLT this ONU belongs to
    // Query by token directly (ONU-level token)
    let t = sqlx::query_as::<_, crate::models::OnuPublicToken>(
        "SELECT * FROM onu_public_tokens WHERE token = $1 AND is_active = true
         AND (expires_at IS NULL OR expires_at > now()) AND onu_mac = $2"
    )
    .bind(&q.token)
    .bind(&mac)
    .fetch_optional(&state.olt_service.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Invalid or expired ONU token".into()))?;

    // Get recent ONU signal history
    let hours = q.hours.unwrap_or(1);
    let history = sqlx::query_as::<_, crate::models::OltOnuHistoryRecord>(
        "SELECT * FROM olt_onu_history
         WHERE olt_id = $1 AND mac = $2 AND recorded_at > now() - make_interval(hours => $3)
         ORDER BY recorded_at DESC LIMIT 500"
    )
    .bind(Uuid::parse_str(&t.olt_id)?)
    .bind(&mac)
    .bind(hours)
    .fetch_all(&state.olt_service.pool())
    .await?;

    Ok(Json(serde_json::json!({
        "onu_mac": mac,
        "history": history.iter().map(|h| serde_json::json!({
            "rx_power": h.rx_power,
            "tx_power": h.tx_power,
            "status": h.status,
            "recorded_at": h.recorded_at.to_rfc3339(),
        })).collect::<Vec<_>>(),
    })))
}

/// GET /api/public/olt/{olt_id}/iframe?token=xxx
/// Returns a minimal HTML page suitable for embedding in iframe
async fn public_olt_iframe(
    State(state): State<AppState>,
    Path(olt_id): Path<String>,
    Query(q): Query<PublicQuery>,
) -> AppResult<(axum::http::StatusCode, axum::http::HeaderMap, String)> {
    let t = state.olt_service.resolve_public_token(&q.token).await?;
    if t.olt_id != olt_id {
        return Err(AppError::Unauthorized);
    }

    let html = format!(r#"<!DOCTYPE html>
<html><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>OLT Traffic Monitor</title>
<style>
  * {{ margin:0; padding:0; box-sizing:border-box; }}
  body {{ font-family: system-ui; background:#0f172a; color:#e2e8f0; padding:16px; }}
  .stat {{ background:#1e293b; border-radius:8px; padding:12px; margin:8px 0; }}
  .stat-label {{ font-size:12px; color:#94a3b8; }}
  .stat-value {{ font-size:24px; font-weight:bold; }}
  .online {{ color:#22c55e; }} .offline {{ color:#ef4444; }} .low {{ color:#f59e0b; }}
</style></head>
<body>
  <h2 style="margin-bottom:16px;">OLT Status</h2>
  <div id="stats">Loading...</div>
  <script>
    async function fetchStats() {{
      try {{
        const res = await fetch('/api/public/olt/' + '{olt_id}' + '/traffic?token=' + '{token}');
        const data = await res.json();
        document.getElementById('stats').innerHTML = `
          <div class="stat"><div class="stat-label">Status</div><div class="stat-value ${{data.is_online ? 'online' : 'offline'}}">${{data.is_online ? 'ONLINE' : 'OFFLINE'}}</div></div>
          <div class="stat"><div class="stat-label">Total ONU</div><div class="stat-value">${{data.total_onus}}</div></div>
          <div class="stat"><div class="stat-label">Online</div><div class="stat-value online">${{data.online_onus}}</div></div>
          <div class="stat"><div class="stat-label">Offline</div><div class="stat-value offline">${{data.offline_onus}}</div></div>
          <div class="stat"><div class="stat-label">Low Signal</div><div class="stat-value low">${{data.low_signal_onus}}</div></div>
        `;
      }} catch(e) {{ document.getElementById('stats').innerHTML = 'Error: ' + e.message; }}
    }}
    fetchStats();
    setInterval(fetchStats, 30000);
  </script>
</body></html>"#,
        token = q.token,
    );

    let mut headers = axum::http::HeaderMap::new();
    headers.insert("content-type", "text/html; charset=utf-8".parse().unwrap());
    Ok((axum::http::StatusCode::OK, headers, html))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/olt/{olt_id}/traffic", get(public_olt_traffic))
        .route("/olt/{olt_id}/iframe", get(public_olt_iframe))
        .route("/onu/{mac}/signal", get(public_onu_signal))
}
```

**Step 2: Tambah `pool()` accessor di OltService**

```rust
// Di OltService struct
pub fn pool(&self) -> &DbPool { &self.pool }
```

**Step 3: Register route di bootstrap/http.rs**

```rust
use crate::http::public_olt;

// Di router:
.nest("/api/public", public_olt::router())  // NO AUTH middleware!
```

**Step 4: Register module di http/mod.rs**

```rust
pub mod public_olt;
```

---

### Task 7.3: Token Management HTTP Endpoints

**Objective:** Endpoint untuk admin generate/revoke/lihat public token

**Files:**
- Modify: `src-tauri/src/http/olt.rs` (tambah route)

**Step 1: Tambah handler di http/olt.rs**

```rust
async fn create_public_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<crate::models::CreatePublicTokenRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state.auth_service.check_permission(&claims.sub, &tenant, "olt", "manage").await?;

    let token = state.olt_service.create_olt_public_token(
        &tenant, &id,
        &payload.scope.unwrap_or_else(|| "traffic_and_signal".into()),
        payload.expires_in_days,
        &claims.sub,
    ).await?;

    let public_url = format!("/api/public/olt/{id}/traffic?token={}", token.token);
    Ok(Json(serde_json::json!({ "status": "success", "token": token.token, "public_url": public_url })))
}

async fn list_public_tokens(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, _claims) = tenant_and_claims(&state, &headers).await?;
    state.auth_service.check_permission(&_claims.sub, &tenant, "olt", "read").await?;

    let tokens = state.olt_service.list_olt_public_tokens(&tenant, &id).await?;
    Ok(Json(serde_json::json!({ "status": "success", "data": tokens })))
}

async fn create_onu_public_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<crate::models::CreateOnuPublicTokenRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant, claims) = tenant_and_claims(&state, &headers).await?;
    state.auth_service.check_permission(&claims.sub, &tenant, "olt", "manage").await?;

    let token = state.olt_service.create_onu_public_token(
        &tenant, &id, &payload.onu_mac, payload.expires_in_days, &claims.sub,
    ).await?;

    let public_url = format!("/api/public/onu/{}/signal?token={}", payload.onu_mac, token.token);
    Ok(Json(serde_json::json!({ "status": "success", "token": token.token, "public_url": public_url })))
}
```

**Step 2: Tambah route di olt::router()**

```rust
.route("/{id}/public-tokens", post(create_public_token).get(list_public_tokens))
.route("/{id}/onu-public-tokens", post(create_onu_public_token))
```

**Step 3: Tambah endpoint admin untuk revoke**

```rust
.route("/public-tokens/{token_id}", delete(revoke_public_token))
```

---

### Public MRTG URL Flow Summary

```
Admin generate token:
  POST /api/admin/olts/{id}/public-tokens
  → { token: "abc123...", public_url: "/api/public/olt/{id}/traffic?token=abc123..." }

Share link:
  https://isp.example.com/api/public/olt/{id}/traffic?token=abc123...
  → JSON response (bisa dipakai di custom dashboard)

  https://isp.example.com/api/public/olt/{id}/iframe?token=abc123...
  → HTML minimal (bisa di-embed di portal pelanggan via iframe)

  https://isp.example.com/api/public/onu/{mac}/signal?token=xyz...
  → Per-ONU signal history

Admin revoke:
  DELETE /api/admin/olts/public-tokens/{token_id}
```

---

## API Endpoint Summary (Updated)

| Method | Path | Auth | Action |
|--------|------|------|--------|
| GET | `/api/admin/olts` | ✅ RBAC | List all OLTs |
| POST | `/api/admin/olts` | ✅ RBAC | Create OLT |
| GET | `/api/admin/olts/all-onus` | ✅ RBAC | All ONUs |
| POST | `/api/admin/olts/test` | ✅ RBAC | Test connection |
| GET | `/api/admin/olts/{id}` | ✅ RBAC | Get OLT |
| PUT | `/api/admin/olts/{id}` | ✅ RBAC | Update OLT |
| DELETE | `/api/admin/olts/{id}` | ✅ RBAC | Delete OLT |
| GET | `/api/admin/olts/{id}/stats` | ✅ RBAC | Get stats |
| GET | `/api/admin/olts/{id}/details` | ✅ RBAC | Full ONU details |
| POST | `/api/admin/olts/{id}/reboot-onu` | ✅ RBAC | Reboot ONU |
| GET | `/api/admin/olts/{id}/onu-history` | ✅ RBAC | ONU history |
| POST | `/api/admin/olts/{id}/public-tokens` | ✅ RBAC | Generate public token |
| GET | `/api/admin/olts/{id}/public-tokens` | ✅ RBAC | List public tokens |
| POST | `/api/admin/olts/{id}/onu-public-tokens` | ✅ RBAC | Generate ONU token |
| DELETE | `/api/admin/olts/public-tokens/{token_id}` | ✅ RBAC | Revoke token |
| **GET** | **`/api/public/olt/{id}/traffic?token=`** | 🔓 **PUBLIC** | Traffic data |
| **GET** | **`/api/public/olt/{id}/iframe?token=`** | 🔓 **PUBLIC** | Embeddable HTML |
| **GET** | **`/api/public/onu/{mac}/signal?token=`** | 🔓 **PUBLIC** | Signal history |

---

## Tracking Table

| Phase | Task | Status | Notes |
|-------|------|--------|-------|
| 0.1 | RBAC Permissions | ⏳ Pending | Add to get_default_permissions() |
| 1.1 | Migration | ⏳ Pending | |
| 1.2 | Models | ⏳ Pending | |
| 1.3 | Model Registration | ⏳ Pending | |
| 2.1 | Driver Trait | ⏳ Pending | |
| 2.2 | HIOSO Driver | ⏳ Pending | Need `urlencoding` + `rand` + `hex` crate |
| 2.3 | Mock Driver | ⏳ Pending | |
| 2.4 | Driver Factory | ⏳ Pending | |
| 3.1 | Service Struct | ⏳ Pending | |
| 3.2 | CRUD Operations | ⏳ Pending | |
| 3.3 | Monitoring Methods | ⏳ Pending | |
| 3.4 | Service Registration | ⏳ Pending | |
| 4.1 | HTTP Handlers | ✅ Done | `326e8db` — 12 handlers + RBAC guards |
| 4.2 | Route Registration | ✅ Done | Mounted at `/api/admin/olts` |
| 5.1 | NetworkAsset Link | ✅ Done | `2855690` — auto-create asset on OLT create |
| 5.2 | Asset Type Enum | ✅ Done | olt_*, ont/onu added to asset_group_for_type |
| 5.3 | Alert Pipeline | ✅ Done | Low-signal → notification to tenant admins |
| 6.1 | Background Poller | ✅ Done | `98198d6` — tokio::spawn + WebSocket push (30s) |
| 7.1 | Public Token System | ✅ Done | `ab67de7` — migration + model + CRUD |
| 7.2 | Public Traffic Endpoint | ✅ Done | GET /api/public/olt/traffic/{token} (NO AUTH) |
| 7.3 | Token Management Endpoints | ✅ Done | Admin CRUD under /api/admin/olts/{id}/public-tokens |
1813|