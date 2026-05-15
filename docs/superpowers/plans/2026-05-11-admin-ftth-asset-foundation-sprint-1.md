# Admin FTTH Asset Foundation Sprint 1 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first production-ready FTTH asset registry in `/admin` so tenant admins can create, view, update, and link FTTH assets to customers, locations, and work orders.

**Architecture:** Add a new tenant-scoped `network_assets` domain that sits alongside existing `networkMapping`, `customers`, and `work_orders` flows. Keep Sprint 1 intentionally manual-first: asset CRUD, relationship linking, customer detail visibility, and auditability. Do not add vendor-specific OLT automation, occupancy logic, or warehouse coupling yet.

**Tech Stack:** SvelteKit + Svelte 5 state macros, TypeScript API client wrappers, Tauri commands/services in Rust, SQLx-backed persistence, existing auth/permission/audit patterns, Vitest for frontend contract tests.

---

## File Structure

### New files

- `src/lib/api/networkAssets.ts`
  Frontend client wrapper for FTTH asset commands.
- `src/lib/utils/networkAssetTypes.ts`
  Shared UI helpers for asset types, statuses, labels, and validation defaults.
- `src/lib/utils/networkAssetTypes.test.ts`
  Unit tests for asset type/status helpers.
- `src/routes/(app)/admin/network/assets/+page.svelte`
  Admin asset registry page with table, filters, and create/edit flows.
- `src/routes/(app)/admin/network/assets/networkAssetsPageModules.ts`
  Deferred module loader for page dialogs if needed.
- `src/routes/(app)/admin/network/assets/networkAssetsPageModules.test.ts`
  Tests for deferred page modules.
- `src/routes/(app)/admin/network/assets/NetworkAssetFormModal.svelte`
  Create/edit asset modal.
- `src/routes/(app)/admin/network/assets/networkAssetsPageState.ts`
  Focused filter/state helpers for the asset page.
- `src/routes/(app)/admin/network/assets/networkAssetsPageState.test.ts`
  Tests for asset page filtering/state logic.
- `src/routes/(app)/admin/customers/[id]/CustomerAssetsTab.svelte`
  Customer detail tab for linked FTTH assets.
- `src/routes/(app)/admin/customers/[id]/customerAssetModules.ts`
  Deferred loader for customer assets tab.
- `src/routes/(app)/admin/customers/[id]/customerAssetModules.test.ts`
  Tests for customer asset module loading.
- `src/lib/api/networkAssets.test.ts`
  API wrapper contract tests.
- `src-tauri/src/models/network_asset.rs`
  Rust request/response/domain structs for assets.
- `src-tauri/src/services/network_asset_service.rs`
  Service layer for listing, creating, updating, deleting, and linking assets.
- `src-tauri/src/http/network_assets.rs`
  Optional HTTP passthrough module if this codebase mirrors REST handlers here.
- `src-tauri/src/commands/network_assets.rs`
  Tauri commands for asset operations.

### Modified files

- `src/lib/api/client.ts`
  Export `networkAssets` via the aggregated API surface if needed by existing import style.
- `src/lib/api/core.ts`
  Register command map entries for FTTH asset APIs.
- `src/lib/api/types.ts`
  Add TypeScript request/response types for assets and paginated listing.
- `src/lib/components/layout/Sidebar.svelte`
  Add `FTTH Assets` menu item under `Admin > Network`.
- `src/lib/i18n/namespaces/id/admin.json`
  Add Indonesian labels for asset page, fields, filters, and actions.
- `src/lib/i18n/namespaces/en/admin.json`
  Add English labels for the same UI.
- `src/lib/utils/adminNetworkAccess.ts`
  Include access helper for the new route if the existing pattern requires it.
- `src/lib/utils/customerDetailAccess.ts`
  Add `assets` tab visibility rules.
- `src/routes/(app)/admin/customers/[id]/customerDetailTabModules.ts`
  Register deferred loader for customer assets tab.
- `src/routes/(app)/admin/customers/[id]/customerDetailModules.ts`
  Wire asset tab module if needed by page composition pattern.
- `src/routes/(app)/admin/customers/[id]/+page.svelte`
  Add tab switcher entry and lazy-load asset tab.
- `src/routes/(app)/admin/network/map/+page.svelte`
  Optional small follow-up: expose linked FTTH asset metadata in selected object details if the current UI already surfaces metadata.
- `src-tauri/src/models/mod.rs`
  Export `network_asset`.
- `src-tauri/src/services/mod.rs`
  Export `network_asset_service`.
- `src-tauri/src/commands/mod.rs`
  Export `network_assets`.
- `src-tauri/src/main.rs`
  Register service wiring and Tauri commands.

### Existing files to study before coding

- `src/lib/api/networkMapping.ts`
- `src/lib/api/core.ts`
- `src/routes/(app)/admin/network/routers/+page.svelte`
- `src/routes/(app)/admin/network/dhcp-static/+page.svelte`
- `src/routes/(app)/admin/customers/[id]/+page.svelte`
- `src/routes/(app)/admin/customers/[id]/customerDetailTabModules.ts`
- `src/lib/utils/customerDetailAccess.ts`
- `src-tauri/src/commands/customers.rs`
- `src-tauri/src/services/storage_service.rs`

## Scope Guardrails

- Only build CRUD + linking for Sprint 1.
- Do not build OLT port occupancy.
- Do not build ONU auth/provisioning vendor adapters.
- Do not build warehouse stock logic.
- Do not add map editing workflows specific to FTTH assets beyond metadata visibility.

## Permission Model

Introduce a new resource: `ftth_assets`.

- Read actions:
  - `read`
- Write actions:
  - `manage`

Use existing tenant RBAC patterns. Do not reuse `router_inventory` or `network_noc`; FTTH assets need their own permission boundary.

## Data Model

Create a tenant-scoped `network_assets` record with these fields:

- `id`
- `tenant_id`
- `asset_type`
  Allowed values in Sprint 1:
  - `olt`
  - `odc`
  - `odp`
  - `splitter`
  - `ont`
  - `onu`
  - `fat`
  - `nap`
- `name`
- `code`
- `vendor`
- `model`
- `serial_number`
- `status`
  Allowed values:
  - `available`
  - `reserved`
  - `installed`
  - `faulty`
  - `retired`
- `customer_id`
- `location_id`
- `work_order_id`
- `parent_asset_id`
- `notes`
- `metadata`
- `created_at`
- `updated_at`

Add uniqueness constraints:

- unique `(tenant_id, code)` where code is not null/blank
- unique `(tenant_id, serial_number)` where serial number is not null/blank

Add supporting indexes:

- `(tenant_id, asset_type)`
- `(tenant_id, status)`
- `(tenant_id, customer_id)`
- `(tenant_id, location_id)`
- `(tenant_id, parent_asset_id)`

## API Surface

Frontend wrapper and Tauri command names should align with existing conventions:

- `list_network_assets`
- `get_network_asset`
- `create_network_asset`
- `update_network_asset`
- `delete_network_asset`
- `assign_network_asset_customer`
- `assign_network_asset_location`
- `assign_network_asset_work_order`
- `link_network_asset_parent`
- `list_customer_network_assets`

## Chunk 1: Contracts And Types

### Task 1: Add frontend types for FTTH assets

**Files:**
- Modify: `src/lib/api/types.ts`
- Test: `src/lib/api/networkAssets.test.ts`

- [ ] **Step 1: Write the failing test**

Add assertions in `src/lib/api/networkAssets.test.ts` that expect payload shapes for:
- list response items
- create/update request DTOs
- assign/link commands

- [ ] **Step 2: Run test to verify it fails**

Run: `rtk pnpm vitest src/lib/api/networkAssets.test.ts`
Expected: FAIL because `networkAssets` types and wrapper do not exist yet.

- [ ] **Step 3: Write minimal implementation**

Add new exported types in `src/lib/api/types.ts`:
- `NetworkAssetType`
- `NetworkAssetStatus`
- `NetworkAsset`
- `NetworkAssetListItem`
- `CreateNetworkAssetRequest`
- `UpdateNetworkAssetRequest`
- `AssignNetworkAssetRequest`

- [ ] **Step 4: Run test to verify it passes**

Run: `rtk pnpm vitest src/lib/api/networkAssets.test.ts`
Expected: PASS for type-driven contract cases.

- [ ] **Step 5: Commit**

```bash
git add src/lib/api/types.ts src/lib/api/networkAssets.test.ts
git commit -m "feat: add FTTH asset API types"
```

### Task 2: Add command map and frontend API wrapper

**Files:**
- Create: `src/lib/api/networkAssets.ts`
- Modify: `src/lib/api/core.ts`
- Modify: `src/lib/api/client.ts`
- Test: `src/lib/api/networkAssets.test.ts`

- [ ] **Step 1: Write the failing test**

Add wrapper tests that verify each new method calls `safeInvoke` with:
- the correct command name
- token
- DTO payload

- [ ] **Step 2: Run test to verify it fails**

Run: `rtk pnpm vitest src/lib/api/networkAssets.test.ts`
Expected: FAIL because command map entries and wrapper methods are missing.

- [ ] **Step 3: Write minimal implementation**

Mirror the style of `src/lib/api/networkMapping.ts`:
- export `networkAssets` object
- add list/get/create/update/delete/assign/listCustomerAssets helpers
- register all command map entries in `src/lib/api/core.ts`
- export through `src/lib/api/client.ts`

- [ ] **Step 4: Run test to verify it passes**

Run: `rtk pnpm vitest src/lib/api/networkAssets.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/api/networkAssets.ts src/lib/api/core.ts src/lib/api/client.ts src/lib/api/networkAssets.test.ts
git commit -m "feat: add FTTH asset frontend API wrapper"
```

## Chunk 2: Backend Asset Domain

### Task 3: Add Rust asset models

**Files:**
- Create: `src-tauri/src/models/network_asset.rs`
- Modify: `src-tauri/src/models/mod.rs`
- Test: `src/lib/api/networkAssets.test.ts`

- [ ] **Step 1: Write the failing test**

Expand frontend contract tests to assert field names remain stable across DTOs expected by Tauri commands.

- [ ] **Step 2: Run test to verify it fails**

Run: `rtk pnpm vitest src/lib/api/networkAssets.test.ts`
Expected: FAIL or remain pending until Rust DTO names are aligned in implementation notes.

- [ ] **Step 3: Write minimal implementation**

Define:
- `NetworkAsset`
- `NetworkAssetListItem`
- `CreateNetworkAssetRequest`
- `UpdateNetworkAssetRequest`
- `AssignNetworkAssetRelationRequest`
- `ListNetworkAssetsParams`

Use serde naming consistent with the rest of the codebase.

- [ ] **Step 4: Run test to verify it passes**

Run: `rtk pnpm vitest src/lib/api/networkAssets.test.ts`
Expected: PASS once DTO naming is aligned.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models/network_asset.rs src-tauri/src/models/mod.rs src/lib/api/networkAssets.test.ts
git commit -m "feat: add FTTH asset Rust models"
```

### Task 4: Add service layer for CRUD and linking

**Files:**
- Create: `src-tauri/src/services/network_asset_service.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: add service-level tests in the existing Rust test style if present nearby, otherwise document manual verification in this sprint and add frontend contract coverage first

- [ ] **Step 1: Write the failing test**

If the Rust service layer already has unit test precedent, add tests for:
- listing scoped by tenant
- creating asset with unique serial/code validation
- assigning customer/location/work order
- filtering by type/status/customer

If there is no local service-test pattern, create a minimal test file only if consistent with nearby modules.

- [ ] **Step 2: Run test to verify it fails**

Run the nearest relevant Rust test target for the new file.
Expected: FAIL because `NetworkAssetService` does not exist.

- [ ] **Step 3: Write minimal implementation**

Implement:
- `list_assets`
- `get_asset`
- `create_asset`
- `update_asset`
- `delete_asset`
- `assign_customer`
- `assign_location`
- `assign_work_order`
- `link_parent_asset`
- `list_customer_assets`

Requirements:
- enforce tenant scoping
- reject duplicate serial/code inside tenant
- write audit entries for create/update/delete/assign actions
- keep deletes safe: either soft-delete or block deletion when linked to active customer/work order if that matches current product expectations

- [ ] **Step 4: Run test to verify it passes**

Run the Rust test command chosen in Step 2.
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/network_asset_service.rs src-tauri/src/services/mod.rs
git commit -m "feat: add FTTH asset service layer"
```

### Task 5: Expose Tauri commands

**Files:**
- Create: `src-tauri/src/commands/network_assets.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/main.rs`
- Optional Modify: `src-tauri/src/http/network_assets.rs`

- [ ] **Step 1: Write the failing test**

Use frontend wrapper tests or a small command-level integration test if the project has an established Tauri test pattern.

- [ ] **Step 2: Run test to verify it fails**

Run: `rtk pnpm vitest src/lib/api/networkAssets.test.ts`
Expected: FAIL at invocation mapping or manual verification reveals unregistered commands.

- [ ] **Step 3: Write minimal implementation**

Follow `src-tauri/src/commands/customers.rs` conventions:
- validate token
- resolve tenant from claims
- check `ftth_assets` permission
- delegate to `NetworkAssetService`

Register all commands in `main.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `rtk pnpm vitest src/lib/api/networkAssets.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/network_assets.rs src-tauri/src/commands/mod.rs src-tauri/src/main.rs src-tauri/src/http/network_assets.rs
git commit -m "feat: expose FTTH asset Tauri commands"
```

## Chunk 3: Admin Route And Navigation

### Task 6: Add page state helpers for filtering

**Files:**
- Create: `src/routes/(app)/admin/network/assets/networkAssetsPageState.ts`
- Create: `src/routes/(app)/admin/network/assets/networkAssetsPageState.test.ts`

- [ ] **Step 1: Write the failing test**

Add tests for:
- filtering by `asset_type`
- filtering by `status`
- text search over `name`, `code`, `serial_number`
- sorting by newest updated

- [ ] **Step 2: Run test to verify it fails**

Run: `rtk pnpm vitest src/routes/'(app)'/admin/network/assets/networkAssetsPageState.test.ts`
Expected: FAIL because helper file does not exist.

- [ ] **Step 3: Write minimal implementation**

Implement pure helper functions:
- `filterNetworkAssets`
- `buildNetworkAssetStats`
- `normalizeNetworkAssetSearch`

- [ ] **Step 4: Run test to verify it passes**

Run: `rtk pnpm vitest src/routes/'(app)'/admin/network/assets/networkAssetsPageState.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/routes/(app)/admin/network/assets/networkAssetsPageState.ts src/routes/(app)/admin/network/assets/networkAssetsPageState.test.ts
git commit -m "feat: add FTTH asset page state helpers"
```

### Task 7: Add admin asset registry page and modal

**Files:**
- Create: `src/routes/(app)/admin/network/assets/+page.svelte`
- Create: `src/routes/(app)/admin/network/assets/NetworkAssetFormModal.svelte`
- Create: `src/routes/(app)/admin/network/assets/networkAssetsPageModules.ts`
- Create: `src/routes/(app)/admin/network/assets/networkAssetsPageModules.test.ts`
- Modify: `src/lib/components/layout/Sidebar.svelte`
- Modify: `src/lib/i18n/namespaces/id/admin.json`
- Modify: `src/lib/i18n/namespaces/en/admin.json`
- Optional Modify: `src/lib/utils/adminNetworkAccess.ts`

- [ ] **Step 1: Write the failing test**

Add module loader test for deferred dialog loading.
If there is a route UI smoke-test pattern for admin pages, add a basic assertion that the new route source includes the expected title and page header.

- [ ] **Step 2: Run test to verify it fails**

Run:
- `rtk pnpm vitest src/routes/'(app)'/admin/network/assets/networkAssetsPageModules.test.ts`
- any relevant route smoke test command if added

Expected: FAIL because the page and loaders do not exist.

- [ ] **Step 3: Write minimal implementation**

Page requirements:
- permission gate on `ftth_assets`
- table columns:
  - name
  - type
  - status
  - code
  - serial number
  - customer
  - location
  - updated
  - actions
- filters:
  - search
  - type
  - status
- stats cards:
  - total assets
  - installed
  - available
  - faulty
- actions:
  - create
  - edit
  - link customer/location/work order
  - delete

Sidebar requirements:
- add `FTTH Assets` under Network near routers/PPPoE/DHCP modules

i18n requirements:
- do not hardcode labels except temporary fallback strings

- [ ] **Step 4: Run test to verify it passes**

Run the tests from Step 2.
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/routes/(app)/admin/network/assets src/lib/components/layout/Sidebar.svelte src/lib/i18n/namespaces/id/admin.json src/lib/i18n/namespaces/en/admin.json src/lib/utils/adminNetworkAccess.ts
git commit -m "feat: add FTTH asset admin registry UI"
```

## Chunk 4: Customer Detail Integration

### Task 8: Add customer detail asset tab access helpers

**Files:**
- Modify: `src/lib/utils/customerDetailAccess.ts`
- Test: add or extend customer detail access tests if present next to this utility

- [ ] **Step 1: Write the failing test**

Add tests that verify:
- `assets` tab is visible when `ftth_assets.read` is allowed
- `assets` tab is hidden otherwise

- [ ] **Step 2: Run test to verify it fails**

Run the relevant customer detail access test file.
Expected: FAIL because `assets` tab is not implemented.

- [ ] **Step 3: Write minimal implementation**

Update tab list and access state:
- add `canReadFtthAssets`
- add `assets` tab
- keep autoload behavior off by default unless detail tabs already lazy-load all data tabs uniformly

- [ ] **Step 4: Run test to verify it passes**

Run the same test command.
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/utils/customerDetailAccess.ts
git commit -m "feat: add customer detail FTTH asset tab access"
```

### Task 9: Add customer asset tab module and UI

**Files:**
- Create: `src/routes/(app)/admin/customers/[id]/CustomerAssetsTab.svelte`
- Create: `src/routes/(app)/admin/customers/[id]/customerAssetModules.ts`
- Create: `src/routes/(app)/admin/customers/[id]/customerAssetModules.test.ts`
- Modify: `src/routes/(app)/admin/customers/[id]/customerDetailTabModules.ts`
- Modify: `src/routes/(app)/admin/customers/[id]/+page.svelte`

- [ ] **Step 1: Write the failing test**

Add loader tests for the customer asset tab module.
If customer detail tab module tests exist, extend them to cover the new loader export.

- [ ] **Step 2: Run test to verify it fails**

Run:
- `rtk pnpm vitest src/routes/'(app)'/admin/customers/'[id]'/customerAssetModules.test.ts`
- `rtk pnpm vitest src/routes/'(app)'/admin/customers/'[id]'/customerDetailTabModules.test.ts`

Expected: FAIL because the loader and tab do not exist.

- [ ] **Step 3: Write minimal implementation**

Customer asset tab requirements:
- list linked assets for the current customer
- show:
  - name
  - type
  - serial
  - status
  - linked location
  - parent asset
  - updated
- provide open-link back to `/admin/network/assets`
- optionally allow inline unlink only if manage permission exists

- [ ] **Step 4: Run test to verify it passes**

Run the tests from Step 2.
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/routes/(app)/admin/customers/[id]/CustomerAssetsTab.svelte src/routes/(app)/admin/customers/[id]/customerAssetModules.ts src/routes/(app)/admin/customers/[id]/customerAssetModules.test.ts src/routes/(app)/admin/customers/[id]/customerDetailTabModules.ts src/routes/(app)/admin/customers/[id]/+page.svelte
git commit -m "feat: add customer FTTH asset tab"
```

## Chunk 5: Validation, Labels, And Optional Map Hook

### Task 10: Add shared asset label helpers

**Files:**
- Create: `src/lib/utils/networkAssetTypes.ts`
- Create: `src/lib/utils/networkAssetTypes.test.ts`

- [ ] **Step 1: Write the failing test**

Test:
- asset type label lookup
- status label lookup
- default status for new asset
- type grouping for OLT/ODP/ONT families

- [ ] **Step 2: Run test to verify it fails**

Run: `rtk pnpm vitest src/lib/utils/networkAssetTypes.test.ts`
Expected: FAIL because helper file does not exist.

- [ ] **Step 3: Write minimal implementation**

Export:
- `NETWORK_ASSET_TYPES`
- `NETWORK_ASSET_STATUSES`
- `getNetworkAssetTypeLabel`
- `getNetworkAssetStatusLabel`
- `getDefaultNetworkAssetStatus`

- [ ] **Step 4: Run test to verify it passes**

Run: `rtk pnpm vitest src/lib/utils/networkAssetTypes.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/utils/networkAssetTypes.ts src/lib/utils/networkAssetTypes.test.ts
git commit -m "feat: add FTTH asset label helpers"
```

### Task 11: Add optional metadata exposure in network map

**Files:**
- Modify: `src/routes/(app)/admin/network/map/+page.svelte`
- Modify: related UI helper only if necessary

- [ ] **Step 1: Write the failing test**

Only add tests if there is already a focused helper around selected map object metadata. Otherwise keep this as manual verification to avoid overreaching Sprint 1.

- [ ] **Step 2: Run test to verify it fails**

Run the nearest map helper test if one is introduced.

- [ ] **Step 3: Write minimal implementation**

If selected customer/location/router metadata is already rendered, expose linked FTTH asset count or asset summary without introducing new map editing workflows.

- [ ] **Step 4: Run test to verify it passes**

Run the same test if present.

- [ ] **Step 5: Commit**

```bash
git add src/routes/(app)/admin/network/map/+page.svelte
git commit -m "feat: surface FTTH asset metadata in network map"
```

## Verification Checklist

Run these before calling Sprint 1 complete:

- `rtk pnpm vitest src/lib/api/networkAssets.test.ts`
- `rtk pnpm vitest src/lib/utils/networkAssetTypes.test.ts`
- `rtk pnpm vitest src/routes/'(app)'/admin/network/assets/networkAssetsPageState.test.ts`
- `rtk pnpm vitest src/routes/'(app)'/admin/network/assets/networkAssetsPageModules.test.ts`
- `rtk pnpm vitest src/routes/'(app)'/admin/customers/'[id]'/customerAssetModules.test.ts`
- `rtk pnpm vitest src/routes/'(app)'/admin/customers/'[id]'/customerDetailTabModules.test.ts`
- the relevant customer detail access test command
- the relevant Rust test command for `network_asset_service` if added

Manual verification:

- create an `ONT` asset with serial number
- link it to a customer and location
- open customer detail and confirm the asset appears
- edit asset status from `available` to `installed`
- verify audit log entry exists
- verify unauthorized role cannot open `/admin/network/assets`

## Notes For The Implementer

- Follow existing `api.*` aggregation patterns instead of inventing a parallel import style.
- Prefer small focused helpers over a giant asset page file.
- Reuse existing table, modal, toolbar, and icon components.
- Use current tenant-aware routing conventions.
- Keep metadata flexible but do not let Sprint 1 become a generic schema builder.
- If DB migration files live outside the paths inspected here, add the migration in the project’s normal migration location before service implementation.

## Suggested Commit Boundaries

1. `feat: add FTTH asset API types`
2. `feat: add FTTH asset frontend API wrapper`
3. `feat: add FTTH asset Rust models`
4. `feat: add FTTH asset service layer`
5. `feat: expose FTTH asset Tauri commands`
6. `feat: add FTTH asset page state helpers`
7. `feat: add FTTH asset admin registry UI`
8. `feat: add customer detail FTTH asset tab access`
9. `feat: add customer FTTH asset tab`
10. `feat: add FTTH asset label helpers`
11. `feat: surface FTTH asset metadata in network map`

Plan complete and saved to `docs/superpowers/plans/2026-05-11-admin-ftth-asset-foundation-sprint-1.md`. Ready to execute?
