use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::UpsertSettingDto;
use crate::services::SettingsService;
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct BackupService {
    pool: DbPool,
    app_data_dir: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BackupRecord {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub created_at: chrono::DateTime<Utc>,
    pub backup_type: String, // "global" or "tenant"
    pub tenant_id: Option<String>,
}

impl BackupService {
    pub fn new(pool: DbPool, app_data_dir: PathBuf) -> Self {
        Self { pool, app_data_dir }
    }

    fn get_backup_root_dir(&self) -> PathBuf {
        self.app_data_dir.join("backups")
    }

    fn get_global_backup_dir(&self) -> PathBuf {
        self.get_backup_root_dir().join("global")
    }

    fn get_tenant_backup_dir(&self, tenant_id: &str) -> PathBuf {
        self.get_backup_root_dir().join("tenants").join(tenant_id)
    }

    fn is_sensitive_setting_key(key: &str) -> bool {
        // Tenant backups should not contain credentials or API secrets.
        // Keep this narrow and explicit to avoid surprising data loss.
        let k = key.trim();
        if k.starts_with("email_") {
            return true;
        }
        if k.starts_with("payment_") {
            return true;
        }
        matches!(
            k,
            "storage_s3_access_key" | "storage_s3_secret_key" | "jwt_secret"
        )
    }

    fn redact_settings_rows(
        mut rows: Vec<serde_json::Map<String, serde_json::Value>>,
    ) -> Vec<serde_json::Map<String, serde_json::Value>> {
        for row in &mut rows {
            let key = row.get("key").and_then(|v| v.as_str()).unwrap_or("");
            if !Self::is_sensitive_setting_key(key) {
                continue;
            }

            // Keep row but clear values. This prevents secrets from being exfiltrated via backups.
            // After restore, admins must reconfigure these settings.
            row.insert(
                "value".to_string(),
                serde_json::Value::String(String::new()),
            );
        }
        rows
    }

    /// List all backups
    pub async fn list_backups(&self) -> AppResult<Vec<BackupRecord>> {
        let backup_root = self.get_backup_root_dir();
        if !backup_root.exists() {
            return Ok(vec![]);
        }

        let mut backups = Vec::new();
        let global_dir = self.get_global_backup_dir();
        if global_dir.exists() {
            let mut entries = fs::read_dir(&global_dir)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let metadata = entry
                    .metadata()
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                let name = entry.file_name().to_string_lossy().to_string();

                if !name.starts_with("global_backup_") {
                    continue;
                }

                backups.push(BackupRecord {
                    name,
                    path: path.to_string_lossy().to_string(),
                    size: metadata.len(),
                    created_at: metadata
                        .created()
                        .map(chrono::DateTime::from)
                        .unwrap_or(Utc::now()),
                    backup_type: "global".to_string(),
                    tenant_id: None,
                });
            }
        }

        let tenants_dir = self.get_backup_root_dir().join("tenants");
        if tenants_dir.exists() {
            let mut tenants = fs::read_dir(&tenants_dir)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            while let Ok(Some(tenant_entry)) = tenants.next_entry().await {
                let tenant_path = tenant_entry.path();
                if !tenant_path.is_dir() {
                    continue;
                }
                let tenant_id = tenant_entry.file_name().to_string_lossy().to_string();

                let mut entries = match fs::read_dir(&tenant_path).await {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let metadata = entry
                        .metadata()
                        .await
                        .map_err(|e| AppError::Internal(e.to_string()))?;
                    let name = entry.file_name().to_string_lossy().to_string();

                    let expected_prefix = format!("tenant_{}_", tenant_id);
                    if !name.starts_with(&expected_prefix) {
                        continue;
                    }

                    backups.push(BackupRecord {
                        name,
                        path: path.to_string_lossy().to_string(),
                        size: metadata.len(),
                        created_at: metadata
                            .created()
                            .map(chrono::DateTime::from)
                            .unwrap_or(Utc::now()),
                        backup_type: "tenant".to_string(),
                        tenant_id: Some(tenant_id.clone()),
                    });
                }
            }
        }

        // Sort by created_at desc
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }

    /// Perform a Global System Backup (Logical Export)
    pub async fn create_global_backup(&self) -> AppResult<String> {
        let backup_dir = self.get_global_backup_dir();
        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let zip_filename = format!("global_backup_{}.zip", timestamp);
        let zip_path = backup_dir.join(&zip_filename);

        // Global backups should include all core tables. If you add new tables, add them here.
        let tables = vec![
            "permissions",
            "plans",
            "features",
            "bank_accounts",
            "fx_rates",
            "tenants",
            "users",
            "roles",
            "settings",
            "plan_features",
            "tenant_subscriptions",
            "file_records",
            "invoices",
            "notifications",
            "tenant_members",
            "role_permissions",
            "trusted_devices",
            "notification_preferences",
            "push_subscriptions",
            "audit_logs",
            // Support
            "support_tickets",
            "support_ticket_messages",
            "support_ticket_attachments",
            // Announcements
            "announcements",
            "announcement_dismissals",
            // Email outbox
            "email_outbox",
        ];

        let mut data_map: std::collections::HashMap<String, serde_json::Value> =
            std::collections::HashMap::new();

        for table in tables {
            // Optimized: Use native DB JSON conversion if available
            #[cfg(feature = "postgres")]
            let query = format!("SELECT row_to_json(t) FROM (SELECT * FROM {}) t", table);

            #[cfg(feature = "sqlite")]
            let query = format!("SELECT * FROM {}", table);

            #[cfg(feature = "postgres")]
            {
                let rows: Vec<(serde_json::Value,)> = sqlx::query_as(&query)
                    .fetch_all(&self.pool)
                    .await
                    .unwrap_or_default();

                if !rows.is_empty() {
                    let json_rows: Vec<serde_json::Value> = rows.into_iter().map(|r| r.0).collect();
                    data_map.insert(
                        format!("{}.json", table),
                        serde_json::to_value(json_rows).unwrap(),
                    );
                }
            }

            #[cfg(feature = "sqlite")]
            {
                if let Ok(rows) = self.fetch_rows(&query, "", vec![]).await {
                    if !rows.is_empty() {
                        data_map.insert(
                            format!("{}.json", table),
                            serde_json::to_value(&rows).unwrap(),
                        );
                    }
                }
            }
        }

        // --- ZIP CREATION ---
        use std::io::Write;
        use zip::write::FileOptions;

        let file =
            std::fs::File::create(&zip_path).map_err(|e| AppError::Internal(e.to_string()))?;
        let mut zip = zip::ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for (filename, json_data) in data_map {
            zip.start_file(filename, options)
                .map_err(|e: zip::result::ZipError| AppError::Internal(e.to_string()))?;
            let json_str = serde_json::to_string_pretty(&json_data).unwrap_or_default();
            zip.write_all(json_str.as_bytes())
                .map_err(|e: std::io::Error| AppError::Internal(e.to_string()))?;
        }

        zip.finish()
            .map_err(|e: zip::result::ZipError| AppError::Internal(e.to_string()))?;

        info!("Global Backup successful: {:?}", zip_path);
        Ok(zip_path.to_string_lossy().to_string())
    }

    /// Perform a Tenant Backup (JSON Export)
    pub async fn create_tenant_backup(&self, tenant_id: &str) -> AppResult<String> {
        let backup_dir = self.get_tenant_backup_dir(tenant_id);
        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let zip_filename = format!("tenant_{}_{}.zip", tenant_id, timestamp);
        let zip_path = backup_dir.join(&zip_filename);

        // Prepare data collection
        let mut data_map: std::collections::HashMap<String, serde_json::Value> =
            std::collections::HashMap::new();

        // --- DATA EXPORT STRATEGY ---
        // NOTE: JSON filenames must match DB table names because restore derives table_name from file_stem.
        // Keep "tenant.json" as informational only (not part of restore_order).

        // NOTE: We intentionally do not export the `tenants` table for tenant backups.
        // Tenant restore never restores `tenants`, and exporting it can fail if the DB contains
        // invalid UTF-8 sequences in tenant metadata columns.

        // Tenant backup = tenant-owned data only (no global platform secrets).
        // This is intentionally not a full DB backup and is not meant to migrate global users.
        //
        // 2) Tenant-scoped tables (restore-safe)
        let settings_rows = self
            .fetch_rows(
                "SELECT * FROM settings WHERE tenant_id = ?",
                "SELECT * FROM settings WHERE tenant_id::text = $1",
                vec![tenant_id.to_string()],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to export settings: {}", e)))?;
        let settings_rows = Self::redact_settings_rows(settings_rows);
        data_map.insert(
            "settings.json".to_string(),
            serde_json::to_value(&settings_rows).unwrap(),
        );

        // NOTE: invoices are billing data (superadmin scope) — do not export in tenant backups.

        let file_rows = self
            .fetch_rows(
                "SELECT * FROM file_records WHERE tenant_id = ?",
                "SELECT * FROM file_records WHERE tenant_id::text = $1",
                vec![tenant_id.to_string()],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to export file_records: {}", e)))?;
        data_map.insert(
            "file_records.json".to_string(),
            serde_json::to_value(&file_rows).unwrap(),
        );

        let role_rows = self
            .fetch_rows(
                "SELECT * FROM roles WHERE tenant_id = ?",
                "SELECT * FROM roles WHERE tenant_id::text = $1",
                vec![tenant_id.to_string()],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to export roles: {}", e)))?;
        data_map.insert(
            "roles.json".to_string(),
            serde_json::to_value(&role_rows).unwrap(),
        );

        let member_rows = self
            .fetch_rows(
                "SELECT * FROM tenant_members WHERE tenant_id = ?",
                "SELECT * FROM tenant_members WHERE tenant_id::text = $1",
                vec![tenant_id.to_string()],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to export tenant_members: {}", e)))?;
        data_map.insert(
            "tenant_members.json".to_string(),
            serde_json::to_value(&member_rows).unwrap(),
        );

        let notifications_rows = self
            .fetch_rows(
                "SELECT * FROM notifications WHERE tenant_id = ?",
                "SELECT * FROM notifications WHERE tenant_id::text = $1",
                vec![tenant_id.to_string()],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to export notifications: {}", e)))?;
        data_map.insert(
            "notifications.json".to_string(),
            serde_json::to_value(&notifications_rows).unwrap(),
        );

        // tenant_subscriptions is billing/plan data (superadmin scope) — do not export in tenant backups.

        // Announcements (tenant-scoped only) + dismissals (join by tenant announcements)
        let ann_rows = self
            .fetch_rows(
                "SELECT * FROM announcements WHERE tenant_id = ?",
                "SELECT * FROM announcements WHERE tenant_id::text = $1",
                vec![tenant_id.to_string()],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to export announcements: {}", e)))?;
        data_map.insert(
            "announcements.json".to_string(),
            serde_json::to_value(&ann_rows).unwrap(),
        );

        let ann_dismiss_rows = self
            .fetch_rows(
                "SELECT ad.* FROM announcement_dismissals ad WHERE ad.announcement_id IN (SELECT a.id FROM announcements a WHERE a.tenant_id = ?)",
                "SELECT ad.* FROM announcement_dismissals ad WHERE ad.announcement_id IN (SELECT a.id FROM announcements a WHERE a.tenant_id::text = $1)",
                vec![tenant_id.to_string()],
            )
            .await
            .map_err(|e| {
                AppError::Internal(format!(
                    "Failed to export announcement_dismissals: {}",
                    e
                ))
            })?;
        data_map.insert(
            "announcement_dismissals.json".to_string(),
            serde_json::to_value(&ann_dismiss_rows).unwrap(),
        );

        // Support tickets + messages + attachments (join by tenant tickets)
        let ticket_rows = self
            .fetch_rows(
                "SELECT * FROM support_tickets WHERE tenant_id = ?",
                "SELECT * FROM support_tickets WHERE tenant_id::text = $1",
                vec![tenant_id.to_string()],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to export support_tickets: {}", e)))?;
        data_map.insert(
            "support_tickets.json".to_string(),
            serde_json::to_value(&ticket_rows).unwrap(),
        );

        let msg_rows = self
            .fetch_rows(
                "SELECT m.* FROM support_ticket_messages m WHERE m.ticket_id IN (SELECT t.id FROM support_tickets t WHERE t.tenant_id = ?)",
                "SELECT m.* FROM support_ticket_messages m WHERE m.ticket_id IN (SELECT t.id FROM support_tickets t WHERE t.tenant_id::text = $1)",
                vec![tenant_id.to_string()],
            )
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to export support_ticket_messages: {}", e))
            })?;
        data_map.insert(
            "support_ticket_messages.json".to_string(),
            serde_json::to_value(&msg_rows).unwrap(),
        );

        let att_rows = self
            .fetch_rows(
                "SELECT a.* FROM support_ticket_attachments a WHERE a.message_id IN (SELECT m.id FROM support_ticket_messages m JOIN support_tickets t ON t.id = m.ticket_id WHERE t.tenant_id = ?)",
                "SELECT a.* FROM support_ticket_attachments a WHERE a.message_id IN (SELECT m.id FROM support_ticket_messages m JOIN support_tickets t ON t.id = m.ticket_id WHERE t.tenant_id::text = $1)",
                vec![tenant_id.to_string()],
            )
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to export support_ticket_attachments: {}", e))
            })?;
        data_map.insert(
            "support_ticket_attachments.json".to_string(),
            serde_json::to_value(&att_rows).unwrap(),
        );

        // notification_preferences is user-scoped (no tenant_id); filter by tenant members
        let notif_prefs_rows = self
            .fetch_rows(
                "SELECT np.* FROM notification_preferences np WHERE np.user_id IN (SELECT tm.user_id FROM tenant_members tm WHERE tm.tenant_id = ?)",
                "SELECT np.* FROM notification_preferences np WHERE np.user_id IN (SELECT tm.user_id FROM tenant_members tm WHERE tm.tenant_id::text = $1)",
                vec![tenant_id.to_string()],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to export notification_preferences: {}", e)))?;
        data_map.insert(
            "notification_preferences.json".to_string(),
            serde_json::to_value(&notif_prefs_rows).unwrap(),
        );

        // push_subscriptions is user-scoped (no tenant_id); filter by tenant members
        let push_sub_rows = self
            .fetch_rows(
                "SELECT ps.* FROM push_subscriptions ps WHERE ps.user_id IN (SELECT tm.user_id FROM tenant_members tm WHERE tm.tenant_id = ?)",
                "SELECT ps.* FROM push_subscriptions ps WHERE ps.user_id IN (SELECT tm.user_id FROM tenant_members tm WHERE tm.tenant_id::text = $1)",
                vec![tenant_id.to_string()],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to export push_subscriptions: {}", e)))?;
        data_map.insert(
            "push_subscriptions.json".to_string(),
            serde_json::to_value(&push_sub_rows).unwrap(),
        );

        // 3) Tenant role_permissions (no tenant_id column; derive by tenant roles)
        let role_permissions_rows = self
            .fetch_rows(
                "SELECT rp.* FROM role_permissions rp WHERE rp.role_id IN (SELECT id FROM roles WHERE tenant_id = ?)",
                "SELECT rp.* FROM role_permissions rp WHERE rp.role_id IN (SELECT id FROM roles WHERE tenant_id::text = $1)",
                vec![tenant_id.to_string()],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to export role_permissions: {}", e)))?;
        data_map.insert(
            "role_permissions.json".to_string(),
            serde_json::to_value(&role_permissions_rows).unwrap(),
        );

        // --- ZIP CREATION ---
        use std::io::Write;
        use zip::write::FileOptions;

        let file =
            std::fs::File::create(&zip_path).map_err(|e| AppError::Internal(e.to_string()))?;
        let mut zip = zip::ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for (filename, json_data) in data_map {
            zip.start_file(filename, options)
                .map_err(|e: zip::result::ZipError| AppError::Internal(e.to_string()))?;
            let json_str = serde_json::to_string_pretty(&json_data).unwrap_or_default();
            zip.write_all(json_str.as_bytes())
                .map_err(|e: std::io::Error| AppError::Internal(e.to_string()))?;
        }

        zip.finish()
            .map_err(|e: zip::result::ZipError| AppError::Internal(e.to_string()))?;

        info!("Tenant Backup successful: {:?}", zip_path);
        Ok(zip_path.to_string_lossy().to_string())
    }

    // Helper to fetch generic rows as JSON
    async fn fetch_rows(
        &self,
        _sqlite_sql: &str,
        _pg_sql: &str,
        params: Vec<String>,
    ) -> AppResult<Vec<serde_json::Map<String, serde_json::Value>>> {
        #[cfg(feature = "postgres")]
        {
            // Postgres has row_to_json but let's just fetch generic and map manually if needed or use sqlx::Row
            // Actually, sqlx::Row isn't easily serializable to JSON without knowing schema.
            // For a generic backup, we might just want to use `row_to_json`

            // Reconstruct query to return single JSON column
            // "SELECT row_to_json(t) FROM (SELECT * FROM table WHERE ...) t"
            // This is complex to rewrite generic SQL.

            // Simplified approach: Serialize known structs?
            // No, we want generic backup.

            // Let's use `sqlx::query` and iterate columns.
            use sqlx::{Column, Row};

            let mut query = sqlx::query(_pg_sql);
            for p in &params {
                // Keep params as text; for Postgres UUID columns, callers should use explicit casts
                // (e.g. `WHERE id::text = $1`) to avoid `uuid = text` / `text = uuid` operator errors.
                query = query.bind(p);
            }

            let rows = query
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let mut results = Vec::new();

            for row in rows {
                let mut map = serde_json::Map::new();
                for col in row.columns() {
                    let name = col.name();
                    // Attempt to decode common types
                    // This is hacky. A proper export tool should be used.
                    // But for this "Backup System" feature request, this is a reasonable "Logic" implementation.

                    // Fallback to string representation for most things
                    let val_str: Option<String> = row.try_get(name).ok();
                    if let Some(s) = val_str {
                        // Avoid restore errors caused by embedded NUL bytes in text.
                        let cleaned = s.replace('\u{0000}', "");
                        map.insert(name.to_string(), serde_json::Value::String(cleaned));
                        continue;
                    }

                    let val_int: Option<i64> = row.try_get(name).ok();
                    if let Some(i) = val_int {
                        map.insert(
                            name.to_string(),
                            serde_json::Value::Number(serde_json::Number::from(i)),
                        );
                        continue;
                    }

                    #[cfg(feature = "postgres")]
                    {
                        let val_decimal: Option<sqlx::types::BigDecimal> = row.try_get(name).ok();
                        if let Some(d) = val_decimal {
                            map.insert(name.to_string(), serde_json::Value::String(d.to_string()));
                            continue;
                        }
                    }

                    let val_float: Option<f64> = row.try_get(name).ok();
                    if let Some(f) = val_float {
                        if let Some(num) = serde_json::Number::from_f64(f) {
                            map.insert(name.to_string(), serde_json::Value::Number(num));
                        } else {
                            map.insert(name.to_string(), serde_json::Value::String(f.to_string()));
                        }
                        continue;
                    }

                    let val_bool: Option<bool> = row.try_get(name).ok();
                    if let Some(b) = val_bool {
                        map.insert(name.to_string(), serde_json::Value::Bool(b));
                        continue;
                    }

                    map.insert(name.to_string(), serde_json::Value::Null);
                }
                results.push(map);
            }
            Ok(results)
        }

        #[cfg(feature = "sqlite")]
        {
            use sqlx::{Column, Row};
            let mut query = sqlx::query(_sqlite_sql);
            for p in &params {
                query = query.bind(p);
            }
            let rows = query
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let mut results = Vec::new();

            for row in rows {
                let mut map = serde_json::Map::new();
                for col in row.columns() {
                    let name = col.name();
                    // SQLite is loosely typed, most things can be strings
                    let val_str: Option<String> = row.try_get(name).ok();
                    if let Some(s) = val_str {
                        map.insert(name.to_string(), serde_json::Value::String(s));
                        continue;
                    }
                    let val_int: Option<i64> = row.try_get(name).ok();
                    if let Some(i) = val_int {
                        map.insert(
                            name.to_string(),
                            serde_json::Value::Number(serde_json::Number::from(i)),
                        );
                        continue;
                    }

                    #[cfg(feature = "postgres")]
                    {
                        let val_decimal: Option<sqlx::types::BigDecimal> = row.try_get(name).ok();
                        if let Some(d) = val_decimal {
                            map.insert(name.to_string(), serde_json::Value::String(d.to_string()));
                            continue;
                        }
                    }

                    let val_float: Option<f64> = row.try_get(name).ok();
                    if let Some(f) = val_float {
                        if let Some(num) = serde_json::Number::from_f64(f) {
                            map.insert(name.to_string(), serde_json::Value::Number(num));
                        } else {
                            map.insert(name.to_string(), serde_json::Value::String(f.to_string()));
                        }
                        continue;
                    }

                    map.insert(name.to_string(), serde_json::Value::Null);
                }
                results.push(map);
            }
            Ok(results)
        }
    }

    /// Get absolute path to a backup file, validating its name to prevent directory traversal
    pub fn get_backup_path(&self, filename: &str) -> AppResult<PathBuf> {
        // Basic validation: allow alphanumeric, underscores, dashes, and dots
        if !filename
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
        {
            return Err(AppError::Validation(
                "Invalid characters in filename".to_string(),
            ));
        }

        // Prevent directory traversal
        if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
            return Err(AppError::Validation("Invalid filename format".to_string()));
        }

        let file_path = if filename.starts_with("global_backup_") {
            self.get_global_backup_dir().join(filename)
        } else if filename.starts_with("tenant_") {
            let parts: Vec<&str> = filename.split('_').collect();
            if parts.len() < 3 {
                return Err(AppError::Validation(
                    "Invalid tenant backup filename".to_string(),
                ));
            }
            let tenant_id = parts[1];
            self.get_tenant_backup_dir(tenant_id).join(filename)
        } else {
            return Err(AppError::Validation("Unknown backup filename".to_string()));
        };

        if !file_path.exists() {
            return Err(AppError::NotFound(format!(
                "Backup file {} not found",
                filename
            )));
        }

        Ok(file_path)
    }

    pub async fn delete_backup(&self, filename: String) -> AppResult<()> {
        let path = self.get_backup_path(&filename)?;
        fs::remove_file(path)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Restore system or tenant data from a ZIP backup file
    pub async fn restore_from_zip(
        &self,
        zip_path: PathBuf,
        target_tenant_id: Option<&str>,
    ) -> AppResult<()> {
        info!("Starting restore from {:?}", zip_path);

        // 1. Read everything into memory first
        let mut table_data: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        {
            let file =
                std::fs::File::open(&zip_path).map_err(|e| AppError::Internal(e.to_string()))?;
            let mut archive =
                zip::ZipArchive::new(file).map_err(|e| AppError::Internal(e.to_string()))?;

            for i in 0..archive.len() {
                let mut file = archive
                    .by_index(i)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                let outpath = match file.enclosed_name() {
                    Some(path) => path.to_owned(),
                    None => continue,
                };

                if !outpath.to_string_lossy().ends_with(".json") {
                    continue;
                }

                let table_name = outpath.file_stem().unwrap().to_string_lossy().to_string();

                let mut contents = String::new();
                use std::io::Read;
                file.read_to_string(&mut contents)
                    .map_err(|e| AppError::Internal(e.to_string()))?;

                table_data.insert(table_name, contents);
            }
        }

        // 2. Define Restoration Order (Foreign Key Hierarchy)
        let restore_order = vec![
            "permissions",
            "features",
            "plans",
            "bank_accounts",
            "fx_rates",
            "tenants",
            "users",
            "roles",
            "settings",
            "plan_features",
            "tenant_subscriptions",
            "file_records",
            "invoices",
            "notifications",
            "tenant_members",
            "role_permissions",
            "trusted_devices",
            "notification_preferences",
            "push_subscriptions",
            // Announcements
            "announcements",
            "announcement_dismissals",
            // Support
            "support_tickets",
            "support_ticket_messages",
            "support_ticket_attachments",
            // Outbox (global/admin tools)
            "email_outbox",
            "audit_logs",
        ];

        let tenant_skip: std::collections::HashSet<&str> = if target_tenant_id.is_some() {
            [
                "permissions",
                "features",
                "plans",
                "plan_features",
                "bank_accounts",
                "fx_rates",
                "tenants",
                "users",
                "tenant_subscriptions",
                "invoices",
                "trusted_devices",
                "email_outbox",
            ]
            .into_iter()
            .collect()
        } else {
            std::collections::HashSet::new()
        };

        // If this is a tenant restore, pre-compute allowed role_ids and user_ids for validation
        let allowed_role_ids: std::collections::HashSet<String> = if let Some(tid) =
            target_tenant_id
        {
            if let Some(contents) = table_data.get("roles") {
                let rows: Vec<serde_json::Map<String, serde_json::Value>> =
                    serde_json::from_str(contents)
                        .map_err(|e| AppError::Internal(format!("Invalid JSON in roles: {}", e)))?;
                rows.into_iter()
                    .filter_map(|r| {
                        let tenant_ok = r
                            .get("tenant_id")
                            .and_then(|v| v.as_str())
                            .map(|s| s == tid)
                            .unwrap_or(false);
                        if !tenant_ok {
                            return None;
                        }
                        r.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
                    })
                    .collect()
            } else {
                std::collections::HashSet::new()
            }
        } else {
            std::collections::HashSet::new()
        };

        let allowed_user_ids: std::collections::HashSet<String> = if target_tenant_id.is_some() {
            if let Some(contents) = table_data.get("tenant_members") {
                let rows: Vec<serde_json::Map<String, serde_json::Value>> =
                    serde_json::from_str(contents).map_err(|e| {
                        AppError::Internal(format!("Invalid JSON in tenant_members: {}", e))
                    })?;
                rows.into_iter()
                    .filter_map(|r| {
                        r.get("user_id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            } else {
                std::collections::HashSet::new()
            }
        } else {
            std::collections::HashSet::new()
        };

        // Tenant restore requires the referenced users to exist (tenant_members.user_id is NOT NULL + FK).
        // We intentionally do not include global users in tenant backups for safety.
        if let Some(tid) = target_tenant_id {
            if !allowed_user_ids.is_empty() {
                #[cfg(feature = "postgres")]
                {
                    let ids: Vec<String> = allowed_user_ids.iter().cloned().collect();
                    let existing: Vec<String> =
                        sqlx::query_scalar("SELECT id FROM users WHERE id = ANY($1)")
                            .bind(&ids)
                            .fetch_all(&self.pool)
                            .await
                            .unwrap_or_default();
                    let existing: std::collections::HashSet<String> =
                        existing.into_iter().collect();
                    let missing: Vec<String> = allowed_user_ids
                        .iter()
                        .filter(|id| !existing.contains(*id))
                        .take(10)
                        .cloned()
                        .collect();
                    if !missing.is_empty() {
                        return Err(AppError::Validation(format!(
                            "Tenant restore blocked: {} user(s) referenced by tenant_members are missing in this database (example: {}). Create/import users first, then restore again.",
                            allowed_user_ids.len().saturating_sub(existing.len()),
                            missing.join(", ")
                        )));
                    }
                }

                #[cfg(feature = "sqlite")]
                {
                    // SQLite mode: best-effort check
                    for uid in allowed_user_ids.iter().take(50) {
                        let exists: Option<String> =
                            sqlx::query_scalar("SELECT id FROM users WHERE id = ?")
                                .bind(uid)
                                .fetch_optional(&self.pool)
                                .await
                                .unwrap_or(None);
                        if exists.is_none() {
                            return Err(AppError::Validation(format!(
                                "Tenant restore blocked: user {} referenced by tenant_members is missing in this database. Create/import users first, then restore again.",
                                uid
                            )));
                        }
                    }
                }
            }

            // Ensure tenant_id enforcement is always applied consistently later.
            let _ = tid;
        }

        // 3. Start database operations
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // 4. CLEANUP (Reverse Order)
        if target_tenant_id.is_none() {
            // Global Cleanup
            for table_name in restore_order.iter().rev() {
                let trunc_query = format!("DELETE FROM {}", table_name);
                #[cfg(feature = "postgres")]
                sqlx::query(&trunc_query).execute(&mut *tx).await.ok();

                #[cfg(feature = "sqlite")]
                sqlx::query(&trunc_query).execute(&mut *tx).await.ok();
            }
            // Cleanup sessions separately
            sqlx::query("DELETE FROM sessions")
                .execute(&mut *tx)
                .await
                .ok();
        }

        // 5. RESTORE (In Order)
        for table_name in restore_order {
            if tenant_skip.contains(table_name) {
                continue;
            }
            if let Some(contents) = table_data.get(table_name) {
                info!("Restoring table: {}", table_name);

                let rows: Vec<serde_json::Map<String, serde_json::Value>> =
                    serde_json::from_str(contents).map_err(|e| {
                        AppError::Internal(format!("Invalid JSON in {}: {}", table_name, e))
                    })?;

                if rows.is_empty() {
                    continue;
                }

                if let Some(tid) = target_tenant_id {
                    // Tenant-specific cleanup for this table
                    let tenant_tables_with_tenant_id = [
                        "settings",
                        "invoices",
                        "file_records",
                        "audit_logs",
                        "roles",
                        "tenant_members",
                        "notifications",
                        "announcements",
                        "support_tickets",
                    ];

                    let tenant_tables_without_tenant_id = ["role_permissions"];

                    let tenant_tables_user_scoped =
                        ["notification_preferences", "push_subscriptions"];
                    let tenant_tables_join_scoped = [
                        "announcement_dismissals",
                        "support_ticket_messages",
                        "support_ticket_attachments",
                    ];

                    if tenant_tables_with_tenant_id.contains(&table_name) {
                        #[cfg(feature = "postgres")]
                        let del_query =
                            format!("DELETE FROM {} WHERE tenant_id::text = $1", table_name);
                        #[cfg(feature = "sqlite")]
                        let del_query = format!("DELETE FROM {} WHERE tenant_id = ?", table_name);
                        sqlx::query(&del_query).bind(tid).execute(&mut *tx).await?;
                    } else if tenant_tables_without_tenant_id.contains(&table_name) {
                        // role_permissions is scoped by roles; deleting roles will cascade, but keep this explicit.
                        #[cfg(feature = "postgres")]
                        let del_query = "DELETE FROM role_permissions WHERE role_id IN (SELECT id FROM roles WHERE tenant_id::text = $1)";
                        #[cfg(feature = "sqlite")]
                        let del_query = "DELETE FROM role_permissions WHERE role_id IN (SELECT id FROM roles WHERE tenant_id = ?)";
                        sqlx::query(del_query).bind(tid).execute(&mut *tx).await?;
                    } else if tenant_tables_user_scoped.contains(&table_name) {
                        // These tables are user-scoped; delete only for users that belong to this tenant.
                        #[cfg(feature = "postgres")]
                        let del_query = format!(
                            "DELETE FROM {} WHERE user_id IN (SELECT tm.user_id FROM tenant_members tm WHERE tm.tenant_id::text = $1)",
                            table_name
                        );
                        #[cfg(feature = "sqlite")]
                        let del_query = format!(
                            "DELETE FROM {} WHERE user_id IN (SELECT tm.user_id FROM tenant_members tm WHERE tm.tenant_id = ?)",
                            table_name
                        );
                        sqlx::query(&del_query).bind(tid).execute(&mut *tx).await?;
                    } else if tenant_tables_join_scoped.contains(&table_name) {
                        // Tables without tenant_id, but scoped through a tenant-owned parent.
                        #[cfg(feature = "postgres")]
                        let del_query = match table_name {
                            "announcement_dismissals" => {
                                "DELETE FROM announcement_dismissals WHERE announcement_id IN (SELECT id FROM announcements WHERE tenant_id::text = $1)"
                            }
                            "support_ticket_messages" => {
                                "DELETE FROM support_ticket_messages WHERE ticket_id IN (SELECT id FROM support_tickets WHERE tenant_id::text = $1)"
                            }
                            "support_ticket_attachments" => {
                                "DELETE FROM support_ticket_attachments WHERE message_id IN (SELECT m.id FROM support_ticket_messages m JOIN support_tickets t ON t.id = m.ticket_id WHERE t.tenant_id::text = $1)"
                            }
                            _ => "",
                        };
                        #[cfg(feature = "sqlite")]
                        let del_query = match table_name {
                            "announcement_dismissals" => {
                                "DELETE FROM announcement_dismissals WHERE announcement_id IN (SELECT id FROM announcements WHERE tenant_id = ?)"
                            }
                            "support_ticket_messages" => {
                                "DELETE FROM support_ticket_messages WHERE ticket_id IN (SELECT id FROM support_tickets WHERE tenant_id = ?)"
                            }
                            "support_ticket_attachments" => {
                                "DELETE FROM support_ticket_attachments WHERE message_id IN (SELECT m.id FROM support_ticket_messages m JOIN support_tickets t ON t.id = m.ticket_id WHERE t.tenant_id = ?)"
                            }
                            _ => "",
                        };
                        if !del_query.is_empty() {
                            sqlx::query(del_query).bind(tid).execute(&mut *tx).await?;
                        }
                    } else {
                        continue;
                    }
                }

                // Insert rows
                fn is_time_col(name: &str) -> bool {
                    let lower = name.to_lowercase();
                    lower.ends_with("_at")
                        || lower.ends_with("_date")
                        || lower.contains("date")
                        || lower.contains("time")
                        || lower.contains("createdat")
                        || lower.contains("updatedat")
                        || lower.contains("expires")
                        || lower.contains("locked_until")
                        || lower.contains("trial_ends_at")
                        || lower.contains("current_period_start")
                        || lower.contains("current_period_end")
                }

                for mut row in rows {
                    if let Some(tid) = target_tenant_id {
                        // Enforce tenant isolation:
                        // - For tenant_id tables: force tenant_id = tid (or set it if missing)
                        // - For role_permissions: ensure role_id belongs to tenant roles from this backup
                        if row.contains_key("tenant_id") {
                            row.insert(
                                "tenant_id".to_string(),
                                serde_json::Value::String(tid.to_string()),
                            );
                        }

                        if table_name == "role_permissions" {
                            let role_id = row.get("role_id").and_then(|v| v.as_str()).unwrap_or("");
                            if role_id.is_empty() || !allowed_role_ids.contains(role_id) {
                                continue;
                            }
                        }

                        if table_name == "notification_preferences"
                            || table_name == "push_subscriptions"
                        {
                            let user_id = row.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
                            if user_id.is_empty() || !allowed_user_ids.contains(user_id) {
                                continue;
                            }
                        }

                        // Support ticket FK hygiene:
                        // Tenant backups don't include global users. If a ticket/message references a user
                        // outside this tenant restore scope, null it out so inserts don't fail.
                        if table_name == "support_tickets" {
                            for col in ["created_by", "assigned_to"] {
                                let uid = row.get(col).and_then(|v| v.as_str()).unwrap_or("");
                                if !uid.is_empty() && !allowed_user_ids.contains(uid) {
                                    row.insert(col.to_string(), serde_json::Value::Null);
                                }
                            }
                        } else if table_name == "support_ticket_messages" {
                            let uid = row.get("author_id").and_then(|v| v.as_str()).unwrap_or("");
                            if !uid.is_empty() && !allowed_user_ids.contains(uid) {
                                row.insert("author_id".to_string(), serde_json::Value::Null);
                            }
                        }
                    }

                    let mut col_names = Vec::new();
                    let mut placeholders = Vec::new();
                    let mut values = Vec::new();

                    #[cfg(feature = "postgres")]
                    for (idx, (key, val)) in row.into_iter().enumerate() {
                        col_names.push(key);
                        // If this is a timestamp column and value is a string, cast placeholder to timestamptz
                        let needs_ts_cast = matches!(val, serde_json::Value::String(_))
                            && is_time_col(col_names.last().unwrap());
                        if needs_ts_cast {
                            placeholders.push(format!("${}::timestamptz", idx + 1));
                        } else {
                            placeholders.push(format!("${}", idx + 1));
                        }
                        values.push(val);
                    }

                    #[cfg(feature = "sqlite")]
                    for (key, val) in row.into_iter() {
                        col_names.push(key);
                        placeholders.push("?".to_string());
                        values.push(val);
                    }

                    let ins_query = format!(
                        "INSERT INTO {} ({}) VALUES ({})",
                        table_name,
                        col_names
                            .iter()
                            .map(|c| format!("\"{}\"", c.replace('\"', "\"\"")))
                            .collect::<Vec<_>>()
                            .join(", "),
                        placeholders.join(", ")
                    );

                    let debug_vals: Vec<String> = values
                        .iter()
                        .map(|v| match v {
                            serde_json::Value::String(s) => {
                                if s.len() > 80 {
                                    format!("\"{}...\"", &s[..80])
                                } else {
                                    format!("\"{}\"", s)
                                }
                            }
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            serde_json::Value::Null => "null".to_string(),
                            other => {
                                let raw = other.to_string();
                                if raw.len() > 80 {
                                    format!("{}...", &raw[..80])
                                } else {
                                    raw
                                }
                            }
                        })
                        .collect();

                    // per-row savepoint to allow skipping bad rows without aborting transaction
                    sqlx::query("SAVEPOINT row_save")
                        .execute(&mut *tx)
                        .await
                        .ok();
                    let mut q = sqlx::query(&ins_query);
                    fn sanitize_text_for_db(s: &str) -> String {
                        // Postgres TEXT/VARCHAR cannot contain NUL (0x00). Some legacy backups or
                        // accidental binary payloads may include it; strip so restore can continue.
                        s.chars().filter(|c| *c != '\u{0}').collect()
                    }

                    fn numeric_kind(name: &str) -> Option<&'static str> {
                        let lower = name.to_lowercase();
                        if lower.contains("size")
                            || lower.contains("count")
                            || lower.contains("qty")
                            || lower.contains("quantity")
                            || lower.contains("usage")
                        {
                            Some("int")
                        } else if lower.contains("amount")
                            || lower.contains("price")
                            || lower.contains("rate")
                            || lower.contains("total")
                            || lower.contains("balance")
                        {
                            Some("float")
                        } else {
                            None
                        }
                    }

                    fn is_json_col(name: &str) -> bool {
                        let lc = name.to_lowercase();
                        lc == "feature_overrides"
                            || lc.ends_with("_json")
                            || lc.ends_with("_jsonb")
                            || lc.contains("json")
                            || lc.contains("metadata")
                            || lc.contains("payload")
                    }

                    fn is_uuid_col(table: &str, col: &str) -> bool {
                        // In our schema, almost all IDs are stored as TEXT for cross-DB compatibility.
                        // The only UUID-typed columns are in `audit_logs`.
                        if table != "audit_logs" {
                            return false;
                        }
                        matches!(col.to_lowercase().as_str(), "id" | "user_id" | "tenant_id")
                    }

                    for (col_name, v) in col_names.iter().zip(values.into_iter()) {
                        match v {
                            serde_json::Value::String(s) => {
                                let s = sanitize_text_for_db(&s);
                                let mut bound = false;
                                let is_uuid = is_uuid_col(table_name, col_name);
                                let is_id = col_name.eq_ignore_ascii_case("id");

                                // Handle string "null"/empty early
                                if s == "null" || s.is_empty() {
                                    if is_uuid {
                                        if is_id {
                                            q = q.bind(uuid::Uuid::new_v4());
                                        } else {
                                            q = q.bind(None::<uuid::Uuid>);
                                        }
                                        bound = true;
                                    } else if is_json_col(col_name) {
                                        q = q.bind(sqlx::types::Json(serde_json::Value::Object(
                                            serde_json::Map::new(),
                                        )));
                                        bound = true;
                                    } else if table_name == "settings"
                                        && col_name.eq_ignore_ascii_case("value")
                                    {
                                        // Settings.value is NOT NULL; keep empty string
                                        q = q.bind("");
                                        bound = true;
                                    } else if is_time_col(col_name) {
                                        q = q.bind(chrono::Utc::now());
                                        bound = true;
                                    } else if let Some(kind) = numeric_kind(col_name) {
                                        if kind == "int" {
                                            q = q.bind(0i64);
                                        } else {
                                            q = q.bind(0f64);
                                        }
                                        bound = true;
                                    } else {
                                        q = q.bind(None::<String>);
                                        bound = true;
                                    }
                                }

                                // 1) UUID (Only for Postgres)
                                #[cfg(feature = "postgres")]
                                if !bound && is_uuid && s.len() == 36 && s.contains('-') {
                                    if let Ok(u) = uuid::Uuid::parse_str(&s) {
                                        q = q.bind(u);
                                        bound = true;
                                    }
                                }

                                // 1.5) JSON columns
                                if !bound && is_json_col(col_name) {
                                    let trimmed = s.trim();
                                    let json_val =
                                        if trimmed.starts_with('{') || trimmed.starts_with('[') {
                                            serde_json::from_str::<serde_json::Value>(trimmed)
                                                .unwrap_or_else(|_| {
                                                    serde_json::Value::String(s.clone())
                                                })
                                        } else {
                                            serde_json::Value::String(s.clone())
                                        };
                                    q = q.bind(sqlx::types::Json(json_val));
                                    bound = true;
                                }

                                // Timestamp-ish columns:
                                // We generate `$N::timestamptz` placeholders for string timestamp columns,
                                // so binding the raw string is sufficient (and avoids binary format pitfalls).
                                if !bound && is_time_col(col_name) {
                                    q = q.bind(s.clone());
                                    bound = true;
                                }

                                if !bound {
                                    if let Some(kind) = numeric_kind(col_name) {
                                        if let Ok(i) = s.parse::<i64>() {
                                            q = q.bind(i);
                                            bound = true;
                                        } else if let Ok(f) = s.parse::<f64>() {
                                            q = q.bind(f);
                                            bound = true;
                                        } else if s == "null" || s.is_empty() {
                                            if kind == "int" {
                                                q = q.bind(0i64);
                                            } else {
                                                q = q.bind(0f64);
                                            }
                                            bound = true;
                                        }
                                    }
                                }

                                if !bound {
                                    q = q.bind(s);
                                }
                            }
                            serde_json::Value::Number(n) => {
                                if let Some(i) = n.as_i64() {
                                    q = q.bind(i);
                                } else if let Some(f) = n.as_f64() {
                                    q = q.bind(f);
                                }
                            }
                            serde_json::Value::Bool(b) => q = q.bind(b),
                            serde_json::Value::Null => {
                                if is_uuid_col(table_name, col_name) {
                                    if col_name.eq_ignore_ascii_case("id") {
                                        q = q.bind(uuid::Uuid::new_v4());
                                    } else {
                                        q = q.bind(None::<uuid::Uuid>);
                                    }
                                } else if is_json_col(col_name) {
                                    q = q.bind(sqlx::types::Json(serde_json::Value::Object(
                                        serde_json::Map::new(),
                                    )));
                                } else if is_time_col(col_name) {
                                    // For required timestamp columns, fall back to now() if value is null
                                    q = q.bind(chrono::Utc::now());
                                } else if let Some(kind) = numeric_kind(col_name) {
                                    // For numeric columns, fall back to 0 on null to avoid type mismatch / NOT NULL errors
                                    if kind == "int" {
                                        q = q.bind(0i64);
                                    } else {
                                        q = q.bind(0f64);
                                    }
                                } else if col_name.eq_ignore_ascii_case("value")
                                    && table_name == "settings"
                                {
                                    // Settings.value is NOT NULL; fallback to empty string
                                    q = q.bind("");
                                } else {
                                    q = q.bind(None::<String>);
                                }
                            }
                            serde_json::Value::Array(arr) => {
                                if is_json_col(col_name) {
                                    q = q.bind(sqlx::types::Json(serde_json::Value::Array(arr)));
                                } else {
                                    q = q.bind(serde_json::Value::Array(arr).to_string());
                                }
                            }
                            serde_json::Value::Object(map) => {
                                if is_json_col(col_name) {
                                    q = q.bind(sqlx::types::Json(serde_json::Value::Object(map)));
                                } else {
                                    q = q.bind(serde_json::Value::Object(map).to_string());
                                }
                            }
                        }
                    }
                    if let Err(e) = q.execute(&mut *tx).await {
                        let err_str = e.to_string();
                        if err_str.contains("invalid byte sequence for encoding \"UTF8\"") {
                            // Most commonly this is a NUL byte (0x00) in a text field. We attempt
                            // to sanitize strings, but keep this as a final safety net.
                            warn!(
                                "Skipping row due to invalid text bytes: table={} cols={:?} vals={:?} err={}",
                                table_name,
                                col_names,
                                debug_vals,
                                e
                            );
                            // reset transaction state so subsequent inserts can continue
                            sqlx::query("ROLLBACK TO SAVEPOINT row_save")
                                .execute(&mut *tx)
                                .await
                                .ok();
                            continue;
                        }
                        if table_name == "file_records"
                            && err_str.contains("file_records_tenant_id_fkey")
                            && target_tenant_id.is_some()
                        {
                            error!(
                                "Skipping file_records row due to FK: table={} cols={:?} vals={:?} err={}",
                                table_name,
                                col_names,
                                debug_vals,
                                e
                            );
                            sqlx::query("ROLLBACK TO SAVEPOINT row_save")
                                .execute(&mut *tx)
                                .await
                                .ok();
                            continue;
                        }

                        error!(
                            "Restore insert failed table={} cols={:?} vals={:?} err={}",
                            table_name, col_names, debug_vals, e
                        );
                        return Err(AppError::Internal(format!(
                            "Restore insert failed in {}: {}",
                            table_name, e
                        )));
                    }
                    sqlx::query("RELEASE SAVEPOINT row_save")
                        .execute(&mut *tx)
                        .await
                        .ok();
                }
            }
        }

        tx.commit()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        info!("Restore completed successfully");
        Ok(())
    }

    /// Restore from a file already in the backups directory
    pub async fn restore_local_backup(
        &self,
        filename: String,
        target_tenant_id: Option<&str>,
    ) -> AppResult<()> {
        let path = self.get_backup_path(&filename)?;
        self.restore_from_zip(path, target_tenant_id).await
    }
}

// --- SCHEDULER ---
pub struct BackupScheduler {
    pool: DbPool,
    backup_service: BackupService,
    settings_service: SettingsService,
}

impl BackupScheduler {
    pub fn new(
        pool: DbPool,
        backup_service: BackupService,
        settings_service: SettingsService,
    ) -> Self {
        Self {
            pool,
            backup_service,
            settings_service,
        }
    }

    pub async fn start(&self) {
        let pool = self.pool.clone();
        let service = self.backup_service.clone();
        let settings_service = self.settings_service.clone();

        tokio::spawn(async move {
            info!("Backup Scheduler started.");
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60)); // Check every minute
            let mut warned_missing_schema = false;

            loop {
                interval.tick().await;

                #[cfg(feature = "postgres")]
                {
                    // Prevent duplicate processing when running multiple instances.
                    let locked: bool =
                        sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtext($1))")
                            .bind("backup_scheduler")
                            .fetch_one(&pool)
                            .await
                            .unwrap_or(false);
                    if !locked {
                        continue;
                    }
                }

                // 1. Check Global Schedule
                if let Err(e) = Self::check_and_run_global(&pool, &service, &settings_service).await
                {
                    if e.contains("relation \"settings\" does not exist")
                        || e.contains("relation \"tenants\" does not exist")
                    {
                        if !warned_missing_schema {
                            warned_missing_schema = true;
                            warn!(
                                "Backup scheduler paused: database schema not migrated yet (missing settings/tenants tables)."
                            );
                        }
                    } else {
                        error!("Global backup schedule check failed: {}", e);
                    }
                }

                // 2. Check Tenant Schedules
                if let Err(e) =
                    Self::check_and_run_tenants(&pool, &service, &settings_service).await
                {
                    if e.contains("relation \"settings\" does not exist")
                        || e.contains("relation \"tenants\" does not exist")
                    {
                        if !warned_missing_schema {
                            warned_missing_schema = true;
                            warn!(
                                "Backup scheduler paused: database schema not migrated yet (missing settings/tenants tables)."
                            );
                        }
                    } else {
                        error!("Tenant backup schedule check failed: {}", e);
                    }
                }

                #[cfg(feature = "postgres")]
                {
                    let _ =
                        sqlx::query_scalar::<_, bool>("SELECT pg_advisory_unlock(hashtext($1))")
                            .bind("backup_scheduler")
                            .fetch_one(&pool)
                            .await;
                }
            }
        });
    }

    async fn check_and_run_global(
        _pool: &DbPool,
        service: &BackupService,
        settings_service: &SettingsService,
    ) -> Result<(), String> {
        let tz = get_app_timezone(settings_service).await;
        let trigger_now =
            get_bool_setting(settings_service, None, "backup_global_trigger", false).await?;
        let enabled =
            get_bool_setting(settings_service, None, "backup_global_enabled", false).await?;
        if !enabled && !trigger_now {
            return Ok(());
        }

        let now = Utc::now();
        let last_run =
            get_datetime_setting(settings_service, None, "backup_global_last_run").await?;
        let should_run = if trigger_now {
            true
        } else {
            let cfg = get_mode_settings(
                settings_service,
                None,
                "backup_global",
                "backup_global_schedule",
                "02:00",
            )
            .await?;
            let Some((mode, every, daily, weekday)) = cfg else {
                warn!("Invalid global backup schedule; skipping");
                return Ok(());
            };

            match mode {
                ScheduleMode::Minute | ScheduleMode::Hour => {
                    should_run_interval(now, last_run, every, mode)
                }
                ScheduleMode::Day => should_run_daily(now, last_run, daily, tz),
                ScheduleMode::Week => should_run_weekly(now, last_run, weekday, daily, tz),
            }
        };

        if should_run {
            service
                .create_global_backup()
                .await
                .map_err(|e| format!("Failed to create global backup: {}", e))?;
            set_datetime_setting(
                settings_service,
                None,
                "backup_global_last_run",
                now,
                "Last successful global backup run (UTC)",
            )
            .await?;

            let retention_days =
                get_i64_setting(settings_service, None, "backup_global_retention_days", 30).await?;
            if retention_days > 0 {
                cleanup_backups(service, retention_days, BackupScope::Global).await?;
            }

            if trigger_now {
                set_bool_setting(
                    settings_service,
                    None,
                    "backup_global_trigger",
                    false,
                    "Manual trigger for global backup",
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn check_and_run_tenants(
        pool: &DbPool,
        service: &BackupService,
        settings_service: &SettingsService,
    ) -> Result<(), String> {
        let tz = get_app_timezone(settings_service).await;
        let trigger_now =
            get_bool_setting(settings_service, None, "backup_tenant_trigger", false).await?;
        let global_enabled =
            get_bool_setting(settings_service, None, "backup_tenant_enabled", false).await?;
        let global_cfg = get_mode_settings(
            settings_service,
            None,
            "backup_tenant",
            "backup_tenant_schedule",
            "02:30",
        )
        .await?;
        let global_retention_days =
            get_i64_setting(settings_service, None, "backup_tenant_retention_days", 14).await?;

        if !global_enabled && !trigger_now {
            return Ok(());
        }

        let now = Utc::now();
        let tenant_ids = list_active_tenants(pool)
            .await
            .map_err(|e| format!("Failed to list tenants: {}", e))?;

        for tenant_id in tenant_ids {
            let enabled =
                get_bool_setting(settings_service, Some(&tenant_id), "backup_enabled", true)
                    .await?;
            if !enabled {
                continue;
            }

            let last_run =
                get_datetime_setting(settings_service, Some(&tenant_id), "backup_last_run").await?;
            let should_run = if trigger_now {
                true
            } else {
                let tenant_cfg = get_mode_settings(
                    settings_service,
                    Some(&tenant_id),
                    "backup",
                    "backup_schedule",
                    "02:30",
                )
                .await?;
                let cfg = tenant_cfg.or(global_cfg);
                let Some((mode, every, daily, weekday)) = cfg else {
                    warn!("Invalid backup schedule for tenant {}; skipping", tenant_id);
                    continue;
                };

                match mode {
                    ScheduleMode::Minute | ScheduleMode::Hour => {
                        should_run_interval(now, last_run, every, mode)
                    }
                    ScheduleMode::Day => should_run_daily(now, last_run, daily, tz),
                    ScheduleMode::Week => should_run_weekly(now, last_run, weekday, daily, tz),
                }
            };
            if should_run {
                service
                    .create_tenant_backup(&tenant_id)
                    .await
                    .map_err(|e| {
                        format!("Failed to create tenant backup for {}: {}", tenant_id, e)
                    })?;
                set_datetime_setting(
                    settings_service,
                    Some(&tenant_id),
                    "backup_last_run",
                    now,
                    "Last successful tenant backup run (UTC)",
                )
                .await?;

                let retention_days = get_i64_setting(
                    settings_service,
                    Some(&tenant_id),
                    "backup_retention_days",
                    global_retention_days,
                )
                .await?;
                if retention_days > 0 {
                    cleanup_backups(
                        service,
                        retention_days,
                        BackupScope::Tenant(tenant_id.clone()),
                    )
                    .await?;
                }
            }
        }

        if trigger_now {
            set_bool_setting(
                settings_service,
                None,
                "backup_tenant_trigger",
                false,
                "Manual trigger for tenant backups",
            )
            .await?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct DailySchedule {
    hour: u32,
    minute: u32,
}

fn parse_daily_schedule(input: &str) -> Option<DailySchedule> {
    let raw = input.trim();
    if raw.is_empty() {
        return None;
    }

    if raw.eq_ignore_ascii_case("@daily") || raw.eq_ignore_ascii_case("daily") {
        return Some(DailySchedule { hour: 0, minute: 0 });
    }

    if raw.contains(':') {
        let parts: Vec<&str> = raw.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        let hour: u32 = parts[0].trim().parse().ok()?;
        let minute: u32 = parts[1].trim().parse().ok()?;
        if hour < 24 && minute < 60 {
            return Some(DailySchedule { hour, minute });
        }
        return None;
    }

    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() != 5 {
        return None;
    }
    if parts[2] != "*" || parts[3] != "*" || parts[4] != "*" {
        return None;
    }
    let minute: u32 = parts[0].parse().ok()?;
    let hour: u32 = parts[1].parse().ok()?;
    if hour < 24 && minute < 60 {
        return Some(DailySchedule { hour, minute });
    }
    None
}

fn should_run_daily(
    now: DateTime<Utc>,
    last_run: Option<DateTime<Utc>>,
    schedule: DailySchedule,
    tz: Tz,
) -> bool {
    // Interpret HH:MM in the app's timezone (app_timezone), then convert to UTC for comparison.
    let local_day = now.with_timezone(&tz).date_naive();
    let scheduled_today = scheduled_time_for_day(local_day, schedule, tz);
    if now < scheduled_today {
        return false;
    }
    match last_run {
        None => true,
        Some(last) => last < scheduled_today,
    }
}

fn scheduled_time_for_day(day: NaiveDate, schedule: DailySchedule, tz: Tz) -> DateTime<Utc> {
    let time = NaiveTime::from_hms_opt(schedule.hour, schedule.minute, 0)
        .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());

    let naive = NaiveDateTime::new(day, time);
    // DST can make local times ambiguous or invalid; pick a reasonable instant.
    if let Some(dt) = tz
        .from_local_datetime(&naive)
        .single()
        .or_else(|| tz.from_local_datetime(&naive).earliest())
        .or_else(|| tz.from_local_datetime(&naive).latest())
    {
        return dt.with_timezone(&Utc);
    }

    // Fallback: treat it as UTC if timezone conversion fails (should be rare).
    DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
}

#[derive(Debug, Clone, Copy)]
enum ScheduleMode {
    Minute,
    Hour,
    Day,
    Week,
}

fn parse_mode(input: &str) -> Option<ScheduleMode> {
    match input.trim().to_ascii_lowercase().as_str() {
        "minute" | "minutes" => Some(ScheduleMode::Minute),
        "hour" | "hours" => Some(ScheduleMode::Hour),
        "day" | "daily" => Some(ScheduleMode::Day),
        "week" | "weekly" => Some(ScheduleMode::Week),
        _ => None,
    }
}

fn parse_hhmm(input: &str) -> Option<(u32, u32)> {
    let raw = input.trim();
    let parts: Vec<&str> = raw.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let hour: u32 = parts[0].trim().parse().ok()?;
    let minute: u32 = parts[1].trim().parse().ok()?;
    if hour < 24 && minute < 60 {
        Some((hour, minute))
    } else {
        None
    }
}

fn parse_weekday(input: &str) -> Option<u32> {
    match input.trim().to_ascii_lowercase().as_str() {
        "mon" | "monday" => Some(1),
        "tue" | "tuesday" => Some(2),
        "wed" | "wednesday" => Some(3),
        "thu" | "thursday" => Some(4),
        "fri" | "friday" => Some(5),
        "sat" | "saturday" => Some(6),
        "sun" | "sunday" => Some(7),
        _ => None,
    }
}

fn weekday_of_date(day: NaiveDate) -> u32 {
    day.weekday().number_from_monday()
}

fn should_run_interval(
    now: DateTime<Utc>,
    last_run: Option<DateTime<Utc>>,
    every: i64,
    unit: ScheduleMode,
) -> bool {
    let every = every.max(1);
    let dur = match unit {
        ScheduleMode::Minute => Duration::minutes(every),
        ScheduleMode::Hour => Duration::hours(every),
        _ => return false,
    };
    match last_run {
        None => true,
        Some(last) => now - last >= dur,
    }
}

fn should_run_weekly(
    now: DateTime<Utc>,
    last_run: Option<DateTime<Utc>>,
    weekday: u32,
    schedule: DailySchedule,
    tz: Tz,
) -> bool {
    // Run once per week at weekday+time in app_timezone
    let today = now.with_timezone(&tz).date_naive();
    let scheduled_this_week = scheduled_time_for_week_tz(today, weekday, schedule, tz);
    if now < scheduled_this_week {
        let last_week = today.checked_sub_signed(Duration::days(7)).unwrap_or(today);
        let scheduled_last_week = scheduled_time_for_week_tz(last_week, weekday, schedule, tz);
        return last_run.map(|lr| lr < scheduled_last_week).unwrap_or(false);
    }
    last_run.map(|lr| lr < scheduled_this_week).unwrap_or(true)
}

fn scheduled_time_for_week_tz(
    day: NaiveDate,
    weekday: u32,
    schedule: DailySchedule,
    tz: Tz,
) -> DateTime<Utc> {
    // weekday: 1=Mon .. 7=Sun
    let current = weekday_of_date(day);
    let delta_days = if current <= weekday {
        weekday - current
    } else {
        7 - (current - weekday)
    } as i64;

    let target_day = day
        .checked_add_signed(Duration::days(delta_days))
        .unwrap_or(day);
    scheduled_time_for_day(target_day, schedule, tz)
}

async fn get_mode_settings(
    settings_service: &SettingsService,
    tenant_id: Option<&str>,
    prefix: &str,
    legacy_schedule_key: &str,
    default_at: &str,
) -> Result<Option<(ScheduleMode, i64, DailySchedule, u32)>, String> {
    // Returns (mode, every, time, weekday)
    // If mode not configured, falls back to legacy cron-string (treated as Day).
    let mode_key = format!("{}_mode", prefix);
    let mode_raw = settings_service
        .get_value(tenant_id, &mode_key)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(mode_raw) = mode_raw {
        let mode = parse_mode(&mode_raw)
            .ok_or_else(|| format!("Invalid {} value '{}'", mode_key, mode_raw))?;

        let every_key = format!("{}_every", prefix);
        let at_key = format!("{}_at", prefix);
        let weekday_key = format!("{}_weekday", prefix);

        let every = settings_service
            .get_value(tenant_id, &every_key)
            .await
            .map_err(|e| e.to_string())?
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(15);

        let at = settings_service
            .get_value(tenant_id, &at_key)
            .await
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| default_at.to_string());
        let (hour, minute) = parse_hhmm(&at).unwrap_or((2, 0));

        let weekday = settings_service
            .get_value(tenant_id, &weekday_key)
            .await
            .map_err(|e| e.to_string())?
            .and_then(|v| parse_weekday(&v))
            .unwrap_or(7); // Sun

        return Ok(Some((mode, every, DailySchedule { hour, minute }, weekday)));
    }

    let legacy = settings_service
        .get_value(tenant_id, legacy_schedule_key)
        .await
        .map_err(|e| e.to_string())?;
    let legacy = legacy.unwrap_or_else(|| "0 2 * * *".to_string());
    if let Some(daily) = parse_daily_schedule(&legacy) {
        return Ok(Some((ScheduleMode::Day, 0, daily, 7)));
    }
    Ok(None)
}

async fn get_bool_setting(
    settings_service: &SettingsService,
    tenant_id: Option<&str>,
    key: &str,
    default_value: bool,
) -> Result<bool, String> {
    let raw = settings_service
        .get_value(tenant_id, key)
        .await
        .map_err(|e| e.to_string())?;
    Ok(raw
        .map(|v| v == "true" || v == "1" || v.eq_ignore_ascii_case("yes"))
        .unwrap_or(default_value))
}

async fn get_i64_setting(
    settings_service: &SettingsService,
    tenant_id: Option<&str>,
    key: &str,
    default_value: i64,
) -> Result<i64, String> {
    let raw = settings_service
        .get_value(tenant_id, key)
        .await
        .map_err(|e| e.to_string())?;
    Ok(raw
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(default_value))
}

async fn get_datetime_setting(
    settings_service: &SettingsService,
    tenant_id: Option<&str>,
    key: &str,
) -> Result<Option<DateTime<Utc>>, String> {
    let raw = settings_service
        .get_value(tenant_id, key)
        .await
        .map_err(|e| e.to_string())?;
    Ok(raw
        .and_then(|v| DateTime::parse_from_rfc3339(&v).ok())
        .map(|dt| dt.with_timezone(&Utc)))
}

async fn get_app_timezone(settings_service: &SettingsService) -> Tz {
    let raw = match settings_service.get_value(None, "app_timezone").await {
        Ok(v) => v.unwrap_or_else(|| "UTC".to_string()),
        Err(e) => {
            warn!("Failed to read app_timezone setting: {}", e);
            "UTC".to_string()
        }
    };

    match raw.parse::<Tz>() {
        Ok(tz) => tz,
        Err(_) => {
            warn!("Invalid app_timezone '{}'; falling back to UTC", raw);
            chrono_tz::UTC
        }
    }
}

async fn set_bool_setting(
    settings_service: &SettingsService,
    tenant_id: Option<&str>,
    key: &str,
    value: bool,
    description: &str,
) -> Result<(), String> {
    let dto = UpsertSettingDto {
        key: key.to_string(),
        value: if value {
            "true".to_string()
        } else {
            "false".to_string()
        },
        description: Some(description.to_string()),
    };
    settings_service
        .upsert(tenant_id.map(|t| t.to_string()), dto, None, None)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

async fn set_datetime_setting(
    settings_service: &SettingsService,
    tenant_id: Option<&str>,
    key: &str,
    value: DateTime<Utc>,
    description: &str,
) -> Result<(), String> {
    let dto = UpsertSettingDto {
        key: key.to_string(),
        value: value.to_rfc3339(),
        description: Some(description.to_string()),
    };
    settings_service
        .upsert(tenant_id.map(|t| t.to_string()), dto, None, None)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

async fn list_active_tenants(pool: &DbPool) -> AppResult<Vec<String>> {
    #[cfg(feature = "postgres")]
    {
        let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM tenants WHERE is_active = true")
            .fetch_all(pool)
            .await?;
        Ok(ids)
    }

    #[cfg(feature = "sqlite")]
    {
        let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM tenants WHERE is_active = 1")
            .fetch_all(pool)
            .await?;
        Ok(ids)
    }
}

enum BackupScope {
    Global,
    Tenant(String),
}

async fn cleanup_backups(
    service: &BackupService,
    retention_days: i64,
    scope: BackupScope,
) -> Result<(), String> {
    let cutoff = Utc::now() - Duration::days(retention_days);
    let backups = service.list_backups().await.map_err(|e| e.to_string())?;
    for backup in backups {
        let should_delete = match &scope {
            BackupScope::Global => backup.backup_type == "global",
            BackupScope::Tenant(tid) => {
                backup.backup_type == "tenant" && backup.tenant_id.as_deref() == Some(tid.as_str())
            }
        };
        if should_delete && backup.created_at < cutoff {
            let _ = service.delete_backup(backup.name).await;
        }
    }
    Ok(())
}
