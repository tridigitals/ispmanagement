use super::DbPool;
use std::env;

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
        // API Security
        ("api_rate_limit_per_minute", "300", "Baseline API rate limit per minute (per user/IP); auth and expensive endpoints use stricter policies"),
        ("enable_ip_blocking", "false", "Enable automatic IP blocking on suspicious activity"),
        ("ip_block_threshold", "5", "How many rate-limit hits within a window will trigger blocking"),
        ("ip_block_duration_minutes", "15", "How long an IP stays blocked after triggering"),
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
        ("payment_duitku_enabled", "false", "Enable Duitku Payment Gateway"),
        ("payment_duitku_merchant_code", "", "Duitku Merchant Code"),
        ("payment_duitku_api_key", "", "Duitku API Key"),
        ("payment_duitku_payment_method", "", "Duitku Payment Method Code"),
        ("payment_duitku_payment_methods", "[]", "Enabled Duitku Payment Method Codes"),
        ("payment_duitku_is_production", "false", "Use Duitku Production Environment"),
        ("payment_manual_enabled", "true", "Enable Manual Bank Transfer"),
        ("payment_manual_instructions", "Please transfer the total amount to one of the bank accounts listed below and upload your proof of payment.", "Instructions for manual bank transfer"),
        ("customer_invoice_auto_generate_enabled", "true", "Enable automatic background generation for customer package invoices"),
        ("customer_invoice_generate_days_before_due", "7", "How many days before renewal to generate customer package invoice"),
        ("customer_invoice_scheduler_interval_minutes", "60", "How often background scheduler checks due customer invoices (minutes)"),
        ("billing_auto_suspend_enabled", "false", "Automatically suspend unpaid customer subscriptions after grace period"),
        ("billing_auto_suspend_mode", "grace_period", "Global customer subscription auto suspend mode: grace_period or fixed_day"),
        ("billing_auto_suspend_grace_days", "3", "Grace period (days) after due date before auto suspend is allowed"),
        ("billing_auto_suspend_fixed_day", "1", "Global fixed suspend day of month for overdue customer subscriptions (1-28)"),
        ("billing_auto_suspend_pppoe_action", "disable_secret", "Action for PPPoE when a customer subscription is suspended: disable_secret or move_to_isolation_pool"),
        ("billing_auto_suspend_isolation_pool", "", "Isolation IP pool name used when billing auto suspend PPPoE action moves customers to isolation pool"),
        ("billing_auto_resume_on_payment", "true", "Automatically resume suspended customer subscriptions when invoice is paid"),
        ("billing_reminder_enabled", "true", "Enable automatic invoice reminder notifications"),
        ("billing_reminder_schedule", "H-3,H-1,H+1,H+3", "Reminder schedule offsets around due date, comma separated"),
        ("installation_sla_reminder_enabled", "true", "Enable SLA reminder notifications for overdue installation work orders"),
        ("installation_sla_overdue_minutes", "120", "Minutes after schedule when installation work order is considered overdue"),
        ("installation_sla_reminder_cooldown_minutes", "180", "Cooldown in minutes before repeating the same installation SLA reminder"),
        ("installation_sla_scheduler_interval_minutes", "15", "How often installation SLA scheduler scans for overdue work orders (minutes)"),
        // Alerting Settings
        ("alerting_enabled", "false", "Enable error alerting via email"),
        ("alerting_email", "", "Email address to receive alerts"),
        ("alerting_error_threshold", "5.0", "Error rate threshold percentage to trigger alert"),
        ("alerting_rate_limit_threshold", "50", "Rate limit count threshold to trigger alert"),
        ("alerting_response_time_threshold", "3000.0", "P95 response time threshold in ms"),
        ("alerting_cooldown_minutes", "15", "Minutes to wait before sending same alert type again"),
        // MikroTik Metrics Retention
        ("mikrotik_metrics_retention_days", "14", "Retention days for mikrotik_router_metrics and mikrotik_interface_metrics (0 = disable cleanup)"),
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
        // Email Outbox
        ("email_outbox_enabled", "true", "Queue outgoing emails and retry failures"),
        ("email_outbox_max_attempts", "5", "Max retry attempts for queued emails"),
        ("email_outbox_base_delay_seconds", "30", "Base retry delay in seconds for queued emails (exponential backoff)"),
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
        (
            "NOC",
            "Network operations center access for monitoring and provisioning",
            true,
            35,
        ),
        (
            "Planner",
            "Plan network topology, zones, and coverage",
            true,
            30,
        ),
        (
            "Customer Service",
            "Handle customers, tickets, and billing communication",
            true,
            25,
        ),
        (
            "Technician",
            "Field technician for installation and activation tasks",
            true,
            20,
        ),
        ("Member", "Can view dashboard and read team", true, 10),
        ("Viewer", "Read-only access", true, 0),
        (
            "Customer",
            "Customer portal access (dashboard only)",
            true,
            0,
        ),
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
        sqlx::query("UPDATE roles SET level = 35 WHERE name = 'NOC' AND level = 0")
            .execute(pool)
            .await?;
        sqlx::query("UPDATE roles SET level = 30 WHERE name = 'Planner' AND level = 0")
            .execute(pool)
            .await?;
        sqlx::query("UPDATE roles SET level = 25 WHERE name = 'Customer Service' AND level = 0")
            .execute(pool)
            .await?;
        sqlx::query("UPDATE roles SET level = 20 WHERE name = 'Technician' AND level = 0")
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
        sqlx::query("UPDATE roles SET level = 35 WHERE name = 'NOC' AND level = 0")
            .execute(pool)
            .await?;
        sqlx::query("UPDATE roles SET level = 30 WHERE name = 'Planner' AND level = 0")
            .execute(pool)
            .await?;
        sqlx::query("UPDATE roles SET level = 25 WHERE name = 'Customer Service' AND level = 0")
            .execute(pool)
            .await?;
        sqlx::query("UPDATE roles SET level = 20 WHERE name = 'Technician' AND level = 0")
            .execute(pool)
            .await?;
        sqlx::query("UPDATE roles SET level = 10 WHERE name = 'Member' AND level = 0")
            .execute(pool)
            .await?;
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
