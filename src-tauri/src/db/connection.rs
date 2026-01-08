//! Database connection and initialization module

use sqlx::{Pool, Sqlite, SqlitePool};
use std::path::PathBuf;
use tracing::info;

/// Initialize database connection and run migrations
pub async fn init_db(app_data_dir: PathBuf) -> Result<Pool<Sqlite>, sqlx::Error> {
    let db_path = app_data_dir.join("saas_app.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    info!("Connecting to database at: {}", db_path.display());

    let pool = SqlitePool::connect(&db_url).await?;

    // Run migrations
    run_migrations(&pool).await?;

    info!("Database initialized successfully");
    Ok(pool)
}

/// Run database migrations
async fn run_migrations(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    // Create users table
    sqlx::query(
        r#"
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
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Migration: Add failed_login_attempts if missing (for existing databases)
    let has_failed_login_attempts: bool = sqlx::query_scalar::<_, i32>(
        "SELECT count(*) FROM pragma_table_info('users') WHERE name='failed_login_attempts'"
    )
    .fetch_one(pool)
    .await? > 0;

    if !has_failed_login_attempts {
        info!("Migrating: Adding failed_login_attempts column to users table");
        sqlx::query("ALTER TABLE users ADD COLUMN failed_login_attempts INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await?;
    }

    // Migration: Add locked_until if missing (for existing databases)
    let has_locked_until: bool = sqlx::query_scalar::<_, i32>(
        "SELECT count(*) FROM pragma_table_info('users') WHERE name='locked_until'"
    )
    .fetch_one(pool)
    .await? > 0;

    if !has_locked_until {
        info!("Migrating: Adding locked_until column to users table");
        sqlx::query("ALTER TABLE users ADD COLUMN locked_until TEXT")
            .execute(pool)
            .await?;
    }

    // Migration: Add verification_token if missing
    let has_verification_token: bool = sqlx::query_scalar::<_, i32>(
        "SELECT count(*) FROM pragma_table_info('users') WHERE name='verification_token'"
    )
    .fetch_one(pool)
    .await? > 0;

    if !has_verification_token {
        info!("Migrating: Adding verification_token column to users table");
        sqlx::query("ALTER TABLE users ADD COLUMN verification_token TEXT")
            .execute(pool)
            .await?;
    }

    // Migration: Add reset_token if missing
    let has_reset_token: bool = sqlx::query_scalar::<_, i32>(
        "SELECT count(*) FROM pragma_table_info('users') WHERE name='reset_token'"
    )
    .fetch_one(pool)
    .await? > 0;

    if !has_reset_token {
        info!("Migrating: Adding reset_token column to users table");
        sqlx::query("ALTER TABLE users ADD COLUMN reset_token TEXT")
            .execute(pool)
            .await?;
    }

    // Migration: Add reset_token_expires if missing
    let has_reset_token_expires: bool = sqlx::query_scalar::<_, i32>(
        "SELECT count(*) FROM pragma_table_info('users') WHERE name='reset_token_expires'"
    )
    .fetch_one(pool)
    .await? > 0;

    if !has_reset_token_expires {
        info!("Migrating: Adding reset_token_expires column to users table");
        sqlx::query("ALTER TABLE users ADD COLUMN reset_token_expires TEXT")
            .execute(pool)
            .await?;
    }

    // Create settings table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            id TEXT PRIMARY KEY NOT NULL,
            key TEXT NOT NULL UNIQUE,
            value TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create sessions table for token management
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            token TEXT NOT NULL UNIQUE,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_settings_key ON settings(key)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id)")
        .execute(pool)
        .await?;

    info!("Database migrations completed");
    Ok(())
}

/// Seed default settings
pub async fn seed_defaults(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    // Generate JWT secret ahead of time to avoid temporary value issue
    let jwt_secret = uuid::Uuid::new_v4().to_string();
    
    let defaults = vec![
        // App settings
        ("app_name", "SaaS App", "Application name"),
        ("app_version", "1.0.0", "Application version"),
        
        // JWT settings
        ("jwt_secret", jwt_secret.as_str(), "JWT signing secret"),
        ("auth_jwt_expiry_hours", "24", "JWT token expiry in hours"),
        ("auth_session_timeout_minutes", "60", "Session timeout after inactivity (minutes)"),
        
        // Password policy settings
        ("auth_password_min_length", "8", "Minimum password length"),
        ("auth_password_require_uppercase", "true", "Require uppercase letter in password"),
        ("auth_password_require_number", "true", "Require number in password"),
        ("auth_password_require_special", "false", "Require special character in password"),
        
        // Login security settings
        ("auth_max_login_attempts", "5", "Maximum failed login attempts before lockout"),
        ("auth_lockout_duration_minutes", "15", "Account lockout duration in minutes"),
        
        // Registration settings
        ("auth_allow_registration", "true", "Allow public user registration"),
        ("auth_require_email_verification", "false", "Require email verification after registration"),
    ];

    for (key, value, description) in defaults {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO settings (id, key, value, description, created_at, updated_at)
            VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))
            "#,
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(key)
        .bind(value)
        .bind(description)
        .execute(pool)
        .await?;
    }

    Ok(())
}
