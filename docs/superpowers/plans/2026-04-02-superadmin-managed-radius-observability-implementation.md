# Superadmin Managed RADIUS Observability Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `/superadmin/radius` page that lets platform operators inspect managed RADIUS servers and managed-RADIUS-backed PPPoE users across all tenants.

**Architecture:** Extend the superadmin backend with two global list queries sourced from the billing database, expose them through the existing Tauri/frontend API surface, then build one superadmin page with stats, filters, and read-only tables. Keep all sensitive values masked or omitted and reuse the project’s existing superadmin page patterns.

**Tech Stack:** Rust (`sqlx`, Tauri commands, Axum HTTP), SvelteKit 5, TypeScript, i18n JSON locales

---

## Chunk 1: Backend Data Surface

### Task 1: Add superadmin managed RADIUS response models

**Files:**
- Modify: `src/lib/api/types.ts`
- Modify: `src-tauri/src/commands/superadmin.rs`
- Modify: `src-tauri/src/http/superadmin.rs`

- [ ] **Step 1: Write the failing type/test coverage**

Use or add minimal tests where the project already keeps superadmin/Rust response coverage so the new response shapes are enforced before implementation.

- [ ] **Step 2: Run targeted backend verification to confirm the new surface is still missing**

Run: `cargo test superadmin --manifest-path src-tauri/Cargo.toml`
Expected: FAIL or no matching coverage for the new managed RADIUS list surface.

- [ ] **Step 3: Add minimal serializable response structs**

Add read-only DTOs for:
- managed RADIUS servers
- managed RADIUS users
- summary list responses if needed

- [ ] **Step 4: Re-run targeted backend verification**

Run: `cargo test superadmin --manifest-path src-tauri/Cargo.toml`
Expected: PASS for the response model slice or at least progress to the next missing piece.

### Task 2: Add superadmin managed RADIUS queries and authorization

**Files:**
- Modify: `src-tauri/src/commands/superadmin.rs`
- Modify: `src-tauri/src/http/superadmin.rs`
- Modify: `src-tauri/src/http/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing backend tests for query behavior**

Cover:
- superadmin-only access
- server rows aggregate `router_count`
- user rows return only `account_source = 'managed_radius'`

- [ ] **Step 2: Run targeted tests and watch them fail for the expected reason**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because the superadmin queries do not exist yet.

- [ ] **Step 3: Implement minimal list queries**

Add:
- one query for server observability
- one query for managed RADIUS users

Use billing DB joins only:
- `managed_radius_servers`
- `managed_radius_nas`
- `tenants`
- `mikrotik_routers`
- `pppoe_accounts`

- [ ] **Step 4: Register commands and HTTP routes**

Expose the new superadmin list operations through both surfaces that the app already uses.

- [ ] **Step 5: Re-run targeted backend tests**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS for the new backend behavior.

## Chunk 2: Frontend API and Navigation

### Task 3: Add frontend API wrappers and types

**Files:**
- Modify: `src/lib/api/superadmin.ts`
- Modify: `src/lib/api/core.ts`
- Modify: `src/lib/api/types.ts`

- [ ] **Step 1: Write failing frontend/unit coverage for the new API contract if a nearby test file exists**

If no nearby automated coverage exists, document that limitation and use TypeScript checking as the red/green guard.

- [ ] **Step 2: Run the smallest relevant check before implementation**

Run: `npm run check`
Expected: FAIL because the new superadmin RADIUS methods/types are not wired yet, plus note any unrelated pre-existing failures separately.

- [ ] **Step 3: Implement the API surface**

Add:
- `listManagedRadiusServers`
- `listManagedRadiusUsers`

and any core route mapping/types needed for Tauri/HTTP.

- [ ] **Step 4: Re-run the smallest relevant check**

Run: `npm run check`
Expected: The new API/type slice is wired; report any remaining unrelated pre-existing failures honestly.

### Task 4: Add superadmin navigation entry

**Files:**
- Modify: `src/lib/components/layout/Sidebar.svelte`
- Modify: `src/lib/i18n/locales/en.json`
- Modify: `src/lib/i18n/locales/id.json`

- [ ] **Step 1: Add the failing expectation mentally from the approved spec**

The superadmin sidebar must contain a `RADIUS` entry and the label must be localizable.

- [ ] **Step 2: Implement the minimal navigation and i18n changes**

Add one new superadmin sidebar item and the translation keys needed for page labels and empty states.

- [ ] **Step 3: Re-run a frontend check**

Run: `npm run check`
Expected: No new sidebar/i18n regressions introduced by this change.

## Chunk 3: Superadmin Page

### Task 5: Build the `/superadmin/radius` page

**Files:**
- Create: `src/routes/superadmin/radius/+page.svelte`

- [ ] **Step 1: Write the failing page-level expectation**

The page must:
- load both lists
- compute summary stats
- filter servers and users
- show read-only observability states

- [ ] **Step 2: Run a focused frontend check before implementation**

Run: `npm run check`
Expected: FAIL because the new route/page does not exist yet.

- [ ] **Step 3: Implement the page with minimal moving parts**

Include:
- stats cards
- server filters and table/cards
- user filters and table/cards
- loading, empty, and error states

Keep the page read-only and reuse existing superadmin visual language.

- [ ] **Step 4: Re-run the frontend check**

Run: `npm run check`
Expected: The page compiles; any remaining failure must be called out if it is pre-existing and unrelated.

## Chunk 4: Final Verification

### Task 6: Verify the full feature slice honestly

**Files:**
- Modify as needed based on verification output

- [ ] **Step 1: Run targeted backend verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 2: Run targeted frontend/type verification**

Run: `npm run check`

- [ ] **Step 3: Inspect the final diff for accidental scope creep**

Run: `git diff --stat`

- [ ] **Step 4: Report final status with evidence**

Summarize:
- what was implemented
- what was verified
- any remaining unrelated failures or follow-up risks
