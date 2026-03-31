# Payment Service Decomposition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decompose `src-tauri/src/services/payment_service/mod.rs` into focused internal submodules while preserving all existing payment behavior and keeping caller-facing APIs unchanged.

**Architecture:** Keep `PaymentService` public API methods in `mod.rs` for pass 1 as a stable facade, and extract only private/helper logic into focused internal modules. Use `pub(super)` visibility and split `impl PaymentService` across module files so command/http callers continue using the same service interface. Preserve query semantics, scheduler behavior, notification side effects, and Midtrans flow by moving code without altering control flow.

**Tech Stack:** Rust 2021, Tauri, Tokio, SQLx, Reqwest, Chrono, Serde, Cargo fmt/test.

---

## Scope and Constraints

- Refactor scope is limited to payment service decomposition under `src-tauri/src/services/payment_service/`.
- No feature additions, no behavior changes, no API contract changes.
- `src-tauri/src/commands/**` and `src-tauri/src/http/**` should not require behavioral edits.
- Pass 1 keeps all existing `pub` methods on `PaymentService` in `mod.rs`.
- Internal extraction is allowed only for private/helper logic.

## Target Module Boundaries and File Plan

### Stable facade

- **Modify** `src-tauri/src/services/payment_service/mod.rs`
  - Keep `PaymentService` type and all existing `pub` methods here.
  - Reduce file size by delegating private internals to focused modules.
  - Keep existing exports (`BulkGenerateInvoicesResult`, `BillingCollectionRunResult`, `BillingCollectionSettings`) stable.

### Existing focused modules (expand responsibilities)

- **Modify** `src-tauri/src/services/payment_service/core.rs`
  - Pure/deterministic helpers and constants only.
  - Keep Midtrans transition logic, external-id parsing, assignment scoring, reminder code, plus extracted pure billing/currency/date helper functions.

- **Modify** `src-tauri/src/services/payment_service/repository.rs`
  - DB-focused private helpers (settings read/write, logs insert/read helpers, subscription status persistence, recipient/member queries, work-order persistence, assignment persistence).

- **Modify** `src-tauri/src/services/payment_service/integration.rs`
  - External integration private helpers (Midtrans API HTTP calls, FX provider fetch/cache write path, notification/PPPoE orchestration helpers).

- **Modify** `src-tauri/src/services/payment_service/dto.rs`
  - Internal SQL row DTO structs used by extracted repository routines.

- **Modify** `src-tauri/src/services/payment_service/mapper.rs`
  - Keep recipient filtering/mapping helpers only.

- **Modify** `src-tauri/src/services/payment_service/validation.rs`
  - Keep role and input validation helpers only.

### New focused internal modules (create)

- **Create** `src-tauri/src/services/payment_service/billing_collection.rs`
  - Private billing collection/scheduler internals currently inside `mod.rs`.
  - Includes per-tenant run loop internals and reminder/suspend/resume orchestration private methods.

- **Create** `src-tauri/src/services/payment_service/subscription_flow.rs`
  - Private activation/installation/resume flow internals currently in `mod.rs`.
  - Includes work-order ensure/upsert assignment related private orchestration.

- **Create** `src-tauri/src/services/payment_service/notifications.rs`
  - Private notification dispatch helper methods and recipient aggregation routines.

- **Modify** `src-tauri/src/services/payment_service/tests.rs`
  - Add or adjust characterization tests for extracted pure/internal behavior when needed.

## API-Stability Migration Strategy

1. Keep all existing `pub async fn` methods in `mod.rs` with identical signatures and return types.
2. Extract only private/helper methods first, preserving method names and call order.
3. Use `pub(super)` for extracted internals so visibility remains module-local to payment service.
4. Do not alter command/http caller behavior; no required behavioral edits in:
   - `src-tauri/src/http/payment.rs`
   - `src-tauri/src/commands/payment/**`
5. Keep existing module entrypoint `crate::services::payment_service::PaymentService` unchanged.
6. After each extraction batch, run targeted tests and `cargo test --lib` before continuing.

## Execution Tasks

### Task 1: Baseline verification before refactor

**Files:**
- Modify: none
- Verify against: `src-tauri/src/services/payment_service/mod.rs`
- Test: `src-tauri/src/services/payment_service/tests.rs`

- [ ] Run formatter baseline.
- [ ] Run targeted payment-service tests baseline.
- [ ] Run full lib baseline to lock current behavior before file movement.

Run:
```bash
cd src-tauri && cargo fmt --all
cd src-tauri && cargo test payment_service --lib
cd src-tauri && cargo test --lib
```

Expected:
- Formatting passes.
- Payment service tests pass.
- Library tests pass before refactor starts.

### Task 2: Prepare module scaffolding and stable facade wiring

**Files:**
- Create: `src-tauri/src/services/payment_service/billing_collection.rs`
- Create: `src-tauri/src/services/payment_service/subscription_flow.rs`
- Create: `src-tauri/src/services/payment_service/notifications.rs`
- Modify: `src-tauri/src/services/payment_service/mod.rs`

- [ ] Add new `mod billing_collection;`, `mod subscription_flow;`, `mod notifications;` declarations in `mod.rs`.
- [ ] Keep all public `PaymentService` methods and public structs in `mod.rs` unchanged.
- [ ] Add minimal internal wiring for extracted private helpers with `impl PaymentService` split across files.

Run:
```bash
cd src-tauri && cargo test payment_service --lib
```

Expected:
- Compiles with unchanged public API surface.
- No caller-facing breakage.

### Task 3: Extract pure/helper logic into core and keep behavior fixed

**Files:**
- Modify: `src-tauri/src/services/payment_service/core.rs`
- Modify: `src-tauri/src/services/payment_service/mod.rs`

- [ ] Move pure helper functions from `mod.rs` into `core.rs` (currency/date/billing/reminder/stateless helpers only).
- [ ] Keep existing behavior and edge-case handling identical.
- [ ] Replace old local calls with `core::*` calls without changing branching or defaults.

Run:
```bash
cd src-tauri && cargo test payment_service --lib
cd src-tauri && cargo test midtrans_transition_decision_prevents_duplicate_or_downgrade_side_effects --lib
```

Expected:
- Existing pure-behavior tests pass.
- No logic drift in transition/parse/format helpers.

### Task 4: Extract DB-heavy internals to repository and DTO support

**Files:**
- Modify: `src-tauri/src/services/payment_service/repository.rs`
- Modify: `src-tauri/src/services/payment_service/dto.rs`
- Modify: `src-tauri/src/services/payment_service/mod.rs`

- [ ] Move DB-only private helpers from `mod.rs` into `repository.rs` using `impl PaymentService` private methods.
- [ ] Move supporting internal row structs into `dto.rs` when they are shared across extracted routines.
- [ ] Keep SQL queries, bind order, and error mapping unchanged.

Run:
```bash
cd src-tauri && cargo test payment_service --lib
cd src-tauri && cargo test --lib
```

Expected:
- Query behavior unchanged.
- No signature change for public methods.

### Task 5: Extract external integrations and notification orchestration

**Files:**
- Modify: `src-tauri/src/services/payment_service/integration.rs`
- Create: `src-tauri/src/services/payment_service/notifications.rs`
- Modify: `src-tauri/src/services/payment_service/mod.rs`
- Modify: `src-tauri/src/services/payment_service/mapper.rs`
- Modify: `src-tauri/src/services/payment_service/validation.rs`

- [ ] Move Midtrans and FX external interaction internals into `integration.rs` while preserving request/response and error text semantics.
- [ ] Move notification dispatch helpers and recipient list private helpers to `notifications.rs`.
- [ ] Keep role filtering/validation responsibilities in `mapper.rs` and `validation.rs` only.

Run:
```bash
cd src-tauri && cargo test payment_service --lib
cd src-tauri && cargo test customer_package_external_id_detection --lib
```

Expected:
- External integration behavior remains identical.
- Notification routing behavior remains identical.

### Task 6: Extract billing collection and subscription-flow internals

**Files:**
- Create: `src-tauri/src/services/payment_service/billing_collection.rs`
- Create: `src-tauri/src/services/payment_service/subscription_flow.rs`
- Modify: `src-tauri/src/services/payment_service/mod.rs`

- [ ] Move private billing collection internals into `billing_collection.rs`.
- [ ] Move private activation/installation/resume internals into `subscription_flow.rs`.
- [ ] Keep scheduler cadence, reminder checks, suspend/resume conditions, and installation flow side effects unchanged.

Run:
```bash
cd src-tauri && cargo test payment_service --lib
cd src-tauri && cargo test --lib
```

Expected:
- Billing and lifecycle flows preserve existing outcomes.
- Full lib tests remain green.

### Task 7: Final stabilization and no-behavior-change verification gate

**Files:**
- Modify: `src-tauri/src/services/payment_service/mod.rs`
- Modify: `src-tauri/src/services/payment_service/tests.rs`

- [ ] Remove leftover duplicated private code from `mod.rs` after extraction.
- [ ] Ensure `mod.rs` remains the stable public facade for all existing public methods.
- [ ] Add/adjust focused characterization tests only where extraction risk exists.
- [ ] Run complete verification sequence.

Run:
```bash
cd src-tauri && cargo fmt --all
cd src-tauri && cargo test payment_service --lib
cd src-tauri && cargo test --lib
```

Expected:
- Formatting clean.
- Targeted and full library tests pass.
- No command/http behavioral edits required.

## Rollback Strategy

If any task fails verification:

- [ ] Revert only touched payment service files for that task.
- [ ] Re-run targeted payment service tests.
- [ ] Re-run `cargo test --lib` before resuming.

Rollback template:
```bash
git restore --staged --worktree src-tauri/src/services/payment_service/mod.rs src-tauri/src/services/payment_service/core.rs src-tauri/src/services/payment_service/repository.rs src-tauri/src/services/payment_service/integration.rs src-tauri/src/services/payment_service/dto.rs src-tauri/src/services/payment_service/mapper.rs src-tauri/src/services/payment_service/validation.rs src-tauri/src/services/payment_service/billing_collection.rs src-tauri/src/services/payment_service/subscription_flow.rs src-tauri/src/services/payment_service/notifications.rs src-tauri/src/services/payment_service/tests.rs
cd src-tauri && cargo test payment_service --lib
cd src-tauri && cargo test --lib
```

## Acceptance Criteria

- `PaymentService` public API signatures remain unchanged.
- Command/http callers continue working without behavioral edits.
- `mod.rs` is reduced and focused as a stable facade.
- Extracted modules have clear single responsibilities.
- `cargo fmt --all`, targeted payment tests, and `cargo test --lib` pass.

## Brief Self-Review

### 1. Spec coverage check

- Decomposition boundaries with exact create/modify paths: covered in File Plan and per-task file lists.
- Verification requirements (`cargo fmt`, targeted tests, `cargo test --lib`): covered in Tasks 1, 3, 5, 6, 7.
- Migration/API stability for command/http callers: covered in API-Stability Migration Strategy and Acceptance Criteria.
- Checkbox executable tasks: covered in Tasks 1-7 with command blocks and expected outcomes.
- Required header format: present at document top.
- Scope limited to planning/payment-service decomposition: satisfied.

### 2. Placeholder scan

- No `TBD`, no deferred placeholders, no unresolved markers.
- All tasks include explicit files, concrete actions, and commands.

### 3. Consistency check

- Public-method stability in `mod.rs` is enforced consistently across all tasks.
- Module responsibilities are non-overlapping and align with existing `payment_service` structure.
- Verification commands are consistent and repeatable across task boundaries.
