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
