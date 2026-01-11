use crate::db::connection::DbPool;
use crate::error::AppResult;
use crate::models::AuditLog;
use tauri::State;
use chrono::Utc;

#[derive(Clone)]
pub struct AuditService {
    pub pool: DbPool,
}

impl AuditService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Log an action to the audit_logs table
    pub async fn log(
        &self,
        user_id: Option<&str>,
        tenant_id: Option<&str>,
        action: &str,
        resource: &str,
        resource_id: Option<&str>,
        details: Option<&str>,
        ip_address: Option<&str>,
    ) {
        // We spawn this to not block the main request flow, or just await it.
        // For safety/reliability in this context, we'll await it but ignore errors to not fail the main action.
        let id = uuid::Uuid::new_v4();
        let now = Utc::now();

        // Ensure table exists (Quick fix for no migration runner)
        // ideally this should be done once on startup, but for safety in this dev env:
        let _ = self.ensure_table().await;

        #[cfg(feature = "postgres")]
        let query = r#"
            INSERT INTO audit_logs (id, user_id, tenant_id, action, resource, resource_id, details, ip_address, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            INSERT INTO audit_logs (id, user_id, tenant_id, action, resource, resource_id, details, ip_address, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let user_uuid = user_id.and_then(|u| uuid::Uuid::parse_str(u).ok());
        let tenant_uuid = tenant_id.and_then(|t| uuid::Uuid::parse_str(t).ok());

        #[cfg(feature = "postgres")]
        let res = sqlx::query(query)
            .bind(id)
            .bind(user_uuid)
            .bind(tenant_uuid)
            .bind(action)
            .bind(resource)
            .bind(resource_id)
            .bind(details)
            .bind(ip_address)
            .bind(now);

        #[cfg(feature = "sqlite")]
        let res = sqlx::query(query)
            .bind(id.to_string())
            .bind(user_id) // Sqlite stores UUIDs as TEXT
            .bind(tenant_id)
            .bind(action)
            .bind(resource)
            .bind(resource_id)
            .bind(details)
            .bind(ip_address)
            .bind(now.to_rfc3339());

        if let Err(e) = res.execute(&self.pool).await {
            eprintln!("Failed to write audit log: {}", e);
        }
    }

    pub async fn ensure_table(&self) -> AppResult<()> {
        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS audit_logs (
                id TEXT PRIMARY KEY,
                user_id TEXT,
                tenant_id TEXT,
                action TEXT NOT NULL,
                resource TEXT NOT NULL,
                resource_id TEXT,
                details TEXT,
                ip_address TEXT,
                created_at TEXT NOT NULL
            )"#
        ).execute(&self.pool).await?;

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS audit_logs (
                id UUID PRIMARY KEY,
                user_id UUID,
                tenant_id UUID,
                action VARCHAR(255) NOT NULL,
                resource VARCHAR(255) NOT NULL,
                resource_id TEXT,
                details TEXT,
                ip_address VARCHAR(45),
                created_at TIMESTAMPTZ NOT NULL
            )"#
        ).execute(&self.pool).await?;

        // Migration: Attempt to add ip_address if it doesn't exist (ignore errors if it does)
        #[cfg(feature = "postgres")]
        let _ = sqlx::query("ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS ip_address VARCHAR(45)")
            .execute(&self.pool)
            .await;

        #[cfg(feature = "sqlite")]
        let _ = sqlx::query("ALTER TABLE audit_logs ADD COLUMN ip_address TEXT")
            .execute(&self.pool)
            .await;

        Ok(())
    }

    /// List logs with filters
    pub async fn list(&self, filter: crate::models::AuditLogFilter) -> AppResult<(Vec<crate::models::AuditLogResponse>, i64)> {
        let page = filter.page.unwrap_or(1);
        let per_page = filter.per_page.unwrap_or(20);
        let offset = (page.saturating_sub(1)) * per_page;

        // --- Postgres Implementation ---
        #[cfg(feature = "postgres")]
        {
            use sqlx::{Postgres, QueryBuilder, Row};
            
            let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
                r#"SELECT 
                    l.id, l.user_id, l.tenant_id, l.action, l.resource, l.resource_id, l.details, l.ip_address, l.created_at,
                    u.name as user_name, u.email as user_email,
                    t.name as tenant_name,
                    CASE 
                        WHEN l.resource = 'user' THEN ru.name
                        WHEN l.resource = 'tenant' THEN rt.name
                        WHEN l.resource = 'roles' THEN rr.name
                        WHEN l.resource = 'settings' THEN l.resource_id
                        ELSE l.resource_id
                    END as resource_name
                FROM audit_logs l
                LEFT JOIN users u ON l.user_id::text = u.id::text
                LEFT JOIN tenants t ON l.tenant_id::text = t.id::text
                LEFT JOIN users ru ON l.resource = 'user' AND l.resource_id = ru.id::text
                LEFT JOIN tenants rt ON l.resource = 'tenant' AND l.resource_id = rt.id::text
                LEFT JOIN roles rr ON l.resource = 'roles' AND l.resource_id = rr.id::text
                WHERE 1=1 "#
            );

            if let Some(uid) = &filter.user_id {
                qb.push(" AND l.user_id::text = ");
                qb.push_bind(uid);
            }
            
            if let Some(tid) = &filter.tenant_id {
                qb.push(" AND l.tenant_id::text = ");
                qb.push_bind(tid);
            }

            if let Some(action) = &filter.action {
                qb.push(" AND l.action = ");
                qb.push_bind(action);
            }

            if let Some(date_from) = filter.date_from {
                qb.push(" AND l.created_at >= ");
                qb.push_bind(date_from);
            }
            
            if let Some(date_to) = filter.date_to {
                qb.push(" AND l.created_at <= ");
                qb.push_bind(date_to);
            }

            if let Some(search) = &filter.search {
                let pattern = format!("%{}%", search);
                qb.push(" AND (l.resource ILIKE ");
                qb.push_bind(pattern.clone());
                qb.push(" OR l.details ILIKE ");
                qb.push_bind(pattern.clone());
                qb.push(" OR u.name ILIKE ");
                qb.push_bind(pattern);
                qb.push(")");
            }

            // Count query clone before limit/offset
            // Note: QueryBuilder doesn't easily clone, so we construct count separately or run window function.
            // For simplicity, we'll run a separate count query with same WHERE clauses or just simple string construction for count if needed.
            // Actually, let's just get the count first using a similar builder.
            
            let mut count_qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT COUNT(*) FROM audit_logs l LEFT JOIN users u ON l.user_id::text = u.id::text WHERE 1=1 ");
             if let Some(uid) = &filter.user_id { count_qb.push(" AND l.user_id::text = "); count_qb.push_bind(uid); }
             if let Some(tid) = &filter.tenant_id { count_qb.push(" AND l.tenant_id::text = "); count_qb.push_bind(tid); }
             if let Some(action) = &filter.action { count_qb.push(" AND l.action = "); count_qb.push_bind(action); }
             if let Some(date_from) = filter.date_from { count_qb.push(" AND l.created_at >= "); count_qb.push_bind(date_from); }
             if let Some(date_to) = filter.date_to { count_qb.push(" AND l.created_at <= "); count_qb.push_bind(date_to); }
             if let Some(search) = &filter.search {
                let pattern = format!("%{}%", search);
                count_qb.push(" AND (l.resource ILIKE ");
                count_qb.push_bind(pattern.clone());
                count_qb.push(" OR l.details ILIKE ");
                count_qb.push_bind(pattern.clone());
                count_qb.push(" OR u.name ILIKE ");
                count_qb.push_bind(pattern);
                count_qb.push(")");
            }

            let count: i64 = count_qb.build().fetch_one(&self.pool).await?.try_get(0)?;

            // Ordering and pagination
            qb.push(" ORDER BY l.created_at DESC LIMIT ");
            qb.push_bind(per_page as i64);
            qb.push(" OFFSET ");
            qb.push_bind(offset as i64);

            let logs = qb.build_query_as::<crate::models::AuditLogResponse>().fetch_all(&self.pool).await
                .map_err(|e| {
                    tracing::error!("Failed to fetch audit logs: {}", e);
                    crate::error::AppError::Internal(e.to_string())
                })?;
            
            return Ok((logs, count));
        }

        // --- SQLite Implementation ---
        #[cfg(feature = "sqlite")]
        {
             use sqlx::{Sqlite, QueryBuilder, Row};
            
            let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
                r#"SELECT 
                    l.id, l.user_id, l.tenant_id, l.action, l.resource, l.resource_id, l.details, l.ip_address, l.created_at,
                    u.name as user_name, u.email as user_email,
                    t.name as tenant_name,
                    CASE 
                        WHEN l.resource = 'user' THEN ru.name
                        WHEN l.resource = 'tenant' THEN rt.name
                        WHEN l.resource = 'roles' THEN rr.name
                        WHEN l.resource = 'settings' THEN l.resource_id
                        ELSE l.resource_id
                    END as resource_name
                FROM audit_logs l
                LEFT JOIN users u ON l.user_id = u.id
                LEFT JOIN tenants t ON l.tenant_id = t.id
                LEFT JOIN users ru ON l.resource = 'user' AND l.resource_id = ru.id
                LEFT JOIN tenants rt ON l.resource = 'tenant' AND l.resource_id = rt.id
                LEFT JOIN roles rr ON l.resource = 'roles' AND l.resource_id = rr.id
                WHERE 1=1 "#
            );

            if let Some(uid) = &filter.user_id {
                qb.push(" AND l.user_id = ");
                qb.push_bind(uid);
            }
             if let Some(tid) = &filter.tenant_id {
                qb.push(" AND l.tenant_id = ");
                qb.push_bind(tid);
            }
            if let Some(action) = &filter.action {
                qb.push(" AND l.action = ");
                qb.push_bind(action);
            }
            if let Some(date_from) = filter.date_from {
                qb.push(" AND l.created_at >= ");
                qb.push_bind(date_from.to_rfc3339());
            }
            if let Some(date_to) = filter.date_to {
                qb.push(" AND l.created_at <= ");
                qb.push_bind(date_to.to_rfc3339());
            }
            if let Some(search) = &filter.search {
                let pattern = format!("%{}%", search);
                qb.push(" AND (l.resource LIKE ");
                qb.push_bind(pattern.clone());
                qb.push(" OR l.details LIKE ");
                qb.push_bind(pattern.clone());
                qb.push(" OR u.name LIKE ");
                qb.push_bind(pattern);
                qb.push(")");
            }

            // Count
            let mut count_qb: QueryBuilder<Sqlite> = QueryBuilder::new("SELECT COUNT(*) FROM audit_logs l LEFT JOIN users u ON l.user_id = u.id WHERE 1=1 ");
            if let Some(uid) = &filter.user_id { count_qb.push(" AND l.user_id = "); count_qb.push_bind(uid); }
            if let Some(tid) = &filter.tenant_id { count_qb.push(" AND l.tenant_id = "); count_qb.push_bind(tid); }
            if let Some(action) = &filter.action { count_qb.push(" AND l.action = "); count_qb.push_bind(action); }
            if let Some(date_from) = filter.date_from { count_qb.push(" AND l.created_at >= "); count_qb.push_bind(date_from.to_rfc3339()); }
            if let Some(date_to) = filter.date_to { count_qb.push(" AND l.created_at <= "); count_qb.push_bind(date_to.to_rfc3339()); }
            if let Some(search) = &filter.search {
                let pattern = format!("%{}%", search);
                count_qb.push(" AND (l.resource LIKE ");
                count_qb.push_bind(pattern.clone());
                count_qb.push(" OR l.details LIKE ");
                count_qb.push_bind(pattern.clone());
                count_qb.push(" OR u.name LIKE ");
                count_qb.push_bind(pattern);
                count_qb.push(")");
            }
            
            let count: i64 = count_qb.build().fetch_one(&self.pool).await?.try_get(0)?;

            // Order Limit Offset
            qb.push(" ORDER BY l.created_at DESC LIMIT ");
            qb.push_bind(per_page as i64);
            qb.push(" OFFSET ");
            qb.push_bind(offset as i64);

            let logs = qb.build_query_as::<crate::models::AuditLogResponse>().fetch_all(&self.pool).await
                 .map_err(|e| {
                    tracing::error!("Failed to fetch audit logs: {}", e);
                    crate::error::AppError::Internal(e.to_string())
                })?;
            
            Ok((logs, count))
        }
    }
}
