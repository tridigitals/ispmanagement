# Rust Backend Restructure (No-Behavior-Change) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the Rust backend structure under `src-tauri/` for maintainability while preserving all runtime behavior, API payloads, HTTP status semantics, scheduler side effects, and command/HTTP contracts.

**Architecture:** Execute a phased full-backend refactor in strict module order, using a stable facade (`mod.rs`) per refactored module and focused internal units (`core`, `repository`, `integration`, `scheduler`, `dto`, `mapper`, `validation`) only where each is needed. Every phase follows Baseline → Refactor (split + local dedup) → Verify → Freeze with rollback-on-failure discipline before any next phase.

**Tech Stack:** Rust 2021, Tauri v2, Axum, Tokio, SQLx, Cargo test/check, existing in-file `#[cfg(test)]` unit tests and module-scoped characterization tests.

---

## Scope, Constraints, and Safety Rules

- Refactor plan scope is limited to backend files under `src-tauri/` and planning docs under `docs/`.
- No feature additions, no endpoint additions, no auth policy changes, no scheduler behavior redesign, no schema/migration semantic changes.
- No cross-module dedup outside current phase scope.
- Command and HTTP layers remain thin adapters only.
- No new command → HTTP dependency is allowed.
- Every phase must pass both verification gates before freeze:
  - Targeted module tests (`cargo test <selector>`)
  - Workspace validation from `src-tauri` (`cargo check --workspace`)

## Rollback Policy (Mandatory)

If any phase verification gate fails:

1. Stop implementation immediately.
2. Revert only that phase’s touched files.
3. Re-run baseline targeted tests for the module to confirm restored state.
4. Do not start the next phase until rollback state is green.

Rollback command template (fill with the exact path list from each task):

```bash
git restore --staged --worktree <phase-path-1> <phase-path-2> <phase-path-3>
git clean -fd <phase-new-dir-if-any>
cd src-tauri && cargo test <phase-baseline-selector> --lib
```

Expected outcome:
- Reverted files show clean restoration.
- Baseline targeted tests pass again.

## Non-Goals

- Redesigning API contracts, DTO schema, or HTTP payload formats.
- Altering side-effect timing (backup scheduler, email sender, alert loops, security refresh loops).
- Replacing SQL queries with new query logic beyond structural extraction.
- Introducing architectural coupling between command and HTTP modules.

## File-Structure Mapping (Create/Modify + Responsibility)

### Shared orchestrator and module map

- **Modify** `src-tauri/src/services/mod.rs`
  - Keep stable exports while switching service modules from single-file implementations to directory `mod.rs` facades.
- **Modify** `src-tauri/src/lib.rs`
  - Split app/bootstrap wiring into dedicated bootstrap units while preserving initialization sequence.
- **Modify** `src-tauri/src/http/mod.rs`
  - Split router/startup internals into thin bootstrap helpers while preserving route registration and middleware behavior.

### Phase 1: announcements/support dedup

- **Modify** `src-tauri/src/commands/announcements.rs`
- **Modify** `src-tauri/src/commands/support.rs`
- **Create** `src-tauri/src/commands/announcements_support_common.rs`
  - Shared command-layer validation/mapping helpers for announcements/support only.
- **Modify** `src-tauri/src/commands/mod.rs`
  - Export new common helper module.
- **Modify** `src-tauri/src/http/announcements.rs`
- **Modify** `src-tauri/src/http/support.rs`
- **Create** `src-tauri/src/http/announcements_support_common.rs`
  - Shared HTTP-layer request parsing/response mapping helpers for announcements/support only.
- **Modify** `src-tauri/src/http/mod.rs`
  - Export HTTP common helper module.

### Phase 2: db bootstrap split

- **Create** `src-tauri/src/db/connection/mod.rs`
- **Create** `src-tauri/src/db/connection/bootstrap.rs`
- **Create** `src-tauri/src/db/connection/migrations.rs`
- **Create** `src-tauri/src/db/connection/seed.rs`
- **Modify** `src-tauri/src/db/mod.rs`
  - Keep stable public exports (`init_db`, `seed_defaults`) via facade.
- **Delete/Rename** `src-tauri/src/db/connection.rs` → `src-tauri/src/db/connection/mod.rs`

### Phase 3: app/http bootstrap split

- **Create** `src-tauri/src/bootstrap/mod.rs`
- **Create** `src-tauri/src/bootstrap/app.rs`
- **Create** `src-tauri/src/bootstrap/http.rs`
- **Modify** `src-tauri/src/lib.rs`
  - Delegate setup/bootstrap logic to `bootstrap::*` while preserving sequence.
- **Modify** `src-tauri/src/http/mod.rs`
  - Extract router-build/start-server internals into callable units used by `bootstrap/http.rs`.

### Phase 4: payment service split

- **Create** `src-tauri/src/services/payment_service/mod.rs`
- **Create** `src-tauri/src/services/payment_service/core.rs`
- **Create** `src-tauri/src/services/payment_service/repository.rs`
- **Create** `src-tauri/src/services/payment_service/dto.rs`
- **Create** `src-tauri/src/services/payment_service/mapper.rs`
- **Create** `src-tauri/src/services/payment_service/validation.rs`
- **Create** `src-tauri/src/services/payment_service/integration.rs`
- **Modify** `src-tauri/src/services/mod.rs`
- **Delete/Rename** `src-tauri/src/services/payment_service.rs` → `src-tauri/src/services/payment_service/mod.rs`

### Phase 5: mikrotik service split

- **Create** `src-tauri/src/services/mikrotik_service/mod.rs`
- **Create** `src-tauri/src/services/mikrotik_service/core.rs`
- **Create** `src-tauri/src/services/mikrotik_service/repository.rs`
- **Create** `src-tauri/src/services/mikrotik_service/dto.rs`
- **Create** `src-tauri/src/services/mikrotik_service/mapper.rs`
- **Create** `src-tauri/src/services/mikrotik_service/validation.rs`
- **Create** `src-tauri/src/services/mikrotik_service/integration.rs`
- **Modify** `src-tauri/src/services/mod.rs`
- **Delete/Rename** `src-tauri/src/services/mikrotik_service.rs` → `src-tauri/src/services/mikrotik_service/mod.rs`

### Phase 6: customer service split

- **Create** `src-tauri/src/services/customer_service/mod.rs`
- **Modify** existing internals:
  - `src-tauri/src/services/customer_service/helpers.rs`
  - `src-tauri/src/services/customer_service/lifecycle.rs`
  - `src-tauri/src/services/customer_service/portal.rs`
  - `src-tauri/src/services/customer_service/registration.rs`
  - `src-tauri/src/services/customer_service/reschedule.rs`
  - `src-tauri/src/services/customer_service/subscriptions.rs`
  - `src-tauri/src/services/customer_service/work_orders.rs`
- **Create** `src-tauri/src/services/customer_service/core.rs`
- **Create** `src-tauri/src/services/customer_service/repository.rs`
- **Create** `src-tauri/src/services/customer_service/dto.rs`
- **Create** `src-tauri/src/services/customer_service/mapper.rs`
- **Create** `src-tauri/src/services/customer_service/validation.rs`
- **Modify** `src-tauri/src/services/mod.rs`
- **Delete/Rename** `src-tauri/src/services/customer_service.rs` → `src-tauri/src/services/customer_service/mod.rs`

### Phase 7: network_mapping service split

- **Create** `src-tauri/src/services/network_mapping_service/mod.rs`
- **Create** `src-tauri/src/services/network_mapping_service/core.rs`
- **Create** `src-tauri/src/services/network_mapping_service/repository.rs`
- **Create** `src-tauri/src/services/network_mapping_service/dto.rs`
- **Create** `src-tauri/src/services/network_mapping_service/mapper.rs`
- **Create** `src-tauri/src/services/network_mapping_service/validation.rs`
- **Create** `src-tauri/src/services/network_mapping_service/integration.rs`
- **Modify** `src-tauri/src/services/mod.rs`
- **Delete/Rename** `src-tauri/src/services/network_mapping_service.rs` → `src-tauri/src/services/network_mapping_service/mod.rs`

### Phase 8: auth service split

- **Create** `src-tauri/src/services/auth_service/mod.rs`
- **Create** `src-tauri/src/services/auth_service/core.rs`
- **Create** `src-tauri/src/services/auth_service/repository.rs`
- **Create** `src-tauri/src/services/auth_service/dto.rs`
- **Create** `src-tauri/src/services/auth_service/mapper.rs`
- **Create** `src-tauri/src/services/auth_service/validation.rs`
- **Create** `src-tauri/src/services/auth_service/integration.rs`
- **Modify** `src-tauri/src/services/mod.rs`
- **Delete/Rename** `src-tauri/src/services/auth_service.rs` → `src-tauri/src/services/auth_service/mod.rs`

---

## Phase-by-Phase Execution Tasks

## Phase 1 — Announcements/Support Dedup (`commands/*` + `http/*`)

### Task 1.1: Baseline characterization for announcements/support adapters

**Files:**
- Modify: `src-tauri/src/commands/announcements.rs`
- Modify: `src-tauri/src/commands/support.rs`
- Modify: `src-tauri/src/http/announcements.rs`
- Modify: `src-tauri/src/http/support.rs`
- Test: same files (`#[cfg(test)]` modules)

- [ ] Add characterization tests that lock current response/status mapping and error conversion behavior for announcements command handlers.
- [ ] Add characterization tests that lock current response/status mapping and error conversion behavior for support command handlers.
- [ ] Add characterization tests for HTTP request parsing and response envelope behavior for announcements routes.
- [ ] Add characterization tests for HTTP request parsing and response envelope behavior for support routes.
- [ ] Run targeted tests to capture baseline behavior.

Run:
```bash
cd src-tauri && cargo test announcements --lib
cd src-tauri && cargo test support --lib
```

Expected:
- Existing + new characterization tests pass.
- No behavior drift reported.

### Task 1.2: Extract local dedup helpers without changing adapters

**Files:**
- Create: `src-tauri/src/commands/announcements_support_common.rs`
- Modify: `src-tauri/src/commands/announcements.rs`
- Modify: `src-tauri/src/commands/support.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/http/announcements_support_common.rs`
- Modify: `src-tauri/src/http/announcements.rs`
- Modify: `src-tauri/src/http/support.rs`
- Modify: `src-tauri/src/http/mod.rs`
- Test: `src-tauri/src/commands/announcements.rs`, `src-tauri/src/commands/support.rs`, `src-tauri/src/http/announcements.rs`, `src-tauri/src/http/support.rs`

- [ ] Move duplicated pure helper logic from announcement/support commands into `commands/announcements_support_common.rs`.
- [ ] Replace duplicate command code with helper calls, preserving function signatures and return types.
- [ ] Move duplicated HTTP helper logic into `http/announcements_support_common.rs`.
- [ ] Replace duplicate HTTP code with helper calls, preserving route handler signatures and payload shapes.
- [ ] Ensure no command module imports HTTP modules.

Run:
```bash
cd src-tauri && cargo test announcements --lib
cd src-tauri && cargo test support --lib
cd src-tauri && cargo check --workspace
```

Expected:
- `cargo test announcements --lib` passes.
- `cargo test support --lib` passes.
- `cargo check --workspace` succeeds with no new compile errors.

### Task 1.3: Freeze + acceptance + rollback gate

**Files:**
- Modify: `docs/superpowers/plans/2026-03-27-rust-backend-restructure.md` (phase notes section updates during execution)

- [ ] Record touched paths and helper extraction summary.
- [ ] Confirm no additional opportunistic refactors were included.
- [ ] If any gate failed, rollback exactly these paths:
  - `src-tauri/src/commands/announcements.rs`
  - `src-tauri/src/commands/support.rs`
  - `src-tauri/src/commands/announcements_support_common.rs`
  - `src-tauri/src/commands/mod.rs`
  - `src-tauri/src/http/announcements.rs`
  - `src-tauri/src/http/support.rs`
  - `src-tauri/src/http/announcements_support_common.rs`
  - `src-tauri/src/http/mod.rs`

Rollback:
```bash
git restore --staged --worktree src-tauri/src/commands/announcements.rs src-tauri/src/commands/support.rs src-tauri/src/commands/announcements_support_common.rs src-tauri/src/commands/mod.rs src-tauri/src/http/announcements.rs src-tauri/src/http/support.rs src-tauri/src/http/announcements_support_common.rs src-tauri/src/http/mod.rs
git clean -fd src-tauri/src/commands/announcements_support_common.rs src-tauri/src/http/announcements_support_common.rs
cd src-tauri && cargo test announcements --lib
cd src-tauri && cargo test support --lib
```

Expected:
- On success path, no rollback needed.
- On failure path, restored baseline tests pass.

**Phase 1 acceptance criteria:**
- Announcements/support behavior unchanged.
- Dedup limited to announcements/support scope.
- No command → HTTP dependency introduced.
- Targeted tests + workspace check passed.

**Phase 1 freeze note (Task 1.3, 2026-03-27):**
- Completed commits:
  - Task 1.1: `452d7f8` (`test(adapters): add baseline characterization for announcements and support`)
  - Task 1.2: `3207305` (`refactor(adapters): extract local announcements/support dedup helpers`)
- Verification summary:
  - `cargo test announcements --lib` ✅
  - `cargo test support --lib` ✅
  - `cargo check --workspace` ✅
- Scope boundary confirmed: only Phase 1 announcements/support command+HTTP adapter files were touched; no Phase 2 (`src-tauri/src/db/**`) files touched.

---

## Phase 2 — DB Bootstrap Split (`src-tauri/src/db/connection.rs`)

### Task 2.1: Characterize DB bootstrap contract before split

**Files:**
- Modify: `src-tauri/src/db/connection.rs`
- Test: `src-tauri/src/db/connection.rs` (`#[cfg(test)]` module)

- [ ] Add/extend characterization tests for `init_db` and `seed_defaults` observable outcomes (successful pool init, expected idempotent seed behavior, expected error propagation).
- [ ] Ensure tests assert behavior, not internal implementation details.
- [ ] Run baseline DB-targeted tests.

Run:
```bash
cd src-tauri && cargo test connection --lib
```

Expected:
- Characterization tests pass and define contract for split.

### Task 2.2: Split connection module into facade + focused units

**Files:**
- Create: `src-tauri/src/db/connection/mod.rs`
- Create: `src-tauri/src/db/connection/bootstrap.rs`
- Create: `src-tauri/src/db/connection/migrations.rs`
- Create: `src-tauri/src/db/connection/seed.rs`
- Modify: `src-tauri/src/db/mod.rs`
- Delete/Rename: `src-tauri/src/db/connection.rs` → `src-tauri/src/db/connection/mod.rs`
- Test: `src-tauri/src/db/connection/mod.rs`, `src-tauri/src/db/connection/bootstrap.rs`, `src-tauri/src/db/connection/seed.rs`

- [ ] Move public API (`init_db`, `seed_defaults`) to `connection/mod.rs` facade re-exporting internal units.
- [ ] Move pool/bootstrap responsibilities to `bootstrap.rs`.
- [ ] Move migration application responsibilities to `migrations.rs`.
- [ ] Move default-seeding logic to `seed.rs`.
- [ ] Keep function signatures used by `src-tauri/src/lib.rs` unchanged.

Run:
```bash
cd src-tauri && cargo test connection --lib
cd src-tauri && cargo check --workspace
```

Expected:
- DB tests pass with unchanged behavior.
- Workspace compiles successfully.

### Task 2.3: Freeze + acceptance + rollback gate

**Files:**
- Modify: `docs/superpowers/plans/2026-03-27-rust-backend-restructure.md` (phase notes section updates during execution)

- [ ] Record split map from old file to new internal files.
- [ ] Confirm no schema/migration semantic changes.
- [ ] If gate fails, rollback exactly:
  - `src-tauri/src/db/connection/mod.rs`
  - `src-tauri/src/db/connection/bootstrap.rs`
  - `src-tauri/src/db/connection/migrations.rs`
  - `src-tauri/src/db/connection/seed.rs`
  - `src-tauri/src/db/mod.rs`

Rollback:
```bash
git restore --staged --worktree src-tauri/src/db/mod.rs src-tauri/src/db/connection/mod.rs src-tauri/src/db/connection/bootstrap.rs src-tauri/src/db/connection/migrations.rs src-tauri/src/db/connection/seed.rs
git clean -fd src-tauri/src/db/connection
cd src-tauri && cargo test connection --lib
```

Expected:
- Baseline `connection` tests pass after rollback when required.

**Phase 2 acceptance criteria:**
- `init_db` / `seed_defaults` contracts unchanged.
- Internal split implemented with stable facade.
- Targeted tests + workspace check passed.

**Phase 2 freeze note (Task 2.3, 2026-03-27):**
- Completed commits:
  - Task 2.1: `1c35e85` (`test(db): replace brittle connection source assertions with behavior checks`)
  - Task 2.2: `fd65a61` (`refactor(db): split connection module into facade and focused units`)
- Verification summary:
  - `cargo test connection --lib` ✅
  - `cargo check --workspace` ✅
- Scope boundary confirmed: no Phase 3 files were touched; changes remain in Phase 2 DB scope (`src-tauri/src/db/**`) plus this plan freeze note.

---

## Phase 3 — App/HTTP Bootstrap Split

### Task 3.1: Baseline bootstrap sequencing tests

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/http/mod.rs`
- Test: `src-tauri/src/http/mod.rs` and bootstrap-related testable helpers in `src-tauri/src/lib.rs`

- [ ] Add characterization tests for route construction invariants and startup dependency ordering boundaries that can be tested without changing runtime behavior.
- [ ] Add tests for thin-adapter expectation in HTTP bootstrap (router build remains transport orchestration only).
- [ ] Run baseline HTTP-targeted tests.

Run:
```bash
cd src-tauri && cargo test http --lib
```

Expected:
- Baseline startup/router tests pass.

### Task 3.2: Extract bootstrap internals into dedicated modules

**Files:**
- Create: `src-tauri/src/bootstrap/mod.rs`
- Create: `src-tauri/src/bootstrap/app.rs`
- Create: `src-tauri/src/bootstrap/http.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/http/mod.rs`
- Test: `src-tauri/src/bootstrap/app.rs`, `src-tauri/src/bootstrap/http.rs`, `src-tauri/src/http/mod.rs`

- [ ] Move app bootstrap orchestration code from `lib.rs` into `bootstrap/app.rs`.
- [ ] Move HTTP server bootstrap orchestration into `bootstrap/http.rs`.
- [ ] Keep `run()` behavior identical (same service initialization order and side-effect scheduling points).
- [ ] Keep `http` module adapters thin and avoid domain logic relocation into HTTP layer.

Run:
```bash
cd src-tauri && cargo test http --lib
cd src-tauri && cargo check --workspace
```

Expected:
- HTTP-targeted tests pass.
- Workspace check passes with no new behavior-impacting compile warnings/errors.

### Task 3.3: Freeze + acceptance + rollback gate

**Files:**
- Modify: `docs/superpowers/plans/2026-03-27-rust-backend-restructure.md` (phase notes section updates during execution)

- [ ] Record startup and HTTP split map.
- [ ] Confirm no changes to startup side-effect order (backup scheduler, alert loop, email sender, security refresh).
- [ ] If gate fails, rollback exactly:
  - `src-tauri/src/bootstrap/mod.rs`
  - `src-tauri/src/bootstrap/app.rs`
  - `src-tauri/src/bootstrap/http.rs`
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/http/mod.rs`

Rollback:
```bash
git restore --staged --worktree src-tauri/src/bootstrap/mod.rs src-tauri/src/bootstrap/app.rs src-tauri/src/bootstrap/http.rs src-tauri/src/lib.rs src-tauri/src/http/mod.rs
git clean -fd src-tauri/src/bootstrap
cd src-tauri && cargo test http --lib
```

Expected:
- Baseline HTTP-targeted tests pass after rollback when required.

**Phase 3 acceptance criteria:**
- Startup + HTTP bootstrap internals split with stable behavior.
- No new command → HTTP coupling.
- Targeted tests + workspace check passed.

**Phase 3 freeze note (Task 3.3, 2026-03-27):**
- Completed commits:
  - Task 3.1: `ce06b4f` (`test(rust): strengthen bootstrap and router sequencing characterization`)
  - Task 3.2: `f5cc2fb` (`refactor(bootstrap): extract app/http internals with stable behavior`)
- Verification summary:
  - `cargo test http --lib` ✅
  - `cargo check --workspace` ✅
- Scope boundary confirmed: no Phase 4 files were touched; Phase 3 changes remain within `src-tauri/src/bootstrap/**`, `src-tauri/src/http/mod.rs`, `src-tauri/src/lib.rs`, plus this plan freeze note.

---

## Phase 4 — Payment Service Split

### Task 4.1: Payment characterization tests (contract lock)

**Files:**
- Modify: `src-tauri/src/services/payment_service.rs`
- Test: `src-tauri/src/services/payment_service.rs`

- [ ] Add characterization tests for invoice generation, payment recording, and result mapping behaviors currently exposed by `PaymentService`.
- [ ] Assert externally observable outputs and error semantics only.
- [ ] Run baseline payment tests.

Run:
```bash
cd src-tauri && cargo test payment_service --lib
```

Expected:
- Payment baseline tests pass and lock behavior.

### Task 4.2: Split payment service into facade + focused units

**Files:**
- Create: `src-tauri/src/services/payment_service/mod.rs`
- Create: `src-tauri/src/services/payment_service/core.rs`
- Create: `src-tauri/src/services/payment_service/repository.rs`
- Create: `src-tauri/src/services/payment_service/dto.rs`
- Create: `src-tauri/src/services/payment_service/mapper.rs`
- Create: `src-tauri/src/services/payment_service/validation.rs`
- Create: `src-tauri/src/services/payment_service/integration.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Delete/Rename: `src-tauri/src/services/payment_service.rs` → `src-tauri/src/services/payment_service/mod.rs`
- Test: `src-tauri/src/services/payment_service/mod.rs` and submodules as needed

- [ ] Keep `PaymentService` public API and exported types unchanged (`BillingCollectionRunResult`, `BulkGenerateInvoicesResult`, `PaymentService`).
- [ ] Move pure business rules to `core.rs`.
- [ ] Move DB calls to `repository.rs`.
- [ ] Move struct/request-response shaping to `dto.rs` + `mapper.rs`.
- [ ] Move input guards to `validation.rs`.
- [ ] Keep third-party or cross-service adapters in `integration.rs`.

Run:
```bash
cd src-tauri && cargo test payment_service --lib
cd src-tauri && cargo check --workspace
```

Expected:
- Payment tests pass with unchanged behavior.
- Workspace check succeeds.

### Task 4.3: Freeze + acceptance + rollback gate

**Files:**
- Modify: `docs/superpowers/plans/2026-03-27-rust-backend-restructure.md` (phase notes section updates during execution)

- [ ] Record payment split map and local dedup summary.
- [ ] Confirm no changes to HTTP/command payloads fed by payment flows.
- [ ] If gate fails, rollback exactly:
  - `src-tauri/src/services/payment_service/mod.rs`
  - `src-tauri/src/services/payment_service/core.rs`
  - `src-tauri/src/services/payment_service/repository.rs`
  - `src-tauri/src/services/payment_service/dto.rs`
  - `src-tauri/src/services/payment_service/mapper.rs`
  - `src-tauri/src/services/payment_service/validation.rs`
  - `src-tauri/src/services/payment_service/integration.rs`
  - `src-tauri/src/services/mod.rs`

Rollback:
```bash
git restore --staged --worktree src-tauri/src/services/payment_service/mod.rs src-tauri/src/services/payment_service/core.rs src-tauri/src/services/payment_service/repository.rs src-tauri/src/services/payment_service/dto.rs src-tauri/src/services/payment_service/mapper.rs src-tauri/src/services/payment_service/validation.rs src-tauri/src/services/payment_service/integration.rs src-tauri/src/services/mod.rs
git clean -fd src-tauri/src/services/payment_service
cd src-tauri && cargo test payment_service --lib
```

Expected:
- Baseline payment tests pass after rollback when required.

**Phase 4 acceptance criteria:**
- Payment service split with stable facade and unchanged outputs.
- Local dedup only within payment scope.
- Targeted tests + workspace check passed.

---

## Phase 5 — Mikrotik Service Split

### Task 5.1: Mikrotik characterization tests

**Files:**
- Modify: `src-tauri/src/services/mikrotik_service.rs`
- Test: `src-tauri/src/services/mikrotik_service.rs`

- [ ] Add characterization tests for connection/session behavior, sync behavior, and error translation currently exposed by `MikrotikService`.
- [ ] Keep assertions focused on public method contracts.
- [ ] Run baseline mikrotik-targeted tests.

Run:
```bash
cd src-tauri && cargo test mikrotik_service --lib
```

Expected:
- Baseline tests pass and lock behavior.

### Task 5.2: Split mikrotik service into internal boundaries

**Files:**
- Create: `src-tauri/src/services/mikrotik_service/mod.rs`
- Create: `src-tauri/src/services/mikrotik_service/core.rs`
- Create: `src-tauri/src/services/mikrotik_service/repository.rs`
- Create: `src-tauri/src/services/mikrotik_service/dto.rs`
- Create: `src-tauri/src/services/mikrotik_service/mapper.rs`
- Create: `src-tauri/src/services/mikrotik_service/validation.rs`
- Create: `src-tauri/src/services/mikrotik_service/integration.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Delete/Rename: `src-tauri/src/services/mikrotik_service.rs` → `src-tauri/src/services/mikrotik_service/mod.rs`
- Test: `src-tauri/src/services/mikrotik_service/mod.rs` and submodules as needed

- [ ] Keep `MikrotikService` constructor and public methods unchanged.
- [ ] Isolate router/device integration code in `integration.rs`.
- [ ] Keep repository DB access isolated in `repository.rs`.
- [ ] Keep validation rules in `validation.rs` and DTO mapping in `dto.rs`/`mapper.rs`.

Run:
```bash
cd src-tauri && cargo test mikrotik_service --lib
cd src-tauri && cargo check --workspace
```

Expected:
- Mikrotik tests pass.
- Workspace check passes.

### Task 5.3: Freeze + acceptance + rollback gate

**Files:**
- Modify: `docs/superpowers/plans/2026-03-27-rust-backend-restructure.md` (phase notes section updates during execution)

- [ ] Record mikrotik split map and risk mitigations.
- [ ] Confirm no behavior change in external router interaction semantics.
- [ ] If gate fails, rollback exactly:
  - `src-tauri/src/services/mikrotik_service/mod.rs`
  - `src-tauri/src/services/mikrotik_service/core.rs`
  - `src-tauri/src/services/mikrotik_service/repository.rs`
  - `src-tauri/src/services/mikrotik_service/dto.rs`
  - `src-tauri/src/services/mikrotik_service/mapper.rs`
  - `src-tauri/src/services/mikrotik_service/validation.rs`
  - `src-tauri/src/services/mikrotik_service/integration.rs`
  - `src-tauri/src/services/mod.rs`

Rollback:
```bash
git restore --staged --worktree src-tauri/src/services/mikrotik_service/mod.rs src-tauri/src/services/mikrotik_service/core.rs src-tauri/src/services/mikrotik_service/repository.rs src-tauri/src/services/mikrotik_service/dto.rs src-tauri/src/services/mikrotik_service/mapper.rs src-tauri/src/services/mikrotik_service/validation.rs src-tauri/src/services/mikrotik_service/integration.rs src-tauri/src/services/mod.rs
git clean -fd src-tauri/src/services/mikrotik_service
cd src-tauri && cargo test mikrotik_service --lib
```

Expected:
- Baseline mikrotik tests pass after rollback when required.

**Phase 5 acceptance criteria:**
- Mikrotik service split with stable public contract.
- Integration/repository/validation boundaries are explicit.
- Targeted tests + workspace check passed.

---

## Phase 6 — Customer Service Split

### Task 6.1: Customer characterization tests

**Files:**
- Modify: `src-tauri/src/services/customer_service.rs`
- Test: `src-tauri/src/services/customer_service.rs`

- [ ] Expand characterization tests for customer lifecycle, registration, portal, reschedule, subscription, and work-order behavior currently exposed.
- [ ] Lock existing status/result semantics used by commands/HTTP adapters.
- [ ] Run baseline customer-targeted tests.

Run:
```bash
cd src-tauri && cargo test customer_service --lib
```

Expected:
- Customer baseline tests pass.

### Task 6.2: Convert customer service to directory facade using existing submodules

**Files:**
- Create: `src-tauri/src/services/customer_service/mod.rs`
- Create: `src-tauri/src/services/customer_service/core.rs`
- Create: `src-tauri/src/services/customer_service/repository.rs`
- Create: `src-tauri/src/services/customer_service/dto.rs`
- Create: `src-tauri/src/services/customer_service/mapper.rs`
- Create: `src-tauri/src/services/customer_service/validation.rs`
- Modify: `src-tauri/src/services/customer_service/helpers.rs`
- Modify: `src-tauri/src/services/customer_service/lifecycle.rs`
- Modify: `src-tauri/src/services/customer_service/portal.rs`
- Modify: `src-tauri/src/services/customer_service/registration.rs`
- Modify: `src-tauri/src/services/customer_service/reschedule.rs`
- Modify: `src-tauri/src/services/customer_service/subscriptions.rs`
- Modify: `src-tauri/src/services/customer_service/work_orders.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Delete/Rename: `src-tauri/src/services/customer_service.rs` → `src-tauri/src/services/customer_service/mod.rs`
- Test: `src-tauri/src/services/customer_service/mod.rs` + existing submodule tests

- [ ] Preserve `CustomerService` public type and constructor signature.
- [ ] Keep existing submodule intent and group pure business flows in `core.rs`.
- [ ] Isolate DB logic in `repository.rs` and mapping/validation in dedicated units.
- [ ] Ensure existing helper modules remain internal and are re-exported only through facade when required.

Run:
```bash
cd src-tauri && cargo test customer_service --lib
cd src-tauri && cargo check --workspace
```

Expected:
- Customer tests pass.
- Workspace check passes.

### Task 6.3: Freeze + acceptance + rollback gate

**Files:**
- Modify: `docs/superpowers/plans/2026-03-27-rust-backend-restructure.md` (phase notes section updates during execution)

- [ ] Record customer split map and local dedup notes.
- [ ] Confirm command/HTTP adapters still consume unchanged customer outputs.
- [ ] If gate fails, rollback exactly:
  - `src-tauri/src/services/customer_service/mod.rs`
  - `src-tauri/src/services/customer_service/core.rs`
  - `src-tauri/src/services/customer_service/repository.rs`
  - `src-tauri/src/services/customer_service/dto.rs`
  - `src-tauri/src/services/customer_service/mapper.rs`
  - `src-tauri/src/services/customer_service/validation.rs`
  - `src-tauri/src/services/customer_service/helpers.rs`
  - `src-tauri/src/services/customer_service/lifecycle.rs`
  - `src-tauri/src/services/customer_service/portal.rs`
  - `src-tauri/src/services/customer_service/registration.rs`
  - `src-tauri/src/services/customer_service/reschedule.rs`
  - `src-tauri/src/services/customer_service/subscriptions.rs`
  - `src-tauri/src/services/customer_service/work_orders.rs`
  - `src-tauri/src/services/mod.rs`

Rollback:
```bash
git restore --staged --worktree src-tauri/src/services/customer_service/mod.rs src-tauri/src/services/customer_service/core.rs src-tauri/src/services/customer_service/repository.rs src-tauri/src/services/customer_service/dto.rs src-tauri/src/services/customer_service/mapper.rs src-tauri/src/services/customer_service/validation.rs src-tauri/src/services/customer_service/helpers.rs src-tauri/src/services/customer_service/lifecycle.rs src-tauri/src/services/customer_service/portal.rs src-tauri/src/services/customer_service/registration.rs src-tauri/src/services/customer_service/reschedule.rs src-tauri/src/services/customer_service/subscriptions.rs src-tauri/src/services/customer_service/work_orders.rs src-tauri/src/services/mod.rs
git clean -fd src-tauri/src/services/customer_service
cd src-tauri && cargo test customer_service --lib
```

Expected:
- Baseline customer tests pass after rollback when required.

**Phase 6 acceptance criteria:**
- Customer service is facade-driven with focused internal boundaries.
- Existing customer behavior contracts remain unchanged.
- Targeted tests + workspace check passed.

---

## Phase 7 — Network Mapping Service Split

### Task 7.1: Network mapping characterization tests

**Files:**
- Modify: `src-tauri/src/services/network_mapping_service.rs`
- Test: `src-tauri/src/services/network_mapping_service.rs`

- [ ] Add characterization tests for graph/node/link operations, repository interactions, and validation outcomes.
- [ ] Lock current error and success payload-shaping behavior.
- [ ] Run baseline network mapping tests.

Run:
```bash
cd src-tauri && cargo test network_mapping_service --lib
```

Expected:
- Baseline tests pass.

### Task 7.2: Split network mapping into facade + internal units

**Files:**
- Create: `src-tauri/src/services/network_mapping_service/mod.rs`
- Create: `src-tauri/src/services/network_mapping_service/core.rs`
- Create: `src-tauri/src/services/network_mapping_service/repository.rs`
- Create: `src-tauri/src/services/network_mapping_service/dto.rs`
- Create: `src-tauri/src/services/network_mapping_service/mapper.rs`
- Create: `src-tauri/src/services/network_mapping_service/validation.rs`
- Create: `src-tauri/src/services/network_mapping_service/integration.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Delete/Rename: `src-tauri/src/services/network_mapping_service.rs` → `src-tauri/src/services/network_mapping_service/mod.rs`
- Test: `src-tauri/src/services/network_mapping_service/mod.rs` + submodules as needed

- [ ] Preserve `NetworkMappingService` public API and constructor signature.
- [ ] Move domain rules to `core.rs` and DB logic to `repository.rs`.
- [ ] Keep DTO/mapper/validation responsibilities isolated.

Run:
```bash
cd src-tauri && cargo test network_mapping_service --lib
cd src-tauri && cargo check --workspace
```

Expected:
- Network mapping tests pass.
- Workspace check passes.

### Task 7.3: Freeze + acceptance + rollback gate

**Files:**
- Modify: `docs/superpowers/plans/2026-03-27-rust-backend-restructure.md` (phase notes section updates during execution)

- [ ] Record network mapping split map and risk mitigations.
- [ ] Confirm no API contract drift reaches HTTP/command layers.
- [ ] If gate fails, rollback exactly:
  - `src-tauri/src/services/network_mapping_service/mod.rs`
  - `src-tauri/src/services/network_mapping_service/core.rs`
  - `src-tauri/src/services/network_mapping_service/repository.rs`
  - `src-tauri/src/services/network_mapping_service/dto.rs`
  - `src-tauri/src/services/network_mapping_service/mapper.rs`
  - `src-tauri/src/services/network_mapping_service/validation.rs`
  - `src-tauri/src/services/network_mapping_service/integration.rs`
  - `src-tauri/src/services/mod.rs`

Rollback:
```bash
git restore --staged --worktree src-tauri/src/services/network_mapping_service/mod.rs src-tauri/src/services/network_mapping_service/core.rs src-tauri/src/services/network_mapping_service/repository.rs src-tauri/src/services/network_mapping_service/dto.rs src-tauri/src/services/network_mapping_service/mapper.rs src-tauri/src/services/network_mapping_service/validation.rs src-tauri/src/services/network_mapping_service/integration.rs src-tauri/src/services/mod.rs
git clean -fd src-tauri/src/services/network_mapping_service
cd src-tauri && cargo test network_mapping_service --lib
```

Expected:
- Baseline network mapping tests pass after rollback when required.

**Phase 7 acceptance criteria:**
- Network mapping service refactored into explicit boundaries.
- No behavior changes in mapping workflows.
- Targeted tests + workspace check passed.

---

## Phase 8 — Auth Service Split

### Task 8.1: Auth characterization tests

**Files:**
- Modify: `src-tauri/src/services/auth_service.rs`
- Test: `src-tauri/src/services/auth_service.rs`

- [ ] Add characterization tests for authentication/login flows, token issuance/validation behavior, and error semantics.
- [ ] Lock public contract and status semantics consumed by adapters.
- [ ] Run baseline auth-targeted tests.

Run:
```bash
cd src-tauri && cargo test auth_service --lib
```

Expected:
- Baseline auth tests pass.

### Task 8.2: Split auth service into facade + focused units

**Files:**
- Create: `src-tauri/src/services/auth_service/mod.rs`
- Create: `src-tauri/src/services/auth_service/core.rs`
- Create: `src-tauri/src/services/auth_service/repository.rs`
- Create: `src-tauri/src/services/auth_service/dto.rs`
- Create: `src-tauri/src/services/auth_service/mapper.rs`
- Create: `src-tauri/src/services/auth_service/validation.rs`
- Create: `src-tauri/src/services/auth_service/integration.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Delete/Rename: `src-tauri/src/services/auth_service.rs` → `src-tauri/src/services/auth_service/mod.rs`
- Test: `src-tauri/src/services/auth_service/mod.rs` + submodules as needed

- [ ] Preserve `AuthService` constructor and public methods used by `lib.rs`, `commands/*`, and `http/*`.
- [ ] Isolate DB/token storage in `repository.rs` and domain auth logic in `core.rs`.
- [ ] Keep validation and DTO mapping isolated in dedicated units.

Run:
```bash
cd src-tauri && cargo test auth_service --lib
cd src-tauri && cargo check --workspace
```

Expected:
- Auth tests pass.
- Workspace check passes.

### Task 8.3: Freeze + acceptance + rollback gate

**Files:**
- Modify: `docs/superpowers/plans/2026-03-27-rust-backend-restructure.md` (phase notes section updates during execution)

- [ ] Record auth split map and mitigation notes for auth-risk areas.
- [ ] Confirm no auth policy behavior changes were introduced.
- [ ] If gate fails, rollback exactly:
  - `src-tauri/src/services/auth_service/mod.rs`
  - `src-tauri/src/services/auth_service/core.rs`
  - `src-tauri/src/services/auth_service/repository.rs`
  - `src-tauri/src/services/auth_service/dto.rs`
  - `src-tauri/src/services/auth_service/mapper.rs`
  - `src-tauri/src/services/auth_service/validation.rs`
  - `src-tauri/src/services/auth_service/integration.rs`
  - `src-tauri/src/services/mod.rs`

Rollback:
```bash
git restore --staged --worktree src-tauri/src/services/auth_service/mod.rs src-tauri/src/services/auth_service/core.rs src-tauri/src/services/auth_service/repository.rs src-tauri/src/services/auth_service/dto.rs src-tauri/src/services/auth_service/mapper.rs src-tauri/src/services/auth_service/validation.rs src-tauri/src/services/auth_service/integration.rs src-tauri/src/services/mod.rs
git clean -fd src-tauri/src/services/auth_service
cd src-tauri && cargo test auth_service --lib
```

Expected:
- Baseline auth tests pass after rollback when required.

**Phase 8 acceptance criteria:**
- Auth service split with unchanged external behavior.
- No auth policy redesign introduced.
- Targeted tests + workspace check passed.

---

## Cross-Phase Verification Gate (Run at End of Every Phase)

```bash
cd src-tauri && cargo test <phase-selector> --lib
cd src-tauri && cargo check --workspace
```

Expected:
- Targeted module tests pass.
- Workspace check passes.
- If either fails, rollback phase and do not continue.

## Commit Policy (Per Task/Phase)

After each successful task group:

```bash
git add <phase-files>
git commit -m "refactor(<module>): split internals with stable facade and no behavior change"
```

Expected:
- Small, auditable commits that map to one phase and one verification artifact.

---

## Final Global Completion Checklist

- [ ] Phase 1 complete: announcements/support dedup done with unchanged behavior.
- [ ] Phase 2 complete: db bootstrap split done with stable `init_db`/`seed_defaults` contract.
- [ ] Phase 3 complete: app/http bootstrap split done with unchanged startup and router semantics.
- [ ] Phase 4 complete: payment split done with stable public API.
- [ ] Phase 5 complete: mikrotik split done with stable public API.
- [ ] Phase 6 complete: customer split done with stable public API.
- [ ] Phase 7 complete: network mapping split done with stable public API.
- [ ] Phase 8 complete: auth split done with stable public API.
- [ ] All phase verification gates executed and passed (`cargo test <selector>` + `cargo check --workspace`).
- [ ] No new command → HTTP dependency introduced.
- [ ] No behavior contract changes in payload shapes/status semantics/side effects.
- [ ] Rollback actions executed for failed phases where applicable and documented.

---

## Self-Review

### 1) Spec coverage

- Covered full phased backend strategy with strict order:
  1. announcements/support dedup
  2. db bootstrap split
  3. app/http bootstrap split
  4. payment
  5. mikrotik
  6. customer
  7. network_mapping
  8. auth
- Included mandatory gate per phase: targeted `cargo test` + `cargo check --workspace` in `src-tauri`.
- Included rollback discipline and freeze behavior per phase.
- Included non-goals and no-behavior-change constraints.
- Included explicit per-phase acceptance criteria and final global completion checklist.

### 2) Placeholder scan

- Verified: no `TODO`, `TBD`, `implement later`, `fill in details`, or vague deferred steps.
- Verified: every task contains concrete file paths, ordered checklist steps, exact commands, and expected outcomes.

### 3) Type/signature consistency

- Plan requires preserving existing public service contracts (`PaymentService`, `MikrotikService`, `CustomerService`, `NetworkMappingService`, `AuthService`) through `mod.rs` facades.
- Plan requires preserving db bootstrap signatures (`init_db`, `seed_defaults`) consumed by `src-tauri/src/lib.rs`.
- Plan requires preserving adapter-facing command/HTTP contracts and preventing new command → HTTP dependency edges.
