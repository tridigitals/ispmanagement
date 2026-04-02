# Superadmin Managed RADIUS Global Server Refactor Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor Managed RADIUS from tenant-scoped server records into a global server model where one platform-owned RADIUS server can serve many tenants, while each tenant keeps exactly one active assignment and many router/NAS mappings.

**Architecture:** Introduce a new global `radius_servers` table plus a `tenant_radius_assignments` table, migrate current tenant-scoped records forward, and update backend resolution so managed-RADIUS PPPoE provisioning resolves through tenant assignment instead of tenant-owned server rows. Then reshape `/superadmin/radius` into three management surfaces: global servers, tenant assignments, and router/NAS mappings.

**Tech Stack:** Rust (`sqlx`, Tauri commands, Axum HTTP), PostgreSQL migrations, SvelteKit 5, TypeScript, Vitest, i18n JSON locales

---

## File Map

**Database / backend core**
- Create: `src-tauri/migrations/20260402180000_refactor_managed_radius_global_servers.up.sql`
- Create: `src-tauri/migrations/20260402180000_refactor_managed_radius_global_servers.down.sql`
- Modify: `src-tauri/src/services/managed_radius_service.rs`
- Modify: `src-tauri/src/models/pppoe.rs`
- Modify: `src-tauri/src/models/mikrotik.rs`

**Superadmin command / HTTP surface**
- Modify: `src-tauri/src/commands/superadmin.rs`
- Modify: `src-tauri/src/http/superadmin.rs`
- Modify: `src-tauri/src/bootstrap/http.rs`
- Modify: `src-tauri/src/lib.rs`

**Tenant router surface**
- Modify: `src-tauri/src/commands/mikrotik.rs`
- Modify: `src-tauri/src/http/mikrotik.rs`

**Frontend API / types**
- Modify: `src/lib/api/core.ts`
- Modify: `src/lib/api/superadmin.ts`
- Modify: `src/lib/api/types.ts`

**Frontend UI**
- Modify: `src/routes/superadmin/radius/+page.svelte`
- Modify: `src/lib/components/superadmin/radius/ServerFormModal.svelte`
- Modify: `src/lib/components/superadmin/radius/MappingFormModal.svelte`
- Modify: `src/lib/components/superadmin/radius/MappingSecretDialog.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/network/routers/[id]/+page.svelte`
- Modify: `src/lib/utils/managedRadiusControlPlane.ts`

**Permissions / i18n**
- Modify: `src-tauri/src/services/role_service.rs`
- Modify: `src-tauri/src/db/connection/seed.rs`
- Modify: `src/lib/i18n/locales/en.json`
- Modify: `src/lib/i18n/locales/id.json`

**Tests**
- Modify: `src/lib/api/superadmin.test.ts`
- Modify: `src/lib/utils/managedRadiusControlPlane.test.ts`
- Add/Modify Rust tests in `src-tauri/src/services/managed_radius_service.rs`

## Chunk 1: Schema + Migration Foundation

### Task 1: Introduce global server and tenant assignment schema

**Files:**
- Create: `src-tauri/migrations/20260402180000_refactor_managed_radius_global_servers.up.sql`
- Create: `src-tauri/migrations/20260402180000_refactor_managed_radius_global_servers.down.sql`

- [ ] **Step 1: Write the migration expectations as comments/checklist in the SQL file header**

Capture these required outcomes in the migration comments:
- create global `radius_servers`
- create `tenant_radius_assignments`
- migrate/deduplicate existing tenant-scoped `managed_radius_servers`
- repoint `managed_radius_nas.radius_server_id`
- preserve current data

- [ ] **Step 2: Run current managed-radius tests to establish baseline**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS on current code before schema changes.

- [ ] **Step 3: Write the forward migration**

Include:
- create `radius_servers`
- create `tenant_radius_assignments`
- copy/deduplicate current server rows into `radius_servers`
- create tenant assignment rows from legacy server ownership
- update `managed_radius_nas.radius_server_id` to new global IDs
- drop old foreign key and recreate against `radius_servers`
- drop old `managed_radius_servers`

- [ ] **Step 4: Write the rollback migration**

Include:
- recreate tenant-scoped `managed_radius_servers`
- rebuild tenant-owned rows from assignments/global servers
- repoint NAS rows back
- drop `tenant_radius_assignments`
- drop `radius_servers`

- [ ] **Step 5: Re-run baseline verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: FAIL if compile/runtime assumptions still reference old schema, revealing next backend work.

## Chunk 2: Backend Global Server / Assignment Model

### Task 2: Refactor service-layer data access to global servers + tenant assignments

**Files:**
- Modify: `src-tauri/src/services/managed_radius_service.rs`
- Modify: `src-tauri/src/models/pppoe.rs`
- Modify: `src-tauri/src/models/mikrotik.rs`

- [ ] **Step 1: Write failing Rust tests for assignment-aware resolution**

Add/adjust tests for:
- tenant resolves its active assigned server
- inactive server cannot be used
- router NAS mapping must match tenant’s active assignment
- router setup hides clear secret when caller is not entitled at higher layers

- [ ] **Step 2: Run targeted Rust tests and confirm failure for the expected reason**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because service code still looks up `managed_radius_servers` by tenant ownership.

- [ ] **Step 3: Refactor `ManagedRadiusService` server resolution**

Implement minimal code so:
- account apply/reconcile resolves active `tenant_radius_assignments`
- connection info comes from `radius_servers`
- NAS create/update validates against active assignment

- [ ] **Step 4: Re-run targeted Rust verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS for assignment-aware service behavior.

### Task 3: Add backend CRUD for global servers and tenant assignments

**Files:**
- Modify: `src-tauri/src/services/managed_radius_service.rs`
- Modify: `src-tauri/src/commands/superadmin.rs`
- Modify: `src-tauri/src/http/superadmin.rs`
- Modify: `src-tauri/src/bootstrap/http.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing backend tests or compile-contract checks for new DTOs**

Cover:
- global server list shape
- tenant assignment list shape
- assignment activation deactivates previous active assignment for that tenant

- [ ] **Step 2: Run verification to surface missing backend operations**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because global-server/assignment CRUD does not exist yet.

- [ ] **Step 3: Implement global server CRUD**

Add:
- list global servers
- create/update server
- activate/deactivate server

- [ ] **Step 4: Implement tenant assignment CRUD**

Add:
- list assignments
- create/update assignment
- activate/deactivate assignment

- [ ] **Step 5: Wire Tauri command registration and HTTP routes**

Add command/route coverage for:
- global servers
- assignments
- NAS mappings using assignment-aware validation

- [ ] **Step 6: Re-run backend verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS.

## Chunk 3: Frontend API / Type Contracts

### Task 4: Update frontend API for global servers and tenant assignments

**Files:**
- Modify: `src/lib/api/core.ts`
- Modify: `src/lib/api/superadmin.ts`
- Modify: `src/lib/api/types.ts`
- Modify: `src/lib/api/superadmin.test.ts`

- [ ] **Step 1: Write failing API wrapper tests**

Cover:
- list global servers
- list tenant assignments
- create/update assignment payloads

- [ ] **Step 2: Run the targeted frontend tests and confirm failure**

Run: `npm run test:unit -- src/lib/api/superadmin.test.ts`
Expected: FAIL because wrappers still expose tenant-scoped server semantics.

- [ ] **Step 3: Implement updated frontend types and wrappers**

Add:
- `RadiusServer`
- `TenantRadiusAssignment`
- updated mapping response types

- [ ] **Step 4: Re-run targeted frontend tests**

Run: `npm run test:unit -- src/lib/api/superadmin.test.ts`
Expected: PASS.

## Chunk 4: Superadmin UI Refactor

### Task 5: Refactor `/superadmin/radius` into global servers, tenant assignments, and NAS mappings

**Files:**
- Modify: `src/routes/superadmin/radius/+page.svelte`
- Modify: `src/lib/components/superadmin/radius/ServerFormModal.svelte`
- Modify: `src/lib/components/superadmin/radius/MappingFormModal.svelte`
- Modify: `src/lib/components/superadmin/radius/MappingSecretDialog.svelte`
- Modify: `src/lib/utils/managedRadiusControlPlane.ts`
- Modify: `src/lib/utils/managedRadiusControlPlane.test.ts`

- [ ] **Step 1: Write failing helper/UI tests**

Cover:
- mapping helper uses assignment-aware filtering
- RouterOS CLI builder still works

- [ ] **Step 2: Run targeted helper tests to verify failure**

Run: `npm run test:unit -- src/lib/utils/managedRadiusControlPlane.test.ts`
Expected: FAIL if helper logic still assumes tenant-scoped servers only.

- [ ] **Step 3: Refactor UI state and sections**

Implement:
- `Global Servers` section
- `Tenant Assignments` section
- `Router / NAS Mappings` section

- [ ] **Step 4: Update forms**

Adjust:
- server form removes tenant selector
- assignment form selects tenant + global server
- mapping form uses tenant and auto-resolves assigned server

- [ ] **Step 5: Re-run helper tests**

Run: `npm run test:unit -- src/lib/utils/managedRadiusControlPlane.test.ts`
Expected: PASS.

- [ ] **Step 6: Run Svelte/TS verification**

Run: `npm run check`
Expected: no new `/superadmin/radius` errors; report pre-existing unrelated failures honestly if they remain.

## Chunk 5: Tenant Router Detail + Permission Preservation

### Task 6: Keep tenant router detail working with global server assignments

**Files:**
- Modify: `src/routes/[tenant]/(app)/admin/network/routers/[id]/+page.svelte`
- Modify: `src-tauri/src/commands/mikrotik.rs`
- Modify: `src-tauri/src/http/mikrotik.rs`
- Modify: `src-tauri/src/services/role_service.rs`
- Modify: `src-tauri/src/db/connection/seed.rs`
- Modify: `src/lib/i18n/locales/en.json`
- Modify: `src/lib/i18n/locales/id.json`

- [ ] **Step 1: Write failing verification target**

The router detail flow must still:
- show assigned server
- show masked secret
- hide clear secret unless `network_routers:manage_radius_secret`
- allow CLI copy without reveal permission

- [ ] **Step 2: Implement the minimal backend and UI adjustments**

Ensure router setup resolution uses tenant assignment, not legacy tenant-owned server rows.

- [ ] **Step 3: Re-run targeted backend verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS.

- [ ] **Step 4: Re-run i18n verification**

Run: `npm run i18n:check`
Expected: PASS.

## Chunk 6: End-to-End Verification and Cleanup

### Task 7: Verify merged behavior and remove legacy assumptions

**Files:**
- Modify as needed based on verification output

- [ ] **Step 1: Run final backend verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS.

- [ ] **Step 2: Run final frontend unit verification**

Run: `npm run test:unit -- src/lib/api/superadmin.test.ts src/lib/utils/managedRadiusControlPlane.test.ts`
Expected: PASS.

- [ ] **Step 3: Run final i18n verification**

Run: `npm run i18n:check`
Expected: PASS.

- [ ] **Step 4: Run final typecheck**

Run: `npm run check`
Expected: PASS or only known unrelated failures, which must be reported explicitly.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/migrations/20260402180000_refactor_managed_radius_global_servers.up.sql \
  src-tauri/migrations/20260402180000_refactor_managed_radius_global_servers.down.sql \
  src-tauri/src/services/managed_radius_service.rs \
  src-tauri/src/models/pppoe.rs \
  src-tauri/src/models/mikrotik.rs \
  src-tauri/src/commands/superadmin.rs \
  src-tauri/src/http/superadmin.rs \
  src-tauri/src/bootstrap/http.rs \
  src-tauri/src/lib.rs \
  src-tauri/src/commands/mikrotik.rs \
  src-tauri/src/http/mikrotik.rs \
  src/lib/api/core.ts \
  src/lib/api/superadmin.ts \
  src/lib/api/types.ts \
  src/routes/superadmin/radius/+page.svelte \
  src/lib/components/superadmin/radius \
  src/routes/[tenant]/(app)/admin/network/routers/[id]/+page.svelte \
  src/lib/utils/managedRadiusControlPlane.ts \
  src/lib/utils/managedRadiusControlPlane.test.ts \
  src/lib/api/superadmin.test.ts \
  src-tauri/src/services/role_service.rs \
  src-tauri/src/db/connection/seed.rs \
  src/lib/i18n/locales/en.json \
  src/lib/i18n/locales/id.json
git commit -m "Refactor managed RADIUS to global server assignments"
```
