//! Settings Service

use crate::db::connection::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{Setting, UpsertSettingDto};
use chrono::Utc;

/// Settings service for key-value configuration
#[derive(Clone)]
pub struct SettingsService {
    pool: DbPool,
}

impl SettingsService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get all settings for a tenant (or global if tenant_id is None)
    pub async fn get_all(&self, tenant_id: Option<&str>) -> AppResult<Vec<Setting>> {
        let settings = if let Some(tid) = tenant_id {
            sqlx::query_as("SELECT * FROM settings WHERE tenant_id = $1 ORDER BY key")
                .bind(tid)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT * FROM settings WHERE tenant_id IS NULL ORDER BY key")
                .fetch_all(&self.pool)
                .await?
        };

        Ok(settings)
    }

    /// Get setting by key and tenant
    pub async fn get_by_key(&self, tenant_id: Option<&str>, key: &str) -> AppResult<Option<Setting>> {
        let setting = if let Some(tid) = tenant_id {
            sqlx::query_as("SELECT * FROM settings WHERE tenant_id = $1 AND key = $2")
                .bind(tid)
                .bind(key)
                .fetch_optional(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT * FROM settings WHERE tenant_id IS NULL AND key = $1")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?
        };

        Ok(setting)
    }

    /// Get setting value by key and tenant
    pub async fn get_value(&self, tenant_id: Option<&str>, key: &str) -> AppResult<Option<String>> {
        let setting = self.get_by_key(tenant_id, key).await?;
        Ok(setting.map(|s| s.value))
    }

    /// Upsert (insert or update) setting for a tenant
    pub async fn upsert(&self, tenant_id: Option<String>, dto: UpsertSettingDto) -> AppResult<Setting> {
        let now = Utc::now();

        // Check if setting exists
        let existing = self.get_by_key(tenant_id.as_deref(), &dto.key).await?;

        if let Some(mut setting) = existing {
            // Update existing
            setting.value = dto.value;
            setting.description = dto.description;
            setting.updated_at = now;

            if let Some(tid) = &tenant_id {
                let query = sqlx::query(
                    "UPDATE settings SET value = $1, description = $2, updated_at = $3 WHERE tenant_id = $4 AND key = $5"
                )
                .bind(&setting.value)
                .bind(&setting.description);

                #[cfg(feature = "postgres")]
                let query = query.bind(setting.updated_at);
                #[cfg(not(feature = "postgres"))]
                let query = query.bind(setting.updated_at.to_rfc3339());

                query.bind(tid).bind(&setting.key).execute(&self.pool).await?;
            } else {
                let query = sqlx::query(
                    "UPDATE settings SET value = $1, description = $2, updated_at = $3 WHERE tenant_id IS NULL AND key = $4"
                )
                .bind(&setting.value)
                .bind(&setting.description);

                #[cfg(feature = "postgres")]
                let query = query.bind(setting.updated_at);
                #[cfg(not(feature = "postgres"))]
                let query = query.bind(setting.updated_at.to_rfc3339());

                query.bind(&setting.key).execute(&self.pool).await?;
            }

            Ok(setting)
        } else {
            // Insert new
            let setting = Setting::new(tenant_id.clone(), dto.key, dto.value, dto.description);

            let query = sqlx::query(
                r#"
                INSERT INTO settings (id, tenant_id, key, value, description, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(&setting.id)
            .bind(&setting.tenant_id)
            .bind(&setting.key)
            .bind(&setting.value)
            .bind(&setting.description);

            #[cfg(feature = "postgres")]
            let query = query
                .bind(setting.created_at)
                .bind(setting.updated_at);

            #[cfg(not(feature = "postgres"))]
            let query = query
                .bind(setting.created_at.to_rfc3339())
                .bind(setting.updated_at.to_rfc3339());

            query.execute(&self.pool).await?;

            Ok(setting)
        }
    }

    /// Delete setting by key and tenant
    pub async fn delete(&self, tenant_id: Option<&str>, key: &str) -> AppResult<()> {
        let result = if let Some(tid) = tenant_id {
            sqlx::query("DELETE FROM settings WHERE tenant_id = $1 AND key = $2")
                .bind(tid)
                .bind(key)
                .execute(&self.pool)
                .await?
        } else {
            sqlx::query("DELETE FROM settings WHERE tenant_id IS NULL AND key = $1")
                .bind(key)
                .execute(&self.pool)
                .await?
        };

        if result.rows_affected() == 0 {
            return Err(AppError::Validation(format!("Setting '{}' not found", key)));
        }

        Ok(())
    }
}
