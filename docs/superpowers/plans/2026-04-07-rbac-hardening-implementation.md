# RBAC Hardening Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace overlapping tenant RBAC with a granular, consistent permission model across seeds, frontend guards, and backend enforcement.

**Architecture:** Introduce domain-level permissions for network and storage concerns, centralize default permission and role definitions in one backend source, and update frontend/backend access checks to use the same resource names. Preserve owner/admin authority while sharply reducing overbroad technician, viewer, and storage access.

**Tech Stack:** Rust, SQLx, Axum, Tauri, SvelteKit, TypeScript

---

## Chunk 1: RBAC Source Of Truth

### Task 1: Centralize permission catalog and default roles

**Files:**
- Modify: `src-tauri/src/services/role_service.rs`
- Modify: `src-tauri/src/db/connection/seed.rs`
- Test: `src-tauri/src/services/auth_service/tests.rs`

- [ ] **Step 1: Write failing backend tests for granular permission catalog/role grants**
- [ ] **Step 2: Run focused auth/RBAC tests and verify failure**
- [ ] **Step 3: Move default permission catalog and role matrix to a single source**
- [ ] **Step 4: Remove legacy overlapping grants from DB seed path**
- [ ] **Step 5: Re-run focused tests and verify pass**

## Chunk 2: Backend Enforcement

### Task 2: Update HTTP/Tauri authorization checks to match granular permissions

**Files:**
- Modify: `src-tauri/src/http/mikrotik.rs`
- Modify: `src-tauri/src/commands/mikrotik.rs`
- Modify: `src-tauri/src/http/storage.rs`
- Test: `src-tauri/src/services/auth_service/tests.rs`

- [ ] **Step 1: Add failing tests for permission-dependent access helpers where practical**
- [ ] **Step 2: Run tests to verify failure**
- [ ] **Step 3: Replace broad `network_routers` and `storage` checks with granular resources**
- [ ] **Step 4: Re-run focused tests and verify pass**

## Chunk 3: Frontend Guards

### Task 3: Align tenant layout, sidebar, and page guards with backend permissions

**Files:**
- Modify: `src/routes/[tenant]/(app)/+layout.svelte`
- Modify: `src/lib/components/layout/Sidebar.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/storage/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/support/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/support/[id]/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/network/**/*.svelte`

- [ ] **Step 1: Update admin capability checks and route access mapping**
- [ ] **Step 2: Update sidebar item visibility to use granular permissions**
- [ ] **Step 3: Update page-level guards and action visibility**
- [ ] **Step 4: Run frontend checks/build as validation**

## Chunk 4: Verification

### Task 4: Verify no major collisions remain

**Files:**
- Modify: `src/lib/i18n/locales/en.json` (only if new labels/messages are needed)

- [ ] **Step 1: Run backend test suite relevant to auth/RBAC**
- [ ] **Step 2: Run frontend/build verification**
- [ ] **Step 3: Review remaining references to deprecated broad permissions**
- [ ] **Step 4: Summarize residual risks**
