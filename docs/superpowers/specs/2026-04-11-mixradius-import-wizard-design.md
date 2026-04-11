# MixRadius Import Wizard Design (2026-04-11)

## Background / Current State
- ISP Management already has tenant-scoped data models for:
  - customers
  - customer locations
  - customer subscriptions
  - ISP packages
  - PPPoE accounts
  - MikroTik routers
- The product already supports PPPoE import from MikroTik router secrets through the PPPoE admin area.
- The product also already has a Managed RADIUS control plane, but no workflow exists for migrating legacy customer data from a MixRadius backup.
- The validated MixRadius backup currently available in this workspace is:
  - [MixRadiusDB_Gasal_2026-04-11_101103.sql.gz](/home/xtrabit/ISPMANAGEMENT/MixRadiusDB_Gasal_2026-04-11_101103.sql.gz)
- That backup is materially complete for PPP migration and contains:
  - `nas`
  - `tbl_customers`
  - `tbl_customers_sub`
  - `tbl_customers_map`
  - `tbl_odp_data`
  - `tbl_plans`
  - `tbl_bandwidth`
  - `tbl_transactions`
  - `radcheck`
  - `radreply`
  - `radusergroup`
  - `radacct`
- Backup analysis shows:
  - 543 PPP customers
  - 12 PPP plans
  - 2 NAS/router rows
  - 460 customer map/location rows
  - 1902 transactions
  - 3811 accounting rows
- MixRadius stores legacy lifecycle concepts directly inside customer and transaction tables, but ISP Management separates:
  - package definition
  - customer identity
  - subscription lifecycle
  - PPP credential/provisioning state

## Product Decision
Build a tenant-admin-facing MixRadius import wizard that accepts `.sql` and `.sql.gz` database backups, stages the extracted data, previews conflicts and lifecycle outcomes, and then safely imports supported records into ISP Management.

The import wizard will be:
- file-based, not live-connected to MixRadius
- staging-first, not direct-write
- dry-run-first, not immediate execution
- tenant-scoped
- lifecycle-aware for packages, subscriptions, billing state, and PPP provisioning

After cutover, ISP Management becomes the source of truth for:
- active subscriptions
- future billing lifecycle
- package lifecycle
- PPPoE provisioning lifecycle

MixRadius data is treated as:
- migration source
- validation source for technical PPP fields
- legacy billing/history source

## Goals / Non-goals
### Goals
- Add a safe admin workflow to migrate MixRadius PPP customer data into ISP Management.
- Support upload of `.sql` and `.sql.gz` MixRadius backups from the web/app.
- Parse and stage MixRadius records before touching production tables.
- Preview:
  - customers to create or merge
  - packages to create or reuse
  - subscriptions to create or update
  - PPPoE accounts to create or update
  - conflicts, warnings, and blocked items
- Preserve ISP Management lifecycle integrity for:
  - package ownership
  - billing lifecycle
  - subscription state
  - PPP provisioning state
- Make repeated imports idempotent.
- Keep auditability and rollback visibility per import batch.

### Non-goals
- No direct live sync with MixRadius in phase one.
- No voucher migration in phase one.
- No hotspot migration in phase one.
- No raw database restore into ISP Management.
- No mass auto-apply to routers or Managed RADIUS during import in phase one.
- No full `radacct` analytics migration in phase one.
- No background recurring MixRadius sync in phase one.

## Recommended Architecture
Use a five-layer import architecture:

1. Upload and batch registration
2. SQL backup parser
3. Staging tables
4. Mapping and preview resolver
5. Safe execution into domain services

This keeps parsing concerns separate from domain lifecycle concerns.

### Why this architecture fits the existing codebase
- The current PPPoE module already has preview/import patterns for router-based imports.
- The current domain model already separates customers, locations, packages, subscriptions, and PPPoE accounts.
- The current product already values auditability and safe tenant-scoped workflows.
- A staging-based design can reuse existing customer, package, and PPPoE services instead of bypassing lifecycle rules.

## Route and UX Design
Place the feature in the tenant admin PPPoE/network area, not superadmin.

Recommended route:
- `/{tenant}/admin/network/pppoe/import-mixradius`

The wizard should also be reachable from the existing PPPoE page as a primary action:
- `Import from MixRadius`

### Wizard steps
1. Upload backup
2. Review source summary
3. Mapping and conflict review
4. Preview import outcome
5. Execute import

### Step 1: Upload backup
Inputs:
- `.sql`
- `.sql.gz`

Immediate validations:
- file extension
- file size
- gzip readability when applicable
- required MixRadius tables present

Immediate summary:
- backup file name
- parse status
- detected tables
- detected coverage window
- counts for customers, plans, routers, transactions

### Step 2: Review source summary
Show:
- total PPP customers
- total hotspot customers found but unsupported in MVP
- total plans
- total NAS rows
- total transactions
- total accounting rows
- total rows with location metadata

Show warnings such as:
- missing password rows
- missing plan rows
- duplicate usernames
- plans with mismatched price or bandwidth
- unsupported table gaps

### Step 3: Mapping and conflict review
Allow the admin to map:
- MixRadius NAS -> ISP Management router
- MixRadius plan -> existing package or new package
- conflicting customer candidates -> merge or create new
- ODP/POP -> location metadata strategy

Recommended import policies:
- package exact-name reuse enabled by default
- customer merge disabled by default unless confidence is high
- router mapping manual when multiple NAS rows exist
- unsupported or ambiguous records blocked until reviewed

### Step 4: Preview import outcome
Show separate tabs:
- Customers
- Packages
- Subscriptions
- PPPoE Accounts
- Warnings
- Conflicts

Preview summary should explicitly show:
- create count
- update count
- skip count
- blocked count

### Step 5: Execute import
Execution modes:
- `Preview only`
- `Safe import` (recommended default)
- `Force sync`

Execution result should show:
- created records
- updated records
- skipped records
- failed records
- conflicts still unresolved
- link to batch report

## Data Model Changes
### Import batch and staging tables
Add staging tables for MixRadius imports:
- `mixradius_import_batches`
- `mixradius_staging_nas`
- `mixradius_staging_plans`
- `mixradius_staging_customers`
- `mixradius_staging_customer_locations`
- `mixradius_staging_transactions`
- `mixradius_staging_usage`
- `mixradius_import_conflicts`

### `mixradius_import_batches`
Responsibility:
- one row per uploaded backup
- tenant ownership
- lifecycle status
- summary and audit metadata

Recommended fields:
- `id`
- `tenant_id`
- `source_filename`
- `source_sha256`
- `source_size_bytes`
- `parse_status`
- `execution_status`
- `started_at`
- `completed_at`
- `summary_json`
- `error_json`
- `created_by`
- `created_at`
- `updated_at`

### Staging customer rows
Store raw or normalized source fields required for mapping:
- `member_id`
- `username`
- `password`
- `fullname`
- `email`
- `phonenumber`
- `identity_number`
- `address`
- `created_at`
- `plan_name`
- `price`
- `total`
- `renewed_on`
- `expired_on`
- `trx_invoice`
- `trx_status`
- `payment_type`
- `auth_status`
- `bind_mac`
- `mac_address`
- source latitude/longitude
- source ODP ID / ODP name
- source technical fields from RADIUS resolution

### External reference tracking
Do not overload core production tables with many one-off MixRadius columns.

Preferred approach:
- add generic external-reference tables or equivalent metadata tables for:
  - customers
  - packages
  - PPPoE accounts
  - import-created subscriptions

Each external ref should support:
- `source_system`
- `source_ref`
- `import_batch_id`
- `last_seen_at`

Example source refs:
- `mixradius:customer:{member_id}`
- `mixradius:pppoe:{username}`
- `mixradius:plan:{plan_id}`
- `mixradius:invoice:{invoice}`

## Source Table Coverage
### Required in MVP
- `nas`
- `tbl_customers`
- `tbl_customers_sub`
- `tbl_customers_map`
- `tbl_odp_data`
- `tbl_plans`
- `tbl_bandwidth`
- `radcheck`
- `radreply`
- `radusergroup`
- `tbl_transactions`

### Optional in MVP
- `radacct`
- `tbl_usage_reports`

### Explicitly ignored in MVP
- hotspot voucher tables
- e-voucher tables
- online payment gateway transaction tables
- MixRadius operator/admin user tables for access management migration

## Lifecycle Mapping Rules
The importer must normalize MixRadius lifecycle fields into ISP Management’s domain model rather than copying them blindly.

### Package lifecycle
Source:
- `tbl_plans`
- `tbl_bandwidth`
- `radusergroup`

Target:
- `isp_packages`
- `isp_package_router_mappings`

Import rules:
- create or reuse tenant-scoped packages by exact name first
- preserve source metadata for:
  - bandwidth name
  - sell price
  - validity
  - shared users
  - profile group
- service type defaults to PPP/internet PPPoE package semantics
- plan name remains the visible package name

### Customer lifecycle
Source:
- `tbl_customers`
- `tbl_customers_map`
- `tbl_odp_data`

Target:
- `customers`
- `customer_locations`

Import rules:
- one MixRadius PPP customer becomes one ISP customer
- one default service location is created per imported customer
- coordinates map to customer location latitude/longitude
- ODP/POP data is preserved as location notes or source metadata in phase one
- customer active flag defaults to active unless explicitly blocked by import policy

### Subscription lifecycle
Source:
- `tbl_customers`
- `tbl_customers_sub`
- `tbl_transactions`

Target:
- `customer_subscriptions`

Import rules:
- package is resolved first
- one primary subscription per imported PPP location
- `starts_at` derives from best available source:
  - `renewed_on` if present and meaningful
  - else `created_at`
- `ends_at` derives from `expired_on`
- `price` derives from:
  - `tbl_customers.total` first
  - fallback `tbl_plans.sell_price`
- `billing_cycle` derives from plan validity:
  - `1 M` -> `monthly`
  - supported yearly values may map to `yearly`
  - unsupported validity patterns remain monthly with source metadata preserved

### Subscription status rules
Recommended normalized status rules:
- `PAID` and not expired -> `active`
- `UNPAID` and expired or isolated -> `suspended`
- `UNPAID` and not expired -> `active`, but flagged in preview as billing attention
- `PENDING` -> `active` or `needs review` metadata, depending on source dates
- explicit cancellation is never auto-inferred in phase one

This avoids false cancellations during migration.

### PPPoE provisioning lifecycle
Source:
- `tbl_customers`
- `radcheck`
- `radreply`
- `radusergroup`

Target:
- `pppoe_accounts`

Import rules:
- `username` from MixRadius username
- `password_enc` derived from plaintext MixRadius password or `Cleartext-Password`
- `remote_address` from `Framed-IP-Address` when available
- `router_profile_name` from package/router mapping when resolved
- `package_id` linked to imported or reused package
- `router_id` mapped from NAS resolver
- `disabled` should not be inferred from billing alone

The subscription lifecycle and PPP enable/disable lifecycle must remain separate, even if later automation chooses to coordinate them.

### Billing lifecycle
Source:
- `tbl_transactions`

Phase-one decision:
- do not create active production invoices from imported MixRadius transactions
- treat imported transactions as legacy history only
- after cutover, all new billing lifecycle is generated by ISP Management

This prevents mixed invoice engines after migration.

## Mapping and Conflict Resolution
### NAS / router mapping
Source:
- `nas`

Target:
- `mikrotik_routers`

Rules:
- exact host/IP match may be auto-suggested
- multi-router imports require explicit admin review if ambiguous
- unresolved NAS mapping blocks PPP account execution for dependent rows

### Package mapping
Rules:
- exact package-name match -> reuse by default
- no exact match -> create new package candidate
- same name but different price or bandwidth -> `needs_review`

### Customer matching
Recommended confidence order:
1. external ref existing
2. exact `member_id`
3. exact PPP username
4. conservative heuristic by name + phone

Default behavior:
- no automatic heuristic merge unless confidence is high and user has enabled it

### PPP account matching
Rules:
- unique by tenant + router + username
- same username on different router -> conflict
- same username on same router -> update existing account candidate

### Conflict states
Each preview row should resolve to one of:
- `auto_matched`
- `needs_review`
- `conflict`
- `blocked`
- `skipped`

### Update precedence
By default, preserve local ISP Management edits for customer-facing profile data.

Safe overwrite candidates:
- subscription end date
- subscription renewal date
- package assignment
- technical PPP fields

Do not auto-overwrite by default:
- manually edited customer name
- manually improved address fields
- local notes
- internal operational annotations

## Idempotency Rules
The importer must be safe to run repeatedly on the same backup or newer backups from the same MixRadius system.

Requirements:
- repeated imports do not create duplicate customers
- repeated imports do not create duplicate packages
- repeated imports do not create duplicate PPPoE accounts
- repeated imports update matched records only when source data changed

Idempotency is anchored on external reference keys, not only display names.

## Backend Unit Design
### HTTP layer
Add a new tenant-admin HTTP surface dedicated to MixRadius import.

Recommended routes:
- `POST /api/admin/mixradius-import/upload`
- `GET /api/admin/mixradius-import/batches`
- `GET /api/admin/mixradius-import/batches/:id`
- `POST /api/admin/mixradius-import/batches/:id/preview`
- `POST /api/admin/mixradius-import/batches/:id/execute`
- `POST /api/admin/mixradius-import/batches/:id/cancel`

The Tauri command layer should mirror the same operations.

### `mixradius_import_service`
Responsibilities:
- register upload batch
- parse SQL backup
- stage relevant rows
- build preview
- resolve conflicts
- execute domain writes
- persist report and audit output

### SQL parser unit
Responsibilities:
- accept plain SQL and gzipped SQL
- parse only supported tables
- validate required table presence
- produce normalized staging records

This unit should remain independent from business lifecycle rules.

### Execution unit
Responsibilities:
- resolve mappings
- call existing domain services when possible:
  - customer services
  - ISP package services
  - PPPoE services
- enforce safe ordering

Recommended execution order:
1. packages
2. customers
3. locations
4. subscriptions
5. PPPoE accounts

### Transaction strategy
Do not wrap the full import batch in one giant transaction.

Recommended approach:
- transaction per major domain execution step
- batch status and partial progress recorded after each step
- failure in a later step does not erase successfully imported earlier steps

This supports safer recovery and clearer audit visibility.

## Permissions / Authorization
- tenant admin with `pppoe:manage` may use the import wizard
- read-only PPPoE users may not execute imports
- import batches are tenant-scoped and must never leak across tenants

## Error Handling
The wizard should stop early on:
- unreadable file
- unsupported format
- missing required tables
- parse failure

The wizard should continue with warnings on:
- unsupported optional tables
- missing accounting rows
- package metadata mismatches
- missing geolocation rows

Execution should be blocked for rows with:
- unresolved router mapping
- unresolved package conflict
- duplicate PPP identity conflict

## Testing Strategy
### Parser tests
- `.sql` parsing succeeds on valid MixRadius backup
- `.sql.gz` parsing succeeds on valid MixRadius backup
- malformed gzip fails cleanly
- missing required tables fail with actionable error

### Preview tests
- preview counts match known fixture counts
- conflicts surface when username duplicates exist
- package reuse is suggested for exact package-name matches
- unresolved routers block dependent PPP rows

### Execution tests
- customers are created once and reused on repeated import
- packages are reused on repeated import
- subscriptions follow normalized status rules
- PPP passwords are encrypted into `password_enc`
- PPP rows upsert by tenant + router + username

### Lifecycle tests
- `PAID + not expired -> active`
- `UNPAID + expired -> suspended`
- `UNPAID + not expired -> active with billing attention`
- package mapping preserves future package lifecycle semantics
- imported legacy transactions do not generate active production invoices

## Rollout / MVP
### MVP phase one
Ship:
- upload
- parse
- staging
- preview
- safe execution
- customer/package/subscription/PPPoE import
- conflict reporting
- audit trail

### Explicitly delayed beyond MVP
- hotspot migration
- voucher migration
- full accounting-history import
- live MixRadius connection
- automatic post-import provisioning apply

## Open Questions
- Whether imported legacy transactions should get a dedicated read-only UI in phase one or remain batch-report only.
- Whether ODP/POP should map only into location metadata in phase one or also seed future network-mapping nodes.
- Whether a low-risk rollback tool should be included in MVP or deferred until after first successful migration runs.

## Recommended Next Step
After this design is accepted, create an implementation plan that breaks the feature into these phases:
1. schema and staging foundations
2. parser and preview pipeline
3. wizard UI
4. safe execution pipeline
5. lifecycle and idempotency hardening
6. verification using the validated MixRadius backup fixture
