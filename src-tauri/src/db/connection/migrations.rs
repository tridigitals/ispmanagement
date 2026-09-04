#[cfg(feature = "sqlite")]
use super::seed::{seed_defaults, seed_roles};
use super::DbPool;
use tracing::info;

#[cfg(feature = "postgres")]
pub(super) async fn run_migrations_pg(pool: &DbPool) -> Result<(), sqlx::Error> {
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
pub(super) async fn run_migrations_sqlite(pool: &DbPool) -> Result<(), sqlx::Error> {
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

    // Create user_addresses table (multi address support)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_addresses (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            label TEXT,
            recipient_name TEXT,
            phone TEXT,
            line1 TEXT NOT NULL,
            line2 TEXT,
            city TEXT,
            state TEXT,
            postal_code TEXT,
            country_code TEXT NOT NULL DEFAULT 'ID',
            is_default_shipping INTEGER NOT NULL DEFAULT 0,
            is_default_billing INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS user_addresses_user_id_idx ON user_addresses(user_id)",
    )
    .execute(pool)
    .await;

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
            rejection_reason TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
            FOREIGN KEY (merchant_id) REFERENCES tenants(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Billing collection logs (SQLite)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS invoice_reminder_logs (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT NOT NULL,
            invoice_id TEXT NOT NULL,
            reminder_code TEXT NOT NULL,
            channel TEXT NOT NULL DEFAULT 'email',
            recipient TEXT,
            status TEXT NOT NULL DEFAULT 'sent',
            detail TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
            FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS billing_collection_logs (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT NOT NULL,
            invoice_id TEXT NOT NULL,
            subscription_id TEXT,
            action TEXT NOT NULL,
            result TEXT NOT NULL,
            reason TEXT,
            actor_type TEXT NOT NULL DEFAULT 'system',
            actor_id TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
            FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    // Customer registration invites (SQLite)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS customer_registration_invites (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT NOT NULL,
            token_hash TEXT NOT NULL UNIQUE,
            token_enc TEXT,
            created_by TEXT,
            max_uses INTEGER NOT NULL DEFAULT 1,
            used_count INTEGER NOT NULL DEFAULT 0,
            expires_at TEXT NOT NULL,
            is_revoked INTEGER NOT NULL DEFAULT 0,
            revoked_at TEXT,
            last_used_at TEXT,
            note TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
            FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL
        )
    "#,
    )
    .execute(pool)
    .await?;

    let _ = sqlx::query("ALTER TABLE customer_registration_invites ADD COLUMN token_enc TEXT")
        .execute(pool)
        .await;

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

    // Migration: Add tenant_id to bank_accounts if not exists (SQLite)
    let _ = sqlx::query(
        "ALTER TABLE bank_accounts ADD COLUMN tenant_id TEXT REFERENCES tenants(id) ON DELETE CASCADE",
    )
    .execute(pool)
    .await;

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
    let _ = sqlx::query("ALTER TABLE invoices ADD COLUMN rejection_reason TEXT")
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
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_invoice_reminder_logs_tenant_created ON invoice_reminder_logs(tenant_id, created_at DESC)",
    )
    .execute(pool)
    .await
    .ok();
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_invoice_reminder_logs_invoice_created ON invoice_reminder_logs(invoice_id, created_at DESC)",
    )
    .execute(pool)
    .await
    .ok();
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_billing_collection_logs_tenant_created ON billing_collection_logs(tenant_id, created_at DESC)",
    )
    .execute(pool)
    .await
    .ok();
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_billing_collection_logs_invoice_created ON billing_collection_logs(invoice_id, created_at DESC)",
    )
    .execute(pool)
    .await
    .ok();
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_customer_registration_invites_tenant_created ON customer_registration_invites(tenant_id, created_at DESC)",
    )
    .execute(pool)
    .await
    .ok();
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_customer_registration_invites_tenant_expires ON customer_registration_invites(tenant_id, expires_at DESC)",
    )
    .execute(pool)
    .await
    .ok();

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

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS message_templates (
            id TEXT PRIMARY KEY NOT NULL,
            tenant_id TEXT NOT NULL,
            key TEXT NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            use_case TEXT NOT NULL DEFAULT 'custom',
            target TEXT NOT NULL DEFAULT 'customer',
            trigger_mode TEXT NOT NULL DEFAULT 'manual',
            event_key TEXT,
            channel TEXT NOT NULL DEFAULT 'whatsapp',
            locale TEXT NOT NULL DEFAULT 'id-ID',
            status TEXT NOT NULL DEFAULT 'draft',
            whatsapp_body TEXT,
            email_subject TEXT,
            email_body TEXT,
            variables TEXT NOT NULL DEFAULT '[]',
            version INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(tenant_id, key),
            FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
        )
    "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_message_templates_tenant_updated ON message_templates(tenant_id, updated_at DESC)",
    )
    .execute(pool)
    .await
    .ok();
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_message_templates_tenant_filters ON message_templates(tenant_id, status, channel, target, trigger_mode)",
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        r#"
        WITH default_templates AS (
            SELECT
                'billing_payment_reminder' AS key,
                'Billing - Friendly Payment Reminder' AS name,
                'A polite manual reminder for customers who need a payment follow-up.' AS description,
                'billing' AS use_case,
                'manual' AS trigger_mode,
                'billing.payment_reminder' AS event_key,
                'both' AS channel,
                'active' AS status,
                'Halo {{customer.name}}, kami dari {{tenant.name}} ingin mengingatkan tagihan layanan internet Anda.

Jika sudah melakukan pembayaran, abaikan pesan ini. Jika membutuhkan bantuan, balas pesan ini agar tim kami bisa membantu.' AS whatsapp_body,
                'Pengingat pembayaran layanan {{tenant.name}}' AS email_subject,
                'Halo {{customer.name}},

Kami dari {{tenant.name}} ingin mengingatkan tagihan layanan internet Anda.

Jika pembayaran sudah dilakukan, email ini dapat diabaikan. Jika ada kendala pembayaran atau membutuhkan bantuan, silakan hubungi tim kami melalui channel resmi.

Terima kasih,
{{tenant.name}}' AS email_body,
                '["tenant.name","customer.name"]' AS variables
            UNION ALL SELECT
                'billing_overdue_followup',
                'Billing - Overdue Follow-up',
                'A firmer follow-up for overdue billing without sounding aggressive.',
                'billing',
                'manual',
                'billing.overdue_followup',
                'both',
                'active',
                'Halo {{customer.name}}, kami dari {{tenant.name}} mencatat pembayaran layanan Anda masih perlu ditindaklanjuti.

Mohon lakukan pembayaran atau hubungi kami jika ada kendala, agar layanan tetap berjalan dengan baik.',
                'Tindak lanjut pembayaran layanan {{tenant.name}}',
                'Halo {{customer.name}},

Kami mencatat pembayaran layanan Anda masih perlu ditindaklanjuti.

Mohon lakukan pembayaran melalui metode yang tersedia. Jika ada kendala, balas email ini atau hubungi tim kami agar kami dapat membantu pengecekan.

Terima kasih,
{{tenant.name}}',
                '["tenant.name","customer.name"]'
            UNION ALL SELECT
                'installation_schedule_confirmation',
                'Installation - Schedule Confirmation',
                'Confirm installation readiness and keep the customer informed before field work.',
                'installation',
                'manual',
                'installation.schedule_confirmation',
                'both',
                'active',
                'Halo {{customer.name}}, tim {{tenant.name}} akan menindaklanjuti jadwal instalasi layanan Anda.

Mohon pastikan lokasi dapat diakses dan nomor ini aktif untuk koordinasi teknisi. Jika ada perubahan jadwal, balas pesan ini.',
                'Konfirmasi jadwal instalasi {{tenant.name}}',
                'Halo {{customer.name}},

Tim {{tenant.name}} akan menindaklanjuti jadwal instalasi layanan Anda.

Mohon pastikan lokasi dapat diakses, ada PIC yang dapat ditemui, dan nomor kontak tetap aktif untuk koordinasi teknisi. Jika ada perubahan jadwal atau akses lokasi, silakan balas email ini agar tim kami dapat menyesuaikan kunjungan.

Terima kasih,
{{tenant.name}}',
                '["tenant.name","customer.name"]'
            UNION ALL SELECT
                'installation_completed',
                'Installation - Completed Handoff',
                'Send after installation is completed to guide the customer on the next step.',
                'installation',
                'manual',
                'installation.completed',
                'both',
                'active',
                'Halo {{customer.name}}, instalasi layanan {{tenant.name}} sudah selesai.

Silakan coba koneksi internet Anda. Jika ada kendala, balas pesan ini agar tim kami dapat membantu pengecekan.',
                'Instalasi layanan {{tenant.name}} selesai',
                'Halo {{customer.name}},

Instalasi layanan {{tenant.name}} sudah selesai.

Silakan coba koneksi internet Anda. Jika ada kendala setelah instalasi, hubungi tim kami dengan menjelaskan gejala yang dialami agar pengecekan dapat dilakukan lebih cepat.

Terima kasih,
{{tenant.name}}',
                '["tenant.name","customer.name"]'
            UNION ALL SELECT
                'outage_customer_notice',
                'Outage - Customer Notice',
                'Notify customers about an incident while keeping the message calm and concise.',
                'outage',
                'manual',
                'network.outage_notice',
                'both',
                'active',
                'Halo {{customer.name}}, saat ini tim {{tenant.name}} sedang menangani gangguan layanan di beberapa area.

Kami akan menginformasikan pembaruan berikutnya setelah pengecekan selesai. Terima kasih atas kesabarannya.',
                'Informasi gangguan layanan {{tenant.name}}',
                'Halo {{customer.name}},

Saat ini tim {{tenant.name}} sedang menangani gangguan layanan di beberapa area.

Tim teknis sedang melakukan pengecekan dan kami akan menginformasikan pembaruan berikutnya setelah ada perkembangan. Terima kasih atas pengertian dan kesabarannya.

Hormat kami,
{{tenant.name}}',
                '["tenant.name","customer.name"]'
            UNION ALL SELECT
                'support_followup',
                'Support - Follow-up Check',
                'A clean follow-up message after a support case is handled.',
                'support',
                'manual',
                'support.followup',
                'both',
                'active',
                'Halo {{customer.name}}, kami dari {{tenant.name}} ingin memastikan kendala Anda sudah terbantu.

Jika masih ada masalah, balas pesan ini agar tim support dapat melanjutkan pengecekan.',
                'Follow-up bantuan dari {{tenant.name}}',
                'Halo {{customer.name}},

Kami ingin memastikan kendala Anda sudah terbantu oleh tim {{tenant.name}}.

Jika masih ada masalah atau membutuhkan bantuan lanjutan, silakan balas email ini dengan detail kendala yang dialami.

Terima kasih,
{{tenant.name}}',
                '["tenant.name","customer.name"]'
        )
        INSERT OR IGNORE INTO message_templates (
            id,
            tenant_id,
            key,
            name,
            description,
            use_case,
            target,
            trigger_mode,
            event_key,
            channel,
            locale,
            status,
            whatsapp_body,
            email_subject,
            email_body,
            variables,
            version,
            created_at,
            updated_at
        )
        SELECT
            'seed_tpl_' || replace(t.id, '-', '') || '_' || d.key,
            t.id,
            d.key,
            d.name,
            d.description,
            d.use_case,
            'customer',
            d.trigger_mode,
            d.event_key,
            d.channel,
            'id-ID',
            d.status,
            d.whatsapp_body,
            d.email_subject,
            d.email_body,
            d.variables,
            1,
            datetime('now'),
            datetime('now')
        FROM tenants t
        CROSS JOIN default_templates d
        "#,
    )
    .execute(pool)
    .await
    .ok();

    seed_defaults(pool).await?;
    seed_roles(pool).await?;

    Ok(())
}
