# Managed RADIUS Default Assignment And Plan Gating Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add one default global RADIUS server, auto-assign eligible new tenants to it, and show upgrade guidance on tenant router pages when the subscription plan does not include Managed RADIUS.

**Architecture:** Extend the global `radius_servers` model with a single-default flag, reuse the existing plan feature system with a new `managed_radius` boolean feature, and hook tenant creation so it can auto-create one active assignment when the selected plan is eligible. Then expose plan-aware state on router detail so tenant admins see either setup guidance or an upgrade notice.

**Tech Stack:** Rust (`sqlx`, Tauri commands, Axum HTTP), PostgreSQL migrations, SvelteKit 5, TypeScript, i18n JSON locales

---

## File Map

**Database / seeds**
- Create: `src-tauri/migrations/20260402193000_add_managed_radius_default_and_plan_gate.up.sql`
- Create: `src-tauri/migrations/20260402193000_add_managed_radius_default_and_plan_gate.down.sql`
- Modify: `src-tauri/src/db/connection/seed.rs`

**Backend models / services**
- Modify: `src-tauri/src/models/pppoe.rs`
- Modify: `src-tauri/src/models/mikrotik.rs`
- Modify: `src-tauri/src/services/managed_radius_service.rs`
- Modify: `src-tauri/src/services/plan_service.rs`
- Modify: `src-tauri/src/commands/superadmin.rs`
- Modify: `src-tauri/src/http/superadmin.rs`
- Modify: `src-tauri/src/commands/mikrotik.rs`
- Modify: `src-tauri/src/http/mikrotik.rs`

**Tenant creation flow**
- Modify: `src-tauri/src/commands/superadmin.rs`
- Modify: `src-tauri/src/http/superadmin.rs`

**Frontend**
- Modify: `src/lib/api/types.ts`
- Modify: `src/lib/api/superadmin.ts`
- Modify: `src/lib/api/mikrotik.ts`
- Modify: `src/routes/superadmin/radius/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/network/routers/[id]/+page.svelte`
- Modify: `src/lib/i18n/locales/en.json`
- Modify: `src/lib/i18n/locales/id.json`

## Chunk 1: Schema And Seed Foundation

### Task 1: Add default-server support and seed the plan feature

**Files:**
- Create: `src-tauri/migrations/20260402193000_add_managed_radius_default_and_plan_gate.up.sql`
- Create: `src-tauri/migrations/20260402193000_add_managed_radius_default_and_plan_gate.down.sql`
- Modify: `src-tauri/src/db/connection/seed.rs`

- [ ] **Step 1: Write the migration checklist in SQL comments**

Capture:
- add `is_default` to `radius_servers`
- enforce only one default server
- seed `managed_radius` plan feature definition

- [ ] **Step 2: Run baseline verification before schema changes**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS on current state.

- [ ] **Step 3: Write the forward migration**

Include:
- `ALTER TABLE radius_servers ADD COLUMN is_default`
- partial unique index for one default server
- update existing rows to `false`

- [ ] **Step 4: Seed the `managed_radius` feature definition**

Update seed/bootstrap so environments get:
- code `managed_radius`
- boolean type
- default `false`

- [ ] **Step 5: Re-run verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS or reveal the next backend compile gap.

## Chunk 2: Backend Default Server + Auto Assignment

### Task 2: Add default-server backend behavior

**Files:**
- Modify: `src-tauri/src/models/pppoe.rs`
- Modify: `src-tauri/src/services/managed_radius_service.rs`
- Modify: `src-tauri/src/commands/superadmin.rs`
- Modify: `src-tauri/src/http/superadmin.rs`

- [ ] **Step 1: Write failing tests for default-server behavior**

Cover:
- one server can be default
- setting a new default clears the previous one
- inactive server cannot become default

- [ ] **Step 2: Run targeted Rust verification and confirm failure**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because default-server logic does not exist yet.

- [ ] **Step 3: Implement minimal default-server support**

Add:
- `is_default` model field
- service methods to set/get default server
- superadmin list DTO updates
- superadmin mutation endpoint/command

- [ ] **Step 4: Re-run targeted Rust verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS for default-server behavior.

### Task 3: Auto-assign new tenants when the plan allows Managed RADIUS

**Files:**
- Modify: `src-tauri/src/services/plan_service.rs`
- Modify: `src-tauri/src/commands/superadmin.rs`
- Modify: `src-tauri/src/http/superadmin.rs`
- Modify: `src-tauri/src/services/managed_radius_service.rs`

- [ ] **Step 1: Write failing tests for tenant onboarding behavior**

Cover:
- tenant with `managed_radius=true` and default server gets assignment
- tenant with `managed_radius=false` does not get assignment
- no default server does not fail tenant creation

- [ ] **Step 2: Run targeted backend verification and confirm failure**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because tenant creation does not auto-assign yet.

- [ ] **Step 3: Implement minimal tenant auto-assignment flow**

Sequence:
- create tenant
- assign plan
- check feature access `managed_radius`
- resolve default server
- create active `tenant_radius_assignment` when eligible
- fail soft if assignment cannot be created

- [ ] **Step 4: Re-run targeted backend verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS for the onboarding slice.

## Chunk 3: Frontend Superadmin Surface

### Task 4: Expose default-server controls in `/superadmin/radius`

**Files:**
- Modify: `src/lib/api/types.ts`
- Modify: `src/lib/api/superadmin.ts`
- Modify: `src/routes/superadmin/radius/+page.svelte`
- Modify: `src/lib/i18n/locales/en.json`
- Modify: `src/lib/i18n/locales/id.json`

- [ ] **Step 1: Write failing frontend/API coverage where practical**

Cover:
- server payload/response includes `is_default`
- API wrapper for setting default exists

- [ ] **Step 2: Run targeted frontend verification**

Run: `npm run test:unit -- src/lib/api/superadmin.test.ts`
Expected: FAIL because default-server contract is not wired yet.

- [ ] **Step 3: Implement the minimal superadmin UI**

Add:
- default badge on server rows
- set-default action
- localized labels/toasts

- [ ] **Step 4: Re-run targeted verification**

Run: `npm run test:unit -- src/lib/api/superadmin.test.ts`
Expected: PASS.

## Chunk 4: Router Plan Gate UX

### Task 5: Show upgrade guidance on router detail when the plan does not include Managed RADIUS

**Files:**
- Modify: `src-tauri/src/models/mikrotik.rs`
- Modify: `src-tauri/src/services/plan_service.rs`
- Modify: `src-tauri/src/services/managed_radius_service.rs`
- Modify: `src-tauri/src/commands/mikrotik.rs`
- Modify: `src-tauri/src/http/mikrotik.rs`
- Modify: `src/lib/api/types.ts`
- Modify: `src/lib/api/mikrotik.ts`
- Modify: `src/routes/[tenant]/(app)/admin/network/routers/[id]/+page.svelte`
- Modify: `src/lib/i18n/locales/en.json`
- Modify: `src/lib/i18n/locales/id.json`

- [ ] **Step 1: Write failing tests or type assertions for the router setup contract**

Cover:
- setup response includes plan entitlement / upgrade state
- upgrade path points to `/admin/subscription`

- [ ] **Step 2: Run relevant frontend/type verification to confirm the gap**

Run: `npm run check`
Expected: FAIL or no coverage yet for the new contract.

- [ ] **Step 3: Implement the minimal router-page entitlement UX**

If tenant plan disallows Managed RADIUS:
- return plan-gated setup state
- show upgrade notice instead of setup instructions

If tenant plan allows Managed RADIUS:
- preserve current setup behavior

- [ ] **Step 4: Re-run verification**

Run: `npm run check`
Expected: New router-page slice compiles; report any unrelated pre-existing failure honestly.

## Chunk 5: Final Verification

### Task 6: Verify the full feature slice honestly

**Files:**
- Modify as needed based on verification output

- [ ] **Step 1: Run targeted Rust verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 2: Run targeted frontend verification**

Run: `npm run test:unit -- src/lib/api/superadmin.test.ts src/lib/utils/managedRadiusControlPlane.test.ts`

- [ ] **Step 3: Run i18n verification**

Run: `npm run i18n:check`

- [ ] **Step 4: Run app typecheck**

Run: `npm run check`

- [ ] **Step 5: Inspect final scope**

Run: `git diff --stat`

- [ ] **Step 6: Report verified outcome with any remaining unrelated failures**
