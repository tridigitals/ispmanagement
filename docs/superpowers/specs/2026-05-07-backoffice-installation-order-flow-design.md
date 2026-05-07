# Backoffice Installation Order Flow Design

## Goal
- Add a backoffice `Create Order` flow for `admin`, `sales`, or any tenant role granted a dedicated order-creation permission.
- Let staff create an installation order from one guided flow instead of manually creating customer, address, subscription, and work order records separately.
- Reuse the existing customer, location, subscription, and installation work order domain models instead of introducing a separate order aggregate.

## Problem
The current codebase already supports:
- customer CRUD
- customer location CRUD
- customer subscription creation
- portal order requests that create `pending_installation` subscriptions and installation work orders

What is missing is a single backoffice workflow that lets internal staff:
- choose a new or existing customer
- choose an existing address or add a new address
- pick a package/service
- submit directly into `pending_installation`
- automatically generate the installation work order

Without that orchestrated flow, sales or admin users have to stitch together multiple screens and permissions, which is slower and harder to control with RBAC.

## Product Decision
Build a dedicated backoffice `Create Order` wizard backed by a single orchestration endpoint and service method.

The flow must support:
- `new customer` and `existing customer`
- `existing address` and `new address`
- direct submission into `pending_installation`
- automatic installation work order creation

The flow will not introduce a draft or quotation stage in this first implementation.

## Scope
This design covers:
- a new permission-gated order creation flow for internal users
- backend orchestration endpoint and service logic
- frontend wizard page for admin/sales operators
- audit logging for order-driven writes
- validation and testing requirements

This design does not cover:
- a separate persistent `orders` table
- quotation, approval, or draft lifecycle
- billing invoice generation during order creation
- technician scheduling changes beyond existing work order behavior

## Recommended Architecture
Use an orchestration layer on top of the existing customer domain.

Backend write sequence on submit:
1. Validate actor permission for order creation.
2. Resolve or create customer.
3. Resolve or create customer location.
4. Resolve package and pricing inputs.
5. Create `customer_subscriptions` row with status `pending_installation`.
6. Call the existing helper that ensures an installation work order exists for the subscription.
7. Return the created or resolved records needed by the UI.

This flow should run in one transaction so partial write states do not escape:
- no customer without the intended subscription
- no subscription without the intended installation work order

## Permission Model
Add a dedicated permission for the order wizard instead of granting full customer management access.

Recommended permission:
- `orders:create`

Permission behavior:
- `orders:create` allows opening the wizard, resolving relevant customers and locations for the flow, selecting packages, and submitting the order.
- `customers:manage` remains the broader permission for full customer CRUD.
- `work_orders:manage` remains separate from order creation.
- `work_orders:read` is optional for users who should be able to view the resulting installation queue.

Why this split is preferred:
- sales should not need broad customer admin rights just to create an order
- order creation becomes auditable and intentionally scoped
- future internal roles can receive order intake access without inheriting unrelated operational permissions

## Backend Contract
Add a new backoffice endpoint dedicated to this workflow.

Recommended path:
- `POST /api/admin/orders/installations`

Recommended request shape:
- `customer_mode`: `new | existing`
- `customer_id`: required when `customer_mode=existing`
- `customer`: required when `customer_mode=new`
- `location_mode`: `new | existing`
- `location_id`: required when `location_mode=existing`
- `location`: required when `location_mode=new`
- `package_id`: required
- `billing_cycle`: explicit, defaulting to `monthly` if omitted
- `notes`: optional
- `requested_installation_date`: optional
- `sales_context`: optional metadata for future lead/source tracking

Recommended response shape:
- `customer`
- `location`
- `subscription`
- `work_order`

This contract keeps the frontend wizard simple and avoids multiple chained writes from the browser.

## Validation Rules
Minimum backend validation:

### Customer resolution
- If `customer_mode=new`, require `name` and at least one contact channel from `phone` or `email`.
- If `customer_mode=existing`, verify the customer exists in the same tenant and is visible to the actor under the order flow rules.

### Location resolution
- If `location_mode=new`, require `label` and `address_line1`.
- If `location_mode=existing`, verify the location belongs to the selected customer and tenant.
- New location coordinates remain optional.

### Package resolution
- `package_id` must exist, belong to the tenant, and be active.
- Billing cycle must be supported by the package pricing configuration.

### Subscription and work order
- The new subscription must be created directly with status `pending_installation`.
- Work order creation must reuse the existing `ensure_installation_work_order_for_subscription` behavior to avoid duplicate domain logic.
- If a work order already exists for the new subscription, the helper should return the existing row instead of creating a duplicate.

## UI Flow
Implement this as a dedicated wizard page, not a compact modal.

Recommended page shape:
1. `Customer`
2. `Address & Service`
3. `Review & Submit`

### Step 1: Customer
- Toggle between `Existing Customer` and `New Customer`.
- Existing customer path shows search and selection UI.
- New customer path shows a short intake form.

### Step 2: Address & Service
- For existing customers, let the actor choose between `Use Existing Address` and `Add New Address`.
- For new customers, default into the new address form.
- Show package selection in the same step to keep the wizard compact.

### Step 3: Review & Submit
- Show customer summary, selected or new address, package, billing cycle, and notes.
- Submit creates the subscription and installation work order immediately.

## Menu Placement
Initial placement should avoid adding a full new navigation domain.

Recommended entry points:
- primary `Create Order` action on the customer list page
- contextual `Create Order` action on customer detail pages

This keeps the change focused on order intake without prematurely creating a full standalone order module in the sidebar.

## UX and Redirect Rules
- Users with `orders:create` should be able to use the wizard even if they do not have broad customer management access.
- Existing customer search should be limited to fields needed for order intake.
- After success:
  - if actor has `work_orders:read` or `work_orders:manage`, redirect to the resulting installation context
  - otherwise redirect to the resulting customer detail page or another permitted context

## Audit Logging
Record clear order-specific audit events so the flow remains traceable.

Recommended events:
- `ORDER_CREATE`
- `CUSTOMER_CREATE_FROM_ORDER`
- `CUSTOMER_LOCATION_CREATE_FROM_ORDER`
- `CUSTOMER_SUBSCRIPTION_CREATE_FROM_ORDER`
- `INSTALLATION_WORK_ORDER_CREATE_FROM_ORDER`

At minimum, the logs should capture:
- actor
- tenant
- customer id
- location id
- subscription id
- work order id
- request IP when available

## Testing Strategy
Minimum backend coverage:
- existing customer + existing location
- existing customer + new location
- new customer + new location
- actor without `orders:create` is rejected
- location that does not belong to the selected customer is rejected
- inactive or invalid package is rejected
- one submission results in one installation work order for the created subscription

Minimum frontend coverage:
- wizard branch for `new` versus `existing` customer
- wizard branch for `existing` versus `new` address
- permission-gated visibility for entry actions
- success redirect behavior based on permission surface

## Risks and Constraints
- Do not leak full customer management capability to sales just because they need order intake.
- Do not duplicate subscription or work order creation rules that already exist in the portal flow.
- Keep the first implementation narrow: installation order intake only, no order lifecycle expansion.

## Recommended Implementation Shape
Backend:
- add permission seed for `orders:create`
- add request/response DTOs in the customer/order domain
- add one service method to orchestrate the flow
- wire one HTTP endpoint for the backoffice order submit

Frontend:
- add one wizard page and shared form state
- add customer lookup path for existing customer selection
- add `Create Order` actions where permissions allow
- redirect based on resulting permission access

## Success Criteria
- A sales or admin actor can create an installation order from one backoffice workflow.
- The flow supports both new and existing customers.
- The flow supports both existing and newly entered addresses.
- Submit creates a `pending_installation` subscription immediately.
- Submit automatically creates the linked installation work order.
- RBAC remains narrower than full customer CRUD for non-admin order intake roles.
