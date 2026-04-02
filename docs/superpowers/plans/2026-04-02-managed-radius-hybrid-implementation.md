# Managed RADIUS Hybrid PPPoE Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first production-usable managed RADIUS path for PPPoE accounts while preserving existing router-backed PPP secrets.

**Architecture:** Extend the PPPoE domain with a source flag and target-specific sync state, then route provisioning through either the existing MikroTik secret flow or a new managed-RADIUS service that writes to a PostgreSQL-backed FreeRADIUS schema. Keep the admin UX unified and add minimal deployment scaffolding for a separate RADIUS stack.

**Tech Stack:** Rust (`sqlx`, Tauri backend services), Svelte, PostgreSQL, Docker Compose, FreeRADIUS-compatible SQL tables

---

## Chunk 1: Foundation

### Task 1: Add domain and migration coverage for PPPoE account source

**Files:**

- Modify: `src-tauri/src/models/pppoe.rs`
- Modify: `src-tauri/src/services/pppoe_service.rs`
- Modify: `src/lib/api/types.ts`
- Create: `src-tauri/migrations/20260402110000_add_managed_radius_foundation.up.sql`
- Create: `src-tauri/migrations/20260402110000_add_managed_radius_foundation.down.sql`

- [ ] Write failing tests for source parsing/defaulting and source-aware status handling.
- [ ] Run targeted Rust tests to verify failure.
- [ ] Add model fields and migration for `account_source`, `radius_present`, `radius_identity`, `radius_last_sync_at`, `radius_last_error`, `managed_radius_servers`, and `managed_radius_nas`.
- [ ] Re-run targeted tests to verify pass.

### Task 2: Add managed RADIUS backend service boundary

**Files:**

- Create: `src-tauri/src/services/managed_radius_service.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/http/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] Write failing tests for source-based delegation from PPPoE apply flow.
- [ ] Run the targeted Rust tests to verify failure.
- [ ] Implement the managed RADIUS service shell plus injectable helpers needed for tests.
- [ ] Re-run targeted Rust tests to verify pass.

## Chunk 2: PPPoE Execution Path

### Task 3: Route PPPoE create/update/apply/reconcile by account source

**Files:**

- Modify: `src-tauri/src/services/pppoe_service.rs`
- Modify: `src-tauri/src/http/pppoe.rs`
- Modify: `src/lib/api/pppoe.ts`
- Modify: `src/lib/api/core.ts`

- [ ] Write failing tests for managed-RADIUS-backed apply / reconcile behavior.
- [ ] Run targeted tests to verify failure.
- [ ] Implement source-aware provisioning and status updates.
- [ ] Re-run targeted tests to verify pass.

### Task 4: Implement PostgreSQL-backed managed RADIUS provisioning

**Files:**

- Modify: `src-tauri/src/services/managed_radius_service.rs`

- [ ] Write failing tests for tenant-aware NAS/account resolution and target table writes.
- [ ] Run targeted tests to verify failure.
- [ ] Implement minimal provisioning logic against FreeRADIUS-compatible SQL tables.
- [ ] Re-run targeted tests to verify pass.

## Chunk 3: Admin UX and Deployment

### Task 5: Expose account source in the PPPoE admin UI

**Files:**

- Modify: `src/routes/[tenant]/(app)/admin/network/pppoe/+page.svelte`
- Modify: `src/lib/api/types.ts`

- [ ] Write or extend UI-facing type assertions / usage checks where practical.
- [ ] Implement source selector, source column, and target-aware status rendering.
- [ ] Manually verify no router-backed flow regression in the page logic.

### Task 6: Add managed RADIUS deployment scaffolding

**Files:**

- Create: `docker-compose.radius.yml`
- Create: `deploy/freeradius/README.md`
- Create: `deploy/freeradius/mods-config/sql/main/postgresql/queries.conf`
- Create: `deploy/freeradius/clients.conf.example`
- Modify: `.env.example`

- [ ] Add FreeRADIUS + PostgreSQL Compose scaffolding for aaPanel deployment.
- [ ] Document required environment variables and router client provisioning assumptions.

## Chunk 4: Verification

### Task 7: Verify the implementation slice

**Files:**

- Modify as needed based on verification failures.

- [ ] Run targeted Rust tests for PPPoE and managed RADIUS logic.
- [ ] Run relevant frontend/type/build verification if available.
- [ ] Inspect diff for accidental scope creep.
- [ ] Report verified status and any known gaps honestly.
