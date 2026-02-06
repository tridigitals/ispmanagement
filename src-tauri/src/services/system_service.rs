//! System Health & Monitoring Service

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Pool;
#[cfg(feature = "postgres")]
use sqlx::Postgres;
#[cfg(feature = "sqlite")]
use sqlx::Sqlite;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::sync::RwLock;

use crate::services::SettingsService;

#[derive(Debug, Serialize, Clone)]
pub struct DatabaseStats {
    pub is_connected: bool,
    pub database_type: String,
    pub database_size_bytes: i64,
    pub total_tables: i64,
    pub tenants_count: i64,
    pub users_count: i64,
    pub audit_logs_count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct SystemResources {
    pub cpu_usage: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub os_name: String,
    pub os_version: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct TableInfo {
    pub name: String,
    pub row_count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct RecentActivity {
    pub id: String,
    pub action: String,
    pub resource: String,
    pub user_email: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SystemHealth {
    pub database: DatabaseStats,
    pub resources: SystemResources,
    pub tables: Vec<TableInfo>,
    pub active_sessions: i64,
    pub recent_activity: Vec<RecentActivity>,
    pub uptime_seconds: u64,
    pub app_version: String,
    pub collected_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_metrics: Option<crate::services::metrics_service::RequestMetrics>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MigrationItem {
    pub version: i64,
    pub description: String,
    pub installed_on: DateTime<Utc>,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MigrationSummary {
    pub resolved_count: i64,
    pub applied_count: i64,
    pub pending_count: i64,
    pub missing_count: i64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pending_versions: Vec<i64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing_versions: Vec<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_applied_version: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SettingsSnapshot {
    pub app_name: Option<String>,
    pub app_public_url: Option<String>,
    pub app_timezone: Option<String>,
    pub base_currency_code: Option<String>,
    pub currency_code: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct BackupSnapshot {
    pub global_enabled: bool,
    pub global_mode: Option<String>,
    pub global_every: Option<i64>,
    pub global_at: Option<String>,
    pub global_weekday: Option<String>,
    pub global_retention_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_last_run_utc: Option<DateTime<Utc>>,

    pub tenant_enabled: bool,
    pub tenant_mode: Option<String>,
    pub tenant_every: Option<i64>,
    pub tenant_at: Option<String>,
    pub tenant_weekday: Option<String>,
    pub tenant_retention_days: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SystemDiagnostics {
    pub database: DatabaseStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_server_version: Option<String>,
    pub migrations: MigrationSummary,
    pub applied_migrations: Vec<MigrationItem>,
    pub settings: SettingsSnapshot,
    pub backups: BackupSnapshot,
    pub collected_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct SystemService {
    #[cfg(feature = "postgres")]
    pub pool: Pool<Postgres>,
    #[cfg(feature = "sqlite")]
    pub pool: Pool<Sqlite>,
    start_time: Instant,
    cache: Arc<RwLock<Option<(SystemHealth, Instant)>>>,
    metrics: Arc<crate::services::metrics_service::MetricsService>,
}

impl SystemService {
    #[cfg(feature = "postgres")]
    pub fn new(
        pool: Pool<Postgres>,
        metrics: Arc<crate::services::metrics_service::MetricsService>,
    ) -> Self {
        Self {
            pool,
            start_time: Instant::now(),
            cache: Arc::new(RwLock::new(None)),
            metrics,
        }
    }

    #[cfg(feature = "sqlite")]
    pub fn new(
        pool: Pool<Sqlite>,
        metrics: Arc<crate::services::metrics_service::MetricsService>,
    ) -> Self {
        Self {
            pool,
            start_time: Instant::now(),
            cache: Arc::new(RwLock::new(None)),
            metrics,
        }
    }

    pub async fn get_system_health(&self) -> Result<SystemHealth, sqlx::Error> {
        const CACHE_TTL: Duration = Duration::from_secs(10);

        if let Some((cached, at)) = self.cache.read().await.clone() {
            if at.elapsed() < CACHE_TTL {
                return Ok(cached);
            }
        }

        let (database, tables, active_sessions, recent_activity, resources) = tokio::try_join!(
            self.get_database_stats(),
            self.get_table_info(),
            self.get_active_sessions(),
            self.get_recent_activity(10),
            async { Ok::<_, sqlx::Error>(self.get_system_resources().await) },
        )?;

        let health = SystemHealth {
            database,
            resources,
            tables,
            active_sessions,
            recent_activity,
            uptime_seconds: self.start_time.elapsed().as_secs(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            collected_at: Utc::now(),
            request_metrics: Some(self.metrics.get_metrics()),
        };

        *self.cache.write().await = Some((health.clone(), Instant::now()));
        Ok(health)
    }

    pub async fn get_system_diagnostics(
        &self,
        settings_service: &SettingsService,
    ) -> Result<SystemDiagnostics, sqlx::Error> {
        let database = self.get_database_stats().await?;

        let database_server_version = if database.is_connected {
            #[cfg(feature = "postgres")]
            {
                sqlx::query_scalar::<_, String>("SELECT version()")
                    .fetch_one(&self.pool)
                    .await
                    .ok()
            }
            #[cfg(feature = "sqlite")]
            {
                sqlx::query_scalar::<_, String>("SELECT sqlite_version()")
                    .fetch_one(&self.pool)
                    .await
                    .ok()
            }
        } else {
            None
        };

        // Migrations: applied from DB + resolved from embedded migrator
        let applied_migrations = self.get_applied_migrations().await.unwrap_or_default();
        let latest_applied_version = applied_migrations.last().map(|m| m.version);

        // Resolved migrations from embedded files. If this fails for any reason, treat as unknown.
        let resolved_versions = self.get_resolved_migration_versions();

        use std::collections::HashSet;
        let applied_set: HashSet<i64> = applied_migrations.iter().map(|m| m.version).collect();
        let resolved_set: HashSet<i64> = resolved_versions.iter().copied().collect();

        let mut pending_versions: Vec<i64> =
            resolved_set.difference(&applied_set).copied().collect();
        pending_versions.sort_unstable();
        let mut missing_versions: Vec<i64> =
            applied_set.difference(&resolved_set).copied().collect();
        missing_versions.sort_unstable();

        let migrations = MigrationSummary {
            resolved_count: resolved_versions.len() as i64,
            applied_count: applied_migrations.len() as i64,
            pending_count: pending_versions.len() as i64,
            missing_count: missing_versions.len() as i64,
            pending_versions,
            missing_versions,
            latest_applied_version,
        };

        let settings = SettingsSnapshot {
            app_name: settings_service
                .get_value(None, "app_name")
                .await
                .ok()
                .flatten(),
            app_public_url: settings_service
                .get_value(None, "app_public_url")
                .await
                .ok()
                .flatten(),
            app_timezone: settings_service
                .get_value(None, "app_timezone")
                .await
                .ok()
                .flatten(),
            base_currency_code: settings_service
                .get_value(None, "base_currency_code")
                .await
                .ok()
                .flatten(),
            currency_code: settings_service
                .get_value(None, "currency_code")
                .await
                .ok()
                .flatten(),
        };

        let backups = self.get_backup_snapshot(settings_service).await;

        Ok(SystemDiagnostics {
            database,
            database_server_version,
            migrations,
            applied_migrations,
            settings,
            backups,
            collected_at: Utc::now(),
        })
    }

    async fn get_database_stats(&self) -> Result<DatabaseStats, sqlx::Error> {
        // Test connection with simple query
        let is_connected = sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .is_ok();

        let (tenants_count, users_count, audit_logs_count) = if is_connected {
            sqlx::query_as::<_, (i64, i64, i64)>(
                "SELECT \
                    (SELECT COUNT(*) FROM tenants) AS tenants_count, \
                    (SELECT COUNT(*) FROM users) AS users_count, \
                    (SELECT COUNT(*) FROM audit_logs) AS audit_logs_count",
            )
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0, 0, 0))
        } else {
            (0, 0, 0)
        };

        let database_type = if cfg!(feature = "postgres") {
            "PostgreSQL".to_string()
        } else {
            "SQLite".to_string()
        };

        let (database_size_bytes, total_tables) = if is_connected {
            tokio::join!(self.get_database_size(), self.get_total_tables())
        } else {
            (0, 0)
        };

        Ok(DatabaseStats {
            is_connected,
            database_type,
            database_size_bytes,
            total_tables,
            tenants_count,
            users_count,
            audit_logs_count,
        })
    }

    fn get_resolved_migration_versions(&self) -> Vec<i64> {
        // Best-effort: if the migrations aren't available (shouldn't happen in dev),
        // diagnostics will still show applied migrations from DB.
        static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

        MIGRATOR.iter().map(|m| m.version).collect()
    }

    async fn get_applied_migrations(&self) -> Result<Vec<MigrationItem>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct Row {
            version: i64,
            description: String,
            installed_on: DateTime<Utc>,
            success: bool,
            execution_time: Option<i64>,
        }

        let rows: Vec<Row> = match sqlx::query_as::<_, Row>(
            "SELECT version, description, installed_on, success, execution_time FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(&self.pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                // Missing table on fresh DB before migrations: treat as none.
                #[cfg(feature = "postgres")]
                if let sqlx::Error::Database(db) = &e {
                    if db.code().as_deref() == Some("42P01") {
                        return Ok(Vec::new());
                    }
                }
                return Err(e);
            }
        };

        Ok(rows
            .into_iter()
            .map(|r| MigrationItem {
                version: r.version,
                description: r.description,
                installed_on: r.installed_on,
                success: r.success,
                execution_time_ms: r.execution_time.map(|v| v / 1_000_000),
            })
            .collect())
    }

    async fn get_backup_snapshot(&self, settings_service: &SettingsService) -> BackupSnapshot {
        let get_str = |k: &'static str| async move {
            settings_service.get_value(None, k).await.ok().flatten()
        };

        let get_bool = |k: &'static str| async move {
            settings_service
                .get_value(None, k)
                .await
                .ok()
                .flatten()
                .map(|v| v == "true")
                .unwrap_or(false)
        };

        let get_i64 = |k: &'static str| async move {
            settings_service
                .get_value(None, k)
                .await
                .ok()
                .flatten()
                .and_then(|v| v.parse::<i64>().ok())
        };

        let global_enabled = get_bool("backup_global_enabled").await;
        let tenant_enabled = get_bool("backup_tenant_enabled").await;

        let global_last_run_utc = settings_service
            .get_value(None, "backup_global_last_run")
            .await
            .ok()
            .flatten()
            .and_then(|raw| chrono::DateTime::parse_from_rfc3339(&raw).ok())
            .map(|dt| dt.with_timezone(&Utc));

        BackupSnapshot {
            global_enabled,
            global_mode: get_str("backup_global_mode").await,
            global_every: get_i64("backup_global_every").await,
            global_at: get_str("backup_global_at").await,
            global_weekday: get_str("backup_global_weekday").await,
            global_retention_days: get_i64("backup_global_retention_days").await,
            global_last_run_utc,
            tenant_enabled,
            tenant_mode: get_str("backup_tenant_mode").await,
            tenant_every: get_i64("backup_tenant_every").await,
            tenant_at: get_str("backup_tenant_at").await,
            tenant_weekday: get_str("backup_tenant_weekday").await,
            tenant_retention_days: get_i64("backup_tenant_retention_days").await,
        }
    }

    async fn get_table_info(&self) -> Result<Vec<TableInfo>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct TableRow {
            name: String,
            row_count: i64,
        }

        // Single query (fast) + fallback (tolerant) if any table is missing.
        let union_query = "\
            SELECT 'users' AS name, (SELECT COUNT(*) FROM users) AS row_count \
            UNION ALL SELECT 'tenants', (SELECT COUNT(*) FROM tenants) \
            UNION ALL SELECT 'tenant_members', (SELECT COUNT(*) FROM tenant_members) \
            UNION ALL SELECT 'roles', (SELECT COUNT(*) FROM roles) \
            UNION ALL SELECT 'role_permissions', (SELECT COUNT(*) FROM role_permissions) \
            UNION ALL SELECT 'permissions', (SELECT COUNT(*) FROM permissions) \
            UNION ALL SELECT 'settings', (SELECT COUNT(*) FROM settings) \
            UNION ALL SELECT 'audit_logs', (SELECT COUNT(*) FROM audit_logs)";

        match sqlx::query_as::<_, TableRow>(union_query)
            .fetch_all(&self.pool)
            .await
        {
            Ok(rows) => Ok(rows
                .into_iter()
                .map(|r| TableInfo {
                    name: r.name,
                    row_count: r.row_count,
                })
                .collect()),
            Err(_) => {
                let mut tables = Vec::new();
                let table_names = vec![
                    "users",
                    "tenants",
                    "tenant_members",
                    "roles",
                    "role_permissions",
                    "permissions",
                    "settings",
                    "audit_logs",
                ];

                for name in table_names {
                    let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", name))
                        .fetch_one(&self.pool)
                        .await
                        .unwrap_or(0);

                    tables.push(TableInfo {
                        name: name.to_string(),
                        row_count: count,
                    });
                }

                Ok(tables)
            }
        }
    }

    async fn get_active_sessions(&self) -> Result<i64, sqlx::Error> {
        // Count users who have logged in within the last 24 hours (based on updated_at)
        // This is an approximation since we don't have a sessions table
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE is_active = true")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        Ok(count)
    }

    async fn get_recent_activity(&self, limit: i64) -> Result<Vec<RecentActivity>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        {
            #[derive(sqlx::FromRow)]
            struct ActivityRow {
                id: uuid::Uuid,
                action: String,
                resource: String,
                user_email: Option<String>,
                created_at: DateTime<Utc>,
            }

            let rows: Vec<ActivityRow> = sqlx::query_as(
                "SELECT a.id, a.action, a.resource, u.email as user_email, a.created_at \
                 FROM audit_logs a \
                 LEFT JOIN users u ON u.id = a.user_id \
                 ORDER BY a.created_at DESC LIMIT $1",
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            Ok(rows
                .into_iter()
                .map(|row| RecentActivity {
                    id: row.id.to_string(),
                    action: row.action,
                    resource: row.resource,
                    user_email: row.user_email,
                    created_at: row.created_at,
                })
                .collect())
        }

        #[cfg(feature = "sqlite")]
        {
            #[derive(sqlx::FromRow)]
            struct ActivityRow {
                id: String,
                action: String,
                resource: String,
                user_email: Option<String>,
                created_at: String,
            }

            let rows: Vec<ActivityRow> = sqlx::query_as(
                "SELECT a.id, a.action, a.resource, u.email as user_email, a.created_at \
                 FROM audit_logs a \
                 LEFT JOIN users u ON u.id = a.user_id \
                 ORDER BY a.created_at DESC LIMIT $1",
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            let mut activities = Vec::new();
            for row in rows {
                let created_at = DateTime::parse_from_rfc3339(&row.created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                activities.push(RecentActivity {
                    id: row.id,
                    action: row.action,
                    resource: row.resource,
                    user_email: row.user_email,
                    created_at,
                });
            }

            Ok(activities)
        }
    }

    async fn get_database_size(&self) -> i64 {
        #[cfg(feature = "postgres")]
        {
            sqlx::query_scalar("SELECT pg_database_size(current_database())")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0)
        }

        #[cfg(feature = "sqlite")]
        {
            let page_count: i64 = sqlx::query_scalar("PRAGMA page_count")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);
            let page_size: i64 = sqlx::query_scalar("PRAGMA page_size")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);
            page_count * page_size
        }
    }

    async fn get_system_resources(&self) -> SystemResources {
        let mut sys = System::new_all();
        sys.refresh_all();

        // Brief sleep to get accurate CPU usage
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        sys.refresh_all();

        SystemResources {
            cpu_usage: sys.global_cpu_info().cpu_usage(),
            memory_used_bytes: sys.used_memory(),
            memory_total_bytes: sys.total_memory(),
            os_name: System::name().unwrap_or_default(),
            os_version: System::os_version().unwrap_or_default(),
        }
    }

    async fn get_total_tables(&self) -> i64 {
        #[cfg(feature = "postgres")]
        {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public'",
            )
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0)
        }

        #[cfg(feature = "sqlite")]
        {
            sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0)
        }
    }
}
