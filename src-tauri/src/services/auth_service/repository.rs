use crate::db::connection::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::User;

use super::dto::AuthSettings;
use chrono::{DateTime, Utc};

pub async fn fetch_auth_settings_from_db(pool: &DbPool) -> AuthSettings {
    let mut settings = AuthSettings::default();

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM settings WHERE tenant_id IS NULL AND key LIKE 'auth_%' OR key IN ('max_login_attempts', 'lockout_duration_minutes', 'app_main_domain')"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let settings_map: std::collections::HashMap<String, String> = rows.into_iter().collect();

    if let Some(val) = settings_map.get("auth_jwt_expiry_hours") {
        settings.jwt_expiry_hours = val.parse().unwrap_or(24);
    }
    if let Some(val) = settings_map.get("auth_session_timeout_minutes") {
        settings.session_timeout_minutes = val.parse().unwrap_or(60);
    }
    if let Some(val) = settings_map.get("auth_password_min_length") {
        settings.password_min_length = val.parse().unwrap_or(8);
    }
    if let Some(val) = settings_map.get("auth_password_require_uppercase") {
        settings.password_require_uppercase = val == "true";
    }
    if let Some(val) = settings_map.get("auth_password_require_number") {
        settings.password_require_number = val == "true";
    }
    if let Some(val) = settings_map.get("auth_password_require_special") {
        settings.password_require_special = val == "true";
    }

    if let Some(val) = settings_map.get("auth_max_login_attempts") {
        settings.max_login_attempts = val.parse().unwrap_or(5);
    } else if let Some(val) = settings_map.get("max_login_attempts") {
        settings.max_login_attempts = val.parse().unwrap_or(5);
    }

    if let Some(val) = settings_map.get("auth_lockout_duration_minutes") {
        settings.lockout_duration_minutes = val.parse().unwrap_or(15);
    } else if let Some(val) = settings_map.get("lockout_duration_minutes") {
        settings.lockout_duration_minutes = val.parse().unwrap_or(15);
    }

    if let Some(val) = settings_map.get("auth_allow_registration") {
        settings.allow_registration = val == "true";
    }
    if let Some(val) = settings_map.get("auth_logout_all_on_password_change") {
        settings.logout_all_on_password_change = val == "true";
    }
    if let Some(val) = settings_map.get("auth_require_email_verification") {
        settings.require_email_verification = val == "true";
    }

    if let Some(val) = settings_map.get("app_main_domain") {
        if !val.is_empty() {
            settings.main_domain = Some(val.clone());
        }
    }

    settings
}

pub async fn insert_session(
    pool: &DbPool,
    session_id: &str,
    user_id: &str,
    tenant_id: Option<&str>,
    token: &str,
    session_expires_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let query = sqlx::query(
        "INSERT INTO sessions (id, user_id, tenant_id, token, expires_at, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(tenant_id)
    .bind(token);

    #[cfg(feature = "postgres")]
    let query = query.bind(session_expires_at).bind(now);

    #[cfg(not(feature = "postgres"))]
    let query = query
        .bind(session_expires_at.to_rfc3339())
        .bind(now.to_rfc3339());

    query.execute(pool).await?;
    Ok(())
}

pub async fn get_user_by_id(pool: &DbPool, user_id: &str) -> AppResult<User> {
    sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::UserNotFound)
}
