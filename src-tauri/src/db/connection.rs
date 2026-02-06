//! Database connection and initialization module
//! Supports PostgreSQL (default/online) and SQLite (optional/offline)

// These features are mutually exclusive. Enabling both breaks compilation due to duplicated types/impls.
#[cfg(all(feature = "postgres", feature = "sqlite"))]
compile_error!("Features 'postgres' and 'sqlite' are mutually exclusive. Use default (postgres) OR --no-default-features --features sqlite.");

#[cfg(feature = "postgres")]
use sqlx::{PgPool, Pool, Postgres};

#[cfg(feature = "sqlite")]
use sqlx::{Pool, Sqlite, SqlitePool};

use std::env;
use std::path::PathBuf;
use tracing::info;

#[cfg(feature = "postgres")]
pub type DbPool = Pool<Postgres>;

#[cfg(feature = "sqlite")]
pub type DbPool = Pool<Sqlite>;

/// Initialize database connection
pub async fn init_db(app_data_dir: PathBuf) -> Result<DbPool, sqlx::Error> {
    #[cfg(feature = "postgres")]
    {
        // app_data_dir is used for SQLite mode; keep signature consistent.
        let _ = &app_data_dir;

        let database_url = env::var("DATABASE_URL").map_err(|_| {
            sqlx::Error::Configuration(
                "DATABASE_URL must be set for PostgreSQL mode. Please check your .env file.".into(),
            )
        })?;

        info!("Connecting to PostgreSQL database");

        let pool = PgPool::connect(&database_url).await?;
        run_migrations_pg(&pool).await?;

        info!("PostgreSQL database initialized successfully");

        seed_defaults(&pool).await?;
        seed_roles(&pool).await?;
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
        seed_roles(&pool).await?;
        seed_plans(&pool).await?;

        Ok(pool)
    }
}

#[cfg(feature = "postgres")]
async fn run_migrations_pg(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    // Postgres schema is managed exclusively via SQLx migrations.
    static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

    MIGRATOR
        .run(pool)
        .await
        .map_err(|e| sqlx::Error::Migrate(Box::new(e)))?;

    info!("PostgreSQL migrations completed");
    Ok(())
}
#[cfg(feature = "sqlite")]
async fn run_migrations_sqlite(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    // Create tenants table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tenants (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            slug TEXT UNIQUE NOT NULL,
            custom_domain TEXT UNIQUE,
            logo_url TEXT,
            is_active INTEGER NOT NULL DEFAULT 1,
            storage_usage INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Add storage_usage column if it doesn't exist (SQLite)
    let _ = sqlx::query("ALTER TABLE tenants ADD COLUMN storage_usage INTEGER NOT NULL DEFAULT 0")
        .execute(pool)
        .await;

    // Add enforce_2fa column if it doesn't exist (SQLite)
    let _ = sqlx::query("ALTER TABLE tenants ADD COLUMN enforce_2fa INTEGER NOT NULL DEFAULT 0")
        .execute(pool)
        .await;

    // Add storage_usage column if it doesn't exist (SQLite)
    let _ = sqlx::query("ALTER TABLE tenants ADD COLUMN storage_usage INTEGER NOT NULL DEFAULT 0")
        .execute(pool)
        .await;

    // Create users table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY NOT NULL,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            name TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            is_super_admin INTEGER NOT NULL DEFAULT 0,
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
    "#,
    )
    .execute(pool)
    .await?;

    // Create tenant_members table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tenant_members (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'member',
            created_at TEXT NOT NULL,
            UNIQUE(tenant_id, user_id),
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create settings table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create sessions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            tenant_id TEXT,
            token TEXT NOT NULL UNIQUE,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create permissions table (RBAC)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS permissions (
            id TEXT PRIMARY KEY NOT NULL,
            resource TEXT NOT NULL,
            action TEXT NOT NULL,
            description TEXT,
            UNIQUE(resource, action)
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create roles table (RBAC)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS roles (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT,
            name TEXT NOT NULL,
            description TEXT,
            is_system INTEGER NOT NULL DEFAULT 0,
            level INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create role_permissions pivot table (RBAC)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS role_permissions (
            role_id TEXT NOT NULL,
            permission_id TEXT NOT NULL,
            PRIMARY KEY (role_id, permission_id),
            FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
            FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Migration: Add level column to roles if not exists for SQLite
    // SQLite doesn't support IF NOT EXISTS in ADD COLUMN directly in all versions or easy check,
    // but newer versions do. Or we can just try/catch.
    let _ = sqlx::query("ALTER TABLE roles ADD COLUMN level INTEGER NOT NULL DEFAULT 0")
        .execute(pool)
        .await;

    // Create indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
        .execute(pool)
        .await
        .ok();
    // Unique partial indexes for SQLite
    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_settings_global_key ON settings(key) WHERE tenant_id IS NULL").execute(pool).await.ok();
    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_settings_tenant_key ON settings(tenant_id, key) WHERE tenant_id IS NOT NULL").execute(pool).await.ok();

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token)")
        .execute(pool)
        .await
        .ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug)")
        .execute(pool)
        .await
        .ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_roles_tenant ON roles(tenant_id)")
        .execute(pool)
        .await
        .ok();

    // ==================== SUBSCRIPTION PLANS (SQLite) ====================

    // Create plans table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS plans (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            slug TEXT UNIQUE NOT NULL,
            description TEXT,
            price_monthly REAL DEFAULT 0,
            price_yearly REAL DEFAULT 0,
            is_active INTEGER DEFAULT 1,
            is_default INTEGER DEFAULT 0,
            sort_order INTEGER DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create feature_definitions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS feature_definitions (
            id TEXT PRIMARY KEY NOT NULL,
            code TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            value_type TEXT NOT NULL DEFAULT 'boolean',
            category TEXT DEFAULT 'general',
            default_value TEXT DEFAULT 'false',
            sort_order INTEGER DEFAULT 0,
            created_at TEXT NOT NULL
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create plan_features table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS plan_features (
            id TEXT PRIMARY KEY NOT NULL,
            plan_id TEXT NOT NULL,
            feature_id TEXT NOT NULL,
            value TEXT NOT NULL,
            UNIQUE(plan_id, feature_id),
            FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE,
            FOREIGN KEY (feature_id) REFERENCES feature_definitions(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create tenant_subscriptions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tenant_subscriptions (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT NOT NULL,
            plan_id TEXT NOT NULL,
            status TEXT DEFAULT 'active',
            trial_ends_at TEXT,
            current_period_start TEXT,
            current_period_end TEXT,
            feature_overrides TEXT DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(tenant_id),
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
            FOREIGN KEY (plan_id) REFERENCES plans(id)
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create file_records table (SQLite)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS file_records (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT NOT NULL,
            name TEXT NOT NULL,
            original_name TEXT NOT NULL,
            path TEXT NOT NULL,
            size INTEGER NOT NULL,
            content_type TEXT NOT NULL,
            storage_provider TEXT NOT NULL DEFAULT 'local',
            uploaded_by TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
            FOREIGN KEY (uploaded_by) REFERENCES users(id) ON DELETE SET NULL
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create invoices table (SQLite)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS invoices (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT NOT NULL,
            invoice_number TEXT UNIQUE NOT NULL,
            amount REAL NOT NULL,
            currency_code TEXT NOT NULL DEFAULT 'IDR',
            base_currency_code TEXT NOT NULL DEFAULT 'IDR',
            fx_rate REAL,
            fx_source TEXT,
            fx_fetched_at TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            description TEXT,
            due_date TEXT NOT NULL,
            paid_at TEXT,
            payment_method TEXT,
            external_id TEXT,
            merchant_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
            FOREIGN KEY (merchant_id) REFERENCES tenants(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create bank_accounts table (SQLite)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS bank_accounts (
            id TEXT PRIMARY KEY NOT NULL,
            bank_name TEXT NOT NULL,
            account_number TEXT NOT NULL,
            account_holder TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Migration: Add storage_provider to file_records if not exists (SQLite)
    let _ = sqlx::query(
        "ALTER TABLE file_records ADD COLUMN storage_provider TEXT NOT NULL DEFAULT 'local'",
    )
    .execute(pool)
    .await?;

    // Migration: Add merchant_id and proof_attachment to invoices (SQLite)
    let _ = sqlx::query(
        "ALTER TABLE invoices ADD COLUMN merchant_id TEXT REFERENCES tenants(id) ON DELETE CASCADE",
    )
    .execute(pool)
    .await;

    let _ =
        sqlx::query("ALTER TABLE invoices ADD COLUMN currency_code TEXT NOT NULL DEFAULT 'IDR'")
            .execute(pool)
            .await;

    let _ = sqlx::query(
        "ALTER TABLE invoices ADD COLUMN base_currency_code TEXT NOT NULL DEFAULT 'IDR'",
    )
    .execute(pool)
    .await;
    let _ = sqlx::query("ALTER TABLE invoices ADD COLUMN fx_rate REAL")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE invoices ADD COLUMN fx_source TEXT")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE invoices ADD COLUMN fx_fetched_at TEXT")
        .execute(pool)
        .await;

    let _ = sqlx::query("ALTER TABLE invoices ADD COLUMN proof_attachment TEXT")
        .execute(pool)
        .await;

    // FX cache table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS fx_rates (
            base_currency TEXT NOT NULL,
            quote_currency TEXT NOT NULL,
            rate REAL NOT NULL,
            fetched_at TEXT NOT NULL,
            source TEXT NOT NULL,
            PRIMARY KEY (base_currency, quote_currency)
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create indexes for plans (SQLite)
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_plans_slug ON plans(slug)")
        .execute(pool)
        .await
        .ok();
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_feature_definitions_code ON feature_definitions(code)",
    )
    .execute(pool)
    .await
    .ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_plan_features_plan ON plan_features(plan_id)")
        .execute(pool)
        .await
        .ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenant_subscriptions_tenant ON tenant_subscriptions(tenant_id)").execute(pool).await.ok();

    info!("SQLite migrations completed");

    // Create notifications table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS notifications (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            tenant_id TEXT,
            title TEXT NOT NULL,
            message TEXT NOT NULL,
            notification_type TEXT DEFAULT 'info',
            category TEXT DEFAULT 'system',
            action_url TEXT,
            is_read INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Migration: Add columns to notifications if not exists (SQLite)
    // We ignore errors if columns already exist
    let _ =
        sqlx::query("ALTER TABLE notifications ADD COLUMN notification_type TEXT DEFAULT 'info'")
            .execute(pool)
            .await;
    let _ = sqlx::query("ALTER TABLE notifications ADD COLUMN category TEXT DEFAULT 'system'")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE notifications ADD COLUMN action_url TEXT")
        .execute(pool)
        .await;

    // Create notification_preferences table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS notification_preferences (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            channel TEXT NOT NULL,
            category TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            updated_at TEXT NOT NULL,
            UNIQUE(user_id, channel, category),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Create push_subscriptions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS push_subscriptions (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            endpoint TEXT UNIQUE NOT NULL,
            p256dh TEXT NOT NULL,
            auth TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Migration: Add 2FA columns to users if not exists (SQLite)
    let _ =
        sqlx::query("ALTER TABLE users ADD COLUMN two_factor_enabled INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN two_factor_secret TEXT")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN two_factor_recovery_codes TEXT")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN email_otp_code TEXT")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN email_otp_expires TEXT")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN preferred_2fa_method TEXT DEFAULT 'totp'")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN totp_enabled INTEGER NOT NULL DEFAULT 0")
        .execute(pool)
        .await;
    let _ =
        sqlx::query("ALTER TABLE users ADD COLUMN email_2fa_enabled INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await;

    // Data migration: Set totp_enabled=1 for existing users who have TOTP secret (SQLite uses INTEGER)
    let _ = sqlx::query("UPDATE users SET totp_enabled = 1 WHERE two_factor_secret IS NOT NULL AND totp_enabled = 0")
        .execute(pool)
        .await;

    // Data migration: Set email_2fa_enabled=1 for existing users with email 2FA preference
    let _ = sqlx::query("UPDATE users SET email_2fa_enabled = 1 WHERE two_factor_enabled = 1 AND preferred_2fa_method = 'email' AND email_2fa_enabled = 0")
        .execute(pool)
        .await;

    // Create trusted_devices table for 2FA device trust (SQLite)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS trusted_devices (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            device_fingerprint TEXT NOT NULL,
            ip_address TEXT,
            user_agent TEXT,
            trusted_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            last_used_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_trusted_devices_user ON trusted_devices(user_id)")
        .execute(pool)
        .await
        .ok();
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_trusted_devices_expires ON trusted_devices(expires_at)",
    )
    .execute(pool)
    .await
    .ok();

    seed_defaults(pool).await?;
    seed_roles(pool).await?;

    Ok(())
}

/// Seed default settings
pub async fn seed_defaults(pool: &DbPool) -> Result<(), sqlx::Error> {
    let jwt_secret = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    let app_name = env::var("APP_NAME").unwrap_or_else(|_| "SaaS Boilerplate".to_string());

    let defaults = vec![
        ("app_name", app_name.as_str(), "Application name"),
        ("app_description", "Enterprise-grade boilerplate built with Rust and SvelteKit. Secure, scalable, and lightweight.", "Application description"),
        ("app_public_url", "https://apisaas.tridigitals.com", "Public URL of the application"),
        ("app_version", "1.0.0", "Application version"),
        // Currency
        // base_currency_code is the pricing/base currency stored in the database (plans, limits, etc).
        // currency_code is the default display currency (tenants may override).
        ("base_currency_code", "IDR", "Base currency for pricing (keep stable)"),
        ("currency_code", "IDR", "Default display currency code (ISO 4217)"),
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
        ("maintenance_mode", "false", "System maintenance mode"),
        ("maintenance_message", "The system is currently under maintenance. Please try again later.", "Maintenance message displayed to users"),
        ("storage_max_file_size_mb", "500", "Maximum file upload size in Megabytes"),
        ("storage_allowed_extensions", "jpg,jpeg,png,gif,pdf,doc,docx,xls,xlsx,zip,rar,7z,mp4,mov,avi,mp3,wav", "Comma-separated list of allowed file extensions"),
        // Storage Driver Settings
        ("storage_driver", "local", "Storage driver: local, s3, or r2"),
        ("storage_s3_bucket", "", "S3 Bucket Name"),
        ("storage_s3_region", "auto", "S3 Region (e.g. us-east-1, auto for R2)"),
        ("storage_s3_endpoint", "", "S3 Endpoint URL (Required for R2/MinIO)"),
        ("storage_s3_access_key", "", "S3 Access Key ID"),
        ("storage_s3_secret_key", "", "S3 Secret Access Key"),
        ("storage_s3_public_url", "", "Public CDN URL for S3 files (optional)"),
        // Payment Settings
        ("payment_midtrans_enabled", "false", "Enable Midtrans Payment Gateway"),
        ("payment_midtrans_merchant_id", "", "Midtrans Merchant ID"),
        ("payment_midtrans_server_key", "", "Midtrans Server Key"),
        ("payment_midtrans_client_key", "", "Midtrans Client Key"),
        ("payment_midtrans_is_production", "false", "Use Midtrans Production Environment"),
        ("payment_manual_enabled", "true", "Enable Manual Bank Transfer"),
        ("payment_manual_instructions", "Please transfer the total amount to one of the bank accounts listed below and upload your proof of payment.", "Instructions for manual bank transfer"),
        // Alerting Settings
        ("alerting_enabled", "false", "Enable error alerting via email"),
        ("alerting_email", "", "Email address to receive alerts"),
        ("alerting_error_threshold", "5.0", "Error rate threshold percentage to trigger alert"),
        ("alerting_rate_limit_threshold", "50", "Rate limit count threshold to trigger alert"),
        ("alerting_response_time_threshold", "3000.0", "P95 response time threshold in ms"),
        ("alerting_cooldown_minutes", "15", "Minutes to wait before sending same alert type again"),
        // Timezone (IANA TZ database name, e.g. Asia/Jakarta). Used for schedules shown in the UI.
        ("app_timezone", "UTC", "Application timezone for schedules (IANA, e.g. Asia/Jakarta)"),
        // Backup Scheduler
        ("backup_global_enabled", "false", "Enable automatic global backups"),
        ("backup_global_mode", "day", "Global backup schedule mode: minute, hour, day, week"),
        ("backup_global_every", "15", "Global backup interval value for minute/hour modes"),
        ("backup_global_at", "02:00", "Global backup time (HH:MM) for day/week modes (app_timezone)"),
        ("backup_global_weekday", "sun", "Global backup weekday for weekly mode (mon..sun)"),
        ("backup_global_schedule", "0 2 * * *", "Legacy global backup schedule in cron (min hour * * *) or HH:MM (app_timezone)"),
        ("backup_global_retention_days", "30", "Retention days for global backups"),
        ("backup_global_trigger", "false", "Manual trigger for global backup"),
        ("backup_tenant_enabled", "false", "Enable automatic tenant backups"),
        ("backup_tenant_mode", "day", "Tenant backup schedule mode: minute, hour, day, week"),
        ("backup_tenant_every", "60", "Tenant backup interval value for minute/hour modes"),
        ("backup_tenant_at", "02:30", "Tenant backup time (HH:MM) for day/week modes (app_timezone)"),
        ("backup_tenant_weekday", "sun", "Tenant backup weekday for weekly mode (mon..sun)"),
        ("backup_tenant_schedule", "30 2 * * *", "Legacy tenant backup schedule in cron (min hour * * *) or HH:MM (app_timezone)"),
        ("backup_tenant_retention_days", "14", "Retention days for tenant backups"),
        ("backup_tenant_trigger", "false", "Manual trigger for tenant backups"),
    ];

    for (key, value, description) in defaults {
        #[cfg(feature = "postgres")]
        {
            sqlx::query(r#"
                INSERT INTO settings (id, tenant_id, key, value, description, created_at, updated_at)
                VALUES ($1, NULL, $2, $3, $4, $5, $6)
                ON CONFLICT (key) WHERE tenant_id IS NULL DO NOTHING
            "#)
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(key)
            .bind(value)
            .bind(description)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }

        #[cfg(feature = "sqlite")]
        {
            let now_str = now.to_rfc3339();
            sqlx::query(r#"
                INSERT OR IGNORE INTO settings (id, tenant_id, key, value, description, created_at, updated_at)
                VALUES (?, NULL, ?, ?, ?, ?, ?)
            "#)
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(key)
            .bind(value)
            .bind(description)
            .bind(&now_str)
            .bind(&now_str)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

/// Seed default roles and permissions
pub async fn seed_roles(pool: &DbPool) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now();
    let roles = vec![
        ("Owner", "Full access to all resources", true, 100),
        ("Admin", "Access to settings and team management", true, 50),
        ("Member", "Can view dashboard and read team", true, 10),
        ("Viewer", "Read-only access", true, 0),
    ];

    for (name, description, is_system, level) in roles {
        let role_id = uuid::Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        {
            sqlx::query(r#"
                INSERT INTO roles (id, tenant_id, name, description, is_system, level, created_at, updated_at)
                VALUES ($1, NULL, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (name) WHERE tenant_id IS NULL DO NOTHING
            "#)
            .bind(&role_id)
            .bind(name)
            .bind(description)
            .bind(is_system)
            .bind(level)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }

        #[cfg(feature = "sqlite")]
        {
            let now_str = now.to_rfc3339();
            // Check if exists first for SQLite to simulate filtered unique index behavior if needed,
            // but since name isn't unique globally (only per tenant or null), we need careful insertion.
            // Simplified: Insert if not exists where tenant_id is null.
            let exists: bool = sqlx::query_scalar(
                "SELECT COUNT(*) FROM roles WHERE name = ? AND tenant_id IS NULL",
            )
            .bind(name)
            .fetch_one(pool)
            .await
            .map(|c: i64| c > 0)
            .unwrap_or(false);

            if !exists {
                sqlx::query(r#"
                    INSERT INTO roles (id, tenant_id, name, description, is_system, level, created_at, updated_at)
                    VALUES (?, NULL, ?, ?, ?, ?, ?, ?)
                "#)
                .bind(&role_id)
                .bind(name)
                .bind(description)
                .bind(is_system)
                .bind(level)
                .bind(&now_str)
                .bind(&now_str)
                .execute(pool)
                .await?;
            }
        }
    }

    // Fix missing role_ids for existing Owners
    #[cfg(feature = "postgres")]
    sqlx::query(
        r#"
        UPDATE tenant_members 
        SET role_id = (SELECT id FROM roles WHERE name = 'Owner' AND tenant_id IS NULL LIMIT 1)
        WHERE role IN ('Owner', 'owner') AND role_id IS NULL
    "#,
    )
    .execute(pool)
    .await?;

    #[cfg(feature = "sqlite")]
    sqlx::query(
        r#"
        UPDATE tenant_members 
        SET role_id = (SELECT id FROM roles WHERE name = 'Owner' AND tenant_id IS NULL LIMIT 1)
        WHERE role IN ('Owner', 'owner') AND role_id IS NULL
    "#,
    )
    .execute(pool)
    .await?;

    // Fix levels for existing roles
    #[cfg(feature = "postgres")]
    {
        sqlx::query("UPDATE roles SET level = 100 WHERE name = 'Owner' AND level = 0")
            .execute(pool)
            .await?;
        sqlx::query("UPDATE roles SET level = 50 WHERE name = 'Admin' AND level = 0")
            .execute(pool)
            .await?;
        sqlx::query("UPDATE roles SET level = 10 WHERE name = 'Member' AND level = 0")
            .execute(pool)
            .await?;
    }

    #[cfg(feature = "sqlite")]
    {
        sqlx::query("UPDATE roles SET level = 100 WHERE name = 'Owner' AND level = 0")
            .execute(pool)
            .await?;
        sqlx::query("UPDATE roles SET level = 50 WHERE name = 'Admin' AND level = 0")
            .execute(pool)
            .await?;
        sqlx::query("UPDATE roles SET level = 10 WHERE name = 'Member' AND level = 0")
            .execute(pool)
            .await?;
    }

    // --- Seed Permissions ---
    let permissions = vec![
        // Admin Panel Access (gate for admin UI)
        ("admin", "access", "Access to admin panel"),
        // Dashboard Access (default for all roles)
        ("dashboard", "read", "Access to user dashboard"),
        // Team Management
        ("team", "read", "View team members"),
        ("team", "create", "Invite new members"),
        ("team", "update", "Update member roles"),
        ("team", "delete", "Remove members"),
        // Role Management
        ("roles", "read", "View roles"),
        ("roles", "create", "Create new roles"),
        ("roles", "update", "Update roles"),
        ("roles", "delete", "Delete roles"),
        // Settings Management
        ("settings", "read", "View settings"),
        ("settings", "update", "Update settings"),
        // Storage Management
        ("storage", "read", "View files"),
        ("storage", "upload", "Upload files"),
        ("storage", "delete", "Delete files"),
        // Backups (tenant-scoped)
        ("backups", "read", "View backups page"),
        ("backups", "create", "Create backups"),
        ("backups", "download", "Download backups"),
        ("backups", "restore", "Restore backups"),
        ("backups", "delete", "Delete backups"),
        // Support Tickets (tenant-scoped)
        ("support", "create", "Create support tickets"),
        ("support", "read", "Read own support tickets"),
        ("support", "read_all", "Read all support tickets in tenant"),
        ("support", "reply", "Reply to support tickets"),
        (
            "support",
            "update",
            "Update support tickets (status/priority)",
        ),
        ("support", "assign", "Assign support tickets"),
        ("support", "internal", "Post internal support notes"),
    ];

    // Cleanup: Remove permissions with non-standard IDs (e.g. random UUIDs)
    // This resolves "duplicate key value" conflict when we try to insert standardized "res:act" IDs
    #[cfg(feature = "postgres")]
    sqlx::query("DELETE FROM permissions WHERE id != resource || ':' || action")
        .execute(pool)
        .await?;

    #[cfg(feature = "sqlite")]
    sqlx::query("DELETE FROM permissions WHERE id != resource || ':' || action")
        .execute(pool)
        .await?;

    for (resource, action, description) in permissions {
        let perm_id = format!("{}:{}", resource, action); // Use deterministic ID for permissions

        // Upsert permission
        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO permissions (id, resource, action, description)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO NOTHING
        "#,
        )
        .bind(&perm_id)
        .bind(resource)
        .bind(action)
        .bind(description)
        .execute(pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO permissions (id, resource, action, description)
            VALUES (?, ?, ?, ?)
        "#,
        )
        .bind(&perm_id)
        .bind(resource)
        .bind(action)
        .bind(description)
        .execute(pool)
        .await?;
    }

    // --- Assign Permissions to Roles ---
    // Helper to assign permission to role name
    async fn assign_perm(pool: &DbPool, role_name: &str, perm_id: &str) -> Result<(), sqlx::Error> {
        #[cfg(feature = "postgres")]
        let role_query = "SELECT id FROM roles WHERE name = $1 AND tenant_id IS NULL";
        #[cfg(feature = "sqlite")]
        let role_query = "SELECT id FROM roles WHERE name = ? AND tenant_id IS NULL";

        let role_id: Option<(String,)> = sqlx::query_as(role_query)
            .bind(role_name)
            .fetch_optional(pool)
            .await?;

        if let Some((rid,)) = role_id {
            #[cfg(feature = "postgres")]
            sqlx::query("INSERT INTO role_permissions (role_id, permission_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(&rid)
                .bind(perm_id)
                .execute(pool)
                .await?;

            #[cfg(feature = "sqlite")]
            sqlx::query(
                "INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)",
            )
            .bind(&rid)
            .bind(perm_id)
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    // Owner gets everything (handled via logic usually, but let's be explicit for now)
    // Actually, usually Owner bypasses checks, but for RBAC purity we can assign all.
    // For now, let's assign Admin specific permissions.
    let admin_perms = vec![
        "admin:access", // Access to admin panel
        "team:read",
        "team:create",
        "team:update",
        "team:delete",
        "roles:read",
        "roles:create",
        "roles:update",
        "roles:delete",
        "settings:read",
        "settings:update",
        "storage:read",
        "storage:upload",
        "storage:delete",
        "backups:read",
        "backups:create",
        "backups:download",
        "backups:restore",
        "backups:delete",
        "support:create",
        "support:read",
        "support:read_all",
        "support:reply",
        "support:update",
        "support:assign",
        "support:internal",
    ];
    for p in admin_perms {
        assign_perm(pool, "Admin", p).await?;
        assign_perm(pool, "Owner", p).await?; // Owner gets all admin perms too
    }

    let member_perms = vec![
        "dashboard:read",
        "team:read",
        "storage:read",
        "storage:upload",
        "support:create",
        "support:read",
        "support:reply",
    ];
    for p in member_perms {
        assign_perm(pool, "Member", p).await?;
    }

    // All roles get dashboard access by default
    let all_roles_perms = vec!["dashboard:read", "storage:read"];
    for p in all_roles_perms {
        assign_perm(pool, "Owner", p).await?;
        assign_perm(pool, "Admin", p).await?;
        assign_perm(pool, "Member", p).await?;
        assign_perm(pool, "Viewer", p).await?;
    }

    Ok(())
}

/// Seed default subscription plans
pub async fn seed_plans(pool: &DbPool) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now();

    // 1. Seed Features
    let features = vec![
        (
            "max_storage_gb",
            "Storage Limit (GB)",
            "Maximum storage space allowed",
            "number",
            "0.5",
        ),
        (
            "max_members",
            "Team Member Limit",
            "Maximum number of team members",
            "number",
            "2",
        ),
        (
            "support_level",
            "Support Level",
            "Level of customer support provided",
            "string",
            "basic",
        ),
        (
            "custom_domain",
            "Custom Domain",
            "Ability to use custom domain",
            "boolean",
            "false",
        ),
    ];

    for (code, name, desc, vtype, default_val) in features {
        let id = uuid::Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        sqlx::query(r#"
            INSERT INTO features (id, code, name, description, value_type, default_value, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (code) DO NOTHING
        "#)
        .bind(id).bind(code).bind(name).bind(desc).bind(vtype).bind(default_val).bind(now)
        .execute(pool).await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(r#"
            INSERT OR IGNORE INTO features (id, code, name, description, value_type, default_value, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(id).bind(code).bind(name).bind(desc).bind(vtype).bind(default_val).bind(now.to_rfc3339())
        .execute(pool).await?;
    }

    // 2. Seed Plans
    let plans = vec![
        (
            "Free",
            "free",
            "Perfect for getting started",
            0.0,
            0.0,
            true,
            true,
            1,
        ),
        (
            "Pro",
            "pro",
            "For growing teams",
            290_000.0,
            2_900_000.0,
            true,
            false,
            2,
        ),
        (
            "Enterprise",
            "enterprise",
            "For large organizations",
            990_000.0,
            9_900_000.0,
            true,
            false,
            3,
        ),
    ];

    for (name, slug, desc, price_m, price_y, active, is_default, order) in plans {
        let plan_id = uuid::Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        sqlx::query(r#"
            INSERT INTO plans (id, name, slug, description, price_monthly, price_yearly, is_active, is_default, sort_order, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (slug) DO NOTHING
        "#)
        .bind(&plan_id).bind(name).bind(slug).bind(desc).bind(price_m).bind(price_y).bind(active).bind(is_default).bind(order).bind(now).bind(now)
        .execute(pool).await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(r#"
            INSERT OR IGNORE INTO plans (id, name, slug, description, price_monthly, price_yearly, is_active, is_default, sort_order, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&plan_id).bind(name).bind(slug).bind(desc).bind(price_m).bind(price_y).bind(active).bind(is_default).bind(order).bind(now.to_rfc3339()).bind(now.to_rfc3339())
        .execute(pool).await?;

        // 3. Link Features to Plans (Fetch IDs first)
        #[cfg(feature = "postgres")]
        let pid_query = "SELECT id FROM plans WHERE slug = $1";
        #[cfg(feature = "sqlite")]
        let pid_query = "SELECT id FROM plans WHERE slug = ?";

        let fetched_pid: Option<String> = sqlx::query_scalar(pid_query)
            .bind(slug)
            .fetch_optional(pool)
            .await?;

        if let Some(pid) = fetched_pid {
            let features_to_add = match slug {
                "free" => vec![
                    ("max_storage_gb", "0.5"),
                    ("max_members", "2"),
                    ("support_level", "community"),
                    ("custom_domain", "false"),
                ],
                "pro" => vec![
                    ("max_storage_gb", "50"),
                    ("max_members", "10"),
                    ("support_level", "priority"),
                    ("custom_domain", "true"),
                ],
                "enterprise" => vec![
                    ("max_storage_gb", "500"),
                    ("max_members", "999"),
                    ("support_level", "dedicated"),
                    ("custom_domain", "true"),
                ],
                _ => vec![],
            };

            for (code, val) in features_to_add {
                #[cfg(feature = "postgres")]
                let fid_query = "SELECT id FROM features WHERE code = $1";
                #[cfg(feature = "sqlite")]
                let fid_query = "SELECT id FROM features WHERE code = ?";

                let fid: Option<String> = sqlx::query_scalar(fid_query)
                    .bind(code)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None);

                if let Some(fid) = fid {
                    let pf_id = uuid::Uuid::new_v4().to_string();
                    #[cfg(feature = "postgres")]
                    sqlx::query("INSERT INTO plan_features (id, plan_id, feature_id, value) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING")
                        .bind(&pf_id).bind(&pid).bind(&fid).bind(val).execute(pool).await.ok();
                    #[cfg(feature = "sqlite")]
                    sqlx::query("INSERT OR IGNORE INTO plan_features (id, plan_id, feature_id, value) VALUES (?, ?, ?, ?)")
                        .bind(&pf_id).bind(&pid).bind(&fid).bind(val).execute(pool).await.ok();
                }
            }
        }
    }

    Ok(())
}
