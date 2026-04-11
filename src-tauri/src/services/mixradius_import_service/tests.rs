use uuid::Uuid;

const TEST_ADMIN_DATABASE_URL: &str = "postgres://postgres:postgres@127.0.0.1/postgres";
const MIXRADIUS_IMPORT_FOUNDATION_UP_SQL: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/migrations/20260411120000_add_mixradius_import_foundation.up.sql");
const MIXRADIUS_IMPORT_FOUNDATION_DOWN_SQL: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/migrations/20260411120000_add_mixradius_import_foundation.down.sql");

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
        };
        assert!(preview.validate().is_err());

        let preview_override = MixradiusImportPreviewRequest {
            batch_id: "batch-1".into(),
            mapping_overrides: vec![crate::models::mixradius_import::MixradiusImportMappingOverride {
                source_kind: "   ".into(),
                source_value: "   ".into(),
                target_kind: "   ".into(),
                target_value: "   ".into(),
            }],
            customer_conflict_resolution: None,
            location_strategy: None,
        };
        assert!(preview.validate().is_err());
        assert!(preview_override.validate().is_err());

        let execute = MixradiusImportExecuteRequest {
            batch_id: "   ".into(),
            execution_mode: MixradiusImportExecutionMode::ForceSync,
            mapping_overrides: vec![],
            customer_conflict_resolution: None,
            location_strategy: None,
        };
        assert!(execute.validate().is_err());

        let execute_override = MixradiusImportExecuteRequest {
            batch_id: "batch-2".into(),
            execution_mode: MixradiusImportExecutionMode::ForceSync,
            mapping_overrides: vec![crate::models::mixradius_import::MixradiusImportMappingOverride {
                source_kind: "   ".into(),
                source_value: "   ".into(),
                target_kind: "   ".into(),
                target_value: "   ".into(),
            }],
            customer_conflict_resolution: None,
            location_strategy: None,
        };
        assert!(execute.validate().is_err());
        assert!(execute_override.validate().is_err());
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
