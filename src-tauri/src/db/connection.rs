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
pub async fn init_db(_app_data_dir: PathBuf) -> Result<DbPool, sqlx::Error> {
    #[cfg(feature = "postgres")]
    {
        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set for PostgreSQL mode");
        
        info!("Connecting to PostgreSQL database");
        
        let pool = PgPool::connect(&database_url).await?;
        run_migrations_pg(&pool).await?;
        
        info!("PostgreSQL database initialized successfully");
        
        seed_defaults(&pool).await?;
        seed_roles(&pool).await?;

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
    // Create tenants table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS tenants (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            slug TEXT UNIQUE NOT NULL,
            custom_domain TEXT UNIQUE,
            logo_url TEXT,
            is_active BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        )
    "#)
    .execute(pool)
    .await?;

    // Create users table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY NOT NULL,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            name TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            is_super_admin BOOLEAN NOT NULL DEFAULT false,
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

    // Create tenant_members table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS tenant_members (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            role TEXT NOT NULL DEFAULT 'member',
            created_at TIMESTAMPTZ NOT NULL,
            UNIQUE(tenant_id, user_id)
        )
    "#)
    .execute(pool)
    .await?;

    // Create settings table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS settings (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT REFERENCES tenants(id) ON DELETE CASCADE,
            key TEXT NOT NULL,
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
            tenant_id TEXT REFERENCES tenants(id) ON DELETE CASCADE,
            token TEXT NOT NULL UNIQUE,
            expires_at TIMESTAMPTZ NOT NULL,
            created_at TIMESTAMPTZ NOT NULL
        )
    "#)
    .execute(pool)
    .await?;

    // Create permissions table (RBAC)
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS permissions (
            id TEXT PRIMARY KEY NOT NULL,
            resource TEXT NOT NULL,
            action TEXT NOT NULL,
            description TEXT,
            UNIQUE(resource, action)
        )
    "#)
    .execute(pool)
    .await?;

    // Create roles table (RBAC)
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS roles (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT REFERENCES tenants(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            description TEXT,
            is_system BOOLEAN NOT NULL DEFAULT false,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        )
    "#)
    .execute(pool)
    .await?;

    // Create role_permissions pivot table (RBAC)
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS role_permissions (
            role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
            permission_id TEXT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
            PRIMARY KEY (role_id, permission_id)
        )
    "#)
    .execute(pool)
    .await?;

    // Add role_id to tenant_members if not exists
    sqlx::query(r#"
        DO $$ 
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                          WHERE table_name='tenant_members' AND column_name='role_id') THEN
                ALTER TABLE tenant_members ADD COLUMN role_id TEXT REFERENCES roles(id);
            END IF;
        END $$;
    "#)
    .execute(pool)
    .await?;

    // Create indexes
    if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)").execute(pool).await {
        tracing::error!("Failed to create idx_users_email: {}", e);
    }
    // Unique index for Global Settings (where tenant_id is NULL)
    if let Err(e) = sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_settings_global_key ON settings(key) WHERE tenant_id IS NULL").execute(pool).await {
        tracing::error!("Failed to create idx_settings_global_key: {}", e);
    }
    // Unique index for Tenant Settings (where tenant_id is NOT NULL)
    if let Err(e) = sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_settings_tenant_key ON settings(tenant_id, key) WHERE tenant_id IS NOT NULL").execute(pool).await {
        tracing::error!("Failed to create idx_settings_tenant_key: {}", e);
    }
    
    if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_settings_tenant ON settings(tenant_id)").execute(pool).await {
        tracing::error!("Failed to create idx_settings_tenant: {}", e);
    }
    if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token)").execute(pool).await {
        tracing::error!("Failed to create idx_sessions_token: {}", e);
    }
    if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug)").execute(pool).await {
        tracing::error!("Failed to create idx_tenants_slug: {}", e);
    }
    if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_roles_tenant ON roles(tenant_id)").execute(pool).await {
        tracing::error!("Failed to create idx_roles_tenant: {}", e);
    }
    // Unique index for Global Roles
    if let Err(e) = sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_roles_name_global ON roles(name) WHERE tenant_id IS NULL").execute(pool).await {
        tracing::error!("Failed to create idx_roles_name_global: {}", e);
    }

    info!("PostgreSQL migrations completed");
    Ok(())
}

#[cfg(feature = "sqlite")]
async fn run_migrations_sqlite(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    // Create tenants table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS tenants (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            slug TEXT UNIQUE NOT NULL,
            custom_domain TEXT UNIQUE,
            logo_url TEXT,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
    "#)
    .execute(pool)
    .await?;

    // Create users table
    sqlx::query(r#"
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
    "#)
    .execute(pool)
    .await?;

    // Create tenant_members table
    sqlx::query(r#"
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
    "#)
    .execute(pool)
    .await?;

    // Create settings table
    sqlx::query(r#"
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
    "#)
    .execute(pool)
    .await?;

    // Create sessions table
    sqlx::query(r#"
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
    "#)
    .execute(pool)
    .await?;

    // Create permissions table (RBAC)
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS permissions (
            id TEXT PRIMARY KEY NOT NULL,
            resource TEXT NOT NULL,
            action TEXT NOT NULL,
            description TEXT,
            UNIQUE(resource, action)
        )
    "#)
    .execute(pool)
    .await?;

    // Create roles table (RBAC)
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS roles (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT,
            name TEXT NOT NULL,
            description TEXT,
            is_system INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
        )
    "#)
    .execute(pool)
    .await?;

    // Create role_permissions pivot table (RBAC)
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS role_permissions (
            role_id TEXT NOT NULL,
            permission_id TEXT NOT NULL,
            PRIMARY KEY (role_id, permission_id),
            FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
            FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
        )
    "#)
    .execute(pool)
    .await?;

    // Create indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)").execute(pool).await.ok();
    // Unique partial indexes for SQLite
    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_settings_global_key ON settings(key) WHERE tenant_id IS NULL").execute(pool).await.ok();
    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_settings_tenant_key ON settings(tenant_id, key) WHERE tenant_id IS NOT NULL").execute(pool).await.ok();
    
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token)").execute(pool).await.ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug)").execute(pool).await.ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_roles_tenant ON roles(tenant_id)").execute(pool).await.ok();

    info!("SQLite migrations completed");
    
    seed_defaults(pool).await?;
    seed_roles(pool).await?;

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
        ("Owner", "Full access to all resources", true),
        ("Admin", "Access to settings and team management", true),
        ("Member", "Can view dashboard and read team", true),
        ("Viewer", "Read-only access", true),
    ];

    for (name, description, is_system) in roles {
        let role_id = uuid::Uuid::new_v4().to_string();
        
        #[cfg(feature = "postgres")]
        {
            sqlx::query(r#"
                INSERT INTO roles (id, tenant_id, name, description, is_system, created_at, updated_at)
                VALUES ($1, NULL, $2, $3, $4, $5, $6)
                ON CONFLICT (name) WHERE tenant_id IS NULL DO NOTHING
            "#)
            .bind(&role_id)
            .bind(name)
            .bind(description)
            .bind(is_system)
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
            let exists: bool = sqlx::query_scalar("SELECT COUNT(*) FROM roles WHERE name = ? AND tenant_id IS NULL")
                .bind(name)
                .fetch_one(pool)
                .await
                .map(|c: i64| c > 0)
                .unwrap_or(false);

            if !exists {
                sqlx::query(r#"
                    INSERT INTO roles (id, tenant_id, name, description, is_system, created_at, updated_at)
                    VALUES (?, NULL, ?, ?, ?, ?, ?)
                "#)
                .bind(&role_id)
                .bind(name)
                .bind(description)
                .bind(is_system)
                .bind(&now_str)
                .bind(&now_str)
                .execute(pool)
                .await?;
            }
        }
    }

    // Fix missing role_ids for existing Owners
    #[cfg(feature = "postgres")]
    sqlx::query(r#"
        UPDATE tenant_members 
        SET role_id = (SELECT id FROM roles WHERE name = 'Owner' AND tenant_id IS NULL LIMIT 1)
        WHERE role IN ('Owner', 'owner') AND role_id IS NULL
    "#)
    .execute(pool)
    .await?;

    #[cfg(feature = "sqlite")]
    sqlx::query(r#"
        UPDATE tenant_members 
        SET role_id = (SELECT id FROM roles WHERE name = 'Owner' AND tenant_id IS NULL LIMIT 1)
        WHERE role IN ('Owner', 'owner') AND role_id IS NULL
    "#)
    .execute(pool)
    .await?;

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
        sqlx::query(r#"
            INSERT INTO permissions (id, resource, action, description)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO NOTHING
        "#)
        .bind(&perm_id)
        .bind(resource)
        .bind(action)
        .bind(description)
        .execute(pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(r#"
            INSERT OR IGNORE INTO permissions (id, resource, action, description)
            VALUES (?, ?, ?, ?)
        "#)
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
            sqlx::query("INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
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
        "team:read", "team:create", "team:update", "team:delete",
        "roles:read", "roles:create", "roles:update", "roles:delete",
        "settings:read", "settings:update"
    ];
    for p in admin_perms {
        assign_perm(pool, "Admin", p).await?;
        assign_perm(pool, "Owner", p).await?; // Owner gets all admin perms too
    }

    let member_perms = vec!["dashboard:read", "team:read"];
    for p in member_perms {
        assign_perm(pool, "Member", p).await?;
    }

    // All roles get dashboard access by default
    let all_roles_perms = vec!["dashboard:read"];
    for p in all_roles_perms {
        assign_perm(pool, "Owner", p).await?;
        assign_perm(pool, "Admin", p).await?;
        assign_perm(pool, "Member", p).await?;
        assign_perm(pool, "Viewer", p).await?;
    }

    Ok(())
}
