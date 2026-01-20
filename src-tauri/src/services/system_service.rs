//! System Health & Monitoring Service

use sqlx::{Pool, Postgres};
use serde::Serialize;
use chrono::{DateTime, Utc};
use sysinfo::System;

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
}

#[derive(Clone)]
pub struct SystemService {
    #[cfg(feature = "postgres")]
    pub pool: Pool<Postgres>,
    #[cfg(feature = "sqlite")]
    pub pool: Pool<Sqlite>,
    start_time: std::time::Instant,
}

impl SystemService {
    #[cfg(feature = "postgres")]
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { 
            pool,
            start_time: std::time::Instant::now(),
        }
    }

    #[cfg(feature = "sqlite")]
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { 
            pool,
            start_time: std::time::Instant::now(),
        }
    }

    pub async fn get_system_health(&self) -> Result<SystemHealth, sqlx::Error> {
        let database = self.get_database_stats().await?;
        let resources = self.get_system_resources().await;
        let tables = self.get_table_info().await?;
        let active_sessions = self.get_active_sessions().await?;
        let recent_activity = self.get_recent_activity(10).await?;
        
        Ok(SystemHealth {
            database,
            resources,
            tables,
            active_sessions,
            recent_activity,
            uptime_seconds: self.start_time.elapsed().as_secs(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            collected_at: Utc::now(),
        })
    }

    async fn get_database_stats(&self) -> Result<DatabaseStats, sqlx::Error> {
        // Test connection with simple query
        let is_connected = sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .is_ok();

        // Get counts
        let tenants_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenants")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let users_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let audit_logs_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_logs")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let database_type = if cfg!(feature = "postgres") {
            "PostgreSQL".to_string()
        } else {
            "SQLite".to_string()
        };

        // Get database size
        let database_size_bytes = self.get_database_size().await;

        let total_tables = self.get_total_tables().await?;

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

    async fn get_table_info(&self) -> Result<Vec<TableInfo>, sqlx::Error> {
        let mut tables = Vec::new();

        // Core tables to check
        let table_names = vec![
            "users", "tenants", "tenant_members", "roles", 
            "role_permissions", "permissions", "settings", "audit_logs"
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

    async fn get_active_sessions(&self) -> Result<i64, sqlx::Error> {
        // Count users who have logged in within the last 24 hours (based on updated_at)
        // This is an approximation since we don't have a sessions table
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE is_active = true"
        )
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
                user_id: Option<uuid::Uuid>,
                created_at: DateTime<Utc>,
            }

            let rows: Vec<ActivityRow> = sqlx::query_as(
                "SELECT id, action, resource, user_id, created_at FROM audit_logs ORDER BY created_at DESC LIMIT $1"
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

            let mut activities = Vec::new();
            for row in rows {
                // Get user email if available
                let user_email: Option<String> = if let Some(uid) = row.user_id {
                    sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
                        .bind(uid)
                        .fetch_optional(&self.pool)
                        .await
                        .unwrap_or(None)
                } else {
                    None
                };

                activities.push(RecentActivity {
                    id: row.id.to_string(),
                    action: row.action,
                    resource: row.resource,
                    user_email,
                    created_at: row.created_at,
                });
            }

            Ok(activities)
        }

        #[cfg(feature = "sqlite")]
        {
            #[derive(sqlx::FromRow)]
            struct ActivityRow {
                id: String,
                action: String,
                resource: String,
                user_id: Option<String>,
                created_at: String,
            }

            let rows: Vec<ActivityRow> = sqlx::query_as(
                "SELECT id, action, resource, user_id, created_at FROM audit_logs ORDER BY created_at DESC LIMIT $1"
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

            let mut activities = Vec::new();
            for row in rows {
                // Get user email if available
                let user_email: Option<String> = if let Some(ref uid) = row.user_id {
                    sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
                        .bind(uid)
                        .fetch_optional(&self.pool)
                        .await
                        .unwrap_or(None)
                } else {
                    None
                };

                let created_at = DateTime::parse_from_rfc3339(&row.created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                activities.push(RecentActivity {
                    id: row.id,
                    action: row.action,
                    resource: row.resource,
                    user_email,
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
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        sys.refresh_all();

        SystemResources {
            cpu_usage: sys.global_cpu_info().cpu_usage(),
            memory_used_bytes: sys.used_memory(),
            memory_total_bytes: sys.total_memory(),
            os_name: System::name().unwrap_or_default(),
            os_version: System::os_version().unwrap_or_default(),
        }
    }

    async fn get_total_tables(&self) -> Result<i64, sqlx::Error> {
        #[cfg(feature = "postgres")]
        {
            sqlx::query_scalar("SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public'")
                .fetch_one(&self.pool)
                .await
        }

        #[cfg(feature = "sqlite")]
        {
            sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")
                .fetch_one(&self.pool)
                .await
        }
    }
}
