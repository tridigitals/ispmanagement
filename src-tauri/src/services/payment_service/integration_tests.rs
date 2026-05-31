use super::{auto_suspend_threshold_date, AutoSuspendMode, PaymentService};
use crate::http::WsHub;
use crate::services::{
    AuditService, EmailOutboxService, EmailService, NotificationService, PppoeService,
    SettingsService,
};
use chrono::{Datelike, Duration, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

const TEST_ADMIN_DATABASE_URL: &str = "postgres://postgres:postgres@127.0.0.1/postgres";

struct BillingFixture {
    pool: PgPool,
    db_name: String,
    tenant_id: String,
    customer_id: String,
    location_id: String,
    package_id: String,
    router_id: String,
    subscription_id: String,
}

struct TenantSeed {
    tenant_id: String,
    customer_id: String,
    location_id: String,
    package_id: String,
    router_id: String,
    subscription_id: String,
}

impl BillingFixture {
    async fn new() -> Self {
        let db_name = format!("billing_flow_{}", Uuid::new_v4().simple());
        let admin_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(TEST_ADMIN_DATABASE_URL)
            .await
            .expect("postgres admin database should be available");

        sqlx::query(&format!(r#"CREATE DATABASE "{}""#, db_name))
            .execute(&admin_pool)
            .await
            .expect("temporary billing database should be creatable");
        admin_pool.close().await;

        let database_url = format!("postgres://postgres:postgres@127.0.0.1/{db_name}");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("temporary billing database should be connectable");

        create_schema(&pool).await;

        let primary = seed_tenant_fixture(
            &pool,
            "tenant-billing",
            "Tenant Billing",
            "tenant-billing",
            true,
        )
        .await;

        for (key, value) in [
            ("email_outbox_enabled", "false"),
            ("customer_invoice_auto_generate_enabled", "true"),
            ("customer_invoice_generate_days_before_due", "7"),
            ("billing_auto_suspend_enabled", "true"),
            ("billing_auto_suspend_mode", "grace_period"),
            ("billing_auto_suspend_grace_days", "0"),
            ("billing_auto_resume_on_payment", "true"),
            ("billing_reminder_enabled", "false"),
        ] {
            upsert_setting(&pool, Some(&primary.tenant_id), key, value).await;
        }

        Self {
            pool,
            db_name,
            tenant_id: primary.tenant_id,
            customer_id: primary.customer_id,
            location_id: primary.location_id,
            package_id: primary.package_id,
            router_id: primary.router_id,
            subscription_id: primary.subscription_id,
        }
    }

    async fn service(&self) -> PaymentService {
        let audit_service = AuditService::new(self.pool.clone(), None);
        let settings_service = SettingsService::new(self.pool.clone(), audit_service.clone());
        let email_service = EmailService::new(settings_service.clone());
        let email_outbox = EmailOutboxService::new(
            self.pool.clone(),
            settings_service.clone(),
            email_service.clone(),
        );
        let notification_service =
            NotificationService::new(self.pool.clone(), Arc::new(WsHub::new()), email_outbox);
        let auth_service = crate::services::AuthService::new(
            self.pool.clone(),
            "test-jwt-secret".to_string(),
            email_service,
            audit_service.clone(),
            settings_service.clone(),
        );
        let pppoe_service = PppoeService::new(
            self.pool.clone(),
            auth_service,
            audit_service.clone(),
            settings_service,
        );

        PaymentService::new(
            self.pool.clone(),
            notification_service,
            pppoe_service,
            audit_service,
        )
    }

    async fn create_pppoe_account(&self, address_pool: &str) {
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO pppoe_accounts (
              id, tenant_id, router_id, customer_id, location_id, username, password_enc, package_id,
              profile_id, router_profile_name, remote_address, address_pool, disabled, comment,
              account_source, router_present, created_at, updated_at
            )
            VALUES (
              $1, $2, $3, $4, $5, 'billing-user', 'pppoe-pass', $6,
              NULL, 'profile-main', NULL, $7, false, 'billing flow account',
              'router', false, $8, $8
            )
            "#,
        )
        .bind("pppoe-account-1")
        .bind(&self.tenant_id)
        .bind(&self.router_id)
        .bind(&self.customer_id)
        .bind(&self.location_id)
        .bind(&self.package_id)
        .bind(address_pool)
        .bind(now)
        .execute(&self.pool)
        .await
        .expect("pppoe account should seed");
    }

    async fn cleanup(self) {
        let db_name = self.db_name.clone();
        self.pool.close().await;

        let admin_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(TEST_ADMIN_DATABASE_URL)
            .await
            .expect("postgres admin database should be available for cleanup");

        sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = $1")
            .bind(&db_name)
            .execute(&admin_pool)
            .await
            .expect("temporary billing database connections should be terminable");

        sqlx::query(&format!(r#"DROP DATABASE IF EXISTS "{}""#, db_name))
            .execute(&admin_pool)
            .await
            .expect("temporary billing database should be droppable");
        admin_pool.close().await;
    }
}

async fn upsert_setting(pool: &PgPool, tenant_id: Option<&str>, key: &str, value: &str) {
    let now = Utc::now();
    let updated = if tenant_id.is_some() {
        sqlx::query(
            r#"
            UPDATE settings
            SET value = $1, description = $2, updated_at = $3
            WHERE tenant_id = $4 AND key = $5
            "#,
        )
        .bind(value)
        .bind("billing integration test")
        .bind(now)
        .bind(tenant_id)
        .bind(key)
        .execute(pool)
        .await
        .expect("tenant setting should update")
        .rows_affected()
    } else {
        sqlx::query(
            r#"
            UPDATE settings
            SET value = $1, description = $2, updated_at = $3
            WHERE tenant_id IS NULL AND key = $4
            "#,
        )
        .bind(value)
        .bind("billing integration test")
        .bind(now)
        .bind(key)
        .execute(pool)
        .await
        .expect("global setting should update")
        .rows_affected()
    };

    if updated == 0 {
        sqlx::query(
            r#"
            INSERT INTO settings (id, tenant_id, key, value, description, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $6)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(key)
        .bind(value)
        .bind("billing integration test")
        .bind(now)
        .execute(pool)
        .await
        .expect("setting should insert");
    }
}

#[derive(sqlx::FromRow, Debug)]
struct AuditLogRow {
    #[allow(dead_code)]
    action: String,
    resource: String,
    resource_id: Option<String>,
    #[allow(dead_code)]
    tenant_id: Option<String>,
    #[allow(dead_code)]
    user_id: Option<String>,
    details: Option<String>,
}

/// Fetch audit_logs rows by action only.
///
/// NOTE: filtering is by `action` and not `tenant_id` because the production
/// `AuditService::log` postgres path parses `tenant_id` as a `Uuid` and falls
/// back to `NULL` when parsing fails; the integration-test fixture uses string
/// IDs like `"tenant-billing"` which never parse, so rows in audit_logs have
/// `tenant_id = NULL`. Each test uses an isolated temp database, so action
/// alone is sufficient to scope the query.
async fn fetch_audit_logs_by_action(pool: &PgPool, action: &str) -> Vec<AuditLogRow> {
    sqlx::query_as::<_, AuditLogRow>(
        r#"
        SELECT action,
               resource,
               resource_id,
               tenant_id::text AS tenant_id,
               user_id::text AS user_id,
               details
        FROM audit_logs
        WHERE action = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(action)
    .fetch_all(pool)
    .await
    .expect("audit logs should query")
}

async fn latest_customer_invoice(
    pool: &PgPool,
    tenant_id: &str,
    subscription_id: &str,
) -> (String, String) {
    sqlx::query_as(
        r#"
        SELECT id, invoice_number
        FROM invoices
        WHERE tenant_id = $1
          AND external_id LIKE ($2 || '%')
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(format!("pkgsub:{subscription_id}:"))
    .fetch_one(pool)
    .await
    .expect("customer package invoice should exist")
}

async fn seed_tenant_fixture(
    pool: &PgPool,
    key: &str,
    tenant_name: &str,
    tenant_slug: &str,
    completed_installation: bool,
) -> TenantSeed {
    let tenant_id = key.to_string();
    let owner_user_id = format!("{key}-owner");
    let customer_user_id = format!("{key}-customer-user");
    let customer_id = format!("{key}-customer");
    let location_id = format!("{key}-location");
    let package_id = format!("{key}-package");
    let router_id = format!("{key}-router");
    let subscription_id = format!("{key}-subscription");
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO tenants (id, name, slug, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, true, $4, $4)
        "#,
    )
    .bind(&tenant_id)
    .bind(tenant_name)
    .bind(tenant_slug)
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant should seed");

    for (user_id, email, name, role) in [
        (
            owner_user_id.as_str(),
            format!("{key}-owner@example.test"),
            format!("{tenant_name} Owner"),
            "owner".to_string(),
        ),
        (
            customer_user_id.as_str(),
            format!("{key}-customer@example.test"),
            format!("{tenant_name} Customer"),
            "customer".to_string(),
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO users (
              id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, false, true, $6, $6)
            "#,
        )
        .bind(user_id)
        .bind(email)
        .bind("test-password-hash")
        .bind(name)
        .bind(role)
        .bind(now)
        .execute(pool)
        .await
        .expect("user should seed");
    }

    for (membership_id, user_id, role) in [
        (
            format!("{key}-membership-owner"),
            owner_user_id.as_str(),
            "owner",
        ),
        (
            format!("{key}-membership-customer"),
            customer_user_id.as_str(),
            "customer",
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO tenant_members (id, tenant_id, user_id, role, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(membership_id)
        .bind(&tenant_id)
        .bind(user_id)
        .bind(role)
        .bind(now)
        .execute(pool)
        .await
        .expect("tenant member should seed");
    }

    sqlx::query(
        r#"
        INSERT INTO customers (id, tenant_id, name, email, phone, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, true, $6, $6)
        "#,
    )
    .bind(&customer_id)
    .bind(&tenant_id)
    .bind(format!("{tenant_name} Customer"))
    .bind(format!("{key}-customer@example.test"))
    .bind("08123456789")
    .bind(now)
    .execute(pool)
    .await
    .expect("customer should seed");

    sqlx::query(
        r#"
        INSERT INTO customer_users (id, tenant_id, customer_id, user_id, created_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(format!("{key}-customer-user-link"))
    .bind(&tenant_id)
    .bind(&customer_id)
    .bind(&customer_user_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("customer user should seed");

    sqlx::query(
        r#"
        INSERT INTO customer_locations (
          id, tenant_id, customer_id, label, address_line1, city, country, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
        "#,
    )
    .bind(&location_id)
    .bind(&tenant_id)
    .bind(&customer_id)
    .bind("Rumah")
    .bind("Jl. Billing")
    .bind("Jakarta")
    .bind("ID")
    .bind(now)
    .execute(pool)
    .await
    .expect("location should seed");

    sqlx::query(
        r#"
        INSERT INTO mikrotik_routers (
          id, tenant_id, name, host, port, username, password, use_tls, enabled, is_online, created_at, updated_at
        )
        VALUES ($1, $2, $3, '127.0.0.1', 1, 'admin', 'router-pass', false, true, false, $4, $4)
        "#,
    )
    .bind(&router_id)
    .bind(&tenant_id)
    .bind(format!("{tenant_name} Router"))
    .bind(now)
    .execute(pool)
    .await
    .expect("router should seed");

    sqlx::query(
        r#"
        INSERT INTO isp_packages (
          id, tenant_id, service_type, provisioning_type, name, description, features, is_active,
          price_monthly, price_yearly, created_at, updated_at
        )
        VALUES ($1, $2, 'internet_pppoe', 'pppoe', $3, 'Package for billing test', '{}', true, 150000, 1500000, $4, $4)
        "#,
    )
    .bind(&package_id)
    .bind(&tenant_id)
    .bind(format!("{tenant_name} Package"))
    .bind(now)
    .execute(pool)
    .await
    .expect("package should seed");

    sqlx::query(
        r#"
        INSERT INTO isp_package_router_mappings (
          id, tenant_id, router_id, package_id, router_profile_name, address_pool, isolation_pool, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, 'profile-main', 'pool-main', 'pool-isolir-default', $5, $5)
        "#,
    )
    .bind(format!("{key}-mapping"))
    .bind(&tenant_id)
    .bind(&router_id)
    .bind(&package_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("mapping should seed");

    sqlx::query(
        r#"
        INSERT INTO customer_subscriptions (
          id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle,
          price, currency_code, status, starts_at, ends_at, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'monthly', 150000, 'IDR', 'active', $7, $8, $9, $9)
        "#,
    )
    .bind(&subscription_id)
    .bind(&tenant_id)
    .bind(&customer_id)
    .bind(&location_id)
    .bind(&package_id)
    .bind(&router_id)
    .bind(now - Duration::days(29))
    .bind(now + Duration::days(90))
    .bind(now)
    .execute(pool)
    .await
    .expect("subscription should seed");

    if completed_installation {
        sqlx::query(
            r#"
            INSERT INTO installation_work_orders (
              id, tenant_id, subscription_id, customer_id, location_id, router_id, status,
              completed_at, notes, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'completed', $7, 'Completed install', $8, $8)
            "#,
        )
        .bind(format!("{key}-work-order-completed"))
        .bind(&tenant_id)
        .bind(&subscription_id)
        .bind(&customer_id)
        .bind(&location_id)
        .bind(&router_id)
        .bind(now - Duration::days(40))
        .bind(now)
        .execute(pool)
        .await
        .expect("completed installation work order should seed");
    }

    TenantSeed {
        tenant_id,
        customer_id,
        location_id,
        package_id,
        router_id,
        subscription_id,
    }
}

async fn create_schema(pool: &PgPool) {
    sqlx::raw_sql(
        r#"
        CREATE TABLE tenants (
            id text PRIMARY KEY NOT NULL,
            name text NOT NULL,
            slug text NOT NULL UNIQUE,
            custom_domain text,
            logo_url text,
            is_active boolean NOT NULL DEFAULT true,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL,
            storage_usage bigint NOT NULL DEFAULT 0,
            enforce_2fa boolean NOT NULL DEFAULT false
        );

        CREATE TABLE users (
            id text PRIMARY KEY NOT NULL,
            email text NOT NULL UNIQUE,
            password_hash text NOT NULL,
            name text NOT NULL,
            role text NOT NULL DEFAULT 'user',
            is_super_admin boolean NOT NULL DEFAULT false,
            avatar_url text,
            is_active boolean NOT NULL DEFAULT true,
            email_verified_at timestamp with time zone,
            failed_login_attempts integer NOT NULL DEFAULT 0,
            locked_until timestamp with time zone,
            verification_token text,
            reset_token text,
            reset_token_expires timestamp with time zone,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL,
            two_factor_enabled boolean NOT NULL DEFAULT false,
            two_factor_secret text,
            two_factor_recovery_codes text,
            email_otp_code text,
            email_otp_expires_at timestamp with time zone
        );

        CREATE TABLE tenant_members (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            user_id text NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            role text NOT NULL DEFAULT 'member',
            created_at timestamp with time zone NOT NULL,
            role_id text,
            UNIQUE (tenant_id, user_id)
        );

        CREATE TABLE settings (
            id text PRIMARY KEY NOT NULL,
            tenant_id text REFERENCES tenants(id) ON DELETE CASCADE,
            key text NOT NULL,
            value text NOT NULL,
            description text,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL
        );

        CREATE UNIQUE INDEX idx_settings_global_key
            ON settings (key) WHERE tenant_id IS NULL;
        CREATE UNIQUE INDEX idx_settings_tenant_key
            ON settings (tenant_id, key) WHERE tenant_id IS NOT NULL;

        CREATE TABLE customers (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            name text NOT NULL,
            email text,
            phone text,
            notes text,
            is_active boolean NOT NULL DEFAULT true,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL
        );

        CREATE TABLE customer_users (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            customer_id text NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
            user_id text NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            created_at timestamp with time zone NOT NULL,
            UNIQUE (tenant_id, user_id),
            UNIQUE (customer_id, user_id)
        );

        CREATE TABLE customer_locations (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            customer_id text NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
            label text NOT NULL,
            address_line1 text,
            address_line2 text,
            city text,
            state text,
            postal_code text,
            country text,
            latitude numeric(10,6),
            longitude numeric(10,6),
            notes text,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL
        );

        CREATE TABLE notifications (
            id text PRIMARY KEY NOT NULL,
            user_id text REFERENCES users(id) ON DELETE CASCADE,
            tenant_id text REFERENCES tenants(id) ON DELETE CASCADE,
            title text NOT NULL,
            message text NOT NULL,
            type text NOT NULL DEFAULT 'info',
            is_read boolean NOT NULL DEFAULT false,
            link text,
            created_at timestamp with time zone NOT NULL,
            notification_type text DEFAULT 'info',
            category text DEFAULT 'system',
            action_url text
        );

        CREATE TABLE notification_preferences (
            id text PRIMARY KEY NOT NULL,
            user_id text NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            channel text NOT NULL,
            category text NOT NULL,
            enabled boolean NOT NULL DEFAULT true,
            updated_at timestamp with time zone NOT NULL,
            UNIQUE (user_id, channel, category)
        );

        CREATE TABLE push_subscriptions (
            id text PRIMARY KEY NOT NULL,
            user_id text NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            endpoint text NOT NULL,
            p256dh text NOT NULL,
            auth text NOT NULL,
            created_at timestamp with time zone NOT NULL
        );

        CREATE TABLE mikrotik_routers (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            name text NOT NULL,
            host text NOT NULL,
            port integer NOT NULL DEFAULT 8728,
            username text NOT NULL,
            password text NOT NULL,
            use_tls boolean NOT NULL DEFAULT false,
            enabled boolean NOT NULL DEFAULT true,
            identity text,
            ros_version text,
            is_online boolean NOT NULL DEFAULT false,
            last_seen_at timestamp with time zone,
            latency_ms integer,
            last_error text,
            maintenance_until timestamp with time zone,
            maintenance_reason text,
            latitude double precision,
            longitude double precision,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL
        );

        CREATE TABLE pppoe_profiles (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            name text NOT NULL,
            rate_limit text,
            session_timeout_seconds integer,
            is_active boolean NOT NULL DEFAULT true,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL,
            UNIQUE (tenant_id, name)
        );

        CREATE TABLE isp_packages (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            service_type text NOT NULL DEFAULT 'internet_pppoe',
            provisioning_type text NOT NULL DEFAULT 'pppoe',
            name text NOT NULL,
            description text,
            features text[] NOT NULL DEFAULT '{}',
            is_active boolean NOT NULL DEFAULT true,
            price_monthly numeric(12,2) NOT NULL DEFAULT 0,
            price_yearly numeric(12,2) NOT NULL DEFAULT 0,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL,
            UNIQUE (tenant_id, name)
        );

        CREATE TABLE isp_package_router_mappings (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            router_id text NOT NULL REFERENCES mikrotik_routers(id) ON DELETE CASCADE,
            package_id text NOT NULL REFERENCES isp_packages(id) ON DELETE CASCADE,
            router_profile_name text NOT NULL,
            address_pool text,
            isolation_pool text,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL,
            UNIQUE (tenant_id, router_id, package_id)
        );

        CREATE TABLE customer_subscriptions (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            customer_id text NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
            location_id text NOT NULL REFERENCES customer_locations(id) ON DELETE CASCADE,
            package_id text NOT NULL REFERENCES isp_packages(id) ON DELETE RESTRICT,
            router_id text REFERENCES mikrotik_routers(id) ON DELETE SET NULL,
            billing_cycle text NOT NULL DEFAULT 'monthly',
            price numeric(12,2) NOT NULL,
            currency_code text NOT NULL DEFAULT 'IDR',
            status text NOT NULL DEFAULT 'active',
            starts_at timestamp with time zone,
            ends_at timestamp with time zone,
            grace_started_at timestamp with time zone,
            grace_until timestamp with time zone,
            notes text,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL,
            CHECK (billing_cycle IN ('monthly', 'yearly')),
            CHECK (
                status IN (
                    'active',
                    'grace_active',
                    'pending_installation',
                    'installation_done_awaiting_payment',
                    'suspended',
                    'cancelled'
                )
            )
        );

        CREATE TABLE invoices (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            invoice_number text NOT NULL,
            amount numeric(10,2) NOT NULL,
            currency_code text NOT NULL DEFAULT 'IDR',
            base_currency_code text NOT NULL DEFAULT 'IDR',
            fx_rate numeric(18,8),
            fx_source text,
            fx_fetched_at timestamp with time zone,
            status text NOT NULL DEFAULT 'pending',
            description text,
            due_date timestamp with time zone NOT NULL,
            paid_at timestamp with time zone,
            payment_method text,
            external_id text,
            merchant_id text,
            proof_attachment text,
            rejection_reason text,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL
        );

        CREATE TABLE installation_work_orders (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            subscription_id text NOT NULL REFERENCES customer_subscriptions(id) ON DELETE CASCADE,
            invoice_id text REFERENCES invoices(id) ON DELETE SET NULL,
            customer_id text NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
            location_id text NOT NULL REFERENCES customer_locations(id) ON DELETE CASCADE,
            router_id text REFERENCES mikrotik_routers(id) ON DELETE SET NULL,
            status text NOT NULL DEFAULT 'pending',
            assigned_to text REFERENCES users(id) ON DELETE SET NULL,
            scheduled_at timestamp with time zone,
            completed_at timestamp with time zone,
            notes text,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL
        );

        CREATE TABLE pppoe_accounts (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            router_id text NOT NULL REFERENCES mikrotik_routers(id) ON DELETE CASCADE,
            customer_id text NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
            location_id text NOT NULL REFERENCES customer_locations(id) ON DELETE CASCADE,
            username text NOT NULL,
            password_enc text NOT NULL,
            package_id text REFERENCES isp_packages(id) ON DELETE SET NULL,
            profile_id text REFERENCES pppoe_profiles(id) ON DELETE SET NULL,
            router_profile_name text,
            remote_address text,
            address_pool text,
            disabled boolean NOT NULL DEFAULT false,
            comment text,
            account_source text NOT NULL DEFAULT 'router',
            router_present boolean NOT NULL DEFAULT false,
            router_secret_id text,
            last_sync_at timestamp with time zone,
            last_error text,
            is_provisioned boolean NOT NULL DEFAULT false,
            radius_identity text,
            provisioned_at timestamp with time zone,
            provisioning_error text,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL,
            UNIQUE (tenant_id, router_id, username),
            CHECK (account_source IN ('router', 'managed_radius'))
        );

        CREATE TABLE invoice_reminder_logs (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            invoice_id text NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
            reminder_code text NOT NULL,
            channel text NOT NULL DEFAULT 'email',
            recipient text,
            status text NOT NULL DEFAULT 'sent',
            detail text,
            created_at timestamp with time zone NOT NULL
        );

        CREATE TABLE billing_collection_logs (
            id text PRIMARY KEY NOT NULL,
            tenant_id text NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            invoice_id text NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
            subscription_id text,
            action text NOT NULL,
            result text NOT NULL,
            reason text,
            actor_type text NOT NULL DEFAULT 'system',
            actor_id text,
            created_at timestamp with time zone NOT NULL
        );

        CREATE TABLE audit_logs (
            id uuid PRIMARY KEY NOT NULL,
            user_id uuid,
            tenant_id uuid,
            action text NOT NULL,
            resource text NOT NULL,
            resource_id text,
            details text,
            ip_address text,
            created_at timestamp with time zone NOT NULL
        );

        -- Mirrors production migration: invoice_number uniqueness scoped per-tenant
        -- + monotonic sequence used by PaymentService::create_invoice.
        CREATE UNIQUE INDEX idx_invoices_tenant_invoice_number
            ON invoices (tenant_id, invoice_number);
        CREATE SEQUENCE invoice_number_seq;
        "#,
    )
    .execute(pool)
    .await
    .expect("billing integration schema should be creatable");
}

#[tokio::test]
async fn background_billing_flow_generates_suspends_and_resumes_subscription() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;

    let generation = service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("invoice generation should succeed");
    assert_eq!(generation.created_count, 1);

    let (invoice_id, invoice_number) =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;

    let last_run: Option<String> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE tenant_id = $1 AND key = 'customer_invoice_last_run_at'",
    )
    .bind(&fixture.tenant_id)
    .fetch_optional(&fixture.pool)
    .await
    .expect("last run setting should query");
    assert!(last_run.is_some());

    sqlx::query("UPDATE invoices SET due_date = $1, updated_at = $2 WHERE id = $3")
        .bind(Utc::now() - Duration::days(1))
        .bind(Utc::now())
        .bind(&invoice_id)
        .execute(&fixture.pool)
        .await
        .expect("invoice due date should backdate");

    let collection = service
        .run_billing_collection_for_all_tenants()
        .await
        .expect("billing collection should succeed");
    assert_eq!(collection.evaluated_count, 1);
    assert_eq!(collection.suspended_count, 1);

    let subscription_status: String = sqlx::query_scalar(
        "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("subscription status should query");
    assert_eq!(subscription_status, "suspended");

    let suspend_logs: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM billing_collection_logs
        WHERE tenant_id = $1 AND invoice_id = $2 AND action = 'suspend' AND result = 'success'
        "#,
    )
    .bind(&fixture.tenant_id)
    .bind(&invoice_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("suspend logs should query");
    assert_eq!(suspend_logs, 1);

    service
        .process_midtrans_notification(&invoice_number, "paid", None, None)
        .await
        .expect("paid callback should succeed");

    let invoice_status: String = sqlx::query_scalar("SELECT status FROM invoices WHERE id = $1")
        .bind(&invoice_id)
        .fetch_one(&fixture.pool)
        .await
        .expect("invoice status should query");
    assert_eq!(invoice_status, "paid");

    let resumed_status: String = sqlx::query_scalar(
        "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("resumed subscription status should query");
    assert_eq!(resumed_status, "active");

    let callback_logs: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM billing_collection_logs
        WHERE tenant_id = $1 AND invoice_id = $2 AND action = 'payment_callback' AND result = 'success'
        "#,
    )
    .bind(&fixture.tenant_id)
    .bind(&invoice_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("callback logs should query");
    assert_eq!(callback_logs, 1);

    fixture.cleanup().await;
}

#[tokio::test]
async fn suspend_to_isolation_pool_restores_package_pool_after_payment() {
    let fixture = BillingFixture::new().await;
    fixture.create_pppoe_account("pool-main").await;
    sqlx::query(
        r#"
        UPDATE isp_package_router_mappings
        SET isolation_pool = 'pool-isolir-router-1', updated_at = $1
        WHERE tenant_id = $2 AND router_id = $3 AND package_id = $4
        "#,
    )
    .bind(Utc::now())
    .bind(&fixture.tenant_id)
    .bind(&fixture.router_id)
    .bind(&fixture.package_id)
    .execute(&fixture.pool)
    .await
    .expect("router mapping isolation pool should update");
    upsert_setting(
        &fixture.pool,
        Some(&fixture.tenant_id),
        "billing_auto_suspend_pppoe_action",
        "move_to_isolation_pool",
    )
    .await;
    upsert_setting(
        &fixture.pool,
        Some(&fixture.tenant_id),
        "billing_auto_suspend_isolation_pool",
        "pool-isolir-global",
    )
    .await;

    let service = fixture.service().await;
    service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("invoice generation should succeed");

    let (invoice_id, invoice_number) =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;
    sqlx::query("UPDATE invoices SET due_date = $1, updated_at = $2 WHERE id = $3")
        .bind(Utc::now() - Duration::days(1))
        .bind(Utc::now())
        .bind(&invoice_id)
        .execute(&fixture.pool)
        .await
        .expect("invoice due date should backdate");

    service
        .run_billing_collection_for_all_tenants()
        .await
        .expect("billing collection should succeed");

    let suspended_pool: Option<String> = sqlx::query_scalar(
        "SELECT address_pool FROM pppoe_accounts WHERE tenant_id = $1 AND id = 'pppoe-account-1'",
    )
    .bind(&fixture.tenant_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("pppoe account should query after suspend");
    assert_eq!(suspended_pool.as_deref(), Some("pool-isolir-router-1"));

    let suspended_disabled: bool = sqlx::query_scalar(
        "SELECT disabled FROM pppoe_accounts WHERE tenant_id = $1 AND id = 'pppoe-account-1'",
    )
    .bind(&fixture.tenant_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("pppoe disabled flag should query after suspend");
    assert!(!suspended_disabled);

    service
        .process_midtrans_notification(&invoice_number, "paid", None, None)
        .await
        .expect("paid callback should succeed");

    let restored_pool: Option<String> = sqlx::query_scalar(
        "SELECT address_pool FROM pppoe_accounts WHERE tenant_id = $1 AND id = 'pppoe-account-1'",
    )
    .bind(&fixture.tenant_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("pppoe account should query after resume");
    assert_eq!(restored_pool.as_deref(), Some("pool-main"));

    fixture.cleanup().await;
}

#[tokio::test]
async fn isolation_pool_suspend_without_mapping_falls_back_to_global_pool() {
    let fixture = BillingFixture::new().await;
    fixture.create_pppoe_account("pool-main").await;
    sqlx::query(
        r#"
        UPDATE isp_package_router_mappings
        SET isolation_pool = NULL, updated_at = $1
        WHERE tenant_id = $2 AND router_id = $3 AND package_id = $4
        "#,
    )
    .bind(Utc::now())
    .bind(&fixture.tenant_id)
    .bind(&fixture.router_id)
    .bind(&fixture.package_id)
    .execute(&fixture.pool)
    .await
    .expect("router mapping isolation pool should clear");
    upsert_setting(
        &fixture.pool,
        Some(&fixture.tenant_id),
        "billing_auto_suspend_pppoe_action",
        "move_to_isolation_pool",
    )
    .await;
    upsert_setting(
        &fixture.pool,
        Some(&fixture.tenant_id),
        "billing_auto_suspend_isolation_pool",
        "pool-isolir-global",
    )
    .await;

    let service = fixture.service().await;
    service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("invoice generation should succeed");

    let (invoice_id, _) =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;
    sqlx::query("UPDATE invoices SET due_date = $1, updated_at = $2 WHERE id = $3")
        .bind(Utc::now() - Duration::days(1))
        .bind(Utc::now())
        .bind(&invoice_id)
        .execute(&fixture.pool)
        .await
        .expect("invoice due date should backdate");

    service
        .run_billing_collection_for_all_tenants()
        .await
        .expect("billing collection should succeed");

    let address_pool: Option<String> = sqlx::query_scalar(
        "SELECT address_pool FROM pppoe_accounts WHERE tenant_id = $1 AND id = 'pppoe-account-1'",
    )
    .bind(&fixture.tenant_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("pppoe address pool should query after global fallback suspend");
    assert_eq!(address_pool.as_deref(), Some("pool-isolir-global"));

    fixture.cleanup().await;
}

#[tokio::test]
async fn fixed_day_suspend_waits_until_threshold_day() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;
    let today = Utc::now().date_naive();
    assert!(today.day() < 28, "test expects a day-of-month below 28");

    upsert_setting(
        &fixture.pool,
        Some(&fixture.tenant_id),
        "billing_auto_suspend_mode",
        "fixed_day",
    )
    .await;

    let future_fixed_day = (today.day() + 1) as i64;
    upsert_setting(
        &fixture.pool,
        Some(&fixture.tenant_id),
        "billing_auto_suspend_fixed_day",
        &future_fixed_day.to_string(),
    )
    .await;

    service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("invoice generation should succeed");
    let (invoice_id, _) =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;

    let due_before_threshold = today - Duration::days(1);
    let threshold = auto_suspend_threshold_date(
        due_before_threshold,
        AutoSuspendMode::FixedDay,
        0,
        future_fixed_day,
    );
    assert!(threshold > today, "threshold should still be in the future");

    sqlx::query("UPDATE invoices SET due_date = $1, updated_at = $2 WHERE id = $3")
        .bind(
            due_before_threshold
                .and_hms_opt(0, 0, 0)
                .expect("valid midnight"),
        )
        .bind(Utc::now())
        .bind(&invoice_id)
        .execute(&fixture.pool)
        .await
        .expect("invoice due date should update");

    let first_run = service
        .run_billing_collection_for_all_tenants()
        .await
        .expect("billing collection should succeed before threshold");
    assert_eq!(first_run.suspended_count, 0);

    let status_before: String = sqlx::query_scalar(
        "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("subscription status should query before threshold");
    assert_eq!(status_before, "active");

    upsert_setting(
        &fixture.pool,
        Some(&fixture.tenant_id),
        "billing_auto_suspend_fixed_day",
        &today.day().to_string(),
    )
    .await;

    let second_run = service
        .run_billing_collection_for_all_tenants()
        .await
        .expect("billing collection should succeed on threshold");
    assert_eq!(second_run.suspended_count, 1);

    let status_after: String = sqlx::query_scalar(
        "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("subscription status should query after threshold");
    assert_eq!(status_after, "suspended");

    fixture.cleanup().await;
}

#[tokio::test]
async fn reminder_run_is_logged_once_per_schedule_code() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;
    let now = Utc::now();

    upsert_setting(
        &fixture.pool,
        Some(&fixture.tenant_id),
        "billing_reminder_enabled",
        "true",
    )
    .await;
    upsert_setting(
        &fixture.pool,
        Some(&fixture.tenant_id),
        "billing_reminder_schedule",
        "H-3",
    )
    .await;
    upsert_setting(
        &fixture.pool,
        Some(&fixture.tenant_id),
        "billing_auto_suspend_grace_days",
        "30",
    )
    .await;

    service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("invoice generation should succeed");
    let (invoice_id, _) =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;

    sqlx::query("UPDATE invoices SET due_date = $1, updated_at = $2 WHERE id = $3")
        .bind(now + Duration::days(3))
        .bind(now)
        .bind(&invoice_id)
        .execute(&fixture.pool)
        .await
        .expect("invoice due date should move to reminder day");

    let first_run = service
        .run_billing_collection_for_all_tenants()
        .await
        .expect("first billing collection should succeed");
    assert_eq!(first_run.reminder_sent_count, 1);
    assert_eq!(first_run.reminder_skipped_count, 0);

    let second_run = service
        .run_billing_collection_for_all_tenants()
        .await
        .expect("second billing collection should succeed");
    assert_eq!(second_run.reminder_sent_count, 0);
    assert_eq!(second_run.reminder_skipped_count, 1);

    let reminder_log_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM invoice_reminder_logs
        WHERE tenant_id = $1
          AND invoice_id = $2
          AND reminder_code = 'H-3'
          AND status = 'sent'
        "#,
    )
    .bind(&fixture.tenant_id)
    .bind(&invoice_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("reminder sent logs should query");
    assert_eq!(reminder_log_count, 1);

    let skipped_collection_logs: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM billing_collection_logs
        WHERE tenant_id = $1
          AND invoice_id = $2
          AND action = 'reminder'
          AND result = 'skipped'
          AND reason = 'Reminder already sent for this code'
        "#,
    )
    .bind(&fixture.tenant_id)
    .bind(&invoice_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("duplicate reminder skip logs should query");
    assert_eq!(skipped_collection_logs, 1);

    fixture.cleanup().await;
}

#[tokio::test]
async fn global_runner_respects_per_tenant_billing_settings() {
    let fixture = BillingFixture::new().await;
    let second = seed_tenant_fixture(
        &fixture.pool,
        "tenant-second",
        "Tenant Second",
        "tenant-second",
        true,
    )
    .await;

    upsert_setting(
        &fixture.pool,
        Some(&second.tenant_id),
        "customer_invoice_auto_generate_enabled",
        "false",
    )
    .await;
    upsert_setting(
        &fixture.pool,
        Some(&second.tenant_id),
        "billing_auto_suspend_enabled",
        "false",
    )
    .await;
    upsert_setting(
        &fixture.pool,
        Some(&second.tenant_id),
        "billing_reminder_enabled",
        "false",
    )
    .await;

    let service = fixture.service().await;

    let generation = service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("global invoice generation should succeed");
    assert_eq!(generation.created_count, 1);

    let first_invoice =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;
    let second_invoice_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM invoices WHERE tenant_id = $1 AND external_id LIKE ($2 || '%')",
    )
    .bind(&second.tenant_id)
    .bind(format!("pkgsub:{}:", second.subscription_id))
    .fetch_one(&fixture.pool)
    .await
    .expect("second tenant invoice count should query");
    assert_eq!(second_invoice_count, 0);

    sqlx::query("UPDATE invoices SET due_date = $1, updated_at = $2 WHERE id = $3")
        .bind(
            (Utc::now() - Duration::days(1))
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .expect("midnight"),
        )
        .bind(Utc::now())
        .bind(&first_invoice.0)
        .execute(&fixture.pool)
        .await
        .expect("first tenant invoice should backdate");

    let manual_invoice_id = "invoice-second-manual";
    sqlx::query(
        r#"
        INSERT INTO invoices (
          id, tenant_id, invoice_number, amount, currency_code, base_currency_code, status,
          description, due_date, external_id, created_at, updated_at
        )
        VALUES ($1, $2, $3, 150000, 'IDR', 'IDR', 'pending', 'Manual overdue', $4, $5, $6, $6)
        "#,
    )
    .bind(manual_invoice_id)
    .bind(&second.tenant_id)
    .bind("INV-SECOND-MANUAL")
    .bind(
        (Utc::now() - Duration::days(5))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("midnight"),
    )
    .bind(format!("pkgsub:{}:manual", second.subscription_id))
    .bind(Utc::now())
    .execute(&fixture.pool)
    .await
    .expect("manual second tenant invoice should seed");

    let collection = service
        .run_billing_collection_for_all_tenants()
        .await
        .expect("global billing collection should succeed");
    assert_eq!(collection.suspended_count, 1);

    let first_status: String = sqlx::query_scalar(
        "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("first tenant status should query");
    assert_eq!(first_status, "suspended");

    let second_status: String = sqlx::query_scalar(
        "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&second.tenant_id)
    .bind(&second.subscription_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("second tenant status should query");
    assert_eq!(second_status, "active");

    let second_logs: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM billing_collection_logs WHERE tenant_id = $1 AND invoice_id = $2 AND action = 'suspend'",
    )
    .bind(&second.tenant_id)
    .bind(manual_invoice_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("second tenant suspend logs should query");
    assert_eq!(second_logs, 0);

    fixture.cleanup().await;
}

#[tokio::test]
async fn paid_pending_installation_stays_pending_and_creates_work_order() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;
    let now = Utc::now();

    sqlx::query(
        "DELETE FROM installation_work_orders WHERE tenant_id = $1 AND subscription_id = $2",
    )
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .execute(&fixture.pool)
    .await
    .expect("existing installation work orders should be removable for setup");

    sqlx::query(
        r#"
        UPDATE customer_subscriptions
        SET status = 'pending_installation',
            grace_started_at = NULL,
            grace_until = NULL,
            updated_at = $1
        WHERE tenant_id = $2 AND id = $3
        "#,
    )
    .bind(now)
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .execute(&fixture.pool)
    .await
    .expect("subscription should switch to pending_installation");

    let invoice_id = "invoice-pending-install";
    let invoice_number = "INV-PENDING-INSTALL";
    sqlx::query(
        r#"
        INSERT INTO invoices (
          id, tenant_id, invoice_number, amount, currency_code, base_currency_code, status,
          description, due_date, external_id, created_at, updated_at
        )
        VALUES ($1, $2, $3, 150000, 'IDR', 'IDR', 'pending', 'Pending installation invoice', $4, $5, $6, $6)
        "#,
    )
    .bind(invoice_id)
    .bind(&fixture.tenant_id)
    .bind(invoice_number)
    .bind(now)
    .bind(format!("pkgsub:{}:pending-install", fixture.subscription_id))
    .bind(now)
    .execute(&fixture.pool)
    .await
    .expect("pending-install invoice should seed");

    service
        .process_midtrans_notification(invoice_number, "paid", None, None)
        .await
        .expect("paid callback for pending installation should succeed");

    let subscription_status: String = sqlx::query_scalar(
        "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("subscription status should query");
    assert_eq!(subscription_status, "pending_installation");

    let invoice_status: String = sqlx::query_scalar("SELECT status FROM invoices WHERE id = $1")
        .bind(invoice_id)
        .fetch_one(&fixture.pool)
        .await
        .expect("invoice status should query");
    assert_eq!(invoice_status, "paid");

    let pending_work_orders: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM installation_work_orders
        WHERE tenant_id = $1
          AND subscription_id = $2
          AND invoice_id = $3
          AND status = 'pending'
        "#,
    )
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .bind(invoice_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("pending work order count should query");
    assert_eq!(pending_work_orders, 1);

    let installation_logs: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM billing_collection_logs
        WHERE tenant_id = $1
          AND invoice_id = $2
          AND action = 'installation'
          AND result = 'success'
        "#,
    )
    .bind(&fixture.tenant_id)
    .bind(invoice_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("installation logs should query");
    assert_eq!(installation_logs, 1);

    fixture.cleanup().await;
}

#[tokio::test]
async fn paid_installation_done_awaiting_payment_activates_subscription() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;
    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE customer_subscriptions
        SET status = 'installation_done_awaiting_payment',
            grace_started_at = $1,
            grace_until = $2,
            updated_at = $1
        WHERE tenant_id = $3 AND id = $4
        "#,
    )
    .bind(now - Duration::days(2))
    .bind(now + Duration::days(3))
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .execute(&fixture.pool)
    .await
    .expect("subscription should switch to installation_done_awaiting_payment");

    let invoice_id = "invoice-install-done";
    let invoice_number = "INV-INSTALL-DONE";
    sqlx::query(
        r#"
        INSERT INTO invoices (
          id, tenant_id, invoice_number, amount, currency_code, base_currency_code, status,
          description, due_date, external_id, created_at, updated_at
        )
        VALUES ($1, $2, $3, 150000, 'IDR', 'IDR', 'pending', 'Install done awaiting payment', $4, $5, $6, $6)
        "#,
    )
    .bind(invoice_id)
    .bind(&fixture.tenant_id)
    .bind(invoice_number)
    .bind(now)
    .bind(format!("pkgsub:{}:install-done", fixture.subscription_id))
    .bind(now)
    .execute(&fixture.pool)
    .await
    .expect("install-done invoice should seed");

    service
        .process_midtrans_notification(invoice_number, "paid", None, None)
        .await
        .expect("paid callback for installation_done_awaiting_payment should succeed");

    let subscription_status: String = sqlx::query_scalar(
        "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("subscription status should query");
    assert_eq!(subscription_status, "active");

    let grace_until: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT grace_until FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("grace_until should query");
    assert!(grace_until.is_none());

    let resume_logs: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM billing_collection_logs
        WHERE tenant_id = $1
          AND invoice_id = $2
          AND action = 'resume'
          AND result = 'success'
        "#,
    )
    .bind(&fixture.tenant_id)
    .bind(invoice_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("resume logs should query");
    assert_eq!(resume_logs, 1);

    fixture.cleanup().await;
}

#[tokio::test]
async fn paid_suspended_subscription_stays_suspended_when_auto_resume_disabled() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;

    upsert_setting(
        &fixture.pool,
        Some(&fixture.tenant_id),
        "billing_auto_resume_on_payment",
        "false",
    )
    .await;

    service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("invoice generation should succeed");

    let (invoice_id, invoice_number) =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;

    sqlx::query("UPDATE invoices SET due_date = $1, updated_at = $2 WHERE id = $3")
        .bind(Utc::now() - Duration::days(1))
        .bind(Utc::now())
        .bind(&invoice_id)
        .execute(&fixture.pool)
        .await
        .expect("invoice due date should backdate");

    let collection = service
        .run_billing_collection_for_all_tenants()
        .await
        .expect("billing collection should succeed");
    assert_eq!(collection.suspended_count, 1);

    service
        .process_midtrans_notification(&invoice_number, "paid", None, None)
        .await
        .expect("paid callback should succeed");

    let subscription_status: String = sqlx::query_scalar(
        "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("subscription status should query");
    assert_eq!(subscription_status, "suspended");

    let resume_success_logs: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM billing_collection_logs
        WHERE tenant_id = $1
          AND invoice_id = $2
          AND action = 'resume'
          AND result = 'success'
        "#,
    )
    .bind(&fixture.tenant_id)
    .bind(&invoice_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("resume success logs should query");
    assert_eq!(resume_success_logs, 0);

    let resume_skipped_logs: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM billing_collection_logs
        WHERE tenant_id = $1
          AND invoice_id = $2
          AND action = 'resume'
          AND result = 'skipped'
          AND reason = 'Auto resume disabled by billing setting'
        "#,
    )
    .bind(&fixture.tenant_id)
    .bind(&invoice_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("resume skipped logs should query");
    assert_eq!(resume_skipped_logs, 1);

    fixture.cleanup().await;
}

#[tokio::test]
async fn expired_grace_active_without_payment_gets_suspended() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;
    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE customer_subscriptions
        SET status = 'grace_active',
            grace_started_at = $1,
            grace_until = $2,
            updated_at = $1
        WHERE tenant_id = $3 AND id = $4
        "#,
    )
    .bind(now - Duration::days(5))
    .bind(now - Duration::hours(2))
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .execute(&fixture.pool)
    .await
    .expect("subscription should switch to grace_active");

    let invoice_id = "invoice-grace-expired";
    sqlx::query(
        r#"
        INSERT INTO invoices (
          id, tenant_id, invoice_number, amount, currency_code, base_currency_code, status,
          description, due_date, external_id, created_at, updated_at
        )
        VALUES ($1, $2, 'INV-GRACE-EXPIRED', 150000, 'IDR', 'IDR', 'pending', 'Grace expired unpaid', $3, $4, $5, $5)
        "#,
    )
    .bind(invoice_id)
    .bind(&fixture.tenant_id)
    .bind(now - Duration::days(5))
    .bind(format!("pkgsub:{}:grace-expired", fixture.subscription_id))
    .bind(now)
    .execute(&fixture.pool)
    .await
    .expect("grace invoice should seed");

    let run = service
        .run_billing_collection_for_all_tenants()
        .await
        .expect("billing collection should handle expired grace");
    assert_eq!(run.suspended_count, 1);

    let subscription_status: String = sqlx::query_scalar(
        "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("subscription status should query");
    assert_eq!(subscription_status, "suspended");

    let grace_logs: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM billing_collection_logs
        WHERE tenant_id = $1
          AND invoice_id = $2
          AND action = 'grace_expire_suspend'
          AND result = 'success'
        "#,
    )
    .bind(&fixture.tenant_id)
    .bind(invoice_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("grace expire logs should query");
    assert_eq!(grace_logs, 1);

    fixture.cleanup().await;
}

#[tokio::test]
async fn process_midtrans_notification_writes_audit_log_for_status_change() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;

    service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("invoice generation should succeed");

    let (invoice_id, invoice_number) =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;

    service
        .process_midtrans_notification(&invoice_number, "paid", Some("req-123"), Some("cb-456"))
        .await
        .expect("paid callback should succeed");

    let logs = fetch_audit_logs_by_action(&fixture.pool, "invoice.status_changed").await;
    assert_eq!(
        logs.len(),
        1,
        "expected exactly one invoice.status_changed audit entry, found {}",
        logs.len()
    );

    let entry = &logs[0];
    assert_eq!(entry.resource, "invoice");
    assert_eq!(entry.resource_id.as_deref(), Some(invoice_id.as_str()));

    let details = entry
        .details
        .as_ref()
        .expect("audit entry should include JSON details");
    let parsed: serde_json::Value =
        serde_json::from_str(details).expect("details should be JSON object");

    assert_eq!(
        parsed.get("gateway").and_then(|v| v.as_str()),
        Some("midtrans")
    );
    assert_eq!(
        parsed.get("old_status").and_then(|v| v.as_str()),
        Some("pending")
    );
    assert_eq!(
        parsed.get("new_status").and_then(|v| v.as_str()),
        Some("paid")
    );
    assert_eq!(
        parsed.get("invoice_number").and_then(|v| v.as_str()),
        Some(invoice_number.as_str())
    );

    fixture.cleanup().await;
}

#[tokio::test]
async fn verify_payment_approve_writes_audit_log() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;

    service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("invoice generation should succeed");
    let (invoice_id, _invoice_number) =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;

    // Move invoice to verification_pending so manual verification is meaningful.
    service
        .submit_payment_proof(&invoice_id, "/tmp/proof.png")
        .await
        .expect("submit payment proof should succeed");

    service
        .verify_payment(&invoice_id, "paid", None)
        .await
        .expect("approve should succeed");

    let logs = fetch_audit_logs_by_action(&fixture.pool, "invoice.verified").await;
    assert_eq!(
        logs.len(),
        1,
        "expected exactly one invoice.verified audit entry, found {}",
        logs.len()
    );
    let entry = &logs[0];
    assert_eq!(entry.resource, "invoice");
    assert_eq!(entry.resource_id.as_deref(), Some(invoice_id.as_str()));
    let parsed: serde_json::Value =
        serde_json::from_str(entry.details.as_deref().unwrap_or("null"))
            .expect("details should be JSON object");
    assert_eq!(parsed.get("status").and_then(|v| v.as_str()), Some("paid"));

    fixture.cleanup().await;
}

#[tokio::test]
async fn verify_payment_reject_writes_audit_log() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;

    service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("invoice generation should succeed");
    let (invoice_id, _invoice_number) =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;

    service
        .submit_payment_proof(&invoice_id, "/tmp/proof.png")
        .await
        .expect("submit payment proof should succeed");

    service
        .verify_payment(&invoice_id, "failed", Some("Bukti tidak jelas".to_string()))
        .await
        .expect("reject should succeed");

    let logs = fetch_audit_logs_by_action(&fixture.pool, "invoice.rejected").await;
    assert_eq!(
        logs.len(),
        1,
        "expected exactly one invoice.rejected audit entry, found {}",
        logs.len()
    );
    let entry = &logs[0];
    assert_eq!(entry.resource_id.as_deref(), Some(invoice_id.as_str()));
    let parsed: serde_json::Value =
        serde_json::from_str(entry.details.as_deref().unwrap_or("null"))
            .expect("details should be JSON object");
    assert_eq!(
        parsed.get("rejection_reason").and_then(|v| v.as_str()),
        Some("Bukti tidak jelas")
    );

    fixture.cleanup().await;
}

#[tokio::test]
async fn submit_payment_proof_writes_audit_log() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;

    service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("invoice generation should succeed");
    let (invoice_id, _invoice_number) =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;

    service
        .submit_payment_proof(&invoice_id, "/storage/proofs/abc.jpg")
        .await
        .expect("submit_payment_proof should succeed");

    let logs = fetch_audit_logs_by_action(&fixture.pool, "invoice.payment_proof_uploaded").await;
    assert_eq!(
        logs.len(),
        1,
        "expected exactly one invoice.payment_proof_uploaded audit entry, found {}",
        logs.len()
    );
    let entry = &logs[0];
    assert_eq!(entry.resource, "invoice");
    assert_eq!(entry.resource_id.as_deref(), Some(invoice_id.as_str()));
    let parsed: serde_json::Value =
        serde_json::from_str(entry.details.as_deref().unwrap_or("null"))
            .expect("details should be JSON object");
    assert_eq!(
        parsed.get("file_path").and_then(|v| v.as_str()),
        Some("/storage/proofs/abc.jpg")
    );

    fixture.cleanup().await;
}

#[tokio::test]
async fn billing_collection_run_writes_summary_audit_log() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;

    service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("invoice generation should succeed");

    let (invoice_id, _) =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;
    sqlx::query("UPDATE invoices SET due_date = $1, updated_at = $2 WHERE id = $3")
        .bind(Utc::now() - Duration::days(1))
        .bind(Utc::now())
        .bind(&invoice_id)
        .execute(&fixture.pool)
        .await
        .expect("invoice due date should backdate");

    service
        .run_billing_collection_for_all_tenants()
        .await
        .expect("billing collection should succeed");

    let logs = fetch_audit_logs_by_action(&fixture.pool, "billing.collection_run").await;
    assert!(
        !logs.is_empty(),
        "expected at least one billing.collection_run audit entry, found 0"
    );
    let entry = &logs[0];
    assert_eq!(entry.resource, "billing");
    let parsed: serde_json::Value =
        serde_json::from_str(entry.details.as_deref().unwrap_or("null"))
            .expect("details should be JSON object");
    // Suspended_count should reflect at least one auto-suspend in this scenario.
    let suspended = parsed
        .get("suspended_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(
        suspended >= 1,
        "expected suspended_count>=1 in billing.collection_run audit details, got {}",
        suspended
    );

    fixture.cleanup().await;
}

#[tokio::test]
async fn auto_suspend_writes_subscription_audit_log() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;

    service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("invoice generation should succeed");
    let (invoice_id, _) =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;
    sqlx::query("UPDATE invoices SET due_date = $1, updated_at = $2 WHERE id = $3")
        .bind(Utc::now() - Duration::days(1))
        .bind(Utc::now())
        .bind(&invoice_id)
        .execute(&fixture.pool)
        .await
        .expect("invoice due date should backdate");

    service
        .run_billing_collection_for_all_tenants()
        .await
        .expect("billing collection should succeed");

    let logs = fetch_audit_logs_by_action(&fixture.pool, "subscription.auto_suspended").await;
    assert_eq!(
        logs.len(),
        1,
        "expected exactly one subscription.auto_suspended audit entry, found {}",
        logs.len()
    );
    let entry = &logs[0];
    assert_eq!(entry.resource, "subscription");
    assert_eq!(
        entry.resource_id.as_deref(),
        Some(fixture.subscription_id.as_str())
    );
    let parsed: serde_json::Value =
        serde_json::from_str(entry.details.as_deref().unwrap_or("null"))
            .expect("details should be JSON object");
    assert!(
        parsed.get("reason").and_then(|v| v.as_str()).is_some(),
        "expected reason field in subscription.auto_suspended details"
    );

    fixture.cleanup().await;
}

#[tokio::test]
async fn auto_resume_writes_subscription_audit_log() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;

    service
        .generate_due_customer_package_invoices_for_all_tenants()
        .await
        .expect("invoice generation should succeed");
    let (invoice_id, invoice_number) =
        latest_customer_invoice(&fixture.pool, &fixture.tenant_id, &fixture.subscription_id).await;

    sqlx::query("UPDATE invoices SET due_date = $1, updated_at = $2 WHERE id = $3")
        .bind(Utc::now() - Duration::days(1))
        .bind(Utc::now())
        .bind(&invoice_id)
        .execute(&fixture.pool)
        .await
        .expect("invoice due date should backdate");

    service
        .run_billing_collection_for_all_tenants()
        .await
        .expect("billing collection should succeed");

    // Confirm subscription is suspended before paying.
    let suspended_status: String = sqlx::query_scalar(
        "SELECT status FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
    )
    .bind(&fixture.tenant_id)
    .bind(&fixture.subscription_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("subscription status should query");
    assert_eq!(suspended_status, "suspended");

    service
        .process_midtrans_notification(&invoice_number, "paid", None, None)
        .await
        .expect("paid callback should succeed");

    let logs = fetch_audit_logs_by_action(&fixture.pool, "subscription.auto_resumed").await;
    assert_eq!(
        logs.len(),
        1,
        "expected exactly one subscription.auto_resumed audit entry, found {}",
        logs.len()
    );
    let entry = &logs[0];
    assert_eq!(entry.resource, "subscription");
    assert_eq!(
        entry.resource_id.as_deref(),
        Some(fixture.subscription_id.as_str())
    );
    let parsed: serde_json::Value =
        serde_json::from_str(entry.details.as_deref().unwrap_or("null"))
            .expect("details should be JSON object");
    assert_eq!(
        parsed.get("triggering_invoice_id").and_then(|v| v.as_str()),
        Some(invoice_id.as_str())
    );

    fixture.cleanup().await;
}

// ==================== Invoice number uniqueness / sequence ====================
//
// Regression coverage for HIGH #3 (MVP DoD audit):
// `create_invoice` previously used `INV-{YYYYMMDD-HHMMSS}` granularity, which
// collides under concurrent invocation (scheduler + manual create at same
// second). The format is migrating to `INV-{YYYYMMDD}-{NNNNNN}` driven by
// the Postgres sequence `invoice_number_seq`, with composite uniqueness
// `(tenant_id, invoice_number)` as a structural safety net.

/// Validates the `INV-YYYYMMDD-NNNNNN` invoice number format without pulling
/// in a regex dependency.
fn matches_invoice_number_format(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 19 {
        return false;
    }
    if &bytes[..4] != b"INV-" || bytes[12] != b'-' {
        return false;
    }
    bytes[4..12].iter().all(|c| c.is_ascii_digit())
        && bytes[13..].iter().all(|c| c.is_ascii_digit())
}

fn invoice_number_seq_value(invoice_number: &str) -> i64 {
    let suffix = invoice_number
        .rsplit('-')
        .next()
        .expect("invoice number should contain '-'");
    suffix
        .parse()
        .expect("invoice number suffix should be numeric")
}

#[tokio::test]
async fn create_invoice_uses_sequence_format() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;

    let invoice = service
        .create_invoice(
            &fixture.tenant_id,
            123_456.0,
            Some("format check".to_string()),
            None,
        )
        .await
        .expect("create_invoice should succeed");

    assert!(
        matches_invoice_number_format(&invoice.invoice_number),
        "invoice_number {:?} should match INV-YYYYMMDD-NNNNNN",
        invoice.invoice_number
    );

    // Distinguish the new sequence-driven format from the old HHMMSS format:
    // the production sequence MUST have been consumed by create_invoice.
    let last_value: Option<i64> =
        sqlx::query_scalar("SELECT pg_sequence_last_value('invoice_number_seq')")
            .fetch_one(&fixture.pool)
            .await
            .expect("pg_sequence_last_value query should succeed");
    assert!(
        last_value.is_some(),
        "create_invoice should consume invoice_number_seq, got {last_value:?}"
    );
    let parsed_seq = invoice_number_seq_value(&invoice.invoice_number);
    assert_eq!(
        last_value,
        Some(parsed_seq),
        "invoice_number suffix should match the consumed sequence value"
    );

    fixture.cleanup().await;
}

#[tokio::test]
async fn create_invoice_sequence_is_monotonic() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;

    let mut seqs = Vec::new();
    for i in 0..3 {
        let inv = service
            .create_invoice(
                &fixture.tenant_id,
                100.0 + i as f64,
                Some(format!("monotonic-{i}")),
                None,
            )
            .await
            .expect("create_invoice should succeed");
        seqs.push(invoice_number_seq_value(&inv.invoice_number));
    }

    assert_eq!(seqs.len(), 3);
    assert!(
        seqs[1] > seqs[0] && seqs[2] > seqs[1],
        "sequence should be strictly increasing, got {seqs:?}"
    );

    fixture.cleanup().await;
}

#[tokio::test]
async fn concurrent_create_invoice_yields_unique_numbers() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;

    let mut handles = Vec::with_capacity(5);
    for i in 0..5 {
        let svc = service.clone();
        let tenant = fixture.tenant_id.clone();
        handles.push(tokio::spawn(async move {
            svc.create_invoice(
                &tenant,
                1_000.0 + i as f64,
                Some(format!("burst-{i}")),
                None,
            )
            .await
        }));
    }

    let mut numbers = Vec::with_capacity(5);
    for h in handles {
        let inv = h
            .await
            .expect("task should not panic")
            .expect("create_invoice should succeed under burst");
        numbers.push(inv.invoice_number);
    }

    for n in &numbers {
        assert!(
            matches_invoice_number_format(n),
            "invoice_number {n:?} should match expected format"
        );
    }

    let mut sorted = numbers.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        numbers.len(),
        "expected 5 unique invoice numbers, got duplicates in {numbers:?}"
    );

    fixture.cleanup().await;
}

#[tokio::test]
async fn create_invoice_retries_on_unique_conflict() {
    let fixture = BillingFixture::new().await;
    let service = fixture.service().await;

    // Pre-claim the next sequence value and pre-insert a row that owns the
    // resulting invoice_number for this tenant. We then rewind the sequence
    // with `is_called = false` so the next nextval returns the SAME value
    // again, guaranteeing the first INSERT inside create_invoice collides
    // on the (tenant_id, invoice_number) unique index. A correct
    // implementation must retry with a fresh sequence value and succeed.
    let claimed_seq: i64 = sqlx::query_scalar("SELECT nextval('invoice_number_seq')")
        .fetch_one(&fixture.pool)
        .await
        .expect("nextval should succeed");
    let now = Utc::now();
    let claimed_number = format!("INV-{}-{:06}", now.format("%Y%m%d"), claimed_seq);

    sqlx::query(
        r#"
        INSERT INTO invoices (
            id, tenant_id, invoice_number, amount, currency_code, base_currency_code,
            status, due_date, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, 'IDR', 'IDR', 'pending', $5, $6, $6)
        "#,
    )
    .bind(format!("invoice-conflict-{}", Uuid::new_v4()))
    .bind(&fixture.tenant_id)
    .bind(&claimed_number)
    .bind(50.0_f64)
    .bind(now + Duration::days(1))
    .bind(now)
    .execute(&fixture.pool)
    .await
    .expect("conflict-blocker invoice should insert");

    // is_called=false → next nextval returns claimed_seq again, then advances.
    sqlx::query(&format!(
        "SELECT setval('invoice_number_seq', {claimed_seq}, false)"
    ))
    .execute(&fixture.pool)
    .await
    .expect("setval should succeed");

    let invoice = service
        .create_invoice(
            &fixture.tenant_id,
            999.0,
            Some("retry path".to_string()),
            None,
        )
        .await
        .expect("create_invoice should retry past unique conflict");

    assert!(
        matches_invoice_number_format(&invoice.invoice_number),
        "invoice_number {:?} should match expected format",
        invoice.invoice_number
    );
    assert_ne!(
        invoice.invoice_number, claimed_number,
        "create_invoice must not return the conflicting number"
    );
    let new_seq = invoice_number_seq_value(&invoice.invoice_number);
    assert!(
        new_seq > claimed_seq,
        "retried invoice should consume a later sequence value (claimed {claimed_seq}, got {new_seq})"
    );

    // Confirm the sequence was actually advanced by create_invoice's retry
    // path (>= 2 nextval calls beyond claimed_seq). With a non-retrying or
    // non-sequence-based implementation, last_value stays at claimed_seq.
    let last_value: Option<i64> =
        sqlx::query_scalar("SELECT pg_sequence_last_value('invoice_number_seq')")
            .fetch_one(&fixture.pool)
            .await
            .expect("pg_sequence_last_value query should succeed");
    assert!(
        last_value.is_some_and(|v| v > claimed_seq),
        "sequence should advance past claimed value via retry, got {last_value:?} (claimed {claimed_seq})"
    );

    fixture.cleanup().await;
}
