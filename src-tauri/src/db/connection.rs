//! Database connection and initialization module
//! Supports PostgreSQL (default/online) and SQLite (optional/offline)

#[cfg(feature = "postgres")]
use sqlx::{Pool, Postgres, PgPool};

#[cfg(feature = "sqlite")]
use sqlx::{Pool, Sqlite, SqlitePool};

use std::path::PathBuf;
use std::env;
use tracing::info;

#[cfg(feature = "postgres")]
pub type DbPool = Pool<Postgres>;

#[cfg(feature = "sqlite")]
pub type DbPool = Pool<Sqlite>;

/// Initialize database connection
pub async fn init_db(app_data_dir: PathBuf) -> Result<DbPool, sqlx::Error> {
    #[cfg(feature = "postgres")]
    {
        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set for PostgreSQL mode");
        
        info!("Connecting to PostgreSQL database");
        
        let pool = PgPool::connect(&database_url).await?;
        run_migrations_pg(&pool).await?;
        
        info!("PostgreSQL database initialized successfully");
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
        Ok(pool)
    }
}

#[cfg(feature = "postgres")]
async fn run_migrations_pg(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    // Create users table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY NOT NULL,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            name TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            avatar_url TEXT,
            is_active BOOLEAN NOT NULL DEFAULT true,
            email_verified_at TIMESTAMPTZ,
            failed_login_attempts INTEGER NOT NULL DEFAULT 0,
            locked_until TIMESTAMPTZ,
            verification_token TEXT,
            reset_token TEXT,
            reset_token_expires TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        )
    "#)
    .execute(pool)
    .await?;

    // Create settings table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS settings (
            id TEXT PRIMARY KEY NOT NULL,
            key TEXT NOT NULL UNIQUE,
            value TEXT NOT NULL,
            description TEXT,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        )
    "#)
    .execute(pool)
    .await?;

    // Create sessions table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            token TEXT NOT NULL UNIQUE,
            expires_at TIMESTAMPTZ NOT NULL,
            created_at TIMESTAMPTZ NOT NULL
        )
    "#)
    .execute(pool)
    .await?;

    // Create indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)").execute(pool).await.ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_settings_key ON settings(key)").execute(pool).await.ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token)").execute(pool).await.ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id)").execute(pool).await.ok();

    info!("PostgreSQL migrations completed");
    Ok(())
}

#[cfg(feature = "sqlite")]
async fn run_migrations_sqlite(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    // Create users table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY NOT NULL,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            name TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            avatar_url TEXT,
            is_active INTEGER NOT NULL DEFAULT 1,
            email_verified_at TEXT,
            failed_login_attempts INTEGER NOT NULL DEFAULT 0,
            locked_until TEXT,
            verification_token TEXT,
            reset_token TEXT,
            reset_token_expires TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
    "#)
    .execute(pool)
    .await?;

    // Create settings table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS settings (
            id TEXT PRIMARY KEY NOT NULL,
            key TEXT NOT NULL UNIQUE,
            value TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
    "#)
    .execute(pool)
    .await?;

    // Create sessions table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            token TEXT NOT NULL UNIQUE,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
    "#)
    .execute(pool)
    .await?;

    // Create indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)").execute(pool).await.ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_settings_key ON settings(key)").execute(pool).await.ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token)").execute(pool).await.ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id)").execute(pool).await.ok();

    info!("SQLite migrations completed");
    Ok(())
}

/// Seed default settings
pub async fn seed_defaults(pool: &DbPool) -> Result<(), sqlx::Error> {
    let jwt_secret = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    let defaults = vec![
        ("app_name", "SaaS App", "Application name"),
        ("app_description", "Enterprise-grade boilerplate built with Rust and SvelteKit. Secure, scalable, and lightweight.", "Application description"),
        ("app_version", "1.0.0", "Application version"),
        ("jwt_secret", jwt_secret.as_str(), "JWT signing secret"),
        ("auth_jwt_expiry_hours", "24", "JWT token expiry in hours"),
        ("auth_session_timeout_minutes", "60", "Session timeout after inactivity (minutes)"),
        ("auth_password_min_length", "8", "Minimum password length"),
        ("auth_password_require_uppercase", "true", "Require uppercase letter in password"),
        ("auth_password_require_number", "true", "Require number in password"),
        ("auth_password_require_special", "false", "Require special character in password"),
        ("auth_max_login_attempts", "5", "Maximum failed login attempts before lockout"),
        ("auth_lockout_duration_minutes", "15", "Account lockout duration in minutes"),
        ("auth_allow_registration", "true", "Allow public user registration"),
        ("auth_require_email_verification", "false", "Require email verification after registration"),
    ];

    for (key, value, description) in defaults {
        #[cfg(feature = "postgres")]
        {
            sqlx::query(r#"
                INSERT INTO settings (id, key, value, description, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (key) DO NOTHING
            "#)
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(key)
            .bind(value)
            .bind(description)
            .bind(now)  // DateTime directly for PostgreSQL
            .bind(now)
            .execute(pool)
            .await?;
        }
        
        #[cfg(feature = "sqlite")]
        {
            let now_str = now.to_rfc3339();
            sqlx::query(r#"
                INSERT OR IGNORE INTO settings (id, key, value, description, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?)
            "#)
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(key)
            .bind(value)
            .bind(description)
            .bind(&now_str)  // RFC3339 string for SQLite
            .bind(&now_str)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}
