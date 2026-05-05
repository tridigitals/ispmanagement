# DHCP Static Internet Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add DHCP static internet provisioning alongside PPPoE, including package provisioning metadata, backend CRUD/apply/reconcile support, tenant admin UI, customer detail exposure, and installation/work-order branching.

**Architecture:** Keep PPPoE and DHCP static as separate operational modules while sharing the existing package, subscription, and work-order domain. Extend package records with an explicit `provisioning_type`, introduce a dedicated DHCP static service record with RouterOS lease and optional simple queue sync status, and branch installation UX based on the package provisioning mode.

**Tech Stack:** Rust + Axum + Tauri commands + SQLx/PostgreSQL/SQLite migrations, SvelteKit 5 + TypeScript, MikroTik RouterOS via `mikrotik-rs`, Vitest, Rust unit/integration tests.

---

## Chunk 1: Package Provisioning Contract

### Task 1: Add package provisioning type persistence and API shape

**Files:**
- Modify: `src-tauri/src/models/isp_packages.rs`
- Modify: `src-tauri/src/services/isp_package_service.rs`
- Modify: `src-tauri/src/commands/isp_packages.rs`
- Modify: `src-tauri/src/http/isp_packages.rs`
- Modify: `src-tauri/src/services/customer_service/core.rs`
- Modify: `src/lib/api/types.ts`
- Modify: `src/lib/api/ispPackages.ts`
- Modify: `src/routes/(app)/admin/services/+page.svelte`
- Test: `src/lib/utils/internetOrderPackages.test.ts`
- Test: `src-tauri/src/services/isp_package_service.rs`
- Migration: `src-tauri/migrations/<timestamp>_add_isp_package_provisioning_type.*.sql`

- [ ] Step 1: Write failing tests for provisioning type normalization and defaults.
- [ ] Step 2: Run the targeted Rust and frontend tests to verify they fail for missing `provisioning_type`.
- [ ] Step 3: Add the DB column, model field, normalization rules, and API/client types with `pppoe` default.
- [ ] Step 4: Update admin services package create/edit UX to expose provisioning type for internet packages.
- [ ] Step 5: Run targeted Rust and frontend tests until they pass.

### Task 2: Update internet package consumers to respect provisioning type

**Files:**
- Modify: `src/lib/utils/internetOrderPackages.ts`
- Modify: `src/routes/(app)/dashboard/services/order/+page.svelte`
- Modify: `src/routes/(app)/dashboard/services/order/internet/+page.svelte`
- Modify: `src/routes/(app)/admin/network/installations/+page.svelte`
- Test: `src/lib/utils/internetOrderPackages.test.ts`

- [ ] Step 1: Write failing tests for internet package filtering/counting with DHCP static packages included in internet offerings.
- [ ] Step 2: Run the targeted frontend tests to verify red state.
- [ ] Step 3: Update package filtering/counting helpers and installation page package loading to preserve both internet provisioning types.
- [ ] Step 4: Re-run tests and keep them green.

## Chunk 2: DHCP Static Backend Domain

### Task 3: Add DHCP static data model and persistence

**Files:**
- Create: `src-tauri/src/models/dhcp_static.rs`
- Modify: `src-tauri/src/models/mod.rs`
- Migration: `src-tauri/migrations/<timestamp>_create_dhcp_static_services.*.sql`
- Test: `src-tauri/src/models/dhcp_static.rs`

- [ ] Step 1: Write failing Rust tests for enum serde/default behavior and request DTO validation helpers.
- [ ] Step 2: Run the Rust test target and verify it fails because the model does not exist yet.
- [ ] Step 3: Add DHCP static model structs, public DTOs, queue mode enum, and helper normalization/validation functions.
- [ ] Step 4: Add the SQL migration for the new table and indexes.
- [ ] Step 5: Re-run targeted model tests and make them pass.

### Task 4: Implement DHCP static service CRUD/apply/reconcile logic

**Files:**
- Create: `src-tauri/src/services/dhcp_static_service.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/bootstrap/app.rs`
- Modify: `src-tauri/src/bin/server.rs`
- Modify: `src-tauri/src/http/mod.rs`
- Test: `src-tauri/src/services/dhcp_static_service.rs`

- [ ] Step 1: Write failing service tests for MAC/IP validation, subscription uniqueness, and queue mode behavior.
- [ ] Step 2: Run the targeted Rust tests and verify they fail for missing service implementation.
- [ ] Step 3: Implement CRUD, installation-scope authorization, MikroTik lease apply, optional queue apply, and router reconcile logic.
- [ ] Step 4: Register the service in bootstrap/app state.
- [ ] Step 5: Re-run the targeted Rust service tests and fix failures until green.

### Task 5: Expose DHCP static via commands and HTTP routes

**Files:**
- Create: `src-tauri/src/commands/dhcp_static.rs`
- Create: `src-tauri/src/http/dhcp_static.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/bootstrap/http.rs`
- Modify: `src-tauri/src/http/middleware.rs`
- Test: `src/lib/api/core.ts`

- [ ] Step 1: Write failing frontend/client tests or compile-time checks for the missing command/route contracts.
- [ ] Step 2: Add Tauri commands and Axum routes mirroring PPPoE patterns.
- [ ] Step 3: Wire routes into the HTTP router and commands into the Tauri command registry.
- [ ] Step 4: Re-run targeted checks for route/client contract consistency.

## Chunk 3: Frontend DHCP Static Admin and Customer Views

### Task 6: Add frontend API client and shared types for DHCP static

**Files:**
- Create: `src/lib/api/dhcpStatic.ts`
- Modify: `src/lib/api/client.ts`
- Modify: `src/lib/api/core.ts`
- Modify: `src/lib/api/types.ts`
- Test: `src/lib/api/*.test.ts` or new targeted tests

- [ ] Step 1: Write failing tests for API payload normalization and command name wiring.
- [ ] Step 2: Add shared types and client methods for list/get/create/update/delete/apply/reconcile.
- [ ] Step 3: Re-run targeted tests and keep them green.

### Task 7: Build tenant admin DHCP static page and modal

**Files:**
- Create: `src/routes/(app)/admin/network/dhcp-static/+page.svelte`
- Create: `src/routes/(app)/admin/network/dhcp-static/DhcpStaticModal.svelte`
- Create: `src/routes/(app)/admin/network/dhcp-static/dhcpStaticPageModules.ts`
- Create: `src/routes/(app)/admin/network/dhcp-static/dhcpStaticPageModules.test.ts`
- Modify: `src/routes/(app)/+layout.svelte`
- Modify: `src/lib/utils/pageTitle.ts`
- Modify: `src/routes/admin-network-ui.test.ts`
- Modify: `src/lib/i18n/locales/en.json`
- Modify: `src/lib/i18n/locales/id.json`
- Modify: `src/lib/i18n/namespaces/en/admin.json`
- Modify: `src/lib/i18n/namespaces/id/admin.json`
- Modify: `src/lib/i18n/namespaces/en/sidebar.json`
- Modify: `src/lib/i18n/namespaces/id/sidebar.json`

- [ ] Step 1: Write failing page-module and route-title tests for the new DHCP static admin route.
- [ ] Step 2: Implement the lazy modal loader, admin list page, filtering, summary cards, and create/edit/apply UX.
- [ ] Step 3: Update permission gating, page title, and translations/sidebar labels.
- [ ] Step 4: Re-run targeted frontend tests and fix regressions.

### Task 8: Add DHCP static customer detail tab support

**Files:**
- Modify: `src/routes/(app)/admin/customers/[id]/+page.svelte`
- Modify: `src/lib/utils/customerDetailAccess.ts`
- Modify: `src/lib/utils/customerDetailAccess.test.ts`
- Modify: `src/lib/utils/appLanding.ts`
- Modify: `src/lib/utils/adminDashboard.ts`
- Modify: related tests as needed

- [ ] Step 1: Write failing tests for visible tabs, auto-load behavior, and permissions when DHCP static access exists.
- [ ] Step 2: Implement the separate DHCP static tab and service loading in customer detail.
- [ ] Step 3: Re-run customer-detail and dashboard tests to confirm no regression.

## Chunk 4: Installation and Work-Order Branching

### Task 9: Extend installation detail dialogs for DHCP static provisioning

**Files:**
- Modify: `src/routes/(app)/admin/network/installations/+page.svelte`
- Modify: `src/routes/(app)/admin/network/installations/InstallationDetailDialogs.svelte`
- Modify: `src/lib/api/workOrders.ts` if needed
- Test: `src/routes/admin-network-detail-ui.test.ts`

- [ ] Step 1: Write failing UI tests for DHCP static installation fields rendering when package provisioning type is `dhcp_static`.
- [ ] Step 2: Implement installation state for DHCP fields, load existing DHCP service records, and add save/apply handlers.
- [ ] Step 3: Branch the onsite wizard text and actions between PPPoE and DHCP static.
- [ ] Step 4: Re-run installation detail tests and fix issues until green.

### Task 10: Enforce installation completion rules for DHCP static

**Files:**
- Modify: `src-tauri/src/services/customer_service/work_orders.rs`
- Modify: any shared customer/work-order models touched by completion logic
- Test: targeted Rust tests in `customer_service` and `dhcp_static_service`

- [ ] Step 1: Write failing tests showing installation completion should require a DHCP static record for DHCP subscriptions.
- [ ] Step 2: Update work-order completion logic to validate required provisioning records by package provisioning type.
- [ ] Step 3: Re-run targeted Rust tests to verify both PPPoE and DHCP static flows remain valid.

## Chunk 5: Final Verification and Cleanup

### Task 11: Run focused verification and address regressions

**Files:**
- Modify: any files needed to fix verification failures

- [ ] Step 1: Run targeted Rust tests for `isp_package_service`, `dhcp_static_service`, and work-order completion.
- [ ] Step 2: Run targeted frontend tests for internet order, customer detail, admin network routes, and DHCP static modules.
- [ ] Step 3: Run at least one compile/build-level verification command for frontend/backend surfaces touched.
- [ ] Step 4: Fix any failures, re-run verification, and only then summarize completion.
