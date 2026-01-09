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

    /// Get all settings
    pub async fn get_all(&self) -> AppResult<Vec<Setting>> {
        let settings = sqlx::query_as("SELECT * FROM settings ORDER BY key")
            .fetch_all(&self.pool)
            .await?;

        Ok(settings)
    }

    /// Get setting by key
    pub async fn get_by_key(&self, key: &str) -> AppResult<Option<Setting>> {
        let setting = sqlx::query_as("SELECT * FROM settings WHERE key = $1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        Ok(setting)
    }

    /// Get setting value by key
    pub async fn get_value(&self, key: &str) -> AppResult<Option<String>> {
        let setting: Option<Setting> = sqlx::query_as("SELECT * FROM settings WHERE key = $1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        Ok(setting.map(|s| s.value))
    }

    /// Upsert (insert or update) setting
    pub async fn upsert(&self, dto: UpsertSettingDto) -> AppResult<Setting> {
        let now = Utc::now();

        // Check if setting exists
        let existing = self.get_by_key(&dto.key).await?;

        if let Some(mut setting) = existing {
            // Update existing
            setting.value = dto.value;
            setting.description = dto.description;
            setting.updated_at = now;

            let query = sqlx::query(
                "UPDATE settings SET value = $1, description = $2, updated_at = $3 WHERE key = $4"
            )
            .bind(&setting.value)
            .bind(&setting.description);

            #[cfg(feature = "postgres")]
            let query = query.bind(setting.updated_at);

            #[cfg(not(feature = "postgres"))]
            let query = query.bind(setting.updated_at.to_rfc3339());

            query.bind(&setting.key)
            .execute(&self.pool)
            .await?;

            Ok(setting)
        } else {
            // Insert new
            let setting = Setting::new(dto.key, dto.value, dto.description);

            let query = sqlx::query(
                r#"
                INSERT INTO settings (id, key, value, description, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(&setting.id)
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

    /// Delete setting by key
    pub async fn delete(&self, key: &str) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM settings WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::Validation(format!("Setting '{}' not found", key)));
        }

        Ok(())
    }
}
