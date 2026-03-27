//! Database connection and initialization module
//! Supports PostgreSQL (default/online) and SQLite (optional/offline)

// These features are mutually exclusive. Enabling both breaks compilation due to duplicated types/impls.
#[cfg(all(feature = "postgres", feature = "sqlite"))]
compile_error!("Features 'postgres' and 'sqlite' are mutually exclusive. Use default (postgres) OR --no-default-features --features sqlite.");

#[cfg(feature = "postgres")]
use sqlx::{Pool, Postgres};

#[cfg(feature = "sqlite")]
use sqlx::{Pool, Sqlite};

mod bootstrap;
mod migrations;
mod seed;

pub use bootstrap::init_db;
pub use seed::{seed_defaults, seed_plans, seed_roles};

#[cfg(feature = "postgres")]
pub type DbPool = Pool<Postgres>;

#[cfg(feature = "sqlite")]
pub type DbPool = Pool<Sqlite>;

#[cfg(all(test, feature = "postgres"))]
mod tests {
    //! Task 2.1 characterization tests focus on observable behavior only.
    //!
    //! - Fast tests validate deterministic URL/build/configuration behavior without a live database.
    //! - Live Postgres behavior tests are explicit opt-in (`#[ignore]` + env-gated) to avoid
    //!   false confidence during normal test runs.
    use super::bootstrap::{build_postgres_url_from_env, percent_encode_component};
    use super::{init_db, seed_defaults};
    use sqlx::PgPool;
    use std::env;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    const BEHAVIOR_DB_URL_ENV: &str = "CONNECTION_TEST_DATABASE_URL";

    fn behavior_db_url() -> Option<String> {
        env::var(BEHAVIOR_DB_URL_ENV)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().expect("env lock")
    }

    const ENV_KEYS: [&str; 7] = [
        "DATABASE_URL",
        "POSTGRES_USER",
        "POSTGRES_PASSWORD",
        "POSTGRES_DB",
        "POSTGRES_HOST",
        "POSTGRES_PORT",
        "POSTGRES_SSLMODE",
    ];

    struct EnvRestoreGuard {
        snapshot: Vec<(&'static str, Option<String>)>,
    }

    impl EnvRestoreGuard {
        fn capture(keys: &[&'static str]) -> Self {
            let snapshot = keys.iter().map(|&k| (k, env::var(k).ok())).collect();
            Self { snapshot }
        }
    }

    impl Drop for EnvRestoreGuard {
        fn drop(&mut self) {
            for (key, value) in &self.snapshot {
                match value {
                    Some(v) => env::set_var(key, v),
                    None => env::remove_var(key),
                }
            }
        }
    }

    #[test]
    fn percent_encode_component_encodes_reserved_characters() {
        assert_eq!(
            percent_encode_component("user name:p@ss/word"),
            "user%20name%3Ap%40ss%2Fword"
        );
    }

    #[test]
    fn build_postgres_url_from_env_uses_defaults_and_percent_encoding() {
        let _guard = env_lock();
        let _env_restore = EnvRestoreGuard::capture(&ENV_KEYS);

        env::remove_var("DATABASE_URL");
        env::set_var("POSTGRES_USER", "alice name");
        env::set_var("POSTGRES_PASSWORD", "p@ss/word");
        env::set_var("POSTGRES_DB", "main db");
        env::remove_var("POSTGRES_HOST");
        env::remove_var("POSTGRES_PORT");
        env::remove_var("POSTGRES_SSLMODE");

        let url = build_postgres_url_from_env().expect("postgres url");

        assert_eq!(
            url,
            "postgres://alice%20name:p%40ss%2Fword@localhost:5432/main%20db"
        );

    }

    #[test]
    fn build_postgres_url_from_env_adds_trimmed_sslmode_when_present() {
        let _guard = env_lock();
        let _env_restore = EnvRestoreGuard::capture(&ENV_KEYS);

        env::remove_var("DATABASE_URL");
        env::set_var("POSTGRES_USER", "user");
        env::set_var("POSTGRES_PASSWORD", "pass");
        env::set_var("POSTGRES_DB", "app");
        env::set_var("POSTGRES_HOST", "db.internal");
        env::set_var("POSTGRES_PORT", "5433");
        env::set_var("POSTGRES_SSLMODE", " require mode ");

        let url = build_postgres_url_from_env().expect("postgres url with sslmode");

        assert_eq!(
            url,
            "postgres://user:pass@db.internal:5433/app?sslmode=require%20mode"
        );

    }

    #[tokio::test]
    async fn init_db_returns_configuration_error_when_database_url_and_postgres_env_absent() {
        let _guard = env_lock();
        let _env_restore = EnvRestoreGuard::capture(&ENV_KEYS);

        env::remove_var("DATABASE_URL");
        env::remove_var("POSTGRES_USER");
        env::remove_var("POSTGRES_PASSWORD");
        env::remove_var("POSTGRES_DB");
        env::remove_var("POSTGRES_HOST");
        env::remove_var("POSTGRES_PORT");
        env::remove_var("POSTGRES_SSLMODE");

        let err = init_db(PathBuf::from(".")).await.expect_err("expected config error");

        match err {
            sqlx::Error::Configuration(msg) => {
                let text = msg.to_string();
                assert!(text.contains("Missing POSTGRES_USER"), "unexpected error: {text}");
            }
            other => panic!("expected configuration error, got: {other}"),
        }

    }

    #[tokio::test]
    async fn init_db_with_database_url_does_not_require_postgres_component_env() {
        let _guard = env_lock();
        let _env_restore = EnvRestoreGuard::capture(&ENV_KEYS);

        env::set_var("DATABASE_URL", "postgres://postgres:postgres@127.0.0.1:1/test");
        env::remove_var("POSTGRES_USER");
        env::remove_var("POSTGRES_PASSWORD");
        env::remove_var("POSTGRES_DB");
        env::remove_var("POSTGRES_HOST");
        env::remove_var("POSTGRES_PORT");
        env::remove_var("POSTGRES_SSLMODE");

        let err = init_db(PathBuf::from(".")).await.expect_err("expected connection failure");

        if let sqlx::Error::Configuration(msg) = err {
            let text = msg.to_string();
            assert!(
                !text.contains("Missing POSTGRES_"),
                "DATABASE_URL path should not fail on missing POSTGRES_* env vars: {text}"
            );
        }

    }

    #[tokio::test]
    #[ignore = "requires explicit live Postgres opt-in via CONNECTION_TEST_DATABASE_URL"]
    async fn seed_defaults_idempotence_observable_with_opt_in_live_postgres() -> Result<(), sqlx::Error>
    {
        let Some(database_url) = behavior_db_url() else {
            eprintln!(
                "skipping behavior test: set {} to run against a live Postgres database",
                BEHAVIOR_DB_URL_ENV
            );
            return Ok(());
        };

        let pool = PgPool::connect(&database_url).await?;

        let before_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM settings WHERE tenant_id IS NULL AND key = 'app_name'",
        )
        .fetch_one(&pool)
        .await?;

        seed_defaults(&pool).await?;
        seed_defaults(&pool).await?;

        let after_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM settings WHERE tenant_id IS NULL AND key = 'app_name'",
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(
            after_count,
            std::cmp::max(before_count, 1),
            "seed_defaults should remain idempotent for global app_name setting"
        );

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires explicit live Postgres opt-in via CONNECTION_TEST_DATABASE_URL"]
    async fn init_db_success_path_observable_with_opt_in_live_postgres() -> Result<(), sqlx::Error> {
        let Some(database_url) = behavior_db_url() else {
            eprintln!(
                "skipping behavior test: set {} to run against a live Postgres database",
                BEHAVIOR_DB_URL_ENV
            );
            return Ok(());
        };

        let _guard = env_lock();
        let _env_restore = EnvRestoreGuard::capture(&ENV_KEYS);

        env::set_var("DATABASE_URL", &database_url);
        env::remove_var("POSTGRES_USER");
        env::remove_var("POSTGRES_PASSWORD");
        env::remove_var("POSTGRES_DB");
        env::remove_var("POSTGRES_HOST");
        env::remove_var("POSTGRES_PORT");
        env::remove_var("POSTGRES_SSLMODE");

        let pool = init_db(PathBuf::from(".")).await?;
        let app_name_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM settings WHERE tenant_id IS NULL AND key = 'app_name')",
        )
        .fetch_one(&pool)
        .await?;

        assert!(
            app_name_exists,
            "init_db success path should leave seeded global app_name setting"
        );

        Ok(())
    }
}
