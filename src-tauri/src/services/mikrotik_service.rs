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
    CreateMikrotikIpPoolRequest, CreateMikrotikPppProfileRequest, CreateMikrotikRouterRequest,
    MikrotikAlert, MikrotikHealthSnapshot, MikrotikIncident, MikrotikInterfaceCounter,
    MikrotikInterfaceMetric, MikrotikInterfaceSnapshot, MikrotikIpAddressSnapshot, MikrotikIpPool,
    MikrotikIpPoolDeleteResult, MikrotikIpPoolDependencyItem, MikrotikIpPoolDependencyStatus,
    MikrotikLogClearResult, MikrotikLogEntry, MikrotikLogRetentionSettings, MikrotikLogSyncResult,
    MikrotikPppProfile, MikrotikPppProfileDeleteResult, MikrotikPppProfileDependencyItem,
    MikrotikPppProfileDependencyStatus, MikrotikRouter, MikrotikRouterMetric, MikrotikRouterNocRow,
    MikrotikRouterSnapshot, MikrotikTestResult, PaginatedResponse, UpdateMikrotikIpPoolRequest,
    UpdateMikrotikPppProfileRequest, UpdateMikrotikRouterRequest,
};
use crate::security::secret::{decrypt_secret_opt, encrypt_secret};
use crate::services::{AuditService, NotificationService, SettingsService};
use chrono::DateTime;
use chrono::{Duration as ChronoDuration, Utc};
use mikrotik_rs::{protocol::command::CommandBuilder, protocol::CommandResponse, MikrotikDevice};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};

// Default thresholds (kept in sync with UI "risk" filters).
// TODO: make configurable per tenant via Settings.
const CPU_RISK: i32 = 70;
const CPU_HOT: i32 = 85;
const LATENCY_RISK_MS: i32 = 200;
const LATENCY_HOT_MS: i32 = 400;
const OFFLINE_AFTER_SECS: i64 = 60;
const WALLBOARD_SLOTS_SETTING_KEY: &str = "mikrotik_wallboard_slots_json";
const WALLBOARD_TRACK_CACHE_TTL_SECS: u64 = 10;
pub(crate) const MIKROTIK_LOGS_DEFAULT_PAGE: u32 = 1;
pub(crate) const MIKROTIK_LOGS_DEFAULT_PER_PAGE: u32 = 25;
pub(crate) const MIKROTIK_LOGS_DEFAULT_INCLUDE_TOTAL: bool = false;

#[derive(Clone, Copy)]
struct Thresholds {
    enabled: bool,
    cpu_risk: i32,
    cpu_hot: i32,
    latency_risk_ms: i32,
    latency_hot_ms: i32,
    offline_after_secs: i64,
}

#[cfg(test)]
type SyncLogRawRow = (Option<String>, Option<String>, Option<String>, String);

#[cfg(test)]
#[derive(Clone, Default)]
struct TestSyncLogsInjection {
    rows_override: Option<Vec<SyncLogRawRow>>,
    force_prune_error: bool,
}

#[derive(Clone)]
pub struct MikrotikService {
    pool: DbPool,
    notification_service: NotificationService,
    audit_service: AuditService,
    settings_service: SettingsService,
    wallboard_track_cache:
        Arc<std::sync::RwLock<HashMap<String, (Instant, HashMap<String, HashSet<String>>)>>>,
    #[cfg(test)]
    test_sync_logs_injection: Arc<std::sync::RwLock<Option<TestSyncLogsInjection>>>,
}

impl MikrotikService {
    fn normalize_interface_name(name: &str) -> String {
        name.trim()
            .to_ascii_lowercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect()
    }

    fn has_interface_counters(interface: &MikrotikInterfaceSnapshot) -> bool {
        interface.rx_byte.is_some() || interface.tx_byte.is_some()
    }

    fn is_active_interface(interface: &MikrotikInterfaceSnapshot) -> bool {
        interface.running.unwrap_or(false)
            || (!interface.disabled.unwrap_or(false) && Self::has_interface_counters(interface))
    }

    fn is_priority_physical_interface(interface: &MikrotikInterfaceSnapshot) -> bool {
        let normalized = Self::normalize_interface_name(&interface.name);
        let is_named_physical = normalized.starts_with("ether")
            || normalized.starts_with("sfp")
            || normalized.starts_with("combo")
            || normalized.starts_with("qsfp");
        let is_typed_physical = interface
            .interface_type
            .as_deref()
            .map(|kind| {
                let kind = kind.trim().to_ascii_lowercase();
                matches!(
                    kind.as_str(),
                    "ether" | "ethernet" | "sfp" | "combo" | "qsfpplus" | "qsfp28"
                )
            })
            .unwrap_or(false);
        (is_named_physical || is_typed_physical) && Self::is_active_interface(interface)
    }

    fn validate_router_coordinates(latitude: Option<f64>, longitude: Option<f64>) -> AppResult<()> {
        if let Some(lat) = latitude {
            if !(-90.0..=90.0).contains(&lat) {
                return Err(AppError::Validation(
                    "latitude must be between -90 and 90".to_string(),
                ));
            }
        }
        if let Some(lng) = longitude {
            if !(-180.0..=180.0).contains(&lng) {
                return Err(AppError::Validation(
                    "longitude must be between -180 and 180".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn log_level_from_topics(topics: Option<&str>) -> String {
        let t = topics.unwrap_or_default().to_ascii_lowercase();
        if t.contains("critical") {
            return "critical".to_string();
        }
        if t.contains("error") {
            return "error".to_string();
        }
        if t.contains("warning") {
            return "warning".to_string();
        }
        if t.contains("debug") {
            return "debug".to_string();
        }
        "info".to_string()
    }

    pub fn new(
        pool: DbPool,
        notification_service: NotificationService,
        audit_service: AuditService,
        settings_service: SettingsService,
    ) -> Self {
        Self {
            pool,
            notification_service,
            audit_service,
            settings_service,
            wallboard_track_cache: Arc::new(std::sync::RwLock::new(HashMap::new())),
            #[cfg(test)]
            test_sync_logs_injection: Arc::new(std::sync::RwLock::new(None)),
        }
    }

    #[cfg(test)]
    fn configure_test_sync_logs_injection(&self, injection: TestSyncLogsInjection) {
        if let Ok(mut guard) = self.test_sync_logs_injection.write() {
            *guard = Some(injection);
        }
    }

    #[cfg(test)]
    fn test_sync_rows_override(&self) -> Option<Vec<SyncLogRawRow>> {
        self.test_sync_logs_injection.read().ok().and_then(|guard| {
            guard
                .as_ref()
                .and_then(|injection| injection.rows_override.clone())
        })
    }

    #[cfg(test)]
    fn test_sync_force_prune_error(&self) -> bool {
        self.test_sync_logs_injection
            .read()
            .ok()
            .and_then(|guard| guard.as_ref().map(|injection| injection.force_prune_error))
            .unwrap_or(false)
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
              r.maintenance_until, r.maintenance_reason,
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

    pub async fn list_alerts(
        &self,
        tenant_id: &str,
        active_only: bool,
        limit: u32,
    ) -> AppResult<Vec<MikrotikAlert>> {
        let rows = if active_only {
            sqlx::query_as::<_, MikrotikAlert>(
                r#"
                SELECT * FROM mikrotik_alerts
                WHERE tenant_id = $1 AND resolved_at IS NULL
                ORDER BY updated_at DESC
                LIMIT $2
                "#,
            )
            .bind(tenant_id)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?
        } else {
            sqlx::query_as::<_, MikrotikAlert>(
                r#"
                SELECT * FROM mikrotik_alerts
                WHERE tenant_id = $1
                ORDER BY updated_at DESC
                LIMIT $2
                "#,
            )
            .bind(tenant_id)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?
        };

        Ok(rows)
    }

    pub async fn list_incidents(
        &self,
        tenant_id: &str,
        active_only: bool,
        limit: u32,
    ) -> AppResult<Vec<MikrotikIncident>> {
        let rows = if active_only {
            sqlx::query_as::<_, MikrotikIncident>(
                r#"
                SELECT
                  i.*,
                  EXISTS(
                    SELECT 1 FROM audit_logs a
                    WHERE CAST(a.tenant_id AS TEXT) = CAST(i.tenant_id AS TEXT)
                      AND a.resource = 'mikrotik_incident'
                      AND CAST(a.resource_id AS TEXT) = CAST(i.id AS TEXT)
                      AND a.action = 'escalate'
                  ) AS is_auto_escalated,
                  (
                    SELECT MAX(a.created_at) FROM audit_logs a
                    WHERE CAST(a.tenant_id AS TEXT) = CAST(i.tenant_id AS TEXT)
                      AND a.resource = 'mikrotik_incident'
                      AND CAST(a.resource_id AS TEXT) = CAST(i.id AS TEXT)
                      AND a.action = 'escalate'
                  ) AS escalated_at
                FROM mikrotik_incidents i
                WHERE i.tenant_id = $1 AND i.resolved_at IS NULL
                ORDER BY i.updated_at DESC
                LIMIT $2
                "#,
            )
            .bind(tenant_id)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?
        } else {
            sqlx::query_as::<_, MikrotikIncident>(
                r#"
                SELECT
                  i.*,
                  EXISTS(
                    SELECT 1 FROM audit_logs a
                    WHERE CAST(a.tenant_id AS TEXT) = CAST(i.tenant_id AS TEXT)
                      AND a.resource = 'mikrotik_incident'
                      AND CAST(a.resource_id AS TEXT) = CAST(i.id AS TEXT)
                      AND a.action = 'escalate'
                  ) AS is_auto_escalated,
                  (
                    SELECT MAX(a.created_at) FROM audit_logs a
                    WHERE CAST(a.tenant_id AS TEXT) = CAST(i.tenant_id AS TEXT)
                      AND a.resource = 'mikrotik_incident'
                      AND CAST(a.resource_id AS TEXT) = CAST(i.id AS TEXT)
                      AND a.action = 'escalate'
                  ) AS escalated_at
                FROM mikrotik_incidents i
                WHERE i.tenant_id = $1
                ORDER BY i.updated_at DESC
                LIMIT $2
                "#,
            )
            .bind(tenant_id)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?
        };
        Ok(rows)
    }

    pub async fn trigger_auto_escalation_now(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> AppResult<i64> {
        let count = self.auto_escalate_incidents(tenant_id).await?;
        self.audit_service
            .log(
                Some(user_id),
                Some(tenant_id),
                "run_auto_escalation",
                "mikrotik_incident",
                None,
                Some(&format!(
                    "Manual auto escalation run; escalated {} incident(s)",
                    count
                )),
                None,
            )
            .await;
        Ok(count)
    }

    pub async fn update_incident(
        &self,
        tenant_id: &str,
        incident_id: &str,
        owner_user_id: Option<String>,
        notes: Option<String>,
        user_id: &str,
    ) -> AppResult<MikrotikIncident> {
        let now = Utc::now();
        let previous_owner: Option<String> = sqlx::query_scalar(
            r#"
            SELECT owner_user_id
            FROM mikrotik_incidents
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(incident_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .flatten();
        let normalized_owner = owner_user_id.and_then(|v| {
            let s = v.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        });
        let normalized_notes = notes.and_then(|v| {
            let s = v.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        });

        let affected = sqlx::query(
            r#"
            UPDATE mikrotik_incidents
            SET owner_user_id = $1,
                notes = $2,
                status = CASE
                  WHEN resolved_at IS NULL
                    AND status = 'open'
                    AND ($1 IS NOT NULL OR $2 IS NOT NULL) THEN 'in_progress'
                  ELSE status
                END,
                updated_at = $3
            WHERE id = $4 AND tenant_id = $5
            "#,
        )
        .bind(&normalized_owner)
        .bind(&normalized_notes)
        .bind(now)
        .bind(incident_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::NotFound("Incident not found".to_string()));
        }

        let incident = sqlx::query_as::<_, MikrotikIncident>(
            r#"
            SELECT * FROM mikrotik_incidents
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(incident_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.audit_service
            .log(
                Some(user_id),
                Some(tenant_id),
                "update",
                "mikrotik_incident",
                Some(incident_id),
                Some("Updated incident assignment/notes"),
                None,
            )
            .await;

        if normalized_owner != previous_owner {
            if let Some(assignee_user_id) = normalized_owner.clone() {
                let is_member: bool = sqlx::query_scalar(
                    r#"
                    SELECT EXISTS(
                      SELECT 1 FROM tenant_members
                      WHERE tenant_id = $1 AND user_id = $2
                    )
                    "#,
                )
                .bind(tenant_id)
                .bind(&assignee_user_id)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(false);

                if is_member {
                    let router_name: Option<String> = sqlx::query_scalar(
                        r#"
                        SELECT name FROM mikrotik_routers
                        WHERE id = $1 AND tenant_id = $2
                        "#,
                    )
                    .bind(&incident.router_id)
                    .bind(tenant_id)
                    .fetch_optional(&self.pool)
                    .await
                    .ok()
                    .flatten();

                    let incident_target = if let Some(iface) = incident.interface_name.as_deref() {
                        format!(
                            "{} ({iface})",
                            router_name.unwrap_or(incident.router_id.clone())
                        )
                    } else {
                        router_name.unwrap_or(incident.router_id.clone())
                    };

                    let _ = self
                        .notification_service
                        .create_notification(
                            assignee_user_id.clone(),
                            Some(tenant_id.to_string()),
                            format!("Incident assigned: {}", incident.title),
                            format!(
                                "You were assigned to incident on {}. Current status: {}.",
                                incident_target, incident.status
                            ),
                            "warning".to_string(),
                            "network".to_string(),
                            Some(format!("/admin/network/incidents?incident={}", incident.id)),
                        )
                        .await;

                    let assignment_email_enabled = match self
                        .settings_service
                        .get_value(
                            Some(tenant_id),
                            "mikrotik_incident_assignment_email_enabled",
                        )
                        .await
                    {
                        Ok(Some(v)) => matches!(
                            v.trim().to_ascii_lowercase().as_str(),
                            "true" | "1" | "yes" | "on"
                        ),
                        _ => false,
                    };

                    if assignment_email_enabled {
                        let assignee_email: Option<String> = sqlx::query_scalar(
                            r#"
                            SELECT u.email
                            FROM users u
                            JOIN tenant_members tm ON tm.user_id = u.id
                            WHERE tm.tenant_id = $1 AND u.id = $2
                            LIMIT 1
                            "#,
                        )
                        .bind(tenant_id)
                        .bind(&assignee_user_id)
                        .fetch_optional(&self.pool)
                        .await
                        .ok()
                        .flatten();

                        if let Some(email) = assignee_email {
                            let subject = format!("Incident Assigned: {}", incident.title);
                            let body = format!(
                                "You were assigned to incident:\n{}\n\nTarget: {}\nStatus: {}\nOpen: /admin/network/incidents?incident={}",
                                incident.message, incident_target, incident.status, incident.id
                            );
                            let _ = self
                                .notification_service
                                .force_send_email(
                                    Some(tenant_id.to_string()),
                                    &email,
                                    &subject,
                                    &body,
                                )
                                .await;
                        }
                    }
                }
            }
        }

        Ok(incident)
    }

    pub async fn simulate_incident(
        &self,
        tenant_id: &str,
        user_id: &str,
        router_id: &str,
        incident_type: &str,
        severity: Option<String>,
        interface_name: Option<String>,
        message: Option<String>,
    ) -> AppResult<MikrotikIncident> {
        let router = self
            .get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".to_string()))?;

        let normalized_type = {
            let v = incident_type.trim().to_ascii_lowercase();
            if v.is_empty() {
                return Err(AppError::Validation(
                    "incident_type is required".to_string(),
                ));
            }
            v
        };

        let normalized_severity = severity
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "warning".to_string());

        let severity_value = match normalized_severity.as_str() {
            "info" | "warning" | "critical" => normalized_severity.as_str(),
            _ => "warning",
        };

        let normalized_interface = interface_name.and_then(|v| {
            let s = v.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        });

        let actor_label: String = sqlx::query_scalar(
            r#"
            SELECT COALESCE(NULLIF(name, ''), email, id)
            FROM users
            WHERE id = $1
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| user_id.to_string());

        let message = message
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("Manual simulation triggered by {}", actor_label));
        let title = format!("Simulated {} incident", normalized_type);
        let now = Utc::now();

        self.upsert_incident(
            tenant_id,
            router_id,
            normalized_interface.as_deref(),
            &normalized_type,
            severity_value,
            &title,
            &message,
            None,
            None,
            now,
        )
        .await?;

        let dedup_key = MikrotikIncident::dedup_key(
            router_id,
            normalized_interface.as_deref(),
            &normalized_type,
        );
        let incident = sqlx::query_as::<_, MikrotikIncident>(
            r#"
            SELECT * FROM mikrotik_incidents
            WHERE tenant_id = $1 AND dedup_key = $2 AND resolved_at IS NULL
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&dedup_key)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.audit_service
            .log(
                Some(user_id),
                Some(tenant_id),
                "simulate",
                "mikrotik_incident",
                Some(&incident.id),
                Some(&format!(
                    "Simulated incident type {} on router {}",
                    normalized_type, router.name
                )),
                None,
            )
            .await;

        Ok(incident)
    }

    pub async fn list_logs(
        &self,
        tenant_id: &str,
        router_id: Option<String>,
        level: Option<String>,
        topic: Option<String>,
        q: Option<String>,
        month: Option<u32>,
        year: Option<i32>,
        page: u32,
        per_page: u32,
        include_total: bool,
    ) -> AppResult<PaginatedResponse<MikrotikLogEntry>> {
        validate_log_calendar_filters(month, year)?;
        let q = q.unwrap_or_default().trim().to_string();
        let offset = (page.saturating_sub(1)) * per_page;
        let month_i32 = month.map(|v| v as i32);

        let data: Vec<MikrotikLogEntry> = sqlx::query_as(mikrotik_log_list_sql())
            .bind(tenant_id)
            .bind(&router_id)
            .bind(&level)
            .bind(&topic)
            .bind(&q)
            .bind(month_i32)
            .bind(year)
            .bind(per_page as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?;

        let total = if include_total {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)
                FROM mikrotik_logs l
                WHERE l.tenant_id = $1
                  AND ($2::text IS NULL OR l.router_id = $2)
                  AND ($3::text IS NULL OR l.level = $3)
                  AND ($4::text IS NULL OR l.topics ILIKE '%' || $4 || '%')
                  AND ($5 = '' OR l.message ILIKE '%' || $5 || '%')
                  AND ($6::int4 IS NULL OR EXTRACT(MONTH FROM l.logged_at) = $6)
                  AND ($7::int4 IS NULL OR EXTRACT(YEAR FROM l.logged_at) = $7)
                "#,
            )
            .bind(tenant_id)
            .bind(&router_id)
            .bind(&level)
            .bind(&topic)
            .bind(&q)
            .bind(month_i32)
            .bind(year)
            .fetch_one(&self.pool)
            .await
            .map_err(AppError::Database)?
        } else {
            -1
        };

        Ok(PaginatedResponse {
            data,
            total,
            page,
            per_page,
        })
    }

    pub async fn sync_logs_for_router(
        &self,
        tenant_id: &str,
        router_id: &str,
        fetch_limit: u32,
    ) -> AppResult<MikrotikLogSyncResult> {
        let router = self
            .get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".to_string()))?;

        let mut raw_rows: Vec<(Option<String>, Option<String>, Option<String>, String)> =
            Vec::new();

        #[cfg(test)]
        if let Some(rows) = self.test_sync_rows_override() {
            raw_rows = rows;
        } else {
            let dev = self
                .connect_device(&router)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let cmd = CommandBuilder::new().command("/log/print").build();
            let mut rx = dev
                .send_command(cmd)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            while let Some(res) = rx.recv().await {
                let r = res.map_err(|e| AppError::Internal(e.to_string()))?;
                if let CommandResponse::Reply(reply) = r {
                    let message = reply
                        .attributes
                        .get("message")
                        .and_then(|v| v.clone())
                        .unwrap_or_default();
                    if message.trim().is_empty() {
                        continue;
                    }
                    raw_rows.push((
                        reply.attributes.get(".id").and_then(|v| v.clone()),
                        reply.attributes.get("time").and_then(|v| v.clone()),
                        reply.attributes.get("topics").and_then(|v| v.clone()),
                        message,
                    ));
                }
            }
        }

        #[cfg(not(test))]
        {
            let dev = self
                .connect_device(&router)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let cmd = CommandBuilder::new().command("/log/print").build();
            let mut rx = dev
                .send_command(cmd)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            while let Some(res) = rx.recv().await {
                let r = res.map_err(|e| AppError::Internal(e.to_string()))?;
                if let CommandResponse::Reply(reply) = r {
                    let message = reply
                        .attributes
                        .get("message")
                        .and_then(|v| v.clone())
                        .unwrap_or_default();
                    if message.trim().is_empty() {
                        continue;
                    }
                    raw_rows.push((
                        reply.attributes.get(".id").and_then(|v| v.clone()),
                        reply.attributes.get("time").and_then(|v| v.clone()),
                        reply.attributes.get("topics").and_then(|v| v.clone()),
                        message,
                    ));
                }
            }
        }

        let max_take = fetch_limit.max(1) as usize;
        if raw_rows.len() > max_take {
            let start = raw_rows.len() - max_take;
            raw_rows = raw_rows[start..].to_vec();
        }

        let now = Utc::now();
        let mut upserted = 0u32;

        for (router_log_id, router_time, topics, message) in raw_rows.iter() {
            let level = Self::log_level_from_topics(topics.as_deref());
            if let Some(rid) = router_log_id.as_ref() {
                sqlx::query(
                    r#"
                    INSERT INTO mikrotik_logs
                      (id, tenant_id, router_id, router_log_id, logged_at, router_time, topics, level, message, created_at, updated_at)
                    VALUES
                      ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                    ON CONFLICT (router_id, router_log_id) WHERE router_log_id IS NOT NULL
                    DO UPDATE SET
                      router_time = EXCLUDED.router_time,
                      topics = EXCLUDED.topics,
                      level = EXCLUDED.level,
                      message = EXCLUDED.message,
                      logged_at = EXCLUDED.logged_at,
                      updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(tenant_id)
                .bind(router_id)
                .bind(rid)
                .bind(now)
                .bind(router_time)
                .bind(topics)
                .bind(level)
                .bind(message)
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;
            } else {
                sqlx::query(
                    r#"
                    INSERT INTO mikrotik_logs
                      (id, tenant_id, router_id, router_log_id, logged_at, router_time, topics, level, message, created_at, updated_at)
                    VALUES
                      ($1, $2, $3, NULL, $4, $5, $6, $7, $8, $9, $10)
                    "#,
                )
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(tenant_id)
                .bind(router_id)
                .bind(now)
                .bind(router_time)
                .bind(topics)
                .bind(level)
                .bind(message)
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;
            }
            upserted += 1;
        }

        let retention_days = self
            .get_router_log_retention_days(tenant_id, router_id)
            .await?;
        let retention_cutoff = mikrotik_log_retention_cutoff(now, retention_days);

        #[cfg(test)]
        if self.test_sync_force_prune_error() {
            return Err(AppError::Database(sqlx::Error::Protocol(
                "MIKROTIK_TEST_SYNC_FORCE_PRUNE_ERROR".to_string(),
            )));
        }

        if let Some(retention_cutoff) = retention_cutoff {
            propagate_prune_query_result(
                sqlx::query(mikrotik_log_prune_sql())
                    .bind(tenant_id)
                    .bind(router_id)
                    .bind(retention_cutoff)
                    .execute(&self.pool)
                    .await,
            )?;
        }

        Ok(MikrotikLogSyncResult {
            seen: raw_rows.len() as u32,
            upserted,
        })
    }

    pub async fn get_router_log_retention(
        &self,
        tenant_id: &str,
        router_id: &str,
    ) -> AppResult<MikrotikLogRetentionSettings> {
        self.get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".to_string()))?;

        Ok(MikrotikLogRetentionSettings {
            router_id: router_id.to_string(),
            retention_days: self
                .get_router_log_retention_days(tenant_id, router_id)
                .await?,
        })
    }

    pub async fn update_router_log_retention_days(
        &self,
        tenant_id: &str,
        router_id: &str,
        retention_days: Option<i64>,
    ) -> AppResult<MikrotikLogRetentionSettings> {
        self.get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".to_string()))?;

        validate_router_log_retention_days(retention_days)?;
        let key = router_log_retention_setting_key(router_id);

        if let Some(retention_days) = retention_days {
            self.settings_service
                .upsert(
                    Some(tenant_id.to_string()),
                    crate::models::UpsertSettingDto {
                        key,
                        value: retention_days.to_string(),
                        description: Some(format!(
                            "MikroTik log retention in days for router {router_id}"
                        )),
                    },
                    None,
                    None,
                )
                .await?;
        } else if self
            .settings_service
            .get_value(Some(tenant_id), &key)
            .await?
            .is_some()
        {
            self.settings_service
                .delete(Some(tenant_id), &key, None, None)
                .await?;
        }

        self.get_router_log_retention(tenant_id, router_id).await
    }

    pub async fn clear_logs_for_router(
        &self,
        tenant_id: &str,
        router_id: &str,
    ) -> AppResult<MikrotikLogClearResult> {
        self.get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".to_string()))?;

        let result = sqlx::query(
            r#"
            DELETE FROM mikrotik_logs
            WHERE tenant_id = $1
              AND router_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(MikrotikLogClearResult {
            router_id: router_id.to_string(),
            deleted: result.rows_affected(),
        })
    }

    async fn get_router_log_retention_days(
        &self,
        tenant_id: &str,
        router_id: &str,
    ) -> AppResult<Option<i64>> {
        let key = router_log_retention_setting_key(router_id);
        let raw = self
            .settings_service
            .get_value(Some(tenant_id), &key)
            .await?;
        Ok(resolve_router_log_retention_days(raw.as_deref()))
    }

    pub async fn ack_alert(&self, tenant_id: &str, alert_id: &str, user_id: &str) -> AppResult<()> {
        let now = Utc::now();
        let target = sqlx::query_as::<_, MikrotikAlert>(
            r#"
            SELECT * FROM mikrotik_alerts
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(alert_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        sqlx::query(
            r#"
            UPDATE mikrotik_alerts
            SET status = 'ack',
                acked_at = $1,
                acked_by = $2,
                updated_at = $3
            WHERE id = $4 AND tenant_id = $5 AND resolved_at IS NULL
            "#,
        )
        .bind(now)
        .bind(user_id)
        .bind(now)
        .bind(alert_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if let Some(alert) = target {
            let dedup_key = MikrotikIncident::dedup_key(&alert.router_id, None, &alert.alert_type);
            let _ = sqlx::query(
                r#"
                UPDATE mikrotik_incidents
                SET status = 'ack',
                    acked_at = $1,
                    acked_by = $2,
                    updated_at = $3
                WHERE tenant_id = $4 AND dedup_key = $5 AND resolved_at IS NULL
                "#,
            )
            .bind(now)
            .bind(user_id)
            .bind(now)
            .bind(tenant_id)
            .bind(dedup_key)
            .execute(&self.pool)
            .await;
        }
        Ok(())
    }

    pub async fn ack_incident(
        &self,
        tenant_id: &str,
        incident_id: &str,
        user_id: &str,
    ) -> AppResult<()> {
        let now = Utc::now();
        let target = sqlx::query_as::<_, MikrotikIncident>(
            r#"
            SELECT * FROM mikrotik_incidents
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(incident_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        sqlx::query(
            r#"
            UPDATE mikrotik_incidents
            SET status = 'ack',
                acked_at = $1,
                acked_by = $2,
                updated_at = $3
            WHERE id = $4 AND tenant_id = $5 AND resolved_at IS NULL
            "#,
        )
        .bind(now)
        .bind(user_id)
        .bind(now)
        .bind(incident_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if let Some(incident) = target {
            let _ = sqlx::query(
                r#"
                UPDATE mikrotik_alerts
                SET status = 'ack',
                    acked_at = $1,
                    acked_by = $2,
                    updated_at = $3
                WHERE tenant_id = $4
                  AND router_id = $5
                  AND alert_type = $6
                  AND resolved_at IS NULL
                "#,
            )
            .bind(now)
            .bind(user_id)
            .bind(now)
            .bind(tenant_id)
            .bind(&incident.router_id)
            .bind(&incident.incident_type)
            .execute(&self.pool)
            .await;
        }
        Ok(())
    }

    pub async fn resolve_alert_by_id(
        &self,
        tenant_id: &str,
        alert_id: &str,
        user_id: &str,
    ) -> AppResult<()> {
        let now = Utc::now();

        let alert: Option<MikrotikAlert> = sqlx::query_as::<_, MikrotikAlert>(
            r#"
            SELECT * FROM mikrotik_alerts
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(alert_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let alert = alert.ok_or_else(|| AppError::NotFound("Alert not found".to_string()))?;

        sqlx::query(
            r#"
            UPDATE mikrotik_alerts
            SET status = 'resolved',
                resolved_at = $1,
                updated_at = $2
            WHERE id = $3 AND tenant_id = $4 AND resolved_at IS NULL
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(alert_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let _ = self
            .resolve_incident(tenant_id, &alert.router_id, None, &alert.alert_type)
            .await;

        self.audit_service
            .log(
                Some(user_id),
                Some(tenant_id),
                "resolve",
                "mikrotik_alert",
                Some(alert_id),
                Some(&format!(
                    "Resolved alert {} for router {} (type: {})",
                    alert.title, alert.router_id, alert.alert_type
                )),
                None,
            )
            .await;

        Ok(())
    }

    pub async fn resolve_incident_by_id(
        &self,
        tenant_id: &str,
        incident_id: &str,
        user_id: &str,
    ) -> AppResult<()> {
        let now = Utc::now();
        let incident: Option<MikrotikIncident> = sqlx::query_as::<_, MikrotikIncident>(
            r#"
            SELECT * FROM mikrotik_incidents
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(incident_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let incident =
            incident.ok_or_else(|| AppError::NotFound("Incident not found".to_string()))?;

        sqlx::query(
            r#"
            UPDATE mikrotik_incidents
            SET status = 'resolved',
                resolved_at = $1,
                updated_at = $2
            WHERE id = $3 AND tenant_id = $4 AND resolved_at IS NULL
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(incident_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let _ = sqlx::query(
            r#"
            UPDATE mikrotik_alerts
            SET status = 'resolved',
                resolved_at = $1,
                updated_at = $2
            WHERE tenant_id = $3
              AND router_id = $4
              AND alert_type = $5
              AND resolved_at IS NULL
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(tenant_id)
        .bind(&incident.router_id)
        .bind(&incident.incident_type)
        .execute(&self.pool)
        .await;

        self.audit_service
            .log(
                Some(user_id),
                Some(tenant_id),
                "resolve",
                "mikrotik_incident",
                Some(incident_id),
                Some(&format!(
                    "Resolved incident {} for router {} (type: {})",
                    incident.title, incident.router_id, incident.incident_type
                )),
                None,
            )
            .await;

        Ok(())
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
        Self::validate_router_coordinates(req.latitude, req.longitude)?;
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
            req.latitude,
            req.longitude,
        );

        sqlx::query(
            r#"
            INSERT INTO mikrotik_routers
            (id, tenant_id, name, host, port, username, password, use_tls, enabled,
             identity, ros_version, is_online, last_seen_at, latency_ms, last_error,
             maintenance_until, maintenance_reason, latitude, longitude,
             created_at, updated_at)
            VALUES
            ($1,$2,$3,$4,$5,$6,$7,$8,$9,
             $10,$11,$12,$13,$14,$15,
             $16,$17,$18,$19,
             $20,$21)
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
        .bind(req.maintenance_until)
        .bind(req.maintenance_reason)
        .bind(router.latitude)
        .bind(router.longitude)
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
        let latitude = req.latitude.or(existing.latitude);
        let longitude = req.longitude.or(existing.longitude);
        Self::validate_router_coordinates(latitude, longitude)?;
        // Maintenance is treated as an explicit admin action; allow clearing by passing null.
        // Our client always sends these fields on update.
        let maintenance_until = req.maintenance_until;
        let maintenance_reason = req.maintenance_reason;

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
              maintenance_until = $8,
              maintenance_reason = $9,
              latitude = $10,
              longitude = $11,
              updated_at = $12
            WHERE id = $13 AND tenant_id = $14
            "#,
        )
        .bind(&name)
        .bind(&host)
        .bind(port)
        .bind(&username)
        .bind(&password)
        .bind(use_tls)
        .bind(enabled)
        .bind(maintenance_until)
        .bind(maintenance_reason)
        .bind(latitude)
        .bind(longitude)
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
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2")
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
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2")
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
                WHERE router_id = $1
                  AND lower(trim(interface_name)) = lower(trim($2))
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
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2")
                .bind(router_id)
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?;

        if exists.is_none() {
            return Err(AppError::Forbidden("No access to router".into()));
        }

        #[cfg(feature = "postgres")]
        {
            // Fetch only the latest sample per interface directly in SQL.
            let rows = sqlx::query_as::<_, MikrotikInterfaceMetric>(
                r#"
                SELECT DISTINCT ON (interface_name) *
                FROM mikrotik_interface_metrics
                WHERE router_id = $1
                ORDER BY interface_name ASC, ts DESC
                "#,
            )
            .bind(router_id)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?;
            return Ok(rows);
        }

        #[cfg(not(feature = "postgres"))]
        {
            // Portable fallback for SQLite builds.
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
    }

    /// Live per-interface counters (best-effort) used for realtime UI polling.
    /// This does not persist anything to DB.
    pub async fn get_live_interface_counters(
        &self,
        tenant_id: &str,
        router_id: &str,
        names: Vec<String>,
    ) -> AppResult<Vec<MikrotikInterfaceCounter>> {
        if names.is_empty() {
            return Err(AppError::Validation("names is required".into()));
        }
        if names.len() > 12 {
            return Err(AppError::Validation("too many interfaces (max 12)".into()));
        }

        let router = self
            .get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".to_string()))?;

        let password = decrypt_secret_opt(router.password.as_str())?;
        let addr = format!("{}:{}", router.host, router.port);
        let dev = timeout(
            Duration::from_secs(5),
            MikrotikDevice::connect(addr, router.username.as_str(), password.as_deref()),
        )
        .await
        .map_err(|_| AppError::Internal("Connection timed out".into()))?
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Fetch all interface snapshots and filter. RouterOS doesn't reliably support
        // "IN" queries across all versions; we keep it portable.
        let snaps = self
            .fetch_interfaces_snapshot(&dev)
            .await
            .unwrap_or_default();
        let set: std::collections::HashSet<String> = names.into_iter().collect();

        let mut out: Vec<MikrotikInterfaceCounter> = vec![];
        for s in snaps {
            if !set.contains(&s.name) {
                continue;
            }
            out.push(MikrotikInterfaceCounter {
                name: s.name,
                running: s.running,
                disabled: s.disabled,
                rx_byte: s.rx_byte,
                tx_byte: s.tx_byte,
            });
        }

        Ok(out)
    }

    /// Fetch a "live" snapshot from the router (best-effort).
    ///
    /// This is used by the admin detail UI to show richer data without forcing
    /// the background poller to store huge payloads.
    pub async fn get_snapshot(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<MikrotikRouterSnapshot> {
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
        let interfaces = self
            .fetch_interfaces_snapshot(&dev)
            .await
            .unwrap_or_default();

        // IP addresses
        let ip_addresses = self
            .fetch_ip_addresses_snapshot(&dev)
            .await
            .unwrap_or_default();

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

    pub async fn test_connection(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<MikrotikTestResult> {
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

            let cleanup_interval_secs = std::env::var("MIKROTIK_METRICS_CLEANUP_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|v| *v >= 60 && *v <= 86400)
                .unwrap_or(3600);

            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            let mut last_cleanup = std::time::Instant::now()
                .checked_sub(Duration::from_secs(cleanup_interval_secs))
                .unwrap_or_else(std::time::Instant::now);
            loop {
                interval.tick().await;
                if let Err(e) = self.poll_once().await {
                    warn!("[MikrotikPoller] Poll failed: {}", e);
                }
                if last_cleanup.elapsed().as_secs() >= cleanup_interval_secs {
                    if let Err(e) = self.cleanup_old_metrics().await {
                        warn!("[MikrotikPoller] Metrics cleanup failed: {}", e);
                    }
                    last_cleanup = std::time::Instant::now();
                }
            }
        });
    }

    async fn metrics_retention_days(&self) -> i64 {
        if let Ok(Some(v)) = self
            .settings_service
            .get_value(None, "mikrotik_metrics_retention_days")
            .await
        {
            if let Ok(days) = v.trim().parse::<i64>() {
                // 0 means disabled cleanup.
                return days.clamp(0, 3650);
            }
        }
        14
    }

    async fn cleanup_old_metrics(&self) -> AppResult<()> {
        let retention_days = self.metrics_retention_days().await;
        if retention_days <= 0 {
            return Ok(());
        }

        let cutoff = Utc::now() - ChronoDuration::days(retention_days);

        #[cfg(feature = "postgres")]
        async fn prune_table(
            pool: &DbPool,
            table: &str,
            cutoff: DateTime<Utc>,
            batch_size: i64,
        ) -> Result<u64, sqlx::Error> {
            let mut total = 0u64;
            loop {
                let sql = format!(
                    r#"
                    DELETE FROM {table}
                    WHERE ctid IN (
                        SELECT ctid FROM {table}
                        WHERE ts < $1
                        LIMIT $2
                    )
                    "#
                );

                let affected = sqlx::query(&sql)
                    .bind(cutoff)
                    .bind(batch_size)
                    .execute(pool)
                    .await?
                    .rows_affected();

                total = total.saturating_add(affected);
                if affected == 0 {
                    break;
                }
            }
            Ok(total)
        }

        #[cfg(feature = "sqlite")]
        async fn prune_table(
            pool: &DbPool,
            table: &str,
            cutoff: DateTime<Utc>,
            batch_size: i64,
        ) -> Result<u64, sqlx::Error> {
            let mut total = 0u64;
            loop {
                let sql = format!(
                    r#"
                    DELETE FROM {table}
                    WHERE rowid IN (
                        SELECT rowid FROM {table}
                        WHERE ts < $1
                        LIMIT $2
                    )
                    "#
                );

                let affected = sqlx::query(&sql)
                    .bind(cutoff)
                    .bind(batch_size)
                    .execute(pool)
                    .await?
                    .rows_affected();

                total = total.saturating_add(affected);
                if affected == 0 {
                    break;
                }
            }
            Ok(total)
        }

        let batch_size = 5_000i64;
        let deleted_iface =
            prune_table(&self.pool, "mikrotik_interface_metrics", cutoff, batch_size)
                .await
                .map_err(AppError::Database)?;
        let deleted_router = prune_table(&self.pool, "mikrotik_router_metrics", cutoff, batch_size)
            .await
            .map_err(AppError::Database)?;

        if deleted_iface > 0 || deleted_router > 0 {
            info!(
                "[MikrotikPoller] Metrics cleanup done: deleted interface={} router={} (retention={}d)",
                deleted_iface, deleted_router, retention_days
            );
        }

        Ok(())
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

        let mut tracked_by_tenant: HashMap<String, HashMap<String, HashSet<String>>> =
            HashMap::new();

        for router in routers {
            let tenant_id = router.tenant_id.clone();
            if !tracked_by_tenant.contains_key(&tenant_id) {
                let tracked = self
                    .wallboard_tracked_interfaces_by_router_cached(&tenant_id)
                    .await;
                tracked_by_tenant.insert(tenant_id.clone(), tracked);
            }

            let tracked_for_router = tracked_by_tenant
                .get(&tenant_id)
                .and_then(|m| m.get(&router.id).cloned());

            let _ = self.poll_router(router, tracked_for_router).await;
        }

        for tenant_id in tracked_by_tenant.keys() {
            let _ = self.auto_escalate_incidents(tenant_id).await;
        }
        Ok(())
    }

    async fn auto_escalate_incidents(&self, tenant_id: &str) -> AppResult<i64> {
        let enabled = match self
            .settings_service
            .get_value(Some(tenant_id), "mikrotik_incident_auto_escalation_enabled")
            .await
        {
            Ok(Some(v)) => {
                let x = v.trim().to_ascii_lowercase();
                x == "1" || x == "true" || x == "yes" || x == "on"
            }
            _ => false,
        };
        if !enabled {
            return Ok(0);
        }

        let threshold_minutes = match self
            .settings_service
            .get_value(Some(tenant_id), "mikrotik_incident_escalation_minutes")
            .await
        {
            Ok(Some(v)) => v.trim().parse::<i64>().unwrap_or(60),
            _ => 60,
        }
        .clamp(5, 10_080);
        let threshold = ChronoDuration::minutes(threshold_minutes);
        let now = Utc::now();

        let candidates: Vec<MikrotikIncident> = sqlx::query_as(
            r#"
            SELECT *
            FROM mikrotik_incidents
            WHERE tenant_id = $1
              AND resolved_at IS NULL
              AND acked_at IS NULL
              AND status IN ('open', 'in_progress')
              AND severity <> 'critical'
              AND first_seen_at <= $2
            ORDER BY first_seen_at ASC
            LIMIT 200
            "#,
        )
        .bind(tenant_id)
        .bind(now - threshold)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut escalated_count: i64 = 0;
        for incident in candidates {
            let affected = sqlx::query(
                r#"
                UPDATE mikrotik_incidents
                SET severity = 'critical',
                    updated_at = $1
                WHERE id = $2
                  AND tenant_id = $3
                  AND severity <> 'critical'
                  AND acked_at IS NULL
                  AND resolved_at IS NULL
                "#,
            )
            .bind(now)
            .bind(&incident.id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?
            .rows_affected();

            if affected == 0 {
                continue;
            }
            escalated_count += affected as i64;

            self.notify_tenant(
                tenant_id,
                "Incident escalated",
                format!(
                    "{} has exceeded {} minutes without acknowledgement.",
                    incident.title, threshold_minutes
                ),
                Some(format!("/admin/network/incidents?incident={}", incident.id)),
                "error",
            )
            .await;

            self.audit_service
                .log(
                    None,
                    Some(tenant_id),
                    "escalate",
                    "mikrotik_incident",
                    Some(&incident.id),
                    Some(&format!(
                        "Auto escalated incident {} after {} minutes",
                        incident.title, threshold_minutes
                    )),
                    None,
                )
                .await;
        }

        Ok(escalated_count)
    }

    async fn poll_router(
        &self,
        router: MikrotikRouter,
        tracked_ifaces: Option<std::collections::HashSet<String>>,
    ) -> AppResult<()> {
        let started = Instant::now();
        let prev_online = router.is_online;
        let tenant_id = router.tenant_id.clone();

        let probe = self.connect_and_probe(&router).await;
        let now = Utc::now();
        let latency_ms = Some(started.elapsed().as_millis().min(i32::MAX as u128) as i32);

        let in_maintenance = router.maintenance_until.map(|u| u > now).unwrap_or(false);

        match probe {
            Ok((identity, version)) => {
                // Basic resource snapshot
                let metric = self
                    .fetch_resource_metric(&router)
                    .await
                    .unwrap_or_else(|_| {
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
                if let Ok((rx_bps, tx_bps)) = self
                    .poll_interface_metrics(&router, now, tracked_ifaces.as_ref())
                    .await
                {
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

                // Optional background log ingestion so admins can inspect router logs without manual sync.
                let log_sync_enabled = std::env::var("MIKROTIK_LOG_SYNC_ENABLED")
                    .ok()
                    .map(|v| {
                        let x = v.trim().to_ascii_lowercase();
                        x == "1" || x == "true" || x == "yes" || x == "on"
                    })
                    .unwrap_or(true);
                if log_sync_enabled {
                    let log_fetch_limit = std::env::var("MIKROTIK_LOG_SYNC_FETCH_LIMIT")
                        .ok()
                        .and_then(|v| v.parse::<u32>().ok())
                        .filter(|v| *v >= 50 && *v <= 2000)
                        .unwrap_or(300);
                    if let Err(e) = self
                        .sync_logs_for_router(&tenant_id, &router.id, log_fetch_limit)
                        .await
                    {
                        warn!(
                            "[MikrotikPoller] Log sync failed for {} ({}): {}",
                            router.name, router.host, e
                        );
                    }
                }

                // Resolve "offline" incident and evaluate CPU/latency incidents.
                if in_maintenance {
                    let _ = self.resolve_all_router_alerts(&tenant_id, &router.id).await;
                } else {
                    let _ = self.resolve_alert(&tenant_id, &router.id, "offline").await;
                    let _ = self
                        .eval_cpu_alert(&tenant_id, &router, metric.cpu_load, now)
                        .await;
                    let _ = self
                        .eval_latency_alert(&tenant_id, &router, latency_ms, now)
                        .await;
                }

                if !prev_online {
                    let offline_for_secs = {
                        let base = router.last_seen_at.unwrap_or(router.created_at);
                        (now - base).num_seconds().max(0)
                    };
                    let recovered_after_secs = std::env::var("MIKROTIK_RECOVERED_AFTER_SECS")
                        .ok()
                        .and_then(|v| v.parse::<i64>().ok())
                        .unwrap_or(300)
                        .clamp(30, 24 * 3600);
                    let (title, message) = if offline_for_secs >= recovered_after_secs {
                        (
                            "Router recovered",
                            format!(
                                "{} recovered after {}s offline.",
                                router.name, offline_for_secs
                            ),
                        )
                    } else {
                        ("Router online", format!("{} is back online.", router.name))
                    };
                    self.notify_router_status_change(
                        &tenant_id,
                        title,
                        message,
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
                            Some(&format!(
                                "{} is back online (offline {}s)",
                                router.name, offline_for_secs
                            )),
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

                if in_maintenance {
                    let _ = self.resolve_all_router_alerts(&tenant_id, &router.id).await;
                } else {
                    let th = self.get_thresholds(&tenant_id).await;
                    if !th.enabled {
                        let _ = self.resolve_all_router_alerts(&tenant_id, &router.id).await;
                    } else {
                        // Only open an incident after the router has been unreachable for a while (anti-flap).
                        let base = router.last_seen_at.unwrap_or(router.created_at);
                        let offline_for_secs = (now - base).num_seconds().max(0);
                        if offline_for_secs >= th.offline_after_secs {
                            // Create/refresh "offline" incident. CPU/latency becomes unknown when offline, so resolve them.
                            let created = self
                                .upsert_alert(
                                    &tenant_id,
                                    &router,
                                    "offline",
                                    "critical",
                                    "Router offline",
                                    format!(
                                        "{} is unreachable ({}s).",
                                        router.name, offline_for_secs
                                    ),
                                    Some(offline_for_secs as f64),
                                    Some(th.offline_after_secs.max(0) as f64),
                                    now,
                                )
                                .await
                                .unwrap_or(false);
                            let _ = created;
                        }
                    }
                    let _ = self.resolve_alert(&tenant_id, &router.id, "cpu").await;
                    let _ = self.resolve_alert(&tenant_id, &router.id, "latency").await;
                }

                if prev_online {
                    self.notify_router_status_change(
                        &tenant_id,
                        "Router down",
                        format!("{} became unreachable: {}", router.name, msg),
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

    async fn eval_cpu_alert(
        &self,
        tenant_id: &str,
        router: &MikrotikRouter,
        cpu_load: Option<i32>,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        let th = self.get_thresholds(tenant_id).await;
        if !th.enabled {
            let _ = self.resolve_all_router_alerts(tenant_id, &router.id).await;
            return Ok(());
        }

        if let Some(cpu) = cpu_load {
            if cpu >= th.cpu_risk {
                let created = self
                    .upsert_alert(
                        tenant_id,
                        router,
                        "cpu",
                        if cpu >= th.cpu_hot {
                            "critical"
                        } else {
                            "warning"
                        },
                        "High CPU",
                        format!(
                            "{} CPU is {}% (threshold: {}%).",
                            router.name, cpu, th.cpu_risk
                        ),
                        Some(cpu as f64),
                        Some(th.cpu_risk as f64),
                        now,
                    )
                    .await?;

                if created {
                    self.notify_tenant(
                        tenant_id,
                        "High CPU",
                        format!("{} CPU is {}%.", router.name, cpu),
                        Some(format!("/admin/network/routers/{}", router.id)),
                        "warning",
                    )
                    .await;

                    self.audit_service
                        .log(
                            None,
                            Some(tenant_id),
                            "alert_cpu",
                            "mikrotik_alert",
                            Some(&router.id),
                            Some(&format!("CPU alert: {}% on {}", cpu, router.name)),
                            None,
                        )
                        .await;
                }

                return Ok(());
            }
        }

        let _ = self.resolve_alert(tenant_id, &router.id, "cpu").await;
        Ok(())
    }

    async fn eval_latency_alert(
        &self,
        tenant_id: &str,
        router: &MikrotikRouter,
        latency_ms: Option<i32>,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        let th = self.get_thresholds(tenant_id).await;
        if !th.enabled {
            let _ = self.resolve_all_router_alerts(tenant_id, &router.id).await;
            return Ok(());
        }

        if let Some(lat) = latency_ms {
            if lat >= th.latency_risk_ms {
                let created = self
                    .upsert_alert(
                        tenant_id,
                        router,
                        "latency",
                        if lat >= th.latency_hot_ms {
                            "critical"
                        } else {
                            "warning"
                        },
                        "High latency",
                        format!(
                            "{} latency is {}ms (threshold: {}ms).",
                            router.name, lat, th.latency_risk_ms
                        ),
                        Some(lat as f64),
                        Some(th.latency_risk_ms as f64),
                        now,
                    )
                    .await?;

                if created {
                    self.notify_tenant(
                        tenant_id,
                        "High latency",
                        format!("{} latency is {}ms.", router.name, lat),
                        Some(format!("/admin/network/routers/{}", router.id)),
                        "warning",
                    )
                    .await;

                    self.audit_service
                        .log(
                            None,
                            Some(tenant_id),
                            "alert_latency",
                            "mikrotik_alert",
                            Some(&router.id),
                            Some(&format!("Latency alert: {}ms on {}", lat, router.name)),
                            None,
                        )
                        .await;
                }

                return Ok(());
            }
        }

        let _ = self.resolve_alert(tenant_id, &router.id, "latency").await;
        Ok(())
    }

    async fn upsert_alert(
        &self,
        tenant_id: &str,
        router: &MikrotikRouter,
        alert_type: &str,
        severity: &str,
        title: &str,
        message: String,
        value_num: Option<f64>,
        threshold_num: Option<f64>,
        now: DateTime<Utc>,
    ) -> AppResult<bool> {
        if self
            .should_suppress_correlated_incident(tenant_id, &router.id, alert_type)
            .await?
        {
            return Ok(false);
        }

        // returns true if created new incident
        let existing: Option<String> = sqlx::query_scalar(
            r#"
            SELECT id FROM mikrotik_alerts
            WHERE tenant_id = $1 AND router_id = $2 AND alert_type = $3 AND resolved_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(&router.id)
        .bind(alert_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if let Some(id) = existing {
            sqlx::query(
                r#"
                UPDATE mikrotik_alerts
                SET severity = $1,
                    title = $2,
                    message = $3,
                    value_num = $4,
                    threshold_num = $5,
                    last_seen_at = $6,
                    updated_at = $7
                WHERE id = $8
                "#,
            )
            .bind(severity)
            .bind(title)
            .bind(&message)
            .bind(value_num)
            .bind(threshold_num)
            .bind(now)
            .bind(now)
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
            self.upsert_incident(
                tenant_id,
                &router.id,
                None,
                alert_type,
                severity,
                title,
                &message,
                value_num,
                threshold_num,
                now,
            )
            .await?;
            return Ok(false);
        }

        let mut alert = MikrotikAlert::new(
            tenant_id.to_string(),
            router.id.clone(),
            alert_type.to_string(),
            severity.to_string(),
            title.to_string(),
            message,
            value_num,
            threshold_num,
        );
        alert.triggered_at = now;
        alert.last_seen_at = now;
        alert.created_at = now;
        alert.updated_at = now;

        sqlx::query(
            r#"
            INSERT INTO mikrotik_alerts
            (id, tenant_id, router_id, alert_type, severity, status, title, message,
             value_num, threshold_num, triggered_at, last_seen_at, resolved_at,
             acked_at, acked_by, created_at, updated_at)
            VALUES
            ($1,$2,$3,$4,$5,$6,$7,$8,
             $9,$10,$11,$12,$13,
             $14,$15,$16,$17)
            "#,
        )
        .bind(&alert.id)
        .bind(&alert.tenant_id)
        .bind(&alert.router_id)
        .bind(&alert.alert_type)
        .bind(&alert.severity)
        .bind(&alert.status)
        .bind(&alert.title)
        .bind(&alert.message)
        .bind(alert.value_num)
        .bind(alert.threshold_num)
        .bind(alert.triggered_at)
        .bind(alert.last_seen_at)
        .bind(alert.resolved_at)
        .bind(alert.acked_at)
        .bind(&alert.acked_by)
        .bind(alert.created_at)
        .bind(alert.updated_at)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.upsert_incident(
            tenant_id,
            &router.id,
            None,
            alert_type,
            severity,
            title,
            &alert.message,
            value_num,
            threshold_num,
            now,
        )
        .await?;

        Ok(true)
    }

    async fn should_suppress_correlated_incident(
        &self,
        tenant_id: &str,
        router_id: &str,
        incident_type: &str,
    ) -> AppResult<bool> {
        let normalized = incident_type.trim().to_ascii_lowercase();
        if normalized.is_empty() || normalized == "offline" {
            return Ok(false);
        }

        let correlation_enabled = match self
            .settings_service
            .get_value(Some(tenant_id), "mikrotik_incident_correlation_enabled")
            .await
        {
            Ok(Some(v)) => {
                let x = v.trim().to_ascii_lowercase();
                x == "1" || x == "true" || x == "yes" || x == "on"
            }
            _ => true,
        };
        if !correlation_enabled {
            return Ok(false);
        }

        let offline_open: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1 FROM mikrotik_incidents
              WHERE tenant_id = $1
                AND router_id = $2
                AND incident_type = 'offline'
                AND resolved_at IS NULL
            )
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if !offline_open {
            return Ok(false);
        }

        let had_open_child: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1 FROM mikrotik_incidents
              WHERE tenant_id = $1
                AND router_id = $2
                AND incident_type = $3
                AND resolved_at IS NULL
            )
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(&normalized)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if had_open_child {
            let _ = self.resolve_alert(tenant_id, router_id, &normalized).await;
            self.audit_service
                .log(
                    None,
                    Some(tenant_id),
                    "correlate_suppress",
                    "mikrotik_incident",
                    Some(router_id),
                    Some(&format!(
                        "Suppressed {} incident because offline root-cause is active",
                        normalized
                    )),
                    None,
                )
                .await;
        }

        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    async fn upsert_incident(
        &self,
        tenant_id: &str,
        router_id: &str,
        interface_name: Option<&str>,
        incident_type: &str,
        severity: &str,
        title: &str,
        message: &str,
        value_num: Option<f64>,
        threshold_num: Option<f64>,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        let dedup_key = MikrotikIncident::dedup_key(router_id, interface_name, incident_type);
        let existing: Option<String> = sqlx::query_scalar(
            r#"
            SELECT id FROM mikrotik_incidents
            WHERE tenant_id = $1 AND dedup_key = $2 AND resolved_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(&dedup_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if let Some(id) = existing {
            sqlx::query(
                r#"
                UPDATE mikrotik_incidents
                SET severity = $1,
                    title = $2,
                    message = $3,
                    value_num = $4,
                    threshold_num = $5,
                    last_seen_at = $6,
                    updated_at = $7
                WHERE id = $8
                "#,
            )
            .bind(severity)
            .bind(title)
            .bind(message)
            .bind(value_num)
            .bind(threshold_num)
            .bind(now)
            .bind(now)
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
            return Ok(());
        }

        let mut incident = MikrotikIncident::new(
            tenant_id.to_string(),
            router_id.to_string(),
            interface_name.map(|s| s.to_string()),
            incident_type.to_string(),
            severity.to_string(),
            title.to_string(),
            message.to_string(),
            value_num,
            threshold_num,
        );
        incident.first_seen_at = now;
        incident.last_seen_at = now;
        incident.created_at = now;
        incident.updated_at = now;

        sqlx::query(
            r#"
            INSERT INTO mikrotik_incidents
            (id, tenant_id, router_id, interface_name, incident_type, dedup_key, severity, status,
             title, message, value_num, threshold_num, first_seen_at, last_seen_at, resolved_at,
             acked_at, acked_by, owner_user_id, notes, created_at, updated_at)
            VALUES
            ($1,$2,$3,$4,$5,$6,$7,$8,
             $9,$10,$11,$12,$13,$14,$15,
             $16,$17,$18,$19,$20,$21)
            "#,
        )
        .bind(&incident.id)
        .bind(&incident.tenant_id)
        .bind(&incident.router_id)
        .bind(&incident.interface_name)
        .bind(&incident.incident_type)
        .bind(&incident.dedup_key)
        .bind(&incident.severity)
        .bind(&incident.status)
        .bind(&incident.title)
        .bind(&incident.message)
        .bind(incident.value_num)
        .bind(incident.threshold_num)
        .bind(incident.first_seen_at)
        .bind(incident.last_seen_at)
        .bind(incident.resolved_at)
        .bind(incident.acked_at)
        .bind(&incident.acked_by)
        .bind(&incident.owner_user_id)
        .bind(&incident.notes)
        .bind(incident.created_at)
        .bind(incident.updated_at)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn resolve_alert(
        &self,
        tenant_id: &str,
        router_id: &str,
        alert_type: &str,
    ) -> AppResult<()> {
        let now = Utc::now();
        let _ = sqlx::query(
            r#"
            UPDATE mikrotik_alerts
            SET status = 'resolved',
                resolved_at = $1,
                updated_at = $2
            WHERE tenant_id = $3 AND router_id = $4 AND alert_type = $5 AND resolved_at IS NULL
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(tenant_id)
        .bind(router_id)
        .bind(alert_type)
        .execute(&self.pool)
        .await;
        let _ = self
            .resolve_incident(tenant_id, router_id, None, alert_type)
            .await;
        Ok(())
    }

    async fn resolve_incident(
        &self,
        tenant_id: &str,
        router_id: &str,
        interface_name: Option<&str>,
        incident_type: &str,
    ) -> AppResult<()> {
        let now = Utc::now();
        let dedup_key = MikrotikIncident::dedup_key(router_id, interface_name, incident_type);
        let _ = sqlx::query(
            r#"
            UPDATE mikrotik_incidents
            SET status = 'resolved',
                resolved_at = $1,
                updated_at = $2
            WHERE tenant_id = $3 AND dedup_key = $4 AND resolved_at IS NULL
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(tenant_id)
        .bind(dedup_key)
        .execute(&self.pool)
        .await;
        Ok(())
    }

    async fn resolve_all_router_alerts(&self, tenant_id: &str, router_id: &str) -> AppResult<()> {
        let _ = self.resolve_alert(tenant_id, router_id, "offline").await;
        let _ = self.resolve_alert(tenant_id, router_id, "cpu").await;
        let _ = self.resolve_alert(tenant_id, router_id, "latency").await;
        Ok(())
    }

    async fn get_thresholds(&self, tenant_id: &str) -> Thresholds {
        async fn get_i32(svc: &SettingsService, tenant_id: &str, key: &str, default: i32) -> i32 {
            match svc.get_value(Some(tenant_id), key).await {
                Ok(Some(v)) => v.trim().parse::<i32>().ok().unwrap_or(default),
                _ => default,
            }
        }

        async fn get_bool(
            svc: &SettingsService,
            tenant_id: &str,
            key: &str,
            default: bool,
        ) -> bool {
            match svc.get_value(Some(tenant_id), key).await {
                Ok(Some(v)) => matches!(
                    v.trim().to_lowercase().as_str(),
                    "true" | "1" | "yes" | "on"
                ),
                _ => default,
            }
        }

        async fn get_i64(svc: &SettingsService, tenant_id: &str, key: &str, default: i64) -> i64 {
            match svc.get_value(Some(tenant_id), key).await {
                Ok(Some(v)) => v.trim().parse::<i64>().ok().unwrap_or(default),
                _ => default,
            }
        }

        let enabled = get_bool(
            &self.settings_service,
            tenant_id,
            "mikrotik_alerting_enabled",
            true,
        )
        .await;
        let cpu_risk = get_i32(
            &self.settings_service,
            tenant_id,
            "mikrotik_alert_cpu_risk",
            CPU_RISK,
        )
        .await;
        let cpu_hot = get_i32(
            &self.settings_service,
            tenant_id,
            "mikrotik_alert_cpu_hot",
            CPU_HOT,
        )
        .await;
        let latency_risk_ms = get_i32(
            &self.settings_service,
            tenant_id,
            "mikrotik_alert_latency_risk_ms",
            LATENCY_RISK_MS,
        )
        .await;
        let latency_hot_ms = get_i32(
            &self.settings_service,
            tenant_id,
            "mikrotik_alert_latency_hot_ms",
            LATENCY_HOT_MS,
        )
        .await;

        let offline_after_secs = get_i64(
            &self.settings_service,
            tenant_id,
            "mikrotik_alert_offline_after_secs",
            OFFLINE_AFTER_SECS,
        )
        .await;

        Thresholds {
            enabled,
            cpu_risk,
            cpu_hot: cpu_hot.max(cpu_risk),
            latency_risk_ms,
            latency_hot_ms: latency_hot_ms.max(latency_risk_ms),
            offline_after_secs: offline_after_secs.clamp(0, 24 * 3600),
        }
    }

    async fn poll_interface_metrics(
        &self,
        router: &MikrotikRouter,
        ts: DateTime<Utc>,
        tracked_ifaces: Option<&std::collections::HashSet<String>>,
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

        let snapshot_interfaces = self.fetch_interfaces_snapshot(&dev).await?;
        let untracked_max = std::env::var("MIKROTIK_UNTRACKED_IFACE_MAX")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v >= 1 && *v <= 256)
            .unwrap_or(24);

        let priority_max = std::env::var("MIKROTIK_PRIORITY_IFACE_MAX")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v >= 1 && *v <= 256)
            .unwrap_or(16);

        let interfaces: Vec<MikrotikInterfaceSnapshot> = match tracked_ifaces {
            // Persist only interfaces selected on wallboard when a tracked list exists.
            Some(allowed) if !allowed.is_empty() => {
                let normalized_allowed: std::collections::HashSet<String> = allowed
                    .iter()
                    .map(|name| Self::normalize_interface_name(name))
                    .filter(|name| !name.is_empty())
                    .collect();
                let mut selected: Vec<MikrotikInterfaceSnapshot> = Vec::new();
                let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

                for interface in snapshot_interfaces.iter() {
                    let normalized_name = Self::normalize_interface_name(&interface.name);
                    if normalized_allowed.contains(&normalized_name) && seen.insert(normalized_name)
                    {
                        selected.push(interface.clone());
                    }
                }

                for interface in snapshot_interfaces.iter() {
                    let normalized_name = Self::normalize_interface_name(&interface.name);
                    if seen.contains(&normalized_name) {
                        continue;
                    }
                    if !Self::is_priority_physical_interface(interface) {
                        continue;
                    }
                    if selected.len() >= normalized_allowed.len().saturating_add(priority_max) {
                        break;
                    }
                    seen.insert(normalized_name);
                    selected.push(interface.clone());
                }

                selected
            }
            // Fallback: if no tracked list is configured, still persist a bounded set so
            // historical charts are available instead of staying empty forever.
            _ => snapshot_interfaces
                .into_iter()
                .filter(Self::is_active_interface)
                .take(untracked_max)
                .collect(),
        };

        if interfaces.is_empty() {
            return Ok((None, None));
        }

        // Fetch last metrics per interface in one shot.
        #[cfg(feature = "postgres")]
        let mut prev_rows = {
            let interface_names: Vec<String> = interfaces
                .iter()
                .map(|i| i.name.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();

            if interface_names.is_empty() {
                Vec::<PrevIfaceRow>::new()
            } else {
                sqlx::query_as::<_, PrevIfaceRow>(
                    r#"
                    WITH names AS (
                        SELECT DISTINCT unnest($2::text[]) AS interface_name
                    )
                    SELECT m.interface_name, m.ts, m.rx_byte, m.tx_byte
                    FROM names n
                    JOIN LATERAL (
                        SELECT interface_name, ts, rx_byte, tx_byte
                        FROM mikrotik_interface_metrics
                        WHERE router_id = $1
                          AND interface_name = n.interface_name
                        ORDER BY ts DESC
                        LIMIT 1
                    ) m ON true
                    "#,
                )
                .bind(&router.id)
                .bind(&interface_names)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?
            }
        };

        #[cfg(not(feature = "postgres"))]
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

        let mut prev_map: std::collections::HashMap<String, PrevIfaceRow> =
            std::collections::HashMap::new();
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

            if let (Some(prev_row), Some(cur_rx), Some(prev_rx)) =
                (prev, it.rx_byte, prev.and_then(|p| p.rx_byte))
            {
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

            if let (Some(prev_row), Some(cur_tx), Some(prev_tx)) =
                (prev, it.tx_byte, prev.and_then(|p| p.tx_byte))
            {
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

    async fn wallboard_tracked_interfaces_by_router_cached(
        &self,
        tenant_id: &str,
    ) -> HashMap<String, HashSet<String>> {
        let now = Instant::now();
        if let Ok(cache) = self.wallboard_track_cache.read() {
            if let Some((loaded_at, data)) = cache.get(tenant_id) {
                if now.duration_since(*loaded_at).as_secs() < WALLBOARD_TRACK_CACHE_TTL_SECS {
                    return data.clone();
                }
            }
        }

        let fresh = self.wallboard_tracked_interfaces_by_router(tenant_id).await;
        if let Ok(mut cache) = self.wallboard_track_cache.write() {
            cache.insert(tenant_id.to_string(), (now, fresh.clone()));
        }
        fresh
    }

    async fn wallboard_tracked_interfaces_by_router(
        &self,
        tenant_id: &str,
    ) -> HashMap<String, HashSet<String>> {
        let raw = match self
            .settings_service
            .get_value_fallback(Some(tenant_id), WALLBOARD_SLOTS_SETTING_KEY)
            .await
        {
            Ok(v) => v,
            Err(_) => return HashMap::new(),
        };

        let mut out: HashMap<String, HashSet<String>> = HashMap::new();
        let Some(value) = raw else {
            return out;
        };

        let parsed: serde_json::Value = match serde_json::from_str(&value) {
            Ok(v) => v,
            Err(_) => return out,
        };

        let Some(items) = parsed.as_array() else {
            return out;
        };

        for it in items {
            if it.is_null() {
                continue;
            }

            // Back-compat with old format: ["router-id-1", ...] => default iface ether1
            if let Some(router_id) = it.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                out.entry(router_id.to_string())
                    .or_default()
                    .insert(Self::normalize_interface_name("ether1"));
                continue;
            }

            let Some(obj) = it.as_object() else {
                continue;
            };

            let router_id = obj
                .get("routerId")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let iface = obj
                .get("iface")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty());

            if let (Some(router_id), Some(iface)) = (router_id, iface) {
                let normalized_iface = Self::normalize_interface_name(iface);
                if normalized_iface.is_empty() {
                    continue;
                }
                out.entry(router_id.to_string())
                    .or_default()
                    .insert(normalized_iface);
            }
        }

        out
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
        let cmd = CommandBuilder::new().command("/interface/print").build();
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
                let mac_address = reply
                    .attributes
                    .get("mac-address")
                    .and_then(|v| v.clone())
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| {
                        reply
                            .attributes
                            .get("actual-mac-address")
                            .and_then(|v| v.clone())
                            .filter(|s| !s.trim().is_empty())
                    });

                out.push(MikrotikInterfaceSnapshot {
                    name,
                    interface_type: reply.attributes.get("type").and_then(|v| v.clone()),
                    running,
                    disabled,
                    mtu,
                    mac_address,
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
        let cmd = CommandBuilder::new().command("/ip/address/print").build();
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

    async fn fetch_health_snapshot(
        &self,
        dev: &MikrotikDevice,
    ) -> Result<MikrotikHealthSnapshot, anyhow::Error> {
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

    fn parse_bool_opt(v: Option<&String>) -> Option<bool> {
        v.and_then(|s| {
            let t = s.trim().to_lowercase();
            if t.is_empty() {
                None
            } else if matches!(t.as_str(), "true" | "yes" | "1" | "on") {
                Some(true)
            } else if matches!(t.as_str(), "false" | "no" | "0" | "off") {
                Some(false)
            } else {
                None
            }
        })
    }

    async fn connect_device(
        &self,
        router: &MikrotikRouter,
    ) -> Result<MikrotikDevice, anyhow::Error> {
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

        Ok(dev)
    }

    fn normalize_optional_text(value: Option<String>) -> Option<String> {
        value.and_then(|v| {
            let trimmed = v.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    }

    async fn find_router_ppp_profile_id_by_name(
        &self,
        dev: &MikrotikDevice,
        profile_name: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let cmd = CommandBuilder::new()
            .command("/ppp/profile/print")
            .attribute("detail", Some(""))
            .build();
        let mut rx = dev.send_command(cmd).await?;
        let mut rows = Vec::new();
        while let Some(res) = rx.recv().await {
            let response = res?;
            if let CommandResponse::Reply(reply) = response {
                rows.push((
                    reply.attributes.get(".id").and_then(|v| v.clone()),
                    reply.attributes.get("name").and_then(|v| v.clone()),
                ));
            }
        }
        Ok(Self::find_router_named_item_id(&rows, profile_name))
    }

    async fn get_ppp_profile_row(
        &self,
        tenant_id: &str,
        router_id: &str,
        id: &str,
    ) -> AppResult<MikrotikPppProfile> {
        sqlx::query_as::<_, MikrotikPppProfile>(
            r#"
            SELECT * FROM mikrotik_ppp_profiles
            WHERE tenant_id = $1 AND router_id = $2 AND id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("PPP profile not found".into()))
    }

    async fn find_router_ip_pool_id_by_name(
        &self,
        dev: &MikrotikDevice,
        pool_name: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let cmd = CommandBuilder::new()
            .command("/ip/pool/print")
            .attribute("detail", Some(""))
            .build();
        let mut rx = dev.send_command(cmd).await?;
        let mut rows = Vec::new();
        while let Some(res) = rx.recv().await {
            let response = res?;
            if let CommandResponse::Reply(reply) = response {
                rows.push((
                    reply.attributes.get(".id").and_then(|v| v.clone()),
                    reply.attributes.get("name").and_then(|v| v.clone()),
                ));
            }
        }
        Ok(Self::find_router_named_item_id(&rows, pool_name))
    }

    fn find_router_named_item_id(
        rows: &[(Option<String>, Option<String>)],
        target_name: &str,
    ) -> Option<String> {
        rows.iter().find_map(|(id, name)| {
            if name.as_deref() == Some(target_name) {
                id.clone()
            } else {
                None
            }
        })
    }

    fn compute_router_missing_names(
        existing_names: &[String],
        seen_names: &std::collections::HashSet<String>,
    ) -> Vec<String> {
        existing_names
            .iter()
            .filter(|name| !seen_names.contains(*name))
            .cloned()
            .collect()
    }

    async fn get_ip_pool_row(
        &self,
        tenant_id: &str,
        router_id: &str,
        id: &str,
    ) -> AppResult<MikrotikIpPool> {
        sqlx::query_as::<_, MikrotikIpPool>(
            r#"
            SELECT * FROM mikrotik_ip_pools
            WHERE tenant_id = $1 AND router_id = $2 AND id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("IP pool not found".into()))
    }

    pub async fn get_ppp_profile_dependencies(
        &self,
        tenant_id: &str,
        router_id: &str,
        id: &str,
    ) -> AppResult<MikrotikPppProfileDependencyStatus> {
        let profile = self.get_ppp_profile_row(tenant_id, router_id, id).await?;

        let pppoe_accounts: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM pppoe_accounts
            WHERE tenant_id = $1 AND router_id = $2 AND router_profile_name = $3
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(&profile.name)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let package_mappings: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM isp_package_router_mappings
            WHERE tenant_id = $1 AND router_id = $2 AND router_profile_name = $3
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(&profile.name)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let dependencies = vec![
            MikrotikPppProfileDependencyItem {
                r#type: "pppoe_accounts".into(),
                label: "PPPoE accounts".into(),
                count: pppoe_accounts,
            },
            MikrotikPppProfileDependencyItem {
                r#type: "isp_package_router_mappings".into(),
                label: "ISP package mappings".into(),
                count: package_mappings,
            },
        ];

        Ok(MikrotikPppProfileDependencyStatus {
            profile_id: profile.id,
            profile_name: profile.name,
            router_id: router_id.to_string(),
            can_delete: dependencies.iter().all(|item| item.count == 0),
            dependencies,
        })
    }

    pub async fn create_ppp_profile(
        &self,
        tenant_id: &str,
        router_id: &str,
        payload: CreateMikrotikPppProfileRequest,
    ) -> AppResult<MikrotikPppProfile> {
        let name = payload.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Validation("name is required".into()));
        }

        let router = self
            .get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".into()))?;
        let dev = self
            .connect_device(&router)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if self
            .find_router_ppp_profile_id_by_name(&dev, &name)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .is_some()
        {
            return Err(AppError::Validation(
                "PPP profile name already exists".into(),
            ));
        }

        let mut builder = CommandBuilder::new()
            .command("/ppp/profile/add")
            .attribute("name", Some(name.as_str()));
        let local_address = Self::normalize_optional_text(payload.local_address);
        let remote_address = Self::normalize_optional_text(payload.remote_address);
        let rate_limit = Self::normalize_optional_text(payload.rate_limit);
        let dns_server = Self::normalize_optional_text(payload.dns_server);
        let comment = Self::normalize_optional_text(payload.comment);
        let only_one = payload.only_one.unwrap_or(false);
        if let Some(value) = local_address.as_deref() {
            builder = builder.attribute("local-address", Some(value));
        }
        if let Some(value) = remote_address.as_deref() {
            builder = builder.attribute("remote-address", Some(value));
        }
        if let Some(value) = rate_limit.as_deref() {
            builder = builder.attribute("rate-limit", Some(value));
        }
        if let Some(value) = dns_server.as_deref() {
            builder = builder.attribute("dns-server", Some(value));
        }
        if let Some(value) = comment.as_deref() {
            builder = builder.attribute("comment", Some(value));
        }
        builder = builder.attribute("only-one", Some(if only_one { "yes" } else { "no" }));

        let mut rx = dev
            .send_command(builder.build())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        while let Some(res) = rx.recv().await {
            match res.map_err(|e| AppError::Internal(e.to_string()))? {
                CommandResponse::Trap(trap) => {
                    let message = trap.message.trim().to_string();
                    return Err(AppError::Validation(if message.is_empty() {
                        "Router rejected PPP profile create".into()
                    } else {
                        message
                    }));
                }
                CommandResponse::Done(_) => break,
                _ => {}
            }
        }

        let rows = self.sync_ppp_profiles(tenant_id, router_id).await?;
        rows.into_iter()
            .find(|row| row.name == name)
            .ok_or_else(|| {
                AppError::Internal("PPP profile created on router but mirror refresh failed".into())
            })
    }

    pub async fn update_ppp_profile(
        &self,
        tenant_id: &str,
        router_id: &str,
        id: &str,
        payload: UpdateMikrotikPppProfileRequest,
    ) -> AppResult<MikrotikPppProfile> {
        let profile = self.get_ppp_profile_row(tenant_id, router_id, id).await?;
        if payload
            .name
            .as_ref()
            .map(|value| value.trim() != profile.name)
            .unwrap_or(false)
        {
            return Err(AppError::Validation(
                "PPP profile rename is not allowed in phase one".into(),
            ));
        }

        let router = self
            .get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".into()))?;
        let dev = self
            .connect_device(&router)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let router_profile_id = self
            .find_router_ppp_profile_id_by_name(&dev, &profile.name)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| {
                AppError::Conflict(
                    "PPP profile no longer exists on router. Sync from router before retrying."
                        .into(),
                )
            })?;

        let mut builder = CommandBuilder::new()
            .command("/ppp/profile/set")
            .attribute("numbers", Some(router_profile_id.as_str()));
        let local_address = Self::normalize_optional_text(payload.local_address);
        let remote_address = Self::normalize_optional_text(payload.remote_address);
        let rate_limit = Self::normalize_optional_text(payload.rate_limit);
        let dns_server = Self::normalize_optional_text(payload.dns_server);
        let comment = Self::normalize_optional_text(payload.comment);
        let only_one = payload.only_one;
        builder = builder.attribute("local-address", local_address.as_deref());
        builder = builder.attribute("remote-address", remote_address.as_deref());
        builder = builder.attribute("rate-limit", rate_limit.as_deref());
        builder = builder.attribute("dns-server", dns_server.as_deref());
        builder = builder.attribute("comment", comment.as_deref());
        if let Some(value) = only_one {
            builder = builder.attribute("only-one", Some(if value { "yes" } else { "no" }));
        }

        let mut rx = dev
            .send_command(builder.build())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        while let Some(res) = rx.recv().await {
            match res.map_err(|e| AppError::Internal(e.to_string()))? {
                CommandResponse::Trap(trap) => {
                    let message = trap.message.trim().to_string();
                    return Err(AppError::Validation(if message.is_empty() {
                        "Router rejected PPP profile update".into()
                    } else {
                        message
                    }));
                }
                CommandResponse::Done(_) => break,
                _ => {}
            }
        }

        let rows = self.sync_ppp_profiles(tenant_id, router_id).await?;
        rows.into_iter()
            .find(|row| row.id == profile.id || row.name == profile.name)
            .ok_or_else(|| {
                AppError::Internal("PPP profile updated on router but mirror refresh failed".into())
            })
    }

    pub async fn delete_ppp_profile(
        &self,
        tenant_id: &str,
        router_id: &str,
        id: &str,
    ) -> AppResult<MikrotikPppProfileDeleteResult> {
        let profile = self.get_ppp_profile_row(tenant_id, router_id, id).await?;
        let dependencies = self
            .get_ppp_profile_dependencies(tenant_id, router_id, id)
            .await?;
        if !dependencies.can_delete {
            return Err(AppError::Validation(format!(
                "PPP profile is still in use: {}",
                dependencies
                    .dependencies
                    .iter()
                    .filter(|item| item.count > 0)
                    .map(|item| format!("{} ({})", item.label, item.count))
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }

        let router = self
            .get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".into()))?;
        let dev = self
            .connect_device(&router)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let router_profile_id = self
            .find_router_ppp_profile_id_by_name(&dev, &profile.name)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| {
                AppError::Conflict(
                    "PPP profile no longer exists on router. Sync from router before retrying."
                        .into(),
                )
            })?;

        let dependencies = self
            .get_ppp_profile_dependencies(tenant_id, router_id, id)
            .await?;
        if !dependencies.can_delete {
            return Err(AppError::Validation("PPP profile is still in use".into()));
        }

        let mut rx = dev
            .send_command(
                CommandBuilder::new()
                    .command("/ppp/profile/remove")
                    .attribute("numbers", Some(router_profile_id.as_str()))
                    .build(),
            )
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        while let Some(res) = rx.recv().await {
            match res.map_err(|e| AppError::Internal(e.to_string()))? {
                CommandResponse::Trap(trap) => {
                    let message = trap.message.trim().to_string();
                    return Err(AppError::Validation(if message.is_empty() {
                        "Router rejected PPP profile delete".into()
                    } else {
                        message
                    }));
                }
                CommandResponse::Done(_) => break,
                _ => {}
            }
        }

        sqlx::query(
            r#"
            DELETE FROM mikrotik_ppp_profiles
            WHERE tenant_id = $1 AND router_id = $2 AND id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(&profile.id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.sync_ppp_profiles(tenant_id, router_id).await?;
        Ok(MikrotikPppProfileDeleteResult {
            ok: true,
            deleted_profile_id: profile.id,
            deleted_profile_name: profile.name,
            router_id: router_id.to_string(),
        })
    }

    pub async fn list_ppp_profiles(
        &self,
        tenant_id: &str,
        router_id: &str,
    ) -> AppResult<Vec<crate::models::MikrotikPppProfile>> {
        let rows = sqlx::query_as::<_, crate::models::MikrotikPppProfile>(
            r#"
            SELECT * FROM mikrotik_ppp_profiles
            WHERE tenant_id = $1 AND router_id = $2
            ORDER BY name ASC
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(rows)
    }

    pub async fn list_ip_pools(
        &self,
        tenant_id: &str,
        router_id: &str,
    ) -> AppResult<Vec<crate::models::MikrotikIpPool>> {
        let rows = sqlx::query_as::<_, crate::models::MikrotikIpPool>(
            r#"
            SELECT * FROM mikrotik_ip_pools
            WHERE tenant_id = $1 AND router_id = $2
            ORDER BY name ASC
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(rows)
    }

    pub async fn get_ip_pool_dependencies(
        &self,
        tenant_id: &str,
        router_id: &str,
        id: &str,
    ) -> AppResult<MikrotikIpPoolDependencyStatus> {
        let pool = self.get_ip_pool_row(tenant_id, router_id, id).await?;

        let pppoe_accounts: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM pppoe_accounts
            WHERE tenant_id = $1 AND router_id = $2 AND address_pool = $3
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(&pool.name)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let package_mappings: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM isp_package_router_mappings
            WHERE tenant_id = $1 AND router_id = $2 AND address_pool = $3
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(&pool.name)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let dependencies = build_ip_pool_dependency_items(pppoe_accounts, package_mappings);

        Ok(MikrotikIpPoolDependencyStatus {
            pool_id: pool.id,
            pool_name: pool.name,
            router_id: router_id.to_string(),
            can_delete: true,
            dependencies,
        })
    }

    pub async fn create_ip_pool(
        &self,
        tenant_id: &str,
        router_id: &str,
        payload: CreateMikrotikIpPoolRequest,
    ) -> AppResult<MikrotikIpPool> {
        let name = payload.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Validation("name is required".into()));
        }

        let router = self
            .get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".into()))?;
        let dev = self
            .connect_device(&router)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if self
            .find_router_ip_pool_id_by_name(&dev, &name)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .is_some()
        {
            return Err(AppError::Validation("IP pool name already exists".into()));
        }

        let mut builder = CommandBuilder::new()
            .command("/ip/pool/add")
            .attribute("name", Some(name.as_str()));
        let ranges = Self::normalize_optional_text(payload.ranges);
        let next_pool = Self::normalize_optional_text(payload.next_pool);
        let comment = Self::normalize_optional_text(payload.comment);
        if let Some(value) = ranges.as_deref() {
            builder = builder.attribute("ranges", Some(value));
        }
        if let Some(value) = next_pool.as_deref() {
            builder = builder.attribute("next-pool", Some(value));
        }
        if let Some(value) = comment.as_deref() {
            builder = builder.attribute("comment", Some(value));
        }

        let mut rx = dev
            .send_command(builder.build())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        while let Some(res) = rx.recv().await {
            match res.map_err(|e| AppError::Internal(e.to_string()))? {
                CommandResponse::Trap(trap) => {
                    let message = trap.message.trim().to_string();
                    return Err(AppError::Validation(if message.is_empty() {
                        "Router rejected IP pool create".into()
                    } else {
                        message
                    }));
                }
                CommandResponse::Done(_) => break,
                _ => {}
            }
        }

        let rows = self.sync_ip_pools(tenant_id, router_id).await?;
        rows.into_iter()
            .find(|row| row.name == name)
            .ok_or_else(|| {
                AppError::Internal("IP pool created on router but mirror refresh failed".into())
            })
    }

    pub async fn update_ip_pool(
        &self,
        tenant_id: &str,
        router_id: &str,
        id: &str,
        payload: UpdateMikrotikIpPoolRequest,
    ) -> AppResult<MikrotikIpPool> {
        let pool = self.get_ip_pool_row(tenant_id, router_id, id).await?;
        if let Some(message) = ip_pool_rename_attempted(payload.name.as_deref(), &pool.name) {
            return Err(AppError::Validation(message));
        }

        let router = self
            .get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".into()))?;
        let dev = self
            .connect_device(&router)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let router_pool_id = self
            .find_router_ip_pool_id_by_name(&dev, &pool.name)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| {
                AppError::Conflict(
                    "IP pool no longer exists on router. Sync from router before retrying.".into(),
                )
            })?;

        let mut builder = CommandBuilder::new()
            .command("/ip/pool/set")
            .attribute("numbers", Some(router_pool_id.as_str()));
        let ranges = Self::normalize_optional_text(payload.ranges);
        let next_pool = Self::normalize_optional_text(payload.next_pool);
        let comment = Self::normalize_optional_text(payload.comment);
        builder = builder.attribute("ranges", ranges.as_deref());
        builder = builder.attribute("next-pool", next_pool.as_deref());
        builder = builder.attribute("comment", comment.as_deref());

        let mut rx = dev
            .send_command(builder.build())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        while let Some(res) = rx.recv().await {
            match res.map_err(|e| AppError::Internal(e.to_string()))? {
                CommandResponse::Trap(trap) => {
                    let message = trap.message.trim().to_string();
                    return Err(AppError::Validation(if message.is_empty() {
                        "Router rejected IP pool update".into()
                    } else {
                        message
                    }));
                }
                CommandResponse::Done(_) => break,
                _ => {}
            }
        }

        let rows = self.sync_ip_pools(tenant_id, router_id).await?;
        rows.into_iter()
            .find(|row| row.id == pool.id || row.name == pool.name)
            .ok_or_else(|| {
                AppError::Internal("IP pool updated on router but mirror refresh failed".into())
            })
    }

    pub async fn delete_ip_pool(
        &self,
        tenant_id: &str,
        router_id: &str,
        id: &str,
    ) -> AppResult<MikrotikIpPoolDeleteResult> {
        let pool = self.get_ip_pool_row(tenant_id, router_id, id).await?;
        let router = self
            .get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".into()))?;
        let dev = self
            .connect_device(&router)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let router_pool_id = self
            .find_router_ip_pool_id_by_name(&dev, &pool.name)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| {
                AppError::Conflict(
                    "IP pool no longer exists on router. Sync from router before retrying.".into(),
                )
            })?;

        let latest_warning_status = self
            .get_ip_pool_dependencies(tenant_id, router_id, id)
            .await?;
        let warnings = latest_warning_status.dependencies;

        let mut rx = dev
            .send_command(
                CommandBuilder::new()
                    .command("/ip/pool/remove")
                    .attribute("numbers", Some(router_pool_id.as_str()))
                    .build(),
            )
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        while let Some(res) = rx.recv().await {
            match res.map_err(|e| AppError::Internal(e.to_string()))? {
                CommandResponse::Trap(trap) => {
                    let message = trap.message.trim().to_string();
                    return Err(AppError::Validation(if message.is_empty() {
                        "Router rejected IP pool delete".into()
                    } else {
                        message
                    }));
                }
                CommandResponse::Done(_) => break,
                _ => {}
            }
        }

        sqlx::query(
            r#"
            DELETE FROM mikrotik_ip_pools
            WHERE tenant_id = $1 AND router_id = $2 AND id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(&pool.id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.sync_ip_pools(tenant_id, router_id).await?;
        Ok(MikrotikIpPoolDeleteResult {
            ok: true,
            deleted_pool_id: pool.id,
            deleted_pool_name: pool.name,
            router_id: router_id.to_string(),
            warnings: if ip_pool_has_dependency_warnings(&warnings) {
                warnings
            } else {
                Vec::new()
            },
        })
    }

    pub async fn sync_ppp_profiles(
        &self,
        tenant_id: &str,
        router_id: &str,
    ) -> AppResult<Vec<crate::models::MikrotikPppProfile>> {
        let router = self
            .get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".into()))?;

        let dev = self
            .connect_device(&router)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let cmd = CommandBuilder::new()
            .command("/ppp/profile/print")
            .attribute("detail", Some(""))
            .build();
        let mut rx = dev
            .send_command(cmd)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let now = chrono::Utc::now();
        let mut seen: std::collections::HashSet<String> = Default::default();

        while let Some(res) = rx.recv().await {
            let r = res.map_err(|e| AppError::Internal(e.to_string()))?;
            if let CommandResponse::Reply(reply) = r {
                let name = reply
                    .attributes
                    .get("name")
                    .and_then(|v| v.clone())
                    .unwrap_or_default();
                if name.trim().is_empty() {
                    continue;
                }
                seen.insert(name.clone());

                let local_address = reply
                    .attributes
                    .get("local-address")
                    .and_then(|v| v.clone());
                let remote_address = reply
                    .attributes
                    .get("remote-address")
                    .and_then(|v| v.clone());
                let rate_limit = reply.attributes.get("rate-limit").and_then(|v| v.clone());
                let dns_server = reply.attributes.get("dns-server").and_then(|v| v.clone());

                let only_one =
                    Self::parse_bool_opt(reply.attributes.get("only-one").and_then(|v| v.as_ref()));
                let change_tcp_mss = Self::parse_bool_opt(
                    reply
                        .attributes
                        .get("change-tcp-mss")
                        .and_then(|v| v.as_ref()),
                );
                let use_compression = Self::parse_bool_opt(
                    reply
                        .attributes
                        .get("use-compression")
                        .and_then(|v| v.as_ref()),
                );
                let use_encryption = Self::parse_bool_opt(
                    reply
                        .attributes
                        .get("use-encryption")
                        .and_then(|v| v.as_ref()),
                );
                let use_ipv6 =
                    Self::parse_bool_opt(reply.attributes.get("use-ipv6").and_then(|v| v.as_ref()));
                let bridge = reply.attributes.get("bridge").and_then(|v| v.clone());
                let comment = reply.attributes.get("comment").and_then(|v| v.clone());

                let id: Option<String> = sqlx::query_scalar(
                    "SELECT id FROM mikrotik_ppp_profiles WHERE tenant_id = $1 AND router_id = $2 AND name = $3",
                )
                .bind(tenant_id)
                .bind(router_id)
                .bind(&name)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?;
                let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                sqlx::query(
                    r#"
                    INSERT INTO mikrotik_ppp_profiles
                      (id, tenant_id, router_id, name, local_address, remote_address, rate_limit, dns_server,
                       only_one, change_tcp_mss, use_compression, use_encryption, use_ipv6, bridge, comment,
                       router_present, last_sync_at, created_at, updated_at)
                    VALUES
                      ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,true,$16,$17,$18)
                    ON CONFLICT (tenant_id, router_id, name) DO UPDATE SET
                      local_address = EXCLUDED.local_address,
                      remote_address = EXCLUDED.remote_address,
                      rate_limit = EXCLUDED.rate_limit,
                      dns_server = EXCLUDED.dns_server,
                      only_one = EXCLUDED.only_one,
                      change_tcp_mss = EXCLUDED.change_tcp_mss,
                      use_compression = EXCLUDED.use_compression,
                      use_encryption = EXCLUDED.use_encryption,
                      use_ipv6 = EXCLUDED.use_ipv6,
                      bridge = EXCLUDED.bridge,
                      comment = EXCLUDED.comment,
                      router_present = true,
                      last_sync_at = EXCLUDED.last_sync_at,
                      updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(&id)
                .bind(tenant_id)
                .bind(router_id)
                .bind(&name)
                .bind(local_address)
                .bind(remote_address)
                .bind(rate_limit)
                .bind(dns_server)
                .bind(only_one)
                .bind(change_tcp_mss)
                .bind(use_compression)
                .bind(use_encryption)
                .bind(use_ipv6)
                .bind(bridge)
                .bind(comment)
                .bind(now)
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;
            }
        }

        let existing_names: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT name FROM mikrotik_ppp_profiles
            WHERE tenant_id = $1 AND router_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let missing_names = Self::compute_router_missing_names(&existing_names, &seen);
        if !missing_names.is_empty() {
            sqlx::query(
                r#"
                DELETE FROM mikrotik_ppp_profiles
                WHERE tenant_id = $1 AND router_id = $2 AND name = ANY($3)
                "#,
            )
            .bind(tenant_id)
            .bind(router_id)
            .bind(&missing_names)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        }

        let _ = sqlx::query(
            r#"
            UPDATE mikrotik_ppp_profiles
            SET router_present = true, last_sync_at = $1, updated_at = $2
            WHERE tenant_id = $3 AND router_id = $4
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(tenant_id)
        .bind(router_id)
        .execute(&self.pool)
        .await;

        self.list_ppp_profiles(tenant_id, router_id).await
    }

    pub async fn sync_ip_pools(
        &self,
        tenant_id: &str,
        router_id: &str,
    ) -> AppResult<Vec<crate::models::MikrotikIpPool>> {
        let router = self
            .get_router(tenant_id, router_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Router not found".into()))?;

        let dev = self
            .connect_device(&router)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let cmd = CommandBuilder::new()
            .command("/ip/pool/print")
            .attribute("detail", Some(""))
            .build();
        let mut rx = dev
            .send_command(cmd)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let now = chrono::Utc::now();
        let mut seen: std::collections::HashSet<String> = Default::default();

        while let Some(res) = rx.recv().await {
            let r = res.map_err(|e| AppError::Internal(e.to_string()))?;
            if let CommandResponse::Reply(reply) = r {
                let name = reply
                    .attributes
                    .get("name")
                    .and_then(|v| v.clone())
                    .unwrap_or_default();
                if name.trim().is_empty() {
                    continue;
                }
                seen.insert(name.clone());

                let ranges = reply.attributes.get("ranges").and_then(|v| v.clone());
                let next_pool = reply.attributes.get("next-pool").and_then(|v| v.clone());
                let comment = reply.attributes.get("comment").and_then(|v| v.clone());

                let id: Option<String> = sqlx::query_scalar(
                    "SELECT id FROM mikrotik_ip_pools WHERE tenant_id = $1 AND router_id = $2 AND name = $3",
                )
                .bind(tenant_id)
                .bind(router_id)
                .bind(&name)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?;
                let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                sqlx::query(
                    r#"
                    INSERT INTO mikrotik_ip_pools
                      (id, tenant_id, router_id, name, ranges, next_pool, comment, router_present, last_sync_at, created_at, updated_at)
                    VALUES
                      ($1,$2,$3,$4,$5,$6,$7,true,$8,$9,$10)
                    ON CONFLICT (tenant_id, router_id, name) DO UPDATE SET
                      ranges = EXCLUDED.ranges,
                      next_pool = EXCLUDED.next_pool,
                      comment = EXCLUDED.comment,
                      router_present = true,
                      last_sync_at = EXCLUDED.last_sync_at,
                      updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(&id)
                .bind(tenant_id)
                .bind(router_id)
                .bind(&name)
                .bind(ranges)
                .bind(next_pool)
                .bind(comment)
                .bind(now)
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;
            }
        }

        let existing_names: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT name FROM mikrotik_ip_pools
            WHERE tenant_id = $1 AND router_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let missing_names = Self::compute_router_missing_names(&existing_names, &seen);
        if !missing_names.is_empty() {
            sqlx::query(
                r#"
                DELETE FROM mikrotik_ip_pools
                WHERE tenant_id = $1 AND router_id = $2 AND name = ANY($3)
                "#,
            )
            .bind(tenant_id)
            .bind(router_id)
            .bind(&missing_names)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        }

        let _ = sqlx::query(
            r#"
            UPDATE mikrotik_ip_pools
            SET router_present = true, last_sync_at = $1, updated_at = $2
            WHERE tenant_id = $3 AND router_id = $4
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(tenant_id)
        .bind(router_id)
        .execute(&self.pool)
        .await;

        self.list_ip_pools(tenant_id, router_id).await
    }

    async fn notify_tenant(
        &self,
        tenant_id: &str,
        title: &str,
        message: String,
        action_url: Option<String>,
        notification_type: &str,
    ) {
        // Send to tenant members with relevant network monitoring or router inventory access.
        let user_ids: Result<Vec<String>, sqlx::Error> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT tm.user_id
            FROM tenant_members tm
            JOIN role_permissions rp ON rp.role_id = tm.role_id
            JOIN permissions p ON p.id = rp.permission_id
            WHERE tm.tenant_id = $1
              AND p.resource IN ('network_noc', 'network_alerts', 'network_incidents', 'network_logs', 'router_inventory')
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

        for uid in &user_ids {
            let _ = self
                .notification_service
                .create_notification(
                    uid.clone(),
                    Some(tenant_id.to_string()),
                    title.to_string(),
                    message.clone(),
                    notification_type.to_string(),
                    "network".to_string(),
                    action_url.clone(),
                )
                .await;
        }

        // Optional: email notify to the same audience (tenant-scoped SMTP settings).
        let email_enabled = match self
            .settings_service
            .get_value(Some(tenant_id), "mikrotik_alert_email_enabled")
            .await
        {
            Ok(Some(v)) => matches!(
                v.trim().to_lowercase().as_str(),
                "true" | "1" | "yes" | "on"
            ),
            _ => false,
        };

        if email_enabled {
            let mut body = message.clone();
            if let Some(url) = action_url {
                body.push_str("\n\nOpen: ");
                body.push_str(&url);
            }

            #[cfg(feature = "postgres")]
            {
                let _ = self
                    .notification_service
                    .force_send_email_to_users(Some(tenant_id.to_string()), &user_ids, title, &body)
                    .await;
            }
        }
    }

    async fn notify_router_status_change(
        &self,
        tenant_id: &str,
        title: &str,
        message: String,
        action_url: Option<String>,
        notification_type: &str,
    ) {
        let enabled = match self
            .settings_service
            .get_value(Some(tenant_id), "mikrotik_status_notify_enabled")
            .await
        {
            Ok(Some(v)) => {
                let x = v.trim().to_ascii_lowercase();
                x == "1" || x == "true" || x == "yes" || x == "on"
            }
            Ok(None) => true,
            Err(_) => true,
        };
        if !enabled {
            return;
        }

        let cooldown_secs = match self
            .settings_service
            .get_value(Some(tenant_id), "mikrotik_status_notify_cooldown_secs")
            .await
        {
            Ok(Some(v)) => v.trim().parse::<i64>().unwrap_or(90),
            _ => 90,
        }
        .clamp(0, 3600);

        if cooldown_secs > 0 {
            let latest: Result<Option<DateTime<Utc>>, sqlx::Error> = sqlx::query_scalar(
                r#"
                SELECT created_at
                FROM notifications
                WHERE tenant_id = $1
                  AND category = 'network'
                  AND title = $2
                  AND ($3::text IS NULL OR action_url = $3)
                ORDER BY created_at DESC
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(title)
            .bind(action_url.as_deref())
            .fetch_optional(&self.pool)
            .await;

            if let Ok(Some(last_at)) = latest {
                if Utc::now() - last_at < ChronoDuration::seconds(cooldown_secs) {
                    return;
                }
            }
        }

        self.notify_tenant(tenant_id, title, message, action_url, notification_type)
            .await;
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

fn resolve_router_log_retention_days(raw: Option<&str>) -> Option<i64> {
    match raw.map(str::trim) {
        None | Some("") => None,
        Some(value) if value.eq_ignore_ascii_case("unlimited") || value == "0" => None,
        Some("30") => Some(30),
        Some("90") => Some(90),
        Some("360") => Some(360),
        Some(_) => None,
    }
}

fn validate_router_log_retention_days(retention_days: Option<i64>) -> AppResult<()> {
    if retention_days.is_none() || matches!(retention_days, Some(30 | 90 | 360)) {
        Ok(())
    } else {
        Err(AppError::Validation(
            "Router log retention must be unlimited, 30, 90, or 360 days".to_string(),
        ))
    }
}

fn router_log_retention_setting_key(router_id: &str) -> String {
    format!("mikrotik_logs_retention_days::{router_id}")
}

fn mikrotik_log_retention_cutoff(
    now: DateTime<Utc>,
    retention_days: Option<i64>,
) -> Option<DateTime<Utc>> {
    retention_days.map(|days| now - ChronoDuration::days(days))
}

fn validate_log_calendar_filters(month: Option<u32>, year: Option<i32>) -> AppResult<()> {
    if let Some(month) = month {
        if !(1..=12).contains(&month) {
            return Err(AppError::Validation(
                "Log month filter must be between 1 and 12".to_string(),
            ));
        }
    }
    if let Some(year) = year {
        if !(1970..=9999).contains(&year) {
            return Err(AppError::Validation(
                "Log year filter must be between 1970 and 9999".to_string(),
            ));
        }
    }
    Ok(())
}

fn build_ip_pool_dependency_items(
    pppoe_accounts: i64,
    package_mappings: i64,
) -> Vec<MikrotikIpPoolDependencyItem> {
    vec![
        MikrotikIpPoolDependencyItem {
            r#type: "pppoe_accounts".into(),
            label: "PPPoE accounts".into(),
            count: pppoe_accounts,
        },
        MikrotikIpPoolDependencyItem {
            r#type: "isp_package_router_mappings".into(),
            label: "ISP package mappings".into(),
            count: package_mappings,
        },
    ]
}

fn ip_pool_has_dependency_warnings(dependencies: &[MikrotikIpPoolDependencyItem]) -> bool {
    dependencies.iter().any(|item| item.count > 0)
}

fn ip_pool_rename_attempted(requested_name: Option<&str>, current_name: &str) -> Option<String> {
    requested_name.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed == current_name {
            None
        } else {
            Some("IP pool rename is not allowed in phase one".to_string())
        }
    })
}

fn mikrotik_log_list_sql() -> &'static str {
    r#"
    SELECT l.*
    FROM mikrotik_logs l
    WHERE l.tenant_id = $1
      AND ($2::text IS NULL OR l.router_id = $2)
      AND ($3::text IS NULL OR l.level = $3)
      AND ($4::text IS NULL OR l.topics ILIKE '%' || $4 || '%')
      AND ($5 = '' OR l.message ILIKE '%' || $5 || '%')
      AND ($6::int4 IS NULL OR EXTRACT(MONTH FROM l.logged_at) = $6)
      AND ($7::int4 IS NULL OR EXTRACT(YEAR FROM l.logged_at) = $7)
    ORDER BY l.logged_at DESC, l.updated_at DESC
    LIMIT $8 OFFSET $9
    "#
}

fn mikrotik_log_prune_sql() -> &'static str {
    r#"
    DELETE FROM mikrotik_logs
    WHERE tenant_id = $1
      AND router_id = $2
      AND logged_at < $3
    "#
}

fn propagate_prune_query_result<T>(result: Result<T, sqlx::Error>) -> AppResult<T> {
    result.map_err(AppError::Database)
}

#[cfg(test)]
fn should_prune_log_by_retention_cutoff(logged_at: DateTime<Utc>, cutoff: DateTime<Utc>) -> bool {
    logged_at < cutoff
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::WsHub;
    use crate::services::{AuditService, EmailOutboxService, EmailService, NotificationService};
    use sqlx::postgres::PgPoolOptions;
    use std::sync::Arc;

    fn test_database_url() -> String {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            let host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
            let port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "55432".to_string());
            let user =
                std::env::var("POSTGRES_USER").unwrap_or_else(|_| "ispmanagement".to_string());
            let password =
                std::env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "postgres".to_string());
            let db = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "ispmanagement".to_string());
            format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, db)
        })
    }

    async fn test_pool() -> sqlx::PgPool {
        PgPoolOptions::new()
            .max_connections(1)
            .connect(&test_database_url())
            .await
            .expect("test should connect to postgres")
    }

    fn test_router_password() -> String {
        std::env::var("MIKROTIK_TEST_ROUTER_PASSWORD")
            .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string())
    }

    fn test_mikrotik_service(pool: sqlx::PgPool) -> MikrotikService {
        let audit_service = AuditService::new(pool.clone(), None);
        let settings_service = SettingsService::new(pool.clone(), audit_service.clone());
        let email_service = EmailService::new(settings_service.clone());
        let email_outbox_service =
            EmailOutboxService::new(pool.clone(), settings_service.clone(), email_service);
        let notification_service =
            NotificationService::new(pool.clone(), Arc::new(WsHub::new()), email_outbox_service);

        MikrotikService::new(pool, notification_service, audit_service, settings_service)
    }

    #[test]
    fn mikrotik_router_log_retention_validation() {
        assert_eq!(resolve_router_log_retention_days(None), None);
        assert_eq!(resolve_router_log_retention_days(Some("unlimited")), None);
        assert_eq!(resolve_router_log_retention_days(Some("0")), None);
        assert_eq!(resolve_router_log_retention_days(Some("")), None);
        assert_eq!(resolve_router_log_retention_days(Some("30")), Some(30));
        assert_eq!(resolve_router_log_retention_days(Some("90")), Some(90));
        assert_eq!(resolve_router_log_retention_days(Some("360")), Some(360));
        assert_eq!(resolve_router_log_retention_days(Some("365")), None);
    }

    #[test]
    fn prune_is_disabled_for_unlimited_router_retention() {
        let now = DateTime::parse_from_rfc3339("2026-03-31T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(mikrotik_log_retention_cutoff(now, None), None);
    }

    #[test]
    fn prune_keeps_logs_at_or_newer_than_retention_cutoff() {
        let now = DateTime::parse_from_rfc3339("2026-03-31T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let cutoff =
            mikrotik_log_retention_cutoff(now, Some(30)).expect("cutoff for 30d retention");

        assert!(should_prune_log_by_retention_cutoff(
            cutoff - ChronoDuration::seconds(1),
            cutoff
        ));
        assert!(!should_prune_log_by_retention_cutoff(cutoff, cutoff));
        assert!(!should_prune_log_by_retention_cutoff(
            cutoff + ChronoDuration::seconds(1),
            cutoff
        ));
    }

    #[test]
    fn prune_sql_uses_router_retention_cutoff_without_fixed_5000_cap() {
        let sql = mikrotik_log_prune_sql();
        assert!(sql.contains("logged_at < $3"));
        assert!(!sql.contains("OFFSET 5000"));
    }

    #[test]
    fn ip_pool_dependencies_become_warning_items() {
        let dependencies = build_ip_pool_dependency_items(2, 1);

        assert_eq!(dependencies.len(), 2);
        assert_eq!(dependencies[0].r#type, "pppoe_accounts");
        assert_eq!(dependencies[0].count, 2);
        assert_eq!(dependencies[1].r#type, "isp_package_router_mappings");
        assert_eq!(dependencies[1].count, 1);
        assert!(ip_pool_has_dependency_warnings(&dependencies));
    }

    #[test]
    fn ip_pool_rename_check_allows_same_name_and_rejects_changes() {
        assert!(ip_pool_rename_attempted(None, "pool-a").is_none());
        assert!(ip_pool_rename_attempted(Some(" pool-a "), "pool-a").is_none());
        assert_eq!(
            ip_pool_rename_attempted(Some("pool-b"), "pool-a"),
            Some("IP pool rename is not allowed in phase one".to_string())
        );
    }

    #[test]
    fn router_named_item_id_lookup_uses_exact_name_match_from_print_rows() {
        let rows = vec![
            (Some("*1".to_string()), Some("pool-a".to_string())),
            (Some("*2".to_string()), Some("pool-b".to_string())),
        ];

        assert_eq!(
            MikrotikService::find_router_named_item_id(&rows, "pool-b"),
            Some("*2".to_string())
        );
        assert_eq!(
            MikrotikService::find_router_named_item_id(&rows, "pool-c"),
            None
        );
    }

    #[test]
    fn prune_router_missing_names_keeps_only_seen_pool_names() {
        let existing = vec![
            "pool-a".to_string(),
            "pool-b".to_string(),
            "pool-c".to_string(),
        ];
        let seen: std::collections::HashSet<String> = ["pool-a".to_string(), "pool-c".to_string()]
            .into_iter()
            .collect();

        assert_eq!(
            MikrotikService::compute_router_missing_names(&existing, &seen),
            vec!["pool-b".to_string()]
        );
    }

    #[tokio::test]
    async fn sync_logs_returns_error_when_prune_query_fails() {
        let pool = test_pool().await;
        let service = test_mikrotik_service(pool.clone());
        service.configure_test_sync_logs_injection(TestSyncLogsInjection {
            rows_override: Some(Vec::new()),
            force_prune_error: true,
        });

        let tenant_id = "tenant-sync-prune-error";
        let router_id = "router-sync-prune-error";
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO tenants (id, name, slug, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .bind("Sync Prune Error Tenant")
        .bind("sync-prune-error-tenant")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed tenant");

        sqlx::query(
            r#"
            INSERT INTO mikrotik_routers
              (id, tenant_id, name, host, port, username, password, use_tls, enabled, is_online, created_at, updated_at)
            VALUES
              ($1, $2, $3, $4, $5, $6, $7, false, true, false, $8, $9)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(router_id)
        .bind(tenant_id)
        .bind("Sync Prune Error Router")
        .bind("127.0.0.1")
        .bind(8728_i32)
        .bind("admin")
        .bind(test_router_password())
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed router");

        let sync_result = service
            .sync_logs_for_router(tenant_id, router_id, 100)
            .await;

        let err = sync_result.expect_err("sync should fail when prune query fails on real path");
        assert!(
            err.to_string()
                .contains("MIKROTIK_TEST_SYNC_FORCE_PRUNE_ERROR"),
            "expected forced prune failure marker, got: {err}"
        );

        sqlx::query("DELETE FROM mikrotik_routers WHERE id = $1")
            .bind(router_id)
            .execute(&pool)
            .await
            .expect("cleanup router");
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await
            .expect("cleanup tenant");
    }

    #[tokio::test]
    async fn list_logs_pagination_order_and_defaults_remain_unchanged() {
        let pool = test_pool().await;
        let service = test_mikrotik_service(pool.clone());

        assert_eq!(MIKROTIK_LOGS_DEFAULT_PAGE, 1);
        assert_eq!(MIKROTIK_LOGS_DEFAULT_PER_PAGE, 25);
        assert!(!MIKROTIK_LOGS_DEFAULT_INCLUDE_TOTAL);

        let tenant_id = "tenant-list-logs-pagination";
        let router_id = "router-list-logs-pagination";
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO tenants (id, name, slug, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .bind("List Logs Pagination Tenant")
        .bind("list-logs-pagination-tenant")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed tenant");

        sqlx::query(
            r#"
            INSERT INTO mikrotik_routers
              (id, tenant_id, name, host, port, username, password, use_tls, enabled, is_online, created_at, updated_at)
            VALUES
              ($1, $2, $3, $4, $5, $6, $7, false, true, false, $8, $9)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(router_id)
        .bind(tenant_id)
        .bind("List Logs Pagination Router")
        .bind("127.0.0.1")
        .bind(8728_i32)
        .bind("admin")
        .bind(test_router_password())
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed router");

        sqlx::query(
            r#"
            INSERT INTO mikrotik_logs (id, tenant_id, router_id, logged_at, message, created_at, updated_at)
            VALUES
              ('tie-newer-updated', $1, $2, $3, 'tie newer updated', $4, $4),
              ('tie-older-updated', $1, $2, $3, 'tie older updated', $5, $5)
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(now + ChronoDuration::minutes(1))
        .bind(now)
        .bind(now - ChronoDuration::seconds(1))
        .execute(&pool)
        .await
        .expect("seed tie-breaker logs");

        sqlx::query(
            r#"
            INSERT INTO mikrotik_logs (id, tenant_id, router_id, logged_at, message, created_at, updated_at)
            SELECT
              'page-log-' || gs::text,
              $1,
              $2,
              $3 - make_interval(secs => gs::int),
              'page log ' || gs::text,
              $4,
              $4
            FROM generate_series(1, 30) AS gs
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed paginated logs");

        let page_1 = service
            .list_logs(
                tenant_id,
                Some(router_id.to_string()),
                None,
                None,
                None,
                None,
                None,
                MIKROTIK_LOGS_DEFAULT_PAGE,
                MIKROTIK_LOGS_DEFAULT_PER_PAGE,
                MIKROTIK_LOGS_DEFAULT_INCLUDE_TOTAL,
            )
            .await
            .expect("list logs page 1");

        assert_eq!(page_1.page, 1);
        assert_eq!(page_1.per_page, 25);
        assert_eq!(
            page_1.total, -1,
            "default include_total=false should keep total sentinel"
        );
        assert_eq!(page_1.data.len(), 25);
        assert_eq!(page_1.data[0].id, "tie-newer-updated");
        assert_eq!(page_1.data[1].id, "tie-older-updated");
        assert_eq!(page_1.data[2].id, "page-log-1");
        assert_eq!(page_1.data[24].id, "page-log-23");

        let page_2 = service
            .list_logs(
                tenant_id,
                Some(router_id.to_string()),
                None,
                None,
                None,
                None,
                None,
                2,
                MIKROTIK_LOGS_DEFAULT_PER_PAGE,
                MIKROTIK_LOGS_DEFAULT_INCLUDE_TOTAL,
            )
            .await
            .expect("list logs page 2");

        assert_eq!(page_2.data.len(), 7);
        assert_eq!(page_2.data[0].id, "page-log-24");
        assert_eq!(page_2.data[6].id, "page-log-30");

        sqlx::query("DELETE FROM mikrotik_logs WHERE tenant_id = $1 AND router_id = $2")
            .bind(tenant_id)
            .bind(router_id)
            .execute(&pool)
            .await
            .expect("cleanup logs");
        sqlx::query("DELETE FROM mikrotik_routers WHERE id = $1")
            .bind(router_id)
            .execute(&pool)
            .await
            .expect("cleanup router");
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await
            .expect("cleanup tenant");
    }

    #[tokio::test]
    async fn sync_logs_for_router_prunes_retention_window_without_fixed_5000_cap() {
        let pool = test_pool().await;
        let service = test_mikrotik_service(pool.clone());
        service.configure_test_sync_logs_injection(TestSyncLogsInjection {
            rows_override: Some(Vec::new()),
            force_prune_error: false,
        });

        let tenant_id = "tenant-sync-path-retention";
        let router_id = "router-sync-path-retention";
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO tenants (id, name, slug, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .bind("Sync Path Tenant")
        .bind("sync-path-tenant")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed tenant");

        sqlx::query(
            r#"
            INSERT INTO mikrotik_routers
              (id, tenant_id, name, host, port, username, password, use_tls, enabled, is_online, created_at, updated_at)
            VALUES
              ($1, $2, $3, $4, $5, $6, $7, false, true, false, $8, $9)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(router_id)
        .bind(tenant_id)
        .bind("Sync Path Router")
        .bind("127.0.0.1")
        .bind(8728_i32)
        .bind("admin")
        .bind(test_router_password())
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed router");

        service
            .update_router_log_retention_days(tenant_id, router_id, Some(30))
            .await
            .expect("set router retention to 30d");

        let old_logged_at = now - ChronoDuration::days(40);
        let fresh_logged_at = now - ChronoDuration::days(1);

        sqlx::query(
            r#"
            INSERT INTO mikrotik_logs (id, tenant_id, router_id, logged_at, message, created_at, updated_at)
            SELECT 'old-sync-path-' || gs::text, $1, $2, $3, 'old log', $4, $4
            FROM generate_series(1, 5001) AS gs
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(old_logged_at)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed >5000 old logs");

        sqlx::query(
            r#"
            INSERT INTO mikrotik_logs (id, tenant_id, router_id, logged_at, message, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind("fresh-sync-path")
        .bind(tenant_id)
        .bind(router_id)
        .bind(fresh_logged_at)
        .bind("fresh log")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed fresh log");

        let result = service
            .sync_logs_for_router(tenant_id, router_id, 100)
            .await
            .expect("sync logs via service path");

        assert_eq!(
            result.seen, 0,
            "stubbed sync path should use empty fetched rows"
        );
        assert_eq!(
            result.upserted, 0,
            "stubbed sync path should not upsert new rows"
        );

        let remaining_old_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM mikrotik_logs WHERE tenant_id = $1 AND router_id = $2 AND id LIKE 'old-sync-path-%'",
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_one(&pool)
        .await
        .expect("count old logs after sync");

        let fresh_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM mikrotik_logs WHERE tenant_id = $1 AND router_id = $2 AND id = 'fresh-sync-path'",
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_one(&pool)
        .await
        .expect("count fresh logs after sync");

        assert_eq!(
            remaining_old_count, 0,
            "all old rows should be pruned via sync path"
        );
        assert_eq!(fresh_count, 1, "fresh row should be retained via sync path");

        sqlx::query("DELETE FROM mikrotik_logs WHERE tenant_id = $1 AND router_id = $2")
            .bind(tenant_id)
            .bind(router_id)
            .execute(&pool)
            .await
            .expect("cleanup logs");
        sqlx::query("DELETE FROM mikrotik_routers WHERE id = $1")
            .bind(router_id)
            .execute(&pool)
            .await
            .expect("cleanup router");
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await
            .expect("cleanup tenant");
    }

    #[tokio::test]
    async fn sync_logs_prunes_only_records_older_than_retention_cutoff() {
        let pool = test_pool().await;
        let mut tx = pool.begin().await.expect("begin tx");

        sqlx::query(
            r#"
            CREATE TEMP TABLE mikrotik_logs (
                id text PRIMARY KEY,
                tenant_id text NOT NULL,
                router_id text NOT NULL,
                logged_at timestamptz NOT NULL
            ) ON COMMIT DROP
            "#,
        )
        .execute(&mut *tx)
        .await
        .expect("create temp mikrotik_logs");

        let tenant_id = "tenant-sync-retention";
        let router_id = "router-sync-retention";
        let now = DateTime::parse_from_rfc3339("2026-03-31T00:00:00Z")
            .expect("valid now timestamp")
            .with_timezone(&Utc);
        let cutoff =
            mikrotik_log_retention_cutoff(now, Some(30)).expect("cutoff for 30d retention");

        sqlx::query(
            r#"
            INSERT INTO mikrotik_logs (id, tenant_id, router_id, logged_at)
            VALUES ($1, $2, $3, $4), ($5, $2, $3, $6), ($7, $2, $3, $8)
            "#,
        )
        .bind("old")
        .bind(tenant_id)
        .bind(router_id)
        .bind(cutoff - ChronoDuration::seconds(1))
        .bind("at_cutoff")
        .bind(cutoff)
        .bind("newer")
        .bind(cutoff + ChronoDuration::seconds(1))
        .execute(&mut *tx)
        .await
        .expect("seed logs around cutoff");

        sqlx::query(mikrotik_log_prune_sql())
            .bind(tenant_id)
            .bind(router_id)
            .bind(cutoff)
            .execute(&mut *tx)
            .await
            .expect("prune by retention cutoff");

        let remaining_ids: Vec<String> =
            sqlx::query_scalar("SELECT id FROM mikrotik_logs ORDER BY id ASC")
                .fetch_all(&mut *tx)
                .await
                .expect("fetch remaining ids");

        assert_eq!(
            remaining_ids,
            vec!["at_cutoff".to_string(), "newer".to_string()]
        );

        tx.rollback().await.expect("rollback tx");
    }

    #[tokio::test]
    async fn sync_logs_does_not_apply_fixed_5000_cap_anymore() {
        let pool = test_pool().await;
        let mut tx = pool.begin().await.expect("begin tx");

        sqlx::query(
            r#"
            CREATE TEMP TABLE mikrotik_logs (
                id text PRIMARY KEY,
                tenant_id text NOT NULL,
                router_id text NOT NULL,
                logged_at timestamptz NOT NULL
            ) ON COMMIT DROP
            "#,
        )
        .execute(&mut *tx)
        .await
        .expect("create temp mikrotik_logs");

        let tenant_id = "tenant-sync-no-cap";
        let router_id = "router-sync-no-cap";
        let now = DateTime::parse_from_rfc3339("2026-03-31T00:00:00Z")
            .expect("valid now timestamp")
            .with_timezone(&Utc);
        let cutoff =
            mikrotik_log_retention_cutoff(now, Some(30)).expect("cutoff for 30d retention");
        let older_than_cutoff = cutoff - ChronoDuration::seconds(1);

        sqlx::query(
            r#"
            INSERT INTO mikrotik_logs (id, tenant_id, router_id, logged_at)
            SELECT 'old-' || gs::text, $1, $2, $3
            FROM generate_series(1, 5001) AS gs
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(older_than_cutoff)
        .execute(&mut *tx)
        .await
        .expect("seed >5000 old logs");

        sqlx::query(
            "INSERT INTO mikrotik_logs (id, tenant_id, router_id, logged_at) VALUES ($1, $2, $3, $4)",
        )
        .bind("kept")
        .bind(tenant_id)
        .bind(router_id)
        .bind(cutoff)
        .execute(&mut *tx)
        .await
        .expect("seed cutoff log to keep");

        sqlx::query(mikrotik_log_prune_sql())
            .bind(tenant_id)
            .bind(router_id)
            .bind(cutoff)
            .execute(&mut *tx)
            .await
            .expect("prune old logs");

        let remaining_old_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM mikrotik_logs WHERE id LIKE 'old-%'")
                .fetch_one(&mut *tx)
                .await
                .expect("count remaining old logs");
        let kept_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM mikrotik_logs WHERE id = 'kept'")
                .fetch_one(&mut *tx)
                .await
                .expect("count kept logs");

        assert_eq!(
            remaining_old_count, 0,
            "all old rows should be pruned, not capped"
        );
        assert_eq!(kept_count, 1, "row at cutoff should remain");

        tx.rollback().await.expect("rollback tx");
    }

    #[tokio::test]
    async fn sync_logs_for_router_keeps_all_logs_when_retention_is_unlimited() {
        let pool = test_pool().await;
        let service = test_mikrotik_service(pool.clone());
        service.configure_test_sync_logs_injection(TestSyncLogsInjection {
            rows_override: Some(Vec::new()),
            force_prune_error: false,
        });

        let tenant_id = "tenant-sync-unlimited";
        let router_id = "router-sync-unlimited";
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO tenants (id, name, slug, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .bind("Sync Unlimited Tenant")
        .bind("sync-unlimited-tenant")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed tenant");

        sqlx::query(
            r#"
            INSERT INTO mikrotik_routers
              (id, tenant_id, name, host, port, username, password, use_tls, enabled, is_online, created_at, updated_at)
            VALUES
              ($1, $2, $3, $4, $5, $6, $7, false, true, false, $8, $9)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(router_id)
        .bind(tenant_id)
        .bind("Sync Unlimited Router")
        .bind("127.0.0.1")
        .bind(8728_i32)
        .bind("admin")
        .bind(test_router_password())
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed router");

        let old_logged_at = now - ChronoDuration::days(400);

        sqlx::query(
            r#"
            INSERT INTO mikrotik_logs (id, tenant_id, router_id, logged_at, message, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind("unlimited-old-log")
        .bind(tenant_id)
        .bind(router_id)
        .bind(old_logged_at)
        .bind("very old log")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed old log");

        service
            .sync_logs_for_router(tenant_id, router_id, 100)
            .await
            .expect("sync should not prune in unlimited mode");

        let kept_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM mikrotik_logs WHERE tenant_id = $1 AND router_id = $2 AND id = 'unlimited-old-log'",
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_one(&pool)
        .await
        .expect("count unlimited log");

        assert_eq!(kept_count, 1, "unlimited retention should keep old logs");

        sqlx::query("DELETE FROM mikrotik_logs WHERE tenant_id = $1 AND router_id = $2")
            .bind(tenant_id)
            .bind(router_id)
            .execute(&pool)
            .await
            .expect("cleanup logs");
        sqlx::query("DELETE FROM mikrotik_routers WHERE id = $1")
            .bind(router_id)
            .execute(&pool)
            .await
            .expect("cleanup router");
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await
            .expect("cleanup tenant");
    }

    #[tokio::test]
    async fn list_logs_supports_calendar_filters() {
        let pool = test_pool().await;
        let service = test_mikrotik_service(pool.clone());

        let tenant_id = "tenant-calendar-filters";
        let router_id = "router-calendar-filters";
        let base_now = DateTime::parse_from_rfc3339("2026-04-01T00:00:00Z")
            .expect("valid now timestamp")
            .with_timezone(&Utc);

        sqlx::query(
            r#"
            INSERT INTO tenants (id, name, slug, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .bind("Calendar Filter Tenant")
        .bind("calendar-filter-tenant")
        .bind(base_now)
        .bind(base_now)
        .execute(&pool)
        .await
        .expect("seed tenant");

        sqlx::query(
            r#"
            INSERT INTO mikrotik_routers
              (id, tenant_id, name, host, port, username, password, use_tls, enabled, is_online, created_at, updated_at)
            VALUES
              ($1, $2, $3, $4, $5, $6, $7, false, true, false, $8, $9)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(router_id)
        .bind(tenant_id)
        .bind("Calendar Filter Router")
        .bind("127.0.0.1")
        .bind(8728_i32)
        .bind("admin")
        .bind(test_router_password())
        .bind(base_now)
        .bind(base_now)
        .execute(&pool)
        .await
        .expect("seed router");

        sqlx::query(
            r#"
            INSERT INTO mikrotik_logs (id, tenant_id, router_id, logged_at, message, created_at, updated_at)
            VALUES
              ('log-2026-03', $1, $2, '2026-03-15T10:00:00Z', 'march log', $3, $3),
              ('log-2026-04', $1, $2, '2026-04-02T10:00:00Z', 'april log', $3, $3),
              ('log-2025-04', $1, $2, '2025-04-02T10:00:00Z', 'old april log', $3, $3)
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(base_now)
        .execute(&pool)
        .await
        .expect("seed calendar logs");

        let res = service
            .list_logs(
                tenant_id,
                Some(router_id.to_string()),
                None,
                None,
                None,
                Some(4),
                Some(2026),
                1,
                25,
                true,
            )
            .await
            .expect("list logs with month/year");

        assert_eq!(res.total, 1);
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].id, "log-2026-04");

        sqlx::query("DELETE FROM mikrotik_logs WHERE tenant_id = $1 AND router_id = $2")
            .bind(tenant_id)
            .bind(router_id)
            .execute(&pool)
            .await
            .expect("cleanup logs");
        sqlx::query("DELETE FROM mikrotik_routers WHERE id = $1")
            .bind(router_id)
            .execute(&pool)
            .await
            .expect("cleanup router");
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await
            .expect("cleanup tenant");
    }

    #[tokio::test]
    async fn mikrotik_log_queries_use_retention_indexes() {
        let pool = test_pool().await;
        let mut tx = pool.begin().await.expect("begin tx");

        sqlx::query("SET LOCAL enable_seqscan = off")
            .execute(&mut *tx)
            .await
            .expect("disable seqscan for deterministic planner assertions");

        let run_id = uuid::Uuid::new_v4().to_string();
        let tenant_id = format!("tenant-retention-index-plan-{run_id}");
        let router_id = format!("router-retention-index-plan-{run_id}");
        let tenant_slug = format!("retention-index-tenant-{run_id}");
        let now = Utc::now();
        let cutoff = now - ChronoDuration::days(90);

        sqlx::query(
            r#"
            INSERT INTO tenants (id, name, slug, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(&tenant_id)
        .bind("Retention Index Tenant")
        .bind(&tenant_slug)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .expect("seed tenant");

        sqlx::query(
            r#"
            INSERT INTO mikrotik_routers
              (id, tenant_id, name, host, port, username, password, use_tls, enabled, is_online, created_at, updated_at)
            VALUES
              ($1, $2, $3, $4, $5, $6, $7, false, true, false, $8, $9)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(&router_id)
        .bind(&tenant_id)
        .bind("Retention Index Router")
        .bind("127.0.0.1")
        .bind(8728_i32)
        .bind("admin")
        .bind(test_router_password())
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .expect("seed router");

        sqlx::query(
            r#"
            INSERT INTO mikrotik_logs (id, tenant_id, router_id, logged_at, message, created_at, updated_at)
            SELECT
              $5 || '-retention-plan-log-' || gs::text,
              $1,
              $2,
              $3 - make_interval(secs => gs::int),
              'retention plan log ' || gs::text,
              $4,
              $4
            FROM generate_series(1, 500) AS gs
            "#,
        )
        .bind(&tenant_id)
        .bind(&router_id)
        .bind(now)
        .bind(now)
        .bind(&run_id)
        .execute(&mut *tx)
        .await
        .expect("seed logs for planner checks");

        let prune_explain_sql = format!("EXPLAIN (FORMAT TEXT) {}", mikrotik_log_prune_sql());
        let prune_plan_rows: Vec<String> = sqlx::query_scalar(prune_explain_sql.as_str())
            .bind(&tenant_id)
            .bind(&router_id)
            .bind(cutoff)
            .fetch_all(&mut *tx)
            .await
            .expect("explain prune query");
        let prune_plan = prune_plan_rows.join("\n");

        let list_explain_sql = format!("EXPLAIN (FORMAT TEXT) {}", mikrotik_log_list_sql());
        let router_filter = Some(router_id.clone());
        let level_filter: Option<String> = None;
        let topic_filter: Option<String> = None;
        let search = String::new();
        let list_plan_rows: Vec<String> = sqlx::query_scalar(list_explain_sql.as_str())
            .bind(&tenant_id)
            .bind(&router_filter)
            .bind(&level_filter)
            .bind(&topic_filter)
            .bind(&search)
            .bind(25_i64)
            .bind(0_i64)
            .fetch_all(&mut *tx)
            .await
            .expect("explain paginated list query");
        let list_plan = list_plan_rows.join("\n");

        assert!(
            prune_plan.contains("idx_mikrotik_logs_tenant_router_logged_at"),
            "expected prune plan to use retention cutoff index, got:\n{prune_plan}"
        );
        assert!(
            list_plan.contains("idx_mikrotik_logs_tenant_router_logged_updated"),
            "expected list plan to use pagination ordering index, got:\n{list_plan}"
        );

        tx.rollback().await.expect("rollback tx");
    }
}
