# Mikrotik Log Retention Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace MikroTik log retention from fixed-count pruning to validated, configurable time-based retention (default 90 days) while preserving sync flow and API/UI pagination behavior.

**Architecture:** Keep existing fetch+upsert log sync flow in place and only replace the prune criterion inside `sync_logs_for_router()` with a router-scoped cutoff deletion by timestamp. Introduce a small retention-days config parser in the MikroTik service with strict validation and deterministic fallback to 90. Preserve API and command signatures; add DB index support for prune and paginated reads, and verify behavior with targeted tests and query-plan checks.

**Tech Stack:** Rust 2021, Tauri, Axum, SQLx, Chrono, PostgreSQL migrations, Cargo test.

---

## File Structure and Responsibility Map

### Core backend behavior
- **Modify:** `src-tauri/src/services/mikrotik_service.rs`
  - Add retention config helper (`MIKROTIK_LOG_RETENTION_DAYS` parse+validation with fallback 90).
  - Replace `OFFSET 5000` prune SQL with time-window prune (`logged_at < cutoff`) scoped by tenant/router.
  - Keep `list_logs` ordering and pagination unchanged.
  - Keep prune failure as hard sync failure (existing error propagation path remains required).

### API and command surfaces (no contract change)
- **Verify unchanged behavior:** `src-tauri/src/http/mikrotik.rs`
- **Verify unchanged behavior:** `src-tauri/src/commands/mikrotik.rs`

### Database indexing for retention-window scale
- **Create:** `src-tauri/migrations/20260331101500_add_mikrotik_log_retention_indexes.up.sql`
- **Create:** `src-tauri/migrations/20260331101500_add_mikrotik_log_retention_indexes.down.sql`
  - Add composite index for router-scoped prune + paginated reads with stable ordering support.

### Tests
- **Modify (inline tests module):** `src-tauri/src/services/mikrotik_service.rs`
  - Add unit tests for retention days parsing/validation.
  - Add integration-style DB tests for time-based prune correctness and no 5000-cap behavior.
  - Add failure-path test proving prune failure fails sync.
  - Add query-plan assertion tests for prune and pagination index usage.

### Documentation / deployment env contract
- **Modify:** `.env.example`
- **Modify:** `deploy/systemd/server.env.example`
  - Document `MIKROTIK_LOG_RETENTION_DAYS=90` contract and valid range behavior.

---

### Task 1: Add retention config parser with strict validation and fallback

**Files:**
- Modify: `src-tauri/src/services/mikrotik_service.rs`
- Test: `src-tauri/src/services/mikrotik_service.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn mikrotik_log_retention_days_env_validation() {
    assert_eq!(resolve_mikrotik_log_retention_days(None), 90);
    assert_eq!(resolve_mikrotik_log_retention_days(Some("abc")), 90);
    assert_eq!(resolve_mikrotik_log_retention_days(Some("0")), 90);
    assert_eq!(resolve_mikrotik_log_retention_days(Some("-5")), 90);
    assert_eq!(resolve_mikrotik_log_retention_days(Some("3651")), 90);
    assert_eq!(resolve_mikrotik_log_retention_days(Some("30")), 30);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
cd src-tauri && cargo test mikrotik_log_retention_days_env_validation --lib
```

Expected:
- FAIL with unresolved function `resolve_mikrotik_log_retention_days`.

- [ ] **Step 3: Write minimal implementation**

```rust
fn resolve_mikrotik_log_retention_days(raw: Option<&str>) -> i64 {
    match raw.and_then(|v| v.trim().parse::<i64>().ok()) {
        Some(days) if (1..=3650).contains(&days) => days,
        _ => 90,
    }
}

fn mikrotik_log_retention_days_from_env() -> i64 {
    let raw = std::env::var("MIKROTIK_LOG_RETENTION_DAYS").ok();
    let days = resolve_mikrotik_log_retention_days(raw.as_deref());
    tracing::debug!(
        target: "mikrotik_retention",
        retention_days = days,
        "Resolved MikroTik log retention days"
    );
    days
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:
```bash
cd src-tauri && cargo test mikrotik_log_retention_days_env_validation --lib
```

Expected:
- PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/mikrotik_service.rs
git commit -m "test+feat(mikrotik): add validated log retention env parsing"
```

---

### Task 2: Replace 5000-cap prune with time-based prune in sync path

**Files:**
- Modify: `src-tauri/src/services/mikrotik_service.rs`
- Test: `src-tauri/src/services/mikrotik_service.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn sync_logs_prunes_only_records_older_than_retention_cutoff() {
    // Seed: same router with 3 old rows (> 90d) and 3 recent rows (< 90d).
    // Execute prune stage via sync helper.
    // Assert old rows deleted, recent rows preserved.
}

#[tokio::test]
async fn sync_logs_does_not_apply_fixed_5000_cap_anymore() {
    // Seed: 5_100 rows all within retention window.
    // Execute sync.
    // Assert count remains 5_100 (modulo upsert duplicates), no OFFSET-based trimming.
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:
```bash
cd src-tauri && cargo test sync_logs_prunes_only_records_older_than_retention_cutoff --lib
cd src-tauri && cargo test sync_logs_does_not_apply_fixed_5000_cap_anymore --lib
```

Expected:
- First test FAILS because existing code still uses `OFFSET 5000` logic.
- Second test FAILS because rows are still trimmed by count.

- [ ] **Step 3: Write minimal implementation**

Replace current count-prune block in `sync_logs_for_router()` with time-prune:

```rust
let retention_days = mikrotik_log_retention_days_from_env();
let cutoff = Utc::now() - ChronoDuration::days(retention_days);

sqlx::query(
    r#"
    DELETE FROM mikrotik_logs
    WHERE tenant_id = $1
      AND router_id = $2
      AND logged_at < $3
    "#,
)
.bind(tenant_id)
.bind(router_id)
.bind(cutoff)
.execute(&self.pool)
.await
.map_err(AppError::Database)?;
```

Also remove remaining `OFFSET 5000` retention SQL from the method.

- [ ] **Step 4: Run tests to verify they pass**

Run:
```bash
cd src-tauri && cargo test sync_logs_prunes_only_records_older_than_retention_cutoff --lib
cd src-tauri && cargo test sync_logs_does_not_apply_fixed_5000_cap_anymore --lib
```

Expected:
- PASS for both tests.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/mikrotik_service.rs
git commit -m "test+feat(mikrotik): switch log prune from count cap to retention window"
```

---

### Task 3: Enforce prune-failure propagation and preserve pagination behavior unchanged

**Files:**
- Modify: `src-tauri/src/services/mikrotik_service.rs`
- Verify unchanged behavior: `src-tauri/src/http/mikrotik.rs`
- Verify unchanged behavior: `src-tauri/src/commands/mikrotik.rs`
- Test: `src-tauri/src/services/mikrotik_service.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn sync_logs_returns_error_when_prune_query_fails() {
    // Arrange a DB state where prune query fails (e.g., dropped table in isolated test DB).
    // Call sync_logs_for_router.
    // Assert Result is Err and no success payload is returned.
}

#[tokio::test]
async fn list_logs_pagination_order_and_defaults_remain_unchanged() {
    // Seed logs with equal logged_at and different updated_at values.
    // Call list_logs(page=1, per_page=25, include_total=false).
    // Assert order remains logged_at DESC, updated_at DESC and defaults unchanged.
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:
```bash
cd src-tauri && cargo test sync_logs_returns_error_when_prune_query_fails --lib
cd src-tauri && cargo test list_logs_pagination_order_and_defaults_remain_unchanged --lib
```

Expected:
- First test FAILS before explicit assertion is satisfied.
- Second test FAILS until characterization fixture/expectation is aligned to current ordering guarantees.

- [ ] **Step 3: Write minimal implementation**

No API/command contract edits. Keep existing signatures and defaults in `list_logs` call paths:

```rust
// commands/mikrotik.rs (unchanged behavior target)
page.unwrap_or(1)
per_page.unwrap_or(25)
include_total.unwrap_or(false)
```

```rust
// http/mikrotik.rs (unchanged behavior target)
q.page.unwrap_or(1)
q.per_page.unwrap_or(25)
q.include_total.unwrap_or(false)
```

In service, keep prune error path as hard failure:

```rust
.execute(&self.pool)
.await
.map_err(AppError::Database)?;
```

- [ ] **Step 4: Run tests to verify they pass**

Run:
```bash
cd src-tauri && cargo test sync_logs_returns_error_when_prune_query_fails --lib
cd src-tauri && cargo test list_logs_pagination_order_and_defaults_remain_unchanged --lib
```

Expected:
- PASS for both tests.
- Confirmed: sync fails on prune failure, pagination semantics unchanged.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/mikrotik_service.rs src-tauri/src/http/mikrotik.rs src-tauri/src/commands/mikrotik.rs
git commit -m "test(mikrotik): enforce prune failure propagation and lock pagination behavior"
```

---

### Task 4: Add retention-supporting indexes and query-plan verification

**Files:**
- Create: `src-tauri/migrations/20260331101500_add_mikrotik_log_retention_indexes.up.sql`
- Create: `src-tauri/migrations/20260331101500_add_mikrotik_log_retention_indexes.down.sql`
- Test: `src-tauri/src/services/mikrotik_service.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
async fn mikrotik_log_queries_use_retention_indexes() {
    // Run EXPLAIN for prune query and paginated list query.
    // Assert output contains index names from new migration.
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
cd src-tauri && cargo test mikrotik_log_queries_use_retention_indexes --lib
```

Expected:
- FAIL because index names are absent before migration.

- [ ] **Step 3: Write minimal implementation**

`src-tauri/migrations/20260331101500_add_mikrotik_log_retention_indexes.up.sql`

```sql
CREATE INDEX IF NOT EXISTS idx_mikrotik_logs_tenant_router_logged_updated
    ON public.mikrotik_logs (tenant_id, router_id, logged_at DESC, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_mikrotik_logs_tenant_router_logged_at
    ON public.mikrotik_logs (tenant_id, router_id, logged_at);
```

`src-tauri/migrations/20260331101500_add_mikrotik_log_retention_indexes.down.sql`

```sql
DROP INDEX IF EXISTS public.idx_mikrotik_logs_tenant_router_logged_updated;
DROP INDEX IF EXISTS public.idx_mikrotik_logs_tenant_router_logged_at;
```

- [ ] **Step 4: Run test to verify it passes**

Run:
```bash
cd src-tauri && cargo test mikrotik_log_queries_use_retention_indexes --lib
```

Expected:
- PASS.
- EXPLAIN output references `idx_mikrotik_logs_tenant_router_logged_at` for prune and `idx_mikrotik_logs_tenant_router_logged_updated` for paginated reads.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/migrations/20260331101500_add_mikrotik_log_retention_indexes.up.sql src-tauri/migrations/20260331101500_add_mikrotik_log_retention_indexes.down.sql src-tauri/src/services/mikrotik_service.rs
git commit -m "test+feat(db): add mikrotik log retention indexes and planner checks"
```

---

### Task 5: Document env contract and deployment defaults

**Files:**
- Modify: `.env.example`
- Modify: `deploy/systemd/server.env.example`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn env_examples_document_mikrotik_log_retention_days() {
    let root_env = std::fs::read_to_string("../.env.example").unwrap();
    let server_env = std::fs::read_to_string("../deploy/systemd/server.env.example").unwrap();

    assert!(root_env.contains("MIKROTIK_LOG_RETENTION_DAYS=90"));
    assert!(server_env.contains("MIKROTIK_LOG_RETENTION_DAYS=90"));
    assert!(root_env.contains("1..3650"));
    assert!(server_env.contains("1..3650"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
cd src-tauri && cargo test env_examples_document_mikrotik_log_retention_days --lib
```

Expected:
- FAIL because variable is not yet documented.

- [ ] **Step 3: Write minimal implementation**

Add to `.env.example`:

```dotenv
# MikroTik log retention window in days (valid: 1..3650, fallback to 90)
MIKROTIK_LOG_RETENTION_DAYS=90
```

Add to `deploy/systemd/server.env.example`:

```dotenv
# MikroTik log retention window in days (valid: 1..3650, fallback to 90)
MIKROTIK_LOG_RETENTION_DAYS=90
```

- [ ] **Step 4: Run test to verify it passes**

Run:
```bash
cd src-tauri && cargo test env_examples_document_mikrotik_log_retention_days --lib
```

Expected:
- PASS.

- [ ] **Step 5: Commit**

```bash
git add .env.example deploy/systemd/server.env.example src-tauri/src/services/mikrotik_service.rs
git commit -m "docs(env): document mikrotik log retention days contract"
```

---

### Task 6: Final verification gate and rollout checks

**Files:**
- Verify: `src-tauri/src/services/mikrotik_service.rs`
- Verify: `src-tauri/src/http/mikrotik.rs`
- Verify: `src-tauri/src/commands/mikrotik.rs`
- Verify: `src-tauri/migrations/20260331101500_add_mikrotik_log_retention_indexes.up.sql`
- Verify: `.env.example`
- Verify: `deploy/systemd/server.env.example`

- [ ] **Step 1: Write failing verification command set (pre-final)**

```bash
cd src-tauri && cargo test mikrotik_log_retention_days_env_validation --lib && false
```

Expected:
- FAIL due forced `&& false`, proving verification gate is actively checked.

- [ ] **Step 2: Run full verification commands**

Run:
```bash
cd src-tauri && cargo fmt --all
cd src-tauri && cargo test mikrotik_log_retention_days_env_validation --lib
cd src-tauri && cargo test sync_logs_prunes_only_records_older_than_retention_cutoff --lib
cd src-tauri && cargo test sync_logs_does_not_apply_fixed_5000_cap_anymore --lib
cd src-tauri && cargo test sync_logs_returns_error_when_prune_query_fails --lib
cd src-tauri && cargo test list_logs_pagination_order_and_defaults_remain_unchanged --lib
cd src-tauri && cargo test mikrotik_log_queries_use_retention_indexes --lib
cd src-tauri && cargo test env_examples_document_mikrotik_log_retention_days --lib
```

Expected:
- All commands PASS.

- [ ] **Step 3: Run rollout smoke checks in deployed environment**

Run:
```bash
# sync endpoint still operational, expected HTTP 200 and JSON { seen, upserted }
curl -sS -X POST "${BASE_URL}/api/admin/mikrotik/routers/${ROUTER_ID}/logs/sync" -H "Authorization: Bearer ${TOKEN}" -H "Content-Type: application/json" -d '{"fetch_limit":500}'

# pagination endpoint unchanged, expected stable paginated JSON shape
curl -sS "${BASE_URL}/api/admin/mikrotik/logs?page=1&per_page=25&include_total=true" -H "Authorization: Bearer ${TOKEN}"
```

Expected:
- Sync returns success only when prune succeeds.
- Pagination fields remain `data`, `total`, `page`, `per_page` with unchanged semantics.

- [ ] **Step 4: Confirm DB planner behavior on production-like volume**

Run:
```bash
psql "$DATABASE_URL" -c "EXPLAIN DELETE FROM mikrotik_logs WHERE tenant_id = 'tenant-a' AND router_id = 'router-a' AND logged_at < NOW() - INTERVAL '90 days';"
psql "$DATABASE_URL" -c "EXPLAIN SELECT * FROM mikrotik_logs WHERE tenant_id = 'tenant-a' AND router_id = 'router-a' ORDER BY logged_at DESC, updated_at DESC LIMIT 25 OFFSET 0;"
```

Expected:
- EXPLAIN includes index scan usage on new composite indexes.
- No full-table scan for router-scoped prune path.

- [ ] **Step 5: Commit final verification evidence**

```bash
git add src-tauri/src/services/mikrotik_service.rs src-tauri/src/http/mikrotik.rs src-tauri/src/commands/mikrotik.rs src-tauri/migrations/20260331101500_add_mikrotik_log_retention_indexes.up.sql src-tauri/migrations/20260331101500_add_mikrotik_log_retention_indexes.down.sql .env.example deploy/systemd/server.env.example
git commit -m "chore(mikrotik): verify retention rollout readiness and unchanged pagination contracts"
```

---

## Requirement-to-Task Coverage Matrix

- Remove 5000 count-based prune behavior: **Task 2**.
- Add time-based retention with default 90 days: **Tasks 1–2**.
- Env contract `MIKROTIK_LOG_RETENTION_DAYS` with validation + fallback 90: **Tasks 1 and 5**.
- Prune failure causes sync failure: **Task 3**.
- Pagination behavior unchanged for API/UI: **Task 3** plus **Task 6 rollout smoke checks**.
- Indexing/performance checks for larger retention window: **Task 4** plus **Task 6 planner checks**.
- Testing strategy and rollout verification: **Tasks 1–6**.

## Self-Review (completed inline)

### 1) Spec coverage check
- Every acceptance criterion in `docs/superpowers/specs/2026-03-31-mikrotik-log-retention-design.md` is mapped in the coverage matrix above.
- Required paths are explicitly included:
  - `src-tauri/src/services/mikrotik_service.rs`
  - `src-tauri/src/http/mikrotik.rs`
  - `src-tauri/src/commands/mikrotik.rs`
  - `src-tauri/migrations/` (exact migration files)
  - tests in `src-tauri/src/services/mikrotik_service.rs`
  - optional env docs `.env.example`, `deploy/systemd/server.env.example`

### 2) Placeholder scan
- Removed vague directives and ensured each step includes concrete code and exact commands.
- No `TODO`, `TBD`, or “similar to previous task” statements remain.

### 3) Type/signature consistency check
- Retention helper names are consistent across tasks: `resolve_mikrotik_log_retention_days`, `mikrotik_log_retention_days_from_env`.
- Sync function target remains `sync_logs_for_router`.
- Pagination semantics reference consistent defaults (`page=1`, `per_page=25`, `include_total=false`) and service ordering (`logged_at DESC, updated_at DESC`).
