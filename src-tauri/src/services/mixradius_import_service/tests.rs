use uuid::Uuid;

const TEST_ADMIN_DATABASE_URL: &str = "postgres://postgres:postgres@127.0.0.1/postgres";
const MIXRADIUS_IMPORT_FOUNDATION_UP_SQL: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/20260411120000_add_mixradius_import_foundation.up.sql"
);
const MIXRADIUS_IMPORT_FOUNDATION_DOWN_SQL: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/20260411120000_add_mixradius_import_foundation.down.sql"
);
const VALIDATED_BACKUP_GZ: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../MixRadiusDB_Gasal_2026-04-11_101103.sql.gz"
);

async fn isolated_pool() -> (sqlx::PgPool, String) {
    let db_name = format!("mixradius_import_schema_{}", Uuid::new_v4().simple());
    let admin_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(TEST_ADMIN_DATABASE_URL)
        .await
        .expect("postgres admin database should be available for migration smoke tests");

    sqlx::query(&format!(r#"CREATE DATABASE "{}""#, db_name))
        .execute(&admin_pool)
        .await
        .expect("temporary migration smoke test database should be creatable");
    admin_pool.close().await;

    let database_url = format!("postgres://postgres:postgres@127.0.0.1/{db_name}");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("temporary migration smoke test database should be connectable");

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
        .expect("migrations should apply for schema smoke test");

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
        .expect("temporary migration smoke test database should be droppable");

    admin_pool.close().await;
}

async fn seed_test_tenant(pool: &sqlx::PgPool, tenant_id: &str) {
    sqlx::query("INSERT INTO public.tenants (id) VALUES ($1)")
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("test tenant should be insertable");

    sqlx::query("INSERT INTO public.users (id) VALUES ($1) ON CONFLICT (id) DO NOTHING")
        .bind("user-stage")
        .execute(pool)
        .await
        .expect("test user should be insertable");
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
        "#,
    )
    .execute(pool)
    .await
    .expect("isp_packages table should be creatable for integration tests");
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
    .expect("customer tables should be creatable for integration tests");
}

async fn create_subscription_tables(pool: &sqlx::PgPool) {
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
            status text NOT NULL DEFAULT 'pending',
            description text,
            due_date timestamp with time zone NOT NULL,
            paid_at timestamp with time zone,
            payment_method text,
            external_id text,
            created_at timestamp with time zone NOT NULL,
            updated_at timestamp with time zone NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await
    .expect("subscription tables should be creatable for integration tests");
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
    .expect("router and pppoe tables should be creatable for integration tests");
}

async fn create_router_table_only(pool: &sqlx::PgPool) {
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
        "#,
    )
    .execute(pool)
    .await
    .expect("router table should be creatable for integration tests");
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
    .expect("ready batch should be insertable");

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
            $1, $2, $3, $4, $5, '15 Mbps', $6, '30 days', 1,
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

#[cfg(test)]
mod mixradius_import_models {
    use crate::models::mixradius_import::{
        MixradiusImportBatchStatus, MixradiusImportConflictState, MixradiusImportExecuteRequest,
        MixradiusImportExecutionMode, MixradiusImportExecutionSummary,
        MixradiusImportPreviewRequest, MixradiusImportUploadRequest,
    };
    use serde_json::json;
    use validator::Validate;

    #[test]
    fn batch_status_serde_contract_uses_snake_case() {
        let serialized = serde_json::to_value(MixradiusImportBatchStatus::Running)
            .expect("batch status should serialize");
        assert_eq!(serialized, json!("running"));

        let deserialized: MixradiusImportBatchStatus =
            serde_json::from_value(json!("completed")).expect("batch status should deserialize");
        assert_eq!(deserialized, MixradiusImportBatchStatus::Completed);

        let partial_success = serde_json::to_value(MixradiusImportBatchStatus::PartialSuccess)
            .expect("partial_success should serialize");
        assert_eq!(partial_success, json!("partial_success"));
    }

    #[test]
    fn preview_row_conflict_state_serde_contract_uses_snake_case() {
        let serialized = serde_json::to_value(MixradiusImportConflictState::NeedsReview)
            .expect("conflict state should serialize");
        assert_eq!(serialized, json!("needs_review"));

        let deserialized: MixradiusImportConflictState =
            serde_json::from_value(json!("blocked")).expect("conflict state should deserialize");
        assert_eq!(deserialized, MixradiusImportConflictState::Blocked);
    }

    #[test]
    fn execution_summary_shape_serializes_expected_fields() {
        let summary = MixradiusImportExecutionSummary {
            batch_id: "batch-123".into(),
            mode: MixradiusImportExecutionMode::SafeImport,
            total_rows: 42,
            imported_rows: 30,
            updated_rows: 4,
            skipped_rows: 6,
            blocked_rows: 2,
            conflict_rows: 3,
            warnings: vec!["router mapping missing".into()],
        };

        assert_eq!(
            serde_json::to_value(summary).expect("summary should serialize"),
            json!({
                "batchId": "batch-123",
                "mode": "safe_import",
                "totalRows": 42,
                "importedRows": 30,
                "updatedRows": 4,
                "skippedRows": 6,
                "blockedRows": 2,
                "conflictRows": 3,
                "warnings": ["router mapping missing"]
            })
        );
    }

    #[test]
    fn request_dto_validation_shape_rejects_empty_required_fields() {
        let upload = MixradiusImportUploadRequest {
            file_name: "   ".into(),
            file_size_bytes: 1,
            content_type: None,
            source_checksum: None,
        };
        assert!(upload.validate().is_err());

        let upload_size = MixradiusImportUploadRequest {
            file_name: "valid.sql.gz".into(),
            file_size_bytes: 0,
            content_type: None,
            source_checksum: None,
        };
        assert!(upload_size.validate().is_err());

        let preview = MixradiusImportPreviewRequest {
            batch_id: "   ".into(),
            mapping_overrides: vec![],
            customer_conflict_resolution: None,
            location_strategy: None,
            pppoe_provisioning_target: None,
        };
        assert!(preview.validate().is_err());

        let preview_override = MixradiusImportPreviewRequest {
            batch_id: "batch-1".into(),
            mapping_overrides: vec![
                crate::models::mixradius_import::MixradiusImportMappingOverride {
                    source_kind: "   ".into(),
                    source_value: "   ".into(),
                    target_kind: "   ".into(),
                    target_value: "   ".into(),
                },
            ],
            customer_conflict_resolution: None,
            location_strategy: None,
            pppoe_provisioning_target: None,
        };
        assert!(preview.validate().is_err());
        assert!(preview_override.validate().is_err());

        let execute = MixradiusImportExecuteRequest {
            batch_id: "   ".into(),
            execution_mode: MixradiusImportExecutionMode::ForceSync,
            mapping_overrides: vec![],
            customer_conflict_resolution: None,
            location_strategy: None,
            pppoe_provisioning_target: None,
        };
        assert!(execute.validate().is_err());

        let execute_override = MixradiusImportExecuteRequest {
            batch_id: "batch-2".into(),
            execution_mode: MixradiusImportExecutionMode::ForceSync,
            mapping_overrides: vec![
                crate::models::mixradius_import::MixradiusImportMappingOverride {
                    source_kind: "   ".into(),
                    source_value: "   ".into(),
                    target_kind: "   ".into(),
                    target_value: "   ".into(),
                },
            ],
            customer_conflict_resolution: None,
            location_strategy: None,
            pppoe_provisioning_target: None,
        };
        assert!(execute.validate().is_err());
        assert!(execute_override.validate().is_err());
    }

    #[test]
    fn path_scoped_preview_and_execute_requests_allow_missing_body_batch_id() {
        let preview: MixradiusImportPreviewRequest = serde_json::from_value(json!({
            "mappingOverrides": []
        }))
        .expect("preview request body should deserialize without batchId because route path supplies it");
        assert_eq!(preview.batch_id, "");
        assert!(preview.validate().is_err());

        let execute: MixradiusImportExecuteRequest = serde_json::from_value(json!({
            "executionMode": "safe_import",
            "mappingOverrides": []
        }))
        .expect("execute request body should deserialize without batchId because route path supplies it");
        assert_eq!(execute.batch_id, "");
        assert!(execute.validate().is_err());
    }

    #[test]
    fn preview_and_execute_requests_accept_snake_case_mapping_override_fields() {
        let preview: MixradiusImportPreviewRequest = serde_json::from_value(json!({
            "mappingOverrides": [
                {
                    "source_kind": "nas",
                    "source_value": "5",
                    "target_kind": "router",
                    "target_value": "router-1"
                }
            ]
        }))
        .expect("preview request should accept snake_case mapping override fields");
        assert_eq!(preview.mapping_overrides.len(), 1);
        assert_eq!(preview.mapping_overrides[0].source_kind, "nas");
        assert_eq!(preview.mapping_overrides[0].target_value, "router-1");

        let execute: MixradiusImportExecuteRequest = serde_json::from_value(json!({
            "executionMode": "safe_import",
            "mappingOverrides": [
                {
                    "source_kind": "plan",
                    "source_value": "10Mbps",
                    "target_kind": "package",
                    "target_value": "package-1"
                }
            ]
        }))
        .expect("execute request should accept snake_case mapping override fields");
        assert_eq!(execute.mapping_overrides.len(), 1);
        assert_eq!(execute.mapping_overrides[0].source_kind, "plan");
        assert_eq!(execute.mapping_overrides[0].target_value, "package-1");
    }
}

async fn assert_table_exists(pool: &sqlx::PgPool, table_name: &str) {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = 'public'
              AND table_name = $1
        )
        "#,
    )
    .bind(table_name)
    .fetch_one(pool)
    .await
    .expect("table existence query should succeed");

    assert!(exists, "expected table `{table_name}` to exist");
}

async fn assert_table_missing(pool: &sqlx::PgPool, table_name: &str) {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = 'public'
              AND table_name = $1
        )
        "#,
    )
    .bind(table_name)
    .fetch_one(pool)
    .await
    .expect("table existence query should succeed");

    assert!(!exists, "expected table `{table_name}` to be dropped");
}

async fn assert_column_exists(pool: &sqlx::PgPool, table_name: &str, column_name: &str) {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = $1
              AND column_name = $2
        )
        "#,
    )
    .bind(table_name)
    .bind(column_name)
    .fetch_one(pool)
    .await
    .expect("column existence query should succeed");

    assert!(
        exists,
        "expected column `{column_name}` on table `{table_name}` to exist"
    );
}

async fn assert_index_exists(pool: &sqlx::PgPool, index_name: &str) {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM pg_indexes
            WHERE schemaname = 'public'
              AND indexname = $1
        )
        "#,
    )
    .bind(index_name)
    .fetch_one(pool)
    .await
    .expect("index existence query should succeed");

    assert!(exists, "expected index `{index_name}` to exist");
}

#[tokio::test]
async fn mixradius_import_schema() {
    let (pool, db_name) = isolated_pool().await;

    for table_name in [
        "mixradius_import_batches",
        "mixradius_import_external_refs",
        "mixradius_staging_nas",
        "mixradius_staging_plans",
        "mixradius_staging_customers",
        "mixradius_staging_customer_locations",
        "mixradius_staging_transactions",
        "mixradius_staging_usage",
        "mixradius_import_conflicts",
    ] {
        assert_table_exists(&pool, table_name).await;
    }

    for column_name in [
        "parse_status",
        "execution_status",
        "execution_mode",
        "progress_json",
        "summary_json",
        "error_json",
    ] {
        assert_column_exists(&pool, "mixradius_import_batches", column_name).await;
    }

    assert_column_exists(&pool, "mixradius_staging_usage", "usage_date").await;
    assert_column_exists(&pool, "mixradius_staging_usage", "download_bytes").await;
    assert_column_exists(&pool, "mixradius_import_conflicts", "resolution_status").await;

    for index_name in [
        "idx_mixradius_import_batches_tenant_status",
        "idx_mixradius_staging_usage_member_date",
        "idx_mixradius_import_conflicts_source",
    ] {
        assert_index_exists(&pool, index_name).await;
    }

    sqlx::query(
        r#"
        INSERT INTO public.tenants (id)
        VALUES ('tenant-1')
        "#,
    )
    .execute(&pool)
    .await
    .expect("test tenant should be insertable");

    let default_execution_mode: String = sqlx::query_scalar(
        r#"
        INSERT INTO public.mixradius_import_batches (
            id,
            tenant_id,
            source_filename,
            source_sha256,
            source_size_bytes,
            created_at,
            updated_at
        )
        VALUES (
            'batch-1',
            'tenant-1',
            'MixRadius.sql.gz',
            'sha256',
            128,
            now(),
            now()
        )
        RETURNING execution_mode
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("batch defaults should satisfy schema constraints");
    assert_eq!(default_execution_mode, "preview_only");

    let explicit_safe_import: String = sqlx::query_scalar(
        r#"
        INSERT INTO public.mixradius_import_batches (
            id,
            tenant_id,
            source_filename,
            source_sha256,
            source_size_bytes,
            execution_mode,
            created_at,
            updated_at
        )
        VALUES (
            'batch-2',
            'tenant-1',
            'MixRadius-2.sql.gz',
            'sha256-2',
            256,
            'safe_import',
            now(),
            now()
        )
        RETURNING execution_mode
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("safe_import execution mode should satisfy schema constraints");
    assert_eq!(explicit_safe_import, "safe_import");

    let explicit_partial_success_status: String = sqlx::query_scalar(
        r#"
        INSERT INTO public.mixradius_import_batches (
            id,
            tenant_id,
            source_filename,
            source_sha256,
            source_size_bytes,
            execution_status,
            created_at,
            updated_at
        )
        VALUES (
            'batch-partial-success',
            'tenant-1',
            'MixRadius-partial.sql.gz',
            'sha256-partial',
            512,
            'partial_success',
            now(),
            now()
        )
        RETURNING execution_status
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("partial_success execution status should satisfy schema constraints");
    assert_eq!(explicit_partial_success_status, "partial_success");

    let legacy_execution_mode_result = sqlx::query(
        r#"
        INSERT INTO public.mixradius_import_batches (
            id,
            tenant_id,
            source_filename,
            source_sha256,
            source_size_bytes,
            execution_mode,
            created_at,
            updated_at
        )
        VALUES (
            'batch-legacy',
            'tenant-1',
            'legacy.sql.gz',
            'legacy-sha',
            512,
            'preview',
            now(),
            now()
        )
        "#,
    )
    .execute(&pool)
    .await;
    assert!(
        legacy_execution_mode_result.is_err(),
        "legacy execution mode should be rejected by schema constraints"
    );

    let blank_filename_result = sqlx::query(
        r#"
        INSERT INTO public.mixradius_import_batches (
            id,
            tenant_id,
            source_filename,
            source_sha256,
            source_size_bytes,
            created_at,
            updated_at
        )
        VALUES (
            'batch-blank',
            'tenant-1',
            '   ',
            'sha256-blank',
            10,
            now(),
            now()
        )
        "#,
    )
    .execute(&pool)
    .await;
    assert!(
        blank_filename_result.is_err(),
        "blank source_filename should be rejected by schema constraints"
    );

    let blank_sha256_result = sqlx::query(
        r#"
        INSERT INTO public.mixradius_import_batches (
            id,
            tenant_id,
            source_filename,
            source_sha256,
            source_size_bytes,
            created_at,
            updated_at
        )
        VALUES (
            'batch-blank-sha',
            'tenant-1',
            'blank-sha.sql.gz',
            '   ',
            10,
            now(),
            now()
        )
        "#,
    )
    .execute(&pool)
    .await;
    assert!(
        blank_sha256_result.is_err(),
        "blank source_sha256 should be rejected by schema constraints"
    );

    let zero_size_result = sqlx::query(
        r#"
        INSERT INTO public.mixradius_import_batches (
            id,
            tenant_id,
            source_filename,
            source_sha256,
            source_size_bytes,
            created_at,
            updated_at
        )
        VALUES (
            'batch-zero',
            'tenant-1',
            'zero.sql.gz',
            'sha256-zero',
            0,
            now(),
            now()
        )
        "#,
    )
    .execute(&pool)
    .await;
    assert!(
        zero_size_result.is_err(),
        "non-positive source_size_bytes should be rejected by schema constraints"
    );

    let down_sql = std::fs::read_to_string(MIXRADIUS_IMPORT_FOUNDATION_DOWN_SQL)
        .expect("mixradius import down migration should be readable");
    sqlx::raw_sql(&down_sql)
        .execute(&pool)
        .await
        .expect("down migration should rollback mixradius import schema");

    for table_name in [
        "mixradius_import_conflicts",
        "mixradius_staging_usage",
        "mixradius_staging_transactions",
        "mixradius_staging_customer_locations",
        "mixradius_staging_customers",
        "mixradius_staging_plans",
        "mixradius_staging_nas",
        "mixradius_import_external_refs",
        "mixradius_import_batches",
    ] {
        assert_table_missing(&pool, table_name).await;
    }

    drop_test_database(pool, &db_name).await;
}

#[tokio::test]
async fn mixradius_import_stage_registers_batch_and_stages_counts() {
    let (pool, db_name) = isolated_pool().await;
    seed_test_tenant(&pool, "tenant-stage").await;

    let service = super::MixradiusImportService::new(pool.clone());
    let batch = service
        .stage_backup(
            "tenant-stage",
            Some("user-stage"),
            std::path::Path::new(VALIDATED_BACKUP_GZ),
        )
        .await
        .expect("validated MixRadius backup should stage");

    let batch_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.mixradius_import_batches WHERE tenant_id = $1",
    )
    .bind("tenant-stage")
    .fetch_one(&pool)
    .await
    .expect("batch count should query");
    assert_eq!(batch_count, 1);
    assert_eq!(
        batch.parse_status,
        crate::models::MixradiusImportParseStatus::Ready
    );
    assert_eq!(
        batch.execution_status,
        crate::models::MixradiusImportBatchStatus::Pending
    );

    let customer_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.mixradius_staging_customers WHERE import_batch_id = $1",
    )
    .bind(&batch.id)
    .fetch_one(&pool)
    .await
    .expect("customer staging count should query");
    assert_eq!(customer_count, 545);

    let plan_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.mixradius_staging_plans WHERE import_batch_id = $1",
    )
    .bind(&batch.id)
    .fetch_one(&pool)
    .await
    .expect("plan staging count should query");
    assert_eq!(plan_count, 15);

    let nas_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.mixradius_staging_nas WHERE import_batch_id = $1",
    )
    .bind(&batch.id)
    .fetch_one(&pool)
    .await
    .expect("nas staging count should query");
    assert_eq!(nas_count, 2);

    let orphan_location_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM public.mixradius_staging_customer_locations l
        LEFT JOIN public.mixradius_staging_customers c
          ON c.import_batch_id = l.import_batch_id
         AND c.member_id = l.member_id
        WHERE l.import_batch_id = $1
          AND c.id IS NULL
        "#,
    )
    .bind(&batch.id)
    .fetch_one(&pool)
    .await
    .expect("location join integrity should query");
    assert_eq!(orphan_location_count, 0);

    let staged_nas_name: String = sqlx::query_scalar(
        "SELECT nas_name FROM public.mixradius_staging_nas WHERE source_ref = '5' AND import_batch_id = $1",
    )
    .bind(&batch.id)
    .fetch_one(&pool)
    .await
    .expect("staged NAS name should query");
    assert_eq!(staged_nas_name, "Deres");

    let usage_directional_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM public.mixradius_staging_usage
        WHERE import_batch_id = $1
          AND (download_bytes IS NOT NULL OR upload_bytes IS NOT NULL)
        "#,
    )
    .bind(&batch.id)
    .fetch_one(&pool)
    .await
    .expect("usage directional byte count should query");
    assert_eq!(usage_directional_count, 0);

    let summary: serde_json::Value = sqlx::query_scalar(
        "SELECT summary_json FROM public.mixradius_import_batches WHERE id = $1",
    )
    .bind(&batch.id)
    .fetch_one(&pool)
    .await
    .expect("batch summary should query");
    assert_eq!(summary["customersTotal"], 545);
    assert_eq!(summary["customersPpp"], 543);
    assert_eq!(summary["plansPpp"], 12);
    assert_eq!(summary["nas"], 2);

    drop_test_database(pool, &db_name).await;
}

#[tokio::test]
async fn mixradius_import_authorization_scopes_batches_to_their_tenant() {
    let (pool, db_name) = isolated_pool().await;
    seed_test_tenant(&pool, "tenant-a").await;
    seed_test_tenant(&pool, "tenant-b").await;

    let service = super::MixradiusImportService::new(pool.clone());
    let batch = service
        .stage_backup(
            "tenant-a",
            Some("user-stage"),
            std::path::Path::new(VALIDATED_BACKUP_GZ),
        )
        .await
        .expect("validated MixRadius backup should stage");

    let wrong_tenant_get = service.get_batch("tenant-b", &batch.id).await;
    assert!(
        wrong_tenant_get.is_err(),
        "tenant-b must not be able to read tenant-a batch"
    );

    let wrong_tenant_preview = service
        .build_preview(
            "tenant-b",
            &crate::models::MixradiusImportPreviewRequest {
                batch_id: batch.id.clone(),
                mapping_overrides: vec![],
                customer_conflict_resolution: None,
                location_strategy: None,
                pppoe_provisioning_target: None,
            },
        )
        .await;
    assert!(
        wrong_tenant_preview.is_err(),
        "tenant-b must not be able to preview tenant-a batch"
    );

    let wrong_tenant_execute = service
        .execute_preview(
            "tenant-b",
            &crate::models::MixradiusImportExecuteRequest {
                batch_id: batch.id.clone(),
                execution_mode: crate::models::MixradiusImportExecutionMode::SafeImport,
                mapping_overrides: vec![],
                customer_conflict_resolution: None,
                location_strategy: None,
                pppoe_provisioning_target: None,
            },
        )
        .await;
    assert!(
        wrong_tenant_execute.is_err(),
        "tenant-b must not be able to execute tenant-a batch"
    );

    let wrong_tenant_cancel = service.cancel_batch("tenant-b", &batch.id).await;
    assert!(
        wrong_tenant_cancel.is_err(),
        "tenant-b must not be able to cancel tenant-a batch"
    );

    drop_test_database(pool, &db_name).await;
}

#[tokio::test]
async fn mixradius_import_authorization_cancel_marks_pending_batch_cancelled() {
    let (pool, db_name) = isolated_pool().await;
    seed_test_tenant(&pool, "tenant-stage").await;

    let service = super::MixradiusImportService::new(pool.clone());
    let batch = service
        .stage_backup(
            "tenant-stage",
            Some("user-stage"),
            std::path::Path::new(VALIDATED_BACKUP_GZ),
        )
        .await
        .expect("validated MixRadius backup should stage");

    let cancelled = service
        .cancel_batch("tenant-stage", &batch.id)
        .await
        .expect("pending MixRadius batch should be cancellable");

    assert_eq!(
        cancelled.execution_status,
        crate::models::MixradiusImportBatchStatus::Cancelled
    );
    assert_eq!(cancelled.progress_json["stage"], "cancelled");

    drop_test_database(pool, &db_name).await;
}

#[tokio::test]
async fn mixradius_import_overrides_preview_and_execute_reuse_submitted_decisions() {
    let (pool, db_name) = isolated_pool().await;
    seed_test_tenant(&pool, "tenant-stage").await;
    create_package_table(&pool).await;
    create_customer_tables(&pool).await;
    create_subscription_tables(&pool).await;
    insert_existing_package(
        &pool,
        "tenant-stage",
        "package-override-1",
        "Package Override 1",
        150_000.0,
    )
    .await;

    let service = super::MixradiusImportService::new(pool.clone());
    let batch = service
        .stage_backup(
            "tenant-stage",
            Some("user-stage"),
            std::path::Path::new(VALIDATED_BACKUP_GZ),
        )
        .await
        .expect("validated MixRadius backup should stage");

    let preview_request = crate::models::MixradiusImportPreviewRequest {
        batch_id: batch.id.clone(),
        mapping_overrides: vec![
            crate::models::mixradius_import::MixradiusImportMappingOverride {
                source_kind: "nas".into(),
                source_value: "5".into(),
                target_kind: "router".into(),
                target_value: "router-override-1".into(),
            },
            crate::models::mixradius_import::MixradiusImportMappingOverride {
                source_kind: "plan".into(),
                source_value: "10".into(),
                target_kind: "package".into(),
                target_value: "package-override-1".into(),
            },
        ],
        customer_conflict_resolution: Some(
            crate::models::MixradiusImportCustomerConflictResolution::Skip,
        ),
        location_strategy: Some(crate::models::MixradiusImportLocationStrategy::Replace),
        pppoe_provisioning_target: None,
    };

    let preview = service
        .build_preview("tenant-stage", &preview_request)
        .await
        .expect("preview with overrides should build");

    let nas_row = preview
        .rows
        .iter()
        .find(|row| row.source_kind == "nas" && row.source_ref == "5")
        .expect("NAS override row should exist");
    assert_eq!(
        nas_row.conflict_state,
        crate::models::MixradiusImportConflictState::AutoMatched
    );
    assert_eq!(nas_row.target_id.as_deref(), Some("router-override-1"));

    let plan_row = preview
        .rows
        .iter()
        .find(|row| row.source_kind == "plan" && row.source_ref == "10")
        .expect("plan override row should exist");
    assert_eq!(
        plan_row.conflict_state,
        crate::models::MixradiusImportConflictState::AutoMatched
    );
    assert_eq!(plan_row.target_id.as_deref(), Some("package-override-1"));

    let customer_row = preview
        .rows
        .iter()
        .find(|row| row.source_kind == "customer")
        .expect("customer preview row should exist");
    assert_eq!(
        customer_row.conflict_state,
        crate::models::MixradiusImportConflictState::Skipped
    );
    assert!(customer_row
        .notes
        .as_deref()
        .unwrap_or_default()
        .contains("replace"));

    let persisted_preview_progress: serde_json::Value = sqlx::query_scalar(
        "SELECT progress_json FROM public.mixradius_import_batches WHERE id = $1",
    )
    .bind(&batch.id)
    .fetch_one(&pool)
    .await
    .expect("persisted progress json should query");
    assert_eq!(
        persisted_preview_progress["previewRequest"]["customerConflictResolution"],
        "skip"
    );
    assert_eq!(
        persisted_preview_progress["previewRequest"]["locationStrategy"],
        "replace"
    );

    let execute = service
        .execute_preview(
            "tenant-stage",
            &crate::models::MixradiusImportExecuteRequest {
                batch_id: batch.id.clone(),
                execution_mode: crate::models::MixradiusImportExecutionMode::SafeImport,
                mapping_overrides: preview_request.mapping_overrides.clone(),
                customer_conflict_resolution: preview_request.customer_conflict_resolution,
                location_strategy: preview_request.location_strategy,
                pppoe_provisioning_target: None,
            },
        )
        .await
        .expect("execute preview should reuse submitted overrides");

    let execute_preview = execute
        .preview
        .expect("execute should return preview snapshot");
    let execute_nas_row = execute_preview
        .rows
        .iter()
        .find(|row| row.source_kind == "nas" && row.source_ref == "5")
        .expect("execute preview NAS row should exist");
    assert_eq!(
        execute_nas_row.target_id.as_deref(),
        Some("router-override-1")
    );

    let persisted_execute_progress: serde_json::Value = sqlx::query_scalar(
        "SELECT progress_json FROM public.mixradius_import_batches WHERE id = $1",
    )
    .bind(&batch.id)
    .fetch_one(&pool)
    .await
    .expect("persisted execute progress json should query");
    assert_eq!(
        persisted_execute_progress["executeRequest"]["executionMode"],
        "safe_import"
    );
    assert_eq!(
        persisted_execute_progress["executeRequest"]["mappingOverrides"][0]["targetValue"],
        "router-override-1"
    );

    drop_test_database(pool, &db_name).await;
}

#[tokio::test]
async fn mixradius_import_execute_safe_import_runs_package_executor_and_updates_batch() {
    let (pool, db_name) = isolated_pool().await;
    seed_test_tenant(&pool, "tenant-stage").await;
    create_package_table(&pool).await;
    let batch_id = create_ready_batch(&pool, "tenant-stage").await;
    insert_staged_plan(
        &pool,
        "tenant-stage",
        &batch_id,
        "plan-99",
        "Paket Mix 99 Mbps",
        499_000.0,
    )
    .await;

    let service = super::MixradiusImportService::new(pool.clone());
    let result = service
        .execute_preview(
            "tenant-stage",
            &crate::models::MixradiusImportExecuteRequest {
                batch_id: batch_id.clone(),
                execution_mode: crate::models::MixradiusImportExecutionMode::SafeImport,
                mapping_overrides: vec![],
                customer_conflict_resolution: None,
                location_strategy: None,
                pppoe_provisioning_target: None,
            },
        )
        .await
        .expect("safe import should execute package import");

    assert_eq!(result.summary.imported_rows, 1);
    assert_eq!(result.summary.updated_rows, 0);
    assert_eq!(result.summary.conflict_rows, 0);

    let package_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.isp_packages WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("package count should query");
    assert_eq!(package_count, 1);

    let persisted_batch = service
        .get_batch("tenant-stage", &batch_id)
        .await
        .expect("executed batch should reload");
    assert_eq!(
        persisted_batch.execution_status,
        crate::models::MixradiusImportBatchStatus::Completed
    );
    assert_eq!(
        persisted_batch.execution_mode,
        crate::models::MixradiusImportExecutionMode::SafeImport
    );
    assert_eq!(persisted_batch.summary_json["importedRows"], 1);
    assert_eq!(
        persisted_batch.progress_json["stage"],
        "packages_imported_partial"
    );

    drop_test_database(pool, &db_name).await;
}

#[tokio::test]
async fn mixradius_import_execute_safe_import_runs_customer_executor_and_updates_batch() {
    std::env::set_var("APP_SECRET", "mixradius-test-secret");

    let (pool, db_name) = isolated_pool().await;
    seed_test_tenant(&pool, "tenant-stage").await;
    create_package_table(&pool).await;
    create_customer_tables(&pool).await;
    create_subscription_tables(&pool).await;
    create_router_and_pppoe_tables(&pool).await;
    let batch_id = create_ready_batch(&pool, "tenant-stage").await;
    insert_router(&pool, "tenant-stage", "router-stage-88").await;
    sqlx::query(
        r#"
        INSERT INTO public.mixradius_staging_nas (
            id, tenant_id, import_batch_id, source_ref, nas_name, nas_ip_or_cidr, shortname, source_json, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'nas-stage-88', 'Router Stage 88', '192.0.2.88', 'RTR88', '{}'::jsonb, now(), now())
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind("tenant-stage")
    .bind(&batch_id)
    .execute(&pool)
    .await
    .expect("staged nas row should insert");
    insert_staged_plan(
        &pool,
        "tenant-stage",
        &batch_id,
        "plan-customer-88",
        "Paket Mix 88 Mbps",
        388_000.0,
    )
    .await;
    insert_staged_customer(
        &pool,
        "tenant-stage",
        &batch_id,
        "row-customer-88",
        "MBR-88",
        "cust088",
        "Nurhayati",
        "nurhayati@example.test",
        "0812888888",
        "Jl. Veteran 88",
    )
    .await;
    sqlx::query(
        r#"
        UPDATE public.mixradius_staging_customers
        SET plan_name = 'Paket Mix 88 Mbps',
            price = 388000,
            password = 'pppoe-stage-88',
            renewed_on = '2026-04-01 00:00:00+00'::timestamptz,
            expired_on = '2026-05-01 00:00:00+00'::timestamptz,
            trx_status = 'PAID',
            source_json = jsonb_build_object(
                'memberId', 'MBR-88',
                'username', 'cust088',
                'radreply', jsonb_build_array(
                    jsonb_build_object('attribute', 'Framed-IP-Address', 'value', '10.88.0.2')
                )
            )
        WHERE tenant_id = $1 AND import_batch_id = $2 AND member_id = 'MBR-88'
        "#,
    )
    .bind("tenant-stage")
    .bind(&batch_id)
    .execute(&pool)
    .await
    .expect("staged customer lifecycle fields should update");

    let service = super::MixradiusImportService::new(pool.clone());
    let result = service
        .execute_preview(
            "tenant-stage",
            &crate::models::MixradiusImportExecuteRequest {
                batch_id: batch_id.clone(),
                execution_mode: crate::models::MixradiusImportExecutionMode::SafeImport,
                mapping_overrides: vec![crate::models::MixradiusImportMappingOverride {
                    source_kind: "nas".to_string(),
                    source_value: "nas-stage-88".to_string(),
                    target_kind: "router".to_string(),
                    target_value: "router-stage-88".to_string(),
                }],
                customer_conflict_resolution: None,
                location_strategy: None,
                pppoe_provisioning_target: None,
            },
        )
        .await
        .expect("safe import should execute customer import");

    let customer_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.customers WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("customer count should query");
    let location_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.customer_locations WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("location count should query");
    let package_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.isp_packages WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("package count should query");
    let subscription_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.customer_subscriptions WHERE tenant_id = $1",
    )
    .bind("tenant-stage")
    .fetch_one(&pool)
    .await
    .expect("subscription count should query");
    let pppoe_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.pppoe_accounts WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("pppoe count should query");
    assert_eq!(package_count, 1);
    assert_eq!(customer_count, 1);
    assert_eq!(location_count, 1);
    assert_eq!(subscription_count, 1);
    assert_eq!(pppoe_count, 1);
    assert_eq!(result.summary.imported_rows, 5);
    assert_eq!(result.summary.updated_rows, 0);
    assert!(result
        .summary
        .warnings
        .iter()
        .any(|warning| warning.contains("PPPoE")));

    let persisted_batch = service
        .get_batch("tenant-stage", &batch_id)
        .await
        .expect("executed batch should reload");
    assert_eq!(persisted_batch.summary_json["importedRows"], 5);
    assert_eq!(
        persisted_batch.progress_json["stage"],
        "pppoe_imported_partial"
    );

    drop_test_database(pool, &db_name).await;
}

#[tokio::test]
async fn mixradius_import_execution_modes_preview_only_never_writes_production_data() {
    let (pool, db_name) = isolated_pool().await;
    seed_test_tenant(&pool, "tenant-stage").await;
    create_package_table(&pool).await;
    create_customer_tables(&pool).await;
    create_subscription_tables(&pool).await;
    let batch_id = create_ready_batch(&pool, "tenant-stage").await;
    insert_staged_plan(
        &pool,
        "tenant-stage",
        &batch_id,
        "plan-preview-only",
        "Paket Preview Only",
        123_000.0,
    )
    .await;
    insert_staged_customer(
        &pool,
        "tenant-stage",
        &batch_id,
        "row-preview-only",
        "MBR-PREVIEW",
        "preview001",
        "Preview Customer",
        "preview@example.test",
        "080000000",
        "Jl. Preview",
    )
    .await;

    let service = super::MixradiusImportService::new(pool.clone());
    let result = service
        .execute_preview(
            "tenant-stage",
            &crate::models::MixradiusImportExecuteRequest {
                batch_id: batch_id.clone(),
                execution_mode: crate::models::MixradiusImportExecutionMode::PreviewOnly,
                mapping_overrides: vec![],
                customer_conflict_resolution: None,
                location_strategy: None,
                pppoe_provisioning_target: None,
            },
        )
        .await
        .expect("preview_only should build preview without production writes");

    assert_eq!(result.summary.imported_rows, 0);
    assert_eq!(result.summary.updated_rows, 0);
    let package_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.isp_packages WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("package count should query");
    let customer_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.customers WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("customer count should query");
    assert_eq!(package_count, 0);
    assert_eq!(customer_count, 0);

    let batch = service
        .get_batch("tenant-stage", &batch_id)
        .await
        .expect("batch should reload");
    assert_eq!(
        batch.execution_status,
        crate::models::MixradiusImportBatchStatus::Pending
    );

    drop_test_database(pool, &db_name).await;
}

#[tokio::test]
async fn mixradius_import_execution_modes_safe_import_skips_package_conflicts() {
    let (pool, db_name) = isolated_pool().await;
    seed_test_tenant(&pool, "tenant-stage").await;
    create_package_table(&pool).await;
    let batch_id = create_ready_batch(&pool, "tenant-stage").await;
    insert_existing_package(
        &pool,
        "tenant-stage",
        "pkg-safe-conflict",
        "Paket Conflict",
        100_000.0,
    )
    .await;
    insert_staged_plan(
        &pool,
        "tenant-stage",
        &batch_id,
        "plan-safe-conflict",
        "Paket Conflict",
        200_000.0,
    )
    .await;

    let service = super::MixradiusImportService::new(pool.clone());
    let result = service
        .execute_preview(
            "tenant-stage",
            &crate::models::MixradiusImportExecuteRequest {
                batch_id: batch_id.clone(),
                execution_mode: crate::models::MixradiusImportExecutionMode::SafeImport,
                mapping_overrides: vec![],
                customer_conflict_resolution: None,
                location_strategy: None,
                pppoe_provisioning_target: None,
            },
        )
        .await
        .expect("safe_import should record package conflict without overwriting");

    let price: f64 = sqlx::query_scalar(
        "SELECT price_monthly::float8 FROM public.isp_packages WHERE id = 'pkg-safe-conflict'",
    )
    .fetch_one(&pool)
    .await
    .expect("package price should query");
    assert_eq!(price, 100_000.0);
    assert_eq!(result.summary.conflict_rows, 1);
    assert_eq!(result.summary.imported_rows, 0);

    drop_test_database(pool, &db_name).await;
}

#[tokio::test]
async fn mixradius_import_execution_modes_force_sync_overwrites_matching_package_price() {
    let (pool, db_name) = isolated_pool().await;
    seed_test_tenant(&pool, "tenant-stage").await;
    create_package_table(&pool).await;
    let batch_id = create_ready_batch(&pool, "tenant-stage").await;
    insert_existing_package(
        &pool,
        "tenant-stage",
        "pkg-force-conflict",
        "Paket Force",
        100_000.0,
    )
    .await;
    insert_staged_plan(
        &pool,
        "tenant-stage",
        &batch_id,
        "plan-force-conflict",
        "Paket Force",
        225_000.0,
    )
    .await;

    let service = super::MixradiusImportService::new(pool.clone());
    let result = service
        .execute_preview(
            "tenant-stage",
            &crate::models::MixradiusImportExecuteRequest {
                batch_id: batch_id.clone(),
                execution_mode: crate::models::MixradiusImportExecutionMode::ForceSync,
                mapping_overrides: vec![],
                customer_conflict_resolution: None,
                location_strategy: None,
                pppoe_provisioning_target: None,
            },
        )
        .await
        .expect("force_sync should overwrite allowed package fields");

    let price: f64 = sqlx::query_scalar(
        "SELECT price_monthly::float8 FROM public.isp_packages WHERE id = 'pkg-force-conflict'",
    )
    .fetch_one(&pool)
    .await
    .expect("package price should query");
    assert_eq!(price, 225_000.0);
    assert_eq!(result.summary.conflict_rows, 0);
    assert_eq!(result.summary.updated_rows, 1);

    drop_test_database(pool, &db_name).await;
}

#[tokio::test]
async fn mixradius_import_reports_records_partial_success_after_later_phase_failure() {
    std::env::set_var("APP_SECRET", "mixradius-test-secret");

    let (pool, db_name) = isolated_pool().await;
    seed_test_tenant(&pool, "tenant-stage").await;
    create_package_table(&pool).await;
    create_customer_tables(&pool).await;
    create_subscription_tables(&pool).await;
    create_router_table_only(&pool).await;
    let batch_id = create_ready_batch(&pool, "tenant-stage").await;
    insert_router(&pool, "tenant-stage", "router-partial").await;
    sqlx::query(
        r#"
        INSERT INTO public.mixradius_staging_nas (
            id, tenant_id, import_batch_id, source_ref, nas_name, nas_ip_or_cidr, shortname, source_json, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'nas-partial', 'Router Partial', '192.0.2.44', 'RTR44', '{}'::jsonb, now(), now())
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind("tenant-stage")
    .bind(&batch_id)
    .execute(&pool)
    .await
    .expect("staged nas should insert");
    insert_staged_plan(
        &pool,
        "tenant-stage",
        &batch_id,
        "plan-partial",
        "Paket Partial",
        144_000.0,
    )
    .await;
    insert_staged_customer(
        &pool,
        "tenant-stage",
        &batch_id,
        "row-partial",
        "MBR-PARTIAL",
        "partial001",
        "Partial Customer",
        "partial@example.test",
        "0812444",
        "Jl. Partial",
    )
    .await;
    sqlx::query(
        r#"
        UPDATE public.mixradius_staging_customers
        SET plan_name = 'Paket Partial',
            price = 144000,
            password = 'partial-secret',
            renewed_on = '2026-04-01 00:00:00+00'::timestamptz,
            expired_on = '2026-05-01 00:00:00+00'::timestamptz,
            trx_status = 'PAID'
        WHERE tenant_id = $1 AND import_batch_id = $2 AND member_id = 'MBR-PARTIAL'
        "#,
    )
    .bind("tenant-stage")
    .bind(&batch_id)
    .execute(&pool)
    .await
    .expect("staged customer should update");

    let service = super::MixradiusImportService::new(pool.clone());
    let result = service
        .execute_preview(
            "tenant-stage",
            &crate::models::MixradiusImportExecuteRequest {
                batch_id: batch_id.clone(),
                execution_mode: crate::models::MixradiusImportExecutionMode::SafeImport,
                mapping_overrides: vec![crate::models::MixradiusImportMappingOverride {
                    source_kind: "nas".to_string(),
                    source_value: "nas-partial".to_string(),
                    target_kind: "router".to_string(),
                    target_value: "router-partial".to_string(),
                }],
                customer_conflict_resolution: None,
                location_strategy: None,
                pppoe_provisioning_target: None,
            },
        )
        .await
        .expect("later PPPoE failure should return partial success report");

    assert_eq!(
        result.batch.execution_status,
        crate::models::MixradiusImportBatchStatus::PartialSuccess
    );

    let package_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.isp_packages WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("package count should query");
    let customer_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.customers WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("customer count should query");
    let subscription_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.customer_subscriptions WHERE tenant_id = $1",
    )
    .bind("tenant-stage")
    .fetch_one(&pool)
    .await
    .expect("subscription count should query");
    assert_eq!(package_count, 1);
    assert_eq!(customer_count, 1);
    assert_eq!(subscription_count, 1);

    let persisted = service
        .get_batch("tenant-stage", &batch_id)
        .await
        .expect("partial success batch should reload");
    assert_eq!(
        persisted.execution_status,
        crate::models::MixradiusImportBatchStatus::PartialSuccess
    );
    assert_eq!(persisted.progress_json["stage"], "pppoe_failed_partial");
    assert_eq!(
        persisted.summary_json["phaseReports"]["packages"]["status"],
        "completed"
    );
    assert_eq!(
        persisted.summary_json["phaseReports"]["pppoe"]["status"],
        "failed"
    );
    assert!(persisted.summary_json["errors"][0]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("pppoe_accounts"));

    drop_test_database(pool, &db_name).await;
}

#[tokio::test]
async fn mixradius_import_end_to_end_preview_counts_only_ppp_domains_from_validated_backup() {
    let (pool, db_name) = isolated_pool().await;
    seed_test_tenant(&pool, "tenant-stage").await;

    let service = super::MixradiusImportService::new(pool.clone());
    let batch = service
        .stage_backup(
            "tenant-stage",
            Some("user-stage"),
            std::path::Path::new(VALIDATED_BACKUP_GZ),
        )
        .await
        .expect("validated MixRadius backup should stage");

    let preview = service
        .build_preview(
            "tenant-stage",
            &crate::models::MixradiusImportPreviewRequest {
                batch_id: batch.id.clone(),
                mapping_overrides: vec![],
                customer_conflict_resolution: None,
                location_strategy: None,
                pppoe_provisioning_target: None,
            },
        )
        .await
        .expect("validated MixRadius backup should preview");

    let customer_rows = preview
        .rows
        .iter()
        .filter(|row| row.source_kind == "customer")
        .count();
    let plan_rows = preview
        .rows
        .iter()
        .filter(|row| row.source_kind == "plan")
        .count();
    let nas_rows = preview
        .rows
        .iter()
        .filter(|row| row.source_kind == "nas")
        .count();

    assert_eq!(customer_rows, 543);
    assert_eq!(plan_rows, 12);
    assert_eq!(nas_rows, 2);
    assert_eq!(preview.total_rows, 557);

    drop_test_database(pool, &db_name).await;
}

#[tokio::test]
async fn mixradius_import_end_to_end_safe_execute_is_idempotent_and_keeps_legacy_billing_history_only(
) {
    let (pool, _db_name) = isolated_pool().await;
    seed_test_tenant(&pool, "tenant-stage").await;
    seed_test_tenant(&pool, "tenant-b").await;
    create_package_table(&pool).await;
    create_customer_tables(&pool).await;
    create_subscription_tables(&pool).await;

    let service = super::MixradiusImportService::new(pool.clone());
    let batch = service
        .stage_backup(
            "tenant-stage",
            Some("user-stage"),
            std::path::Path::new(VALIDATED_BACKUP_GZ),
        )
        .await
        .expect("validated MixRadius backup should stage");

    let first = service
        .execute_preview(
            "tenant-stage",
            &crate::models::MixradiusImportExecuteRequest {
                batch_id: batch.id.clone(),
                execution_mode: crate::models::MixradiusImportExecutionMode::SafeImport,
                mapping_overrides: vec![],
                customer_conflict_resolution: None,
                location_strategy: None,
                pppoe_provisioning_target: None,
            },
        )
        .await
        .expect("first safe execute should succeed");

    let package_count_after_first: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.isp_packages WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("package count should query");
    let customer_count_after_first: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.customers WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("customer count should query");
    let location_count_after_first: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.customer_locations WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("location count should query");
    let subscription_count_after_first: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.customer_subscriptions WHERE tenant_id = $1",
    )
    .bind("tenant-stage")
    .fetch_one(&pool)
    .await
    .expect("subscription count should query");
    let invoice_count_after_first: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.invoices WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("invoice count should query");

    let second = service
        .execute_preview(
            "tenant-stage",
            &crate::models::MixradiusImportExecuteRequest {
                batch_id: batch.id.clone(),
                execution_mode: crate::models::MixradiusImportExecutionMode::SafeImport,
                mapping_overrides: vec![],
                customer_conflict_resolution: None,
                location_strategy: None,
                pppoe_provisioning_target: None,
            },
        )
        .await
        .expect("second safe execute should stay idempotent");

    let package_count_after_second: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.isp_packages WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("package count should query");
    let customer_count_after_second: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.customers WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("customer count should query");
    let location_count_after_second: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.customer_locations WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("location count should query");
    let subscription_count_after_second: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.customer_subscriptions WHERE tenant_id = $1",
    )
    .bind("tenant-stage")
    .fetch_one(&pool)
    .await
    .expect("subscription count should query");
    let invoice_count_after_second: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.invoices WHERE tenant_id = $1")
            .bind("tenant-stage")
            .fetch_one(&pool)
            .await
            .expect("invoice count should query");

    assert_eq!(package_count_after_first, package_count_after_second);
    assert_eq!(customer_count_after_first, customer_count_after_second);
    assert_eq!(location_count_after_first, location_count_after_second);
    assert_eq!(
        subscription_count_after_first,
        subscription_count_after_second
    );
    assert_eq!(invoice_count_after_first, 0);
    assert_eq!(invoice_count_after_second, 0);
    assert_eq!(first.batch.summary_json["legacyTransactionCount"], 1902);
    assert_eq!(first.batch.summary_json["productionInvoiceCount"], 0);
    assert_eq!(second.batch.summary_json["legacyTransactionCount"], 1902);
    assert_eq!(second.batch.summary_json["productionInvoiceCount"], 0);

    let tenant_b_access = service.get_batch("tenant-b", &batch.id).await;
    assert!(tenant_b_access.is_err());
}
