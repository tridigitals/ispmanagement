# IP Pool CRUD Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add router-first tenant-admin CRUD for MikroTik IP pools on `/[tenant]/admin/network/ip-pools`, including warning-only dependency checks on delete.

**Architecture:** Extend the existing MikroTik IP pool list/sync surface with CRUD request and response models, backend service methods, HTTP routes, and Tauri commands that treat RouterOS as the source of truth and PostgreSQL as the mirrored cache. Reuse the PPP Profile CRUD UX pattern on the frontend, but keep delete non-blocking by surfacing dependency warnings before and after the router delete.

**Tech Stack:** Rust (`sqlx`, Axum HTTP, Tauri commands), SvelteKit 5, TypeScript, Vitest, i18n JSON locales

---

## Chunk 1: Backend CRUD Surface

### Task 1: Add IP pool CRUD request and response models

**Files:**
- Modify: `src-tauri/src/models/mikrotik.rs`
- Modify: `src/lib/api/mikrotik.ts`
- Modify: `src/lib/api/core.ts`

- [ ] **Step 1: Write the failing contract expectation**

The backend/frontend contract must cover:
- create and update payloads with `name`, `ranges`, `next_pool`, `comment`
- dependency warning payloads
- delete result payloads with warning metadata

- [ ] **Step 2: Run the smallest relevant verification before implementation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: PASS before edits, giving a clean baseline for the backend slice.

- [ ] **Step 3: Add minimal request and response structs**

Add:
- `CreateMikrotikIpPoolRequest`
- `UpdateMikrotikIpPoolRequest`
- `MikrotikIpPoolDependencyStatus`
- `MikrotikIpPoolDeleteResult`
- dependency item structs reused by both HTTP and Tauri

- [ ] **Step 4: Re-run backend verification**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: PASS with the new model surface in place.

### Task 2: Implement router-first IP pool service methods

**Files:**
- Modify: `src-tauri/src/services/mikrotik_service.rs`

- [ ] **Step 1: Write failing backend tests for CRUD validation and warning behavior**

Cover:
- create rejects blank name
- update rejects rename attempts
- delete returns warning metadata when `pppoe_accounts.address_pool` is in use
- delete returns warning metadata when `isp_package_router_mappings.address_pool` is in use

- [ ] **Step 2: Run targeted backend tests and watch them fail**

Run: `cargo test mikrotik_service --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because IP pool CRUD methods and warning logic do not exist yet.

- [ ] **Step 3: Implement the minimal service behavior**

Add:
- local row lookup helpers
- dependency warning query helper
- create/update/delete methods for `/ip/pool`
- mirror refresh after successful router writes
- `mirror_sync_failed`, `router_conflict`, and validation-style errors consistent with PPP CRUD

- [ ] **Step 4: Re-run targeted backend tests**

Run: `cargo test mikrotik_service --manifest-path src-tauri/Cargo.toml`
Expected: PASS for the new IP pool CRUD slice, or fail only on unrelated existing tests that must be called out separately.

### Task 3: Expose HTTP routes and Tauri commands

**Files:**
- Modify: `src-tauri/src/http/mikrotik.rs`
- Modify: `src-tauri/src/commands/mikrotik.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the failing route/command expectation**

The surface must expose:
- list
- create
- update
- delete
- dependency lookup
- sync

- [ ] **Step 2: Run backend compilation once before implementation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: PASS before route additions.

- [ ] **Step 3: Add minimal handlers and command registration**

Match existing PPP permission boundaries:
- `read` for list and dependency lookup
- `manage` for create, update, delete, sync

- [ ] **Step 4: Re-run backend compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: PASS with the full backend IP pool CRUD surface wired.

## Chunk 2: Frontend CRUD UX

### Task 4: Add frontend CRUD helper coverage first

**Files:**
- Create: `src/lib/utils/ipPoolCrud.ts`
- Create: `src/lib/utils/ipPoolCrud.test.ts`

- [ ] **Step 1: Write the failing helper tests**

Cover:
- router-required gating
- delete warning state when dependencies exist
- delete allowed state when dependencies are empty
- mirror-sync vs router-write error messaging

- [ ] **Step 2: Run the focused test and watch it fail**

Run: `npm run test:unit -- src/lib/utils/ipPoolCrud.test.ts`
Expected: FAIL because the helper file does not exist yet.

- [ ] **Step 3: Implement the minimal helper**

Mirror PPP helper structure, but make delete warning-only:
- `warning: true`
- `allowed: true`
- dependency totals for dialog copy

- [ ] **Step 4: Re-run the focused test**

Run: `npm run test:unit -- src/lib/utils/ipPoolCrud.test.ts`
Expected: PASS.

### Task 5: Add dialog component and API wrappers

**Files:**
- Create: `src/lib/components/network/IpPoolFormDialog.svelte`
- Modify: `src/lib/api/mikrotik.ts`
- Modify: `src/lib/api/core.ts`

- [ ] **Step 1: Add the failing integration expectation**

The frontend must be able to call:
- create IP pool
- update IP pool
- delete IP pool
- fetch dependency warnings

- [ ] **Step 2: Run a frontend compile check before implementation**

Run: `npm run check`
Expected: PASS or show only the known unrelated pre-existing wallboard failure before IP pool edits.

- [ ] **Step 3: Implement the minimal dialog and API methods**

Keep:
- `name` read-only during edit
- blank optional strings normalized to `null`
- copy that says router changes apply directly

- [ ] **Step 4: Re-run the frontend compile check**

Run: `npm run check`
Expected: No new API/dialog errors beyond the known unrelated blocker if it still exists.

### Task 6: Upgrade the IP pool page into full CRUD

**Files:**
- Modify: `src/routes/[tenant]/(app)/admin/network/ip-pools/+page.svelte`
- Modify: `src/lib/i18n/locales/en.json`
- Modify: `src/lib/i18n/locales/id.json`

- [ ] **Step 1: Write the failing page expectation**

The page must:
- add `Add pool`
- show row-level `Edit` and `Delete`
- fetch dependency warnings before delete
- still allow delete after warning acknowledgement
- refresh rows after successful mutations

- [ ] **Step 2: Run focused verification before implementation**

Run: `node scripts/check-i18n.mjs --json`
Expected: PASS before new keys are added, establishing a locale baseline.

- [ ] **Step 3: Implement the minimal page changes**

Add:
- form state
- delete warning dialog state
- warning-aware success/error toasts
- action column
- localized copy for the full CRUD path

- [ ] **Step 4: Re-run focused verification**

Run: `node scripts/check-i18n.mjs --json`
Expected: PASS with the new locale keys.

## Chunk 3: Final Verification

### Task 7: Verify the feature slice honestly

**Files:**
- Modify as needed based on verification output

- [ ] **Step 1: Run focused frontend helper tests**

Run: `npm run test:unit -- src/lib/utils/ipPoolCrud.test.ts`

- [ ] **Step 2: Run backend compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 3: Run locale verification**

Run: `node scripts/check-i18n.mjs --json`

- [ ] **Step 4: Run broader frontend/type verification**

Run: `npm run check`
Expected: Report whether the known unrelated wallboard failure at `src/routes/[tenant]/(app)/admin/network/noc/wallboard/+page.svelte:777` still blocks full green.

- [ ] **Step 5: Inspect scope**

Run: `git diff --stat`

- [ ] **Step 6: Report status with evidence**

Summarize:
- backend CRUD surface added
- frontend CRUD UX added
- warning-only delete behavior verified
- any remaining unrelated repo issues
