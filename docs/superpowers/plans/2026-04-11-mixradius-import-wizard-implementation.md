# MixRadius Import Wizard Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a tenant-admin MixRadius backup import wizard that uploads `.sql`/`.sql.gz`, stages parsed data, previews conflicts and lifecycle outcomes, and safely imports packages, customers, locations, subscriptions, and PPPoE accounts into ISP Management.

**Architecture:** The feature extends the existing tenant PPPoE admin area with a new MixRadius import flow. Backend work is split into schema/staging, parser/preview, and execution layers, while production writes reuse existing customer, package, and PPPoE domain services wherever practical so lifecycle rules remain intact.

**Tech Stack:** SvelteKit, TypeScript, Rust, Axum, SQLx, PostgreSQL/SQLite compatibility patterns already used in the repo, existing Tauri command bridge, existing customer/package/PPPoE services.

---

## File Map

### Existing files to modify
- `src/routes/[tenant]/(app)/admin/network/pppoe/+page.svelte`
  - Add entry point for `Import from MixRadius`.
- `src/lib/api/core.ts`
  - Register new Tauri/HTTP route keys for MixRadius import.
- `src/lib/api/client.ts`
  - Export new MixRadius import API surface.
- `src-tauri/src/models/mod.rs`
  - Re-export new MixRadius import models.
- `src-tauri/src/services/mod.rs`
  - Register new MixRadius import service.
- `src-tauri/src/http/mod.rs`
  - Mount new admin HTTP router.
- `src-tauri/src/main.rs`
  - Register new Tauri commands if this app exposes them from the main command registration surface.
- `src-tauri/src/http/pppoe.rs`
  - Reuse conventions and possibly link PPPoE page navigation copy if needed.
- `src-tauri/src/services/pppoe_service.rs`
  - Add import execution helpers or public methods for safe PPPoE upsert reuse.
- `src-tauri/src/services/isp_package_service.rs`
  - Add reusable “find or create by import mapping” helper if needed.
- `src-tauri/src/services/customer_service/subscriptions.rs`
  - Add reusable import-safe subscription create/update path if needed.
- `src-tauri/src/services/customer_service/core.rs`
  - Add reusable import-safe customer create/update helper if needed.

### New backend files
- `src-tauri/migrations/20260411120000_add_mixradius_import_foundation.up.sql`
- `src-tauri/migrations/20260411120000_add_mixradius_import_foundation.down.sql`
- `src-tauri/src/models/mixradius_import.rs`
- `src-tauri/src/http/mixradius_import.rs`
- `src-tauri/src/services/mixradius_import_service.rs`
- `src-tauri/src/services/mixradius_sql_parser.rs`
- `src-tauri/src/services/mixradius_import_mapper.rs`
- `src-tauri/src/services/mixradius_import_executor.rs`
- `src-tauri/src/services/mixradius_import_service/tests.rs`

### New frontend files
- `src/lib/api/mixradiusImport.ts`
- `src/routes/[tenant]/(app)/admin/network/pppoe/import-mixradius/+page.svelte`
- `src/lib/components/network/mixradius/MixRadiusImportWizard.svelte`
- `src/lib/components/network/mixradius/MixRadiusUploadStep.svelte`
- `src/lib/components/network/mixradius/MixRadiusSourceSummaryStep.svelte`
- `src/lib/components/network/mixradius/MixRadiusMappingStep.svelte`
- `src/lib/components/network/mixradius/MixRadiusPreviewStep.svelte`
- `src/lib/components/network/mixradius/MixRadiusExecutionStep.svelte`
- `src/lib/components/network/mixradius/mixradiusImportTypes.ts`
- `src/lib/components/network/mixradius/mixradiusImportTypes.test.ts`

### New tests
- `src-tauri/src/services/mixradius_sql_parser_tests.rs`
- `src-tauri/src/services/mixradius_import_mapper_tests.rs`
- `src-tauri/src/services/mixradius_import_executor_tests.rs`
- `src/lib/api/mixradiusImport.test.ts`
- `src/lib/components/network/mixradius/mixradiusImportTypes.test.ts`

### Fixtures / docs
- Reuse: `MixRadiusDB_Gasal_2026-04-11_101103.sql.gz`
- Create: `src-tauri/tests/fixtures/mixradius-import-minimal.sql`
  - Small deterministic parser/executor fixture so the test suite does not depend only on the large real backup.

## Chunk 1: Schema And Model Foundation

### Task 1: Add migration tests-first checklist for staging schema

**Files:**
- Create: `src-tauri/migrations/20260411120000_add_mixradius_import_foundation.up.sql`
- Create: `src-tauri/migrations/20260411120000_add_mixradius_import_foundation.down.sql`
- Test: `src-tauri/src/services/mixradius_import_service/tests.rs`

- [ ] **Step 1: Write the failing service-level migration smoke test**

Add a test skeleton in `src-tauri/src/services/mixradius_import_service/tests.rs` that expects the schema to contain:
- `mixradius_import_batches`
- `mixradius_staging_nas`
- `mixradius_staging_plans`
- `mixradius_staging_customers`
- `mixradius_staging_customer_locations`
- `mixradius_staging_transactions`
- `mixradius_staging_usage`
- `mixradius_import_conflicts`

- [ ] **Step 2: Run the focused Rust test to verify it fails**

Run: `cargo test mixradius_import_schema --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: FAIL because the tables and test module do not exist yet.

- [ ] **Step 3: Add the migration**

Implement the migration with:
- batch table
- staging tables
- staging usage table for `tbl_usage_reports`
- import-conflict table
- external ref tables or equivalent generic import-reference tables
- tenant-scoped indexes
- batch status fields
- execution mode fields
- progress/report fields

- [ ] **Step 4: Re-run the focused Rust test**

Run: `cargo test mixradius_import_schema --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/migrations/20260411120000_add_mixradius_import_foundation.up.sql src-tauri/migrations/20260411120000_add_mixradius_import_foundation.down.sql src-tauri/src/services/mixradius_import_service/tests.rs
git commit -m "feat: add mixradius import schema foundation"
```

### Task 2: Add backend models for MixRadius import entities

**Files:**
- Create: `src-tauri/src/models/mixradius_import.rs`
- Modify: `src-tauri/src/models/mod.rs`
- Test: `src-tauri/src/services/mixradius_import_service/tests.rs`

- [ ] **Step 1: Write failing serialization and enum contract tests**

Cover:
- batch status serde contract
- preview row conflict state serde contract
- execution summary shape
- request DTO validation shape

- [ ] **Step 2: Run the focused Rust test to verify it fails**

Run: `cargo test mixradius_import_models --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: FAIL because model module and types do not exist.

- [ ] **Step 3: Implement the model file**

Include:
- `MixradiusImportBatch`
- `MixradiusImportBatchStatus`
- `MixradiusImportParseStatus`
- `MixradiusImportPreview`
- `MixradiusImportPreviewRow`
- `MixradiusImportConflictState`
- upload/preview/execute request DTOs
- execution result DTOs

- [ ] **Step 4: Re-run the focused Rust test**

Run: `cargo test mixradius_import_models --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models/mixradius_import.rs src-tauri/src/models/mod.rs src-tauri/src/services/mixradius_import_service/tests.rs
git commit -m "feat: add mixradius import models"
```

## Chunk 2: Parser And Staging Pipeline

### Task 3: Build a focused MixRadius SQL parser

**Files:**
- Create: `src-tauri/src/services/mixradius_sql_parser.rs`
- Create: `src-tauri/src/services/mixradius_sql_parser_tests.rs`

- [ ] **Step 1: Write failing parser tests using the validated backup**

Cover:
- plain `.sql` file is accepted
- `.sql.gz` file is accepted
- required tables are detected
- counts for the validated backup are parsed into normalized structures
- unsupported tables are ignored
- malformed gzip returns actionable error
- missing required tables return actionable error

- [ ] **Step 2: Run parser tests to verify they fail**

Run: `cargo test mixradius_sql_parser --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: FAIL because parser module is missing.

- [ ] **Step 3: Implement minimal parser**

Responsibilities:
- detect gzip by extension or magic header
- stream or read file contents safely
- parse only supported `CREATE TABLE` and `INSERT INTO` sections
- normalize source rows for:
  - `nas`
  - `tbl_customers`
  - `tbl_customers_sub`
  - `tbl_customers_map`
  - `tbl_odp_data`
  - `tbl_plans`
  - `tbl_bandwidth`
  - `tbl_transactions`
  - `radcheck`
  - `radreply`
  - `radusergroup`
- normalize optional `tbl_usage_reports` into usage staging rows

- [ ] **Step 4: Re-run parser tests**

Run: `cargo test mixradius_sql_parser --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/mixradius_sql_parser.rs src-tauri/src/services/mixradius_sql_parser_tests.rs
git commit -m "feat: add mixradius sql parser"
```

### Task 4: Stage parsed rows into import tables

**Files:**
- Create: `src-tauri/src/services/mixradius_import_service.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/mixradius_import_service/tests.rs`

- [ ] **Step 1: Write failing staging tests**

Cover:
- upload batch registration creates one batch row
- staging writes expected customer/plan/router counts
- batch summary is persisted

- [ ] **Step 2: Run service tests to verify they fail**

Run: `cargo test mixradius_import_stage --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: FAIL because service implementation is missing.

- [ ] **Step 3: Implement minimal staging service**

Responsibilities:
- register batch
- call parser
- write staging rows
- write summary JSON
- mark parse status success/failure

- [ ] **Step 4: Re-run service tests**

Run: `cargo test mixradius_import_stage --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/mixradius_import_service.rs src-tauri/src/services/mod.rs src-tauri/src/services/mixradius_import_service/tests.rs
git commit -m "feat: add mixradius import staging service"
```

## Chunk 3: Mapping And Preview Resolver

### Task 5: Add mapping resolver and conflict detector

**Files:**
- Create: `src-tauri/src/services/mixradius_import_mapper.rs`
- Create: `src-tauri/src/services/mixradius_import_mapper_tests.rs`
- Modify: `src-tauri/src/services/mixradius_import_service.rs`

- [ ] **Step 1: Write failing mapper tests**

Cover:
- exact package name reuse
- unresolved router mapping becomes blocked
- PPP username conflict across different routers becomes conflict
- customer external ref reuse becomes auto-matched
- `PAID` plus not expired becomes preview `active`
- `UNPAID` plus expired becomes preview `suspended`
- `UNPAID` plus not expired becomes preview `active` with billing attention warning
- `PENDING` produces an explicit review state or warning according to the request policy
- all conflict states serialize and render: `auto_matched`, `needs_review`, `conflict`, `blocked`, `skipped`
- update-precedence rules preserve local customer profile edits in safe mode

- [ ] **Step 2: Run mapper tests to verify they fail**

Run: `cargo test mixradius_import_mapper --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: FAIL because resolver is missing.

- [ ] **Step 3: Implement resolver**

Implement:
- NAS -> router suggestion rules
- plan -> package suggestion rules
- customer conflict states
- subscription lifecycle normalization for preview
- PPPoE action classification (`new`, `update`, `same`, `blocked`)
- update precedence rules for safe mode
- explicit `PENDING` lifecycle handling

- [ ] **Step 4: Re-run mapper tests**

Run: `cargo test mixradius_import_mapper --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/mixradius_import_mapper.rs src-tauri/src/services/mixradius_import_mapper_tests.rs src-tauri/src/services/mixradius_import_service.rs
git commit -m "feat: add mixradius import preview resolver"
```

### Task 6: Expose preview API and Tauri route surface

**Files:**
- Create: `src-tauri/src/http/mixradius_import.rs`
- Modify: `src-tauri/src/http/mod.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/lib/api/core.ts`
- Create: `src/lib/api/mixradiusImport.ts`
- Modify: `src/lib/api/client.ts`
- Test: `src/lib/api/mixradiusImport.test.ts`

- [ ] **Step 1: Write failing API tests**

Frontend tests should assert:
- upload route key names exist
- list/get route key names exist
- preview and execute route bindings exist
- cancel route binding exists
- client wrapper passes expected params
- mapping override payloads pass through to preview and execute calls
- customer conflict resolution payloads pass through to preview and execute calls
- ODP/POP strategy payloads pass through to preview and execute calls
- execution mode payloads pass through to execute calls

- [ ] **Step 2: Run frontend API tests to verify they fail**

Run: `npm test -- mixradiusImport`

Expected: FAIL because client wrapper and route keys do not exist.

- [ ] **Step 3: Implement backend and frontend API surfaces**

Add:
- batch upload endpoint
- list/get batch endpoints
- preview endpoint
- execute endpoint
- cancel endpoint
- mapping override request payloads
- customer conflict resolution request payloads
- ODP/POP strategy request payloads
- execution mode request payloads: `preview_only`, `safe_import`, `force_sync`

Add matching TypeScript API client wrapper under `src/lib/api/mixradiusImport.ts`.

- [ ] **Step 4: Re-run API tests**

Run: `npm test -- mixradiusImport`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/http/mixradius_import.rs src-tauri/src/http/mod.rs src-tauri/src/main.rs src/lib/api/core.ts src/lib/api/mixradiusImport.ts src/lib/api/client.ts src/lib/api/mixradiusImport.test.ts
git commit -m "feat: expose mixradius import api"
```

### Task 7: Add authorization, tenant isolation, mapping override, and batch cancel behavior

**Files:**
- Modify: `src-tauri/src/http/mixradius_import.rs`
- Modify: `src-tauri/src/services/mixradius_import_service.rs`
- Modify: `src-tauri/src/services/mixradius_import_mapper.rs`
- Test: `src-tauri/src/services/mixradius_import_service/tests.rs`

- [ ] **Step 1: Write failing security and override tests**

Cover:
- user without `pppoe:manage` cannot upload, preview, execute, or cancel
- user without `pppoe:manage` cannot list or read batch details
- user from tenant A cannot read, preview, execute, or cancel tenant B batch
- cancel moves a not-yet-executed batch to cancelled status
- preview accepts admin mapping overrides for NAS -> router and plan -> package
- preview accepts admin decisions for customer conflict resolution (`merge`, `create_new`, `skip`)
- preview accepts ODP/POP import strategy selections for location metadata handling
- execute uses the exact mapping overrides captured in the request

- [ ] **Step 2: Run focused tests to verify they fail**

Run: `cargo test mixradius_import_authorization --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Run: `cargo test mixradius_import_overrides --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: FAIL because tenant guards, cancel, and override persistence are incomplete.

- [ ] **Step 3: Implement the behavior**

Add:
- `pppoe:manage` checks in list/get endpoints as well as mutation endpoints
- `pppoe:manage` checks in all mutation endpoints
- tenant-scoped batch lookups in every batch endpoint
- cancel service method
- mapping override DTOs and persistence on batch preview/execute
- customer-resolution override DTOs and persistence on batch preview/execute
- ODP/POP strategy override DTOs and persistence on batch preview/execute
- preview rebuild that reuses submitted overrides

- [ ] **Step 4: Re-run focused tests**

Run: `cargo test mixradius_import_authorization --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Run: `cargo test mixradius_import_overrides --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/http/mixradius_import.rs src-tauri/src/services/mixradius_import_service.rs src-tauri/src/services/mixradius_import_mapper.rs src-tauri/src/services/mixradius_import_service/tests.rs
git commit -m "feat: secure mixradius import batches"
```

## Chunk 4: Safe Execution Into Production Domains

### Task 8: Add package import execution with idempotent upsert

**Files:**
- Modify: `src-tauri/src/services/isp_package_service.rs`
- Modify: `src-tauri/src/services/mixradius_import_service.rs`
- Create: `src-tauri/src/services/mixradius_import_executor.rs`
- Create: `src-tauri/src/services/mixradius_import_executor_tests.rs`

- [ ] **Step 1: Write failing execution tests for package import**

Cover:
- exact-name reuse does not duplicate package
- new MixRadius plan creates package
- same-name but different pricing creates review conflict instead of blind overwrite

- [ ] **Step 2: Run execution tests to verify they fail**

Run: `cargo test mixradius_import_executor --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: FAIL because executor is missing.

- [ ] **Step 3: Implement package execution path**

Add a small import-safe helper in `IspPackageService` or local executor logic that:
- validates tenant ownership
- reuses package by resolved mapping
- creates package with normalized monthly price and metadata-derived description/features

- [ ] **Step 4: Re-run execution tests**

Run: `cargo test mixradius_import_executor --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS for package cases.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/isp_package_service.rs src-tauri/src/services/mixradius_import_service.rs src-tauri/src/services/mixradius_import_executor.rs src-tauri/src/services/mixradius_import_executor_tests.rs
git commit -m "feat: execute mixradius package imports"
```

### Task 9: Add customer and location import execution

**Files:**
- Modify: `src-tauri/src/services/customer_service/core.rs`
- Modify: `src-tauri/src/services/mixradius_import_executor.rs`
- Modify: `src-tauri/src/services/mixradius_import_executor_tests.rs`

- [ ] **Step 1: Write failing customer/location execution tests**

Cover:
- new MixRadius customer creates customer + default location
- repeat import reuses external reference
- coordinates map to location lat/lon
- local notes are preserved when overwrite policy is safe mode

- [ ] **Step 2: Run execution tests to verify they fail**

Run: `cargo test mixradius_import_executor --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: FAIL for customer/location cases.

- [ ] **Step 3: Implement customer/location execution**

Add import-safe helper(s) that:
- create or reuse customer
- create or reuse service location
- preserve MixRadius source metadata in notes or external refs

- [ ] **Step 4: Re-run execution tests**

Run: `cargo test mixradius_import_executor --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS for customer/location cases.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/customer_service/core.rs src-tauri/src/services/mixradius_import_executor.rs src-tauri/src/services/mixradius_import_executor_tests.rs
git commit -m "feat: execute mixradius customer imports"
```

### Task 10: Add subscription execution with lifecycle-safe normalization

**Files:**
- Modify: `src-tauri/src/services/customer_service/subscriptions.rs`
- Modify: `src-tauri/src/services/mixradius_import_executor.rs`
- Modify: `src-tauri/src/services/mixradius_import_executor_tests.rs`

- [ ] **Step 1: Write failing subscription lifecycle tests**

Cover:
- `PAID + not expired -> active`
- `UNPAID + expired -> suspended`
- `UNPAID + not expired -> active` with import warning metadata
- `PENDING` produces the agreed pending-review behavior and does not silently become cancelled
- duplicate active subscription for same location is updated or blocked according to resolved mapping
- imported legacy transactions do not create active production invoices

- [ ] **Step 2: Run execution tests to verify they fail**

Run: `cargo test mixradius_import_executor --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: FAIL for subscription cases.

- [ ] **Step 3: Implement subscription execution**

Prefer small import helper(s) that normalize:
- billing cycle
- status
- starts_at / ends_at
- router linkage
- notes with import metadata
- legacy transaction metadata without creating production invoices

- [ ] **Step 4: Re-run execution tests**

Run: `cargo test mixradius_import_executor --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS for subscription cases.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/customer_service/subscriptions.rs src-tauri/src/services/mixradius_import_executor.rs src-tauri/src/services/mixradius_import_executor_tests.rs
git commit -m "feat: execute mixradius subscription imports"
```

### Task 11: Add PPPoE execution with encrypted password handling

**Files:**
- Modify: `src-tauri/src/services/pppoe_service.rs`
- Modify: `src-tauri/src/services/mixradius_import_executor.rs`
- Modify: `src-tauri/src/services/mixradius_import_executor_tests.rs`

- [ ] **Step 1: Write failing PPPoE execution tests**

Cover:
- PPPoE account is created with encrypted password
- repeat import updates same `(tenant, router, username)` row instead of duplicating
- router mismatch becomes conflict
- remote address comes from `Framed-IP-Address`

- [ ] **Step 2: Run execution tests to verify they fail**

Run: `cargo test mixradius_import_executor --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: FAIL for PPPoE cases.

- [ ] **Step 3: Implement PPPoE import-safe upsert**

Prefer a helper in `PppoeService` or local executor path that:
- encrypts source password
- respects router/customer/location ownership
- updates existing account when external ref or unique identity already exists

- [ ] **Step 4: Re-run execution tests**

Run: `cargo test mixradius_import_executor --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS for PPPoE cases.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/pppoe_service.rs src-tauri/src/services/mixradius_import_executor.rs src-tauri/src/services/mixradius_import_executor_tests.rs
git commit -m "feat: execute mixradius pppoe imports"
```

### Task 12: Add execution modes, progress recording, and batch reports

**Files:**
- Modify: `src-tauri/src/services/mixradius_import_service.rs`
- Modify: `src-tauri/src/services/mixradius_import_executor.rs`
- Modify: `src-tauri/src/http/mixradius_import.rs`
- Modify: `src-tauri/src/models/mixradius_import.rs`
- Modify: `src-tauri/src/services/mixradius_import_executor_tests.rs`

- [ ] **Step 1: Write failing execution-mode and report tests**

Cover:
- `preview_only` never writes production data
- `safe_import` skips conflicts and blocked rows
- `force_sync` applies allowed overwrites but still respects tenant and router ownership
- progress updates after package/customer/subscription/PPPoE phases
- batch report is retrievable after partial success
- one failing phase does not erase earlier successfully committed phases

- [ ] **Step 2: Run execution tests to verify they fail**

Run: `cargo test mixradius_import_execution_modes --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Run: `cargo test mixradius_import_reports --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: FAIL because mode behavior and reporting are incomplete.

- [ ] **Step 3: Implement mode semantics and reports**

Implement:
- explicit execution-mode enum
- per-domain transaction boundaries
- persisted progress counters
- persisted execution report JSON
- partial-success status handling
- report retrieval through the get-batch endpoint

- [ ] **Step 4: Re-run execution-mode and report tests**

Run: `cargo test mixradius_import_execution_modes --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Run: `cargo test mixradius_import_reports --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/mixradius_import_service.rs src-tauri/src/services/mixradius_import_executor.rs src-tauri/src/http/mixradius_import.rs src-tauri/src/models/mixradius_import.rs src-tauri/src/services/mixradius_import_executor_tests.rs
git commit -m "feat: add mixradius import execution modes"
```

## Chunk 5: Wizard UI

### Task 13: Add frontend import types and API integration

**Files:**
- Create: `src/lib/components/network/mixradius/mixradiusImportTypes.ts`
- Create: `src/lib/components/network/mixradius/mixradiusImportTypes.test.ts`
- Modify: `src/lib/api/mixradiusImport.ts`

- [ ] **Step 1: Write failing TS tests for normalization helpers**

Cover:
- conflict badge labels
- lifecycle status labels
- preview count formatting
- execution mode labels
- blocked/conflict safe-mode disable rules

- [ ] **Step 2: Run the focused TS test**

Run: `npm test -- mixradiusImportTypes`

Expected: FAIL because helper module does not exist.

- [ ] **Step 3: Implement minimal helpers**

Add small, focused UI helper types and label formatters used by multiple wizard steps.

- [ ] **Step 4: Re-run the focused TS test**

Run: `npm test -- mixradiusImportTypes`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/network/mixradius/mixradiusImportTypes.ts src/lib/components/network/mixradius/mixradiusImportTypes.test.ts src/lib/api/mixradiusImport.ts
git commit -m "feat: add mixradius import ui helpers"
```

### Task 14: Build the dedicated multi-step wizard route

**Files:**
- Create: `src/lib/components/network/mixradius/MixRadiusImportWizard.svelte`
- Create: `src/lib/components/network/mixradius/MixRadiusUploadStep.svelte`
- Create: `src/lib/components/network/mixradius/MixRadiusSourceSummaryStep.svelte`
- Create: `src/lib/components/network/mixradius/MixRadiusMappingStep.svelte`
- Create: `src/lib/components/network/mixradius/MixRadiusPreviewStep.svelte`
- Create: `src/lib/components/network/mixradius/MixRadiusExecutionStep.svelte`
- Create: `src/routes/[tenant]/(app)/admin/network/pppoe/import-mixradius/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/network/pppoe/+page.svelte`

- [ ] **Step 1: Write a narrow component test or interaction checklist**

If this repo already has component tests for similar UI, mirror that pattern. Otherwise, create a light test around the helper state transitions and document a manual QA checklist in the component comments or plan execution notes.

Minimum behaviors:
- dedicated route exists at `/{tenant}/admin/network/pppoe/import-mixradius`
- upload step accepts `.sql` and `.sql.gz`
- cannot proceed when upload/parse fails
- mapping step persists admin NAS/package/customer override selections into preview requests
- mapping step persists customer conflict decisions and ODP/POP handling strategy into preview requests
- preview displays counts and conflict tabs
- execute mode selector supports `preview_only`, `safe_import`, and `force_sync`
- execute button is disabled while unresolved blocked items remain in safe mode
- cancel button calls the cancel API before execution

- [ ] **Step 2: Run the focused frontend test or lint check to confirm failure**

Run: `npm test -- MixRadiusImportWizard` or, if no component test harness exists, run `npm run check`

Expected: FAIL or type-check error before components exist.

- [ ] **Step 3: Implement the wizard and page integration**

Requirements:
- entry action on PPPoE page
- dedicated route-mounted wizard page
- step navigation
- error and loading states
- summary cards
- mapping override controls
- conflict tabs
- execution mode selector
- cancel action
- final execution report view

- [ ] **Step 4: Re-run frontend tests and type checks**

Run: `npm run check`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/network/mixradius/MixRadiusImportWizard.svelte src/lib/components/network/mixradius/MixRadiusUploadStep.svelte src/lib/components/network/mixradius/MixRadiusSourceSummaryStep.svelte src/lib/components/network/mixradius/MixRadiusMappingStep.svelte src/lib/components/network/mixradius/MixRadiusPreviewStep.svelte src/lib/components/network/mixradius/MixRadiusExecutionStep.svelte src/routes/[tenant]/(app)/admin/network/pppoe/import-mixradius/+page.svelte src/routes/[tenant]/(app)/admin/network/pppoe/+page.svelte
git commit -m "feat: add mixradius import wizard ui"
```

## Chunk 6: Verification And Hardening

### Task 15: Add end-to-end import happy-path verification against the validated backup

**Files:**
- Modify: `src-tauri/src/services/mixradius_import_service/tests.rs`
- Modify: `src-tauri/src/services/mixradius_import_executor_tests.rs`

- [ ] **Step 1: Write the failing end-to-end import test**

Use the validated backup fixture to assert:
- customer count preview matches expected PPP rows
- package count preview matches expected PPP plans
- router count preview matches expected NAS rows
- safe execution imports without duplicates on second run
- execution report includes legacy transaction count but no production invoice creation count
- tenant B cannot access tenant A batch by ID

- [ ] **Step 2: Run the focused Rust tests to verify failure**

Run: `cargo test mixradius_import_end_to_end --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: FAIL before end-to-end path is complete.

- [ ] **Step 3: Implement missing glue and hardening**

Close gaps found by the test:
- batch status updates
- execution summaries
- idempotency fixes
- import reference persistence

- [ ] **Step 4: Re-run the focused Rust tests**

Run: `cargo test mixradius_import_end_to_end --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/mixradius_import_service/tests.rs src-tauri/src/services/mixradius_import_executor_tests.rs
git commit -m "test: verify mixradius import end-to-end flow"
```

### Task 16: Full verification sweep

**Files:**
- Modify as needed from earlier tasks

- [ ] **Step 1: Run Rust import-related tests**

Run: `cargo test mixradius_import --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS

- [ ] **Step 2: Run PPPoE, package, and customer regression tests**

Run: `cargo test pppoe --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Run: `cargo test customer_subscription --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Run: `cargo test isp_package --manifest-path /home/xtrabit/ISPMANAGEMENT/src-tauri/Cargo.toml`

Expected: PASS

- [ ] **Step 3: Run frontend checks**

Run: `npm run check`

Expected: PASS

- [ ] **Step 4: Run frontend test slices for new APIs/helpers**

Run: `npm test -- mixradiusImport`

Run: `npm test -- mixradiusImportTypes`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: finalize mixradius import wizard"
```

## Notes For The Implementing Agent
- Prefer using existing domain services over raw SQL for production writes whenever doing so does not force invasive refactors.
- Do not import hotspot or voucher data in this plan unless the spec is revised.
- Treat imported MixRadius transactions as legacy history only; do not create active billing invoices from them.
- Preserve tenant scoping at every batch, staging, preview, and execution query.
- Keep parser logic isolated from lifecycle normalization logic.
- Reuse the current PPPoE page’s established UI patterns for modals, tables, filter panels, and toast handling.
- If a helper needed for import-safe upsert becomes too invasive in an existing large service file, split it into a small adjacent helper module instead of expanding the large file further.

## Suggested Execution Order
1. Chunk 1
2. Chunk 2
3. Chunk 3
4. Chunk 4
5. Chunk 5
6. Chunk 6
