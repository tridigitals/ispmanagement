# Rust Native RADIUS Big-Bang Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the legacy external RADIUS runtime with a Rust-native RADIUS server inside the existing backend, using the main app database as the only runtime source of truth.

**Architecture:** Build a new `radius_service` bounded context that owns UDP auth/accounting listeners, NAS resolution, and packet processing while keeping PPPoE CRUD and tenant/business logic in the existing services. Migrate runtime account/auth/accounting data into the main app database, refactor managed RADIUS orchestration away from external runtime DB sync, then remove legacy external-runtime deployment assets.

**Tech Stack:** Rust 2021, Tokio UDP runtime, Axum/Tauri bootstrap, SQLx PostgreSQL/SQLite migrations, `radius` crate for packet primitives, MikroTik PPPoE semantics, Rust tests, optional frontend/Vitest regression tests for config/UI cleanup.

---

## Chunk 1: Runtime Foundation

### Task 1: Add crate dependency and native RADIUS service skeleton

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/services/radius_service/mod.rs`
- Create: `src-tauri/src/services/radius_service/config.rs`
- Create: `src-tauri/src/services/radius_service/models.rs`
- Create: `src-tauri/src/services/radius_service/server.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/radius_service/config.rs`

- [ ] Step 1: Write failing Rust tests for config defaults, enable flag parsing, and auth/acct port normalization.
- [ ] Step 2: Run the targeted Rust test command and verify it fails because `radius_service` does not exist yet.
- [ ] Step 3: Add the `radius` dependency and create the `radius_service` module skeleton with config/model/server stubs.
- [ ] Step 4: Re-run the targeted Rust tests and make the config cases pass without implementing packet logic yet.

### Task 2: Register the native runtime in backend bootstrap

**Files:**
- Modify: `src-tauri/src/bootstrap/app.rs`
- Modify: `src-tauri/src/http/mod.rs`
- Test: `src-tauri/src/bootstrap/app.rs`

- [ ] Step 1: Write a failing bootstrap regression test that asserts the RADIUS runtime is initialized after DB/service setup and before the HTTP server starts when enabled.
- [ ] Step 2: Run the targeted Rust test and verify the failure is about the missing runtime integration.
- [ ] Step 3: Add `RadiusService` construction, state registration, and startup invocation to backend bootstrap with explicit failure when bind/start fails under `RADIUS_ENABLED=true`.
- [ ] Step 4: Re-run the targeted bootstrap test until green.

## Chunk 2: Main-DB Runtime Persistence

### Task 3: Add native accounting/session tables

**Files:**
- Migration: `src-tauri/migrations/<timestamp>_create_radius_accounting_sessions.*.sql`
- Migration: `src-tauri/migrations/<timestamp>_create_radius_auth_log.*.sql`
- Create: `src-tauri/src/models/radius.rs`
- Modify: `src-tauri/src/models/mod.rs`
- Test: `src-tauri/src/models/radius.rs`

- [ ] Step 1: Write failing Rust tests for serde/default behavior on session/auth-log models and status-type normalization helpers.
- [ ] Step 2: Run the targeted Rust tests and verify they fail because the models do not exist.
- [ ] Step 3: Add the new models and SQL migrations for accounting sessions and lightweight auth logs.
- [ ] Step 4: Re-run the targeted model tests and keep them green.

### Task 4: Remove external runtime-account dependence from the data model

**Files:**
- Modify: `src-tauri/src/services/managed_radius_service.rs`
- Modify: `src-tauri/src/models/pppoe.rs`
- Test: `src-tauri/src/services/managed_radius_service.rs`

- [ ] Step 1: Write failing Rust tests that capture the new expectation: native RADIUS uses `pppoe_accounts` as the runtime source of truth and no longer requires external `managed_radius_accounts` sync for auth.
- [ ] Step 2: Run the targeted tests and verify they fail for the right reason.
- [ ] Step 3: Refactor `ManagedRadiusService` responsibilities so account runtime behavior no longer assumes an external runtime DB mirror.
- [ ] Step 4: Re-run the targeted tests and make them pass without deleting legacy code paths prematurely.

## Chunk 3: NAS Resolution and Auth Repository

### Task 5: Implement repository lookups for NAS resolution and managed-RADIUS account lookup

**Files:**
- Create: `src-tauri/src/services/radius_service/repository.rs`
- Modify: `src-tauri/src/services/radius_service/mod.rs`
- Test: `src-tauri/src/services/radius_service/repository.rs`

- [ ] Step 1: Write failing Rust tests for:
- [ ] a. longest-prefix NAS CIDR match
- [ ] b. inactive NAS rejection
- [ ] c. tenant/router-constrained PPPoE account lookup with duplicate usernames across tenants
- [ ] Step 2: Run the targeted Rust tests and verify they fail because the repository does not exist yet.
- [ ] Step 3: Implement repository queries against the main DB for NAS/shared-secret resolution, managed-RADIUS account lookup, auth logging, and accounting upserts.
- [ ] Step 4: Re-run the repository tests until green.

### Task 6: Implement reply-attribute builder for MikroTik PPPoE

**Files:**
- Create: `src-tauri/src/services/radius_service/reply.rs`
- Create: `src-tauri/src/services/radius_service/packet.rs`
- Test: `src-tauri/src/services/radius_service/reply.rs`

- [ ] Step 1: Write failing Rust tests for reply construction covering:
- [ ] a. `Mikrotik-Group`
- [ ] b. `Framed-IP-Address`
- [ ] c. `Framed-Pool`
- [ ] d. empty optional fields not being emitted
- [ ] Step 2: Run the targeted Rust tests and verify they fail before implementation.
- [ ] Step 3: Implement the internal reply builder and packet adapter layer around the `radius` crate primitives.
- [ ] Step 4: Re-run the reply tests until green.

## Chunk 4: Access-Request Runtime

### Task 7: Implement PAP auth flow

**Files:**
- Create: `src-tauri/src/services/radius_service/auth.rs`
- Modify: `src-tauri/src/services/radius_service/server.rs`
- Test: `src-tauri/src/services/radius_service/auth.rs`

- [ ] Step 1: Write failing Rust tests for PAP auth covering:
- [ ] a. valid user accepted
- [ ] b. wrong password rejected
- [ ] c. disabled account rejected
- [ ] d. unknown NAS rejected
- [ ] e. duplicate username isolation by NAS/router context
- [ ] Step 2: Run the targeted Rust tests and verify they fail because the auth pipeline does not exist yet.
- [ ] Step 3: Implement minimal PAP Access-Request handling with NAS resolution, password verification, and Access-Accept/Reject generation.
- [ ] Step 4: Re-run the PAP auth tests until green.

### Task 8: Add CHAP support if the packet/library path is stable

**Files:**
- Modify: `src-tauri/src/services/radius_service/auth.rs`
- Modify: `src-tauri/src/services/radius_service/packet.rs`
- Test: `src-tauri/src/services/radius_service/auth.rs`

- [ ] Step 1: Write failing Rust tests for CHAP request parsing and acceptance/rejection.
- [ ] Step 2: Run the targeted Rust tests and verify the red state.
- [ ] Step 3: Add the minimum CHAP verification path needed for MikroTik PPPoE compatibility.
- [ ] Step 4: Re-run the auth tests and either make them green or explicitly document a scoped deferral if the underlying crate path is not viable.

## Chunk 5: Accounting Runtime

### Task 9: Implement Accounting-Request handling

**Files:**
- Create: `src-tauri/src/services/radius_service/accounting.rs`
- Modify: `src-tauri/src/services/radius_service/server.rs`
- Test: `src-tauri/src/services/radius_service/accounting.rs`

- [ ] Step 1: Write failing Rust tests for accounting `Start`, `Interim-Update`, and `Stop` upsert behavior.
- [ ] Step 2: Run the targeted Rust tests and verify they fail because accounting handling does not exist.
- [ ] Step 3: Implement Accounting-Request parsing, DB persistence, and Accounting-Response generation.
- [ ] Step 4: Re-run the accounting tests until green.

### Task 10: Add end-to-end runtime smoke tests

**Files:**
- Create: `src-tauri/src/services/radius_service/runtime_tests.rs` or colocated tests
- Modify: `src-tauri/src/services/radius_service/mod.rs`

- [ ] Step 1: Write failing runtime smoke tests that exercise the in-process auth/accounting flow against a temporary database fixture.
- [ ] Step 2: Run the targeted Rust tests and verify they fail before the runtime path is complete.
- [ ] Step 3: Implement only the test harness/support code needed to drive the runtime in-process.
- [ ] Step 4: Re-run the smoke tests and keep them green.

## Chunk 6: PPPoE and Control-Plane Refactor

### Task 11: Refactor PPPoE apply flow to native runtime semantics

**Files:**
- Modify: `src-tauri/src/services/pppoe_service.rs`
- Modify: `src-tauri/src/services/managed_radius_service.rs`
- Test: `src-tauri/src/services/pppoe_service.rs`

- [ ] Step 1: Write failing Rust tests that define the new apply behavior:
- [ ] a. managed-radius apply updates local source-of-truth state only
- [ ] b. apply no longer requires external runtime DB connectivity
- [ ] c. local router secret disable behavior still works
- [ ] Step 2: Run the targeted Rust tests and verify the red state.
- [ ] Step 3: Update `pppoe_service` and related orchestration so apply targets the native runtime assumptions instead of provisioning an external DB mirror.
- [ ] Step 4: Re-run the targeted service tests until green.

### Task 12: Keep router-detail and superadmin setup UX functionally intact

**Files:**
- Modify: `src-tauri/src/http/mikrotik.rs`
- Modify: `src-tauri/src/http/superadmin.rs`
- Modify: `src/lib/utils/managedRadiusSetup.ts`
- Test: `src/lib/api/superadmin.test.ts`
- Test: `src/lib/utils/managedRadiusSetup*.test.ts`

- [ ] Step 1: Write failing targeted tests for any API contract changes caused by the runtime swap.
- [ ] Step 2: Run the targeted frontend/unit tests and verify the failure is due to outdated assumptions about the external runtime.
- [ ] Step 3: Update read models and setup messaging so tenant/superadmin flows still expose the correct host/port/NAS info for the native runtime.
- [ ] Step 4: Re-run targeted tests until green.

## Chunk 7: Legacy External Runtime Removal

### Task 13: Remove external runtime deployment and restart hooks

**Files:**
- Delete: `docker-compose.radius.yml`
- Delete: legacy external-runtime deployment assets
- Delete: legacy runtime restart hooks
- Modify: `.env.example`
- Modify: `README.md`
- Modify: `deploy/systemd/server.env.example`
- Modify: `SYSTEM_MAP.md`

- [ ] Step 1: Write failing regression tests or documentation assertions if any exist for the legacy stack references.
- [ ] Step 2: Remove legacy external-runtime deployment assets, separate runtime-database references, and stale environment variables or docs.
- [ ] Step 3: Replace docs with native runtime setup and verification guidance.
- [ ] Step 4: Re-run the targeted regression checks and fix fallout.

## Chunk 8: Final Verification

### Task 14: Run focused verification before completion

**Files:**
- Modify: any files needed to fix verification failures

- [ ] Step 1: Run targeted Rust tests for:
- [ ] a. `radius_service` config/repository/reply/auth/accounting
- [ ] b. `pppoe_service`
- [ ] c. `managed_radius_service`
- [ ] Step 2: Run a backend compile-level verification command for the full touched Rust surface.
- [ ] Step 3: Run any targeted frontend/Vitest checks affected by managed-radius setup contract changes.
- [ ] Step 4: Fix failures, re-run verification, and only then summarize completion.
