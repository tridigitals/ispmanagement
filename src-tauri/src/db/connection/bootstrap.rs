#[cfg(feature = "postgres")]
use super::migrations::run_migrations_pg;
#[cfg(feature = "sqlite")]
use super::migrations::run_migrations_sqlite;
use super::seed::{seed_defaults, seed_plans};
use super::DbPool;

#[cfg(feature = "postgres")]
use sqlx::PgPool;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

use std::env;
use std::path::PathBuf;
use tracing::info;

#[cfg(feature = "postgres")]
pub(super) fn percent_encode_component(raw: &str) -> String {
    // Minimal percent-encoding for URL components (user/password/db).
    // If you need more exotic behavior, set DATABASE_URL explicitly.
    let mut out = String::with_capacity(raw.len());
    for b in raw.bytes() {
        let c = b as char;
        let is_unreserved = matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~');
        if is_unreserved {
            out.push(c);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(feature = "postgres")]
pub(super) fn build_postgres_url_from_env() -> Result<String, sqlx::Error> {
    let user = env::var("POSTGRES_USER").map_err(|_| {
        sqlx::Error::Configuration(
            "Missing POSTGRES_USER. Set DATABASE_URL or POSTGRES_* env vars.".into(),
        )
    })?;
    let password = env::var("POSTGRES_PASSWORD").map_err(|_| {
        sqlx::Error::Configuration(
            "Missing POSTGRES_PASSWORD. Set DATABASE_URL or POSTGRES_* env vars.".into(),
        )
    })?;
    let db = env::var("POSTGRES_DB").map_err(|_| {
        sqlx::Error::Configuration(
            "Missing POSTGRES_DB. Set DATABASE_URL or POSTGRES_* env vars.".into(),
        )
    })?;

    let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
    let sslmode = env::var("POSTGRES_SSLMODE").ok();

    let user = percent_encode_component(&user);
    let password = percent_encode_component(&password);
    let db = percent_encode_component(&db);

    let mut url = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, db);
    if let Some(m) = sslmode {
        if !m.trim().is_empty() {
            url.push_str("?sslmode=");
            url.push_str(&percent_encode_component(m.trim()));
        }
    }

    Ok(url)
}

/// Initialize database connection
pub async fn init_db(app_data_dir: PathBuf) -> Result<DbPool, sqlx::Error> {
    #[cfg(feature = "postgres")]
    {
        // app_data_dir is used for SQLite mode; keep signature consistent.
        let _ = &app_data_dir;

        let database_url = match env::var("DATABASE_URL") {
            Ok(v) => v,
            Err(_) => build_postgres_url_from_env()?,
        };

        info!("Connecting to PostgreSQL database");

        let pool = PgPool::connect(&database_url).await?;
        run_migrations_pg(&pool).await?;

        info!("PostgreSQL database initialized successfully");

        seed_defaults(&pool).await?;
        seed_plans(&pool).await?;

        Ok(pool)
    }

    #[cfg(feature = "sqlite")]
    {
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            let db_path = app_data_dir.join("saas_app.db");
            format!("sqlite:{}?mode=rwc", db_path.display())
        });

        info!("Connecting to SQLite database: {}", database_url);

        let pool = SqlitePool::connect(&database_url).await?;
        run_migrations_sqlite(&pool).await?;

        info!("SQLite database initialized successfully");

        seed_defaults(&pool).await?;
        seed_plans(&pool).await?;

        Ok(pool)
    }
}
