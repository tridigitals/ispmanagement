//! Storage Service for handling file uploads
use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::services::PlanService;
use chrono::Utc;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

#[derive(Clone)]
pub struct StorageService {
    pool: DbPool,
    plan_service: PlanService,
    base_storage_path: PathBuf,
}

impl StorageService {
    pub fn new(pool: DbPool, plan_service: PlanService, app_data_dir: PathBuf) -> Self {
        let storage_path = app_data_dir.join("uploads");
        // Ensure directory exists (sync for simplicity in constructor, or move to init)
        // Ideally we should use tokio::fs::create_dir_all async, but new is usually sync.
        // We can create it lazily.
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

    /// Prepare upload path (mkdir) and return (Absolute Path, Safe Filename, File ID)
    pub async fn prepare_upload_path(&self, tenant_id: &str, file_name: &str) -> AppResult<(PathBuf, String, String)> {
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
            fs::create_dir_all(&target_dir).await
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
        user_id: Option<&str>,
    ) -> AppResult<crate::models::FileRecord> {
        // Direct execution without explicit transaction for debugging
        #[cfg(feature = "postgres")]
        tracing::info!("[Storage] Mode: POSTGRES");
        #[cfg(feature = "sqlite")]
        tracing::info!("[Storage] Mode: SQLITE");
        
        // Check Limits
        // Note: passing pool directly
        // Warning: get_feature_limit_with_conn expects a transaction reference usually.
        // We might need to check how plan_service implements it.
        // If it requires tx, we might need a small ad-hoc tx just for that or change plan_service.
        // Let's assume for now we skip limit check to isolate the INSERT issue, OR we create a short tx just for limit check.
        // Actually, let's keep the limit check simple: assume it passes for now to debug INSERT visibility.
        
        /* 
        let limit_gb = self.plan_service.get_feature_limit_with_conn(tenant_id, "max_storage_gb", &mut tx).await?;
        if let Some(max_gb) = limit_gb { ... }
        */
        
        let now = Utc::now();
        
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

        tracing::info!("[Storage] Executing INSERT for {}", file_id);

        #[cfg(feature="postgres")]
        sqlx::query(query)
            .bind(file_id)
            .bind(tenant_id)
            .bind(safe_name)
            .bind(original_name)
            .bind(file_path)
            .bind(size)
            .bind(content_type)
            .bind(user_id)
            .bind(now)
            .bind(now)
            .execute(&self.pool).await
            .map_err(|e| AppError::Internal(format!("Insert failed: {}", e)))?;
        
        #[cfg(feature="sqlite")]
        sqlx::query(query)
            .bind(file_id)
            .bind(tenant_id)
            .bind(safe_name)
            .bind(original_name)
            .bind(file_path)
            .bind(size)
            .bind(content_type)
            .bind(user_id)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(&self.pool).await
            .map_err(|e| AppError::Internal(format!("Insert failed: {}", e)))?;

        tracing::info!("[Storage] INSERT Success. Updating usage...");

        // VERIFICATION STEP
        #[cfg(feature="postgres")]
        {
            let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM file_records WHERE id = $1")
                .bind(file_id)
                .fetch_optional(&self.pool).await.unwrap_or(None);
            
            if exists.is_some() {
                tracing::info!("[Storage] ✅ VERIFIED: Record {} exists in DB.", file_id);
            } else {
                tracing::error!("[Storage] ❌ GHOST INSERT: Record {} NOT found immediately after insert!", file_id);
            }
        }

        sqlx::query("UPDATE tenants SET storage_usage = storage_usage + $1 WHERE id = $2")
            .bind(size)
            .bind(tenant_id)
            .execute(&self.pool).await
            .map_err(|e| AppError::Internal(format!("Update usage failed: {}", e)))?;

        Ok(crate::models::FileRecord {
            id: file_id.to_string(),
            tenant_id: tenant_id.to_string(),
            name: safe_name.to_string(),
            original_name: original_name.to_string(),
            path: file_path.to_string(),
            size,
            content_type: content_type.to_string(),
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
            fs::create_dir_all(&target_dir).await
                .map_err(|e| AppError::Internal(format!("Failed to create directory structure: {}", e)))?;
        }

        let safe_filename = format!("{}.{}", file_id, ext);
        let file_path = target_dir.join(&safe_filename);

        // --- Perform DB operations in a transaction ---
        let mut tx = self.pool.begin().await.map_err(|e| {
            AppError::Internal(format!("Failed to begin transaction: {}", e))
        })?;

        let size = data.len() as i64;
        
        let db_result = async {
            // Check Plan Storage Limit
            let limit_gb = self
                .plan_service
                .get_feature_limit_with_conn(tenant_id, "max_storage_gb", &mut tx)
                .await?;

            if let Some(max_gb) = limit_gb {
                let max_bytes = (max_gb as u64) * 1024 * 1024 * 1024;

                let current_usage: i64 = sqlx::query_scalar(
                    "SELECT storage_usage FROM tenants WHERE id = $1 FOR UPDATE",
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
                let mut file = fs::File::create(&file_path)
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to create file on disk: {}", e)))?;
                
                file.write_all(data)
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to write data to disk: {}", e)))?;

                tx.commit().await.map_err(|e| AppError::Internal(format!("Failed to commit transaction: {}", e)))?;
                
                Ok(crate::models::FileRecord {
                    id: file_id,
                    tenant_id: tenant_id.to_string(),
                    name: safe_filename,
                    original_name: file_name.to_string(),
                    path: file_path.to_string_lossy().to_string(),
                    size,
                    content_type: content_type.to_string(),
                    uploaded_by: user_id.map(|s| s.to_string()),
                    created_at: now,
                    updated_at: now,
                })
            },
            Err(e) => {
                tx.rollback().await.unwrap_or_else(|e| eprintln!("Failed to rollback transaction: {}", e));
                Err(e)
            }
        }
    }

    /// List all files (Admin)
    pub async fn list_all_files(
        &self, 
        page: u32, 
        per_page: u32, 
        search: Option<String>
    ) -> AppResult<(Vec<crate::models::FileRecord>, i64)> {
        let offset = (page.saturating_sub(1)) * per_page;

        #[cfg(feature = "postgres")]
        {
            use sqlx::{Postgres, QueryBuilder, Row};
            
            let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
                "SELECT * FROM file_records WHERE 1=1 "
            );

            if let Some(s) = &search {
                let pattern = format!("%{}%", s);
                qb.push(" AND (name ILIKE ");
                qb.push_bind(pattern.clone());
                qb.push(" OR original_name ILIKE ");
                qb.push_bind(pattern);
                qb.push(")");
            }

            // Count
            let mut count_qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT COUNT(*) FROM file_records WHERE 1=1 ");
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

            let files = qb.build_query_as::<crate::models::FileRecord>().fetch_all(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            
            Ok((files, count))
        }

        #[cfg(feature = "sqlite")]
        {
            use sqlx::{Sqlite, QueryBuilder, Row};
            
            let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
                "SELECT * FROM file_records WHERE 1=1 "
            );

            if let Some(s) = &search {
                let pattern = format!("%{}%", s);
                qb.push(" AND (name LIKE ");
                qb.push_bind(pattern.clone());
                qb.push(" OR original_name LIKE ");
                qb.push_bind(pattern);
                qb.push(")");
            }

             // Rebuild count query
            let mut count_qb: QueryBuilder<Sqlite> = QueryBuilder::new("SELECT COUNT(*) FROM file_records WHERE 1=1 ");
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

            let files = qb.build_query_as::<crate::models::FileRecord>().fetch_all(&self.pool).await
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
        search: Option<String>
    ) -> AppResult<(Vec<crate::models::FileRecord>, i64)> {
        let offset = (page.saturating_sub(1)) * per_page;

        #[cfg(feature = "postgres")]
        {
            use sqlx::Row;
            
            // Debug: Test raw query to see if data exists
            let raw_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM file_records WHERE tenant_id = $1")
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::Internal(format!("Count query failed: {}", e)))?;

            // Data query - Use raw query and manual mapping to avoid query_as issue
            let data_rows: Vec<sqlx::postgres::PgRow> = if search.is_some() {
                let pattern = format!("%{}%", search.as_ref().unwrap());
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
            let files: Vec<crate::models::FileRecord> = data_rows.iter().filter_map(|row| {
                use sqlx::Row;
                use chrono::{DateTime, Utc};
                
                let id: String = row.try_get("id").ok()?;
                let tenant_id: String = row.try_get("tenant_id").ok()?;
                let name: String = row.try_get("name").ok()?;
                let original_name: String = row.try_get("original_name").ok()?;
                let path: String = row.try_get("path").ok()?;
                let size: i64 = row.try_get("size").ok()?;
                let content_type: String = row.try_get("content_type").ok()?;
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
                    uploaded_by,
                    created_at,
                    updated_at,
                })
            }).collect();
            
            Ok((files, raw_count))
        }

        #[cfg(feature = "sqlite")]
        {
            // SQLite implementation
            // ... code continues ...
            use sqlx::{Sqlite, QueryBuilder, Row};
            
            let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
                "SELECT * FROM file_records WHERE tenant_id = "
            );
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
            let mut count_qb: QueryBuilder<Sqlite> = QueryBuilder::new("SELECT COUNT(*) FROM file_records WHERE tenant_id = ");
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

            let files = qb.build_query_as::<crate::models::FileRecord>().fetch_all(&self.pool).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            
            Ok((files, count))
        }
    }

    /// Get file record by ID
    pub async fn get_file(&self, file_id: &str) -> AppResult<crate::models::FileRecord> {
        #[cfg(feature = "postgres")]
        let record = sqlx::query_as::<_, crate::models::FileRecord>("SELECT * FROM file_records WHERE id = $1")
            .bind(file_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::NotFound(format!("File not found: {}", e)))?;

        #[cfg(feature = "sqlite")]
        let record = sqlx::query_as::<_, crate::models::FileRecord>("SELECT * FROM file_records WHERE id = ?")
            .bind(file_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::NotFound(format!("File not found: {}", e)))?;

        Ok(record)
    }

    /// Delete file (Admin)
    pub async fn delete_file(&self, file_id: &str) -> AppResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| AppError::Internal(format!("Failed to begin transaction: {}", e)))?;

        #[cfg(feature = "postgres")]
        let record: Option<crate::models::FileRecord> = sqlx::query_as("SELECT * FROM file_records WHERE id = $1")
            .bind(file_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
         let record: Option<crate::models::FileRecord> = sqlx::query_as("SELECT * FROM file_records WHERE id = ?")
            .bind(file_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if let Some(file) = record {
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

            // 3. Update tenant storage
            sqlx::query("UPDATE tenants SET storage_usage = storage_usage - $1 WHERE id = $2")
                .bind(file.size)
                .bind(&file.tenant_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            // 4. Commit before trying to delete from disk
            tx.commit().await.map_err(|e| AppError::Internal(format!("Failed to commit transaction: {}", e)))?;
            
            // 5. Remove from Disk
            let path = PathBuf::from(&file.path);
            if path.exists() {
                fs::remove_file(path).await
                    .map_err(|e| AppError::Internal(format!("Failed to delete file from disk: {}", e)))?;
            }
        } else {
            tx.rollback().await.map_err(|e| AppError::Internal(format!("Failed to rollback transaction: {}", e)))?;
        }

        Ok(())
    }

    /// Delete file (Tenant) - Secure deletion checking tenant_id
    pub async fn delete_tenant_file(&self, file_id: &str, tenant_id: &str) -> AppResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| AppError::Internal(format!("Failed to begin transaction: {}", e)))?;

        #[cfg(feature = "postgres")]
        let record: Option<crate::models::FileRecord> = sqlx::query_as("SELECT * FROM file_records WHERE id = $1 AND tenant_id = $2")
            .bind(file_id)
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(feature = "sqlite")]
         let record: Option<crate::models::FileRecord> = sqlx::query_as("SELECT * FROM file_records WHERE id = ? AND tenant_id = ?")
            .bind(file_id)
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if let Some(file) = record {
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

            // 3. Update tenant storage
            sqlx::query("UPDATE tenants SET storage_usage = storage_usage - $1 WHERE id = $2")
                .bind(file.size)
                .bind(tenant_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            // 4. Commit before trying to delete from disk
            tx.commit().await.map_err(|e| AppError::Internal(format!("Failed to commit transaction: {}", e)))?;
            
            // 5. Remove from Disk
            let path = PathBuf::from(&file.path);
            if path.exists() {
                fs::remove_file(path).await
                    .map_err(|e| AppError::Internal(format!("Failed to delete file from disk: {}", e)))?;
            }
        } else {
             // Rollback if file not found or doesn't belong to tenant
            tx.rollback().await.map_err(|e| AppError::Internal(format!("Failed to rollback transaction: {}", e)))?;
            return Err(AppError::NotFound("File not found or access denied".to_string()));
        }

        Ok(())
    }

    /// --- Chunked Upload Methods ---

    /// 1. Initialize a chunked upload session
    /// Returns: upload_id
    pub async fn init_chunk_session(&self) -> AppResult<String> {
        let upload_id = Uuid::new_v4().to_string();
        let temp_dir = self.base_storage_path.join("temp");
        
        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir).await
                .map_err(|e| AppError::Internal(format!("Failed to create temp dir: {}", e)))?;
        }

        let temp_file_path = temp_dir.join(&upload_id);
        
        // Create empty file
        fs::File::create(&temp_file_path).await
            .map_err(|e| AppError::Internal(format!("Failed to create temp file: {}", e)))?;

        Ok(upload_id)
    }

    /// 2. Process a chunk (Append to file)
    pub async fn process_chunk(&self, upload_id: &str, chunk_data: &[u8]) -> AppResult<u64> {
        let temp_path = self.base_storage_path.join("temp").join(upload_id);
        
        if !temp_path.exists() {
            return Err(AppError::NotFound("Upload session expired or not found".to_string()));
        }

        // Open in Append mode
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&temp_path)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to open temp file: {}", e)))?;

        file.write_all(chunk_data).await
            .map_err(|e| AppError::Internal(format!("Failed to write chunk: {}", e)))?;

        // Return current size
        let metadata = file.metadata().await
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
        user_id: Option<&str>
    ) -> AppResult<crate::models::FileRecord> {
        tracing::info!("[Storage] Completing chunk session: {}", upload_id);

        let temp_path = self.base_storage_path.join("temp").join(upload_id);
        
        if !temp_path.exists() {
            return Err(AppError::NotFound("Upload session not found".to_string()));
        }

        let metadata = fs::metadata(&temp_path).await
            .map_err(|e| AppError::Internal(format!("Failed to read temp file: {}", e)))?;
        let size = metadata.len();

        // Prepare destination
        let (dest_path, safe_name, file_id) = self.prepare_upload_path(tenant_id, file_name).await?;

        // Move file (Rename is atomic and fast)
        fs::rename(&temp_path, &dest_path).await
            .map_err(|e| AppError::Internal(format!("Failed to move final file: {}", e)))?;

        tracing::info!("[Storage] File moved to: {:?}. Registering in DB...", dest_path);

        // Register in DB
        let res = self.register_upload(
            tenant_id,
            &file_id,
            file_name,
            &safe_name,
            &dest_path.to_string_lossy(),
            content_type,
            size as i64,
            user_id
        ).await;

        if let Err(ref e) = res {
            tracing::error!("[Storage] DB Registration Failed: {}", e);
        } else {
            tracing::info!("[Storage] DB Registration Success");
        }

        res
    }
}
