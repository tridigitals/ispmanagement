use uuid::Uuid;

const TEST_ADMIN_DATABASE_URL: &str = "postgres://postgres:postgres@127.0.0.1/postgres";
const MIXRADIUS_IMPORT_FOUNDATION_UP_SQL: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/migrations/20260411120000_add_mixradius_import_foundation.up.sql");
const MIXRADIUS_IMPORT_FOUNDATION_DOWN_SQL: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/migrations/20260411120000_add_mixradius_import_foundation.down.sql");
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
    assert_eq!(batch.parse_status, crate::models::MixradiusImportParseStatus::Ready);
    assert_eq!(batch.execution_status, crate::models::MixradiusImportBatchStatus::Pending);

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
        customer_conflict_resolution:
            Some(crate::models::MixradiusImportCustomerConflictResolution::Skip),
        location_strategy: Some(crate::models::MixradiusImportLocationStrategy::Replace),
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
            },
        )
        .await
        .expect("execute preview should reuse submitted overrides");

    let execute_preview = execute.preview.expect("execute should return preview snapshot");
    let execute_nas_row = execute_preview
        .rows
        .iter()
        .find(|row| row.source_kind == "nas" && row.source_ref == "5")
        .expect("execute preview NAS row should exist");
    assert_eq!(execute_nas_row.target_id.as_deref(), Some("router-override-1"));

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
