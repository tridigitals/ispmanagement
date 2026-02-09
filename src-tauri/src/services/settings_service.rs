//! Settings Service

use crate::db::connection::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{Setting, UpsertSettingDto};
use crate::services::audit_service::AuditService;
use chrono::Utc;

/// Settings service for key-value configuration
#[derive(Clone)]
pub struct SettingsService {
    pool: DbPool,
    audit_service: AuditService,
}

impl SettingsService {
    pub fn new(pool: DbPool, audit_service: AuditService) -> Self {
        Self {
            pool,
            audit_service,
        }
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
    pub async fn get_by_key(
        &self,
        tenant_id: Option<&str>,
        key: &str,
    ) -> AppResult<Option<Setting>> {
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
    pub async fn upsert(
        &self,
        tenant_id: Option<String>,
        dto: UpsertSettingDto,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> AppResult<Setting> {
        let now = Utc::now();

        fn is_sensitive_setting_key(key: &str) -> bool {
            let k = key.trim();

            // Email: only secrets should be fully redacted.
            if matches!(k, "email_smtp_password") {
                return true;
            }

            // Payments: redact server/secret keys, but allow auditing non-secret toggles.
            if k.starts_with("payment_") {
                return matches!(
                    k,
                    "payment_midtrans_server_key"
                        | "payment_xendit_secret_key"
                        | "payment_stripe_secret_key"
                        | "payment_paypal_client_secret"
                ) || k.contains("secret")
                    || k.contains("server_key")
                    || k.contains("private_key")
                    || k.contains("client_secret");
            }

            // Storage / auth secrets.
            matches!(
                k,
                "storage_s3_access_key" | "storage_s3_secret_key" | "jwt_secret"
            ) || k.contains("secret")
                || k.contains("password")
                || k.ends_with("_token")
        }

        fn summarize_value(key: &str, value: &str) -> serde_json::Value {
            const MAX: usize = 256;
            let v = value.trim();
            if v.len() <= MAX {
                serde_json::Value::String(v.to_string())
            } else {
                serde_json::json!({
                    "key": key,
                    "truncated": true,
                    "len": v.len(),
                    "preview": format!("{}…", &v[..MAX])
                })
            }
        }

        // Check if verify setting exists
        // (logic omitted for brevity but conceptually similar)

        // Check if setting exists
        let existing = self.get_by_key(tenant_id.as_deref(), &dto.key).await?;

        if let Some(mut setting) = existing {
            // Update existing
            let prev_value = setting.value.clone();
            setting.value = dto.value.clone();
            setting.description = dto.description.clone();
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

                query
                    .bind(tid)
                    .bind(&setting.key)
                    .execute(&self.pool)
                    .await?;
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

            // Audit
            let sensitive = is_sensitive_setting_key(&setting.key);
            let details = if sensitive {
                serde_json::json!({
                    "message": "Updated setting",
                    "key": setting.key,
                    "sensitive": true,
                    "changed": prev_value != setting.value
                })
            } else {
                serde_json::json!({
                    "message": "Updated setting",
                    "key": setting.key,
                    "sensitive": false,
                    "from": summarize_value(&setting.key, &prev_value),
                    "to": summarize_value(&setting.key, &setting.value),
                })
            };
            self.audit_service
                .log(
                    actor_id,
                    tenant_id.as_deref(),
                    "update",
                    "settings",
                    Some(&setting.key),
                    Some(&details.to_string()),
                    ip_address,
                )
                .await;

            Ok(setting)
        } else {
            // Insert new
            let setting = Setting::new(
                tenant_id.clone(),
                dto.key.clone(),
                dto.value.clone(),
                dto.description.clone(),
            );

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
            let query = query.bind(setting.created_at).bind(setting.updated_at);

            #[cfg(not(feature = "postgres"))]
            let query = query
                .bind(setting.created_at.to_rfc3339())
                .bind(setting.updated_at.to_rfc3339());

            query.execute(&self.pool).await?;

            // Audit
            let sensitive = is_sensitive_setting_key(&setting.key);
            let details = if sensitive {
                serde_json::json!({
                    "message": "Created setting",
                    "key": setting.key,
                    "sensitive": true
                })
            } else {
                serde_json::json!({
                    "message": "Created setting",
                    "key": setting.key,
                    "sensitive": false,
                    "value": summarize_value(&setting.key, &setting.value)
                })
            };
            self.audit_service
                .log(
                    actor_id,
                    tenant_id.as_deref(),
                    "create",
                    "settings",
                    Some(&setting.key),
                    Some(&details.to_string()),
                    ip_address,
                )
                .await;

            Ok(setting)
        }
    }

    /// Delete setting by key and tenant
    pub async fn delete(
        &self,
        tenant_id: Option<&str>,
        key: &str,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
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

        // Audit
        let details = serde_json::json!({ "key": key });
        self.audit_service
            .log(
                actor_id,
                tenant_id,
                "delete",
                "settings",
                Some(key),
                Some(&details.to_string()),
                ip_address,
            )
            .await;

        Ok(())
    }
}
