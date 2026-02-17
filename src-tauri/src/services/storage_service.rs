//! Storage Service for handling file uploads
use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::services::PlanService;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{config::Region, Client};
use chrono::Utc;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

#[derive(Debug)]
pub enum StorageContent {
    Local(PathBuf),
    S3(ByteStream),
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub driver: String,
    pub bucket: String,
    pub region: String,
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub public_url: String,
}

#[derive(Clone)]
pub struct StorageService {
    pool: DbPool,
    plan_service: PlanService,
    base_storage_path: PathBuf,
}

impl StorageService {
    #[cfg(feature = "postgres")]
    async fn apply_rls_context_tx_values(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        tenant_id: Option<&str>,
        is_super_admin: bool,
    ) -> AppResult<()> {
        let tenant_id = tenant_id.unwrap_or("").to_string();
        let is_superadmin = if is_super_admin { "true" } else { "false" };

        sqlx::query(
            "SELECT set_config('app.current_tenant_id', $1, true), set_config('app.current_is_superadmin', $2, true)",
        )
        .bind(tenant_id)
        .bind(is_superadmin)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn apply_rls_context_tx_values(
        _tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        _tenant_id: Option<&str>,
        _is_super_admin: bool,
    ) -> AppResult<()> {
        Ok(())
    }

    pub fn new(pool: DbPool, plan_service: PlanService, app_data_dir: PathBuf) -> Self {
        let storage_path = app_data_dir.join("uploads");
        if !storage_path.exists() {
            std::fs::create_dir_all(&storage_path).unwrap_or_else(|e| {
                eprintln!("Failed to create storage directory: {}", e);
            });
        }

        Self {
            pool,
            plan_service,
            base_storage_path: storage_path,
        }
    }

    /// Get file content stream (Local path or S3 Stream)
    pub async fn get_file_content(
        &self,
        file_id: &str,
    ) -> AppResult<(crate::models::FileRecord, StorageContent)> {
        let file = self.get_file(file_id).await?;

        if file.storage_provider == "local" {
            let path = PathBuf::from(&file.path);
            if !path.exists() {
                return Err(AppError::NotFound("File not found on disk".to_string()));
            }
            Ok((file, StorageContent::Local(path)))
        } else if file.storage_provider == "s3" || file.storage_provider == "r2" {
            let config = self.get_storage_config(&file.tenant_id).await?;
            let client = self.get_s3_client(&config).await;

            let output = client
                .get_object()
                .bucket(&config.bucket)
                .key(&file.path)
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to get S3 object: {}", e)))?;

            Ok((file, StorageContent::S3(output.body)))
        } else {
            Err(AppError::Internal("Unknown storage provider".to_string()))
        }
    }

    /// Get storage configuration for a tenant (prioritizing tenant settings over global)
    async fn get_storage_config(&self, tenant_id: &str) -> AppResult<StorageConfig> {
        #[cfg(feature = "postgres")]
        let keys = vec![
            "storage_driver",
            "storage_s3_bucket",
            "storage_s3_region",
            "storage_s3_endpoint",
            "storage_s3_access_key",
            "storage_s3_secret_key",
            "storage_s3_public_url",
        ];

        let mut config = StorageConfig {
            driver: "local".to_string(),
            bucket: "".to_string(),
            region: "us-east-1".to_string(),
            endpoint: "".to_string(),
            access_key: "".to_string(),
            secret_key: "".to_string(),
            public_url: "".to_string(),
        };

        // Fetch Tenant Driver preference first
        #[cfg(feature = "postgres")]
        let tenant_driver: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE tenant_id = $1 AND key = 'storage_driver'",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        #[cfg(feature = "sqlite")]
        let tenant_driver: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE tenant_id = ? AND key = 'storage_driver'",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        let use_tenant_config = matches!(tenant_driver.as_deref(), Some("s3") | Some("r2"));

        #[cfg(feature = "postgres")]
        let rows: Vec<(String, String)> = if use_tenant_config {
            if let Some(td) = tenant_driver {
                config.driver = td;
            }
            sqlx::query_as("SELECT key, value FROM settings WHERE tenant_id = $1 AND key = ANY($2)")
                .bind(tenant_id)
                .bind(&keys)
                .fetch_all(&self.pool)
                .await
                .unwrap_or_default()
        } else {
            sqlx::query_as(
                "SELECT key, value FROM settings WHERE tenant_id IS NULL AND key = ANY($1)",
            )
            .bind(&keys)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
        };

        #[cfg(feature = "sqlite")]
        let rows: Vec<(String, String)> = if use_tenant_config {
            config.driver = tenant_driver.unwrap();
            // SQLite doesn't support ANY, so we use IN with individual bindings
            sqlx::query_as(
                "SELECT key, value FROM settings WHERE tenant_id = ? AND key IN ('storage_driver', 'storage_s3_bucket', 'storage_s3_region', 'storage_s3_endpoint', 'storage_s3_access_key', 'storage_s3_secret_key', 'storage_s3_public_url')"
            )
                .bind(tenant_id)
                .fetch_all(&self.pool).await.unwrap_or_default()
        } else {
            sqlx::query_as(
                "SELECT key, value FROM settings WHERE tenant_id IS NULL AND key IN ('storage_driver', 'storage_s3_bucket', 'storage_s3_region', 'storage_s3_endpoint', 'storage_s3_access_key', 'storage_s3_secret_key', 'storage_s3_public_url')"
            )
                .fetch_all(&self.pool).await.unwrap_or_default()
        };

        for (k, v) in rows {
            match k.as_str() {
                "storage_driver" => {
                    if !use_tenant_config {
                        config.driver = v
                    }
                }
                "storage_s3_bucket" => config.bucket = v,
                "storage_s3_region" => config.region = v,
                "storage_s3_endpoint" => config.endpoint = v,
                "storage_s3_access_key" => config.access_key = v,
                "storage_s3_secret_key" => config.secret_key = v,
                "storage_s3_public_url" => config.public_url = v,
                _ => {}
            }
        }

        Ok(config)
    }

    /// Create S3 Client from config
    async fn get_s3_client(&self, config: &StorageConfig) -> Client {
        let region = Region::new(config.region.clone());

        let creds = aws_sdk_s3::config::Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "static",
        );

        let mut builder = aws_sdk_s3::Config::builder()
            .region(region)
            .credentials_provider(creds)
            .behavior_version_latest();

        if !config.endpoint.is_empty() {
            builder = builder.endpoint_url(&config.endpoint);
        }

        let s3_config = builder.build();
        Client::from_conf(s3_config)
    }

    /// Prepare upload path (mkdir) and return (Absolute Path, Safe Filename, File ID)
    pub async fn prepare_upload_path(
        &self,
        tenant_id: &str,
        file_name: &str,
    ) -> AppResult<(PathBuf, String, String)> {
        let file_id = Uuid::new_v4().to_string();
        let ext = std::path::Path::new(file_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("bin");

        let now = Utc::now();
        let year = now.format("%Y").to_string();
        let month = now.format("%m").to_string();

        let relative_path = std::path::Path::new(tenant_id).join(&year).join(&month);
        let target_dir = self.base_storage_path.join(&relative_path);

        if !target_dir.exists() {
            fs::create_dir_all(&target_dir)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to create directory: {}", e)))?;
        }

        let safe_filename = format!("{}.{}", file_id, ext);
        let file_path = target_dir.join(&safe_filename);

        Ok((file_path, safe_filename, file_id))
    }

    /// Register a file that has been written to disk into the database
    #[allow(clippy::too_many_arguments)]
    pub async fn register_upload(
        &self,
        tenant_id: &str,
        file_id: &str,
        original_name: &str,
        safe_name: &str,
        file_path: &str,
        content_type: &str,
        size: i64,
        storage_provider: &str,
        user_id: Option<&str>,
        bypass_quota: bool,
    ) -> AppResult<crate::models::FileRecord> {
        #[cfg(feature = "postgres")]
        tracing::info!("[Storage] Mode: POSTGRES");
        #[cfg(feature = "sqlite")]
        tracing::info!("[Storage] Mode: SQLITE");

        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let query = r#"
            INSERT INTO file_records (id, tenant_id, name, original_name, path, size, content_type, storage_provider, uploaded_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            INSERT INTO file_records (id, tenant_id, name, original_name, path, size, content_type, storage_provider, uploaded_by, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        tracing::info!("[Storage] Executing INSERT for {}", file_id);

        #[cfg(feature = "postgres")]
        sqlx::query(query)
            .bind(file_id)
            .bind(tenant_id)
            .bind(safe_name)
            .bind(original_name)
            .bind(file_path)
            .bind(size)
            .bind(content_type)
            .bind(storage_provider)
            .bind(user_id)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("Insert failed: {}", e)))?;

        #[cfg(feature = "sqlite")]
        sqlx::query(query)
            .bind(file_id)
            .bind(tenant_id)
            .bind(safe_name)
            .bind(original_name)
            .bind(file_path)
            .bind(size)
            .bind(content_type)
            .bind(storage_provider)
            .bind(user_id)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("Insert failed: {}", e)))?;

        tracing::info!("[Storage] INSERT Success.");

        if !bypass_quota {
            sqlx::query("UPDATE tenants SET storage_usage = storage_usage + $1 WHERE id = $2")
                .bind(size)
                .bind(tenant_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(format!("Update usage failed: {}", e)))?;
        }

        Ok(crate::models::FileRecord {
            id: file_id.to_string(),
            tenant_id: tenant_id.to_string(),
            name: safe_name.to_string(),
            original_name: original_name.to_string(),
            path: file_path.to_string(),
            size,
            content_type: content_type.to_string(),
            storage_provider: storage_provider.to_string(),
            uploaded_by: user_id.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn upload(
        &self,
        tenant_id: &str,
        file_name: &str,
        content_type: &str,
        data: &[u8],
        user_id: Option<&str>,
    ) -> AppResult<crate::models::FileRecord> {
        let file_id = Uuid::new_v4().to_string();
        let ext = std::path::Path::new(file_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("bin");

        // --- Structured Storage Path: tenant_id/YYYY/MM/ ---
        let now = Utc::now();
        let year = now.format("%Y").to_string();
        let month = now.format("%m").to_string();

        // Construct relative path: tenant_id/2024/01/
        let relative_path = std::path::Path::new(tenant_id).join(&year).join(&month);
        let target_dir = self.base_storage_path.join(&relative_path);

        // Ensure directory exists
        if !target_dir.exists() {
            fs::create_dir_all(&target_dir).await.map_err(|e| {
                AppError::Internal(format!("Failed to create directory structure: {}", e))
            })?;
        }

        let safe_filename = format!("{}.{}", file_id, ext);
        let file_path = target_dir.join(&safe_filename);

        // --- Perform DB operations in a transaction ---
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to begin transaction: {}", e)))?;
        Self::apply_rls_context_tx_values(&mut tx, Some(tenant_id), false).await?;

        let size = data.len() as i64;

        let db_result = async {
            // Check Plan Storage Limit
            #[cfg(feature = "postgres")]
            let limit_gb = self
                .plan_service
                .get_feature_limit_with_conn(tenant_id, "max_storage_gb", &mut tx)
                .await?;

            #[cfg(feature = "sqlite")]
            let limit_gb = self.plan_service.get_feature_limit(tenant_id, "max_storage_gb").await?;

            if let Some(max_gb) = limit_gb {
                let max_bytes = (max_gb as u64) * 1024 * 1024 * 1024;

                #[cfg(feature = "postgres")]
                let current_usage: i64 = sqlx::query_scalar(
                    "SELECT storage_usage FROM tenants WHERE id = $1 FOR UPDATE",
                )
                .bind(tenant_id)
                .fetch_one(&mut *tx)
                .await?;

                #[cfg(feature = "sqlite")]
                let current_usage: i64 = sqlx::query_scalar(
                    "SELECT storage_usage FROM tenants WHERE id = ?",
                )
                .bind(tenant_id)
                .fetch_one(&mut *tx)
                .await?;

                if (current_usage as u64) + (size as u64) > max_bytes {
                    return Err(AppError::Validation(format!(
                        "Plan storage limit reached. Max {} GB allowed.",
                        max_gb
                    )));
                }
            }

            // Save metadata to DB
            // 'now' is captured from outer scope (used for path generation)

            #[cfg(feature="postgres")]
            let query = r#"
                INSERT INTO file_records (id, tenant_id, name, original_name, path, size, content_type, uploaded_by, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#;

            #[cfg(feature="sqlite")]
            let query = r#"
                INSERT INTO file_records (id, tenant_id, name, original_name, path, size, content_type, uploaded_by, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#;

            #[cfg(feature="postgres")]
            sqlx::query(query)
                .bind(&file_id)
                .bind(tenant_id)
                .bind(&safe_filename)
                .bind(file_name)
                .bind(file_path.to_string_lossy().to_string())
                .bind(size)
                .bind(content_type)
                .bind(user_id)
                .bind(now)
                .bind(now)
                .execute(&mut *tx)
                .await?;

            #[cfg(feature="sqlite")]
            sqlx::query(query)
                .bind(&file_id)
                .bind(tenant_id)
                .bind(&safe_filename)
                .bind(file_name)
                .bind(file_path.to_string_lossy().to_string())
                .bind(size)
                .bind(content_type)
                .bind(user_id)
                .bind(now.to_rfc3339())
                .bind(now.to_rfc3339())
                .execute(&mut *tx)
                .await?;

            // Update tenant's storage usage
            sqlx::query("UPDATE tenants SET storage_usage = storage_usage + $1 WHERE id = $2")
                .bind(size)
                .bind(tenant_id)
                .execute(&mut *tx)
                .await?;

            Ok(now)
        }.await;

        match db_result {
            Ok(now) => {
                // After DB is confirmed, write file to disk
                let mut file = fs::File::create(&file_path).await.map_err(|e| {
                    AppError::Internal(format!("Failed to create file on disk: {}", e))
                })?;

                file.write_all(data).await.map_err(|e| {
                    AppError::Internal(format!("Failed to write data to disk: {}", e))
                })?;

                tx.commit().await.map_err(|e| {
                    AppError::Internal(format!("Failed to commit transaction: {}", e))
                })?;

                Ok(crate::models::FileRecord {
                    id: file_id,
                    tenant_id: tenant_id.to_string(),
                    name: safe_filename,
                    original_name: file_name.to_string(),
                    path: file_path.to_string_lossy().to_string(),
                    size,
                    content_type: content_type.to_string(),
                    storage_provider: "local".to_string(), // Default for direct upload
                    uploaded_by: user_id.map(|s| s.to_string()),
                    created_at: now,
                    updated_at: now,
                })
            }
            Err(e) => {
                tx.rollback()
                    .await
                    .unwrap_or_else(|e| eprintln!("Failed to rollback transaction: {}", e));
                Err(e)
            }
        }
    }

    /// List all files (Admin)
    pub async fn list_all_files(
        &self,
        page: u32,
        per_page: u32,
        search: Option<String>,
    ) -> AppResult<(Vec<crate::models::FileRecord>, i64)> {
        let offset = (page.saturating_sub(1)) * per_page;

        #[cfg(feature = "postgres")]
        {
            use sqlx::{Postgres, QueryBuilder, Row};

            let mut qb: QueryBuilder<Postgres> =
                QueryBuilder::new("SELECT * FROM file_records WHERE 1=1 ");

            if let Some(s) = &search {
                let pattern = format!("%{}%", s);
                qb.push(" AND (name ILIKE ");
                qb.push_bind(pattern.clone());
                qb.push(" OR original_name ILIKE ");
                qb.push_bind(pattern);
                qb.push(")");
            }

            // Count
            let mut count_qb: QueryBuilder<Postgres> =
                QueryBuilder::new("SELECT COUNT(*) FROM file_records WHERE 1=1 ");
            if let Some(s) = &search {
                let pattern = format!("%{}%", s);
                count_qb.push(" AND (name ILIKE ");
                count_qb.push_bind(pattern.clone());
                count_qb.push(" OR original_name ILIKE ");
                count_qb.push_bind(pattern);
                count_qb.push(")");
            }
            let count: i64 = count_qb.build().fetch_one(&self.pool).await?.try_get(0)?;

            // Data
            qb.push(" ORDER BY created_at DESC LIMIT ");
            qb.push_bind(per_page as i64);
            qb.push(" OFFSET ");
            qb.push_bind(offset as i64);

            let files = qb
                .build_query_as::<crate::models::FileRecord>()
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            Ok((files, count))
        }

        #[cfg(feature = "sqlite")]
        {
            use sqlx::{QueryBuilder, Row, Sqlite};

            let mut qb: QueryBuilder<Sqlite> =
                QueryBuilder::new("SELECT * FROM file_records WHERE 1=1 ");

            if let Some(s) = &search {
                let pattern = format!("%{}%", s);
                qb.push(" AND (name LIKE ");
                qb.push_bind(pattern.clone());
                qb.push(" OR original_name LIKE ");
                qb.push_bind(pattern);
                qb.push(")");
            }

            // Rebuild count query
            let mut count_qb: QueryBuilder<Sqlite> =
                QueryBuilder::new("SELECT COUNT(*) FROM file_records WHERE 1=1 ");
            if let Some(s) = &search {
                let pattern = format!("%{}%", s);
                count_qb.push(" AND (name LIKE ");
                count_qb.push_bind(pattern.clone());
                count_qb.push(" OR original_name LIKE ");
                count_qb.push_bind(pattern);
                count_qb.push(")");
            }
            let count: i64 = count_qb.build().fetch_one(&self.pool).await?.try_get(0)?;

            // Data
            qb.push(" ORDER BY created_at DESC LIMIT ");
            qb.push_bind(per_page as i64);
            qb.push(" OFFSET ");
            qb.push_bind(offset as i64);

            let files = qb
                .build_query_as::<crate::models::FileRecord>()
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            Ok((files, count))
        }
    }

    /// List files for a specific tenant
    pub async fn list_tenant_files(
        &self,
        tenant_id: &str,
        page: u32,
        per_page: u32,
        search: Option<String>,
    ) -> AppResult<(Vec<crate::models::FileRecord>, i64)> {
        let offset = (page.saturating_sub(1)) * per_page;

        #[cfg(feature = "postgres")]
        {
            let total_count: i64 = if let Some(s) = search.as_ref() {
                let pattern = format!("%{}%", s);
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM file_records WHERE tenant_id = $1 AND (name ILIKE $2 OR original_name ILIKE $3)",
                )
                .bind(tenant_id)
                .bind(&pattern)
                .bind(&pattern)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::Internal(format!("Count query failed: {}", e)))?
            } else {
                sqlx::query_scalar("SELECT COUNT(*) FROM file_records WHERE tenant_id = $1")
                    .bind(tenant_id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| AppError::Internal(format!("Count query failed: {}", e)))?
            };

            // Data query - Use raw query and manual mapping to avoid query_as issue
            let data_rows: Vec<sqlx::postgres::PgRow> = if let Some(s) = search.as_ref() {
                let pattern = format!("%{}%", s);
                sqlx::query(
                    "SELECT * FROM file_records WHERE tenant_id = $1 AND (name ILIKE $2 OR original_name ILIKE $3) ORDER BY created_at DESC LIMIT $4 OFFSET $5"
                )
                .bind(tenant_id)
                .bind(&pattern)
                .bind(&pattern)
                .bind(per_page as i64)
                .bind(offset as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Internal(format!("Data query failed: {}", e)))?
            } else {
                sqlx::query(
                    "SELECT * FROM file_records WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
                )
                .bind(tenant_id)
                .bind(per_page as i64)
                .bind(offset as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Internal(format!("Data query failed: {}", e)))?
            };

            // Manual mapping
            let files: Vec<crate::models::FileRecord> = data_rows
                .iter()
                .filter_map(|row| {
                    use chrono::{DateTime, Utc};
                    use sqlx::Row;

                    let id: String = row.try_get("id").ok()?;
                    let tenant_id: String = row.try_get("tenant_id").ok()?;
                    let name: String = row.try_get("name").ok()?;
                    let original_name: String = row.try_get("original_name").ok()?;
                    let path: String = row.try_get("path").ok()?;
                    let size: i64 = row.try_get("size").ok()?;
                    let content_type: String = row.try_get("content_type").ok()?;
                    let storage_provider: String = row
                        .try_get("storage_provider")
                        .unwrap_or("local".to_string());
                    let uploaded_by: Option<String> = row.try_get("uploaded_by").ok();
                    let created_at: DateTime<Utc> = row.try_get("created_at").ok()?;
                    let updated_at: DateTime<Utc> = row.try_get("updated_at").ok()?;

                    Some(crate::models::FileRecord {
                        id,
                        tenant_id,
                        name,
                        original_name,
                        path,
                        size,
                        content_type,
                        storage_provider,
                        uploaded_by,
                        created_at,
                        updated_at,
                    })
                })
                .collect();

            Ok((files, total_count))
        }

        #[cfg(feature = "sqlite")]
        {
            // SQLite implementation
            // ... code continues ...
            use sqlx::{QueryBuilder, Row, Sqlite};

            let mut qb: QueryBuilder<Sqlite> =
                QueryBuilder::new("SELECT * FROM file_records WHERE tenant_id = ");
            qb.push_bind(tenant_id);

            if let Some(s) = &search {
                let pattern = format!("%{}%", s);
                qb.push(" AND (name LIKE ");
                qb.push_bind(pattern.clone());
                qb.push(" OR original_name LIKE ");
                qb.push_bind(pattern);
                qb.push(")");
            }

            // Rebuild count query
            let mut count_qb: QueryBuilder<Sqlite> =
                QueryBuilder::new("SELECT COUNT(*) FROM file_records WHERE tenant_id = ");
            count_qb.push_bind(tenant_id);

            if let Some(s) = &search {
                let pattern = format!("%{}%", s);
                count_qb.push(" AND (name LIKE ");
                count_qb.push_bind(pattern.clone());
                count_qb.push(" OR original_name LIKE ");
                count_qb.push_bind(pattern);
                count_qb.push(")");
            }
            let count: i64 = count_qb.build().fetch_one(&self.pool).await?.try_get(0)?;
            tracing::info!("[StorageService] Count result (SQLite): {}", count);

            // Data
            qb.push(" ORDER BY created_at DESC LIMIT ");
            qb.push_bind(per_page as i64);
            qb.push(" OFFSET ");
            qb.push_bind(offset as i64);

            let files = qb
                .build_query_as::<crate::models::FileRecord>()
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            Ok((files, count))
        }
    }

    /// Get file record by ID
    pub async fn get_file(&self, file_id: &str) -> AppResult<crate::models::FileRecord> {
        #[cfg(feature = "postgres")]
        let record = sqlx::query_as::<_, crate::models::FileRecord>(
            "SELECT * FROM file_records WHERE id = $1",
        )
        .bind(file_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::NotFound(format!("File not found: {}", e)))?;

        #[cfg(feature = "sqlite")]
        let record = sqlx::query_as::<_, crate::models::FileRecord>(
            "SELECT * FROM file_records WHERE id = ?",
        )
        .bind(file_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::NotFound(format!("File not found: {}", e)))?;

        Ok(record)
    }

    /// Delete file (Admin)
    pub async fn delete_file(&self, file_id: &str) -> AppResult<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to begin transaction: {}", e)))?;
        Self::apply_rls_context_tx_values(&mut tx, None, true).await?;

        #[cfg(feature = "postgres")]
        let record: Option<crate::models::FileRecord> =
            sqlx::query_as("SELECT * FROM file_records WHERE id = $1")
                .bind(file_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let record: Option<crate::models::FileRecord> =
            sqlx::query_as("SELECT * FROM file_records WHERE id = ?")
                .bind(file_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

        if let Some(file) = record {
            // 1. Delete from Storage Provider
            if file.storage_provider == "local" {
                let path = PathBuf::from(&file.path);
                if path.exists() {
                    fs::remove_file(path).await.ok();
                }
            } else if file.storage_provider == "s3" || file.storage_provider == "r2" {
                if let Ok(config) = self.get_storage_config(&file.tenant_id).await {
                    let client = self.get_s3_client(&config).await;
                    let _ = client
                        .delete_object()
                        .bucket(&config.bucket)
                        .key(&file.path)
                        .send()
                        .await;
                }
            }

            // 2. Remove from DB
            #[cfg(feature = "postgres")]
            sqlx::query("DELETE FROM file_records WHERE id = $1")
                .bind(file_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            #[cfg(feature = "sqlite")]
            sqlx::query("DELETE FROM file_records WHERE id = ?")
                .bind(file_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            // 3. Update usage (Only if local)
            if file.storage_provider == "local" {
                sqlx::query("UPDATE tenants SET storage_usage = storage_usage - $1 WHERE id = $2")
                    .bind(file.size)
                    .bind(&file.tenant_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }

            tx.commit()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to commit transaction: {}", e)))?;
        } else {
            tx.rollback().await.map_err(|e| {
                AppError::Internal(format!("Failed to rollback transaction: {}", e))
            })?;
        }

        Ok(())
    }

    /// Delete file (Tenant)
    pub async fn delete_tenant_file(&self, file_id: &str, tenant_id: &str) -> AppResult<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to begin transaction: {}", e)))?;
        Self::apply_rls_context_tx_values(&mut tx, Some(tenant_id), false).await?;

        #[cfg(feature = "postgres")]
        let record: Option<crate::models::FileRecord> =
            sqlx::query_as("SELECT * FROM file_records WHERE id = $1 AND tenant_id = $2")
                .bind(file_id)
                .bind(tenant_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
        let record: Option<crate::models::FileRecord> =
            sqlx::query_as("SELECT * FROM file_records WHERE id = ? AND tenant_id = ?")
                .bind(file_id)
                .bind(tenant_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

        if let Some(file) = record {
            if file.storage_provider == "local" {
                let path = PathBuf::from(&file.path);
                if path.exists() {
                    fs::remove_file(path).await.ok();
                }
            } else if file.storage_provider == "s3" || file.storage_provider == "r2" {
                if let Ok(config) = self.get_storage_config(&file.tenant_id).await {
                    let client = self.get_s3_client(&config).await;
                    let _ = client
                        .delete_object()
                        .bucket(&config.bucket)
                        .key(&file.path)
                        .send()
                        .await;
                }
            }

            #[cfg(feature = "postgres")]
            sqlx::query("DELETE FROM file_records WHERE id = $1")
                .bind(file_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            #[cfg(feature = "sqlite")]
            sqlx::query("DELETE FROM file_records WHERE id = ?")
                .bind(file_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            if file.storage_provider == "local" {
                sqlx::query("UPDATE tenants SET storage_usage = storage_usage - $1 WHERE id = $2")
                    .bind(file.size)
                    .bind(tenant_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }

            tx.commit()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to commit transaction: {}", e)))?;
        } else {
            tx.rollback().await.map_err(|e| {
                AppError::Internal(format!("Failed to rollback transaction: {}", e))
            })?;
            return Err(AppError::NotFound(
                "File not found or access denied".to_string(),
            ));
        }

        Ok(())
    }

    /// Chunked upload methods.
    ///
    /// Initialize a chunked upload session.
    ///
    /// Returns: upload_id
    pub async fn init_chunk_session(&self) -> AppResult<String> {
        let upload_id = Uuid::new_v4().to_string();
        let temp_dir = self.base_storage_path.join("temp");

        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to create temp dir: {}", e)))?;
        }

        let temp_file_path = temp_dir.join(&upload_id);

        // Create empty file
        fs::File::create(&temp_file_path)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create temp file: {}", e)))?;

        Ok(upload_id)
    }

    /// 2. Process a chunk (Append to file)
    pub async fn process_chunk(&self, upload_id: &str, chunk_data: &[u8]) -> AppResult<u64> {
        let temp_path = self.base_storage_path.join("temp").join(upload_id);

        if !temp_path.exists() {
            return Err(AppError::NotFound(
                "Upload session expired or not found".to_string(),
            ));
        }

        // Open in Append mode
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&temp_path)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to open temp file: {}", e)))?;

        file.write_all(chunk_data)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to write chunk: {}", e)))?;

        // Return current size
        let metadata = file
            .metadata()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get metadata: {}", e)))?;

        Ok(metadata.len())
    }

    /// 3. Finalize chunked upload
    pub async fn complete_chunk_session(
        &self,
        tenant_id: &str,
        upload_id: &str,
        file_name: &str,
        content_type: &str,
        user_id: Option<&str>,
    ) -> AppResult<crate::models::FileRecord> {
        tracing::info!("[Storage] Completing chunk session: {}", upload_id);

        let temp_path = self.base_storage_path.join("temp").join(upload_id);

        if !temp_path.exists() {
            return Err(AppError::NotFound("Upload session not found".to_string()));
        }

        let metadata = fs::metadata(&temp_path)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to read temp file: {}", e)))?;
        let size = metadata.len();

        // 1. Get Configuration
        let config = self.get_storage_config(tenant_id).await?;
        let is_custom_storage = config.driver == "s3" || config.driver == "r2";

        let final_path: String;
        let safe_name: String;
        let file_id: String;

        // 2. Process File based on Driver
        if is_custom_storage {
            // --- S3 / R2 Upload ---
            let client = self.get_s3_client(&config).await;

            let file_uuid = Uuid::new_v4().to_string();
            let ext = std::path::Path::new(file_name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("bin");
            let now = Utc::now();
            // Structure: tenant_id/YYYY/MM/uuid.ext
            let key = format!(
                "{}/{}/{}/{}.{}",
                tenant_id,
                now.format("%Y"),
                now.format("%m"),
                file_uuid,
                ext
            );

            safe_name = format!("{}.{}", file_uuid, ext);
            file_id = file_uuid;
            final_path = key.clone();

            let body = ByteStream::from_path(&temp_path).await.map_err(|e| {
                AppError::Internal(format!("Failed to read file for S3 upload: {}", e))
            })?;

            client
                .put_object()
                .bucket(&config.bucket)
                .key(&key)
                .body(body)
                .content_type(content_type)
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("S3 Upload Failed: {}", e)))?;

            fs::remove_file(&temp_path).await.ok();
        } else {
            // --- Local Storage ---
            let (dest_path, s_name, f_id) = self.prepare_upload_path(tenant_id, file_name).await?;

            fs::rename(&temp_path, &dest_path)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to move final file: {}", e)))?;

            final_path = dest_path.to_string_lossy().to_string();
            safe_name = s_name;
            file_id = f_id;
        }

        // 3. Register in DB
        let res = self
            .register_upload(
                tenant_id,
                &file_id,
                file_name,
                &safe_name,
                &final_path,
                content_type,
                size as i64,
                &config.driver,
                user_id,
                is_custom_storage, // bypass quota if custom
            )
            .await;

        if let Err(ref e) = res {
            tracing::error!("[Storage] DB Registration Failed: {}", e);
        } else {
            tracing::info!("[Storage] DB Registration Success");
        }

        res
    }
}
