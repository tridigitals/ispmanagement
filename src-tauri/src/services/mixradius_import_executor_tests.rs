#[cfg(all(test, feature = "postgres"))]
mod mixradius_import_executor_tests {
    use crate::models::{MixradiusImportMappingOverride, MixradiusImportPppoeProvisioningTarget};
    use crate::security::secret::decrypt_secret_for;
    use crate::services::mixradius_import_executor::MixradiusImportExecutor;
    use sqlx::Row;
    use uuid::Uuid;

    const TEST_ADMIN_DATABASE_URL: &str = "postgres://postgres:postgres@127.0.0.1/postgres";
    const MIXRADIUS_IMPORT_FOUNDATION_UP_SQL: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migrations/20260411120000_add_mixradius_import_foundation.up.sql"
    );

    async fn isolated_pool() -> (sqlx::PgPool, String) {
        let db_name = format!("mixradius_import_executor_{}", Uuid::new_v4().simple());
        let admin_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(TEST_ADMIN_DATABASE_URL)
            .await
            .expect("postgres admin database should be available for executor tests");

        sqlx::query(&format!(r#"CREATE DATABASE "{}""#, db_name))
            .execute(&admin_pool)
            .await
            .expect("temporary executor test database should be creatable");
        admin_pool.close().await;

        let database_url = format!("postgres://postgres:postgres@127.0.0.1/{db_name}");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("temporary executor test database should be connectable");

        sqlx::raw_sql(
            r#"
            CREATE TABLE public.tenants (
                id text PRIMARY KEY NOT NULL
            );

            CREATE TABLE public.users (
                id text PRIMARY KEY NOT NULL
            );
            "#,
        )
        .execute(&pool)
        .await
        .expect("minimal dependency tables should be creatable");

        let up_sql = std::fs::read_to_string(MIXRADIUS_IMPORT_FOUNDATION_UP_SQL)
            .expect("mixradius import up migration should be readable");
        sqlx::raw_sql(&up_sql)
            .execute(&pool)
            .await
            .expect("mixradius import migration should apply for executor tests");

        // Invoice numbering lives in its own migration
        // (20260529082913_invoice_number_uniqueness), which is not applied here
        // — this suite only applies the mixradius foundation. The executor
        // bootstraps a first invoice, so create the sequence explicitly to keep
        // the test independent of migration ordering.
        sqlx::raw_sql("CREATE SEQUENCE IF NOT EXISTS invoice_number_seq;")
            .execute(&pool)
            .await
            .expect("invoice number sequence should be creatable");

        (pool, db_name)
    }

    async fn drop_test_database(pool: sqlx::PgPool, db_name: &str) {
        pool.close().await;

        let admin_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(TEST_ADMIN_DATABASE_URL)
            .await
            .expect("postgres admin database should be available for cleanup");

        sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = $1")
            .bind(db_name)
            .execute(&admin_pool)
            .await
            .expect("temporary database connections should be terminable");

        sqlx::query(&format!(r#"DROP DATABASE IF EXISTS "{}""#, db_name))
            .execute(&admin_pool)
            .await
            .expect("temporary executor test database should be droppable");

        admin_pool.close().await;
    }

    async fn seed_test_tenant(pool: &sqlx::PgPool, tenant_id: &str) {
        sqlx::query("INSERT INTO public.tenants (id) VALUES ($1)")
            .bind(tenant_id)
            .execute(pool)
            .await
            .expect("test tenant should be insertable");
    }

    async fn create_package_table(pool: &sqlx::PgPool) {
        sqlx::raw_sql(
            r#"
            CREATE TABLE public.isp_packages (
                id text PRIMARY KEY NOT NULL,
                tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
                service_type text NOT NULL DEFAULT 'internet_pppoe',
                name text NOT NULL,
                description text,
                features text[] NOT NULL DEFAULT '{}',
                is_active boolean NOT NULL DEFAULT true,
                price_monthly numeric(12,2) NOT NULL DEFAULT 0,
                price_yearly numeric(12,2) NOT NULL DEFAULT 0,
                created_at timestamp with time zone NOT NULL,
                updated_at timestamp with time zone NOT NULL,
                CONSTRAINT isp_packages_tenant_name_unique UNIQUE (tenant_id, name)
            );

            CREATE INDEX idx_isp_packages_tenant_name
                ON public.isp_packages (tenant_id, name);
            "#,
        )
        .execute(pool)
        .await
        .expect("isp package table should be creatable for executor tests");
    }

    async fn create_customer_tables(pool: &sqlx::PgPool) {
        sqlx::raw_sql(
            r#"
            CREATE TABLE public.customers (
                id text PRIMARY KEY NOT NULL,
                tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
                name text NOT NULL,
                email text,
                phone text,
                notes text,
                is_active boolean NOT NULL DEFAULT true,
                created_at timestamp with time zone NOT NULL,
                updated_at timestamp with time zone NOT NULL
            );

            CREATE INDEX idx_customers_tenant_name
                ON public.customers (tenant_id, name);

            CREATE TABLE public.customer_locations (
                id text PRIMARY KEY NOT NULL,
                tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
                customer_id text NOT NULL REFERENCES public.customers(id) ON DELETE CASCADE,
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
            "#,
        )
        .execute(pool)
        .await
        .expect("customer tables should be creatable for executor tests");
    }

    async fn create_subscription_table(pool: &sqlx::PgPool) {
        sqlx::raw_sql(
            r#"
            CREATE TABLE public.customer_subscriptions (
                id text PRIMARY KEY NOT NULL,
                tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
                customer_id text NOT NULL REFERENCES public.customers(id) ON DELETE CASCADE,
                location_id text NOT NULL REFERENCES public.customer_locations(id) ON DELETE CASCADE,
                package_id text NOT NULL REFERENCES public.isp_packages(id) ON DELETE RESTRICT,
                router_id text,
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
                CONSTRAINT customer_subscriptions_status_check CHECK (
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

            CREATE TABLE public.invoices (
                id text PRIMARY KEY NOT NULL,
                tenant_id text NOT NULL,
                invoice_number text NOT NULL,
                amount numeric(12,2) NOT NULL,
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
                proof_attachment text,
                external_id text,
                merchant_id text,
                rejection_reason text,
                created_at timestamp with time zone NOT NULL,
                updated_at timestamp with time zone NOT NULL
            );

            CREATE TABLE public.settings (
                id text PRIMARY KEY NOT NULL,
                tenant_id text,
                key text NOT NULL,
                value text NOT NULL,
                description text,
                created_at timestamp with time zone NOT NULL DEFAULT now(),
                updated_at timestamp with time zone NOT NULL DEFAULT now()
            );
            "#,
        )
        .execute(pool)
        .await
        .expect("subscription tables should be creatable for executor tests");
    }

    async fn create_router_and_pppoe_tables(pool: &sqlx::PgPool) {
        sqlx::raw_sql(
            r#"
            CREATE TABLE public.mikrotik_routers (
                id text PRIMARY KEY NOT NULL,
                tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
                name text NOT NULL,
                host text NOT NULL,
                port integer NOT NULL DEFAULT 8728,
                username text NOT NULL,
                password text NOT NULL,
                is_active boolean NOT NULL DEFAULT true,
                created_at timestamp with time zone NOT NULL,
                updated_at timestamp with time zone NOT NULL
            );

            CREATE TABLE public.pppoe_profiles (
                id text PRIMARY KEY NOT NULL,
                tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
                name text NOT NULL,
                rate_limit text,
                session_timeout_seconds integer,
                is_active boolean NOT NULL DEFAULT true,
                created_at timestamp with time zone NOT NULL,
                updated_at timestamp with time zone NOT NULL
            );

            CREATE TABLE public.pppoe_accounts (
                id text PRIMARY KEY NOT NULL,
                tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
                router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
                customer_id text NOT NULL REFERENCES public.customers(id) ON DELETE CASCADE,
                location_id text NOT NULL REFERENCES public.customer_locations(id) ON DELETE CASCADE,
                username text NOT NULL,
                password_enc text NOT NULL,
                package_id text REFERENCES public.isp_packages(id) ON DELETE SET NULL,
                profile_id text REFERENCES public.pppoe_profiles(id) ON DELETE SET NULL,
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
                CONSTRAINT pppoe_accounts_tenant_router_username_unique UNIQUE (tenant_id, router_id, username),
                CONSTRAINT chk_pppoe_accounts_account_source CHECK (account_source IN ('router', 'managed_radius'))
            );
            "#,
        )
        .execute(pool)
        .await
        .expect("router and pppoe tables should be creatable for executor tests");
    }

    async fn create_router_inventory_tables(pool: &sqlx::PgPool) {
        sqlx::raw_sql(
            r#"
            CREATE TABLE public.mikrotik_ppp_profiles (
                id text PRIMARY KEY NOT NULL,
                tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
                router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
                name text NOT NULL,
                local_address text,
                remote_address text,
                rate_limit text,
                dns_server text,
                only_one boolean,
                change_tcp_mss boolean,
                use_compression boolean,
                use_encryption boolean,
                use_ipv6 boolean,
                bridge text,
                comment text,
                router_present boolean NOT NULL DEFAULT true,
                last_sync_at timestamp with time zone,
                created_at timestamp with time zone NOT NULL,
                updated_at timestamp with time zone NOT NULL,
                CONSTRAINT mikrotik_ppp_profiles_unique UNIQUE (tenant_id, router_id, name)
            );

            CREATE TABLE public.mikrotik_ip_pools (
                id text PRIMARY KEY NOT NULL,
                tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
                router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
                name text NOT NULL,
                ranges text,
                next_pool text,
                comment text,
                router_present boolean NOT NULL DEFAULT true,
                last_sync_at timestamp with time zone,
                created_at timestamp with time zone NOT NULL,
                updated_at timestamp with time zone NOT NULL,
                CONSTRAINT mikrotik_ip_pools_unique UNIQUE (tenant_id, router_id, name)
            );
            "#,
        )
        .execute(pool)
        .await
        .expect("router inventory tables should be creatable for executor tests");
    }

    async fn create_package_router_mapping_table(pool: &sqlx::PgPool) {
        sqlx::raw_sql(
            r#"
            CREATE TABLE public.isp_package_router_mappings (
                id text PRIMARY KEY NOT NULL,
                tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
                router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
                package_id text NOT NULL REFERENCES public.isp_packages(id) ON DELETE CASCADE,
                router_profile_name text NOT NULL,
                address_pool text,
                created_at timestamp with time zone NOT NULL,
                updated_at timestamp with time zone NOT NULL,
                CONSTRAINT isp_pkg_router_unique UNIQUE (tenant_id, router_id, package_id)
            );
            "#,
        )
        .execute(pool)
        .await
        .expect("package router mapping table should be creatable for executor tests");
    }

    async fn create_ready_batch(pool: &sqlx::PgPool, tenant_id: &str) -> String {
        let batch_id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_import_batches (
                id,
                tenant_id,
                source_filename,
                source_sha256,
                source_size_bytes,
                parse_status,
                execution_status,
                execution_mode,
                progress_json,
                summary_json,
                error_json,
                created_at,
                updated_at
            )
            VALUES (
                $1, $2, 'mixradius.sql.gz', 'checksum', 1024,
                'ready', 'pending', 'preview_only',
                '{}'::jsonb, '{}'::jsonb, '[]'::jsonb, now(), now()
            )
            "#,
        )
        .bind(&batch_id)
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("executor test batch should be insertable");

        batch_id
    }

    async fn insert_staged_plan(
        pool: &sqlx::PgPool,
        tenant_id: &str,
        batch_id: &str,
        source_ref: &str,
        plan_name: &str,
        price: f64,
    ) {
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_staging_plans (
                id,
                tenant_id,
                import_batch_id,
                source_ref,
                plan_name,
                bandwidth_name,
                price,
                validity,
                shared_users,
                source_json,
                created_at,
                updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, '10 Mbps', $6, '30 days', 1,
                jsonb_build_object('sourceRef', $4, 'planName', $5),
                now(), now()
            )
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(batch_id)
        .bind(source_ref)
        .bind(plan_name)
        .bind(price)
        .execute(pool)
        .await
        .expect("staged plan should be insertable");
    }

    async fn insert_existing_package(
        pool: &sqlx::PgPool,
        tenant_id: &str,
        package_id: &str,
        name: &str,
        price_monthly: f64,
    ) {
        sqlx::query(
            r#"
            INSERT INTO public.isp_packages (
                id,
                tenant_id,
                service_type,
                name,
                description,
                features,
                is_active,
                price_monthly,
                price_yearly,
                created_at,
                updated_at
            )
            VALUES (
                $1, $2, 'internet_pppoe', $3, 'Existing package',
                ARRAY['PPPoE']::text[], true, $4, 0, now(), now()
            )
            "#,
        )
        .bind(package_id)
        .bind(tenant_id)
        .bind(name)
        .bind(price_monthly)
        .execute(pool)
        .await
        .expect("existing package should be insertable");
    }

    async fn insert_staged_customer(
        pool: &sqlx::PgPool,
        tenant_id: &str,
        batch_id: &str,
        source_ref: &str,
        member_id: &str,
        username: &str,
        fullname: &str,
        email: &str,
        phone: &str,
        address: &str,
    ) {
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_staging_customers (
                id,
                tenant_id,
                import_batch_id,
                source_ref,
                member_id,
                username,
                fullname,
                email,
                phonenumber,
                address,
                trx_status,
                source_json,
                created_at,
                updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'PAID',
                jsonb_build_object('memberId', $5, 'username', $6),
                now(), now()
            )
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(batch_id)
        .bind(source_ref)
        .bind(member_id)
        .bind(username)
        .bind(fullname)
        .bind(email)
        .bind(phone)
        .bind(address)
        .execute(pool)
        .await
        .expect("staged customer should be insertable");
    }

    async fn insert_staged_customer_with_lifecycle(
        pool: &sqlx::PgPool,
        tenant_id: &str,
        batch_id: &str,
        member_id: &str,
        fullname: &str,
        plan_name: &str,
        price: f64,
        trx_status: &str,
        expired_on: &str,
    ) {
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_staging_customers (
                id,
                tenant_id,
                import_batch_id,
                source_ref,
                member_id,
                username,
                fullname,
                email,
                phonenumber,
                address,
                plan_name,
                price,
                renewed_on,
                expired_on,
                trx_status,
                source_json,
                created_at,
                updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $5, $6, concat($5, '@example.test'), '0800',
                'Alamat import', $7, $8, '2026-04-01 00:00:00+00'::timestamptz,
                $9::timestamptz, $10,
                jsonb_build_object('memberId', $5, 'planName', $7, 'trxStatus', $10),
                now(), now()
            )
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(batch_id)
        .bind(format!("row-{member_id}"))
        .bind(member_id)
        .bind(fullname)
        .bind(plan_name)
        .bind(price)
        .bind(expired_on)
        .bind(trx_status)
        .execute(pool)
        .await
        .expect("staged customer lifecycle row should be insertable");
    }

    async fn insert_staged_customer_with_pppoe_metadata(
        pool: &sqlx::PgPool,
        tenant_id: &str,
        batch_id: &str,
        member_id: &str,
        username: &str,
        password: &str,
        remote_address: Option<&str>,
    ) {
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_staging_customers (
                id,
                tenant_id,
                import_batch_id,
                source_ref,
                member_id,
                username,
                password,
                fullname,
                email,
                phonenumber,
                address,
                plan_name,
                price,
                trx_status,
                source_json,
                created_at,
                updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, concat('Pelanggan ', $5),
                concat($6, '@example.test'), '0800', 'Alamat PPPoE', 'Paket PPPoE', 100000,
                'PAID',
                jsonb_build_object(
                    'radreply', jsonb_build_array(
                        jsonb_build_object('attribute', 'Framed-IP-Address', 'value', $8)
                    )
                ),
                now(), now()
            )
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(batch_id)
        .bind(format!("row-{member_id}"))
        .bind(member_id)
        .bind(username)
        .bind(password)
        .bind(remote_address)
        .execute(pool)
        .await
        .expect("staged customer PPPoE row should be insertable");
    }

    async fn insert_router(pool: &sqlx::PgPool, tenant_id: &str, router_id: &str) {
        sqlx::query(
            r#"
            INSERT INTO public.mikrotik_routers (
                id, tenant_id, name, host, port, username, password, is_active, created_at, updated_at
            )
            VALUES ($1, $2, concat('Router ', $1), '192.0.2.1', 8728, 'admin', 'secret', true, now(), now())
            "#,
        )
        .bind(router_id)
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("router should be insertable");
    }

    async fn insert_router_profile(
        pool: &sqlx::PgPool,
        tenant_id: &str,
        router_id: &str,
        profile_name: &str,
    ) {
        sqlx::query(
            r#"
            INSERT INTO public.mikrotik_ppp_profiles (
                id, tenant_id, router_id, name, router_present, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, true, now(), now())
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(router_id)
        .bind(profile_name)
        .execute(pool)
        .await
        .expect("router profile should be insertable");
    }

    fn router_override(source_ref: &str, router_id: &str) -> Vec<MixradiusImportMappingOverride> {
        vec![MixradiusImportMappingOverride {
            source_kind: "nas".to_string(),
            source_value: source_ref.to_string(),
            target_kind: "router".to_string(),
            target_value: router_id.to_string(),
        }]
    }

    async fn insert_staged_customer_location(
        pool: &sqlx::PgPool,
        tenant_id: &str,
        batch_id: &str,
        source_ref: &str,
        member_id: &str,
        latitude: f64,
        longitude: f64,
    ) {
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_staging_customer_locations (
                id,
                tenant_id,
                import_batch_id,
                source_ref,
                member_id,
                latitude,
                longitude,
                source_json,
                created_at,
                updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                jsonb_build_object('memberId', $5, 'lat', $6, 'lon', $7),
                now(), now()
            )
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(batch_id)
        .bind(source_ref)
        .bind(member_id)
        .bind(latitude)
        .bind(longitude)
        .execute(pool)
        .await
        .expect("staged customer location should be insertable");
    }

    async fn insert_existing_customer(
        pool: &sqlx::PgPool,
        tenant_id: &str,
        customer_id: &str,
        name: &str,
        email: Option<&str>,
        phone: Option<&str>,
        notes: Option<&str>,
    ) {
        sqlx::query(
            r#"
            INSERT INTO public.customers (
                id,
                tenant_id,
                name,
                email,
                phone,
                notes,
                is_active,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, true, now(), now())
            "#,
        )
        .bind(customer_id)
        .bind(tenant_id)
        .bind(name)
        .bind(email)
        .bind(phone)
        .bind(notes)
        .execute(pool)
        .await
        .expect("existing customer should be insertable");
    }

    async fn import_customer_package_and_location_fixture(
        pool: &sqlx::PgPool,
        tenant_id: &str,
        batch_id: &str,
        member_id: &str,
        plan_ref: &str,
        plan_name: &str,
        price: f64,
    ) {
        insert_staged_plan(pool, tenant_id, batch_id, plan_ref, plan_name, price).await;
        let executor = MixradiusImportExecutor::new(pool.clone());
        executor
            .execute_package_imports(tenant_id, batch_id, &[])
            .await
            .expect("fixture package import should succeed");
        executor
            .execute_customer_imports(tenant_id, batch_id)
            .await
            .expect("fixture customer import should succeed");

        let customer_id: String = sqlx::query_scalar(
            r#"
            SELECT entity_id
            FROM public.mixradius_import_external_refs
            WHERE tenant_id = $1 AND entity_type = 'customer' AND source_ref = $2
            "#,
        )
        .bind(tenant_id)
        .bind(member_id)
        .fetch_one(pool)
        .await
        .expect("fixture customer external ref should exist");
        assert!(
            !customer_id.is_empty(),
            "fixture customer external ref should point to an entity"
        );
    }

    #[tokio::test]
    async fn mixradius_import_executor_reuses_exact_name_without_duplicate_package() {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_plan(
            &pool,
            "tenant-executor",
            &batch_id,
            "plan-1",
            "Paket 10 Mbps",
            150_000.0,
        )
        .await;
        insert_existing_package(
            &pool,
            "tenant-executor",
            "pkg-existing",
            "paket 10 mbps",
            150_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_package_imports("tenant-executor", &batch_id, &[])
            .await
            .expect("exact-name matching package should be reused");

        let package_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM public.isp_packages WHERE tenant_id = $1")
                .bind("tenant-executor")
                .fetch_one(&pool)
                .await
                .expect("package count should query");
        assert_eq!(package_count, 1);
        assert_eq!(summary.imported_rows, 0);
        assert_eq!(summary.updated_rows, 1);
        assert_eq!(summary.conflict_rows, 0);

        let external_ref = sqlx::query(
            r#"
            SELECT entity_id
            FROM public.mixradius_import_external_refs
            WHERE tenant_id = $1
              AND import_batch_id = $2
              AND entity_type = 'package'
              AND source_ref = 'plan-1'
            "#,
        )
        .bind("tenant-executor")
        .bind(&batch_id)
        .fetch_one(&pool)
        .await
        .expect("package reuse should register external ref");
        assert_eq!(external_ref.get::<String, _>("entity_id"), "pkg-existing");

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_creates_package_for_new_plan() {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_plan(
            &pool,
            "tenant-executor",
            &batch_id,
            "plan-2",
            "Paket 20 Mbps",
            250_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_package_imports("tenant-executor", &batch_id, &[])
            .await
            .expect("new MixRadius plan should create package");

        let created_package = sqlx::query(
            r#"
            SELECT name, service_type, price_monthly::float8 AS price_monthly, price_yearly::float8 AS price_yearly
            FROM public.isp_packages
            WHERE tenant_id = $1
            "#,
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("created package should query");
        assert_eq!(created_package.get::<String, _>("name"), "Paket 20 Mbps");
        assert_eq!(
            created_package.get::<String, _>("service_type"),
            "internet_pppoe"
        );
        assert_eq!(created_package.get::<f64, _>("price_monthly"), 250_000.0);
        assert_eq!(created_package.get::<f64, _>("price_yearly"), 0.0);
        assert_eq!(summary.imported_rows, 1);
        assert_eq!(summary.updated_rows, 0);
        assert_eq!(summary.conflict_rows, 0);

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_maps_imported_package_to_selected_router_profile() {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_router_and_pppoe_tables(&pool).await;
        create_router_inventory_tables(&pool).await;
        create_package_router_mapping_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_router(&pool, "tenant-executor", "router-mix-1").await;
        insert_router_profile(&pool, "tenant-executor", "router-mix-1", "Paket 20 Mbps").await;
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_staging_nas (
                id, tenant_id, import_batch_id, source_ref, nas_name, nas_ip_or_cidr, shortname, source_json, created_at, updated_at
            )
            VALUES ($1, $2, $3, 'nas-mix-1', 'Router Mix', '192.0.2.10', 'RTR-MIX', '{}'::jsonb, now(), now())
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind("tenant-executor")
        .bind(&batch_id)
        .execute(&pool)
        .await
        .expect("staged nas row should insert");
        insert_staged_plan(
            &pool,
            "tenant-executor",
            &batch_id,
            "plan-2",
            "Paket 20 Mbps",
            250_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_package_imports(
                "tenant-executor",
                &batch_id,
                &router_override("nas-mix-1", "router-mix-1"),
            )
            .await
            .expect("new MixRadius plan should create package and router mapping");

        let mapping = sqlx::query(
            r#"
            SELECT m.router_id, m.router_profile_name, p.name AS package_name
            FROM public.isp_package_router_mappings m
            JOIN public.isp_packages p
              ON p.tenant_id = m.tenant_id
             AND p.id = m.package_id
            WHERE m.tenant_id = $1
            "#,
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("created package router mapping should query");
        assert_eq!(mapping.get::<String, _>("router_id"), "router-mix-1");
        assert_eq!(
            mapping.get::<String, _>("router_profile_name"),
            "Paket 20 Mbps"
        );
        assert_eq!(mapping.get::<String, _>("package_name"), "Paket 20 Mbps");
        assert_eq!(summary.imported_rows, 1);
        assert!(summary.warnings.is_empty());

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_maps_plan_to_router_profile_by_bandwidth_when_names_differ()
    {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_router_and_pppoe_tables(&pool).await;
        create_router_inventory_tables(&pool).await;
        create_package_router_mapping_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_router(&pool, "tenant-executor", "router-mix-bandwidth").await;
        insert_router_profile(
            &pool,
            "tenant-executor",
            "router-mix-bandwidth",
            "22-Upto-100Mbps",
        )
        .await;
        sqlx::query(
            r#"
            UPDATE public.mikrotik_ppp_profiles
            SET rate_limit = '100M/100M'
            WHERE tenant_id = $1 AND router_id = $2 AND name = $3
            "#,
        )
        .bind("tenant-executor")
        .bind("router-mix-bandwidth")
        .bind("22-Upto-100Mbps")
        .execute(&pool)
        .await
        .expect("profile rate limit should update");
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_staging_nas (
                id, tenant_id, import_batch_id, source_ref, nas_name, nas_ip_or_cidr, shortname, source_json, created_at, updated_at
            )
            VALUES ($1, $2, $3, 'nas-mix-bandwidth', 'Router Mix Bandwidth', '192.0.2.30', 'RTR-MIX-BW', '{}'::jsonb, now(), now())
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind("tenant-executor")
        .bind(&batch_id)
        .execute(&pool)
        .await
        .expect("staged nas row should insert");
        insert_staged_plan(
            &pool,
            "tenant-executor",
            &batch_id,
            "plan-bandwidth-100",
            "Elite-100-100Mbps",
            1_225_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_package_imports(
                "tenant-executor",
                &batch_id,
                &router_override("nas-mix-bandwidth", "router-mix-bandwidth"),
            )
            .await
            .expect("package import should map by bandwidth-compatible router profile");

        let router_profile_name: String = sqlx::query_scalar(
            r#"
            SELECT router_profile_name
            FROM public.isp_package_router_mappings
            WHERE tenant_id = $1
            "#,
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("package router mapping should query");
        assert_eq!(router_profile_name, "22-Upto-100Mbps");
        assert!(summary.warnings.is_empty());

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_keeps_router_mapping_even_when_profile_inventory_missing() {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_router_and_pppoe_tables(&pool).await;
        create_router_inventory_tables(&pool).await;
        create_package_router_mapping_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_router(&pool, "tenant-executor", "router-mix-2").await;
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_staging_nas (
                id, tenant_id, import_batch_id, source_ref, nas_name, nas_ip_or_cidr, shortname, source_json, created_at, updated_at
            )
            VALUES ($1, $2, $3, 'nas-mix-2', 'Router Mix 2', '192.0.2.20', 'RTR-MIX-2', '{}'::jsonb, now(), now())
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind("tenant-executor")
        .bind(&batch_id)
        .execute(&pool)
        .await
        .expect("staged nas row should insert");
        insert_staged_plan(
            &pool,
            "tenant-executor",
            &batch_id,
            "plan-missing-profile",
            "Elite-100-100Mbps",
            1_225_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_package_imports(
                "tenant-executor",
                &batch_id,
                &router_override("nas-mix-2", "router-mix-2"),
            )
            .await
            .expect("package import should still create router mapping placeholder");

        let mapping = sqlx::query(
            r#"
            SELECT m.router_id, m.router_profile_name, p.name AS package_name
            FROM public.isp_package_router_mappings m
            JOIN public.isp_packages p
              ON p.tenant_id = m.tenant_id
             AND p.id = m.package_id
            WHERE m.tenant_id = $1
            "#,
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("package router mapping should still be created");
        assert_eq!(mapping.get::<String, _>("router_id"), "router-mix-2");
        assert_eq!(
            mapping.get::<String, _>("router_profile_name"),
            "Elite-100-100Mbps"
        );
        assert_eq!(
            mapping.get::<String, _>("package_name"),
            "Elite-100-100Mbps"
        );
        assert!(summary.warnings.iter().any(|warning| {
            warning.contains("Elite-100-100Mbps")
                && warning.contains("mapping package-router tetap dibuat")
        }));

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_marks_conflict_when_name_matches_but_price_differs() {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_plan(
            &pool,
            "tenant-executor",
            &batch_id,
            "plan-3",
            "Paket 30 Mbps",
            350_000.0,
        )
        .await;
        insert_existing_package(
            &pool,
            "tenant-executor",
            "pkg-existing",
            "paket 30 mbps",
            300_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_package_imports("tenant-executor", &batch_id, &[])
            .await
            .expect("price mismatch should be recorded as conflict, not hard failure");

        let package_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM public.isp_packages WHERE tenant_id = $1")
                .bind("tenant-executor")
                .fetch_one(&pool)
                .await
                .expect("package count should query");
        assert_eq!(package_count, 1);
        assert_eq!(summary.imported_rows, 0);
        assert_eq!(summary.updated_rows, 0);
        assert_eq!(summary.conflict_rows, 1);

        let conflict = sqlx::query(
            r#"
            SELECT conflict_type, conflict_message, resolution_status
            FROM public.mixradius_import_conflicts
            WHERE tenant_id = $1
              AND import_batch_id = $2
              AND source_table = 'tbl_plans'
              AND source_ref = 'plan-3'
            "#,
        )
        .bind("tenant-executor")
        .bind(&batch_id)
        .fetch_one(&pool)
        .await
        .expect("price mismatch should create import conflict");
        assert_eq!(
            conflict.get::<String, _>("conflict_type"),
            "package_price_mismatch"
        );
        assert_eq!(conflict.get::<String, _>("resolution_status"), "open");
        assert!(conflict
            .get::<String, _>("conflict_message")
            .contains("harga"));

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_creates_customer_and_default_location() {
        let (pool, db_name) = isolated_pool().await;
        create_customer_tables(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_customer(
            &pool,
            "tenant-executor",
            &batch_id,
            "row-cust-1",
            "MBR-1",
            "cust001",
            "Budi Santoso",
            "budi@example.test",
            "08123456789",
            "Jl. Merdeka 1",
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_customer_imports("tenant-executor", &batch_id)
            .await
            .expect("new MixRadius customer should create customer and default location");

        let customer = sqlx::query(
            r#"
            SELECT name, email, phone, notes
            FROM public.customers
            WHERE tenant_id = $1
            "#,
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("imported customer should query");
        assert_eq!(customer.get::<String, _>("name"), "Budi Santoso");
        assert_eq!(customer.get::<String, _>("email"), "budi@example.test");
        assert_eq!(customer.get::<String, _>("phone"), "08123456789");
        assert!(customer
            .get::<Option<String>, _>("notes")
            .unwrap_or_default()
            .contains("MixRadius"));

        let location = sqlx::query(
            r#"
            SELECT label, address_line1
            FROM public.customer_locations
            WHERE tenant_id = $1
            "#,
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("default location should query");
        assert_eq!(location.get::<String, _>("label"), "Lokasi Utama");
        assert_eq!(location.get::<String, _>("address_line1"), "Jl. Merdeka 1");
        assert_eq!(summary.imported_rows, 1);
        assert_eq!(summary.location_imported_rows, 1);

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_reuses_customer_external_ref_on_repeat_import() {
        let (pool, db_name) = isolated_pool().await;
        create_customer_tables(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_customer(
            &pool,
            "tenant-executor",
            &batch_id,
            "row-cust-2",
            "MBR-2",
            "cust002",
            "Siti Aminah",
            "siti@example.test",
            "08111111111",
            "Jl. Sudirman 2",
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        executor
            .execute_customer_imports("tenant-executor", &batch_id)
            .await
            .expect("first import should succeed");
        let second = executor
            .execute_customer_imports("tenant-executor", &batch_id)
            .await
            .expect("repeat import should reuse external refs");

        let customer_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM public.customers WHERE tenant_id = $1")
                .bind("tenant-executor")
                .fetch_one(&pool)
                .await
                .expect("customer count should query");
        let location_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM public.customer_locations WHERE tenant_id = $1",
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("location count should query");
        assert_eq!(customer_count, 1);
        assert_eq!(location_count, 1);
        assert_eq!(second.imported_rows, 0);
        assert_eq!(second.updated_rows, 1);
        assert_eq!(second.location_updated_rows, 1);

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_maps_customer_coordinates_to_location() {
        let (pool, db_name) = isolated_pool().await;
        create_customer_tables(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_customer(
            &pool,
            "tenant-executor",
            &batch_id,
            "row-cust-3",
            "MBR-3",
            "cust003",
            "Rina",
            "rina@example.test",
            "08222222222",
            "Jl. Asia Afrika 3",
        )
        .await;
        insert_staged_customer_location(
            &pool,
            "tenant-executor",
            &batch_id,
            "map-3",
            "MBR-3",
            -6.200001,
            106.816666,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        executor
            .execute_customer_imports("tenant-executor", &batch_id)
            .await
            .expect("customer import should map staged coordinates");

        let location = sqlx::query(
            r#"
            SELECT latitude::float8 AS latitude, longitude::float8 AS longitude
            FROM public.customer_locations
            WHERE tenant_id = $1
            "#,
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("location coordinates should query");
        assert_eq!(location.get::<f64, _>("latitude"), -6.200001);
        assert_eq!(location.get::<f64, _>("longitude"), 106.816666);

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_safe_mode_preserves_existing_customer_notes() {
        let (pool, db_name) = isolated_pool().await;
        create_customer_tables(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_customer(
            &pool,
            "tenant-executor",
            &batch_id,
            "row-cust-4",
            "MBR-4",
            "cust004",
            "Ahmad",
            "ahmad@example.test",
            "08333333333",
            "Jl. Diponegoro 4",
        )
        .await;
        insert_existing_customer(
            &pool,
            "tenant-executor",
            "customer-existing-4",
            "Ahmad Lokal",
            Some("lokal@example.test"),
            Some("08000000000"),
            Some("Catatan lokal penting"),
        )
        .await;
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_import_external_refs (
                id,
                tenant_id,
                import_batch_id,
                entity_type,
                entity_id,
                source_system,
                source_ref,
                last_seen_at,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, 'customer', 'customer-existing-4', 'mixradius', 'MBR-4', now(), now(), now())
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind("tenant-executor")
        .bind(&batch_id)
        .execute(&pool)
        .await
        .expect("customer external ref should be insertable");

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_customer_imports("tenant-executor", &batch_id)
            .await
            .expect("safe import should preserve local notes");

        let customer = sqlx::query(
            "SELECT name, email, phone, notes FROM public.customers WHERE id = 'customer-existing-4'",
        )
        .fetch_one(&pool)
        .await
        .expect("existing customer should query");
        assert_eq!(customer.get::<String, _>("name"), "Ahmad Lokal");
        assert_eq!(customer.get::<String, _>("email"), "lokal@example.test");
        assert_eq!(customer.get::<String, _>("phone"), "08000000000");
        assert_eq!(customer.get::<String, _>("notes"), "Catatan lokal penting");
        assert_eq!(summary.updated_rows, 1);
        assert!(summary
            .warnings
            .iter()
            .any(|warning| warning.contains("local edit")));

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_maps_paid_current_subscription_to_active() {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_subscription_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_customer_with_lifecycle(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-1",
            "Pelanggan Aktif",
            "Paket 10 Mbps",
            150_000.0,
            "PAID",
            "2026-05-01 00:00:00+00",
        )
        .await;
        import_customer_package_and_location_fixture(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-1",
            "plan-sub-1",
            "Paket 10 Mbps",
            150_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_subscription_imports("tenant-executor", &batch_id)
            .await
            .expect("paid current subscription should import");

        let row = sqlx::query(
            "SELECT status, price::float8 AS price, currency_code, notes FROM public.customer_subscriptions WHERE tenant_id = $1",
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("subscription should query");
        assert_eq!(row.get::<String, _>("status"), "active");
        assert_eq!(row.get::<f64, _>("price"), 150_000.0);
        assert_eq!(row.get::<String, _>("currency_code"), "IDR");
        assert!(row
            .get::<Option<String>, _>("notes")
            .unwrap_or_default()
            .contains("MixRadius"));
        assert_eq!(summary.imported_rows, 1);

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_resolves_subscription_location_from_staged_map_ref() {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_subscription_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;

        insert_staged_customer_with_lifecycle(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-MAP-1",
            "Pelanggan Map Ref",
            "Paket 15 Mbps",
            175_000.0,
            "PAID",
            "2026-05-01 00:00:00+00",
        )
        .await;
        insert_staged_customer_location(
            &pool,
            "tenant-executor",
            &batch_id,
            "map-row-15",
            "MBR-MAP-1",
            -7.275233,
            110.355211,
        )
        .await;

        insert_existing_package(
            &pool,
            "tenant-executor",
            "pkg-map-ref",
            "Paket 15 Mbps",
            175_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        executor
            .execute_customer_imports("tenant-executor", &batch_id)
            .await
            .expect("customer import should succeed");
        let summary = executor
            .execute_subscription_imports("tenant-executor", &batch_id)
            .await
            .expect("subscription import should resolve location using staged map ref");

        let location_ref: String = sqlx::query_scalar(
            r#"
            SELECT entity_id
            FROM public.mixradius_import_external_refs
            WHERE tenant_id = $1
              AND entity_type = 'location'
              AND source_ref = 'map-row-15'
            "#,
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("mapped location external ref should exist");
        assert!(!location_ref.is_empty());
        let conflict_types: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT conflict_type
            FROM public.mixradius_import_conflicts
            WHERE tenant_id = $1 AND import_batch_id = $2
            ORDER BY created_at ASC
            "#,
        )
        .bind("tenant-executor")
        .bind(&batch_id)
        .fetch_all(&pool)
        .await
        .expect("conflict rows should query");
        assert_eq!(
            summary.imported_rows, 1,
            "summary: {summary:?}, conflicts: {conflict_types:?}"
        );

        let row = sqlx::query(
            r#"
            SELECT cs.status, cl.latitude::float8 AS latitude, cl.longitude::float8 AS longitude
            FROM public.customer_subscriptions cs
            JOIN public.customer_locations cl
              ON cl.tenant_id = cs.tenant_id AND cl.id = cs.location_id
            WHERE cs.tenant_id = $1
            "#,
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("subscription with mapped location should query");

        assert_eq!(row.get::<String, _>("status"), "active");
        assert_eq!(row.get::<f64, _>("latitude"), -7.275233);
        assert_eq!(row.get::<f64, _>("longitude"), 110.355211);

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_maps_unpaid_expired_subscription_to_suspended() {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_subscription_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_customer_with_lifecycle(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-2",
            "Pelanggan Suspend",
            "Paket 20 Mbps",
            250_000.0,
            "UNPAID",
            "2026-04-01 00:00:00+00",
        )
        .await;
        import_customer_package_and_location_fixture(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-2",
            "plan-sub-2",
            "Paket 20 Mbps",
            250_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        executor
            .execute_subscription_imports("tenant-executor", &batch_id)
            .await
            .expect("unpaid expired subscription should import");

        let status: String = sqlx::query_scalar(
            "SELECT status FROM public.customer_subscriptions WHERE tenant_id = $1",
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("subscription status should query");
        assert_eq!(status, "suspended");

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_maps_unpaid_current_to_active_with_warning() {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_subscription_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_customer_with_lifecycle(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-3",
            "Pelanggan Belum Lunas",
            "Paket 30 Mbps",
            350_000.0,
            "UNPAID",
            "2026-05-01 00:00:00+00",
        )
        .await;
        import_customer_package_and_location_fixture(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-3",
            "plan-sub-3",
            "Paket 30 Mbps",
            350_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_subscription_imports("tenant-executor", &batch_id)
            .await
            .expect("unpaid current subscription should import with warning");

        let status: String = sqlx::query_scalar(
            "SELECT status FROM public.customer_subscriptions WHERE tenant_id = $1",
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("subscription status should query");
        assert_eq!(status, "active");
        assert!(summary
            .warnings
            .iter()
            .any(|warning| warning.contains("belum lunas")));

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_maps_pending_to_pending_installation_not_cancelled() {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_subscription_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_customer_with_lifecycle(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-4",
            "Pelanggan Pending",
            "Paket 40 Mbps",
            450_000.0,
            "PENDING",
            "2026-05-01 00:00:00+00",
        )
        .await;
        import_customer_package_and_location_fixture(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-4",
            "plan-sub-4",
            "Paket 40 Mbps",
            450_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_subscription_imports("tenant-executor", &batch_id)
            .await
            .expect("pending subscription should import as pending review lifecycle");

        let status: String = sqlx::query_scalar(
            "SELECT status FROM public.customer_subscriptions WHERE tenant_id = $1",
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("subscription status should query");
        assert_eq!(status, "pending_installation");
        assert_ne!(status, "cancelled");
        assert!(summary
            .warnings
            .iter()
            .any(|warning| warning.contains("PENDING")));

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_blocks_duplicate_location_subscription_without_ref() {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_subscription_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_customer_with_lifecycle(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-5",
            "Pelanggan Duplicate",
            "Paket 50 Mbps",
            550_000.0,
            "PAID",
            "2026-05-01 00:00:00+00",
        )
        .await;
        import_customer_package_and_location_fixture(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-5",
            "plan-sub-5",
            "Paket 50 Mbps",
            550_000.0,
        )
        .await;
        let customer_id: String =
            sqlx::query_scalar("SELECT id FROM public.customers WHERE tenant_id = $1 LIMIT 1")
                .bind("tenant-executor")
                .fetch_one(&pool)
                .await
                .expect("customer id should query");
        let location_id: String = sqlx::query_scalar(
            "SELECT id FROM public.customer_locations WHERE tenant_id = $1 LIMIT 1",
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("location id should query");
        let package_id: String =
            sqlx::query_scalar("SELECT id FROM public.isp_packages WHERE tenant_id = $1 LIMIT 1")
                .bind("tenant-executor")
                .fetch_one(&pool)
                .await
                .expect("package id should query");
        sqlx::query(
            r#"
            INSERT INTO public.customer_subscriptions (
                id, tenant_id, customer_id, location_id, package_id, billing_cycle, price,
                currency_code, status, notes, created_at, updated_at
            )
            VALUES ('existing-sub', $1, $2, $3, $4, 'monthly', 550000, 'IDR', 'active', 'Local active subscription', now(), now())
            "#,
        )
        .bind("tenant-executor")
        .bind(&customer_id)
        .bind(&location_id)
        .bind(&package_id)
        .execute(&pool)
        .await
        .expect("existing subscription should insert");

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_subscription_imports("tenant-executor", &batch_id)
            .await
            .expect("duplicate active subscription should be conflict, not hard failure");

        let subscription_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM public.customer_subscriptions WHERE tenant_id = $1",
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("subscription count should query");
        assert_eq!(subscription_count, 1);
        assert_eq!(summary.conflict_rows, 1);

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_bootstraps_first_invoice_without_replaying_legacy_transactions(
    ) {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_subscription_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_customer_with_lifecycle(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-6",
            "Pelanggan Legacy Tx",
            "Paket 60 Mbps",
            650_000.0,
            "PAID",
            "2026-05-01 00:00:00+00",
        )
        .await;
        import_customer_package_and_location_fixture(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-6",
            "plan-sub-6",
            "Paket 60 Mbps",
            650_000.0,
        )
        .await;
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_staging_transactions (
                id, tenant_id, import_batch_id, source_ref, invoice_no, member_id,
                transaction_status, amount, paid_at, source_json, created_at, updated_at
            )
            VALUES ($1, $2, $3, 'trx-legacy-1', 'INV-OLD-1', 'MBR-SUB-6', 'PAID', 650000, now(), '{}'::jsonb, now(), now())
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind("tenant-executor")
        .bind(&batch_id)
        .execute(&pool)
        .await
        .expect("legacy transaction staging row should insert");

        let executor = MixradiusImportExecutor::new(pool.clone());
        executor
            .execute_subscription_imports("tenant-executor", &batch_id)
            .await
            .expect("subscription import should bootstrap first invoice");

        let invoice = sqlx::query(
            r#"
            SELECT external_id, due_date
            FROM public.invoices
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("bootstrap invoice should query");
        let external_id = invoice
            .get::<Option<String>, _>("external_id")
            .unwrap_or_default();
        assert!(external_id.starts_with("pkgsub:"));
        assert!(
            external_id.ends_with(":2026-04"),
            "unexpected external_id {external_id}"
        );
        assert_eq!(
            invoice
                .get::<chrono::DateTime<chrono::Utc>, _>("due_date")
                .to_rfc3339(),
            "2026-05-01T00:00:00+00:00"
        );

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_reimport_keeps_bootstrap_invoice_idempotent() {
        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_subscription_table(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_staged_customer_with_lifecycle(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-6B",
            "Pelanggan Legacy Tx Repeat",
            "Paket 60 Mbps",
            650_000.0,
            "PAID",
            "2026-05-01 00:00:00+00",
        )
        .await;
        import_customer_package_and_location_fixture(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-SUB-6B",
            "plan-sub-6b",
            "Paket 60 Mbps",
            650_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        executor
            .execute_subscription_imports("tenant-executor", &batch_id)
            .await
            .expect("initial import should succeed");
        executor
            .execute_subscription_imports("tenant-executor", &batch_id)
            .await
            .expect("reimport should stay idempotent");

        let invoice_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM public.invoices WHERE tenant_id = $1 AND external_id LIKE 'pkgsub:%'",
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("invoice count should query");
        assert_eq!(invoice_count, 1);

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_creates_pppoe_account_with_encrypted_password_and_remote_address(
    ) {
        std::env::set_var("APP_SECRET", "mixradius-test-secret");

        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_subscription_table(&pool).await;
        create_router_and_pppoe_tables(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_router(&pool, "tenant-executor", "router-pppoe-1").await;
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_staging_nas (
                id, tenant_id, import_batch_id, source_ref, nas_name, nas_ip_or_cidr, shortname, source_json, created_at, updated_at
            )
            VALUES ($1, $2, $3, 'nas-1', 'Router PPPoE', '192.0.2.1', 'RTR1', '{}'::jsonb, now(), now())
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind("tenant-executor")
        .bind(&batch_id)
        .execute(&pool)
        .await
        .expect("staged nas row should insert");
        insert_staged_customer_with_pppoe_metadata(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-PPP-1",
            "ppp001",
            "rahasia-ppp-1",
            Some("10.10.10.2"),
        )
        .await;
        import_customer_package_and_location_fixture(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-PPP-1",
            "plan-ppp-1",
            "Paket PPPoE",
            100_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_pppoe_imports(
                "tenant-executor",
                &batch_id,
                &router_override("nas-1", "router-pppoe-1"),
            )
            .await
            .expect("pppoe import should create encrypted account");

        let row = sqlx::query(
            r#"
            SELECT username, password_enc, remote_address, router_id, radius_identity
            FROM public.pppoe_accounts
            WHERE tenant_id = $1
            "#,
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("pppoe account should query");
        assert_eq!(row.get::<String, _>("username"), "ppp001");
        assert_eq!(row.get::<String, _>("remote_address"), "10.10.10.2");
        assert_eq!(row.get::<String, _>("router_id"), "router-pppoe-1");
        assert_eq!(
            row.get::<Option<String>, _>("radius_identity"),
            Some("ppp001".into())
        );

        let password_enc = row.get::<String, _>("password_enc");
        assert_ne!(password_enc, "rahasia-ppp-1");
        assert_eq!(
            decrypt_secret_for("pppoe_secrets", &password_enc).expect("password should decrypt"),
            "rahasia-ppp-1"
        );
        assert_eq!(summary.imported_rows, 1);
        assert_eq!(summary.updated_rows, 0);
        assert_eq!(summary.conflict_rows, 0);

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_reuses_same_router_username_pppoe_row_on_repeat_import() {
        std::env::set_var("APP_SECRET", "mixradius-test-secret");

        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_subscription_table(&pool).await;
        create_router_and_pppoe_tables(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_router(&pool, "tenant-executor", "router-pppoe-2").await;
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_staging_nas (
                id, tenant_id, import_batch_id, source_ref, nas_name, nas_ip_or_cidr, shortname, source_json, created_at, updated_at
            )
            VALUES ($1, $2, $3, 'nas-2', 'Router PPPoE 2', '192.0.2.2', 'RTR2', '{}'::jsonb, now(), now())
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind("tenant-executor")
        .bind(&batch_id)
        .execute(&pool)
        .await
        .expect("staged nas row should insert");
        insert_staged_customer_with_pppoe_metadata(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-PPP-2",
            "ppp002",
            "rahasia-awal",
            Some("10.10.20.2"),
        )
        .await;
        import_customer_package_and_location_fixture(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-PPP-2",
            "plan-ppp-2",
            "Paket PPPoE",
            100_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        executor
            .execute_pppoe_imports(
                "tenant-executor",
                &batch_id,
                &router_override("nas-2", "router-pppoe-2"),
            )
            .await
            .expect("first pppoe import should succeed");

        sqlx::query(
            r#"
            UPDATE public.mixradius_staging_customers
            SET password = 'rahasia-baru',
                source_json = jsonb_build_object(
                    'radreply', jsonb_build_array(
                        jsonb_build_object('attribute', 'Framed-IP-Address', 'value', '10.10.20.9')
                    )
                )
            WHERE tenant_id = $1 AND import_batch_id = $2 AND member_id = 'MBR-PPP-2'
            "#,
        )
        .bind("tenant-executor")
        .bind(&batch_id)
        .execute(&pool)
        .await
        .expect("staged pppoe row should update");

        let second = executor
            .execute_pppoe_imports(
                "tenant-executor",
                &batch_id,
                &router_override("nas-2", "router-pppoe-2"),
            )
            .await
            .expect("repeat pppoe import should update same row");

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM public.pppoe_accounts WHERE tenant_id = $1")
                .bind("tenant-executor")
                .fetch_one(&pool)
                .await
                .expect("pppoe account count should query");
        assert_eq!(count, 1);

        let row = sqlx::query(
            "SELECT password_enc, remote_address FROM public.pppoe_accounts WHERE tenant_id = $1",
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("updated pppoe account should query");
        assert_eq!(row.get::<String, _>("remote_address"), "10.10.20.9");
        assert_eq!(
            decrypt_secret_for("pppoe_secrets", &row.get::<String, _>("password_enc"))
                .expect("updated password should decrypt"),
            "rahasia-baru"
        );
        assert_eq!(second.imported_rows, 0);
        assert_eq!(second.updated_rows, 1);

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_marks_router_mismatch_as_pppoe_conflict() {
        std::env::set_var("APP_SECRET", "mixradius-test-secret");

        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_subscription_table(&pool).await;
        create_router_and_pppoe_tables(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_router(&pool, "tenant-executor", "router-pppoe-3a").await;
        insert_router(&pool, "tenant-executor", "router-pppoe-3b").await;
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_staging_nas (
                id, tenant_id, import_batch_id, source_ref, nas_name, nas_ip_or_cidr, shortname, source_json, created_at, updated_at
            )
            VALUES ($1, $2, $3, 'nas-3', 'Router PPPoE 3', '192.0.2.3', 'RTR3', '{}'::jsonb, now(), now())
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind("tenant-executor")
        .bind(&batch_id)
        .execute(&pool)
        .await
        .expect("staged nas row should insert");
        insert_staged_customer_with_pppoe_metadata(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-PPP-3",
            "ppp003",
            "rahasia-3",
            Some("10.10.30.3"),
        )
        .await;
        import_customer_package_and_location_fixture(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-PPP-3",
            "plan-ppp-3",
            "Paket PPPoE",
            100_000.0,
        )
        .await;

        let customer_id: String = sqlx::query_scalar(
            "SELECT entity_id FROM public.mixradius_import_external_refs WHERE tenant_id = $1 AND entity_type = 'customer' AND source_ref = 'MBR-PPP-3'",
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("customer external ref should exist");
        let location_id: String = sqlx::query_scalar(
            "SELECT entity_id FROM public.mixradius_import_external_refs WHERE tenant_id = $1 AND entity_type = 'location' AND source_ref = 'customer:MBR-PPP-3:default-location'",
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("location external ref should exist");

        sqlx::query(
            r#"
            INSERT INTO public.pppoe_accounts (
                id, tenant_id, router_id, customer_id, location_id, username, password_enc,
                package_id, profile_id, router_profile_name, remote_address, address_pool, disabled, comment,
                account_source, router_present, router_secret_id, last_sync_at, last_error,
                is_provisioned, radius_identity, provisioned_at, provisioning_error,
                created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, 'ppp003', $6,
                NULL, NULL, NULL, '10.10.30.8', NULL, false, NULL,
                'router', false, NULL, NULL, NULL,
                false, 'ppp003', NULL, NULL,
                now(), now()
            )
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind("tenant-executor")
        .bind("router-pppoe-3a")
        .bind(customer_id)
        .bind(location_id)
        .bind("enc:v1:dummy")
        .execute(&pool)
        .await
        .expect("existing mismatched router pppoe account should insert");

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_pppoe_imports(
                "tenant-executor",
                &batch_id,
                &router_override("nas-3", "router-pppoe-3b"),
            )
            .await
            .expect("router mismatch should become conflict");

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM public.pppoe_accounts WHERE tenant_id = $1")
                .bind("tenant-executor")
                .fetch_one(&pool)
                .await
                .expect("pppoe count should query");
        assert_eq!(count, 1);
        assert_eq!(summary.conflict_rows, 1);

        let conflict = sqlx::query(
            r#"
            SELECT conflict_type, conflict_message
            FROM public.mixradius_import_conflicts
            WHERE tenant_id = $1 AND import_batch_id = $2 AND source_ref = 'MBR-PPP-3'
            "#,
        )
        .bind("tenant-executor")
        .bind(&batch_id)
        .fetch_one(&pool)
        .await
        .expect("router mismatch conflict should exist");
        assert_eq!(
            conflict.get::<String, _>("conflict_type"),
            "pppoe_router_mismatch"
        );
        assert!(conflict
            .get::<String, _>("conflict_message")
            .contains("router"));

        drop_test_database(pool, &db_name).await;
    }

    #[tokio::test]
    async fn mixradius_import_executor_can_stage_mixradius_radius_users_as_managed_radius_accounts()
    {
        std::env::set_var("APP_SECRET", "mixradius-test-secret");

        let (pool, db_name) = isolated_pool().await;
        create_package_table(&pool).await;
        create_customer_tables(&pool).await;
        create_subscription_table(&pool).await;
        create_router_and_pppoe_tables(&pool).await;
        seed_test_tenant(&pool, "tenant-executor").await;
        let batch_id = create_ready_batch(&pool, "tenant-executor").await;
        insert_router(&pool, "tenant-executor", "router-radius-1").await;
        sqlx::query(
            r#"
            INSERT INTO public.mixradius_staging_nas (
                id, tenant_id, import_batch_id, source_ref, nas_name, nas_ip_or_cidr, shortname, source_json, created_at, updated_at
            )
            VALUES ($1, $2, $3, 'nas-radius-1', 'Router Radius', '192.0.2.44', 'RTR-RADIUS', '{}'::jsonb, now(), now())
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind("tenant-executor")
        .bind(&batch_id)
        .execute(&pool)
        .await
        .expect("staged nas row should insert");
        insert_staged_customer_with_pppoe_metadata(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-RADIUS-1",
            "radius001",
            "rahasia-radius-1",
            Some("10.44.0.10"),
        )
        .await;
        import_customer_package_and_location_fixture(
            &pool,
            "tenant-executor",
            &batch_id,
            "MBR-RADIUS-1",
            "plan-radius-1",
            "Paket Radius",
            100_000.0,
        )
        .await;

        let executor = MixradiusImportExecutor::new(pool.clone());
        let summary = executor
            .execute_pppoe_imports_with_target(
                "tenant-executor",
                &batch_id,
                &router_override("nas-radius-1", "router-radius-1"),
                MixradiusImportPppoeProvisioningTarget::ManagedRadius,
            )
            .await
            .expect("managed radius pppoe import should stage managed radius account");

        let row = sqlx::query(
            r#"
            SELECT username, router_id, account_source, router_present, is_provisioned, radius_identity
            FROM public.pppoe_accounts
            WHERE tenant_id = $1
            "#,
        )
        .bind("tenant-executor")
        .fetch_one(&pool)
        .await
        .expect("managed radius pppoe account should query");
        assert_eq!(row.get::<String, _>("username"), "radius001");
        assert_eq!(row.get::<String, _>("router_id"), "router-radius-1");
        assert_eq!(row.get::<String, _>("account_source"), "managed_radius");
        assert!(!row.get::<bool, _>("router_present"));
        assert!(!row.get::<bool, _>("is_provisioned"));
        assert_eq!(
            row.get::<Option<String>, _>("radius_identity"),
            Some("radius001".into())
        );
        assert_eq!(summary.imported_rows, 1);
        assert_eq!(summary.conflict_rows, 0);

        drop_test_database(pool, &db_name).await;
    }
}
