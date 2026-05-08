# Backoffice Installation Order Flow Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a permission-gated backoffice installation order wizard that can create a new or existing customer installation request and immediately produce a `pending_installation` subscription plus installation work order.

**Architecture:** Reuse the existing customer domain as the source of truth. Add one orchestration backend flow that resolves customer and location inputs, creates a subscription, reuses the existing installation work-order helper, and exposes one dedicated HTTP endpoint consumed by a new frontend wizard page and entry actions.

**Tech Stack:** Rust + Axum + sqlx backend, SvelteKit frontend, existing RBAC/auth service, existing customer/work-order APIs, Vitest and Rust tests.

---

## File Map

- Modify: `src-tauri/src/services/role_service.rs`
  - Add the new `orders:create` permission and assign it to the right default roles.
- Modify: `src-tauri/src/models/customer.rs`
  - Add request/response DTOs for the backoffice order orchestration API.
- Modify: `src-tauri/src/services/customer_service/mod.rs`
  - Re-export the new DTOs and service entrypoints if needed.
- Modify: `src-tauri/src/services/customer_service/core.rs`
  - Add helper logic for order-scoped customer lookup and shared validation when needed.
- Modify: `src-tauri/src/services/customer_service/subscriptions.rs`
  - Add the orchestration service method that creates the order-backed subscription and work order.
- Modify: `src-tauri/src/services/customer_service/repository.rs`
  - Reuse existing subscription/work-order helpers and add focused repository helpers only if required.
- Modify: `src-tauri/src/http/customers.rs`
  - Add one new backoffice route and handler for installation order creation.
- Modify: `src/lib/api/core.ts`
  - Add frontend command mapping for the new endpoint.
- Modify: `src/lib/api/types.ts`
  - Add frontend request/response types for the wizard flow.
- Modify: `src/lib/api/customers.ts`
  - Add a client method for backoffice installation order creation.
- Modify: `src/routes/(app)/admin/customers/+page.svelte`
  - Add a permission-gated `Create Order` entry action from the customer list.
- Modify: `src/routes/(app)/admin/customers/[id]/+page.svelte`
  - Add a permission-gated `Create Order` entry action from customer detail.
- Create: `src/routes/(app)/admin/customers/orders/new/+page.svelte`
  - Implement the wizard UI.
- Create or Modify: route-level test files near the new page and affected admin customer pages
  - Cover permission visibility and branch behavior.
- Modify or Create: Rust customer service tests near existing customer service tests
  - Cover orchestration paths and permission failures.

## Chunk 1: Backend Permission and DTO Surface

### Task 1: Add the new RBAC permission

**Files:**
- Modify: `src-tauri/src/services/role_service.rs`
- Test: `src-tauri/src/services/role_service.rs` existing permission tests

- [ ] **Step 1: Write the failing permission test**
  - Add or extend a role service test that expects `orders:create` to exist in the permission catalog.
  - Add or extend a default-role test that verifies the chosen default internal roles receive `orders:create`.

- [ ] **Step 2: Run the focused Rust test to verify it fails**

Run: `rtk cargo test role_service -- --nocapture`
Expected: FAIL because `orders:create` is not present yet.

- [ ] **Step 3: Add the minimal permission seed**
  - Add `("orders", "create", "Create installation orders from backoffice")` to the permission catalog.
  - Add `orders:create` only to the intended default roles.

- [ ] **Step 4: Re-run the focused Rust test**

Run: `rtk cargo test role_service -- --nocapture`
Expected: PASS for the new permission assertions.

- [ ] **Step 5: Commit**

```bash
rtk git add src-tauri/src/services/role_service.rs
rtk git commit -m "feat: add backoffice order creation permission"
```

### Task 2: Add request and response DTOs for the order flow

**Files:**
- Modify: `src-tauri/src/models/customer.rs`
- Modify: `src-tauri/src/services/customer_service/mod.rs`

- [ ] **Step 1: Write the failing Rust compile-facing test**
  - Add a service test or handler test that references the new DTOs:
    - `CreateBackofficeInstallationOrderRequest`
    - `BackofficeInstallationOrderResponse`
  - Make the test encode one `new customer + new location` payload.

- [ ] **Step 2: Run the focused Rust test to verify it fails**

Run: `rtk cargo test customer_service -- --nocapture`
Expected: FAIL because the DTOs do not exist.

- [ ] **Step 3: Add the minimal DTOs**
  - Add enums or string-backed fields for `customer_mode` and `location_mode`.
  - Add nested data structs for inline customer and location creation payloads.
  - Add a response struct that returns `customer`, `location`, `subscription`, and `work_order`.
  - Re-export them from `customer_service/mod.rs` if needed by the HTTP layer.

- [ ] **Step 4: Re-run the focused Rust test**

Run: `rtk cargo test customer_service -- --nocapture`
Expected: PASS or move on to the next missing behavior in the same new test.

- [ ] **Step 5: Commit**

```bash
rtk git add src-tauri/src/models/customer.rs src-tauri/src/services/customer_service/mod.rs
rtk git commit -m "feat: add backoffice order dto surface"
```

## Chunk 2: Backend Orchestration Service

### Task 3: Add the failing orchestration tests for the happy paths

**Files:**
- Modify or Create: Rust customer service tests near the customer service module
- Reference: `src-tauri/src/services/customer_service/subscriptions.rs`

- [ ] **Step 1: Write failing tests for the three required flows**
  - `existing customer + existing location`
  - `existing customer + new location`
  - `new customer + new location`
  - Each test should assert:
    - returned subscription status is `pending_installation`
    - returned work order links to the new subscription
    - new customer/location rows are created only when expected

- [ ] **Step 2: Run the focused Rust tests to verify they fail**

Run: `rtk cargo test backoffice_installation_order -- --nocapture`
Expected: FAIL because the orchestration method does not exist.

- [ ] **Step 3: Implement the minimal orchestration method**
  - Add a new service method in `subscriptions.rs`.
  - Gate it with `orders:create`.
  - Resolve customer and location based on mode.
  - Validate package and billing cycle using the same pricing rules used by current subscription flows where possible.
  - Create the subscription with `pending_installation`.
  - Reuse `ensure_installation_work_order_for_subscription`.
  - Return the composed response struct.

- [ ] **Step 4: Re-run the focused Rust tests**

Run: `rtk cargo test backoffice_installation_order -- --nocapture`
Expected: PASS for the three happy-path tests.

- [ ] **Step 5: Commit**

```bash
rtk git add src-tauri/src/services/customer_service/subscriptions.rs
rtk git add <customer-service-test-files>
rtk git commit -m "feat: add backoffice installation order service"
```

### Task 4: Add the failing orchestration tests for rejected cases

**Files:**
- Modify or Create: Rust customer service tests near the customer service module
- Modify: `src-tauri/src/services/customer_service/subscriptions.rs`

- [ ] **Step 1: Write failing tests for validation and permission failures**
  - actor without `orders:create` is rejected
  - location not owned by the selected customer is rejected
  - inactive or missing package is rejected

- [ ] **Step 2: Run the focused Rust tests to verify they fail**

Run: `rtk cargo test backoffice_installation_order -- --nocapture`
Expected: FAIL with missing validation or incorrect permission handling.

- [ ] **Step 3: Implement the minimal validation code**
  - Add explicit validation errors in the orchestration method.
  - Reuse existing lookup helpers where possible.

- [ ] **Step 4: Re-run the focused Rust tests**

Run: `rtk cargo test backoffice_installation_order -- --nocapture`
Expected: PASS for the rejection cases.

- [ ] **Step 5: Commit**

```bash
rtk git add src-tauri/src/services/customer_service/subscriptions.rs
rtk git add <customer-service-test-files>
rtk git commit -m "test: cover backoffice order validation cases"
```

## Chunk 3: Backend HTTP Wiring

### Task 5: Add the failing HTTP-level route test

**Files:**
- Modify: `src-tauri/src/http/customers.rs`
- Modify or Create: HTTP tests near customer handlers if present

- [ ] **Step 1: Write the failing handler test**
  - Add a test that posts the new payload to the backoffice endpoint and asserts the response shape.
  - Add a permission failure assertion if the HTTP test setup makes that practical.

- [ ] **Step 2: Run the focused Rust HTTP test to verify it fails**

Run: `rtk cargo test customers -- --nocapture`
Expected: FAIL because the route/handler does not exist.

- [ ] **Step 3: Implement the minimal route and handler**
  - Add the new route in `customers.rs`.
  - Validate token/tenant the same way as existing customer admin routes.
  - Forward to the new orchestration service method.

- [ ] **Step 4: Re-run the focused Rust HTTP test**

Run: `rtk cargo test customers -- --nocapture`
Expected: PASS for the new route coverage.

- [ ] **Step 5: Commit**

```bash
rtk git add src-tauri/src/http/customers.rs <http-test-files>
rtk git commit -m "feat: expose backoffice installation order endpoint"
```

## Chunk 4: Frontend API Client

### Task 6: Add the failing frontend API client test

**Files:**
- Modify: `src/lib/api/core.ts`
- Modify: `src/lib/api/types.ts`
- Modify: `src/lib/api/customers.ts`
- Test: related customer API tests if present

- [ ] **Step 1: Write the failing frontend API test**
  - Add a test that expects `api.customers.orders.createInstallation(...)` or an equivalent method to call the new command with the right payload.

- [ ] **Step 2: Run the focused frontend test to verify it fails**

Run: `rtk npm test -- src/lib/api`
Expected: FAIL because the command or method does not exist.

- [ ] **Step 3: Implement the minimal API client wiring**
  - Add the command mapping in `core.ts`.
  - Add request/response TS types in `types.ts`.
  - Add the client method in `customers.ts`.

- [ ] **Step 4: Re-run the focused frontend test**

Run: `rtk npm test -- src/lib/api`
Expected: PASS for the new client behavior.

- [ ] **Step 5: Commit**

```bash
rtk git add src/lib/api/core.ts src/lib/api/types.ts src/lib/api/customers.ts <frontend-api-test-files>
rtk git commit -m "feat: add frontend api for backoffice installation orders"
```

## Chunk 5: Frontend Wizard and Entry Actions

### Task 7: Add the failing UI visibility tests

**Files:**
- Modify: `src/routes/(app)/admin/customers/+page.svelte`
- Modify: `src/routes/(app)/admin/customers/[id]/+page.svelte`
- Test: nearby route tests

- [ ] **Step 1: Write failing tests for permission-gated entry actions**
  - Customer list page shows `Create Order` when actor has `orders:create`.
  - Customer detail page shows `Create Order` when actor has `orders:create`.
  - The action stays hidden without the permission.

- [ ] **Step 2: Run the focused frontend tests to verify they fail**

Run: `rtk npm test -- customersPageModules detailPageModules`
Expected: FAIL because the new action does not exist.

- [ ] **Step 3: Add the minimal entry actions**
  - Add permission-derived flags.
  - Link to the new wizard route, optionally pre-filling `customer_id` from detail context.

- [ ] **Step 4: Re-run the focused frontend tests**

Run: `rtk npm test -- customersPageModules detailPageModules`
Expected: PASS for the visibility cases.

- [ ] **Step 5: Commit**

```bash
rtk git add src/routes/(app)/admin/customers/+page.svelte src/routes/(app)/admin/customers/[id]/+page.svelte <route-test-files>
rtk git commit -m "feat: add create order entry actions"
```

### Task 8: Add the failing wizard tests

**Files:**
- Create: `src/routes/(app)/admin/customers/orders/new/+page.svelte`
- Create or Modify: tests near the new page

- [ ] **Step 1: Write failing tests for wizard branching**
  - `new customer` path renders customer input form
  - `existing customer` path renders search/selection behavior
  - `existing address` versus `new address` branches switch correctly
  - submit calls the API client with the expected payload

- [ ] **Step 2: Run the focused frontend tests to verify they fail**

Run: `rtk npm test -- orders/new`
Expected: FAIL because the page does not exist.

- [ ] **Step 3: Implement the minimal wizard**
  - Build a compact 3-step page.
  - Support prefilled `customer_id` query param for detail-page entry.
  - Load package options.
  - Load customer locations after customer selection.
  - Submit to the new API client method.
  - Redirect according to permission surface if straightforward now; otherwise redirect to customer detail first.

- [ ] **Step 4: Re-run the focused frontend tests**

Run: `rtk npm test -- orders/new`
Expected: PASS for the wizard branch and submit cases.

- [ ] **Step 5: Commit**

```bash
rtk git add src/routes/(app)/admin/customers/orders/new/+page.svelte <wizard-test-files>
rtk git commit -m "feat: add backoffice installation order wizard"
```

## Chunk 6: Final Verification

### Task 9: Run full targeted verification

**Files:**
- Verify all modified backend and frontend files from previous tasks

- [ ] **Step 1: Run the backend targeted suite**

Run: `rtk cargo test backoffice_installation_order -- --nocapture`
Expected: PASS

- [ ] **Step 2: Run the broader customer and role suites**

Run: `rtk cargo test customer_service -- --nocapture`
Run: `rtk cargo test role_service -- --nocapture`
Expected: PASS

- [ ] **Step 3: Run the frontend targeted suites**

Run: `rtk npm test -- src/lib/api src/routes/(app)/admin/customers`
Expected: PASS

- [ ] **Step 4: Run lint or build if needed for touched areas**

Run: `rtk npm run test -- --run`
Expected: PASS or report exact residual failures

- [ ] **Step 5: Commit**

```bash
rtk git add <all-touched-files>
rtk git commit -m "feat: add backoffice installation order flow"
```

Plan complete and saved to `docs/superpowers/plans/2026-05-07-backoffice-installation-order-flow.md`. Ready to execute.
