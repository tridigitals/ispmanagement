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
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL must be set (e.g. postgres://user:pass@host:5432/db)")?;

        let pool = PgPool::connect(&database_url).await?;

        static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
        MIGRATOR.run(&pool).await?;

        println!("Migrations applied successfully.");
    }

    Ok(())
}
