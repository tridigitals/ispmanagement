# Superadmin Managed RADIUS Control Plane Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add superadmin CRUD for managed RADIUS servers and NAS/router mappings while preserving tenant PPPoE operations and keeping secrets masked by default.

**Architecture:** Extend the existing `/superadmin/radius` observability surface into a control plane backed by tenant-scoped CRUD endpoints for `managed_radius_servers` and `managed_radius_nas`. Keep one active server per tenant through transactional backend rules, add explicit secret rotation/reveal endpoints, and preserve tenant router detail CLI access with permission-gated raw secret reveal.

**Tech Stack:** Rust (`sqlx`, Tauri commands, Axum HTTP), SvelteKit 5, TypeScript, i18n JSON locales, existing secret encryption helpers

---

## Chunk 1: Backend Server CRUD

### Task 1: Add managed RADIUS server request/response contracts

**Files:**
- Modify: `src-tauri/src/commands/superadmin.rs`
- Modify: `src-tauri/src/http/superadmin.rs`
- Modify: `src/lib/api/types.ts`

- [ ] **Step 1: Write the failing backend/type coverage**

Add minimal tests or compile-driven assertions for:
- create server payload validation
- update server payload validation
- server list/read DTO shape including masked-safe fields only

- [ ] **Step 2: Run targeted verification and confirm the new CRUD surface is missing**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: FAIL to cover or expose the missing server CRUD behavior.

- [ ] **Step 3: Add request DTOs and response DTOs**

Add explicit DTOs for:
- create server
- update server
- activate/deactivate server if separate endpoint is used

Do not expose decrypted DB passwords in any list/read DTO.

- [ ] **Step 4: Re-run targeted verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS for the contract slice or progress to the next missing implementation.

### Task 2: Implement server CRUD with one-active-server-per-tenant enforcement

**Files:**
- Modify: `src-tauri/src/commands/superadmin.rs`
- Modify: `src-tauri/src/http/superadmin.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/bootstrap/http.rs`

- [ ] **Step 1: Write failing backend tests for server mutation behavior**

Cover:
- superadmin-only access
- create server stores encrypted DB password
- updating an active server deactivates other active servers in the same tenant
- cross-tenant mutation is rejected

- [ ] **Step 2: Run targeted backend tests and watch them fail for the expected reason**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because server CRUD operations do not exist yet.

- [ ] **Step 3: Implement minimal server CRUD logic**

Use transactions to:
- verify tenant ownership
- encrypt passwords on write
- enforce one active server per tenant

- [ ] **Step 4: Register the commands and HTTP routes**

Add create/update/toggle operations to both Tauri and HTTP surfaces already used by the app.

- [ ] **Step 5: Re-run targeted backend tests**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS for server CRUD behavior.

## Chunk 2: Backend NAS/Router Mapping CRUD

### Task 3: Add NAS mapping contracts, list query, and secret operations

**Files:**
- Modify: `src-tauri/src/commands/superadmin.rs`
- Modify: `src-tauri/src/http/superadmin.rs`
- Modify: `src/lib/api/types.ts`

- [ ] **Step 1: Write failing tests for mapping DTO and response behavior**

Cover:
- mapping read shape includes tenant, server, router, masked secret, and status
- reveal response is separate from list response

- [ ] **Step 2: Run targeted backend verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because mapping CRUD/reveal contracts are not implemented yet.

- [ ] **Step 3: Add DTOs**

Add explicit DTOs for:
- create mapping
- update mapping
- rotate secret
- reveal secret

- [ ] **Step 4: Re-run targeted verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS for DTO/contract coverage or advance to missing implementation.

### Task 4: Implement mapping CRUD, secret rotation, and reveal rules

**Files:**
- Modify: `src-tauri/src/commands/superadmin.rs`
- Modify: `src-tauri/src/http/superadmin.rs`
- Modify: `src-tauri/src/services/managed_radius_service.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/bootstrap/http.rs`

- [ ] **Step 1: Write failing backend tests for mapping behavior**

Cover:
- router and server must belong to the selected tenant
- one router maps to one NAS record
- secret rotation persists encrypted values
- reveal is allowed for superadmin and denied otherwise

- [ ] **Step 2: Run targeted backend tests and verify failure**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because mapping CRUD and secret operations are still missing.

- [ ] **Step 3: Implement minimal mapping CRUD**

Include:
- list mappings for superadmin page
- create/update mapping
- activate/deactivate mapping
- rotate secret
- reveal secret

Reuse existing secret encryption helpers instead of introducing a parallel mechanism.

- [ ] **Step 4: Re-run targeted backend tests**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`
Expected: PASS for mapping CRUD and secret operations.

## Chunk 3: Frontend API and Page State

### Task 5: Extend frontend API surface for server and mapping mutations

**Files:**
- Modify: `src/lib/api/core.ts`
- Modify: `src/lib/api/superadmin.ts`
- Modify: `src/lib/api/types.ts`

- [ ] **Step 1: Write the failing API/type expectation**

The frontend must be able to:
- list/create/update servers
- list/create/update mappings
- rotate/reveal mapping secret

- [ ] **Step 2: Run the smallest relevant frontend verification**

Run: `npm run check`
Expected: FAIL because the new API methods/types are not wired yet, plus report any unrelated pre-existing failures separately.

- [ ] **Step 3: Implement the API wrappers**

Add methods for all server and mapping actions and align them with the new backend contracts.

- [ ] **Step 4: Re-run the frontend verification**

Run: `npm run check`
Expected: The new API/type slice compiles; any remaining failure must be reported honestly if unrelated.

### Task 6: Add reusable superadmin RADIUS forms/components

**Files:**
- Create: `src/lib/components/superadmin/radius/ServerFormModal.svelte`
- Create: `src/lib/components/superadmin/radius/MappingFormModal.svelte`
- Create: `src/lib/components/superadmin/radius/MappingSecretDialog.svelte`
- Modify: `src/lib/i18n/locales/en.json`
- Modify: `src/lib/i18n/locales/id.json`

- [ ] **Step 1: Write the failing UI expectation**

The UI must support:
- create/edit server
- create/edit mapping
- rotate/reveal secret

- [ ] **Step 2: Implement the components with minimal form state**

Keep responsibilities split:
- server modal handles server fields
- mapping modal handles tenant/server/router filtered selection
- secret dialog handles reveal/rotate action

- [ ] **Step 3: Re-run frontend verification**

Run: `npm run check`
Expected: No new compile issues from component props, events, or i18n keys.

## Chunk 4: `/superadmin/radius` Control Plane UI

### Task 7: Expand `/superadmin/radius` from observability into CRUD control plane

**Files:**
- Modify: `src/routes/superadmin/radius/+page.svelte`

- [ ] **Step 1: Write the failing page expectation**

The page must support:
- server CRUD actions
- mapping CRUD actions
- filtered mapping list
- masked secret by default

- [ ] **Step 2: Run frontend verification before implementation**

Run: `npm run check`
Expected: FAIL or remain blocked before the CRUD page state exists.

- [ ] **Step 3: Implement the page changes**

Add:
- top-level actions for new server and new mapping
- server actions: edit, activate/deactivate
- mapping actions: edit, activate/deactivate, rotate secret, reveal secret, copy CLI
- tenant-first filtering in the mapping workflow

- [ ] **Step 4: Re-run frontend verification**

Run: `npm run check`
Expected: The `/superadmin/radius` page compiles; note any unrelated pre-existing failure separately.

## Chunk 5: Tenant Router Detail Permission Refinement

### Task 8: Gate tenant-side raw secret reveal while keeping CLI usable

**Files:**
- Modify: `src/routes/[tenant]/(app)/admin/network/routers/[id]/+page.svelte`
- Modify: `src/lib/api/types.ts`
- Modify: `src/lib/i18n/locales/en.json`
- Modify: `src/lib/i18n/locales/id.json`

- [ ] **Step 1: Write the failing permission/UI expectation**

Tenant admin should:
- still copy CLI
- still see masked secret
- only reveal raw secret if they have the dedicated permission

- [ ] **Step 2: Run relevant frontend verification**

Run: `npm run check`
Expected: FAIL or remain blocked before permission-gated reveal is implemented.

- [ ] **Step 3: Implement the permission-aware UI**

Hide or disable raw secret reveal unless the user has `network_routers.manage_radius_secret`.

- [ ] **Step 4: Re-run frontend verification**

Run: `npm run check`
Expected: Router detail changes compile and preserve CLI access behavior.

## Chunk 6: Final Verification

### Task 9: Verify the full control-plane slice honestly

**Files:**
- Modify as needed based on verification output

- [ ] **Step 1: Run targeted backend verification**

Run: `cargo test managed_radius --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 2: Run frontend/type verification**

Run: `npm run check`

- [ ] **Step 3: Inspect final scope**

Run: `git diff --stat`

- [ ] **Step 4: Report final status with evidence**

Summarize:
- implemented server CRUD
- implemented mapping CRUD and secret operations
- tenant-side permission refinement
- any remaining unrelated failures or follow-up risks
