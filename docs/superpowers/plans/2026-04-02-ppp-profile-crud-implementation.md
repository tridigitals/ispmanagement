# PPP Profile CRUD Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add router-first CRUD for tenant PPP profiles with PostgreSQL mirror refresh, delete dependency safeguards, and UI actions on the PPP Profiles page.

**Architecture:** Extend the existing MikroTik backend surface with router-first PPP profile create, update, delete, and dependency lookup, then expose those operations through the shared frontend API wrapper. Keep RouterOS as the authority and reuse router-scoped sync after every successful mutation so the page always renders from the mirrored PostgreSQL dataset. On the frontend, keep the current page route and router filter, add modal-based create/edit flows, and block destructive delete when dependency counts are non-zero.

**Tech Stack:** Rust, Axum, Tauri commands, SQLx, Svelte, TypeScript, existing app API client wrappers, existing toast and table UI patterns.

---

## Chunk 1: Backend Contracts And Service Logic

### Task 1: Add backend DTOs and focused module boundaries for PPP profile CRUD

**Files:**
- Modify: `src-tauri/src/models/mikrotik.rs`
- Create: `src-tauri/src/services/mikrotik_ppp_profile_service.rs`
- Create: `src-tauri/src/http/mikrotik_ppp_profiles.rs`
- Create: `src-tauri/src/commands/mikrotik_ppp_profiles.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/http/mod.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Test: `src-tauri/src/services/mikrotik_ppp_profile_service.rs`

- [ ] **Step 1: Write the failing test**

Add the first backend unit test in `src-tauri/src/services/mikrotik_ppp_profile_service.rs` that rejects rename attempts in phase one using the new request/response structs.

```rust
#[tokio::test]
async fn update_ppp_profile_rejects_name_change() {
    let service = test_service().await;
    let err = service
        .update_ppp_profile(
            "tenant-1",
            "router-1",
            "profile-row-1",
            UpdateMikrotikPppProfileRequest {
                name: Some("new-name".into()),
                local_address: Some(Some("10.10.10.1".into())),
                remote_address: None,
                rate_limit: None,
                dns_server: None,
                comment: None,
            },
        )
        .await
        .unwrap_err();

    assert!(err.to_string().contains("rename"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test update_ppp_profile_rejects_name_change --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: FAIL because the request/response types or service method do not exist yet.

- [ ] **Step 3: Write minimal implementation**

Add focused PPP profile CRUD DTOs/models in `src-tauri/src/models/mikrotik.rs`:
- create request
- update request
- dependency item
- dependency response
- delete result payload

Create focused feature modules:
- `src-tauri/src/services/mikrotik_ppp_profile_service.rs`
- `src-tauri/src/http/mikrotik_ppp_profiles.rs`
- `src-tauri/src/commands/mikrotik_ppp_profiles.rs`

Export them from the existing `mod.rs` files so the main MikroTik files can delegate rather than absorb all CRUD logic inline.

Keep the DTOs limited to phase-one mutable fields and error/detail needs.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test update_ppp_profile_rejects_name_change --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: PASS or move to the next missing behavior failure inside the service.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models/mikrotik.rs src-tauri/src/services/mikrotik_ppp_profile_service.rs src-tauri/src/http/mikrotik_ppp_profiles.rs src-tauri/src/commands/mikrotik_ppp_profiles.rs src-tauri/src/services/mod.rs src-tauri/src/http/mod.rs src-tauri/src/commands/mod.rs
git commit -m "feat(mikrotik): add PPP profile CRUD DTOs"
```

### Task 2: Add failing service tests for create, update, delete dependency blocking, and dependency lookup

**Files:**
- Create: `src-tauri/src/services/mikrotik_ppp_profile_service.rs`
- Modify: `src-tauri/src/services/mikrotik_service.rs`

- [ ] **Step 1: Write the failing tests**

Add targeted tests covering:
- create rejects blank name
- create rejects duplicate name on the same router
- create success refreshes the mirrored row set
- update success changes a mutable field and returns the refreshed mirrored row
- update rejects rename attempts
- dependency lookup requires row to exist for the selected tenant/router
- dependency lookup returns PPPoE and package mapping counts
- delete is blocked when dependencies exist
- delete success refreshes the mirrored row set and removes the deleted profile
- create/update/delete surface `not_found` correctly when the row/router scope is missing
- update/delete surface `router_conflict` when the mirrored name no longer exists on RouterOS
- mutation surfaces `mirror_sync_failed` when router write succeeds but sync refresh fails

Use the existing MikroTik service test helpers and in-memory/test database scaffolding already used by neighboring MikroTik service tests, but keep the new assertions in the focused PPP-profile module test block.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test ppp_profile --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: FAIL with missing method/behavior errors for the new PPP profile CRUD flows.

- [ ] **Step 3: Write minimal implementation**

Implement the PPP-profile CRUD logic in `src-tauri/src/services/mikrotik_ppp_profile_service.rs` and keep `src-tauri/src/services/mikrotik_service.rs` limited to thin integration points only if the existing `MikrotikService` facade must delegate into the new module:
- `get_ppp_profile_dependencies`
- `create_ppp_profile`
- `update_ppp_profile`
- `delete_ppp_profile`

Rules:
- scope everything by tenant and router
- use row ID from `mikrotik_ppp_profiles.id`
- treat `name` as immutable on update
- re-check dependencies immediately before delete
- call router-scoped sync after every successful mutation
- return structured result payloads

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test ppp_profile --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: PASS for the new PPP profile CRUD tests and existing PPP profile sync tests remain green.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/mikrotik_ppp_profile_service.rs src-tauri/src/services/mikrotik_service.rs
git commit -m "feat(mikrotik): add PPP profile CRUD service flows"
```

### Task 3: Expose PPP profile CRUD through HTTP routes

**Files:**
- Create: `src-tauri/src/http/mikrotik_ppp_profiles.rs`
- Modify: `src-tauri/src/http/mikrotik.rs`
- Modify: `src-tauri/src/http/mod.rs`
- Test: `src-tauri/src/http/mikrotik_ppp_profiles.rs`

- [ ] **Step 1: Write the failing test**

Add HTTP-layer tests for:
- dependency lookup requires `read`
- create/update/delete require `manage`
- list continues to require `read`
- delete returns structured blocking payload when dependencies exist
- create returns duplicate-name rejection
- update/delete return `not_found` and `router_conflict` payloads when appropriate

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test mikrotik::tests --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: FAIL because the PPP profile CRUD routes and handlers do not exist yet.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/http/mikrotik_ppp_profiles.rs`:
- add request body structs for create/update
- add handlers for create/update/delete/dependencies

In `src-tauri/src/http/mikrotik.rs`:
- wire the PPP-profile routes into the main router

In `src-tauri/src/http/mod.rs`:
- export the new HTTP feature module

Keep the existing permission split: `read` for list/dependencies, `manage` for mutations/sync.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test mikrotik::tests --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: PASS for new HTTP route tests.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/http/mikrotik_ppp_profiles.rs src-tauri/src/http/mikrotik.rs src-tauri/src/http/mod.rs
git commit -m "feat(http): expose PPP profile CRUD routes"
```

### Task 4: Expose PPP profile CRUD through Tauri commands and registrations

**Files:**
- Create: `src-tauri/src/commands/mikrotik_ppp_profiles.rs`
- Modify: `src-tauri/src/commands/mikrotik.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/commands/mikrotik_ppp_profiles.rs`

- [ ] **Step 1: Write the failing test**

Add a concrete compile-and-call command test in `src-tauri/src/commands/mikrotik_ppp_profiles.rs` that validates the new PPP profile command functions return `Result<..., String>` and are wired with the expected request types.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test mikrotik --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: FAIL because the new commands are not registered or do not compile.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/commands/mikrotik_ppp_profiles.rs`:
- add `create_mikrotik_ppp_profile`
- add `update_mikrotik_ppp_profile`
- add `delete_mikrotik_ppp_profile`
- add `get_mikrotik_ppp_profile_dependencies`

In `src-tauri/src/commands/mod.rs`:
- export the new command feature module

In `src-tauri/src/commands/mikrotik.rs`:
- keep any shared MikroTik command helpers only if needed for reuse

In `src-tauri/src/lib.rs`:
- register the new commands in the invoke handler list

Keep error mapping aligned with the spec’s `code/message/details` envelope as closely as the current command surface allows.
Keep error mapping aligned with the spec’s `code/message/details` envelope exactly:
- if a service error is already structured, preserve `code`, `message`, and `details`
- if a lower-level error is plain text, normalize it into:
  - `code = "router_write_failed"` for router operation failures
  - `code = "validation_error"` for request-shape or disallowed-input failures
  - `code = "not_found"` for missing tenant/router/row scope
  - `code = "router_conflict"` for stale mirrored identity conflicts
  - `code = "mirror_sync_failed"` for sync-refresh failures after router success

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test mikrotik --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: PASS and project compiles with new command registrations.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/mikrotik_ppp_profiles.rs src-tauri/src/commands/mikrotik.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(tauri): add PPP profile CRUD commands"
```

## Chunk 2: Frontend API, UI Actions, And Verification

### Task 5: Add frontend API wrappers and types for PPP profile CRUD

**Files:**
- Modify: `src/lib/api/core.ts`
- Modify: `src/lib/api/mikrotik.ts`
- Create: `src/lib/api/pppProfiles.ts`
- Create: `src/lib/api/mikrotik.test.ts`

- [ ] **Step 1: Write the failing test**

Add `src/lib/api/mikrotik.test.ts` with concrete wrapper tests that verify:
- the new core route keys exist
- CRUD wrappers pass `routerId` and `id` correctly
- dependency and delete result payloads are typed and mapped correctly

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test:unit -- src/lib/api/mikrotik.test.ts`
Expected: FAIL because the PPP profile CRUD wrappers/types do not exist yet.

- [ ] **Step 3: Write minimal implementation**

In `src/lib/api/core.ts`:
- add HTTP definitions for create/update/delete/dependencies PPP profile endpoints

In `src/lib/api/pppProfiles.ts`:
- add explicit TypeScript types for:
  - PPP profile create/update payloads
  - dependency response
  - delete result payload
- add focused CRUD wrapper functions for PPP profile operations

In `src/lib/api/mikrotik.ts`:
- delegate PPP profile CRUD methods under `api.mikrotik.routers` to the new `pppProfiles.ts` module so the public client shape stays stable without growing `mikrotik.ts` too much

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test:unit -- src/lib/api/mikrotik.test.ts`
Expected: PASS for the new API wrapper contract tests.

- [ ] **Step 5: Commit**

```bash
git add src/lib/api/core.ts src/lib/api/mikrotik.ts src/lib/api/pppProfiles.ts src/lib/api/mikrotik.test.ts
git commit -m "feat(api): add PPP profile CRUD client wrappers"
```

### Task 6: Add a dedicated PPP profile form dialog component

**Files:**
- Create: `src/lib/components/network/PppProfileFormDialog.svelte`
- Create: `src/lib/utils/pppProfileCrud.ts`
- Create: `src/lib/utils/pppProfileCrud.test.ts`
- Modify: `src/lib/i18n/locales/en.json`
- Modify: `src/lib/i18n/locales/id.json`
- Test: `npm run check`

- [ ] **Step 1: Write the failing test**

Add `src/lib/utils/pppProfileCrud.test.ts` with concrete behavior tests for:
- add/edit blocked when no router is selected
- delete action blocked when dependency counts exist
- delete dialog state distinguishes blocked delete from allowed delete
- sync-error messaging is distinct from router-write failure messaging

Then add the component import and strongly typed props usage from the page so `npm run check` also fails until the dialog component contract and i18n keys exist.

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test:unit -- src/lib/utils/pppProfileCrud.test.ts`
Expected: FAIL because the helper and behaviors do not exist yet.

Run: `npm run check`
Expected: FAIL because the dialog component or i18n keys do not exist yet.

- [ ] **Step 3: Write minimal implementation**

Create `src/lib/components/network/PppProfileFormDialog.svelte` with:
- create mode and edit mode
- read-only `name` in edit mode
- local normalization of blank strings to `null`
- submit/cancel events
- phase-one fields only

Create `src/lib/utils/pppProfileCrud.ts` with small pure helpers for:
- router-selection gating
- dependency-to-delete-state mapping
- error-code-to-user-message-state mapping

Add only the required i18n strings in `en.json` and `id.json` for:
- dialog titles
- field labels
- validation messages
- delete/dependency copy
- CRUD success/error toasts

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test:unit -- src/lib/utils/pppProfileCrud.test.ts`
Expected: PASS for the new helper behavior tests.

Run: `npm run check`
Expected: PASS for the new component typing and i18n access.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/network/PppProfileFormDialog.svelte src/lib/utils/pppProfileCrud.ts src/lib/utils/pppProfileCrud.test.ts src/lib/i18n/locales/en.json src/lib/i18n/locales/id.json
git commit -m "feat(ui): add PPP profile form dialog"
```

### Task 7: Add create, edit, delete, and dependency UX to the PPP Profiles page

**Files:**
- Modify: `src/routes/[tenant]/(app)/admin/network/ppp-profiles/+page.svelte`
- Modify: `src/lib/components/ui/Table.svelte` (only if row action slots are required and already pattern-compatible)
- Modify: `src/lib/utils/pppProfileCrud.ts`
- Test: `npm run check`

- [ ] **Step 1: Write the failing test**

Add the new page state, dialog props, and handler signatures first so `npm run check` fails until action handlers and dependency/delete flows are fully wired.

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run check`
Expected: FAIL because the page does not yet know about dialog state, row actions, or delete dependency handling.

- [ ] **Step 3: Write minimal implementation**

Update `src/routes/[tenant]/(app)/admin/network/ppp-profiles/+page.svelte` to:
- import the new dialog
- add `Add profile`
- add row action controls for edit/delete
- call the new API methods
- reload from API responses after successful mutation
- keep `Refresh` as DB-only reload
- keep `Sync from router` as router-backed sync
- request dependencies before opening destructive confirm state
- show blocked-delete detail when dependencies exist
- preserve the existing router selector and table behavior

Only touch `Table.svelte` if the page cannot support row actions without a small, low-risk slot extension.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run check`
Expected: PASS with no Svelte/TypeScript errors in the PPP Profiles page.

Run: `npm run test:unit -- src/lib/utils/pppProfileCrud.test.ts`
Expected: PASS and behavior helpers still cover router gating, blocked delete, and message-state rules after page integration.

- [ ] **Step 5: Commit**

```bash
git add src/routes/[tenant]/\(app\)/admin/network/ppp-profiles/+page.svelte src/lib/components/ui/Table.svelte
git commit -m "feat(network): add PPP profile CRUD UI"
```

### Task 8: Run end-to-end verification and clean up

**Files:**
- Modify: `docs/superpowers/plans/2026-04-02-ppp-profile-crud-implementation.md`

- [ ] **Step 1: Run backend verification**

Run: `cargo test ppp_profile --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: PASS for PPP profile-related backend tests.

- [ ] **Step 2: Run frontend/static verification**

Run: `npm run check`
Expected: PASS with no new type or Svelte errors.

- [ ] **Step 3: Run broader targeted regression checks**

Run: `cargo test mikrotik --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: PASS for related MikroTik service and HTTP tests.

Run: `npm run test:unit -- src/lib/api/mikrotik.test.ts`
Expected: PASS for the targeted frontend API wrapper test subset used during implementation.

Run: `npm run test:unit -- src/lib/utils/pppProfileCrud.test.ts`
Expected: PASS for PPP profile CRUD behavior helpers used by the page.

- [ ] **Step 4: Mark plan progress**

Update this plan file checkboxes for any steps completed during execution.

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/plans/2026-04-02-ppp-profile-crud-implementation.md
git commit -m "docs(plan): track PPP profile CRUD execution"
```
