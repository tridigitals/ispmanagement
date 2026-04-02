# Superadmin Managed RADIUS Observability Design (2026-04-02)

## Background / Current State
- Managed RADIUS hybrid provisioning is already available in tenant admin flows.
- Tenant admins can provision PPPoE accounts either to router-local PPP secrets or to the managed RADIUS platform.
- Router detail pages already expose router-specific MikroTik CLI setup for the shared RADIUS server and NAS shared secret.
- SaaS infrastructure ownership belongs to the platform operator, so global RADIUS visibility should exist under `/superadmin`.
- Tenant admins should remain the authority for editing PPPoE business records and applying changes for their own tenant.

## Goals / Non-goals
### Goals
- Add a central `/superadmin/radius` area for platform-wide visibility into managed RADIUS infrastructure and users.
- Let superadmins inspect managed RADIUS server configuration coverage across tenants and routers.
- Let superadmins inspect managed-RADIUS-backed PPPoE users across all tenants from the billing control plane.
- Keep the initial release operationally useful with low risk: read-only, filterable, and easy for support/debugging.
- Reuse existing superadmin navigation, data-table, stats, and filtering patterns where possible.

### Non-goals
- No tenant-crossing user edits from superadmin in phase one.
- No password reset, delete, disable, or apply actions from the superadmin page in phase one.
- No direct record editing inside the managed RADIUS PostgreSQL database from the UI.
- No per-tenant dedicated FreeRADIUS orchestration workflow in this phase.

## Product Decision
The first superadmin RADIUS module should be an observability surface, not a control surface.

That means:
- superadmin can view global managed RADIUS servers
- superadmin can view global managed-RADIUS-backed PPPoE users
- tenant admins continue to own create, update, apply, suspend, and delete behavior for PPPoE accounts

This preserves a clean authority split:
- `/superadmin` owns platform oversight
- `/{tenant}/admin/network/*` owns tenant operations

## Recommended Route Structure
- Add sidebar item: `RADIUS`
- Add route: `/superadmin/radius`

The page should have two sections on one screen:
1. `Servers`
2. `Users`

This keeps platform operators in one place during troubleshooting.

## Data Sources
### Servers
Read from the billing database tables already used for managed RADIUS infrastructure metadata:
- `managed_radius_servers`
- `managed_radius_nas`
- `mikrotik_routers`
- `tenants`

The server list should represent control-plane configuration, not runtime container introspection.

### Users
Read from the billing database `pppoe_accounts` table, not directly from the managed RADIUS PostgreSQL database.

Filter to:
- `account_source = 'managed_radius'`

This is the correct source for the superadmin list because:
- it reflects SaaS business state
- it already stores sync health fields like `radius_present`, `radius_last_sync_at`, and `radius_last_error`
- it avoids exposing platform internals that bypass app-level meaning

## Page Design
### Top Summary
Show small stats cards, for example:
- total managed RADIUS servers
- active NAS/router mappings
- managed RADIUS users
- users out of sync

### Servers Section
Purpose:
- answer “which tenants and routers are attached to managed RADIUS?”

Recommended columns:
- server name
- tenant
- public host
- auth/acct ports
- database host and database name
- active status
- mapped routers / NAS count
- updated at

Recommended server filters:
- search by tenant, server name, or host
- active status

Phase-one server actions:
- view only

### Users Section
Purpose:
- answer “which tenant users are expected to exist in managed RADIUS, and are they healthy?”

Recommended columns:
- tenant
- router
- username
- radius identity
- profile / package hint
- status
- last synced at
- last error

Status should be derived from billing sync fields:
- `Provisioned` when `radius_present = true`
- `Not provisioned` when `radius_present = false`
- optionally highlight `Needs attention` when `radius_last_error` is present

Recommended user filters:
- search by username, radius identity, router, or tenant
- tenant
- router
- status (`all`, `provisioned`, `not_provisioned`)

Phase-one user actions:
- view only

## API Design
Expose superadmin-only backend endpoints or commands for:

### List Managed RADIUS Servers
Response fields should include enough data for the server table and stats:
- `id`
- `tenant_id`
- `tenant_name`
- `name`
- `host`
- `auth_port`
- `acct_port`
- `db_host`
- `db_port`
- `db_name`
- `is_active`
- `router_count`
- `updated_at`

### List Managed RADIUS Users
Response fields should include:
- `id`
- `tenant_id`
- `tenant_name`
- `router_id`
- `router_name`
- `username`
- `radius_identity`
- `account_source`
- `radius_present`
- `radius_last_sync_at`
- `radius_last_error`
- `router_profile_name`
- `updated_at`

Authorization rule:
- superadmin only

The implementation can use either Tauri commands, HTTP handlers, or both, following the current dual-surface pattern already used in this project.

## Query Strategy
### Servers
Use aggregated billing-side joins:
- `managed_radius_servers` joined to `tenants`
- left join `managed_radius_nas`
- left join `mikrotik_routers`

Return one row per managed RADIUS server with `router_count`.

### Users
Use billing-side joins:
- `pppoe_accounts`
- `tenants`
- `mikrotik_routers`

Filter:
- `pppoe_accounts.account_source = 'managed_radius'`

This allows the page to stay fast and avoids cross-database fanout into the FreeRADIUS PostgreSQL instance.

## UX Behavior
- Follow existing superadmin page patterns: stats, toolbar filters, responsive table/cards where already standard.
- Keep terminology neutral and operator-friendly.
- Reuse the same source-aware wording already introduced in tenant PPPoE pages:
  - `Provisioned`
  - `Not provisioned`
- If no managed RADIUS configuration exists, show an empty-state message explaining that tenant admins must first configure a managed RADIUS server and NAS mapping.

## Error Handling
- If the list query fails, show a standard error state with retry.
- If there are no servers but there are managed-RADIUS users, show the users list and note that infrastructure metadata is missing or incomplete.
- If there are servers but no users, keep the page useful as an infrastructure audit view.

## Security / Data Exposure
- Do not expose decrypted database passwords or NAS shared secrets on this page.
- Do not expose router CLI snippets in superadmin phase one.
- Mask or omit any secret-bearing fields from server responses.
- Superadmin can see tenant names and usernames because this is a platform-operations surface, but write actions remain tenant-scoped elsewhere.

## Testing Strategy
- Backend tests:
  - superadmin authorization enforced
  - managed RADIUS server aggregation returns tenant and router counts correctly
  - managed RADIUS users query returns only `account_source = 'managed_radius'`
  - out-of-sync / error fields are preserved in the response
- Frontend tests:
  - stats derivation
  - filtering by search, tenant, router, and status
  - empty states and error states

## Rollout
1. Add read-only superadmin backend queries.
2. Add frontend API wrappers and types.
3. Add `/superadmin/radius` page and sidebar entry.
4. Verify with real pilot data from the existing managed RADIUS tenant.

## Acceptance Criteria
- Superadmin sidebar contains a `RADIUS` entry.
- `/superadmin/radius` loads only for superadmins.
- Superadmin can see managed RADIUS servers across tenants without any secret values being exposed.
- Superadmin can see managed-RADIUS-backed PPPoE users across tenants.
- User status reflects billing sync state using `radius_present`, `radius_last_sync_at`, and `radius_last_error`.
- The feature is read-only in phase one.
