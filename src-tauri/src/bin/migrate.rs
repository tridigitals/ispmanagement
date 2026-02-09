//! Minimal DB migration runner for Postgres.
//!
//! This binary is intentionally small so it can be used in dev/CI without
//! pulling in the full server/Tauri runtime.

use std::env;

#[cfg(feature = "postgres")]
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    #[cfg(not(feature = "postgres"))]
    {
        eprintln!("This migration runner is intended for Postgres builds.");
        eprintln!("Run without `--no-default-features --features sqlite`.");
        std::process::exit(2);
    }

    #[cfg(feature = "postgres")]
    {
        fn build_database_url_from_env() -> Result<String, Box<dyn std::error::Error>> {
            let user = env::var("POSTGRES_USER")
                .map_err(|_| "Missing POSTGRES_USER (or set DATABASE_URL)")?;
            let password = env::var("POSTGRES_PASSWORD")
                .map_err(|_| "Missing POSTGRES_PASSWORD (or set DATABASE_URL)")?;
            let db =
                env::var("POSTGRES_DB").map_err(|_| "Missing POSTGRES_DB (or set DATABASE_URL)")?;
            let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
            let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
            let sslmode = env::var("POSTGRES_SSLMODE").ok();

            // Keep it simple here: if you have special characters, set DATABASE_URL explicitly.
            let mut url = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, db);
            if let Some(m) = sslmode {
                if !m.trim().is_empty() {
                    url.push_str("?sslmode=");
                    url.push_str(m.trim());
                }
            }

            Ok(url)
        }

        let database_url = match env::var("DATABASE_URL") {
            Ok(v) => v,
            Err(_) => build_database_url_from_env()?,
        };

        let pool = PgPool::connect(&database_url).await?;

        static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
        MIGRATOR.run(&pool).await?;

        println!("Migrations applied successfully.");
    }

    Ok(())
}
