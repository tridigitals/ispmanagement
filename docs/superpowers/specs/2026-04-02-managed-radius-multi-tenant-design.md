# Managed RADIUS Multi-Tenant Design (2026-04-02)

## Background / Current State
- PPPoE management currently assumes router-local secrets as the only execution target.
- The current PPPoE backend reads and writes `/ppp/secret/*` directly on MikroTik routers in `src-tauri/src/services/pppoe_service.rs`.
- The current PPPoE data model is router-oriented: `pppoe_accounts` stores `router_id`, `router_present`, `router_secret_id`, `last_sync_at`, and `last_error`.
- The schema explicitly describes router state as the sync target, with the database acting as the source of truth for business state.
- There is no existing FreeRADIUS or generic RADIUS management layer in the codebase.
- The product is SaaS / multi-tenant, so any managed RADIUS design must isolate tenant traffic and data safely.

## Goals / Non-goals
### Goals
- Add a managed RADIUS execution path for PPPoE accounts while preserving the existing router-secret path.
- Support hybrid operation during migration: some accounts remain router-backed, others become managed-RADIUS-backed.
- Keep the PPPoE admin experience unified in one feature area.
- Design tenant isolation so multiple tenants can share one managed RADIUS platform safely.
- Use one unique shared secret per router/NAS client.
- Support a shared FreeRADIUS deployment with tenant-aware account lookup.
- Keep existing router-backed accounts working without forced migration.
- Prepare for phased rollout on a single VPS using Docker Compose, with FreeRADIUS and PostgreSQL separated from the billing database.

### Non-goals
- No full implementation in this design document.
- No immediate session accounting dashboard redesign.
- No CoA/DM (Change of Authorization / Disconnect Message) support in phase one.
- No dedicated FreeRADIUS instance per tenant as the default SaaS model.
- No simultaneous dual-active auth source for the same PPPoE account in normal operation.

## Recommended Architecture
1. Keep the billing application as the control plane and business source of truth.
2. Introduce a second provisioning target: `managed_radius`.
3. Retain the current router-local source as `router`.
4. Run a shared FreeRADIUS platform for all tenants.
5. Isolate tenants through NAS-to-tenant mapping and tenant-aware account lookup, not by username alone.
6. Use a separate PostgreSQL database for the managed RADIUS platform, even if it lives on the same VPS.
7. Deploy FreeRADIUS and its PostgreSQL backend as a separate Docker Compose stack from the billing application.

## Source Model
- Each PPPoE account gets a single active execution source:
  - `router`
  - `managed_radius`
- Existing accounts default to `router`.
- New accounts may be created as either `router` or `managed_radius`.
- The application must not treat `router` and `managed_radius` as simultaneously authoritative for the same account.

## Data Model Changes
### PPPoE Accounts
Add the following fields to `pppoe_accounts`:
- `account_source text not null default 'router'`
- `radius_present boolean not null default false`
- `radius_identity text null`
- `radius_last_sync_at timestamp with time zone null`
- `radius_last_error text null`

Recommended behavior:
- `radius_identity` defaults to `username` when omitted.
- `router_*` sync fields continue to apply only to router-backed accounts.
- `radius_*` sync fields apply only to managed-RADIUS-backed accounts.

### Managed RADIUS Server Configuration
Add a tenant-scoped configuration table, for example `managed_radius_servers`:
- `id`
- `tenant_id`
- `name`
- `host`
- `port`
- `db_type`
- `db_host`
- `db_port`
- `db_name`
- `db_user`
- `db_password_enc`
- `radius_secret_enc`
- `is_active`
- `created_at`
- `updated_at`

Design note:
- Phase one may enforce a single active managed RADIUS server per tenant in the UI.
- The table still allows future support for dedicated enterprise setups or failover targets.

### NAS / Router Mapping
Add a tenant-aware NAS mapping table, for example `managed_radius_nas`:
- `id`
- `tenant_id`
- `router_id`
- `radius_server_id`
- `nas_name`
- `nas_ip_or_cidr`
- `shared_secret_enc`
- `shortname`
- `is_active`
- `created_at`
- `updated_at`

Constraints / expectations:
- One router maps to one NAS/client record for its active RADIUS target.
- Shared secret is unique per router/NAS.
- NAS lookup must always be tenant-aware.

### Managed RADIUS Accounts
The billing database remains the business source of truth for PPPoE accounts.
The managed RADIUS database stores the execution representation needed by FreeRADIUS.

Two acceptable implementation options:
1. Use FreeRADIUS-compatible tables directly (for example `radcheck`, `radreply`, `radacct`) behind a service boundary.
2. Use app-owned tenant-aware tables/views in the RADIUS database and expose FreeRADIUS-compatible views or queries from them.

Recommendation:
- Prefer app-owned tenant-aware schema plus a service boundary, even if phase one uses FreeRADIUS-compatible tables underneath.

## Tenant Isolation Model
Tenant isolation must be determined from the NAS/router first, then the PPP username.

Authentication resolution flow:
1. Request arrives from a MikroTik NAS/client.
2. FreeRADIUS identifies the NAS/client by source IP and shared secret.
3. The NAS/client resolves to exactly one `tenant_id`.
4. Username lookup is executed within that tenant scope.
5. Authorization attributes are returned only for that tenant-scoped account.

Why this model:
- Usernames can overlap across tenants.
- Shared RADIUS infrastructure stays safe without requiring globally unique usernames.
- Audit and accounting remain partitionable by tenant and router.

## Service Boundaries
### Existing `PppoeService`
Keep `PppoeService` as the feature entry point for PPPoE operations:
- list
- create
- update
- delete
- apply
- reconcile

### New `ManagedRadiusService`
Introduce a dedicated backend service responsible for:
- validating managed RADIUS configuration
- provisioning tenant accounts into the RADIUS execution store
- updating credentials / disabled state / policy attributes
- deleting or disabling managed-RADIUS-backed accounts
- reconciling billing state with managed RADIUS state
- managing NAS/client provisioning for routers

### Routing Rule
`PppoeService` delegates by `account_source`:
- `router` -> existing MikroTik `/ppp/secret` flow
- `managed_radius` -> `ManagedRadiusService`

This preserves a unified UI while keeping infrastructure-specific code out of the feature surface.

## Provisioning Flow
### Create Account
1. User creates PPPoE account in billing.
2. Billing writes the business record to `pppoe_accounts`.
3. `account_source` determines the target execution system.
4. The account is not treated as successfully applied until target sync succeeds.

### Apply Account
- For `router`:
  - continue using `/ppp/secret/add` or `/ppp/secret/set`
  - update `router_present`, `router_secret_id`, `last_sync_at`, `last_error`
- For `managed_radius`:
  - provision credentials and reply attributes into the managed RADIUS store
  - update `radius_present`, `radius_identity`, `radius_last_sync_at`, `radius_last_error`

### Suspend / Unsuspend
- For `router`: toggle `disabled` and re-apply to router.
- For `managed_radius`: update auth state in the managed RADIUS store and re-sync.

### Password Change
- Billing updates the encrypted password first.
- The password is then pushed to the target source based on `account_source`.

### Delete
- Prefer a safety-first behavior in early phases:
  - disable first where possible
  - reserve hard-delete for explicit removal flows
- Router-backed deletion may preserve current best-effort cleanup behavior.
- Managed-RADIUS-backed deletion should remove or deactivate auth records consistently.

## UI / Product Changes
### PPPoE Accounts Page
Extend `/admin/network/pppoe` with:
- `Source` column
- target-aware status badge:
  - `On router`
  - `On RADIUS`
  - `Out of sync`
- filter by source
- source-aware apply / reconcile actions

### Create / Edit PPPoE Account
Add:
- `Account source` selector
- conditional hints depending on source:
  - router-backed accounts provision local PPP secrets
  - managed-RADIUS-backed accounts provision centralized RADIUS auth

Phase-one simplification:
- do not expose advanced RADIUS reply attributes in the main PPPoE form
- map package/profile to RADIUS policy through backend rules

### Router Configuration UX
Add an admin workflow for:
- linking a router to a managed RADIUS server
- generating or rotating a unique shared secret
- displaying NAS/client settings needed on MikroTik

## Deployment Architecture
Recommended deployment on aaPanel VPS:
- billing app stack remains separate
- managed RADIUS stack is added as its own Compose project

Recommended containers / services:
- `billing-app`
- `billing-postgres`
- `freeradius`
- `radius-postgres`

Key rules:
- `radius-postgres` should not be publicly exposed unless there is a strong operational reason.
- `freeradius` exposes UDP `1812` and `1813`.
- Config, SQL init, and database data should use separate volumes.
- Billing DB and RADIUS DB must remain separate, even if both live on the same VPS.

Recommended Compose strategy:
- keep the current billing compose files unchanged where possible
- add a new `docker-compose.radius.yml`
- use Docker Compose rather than native package installation for easier backup, portability, and lifecycle management

## Migration Strategy
### Phase 1: Foundation
- Add new PPPoE schema fields.
- Add managed RADIUS server / NAS configuration models.
- Do not alter behavior of existing accounts.
- Default all existing accounts to `account_source='router'`.

### Phase 2: Infrastructure
- Deploy FreeRADIUS and `radius-postgres`.
- Validate one test tenant / pilot router manually.
- Configure one router as NAS/client with a unique secret.

### Phase 3: Hybrid Support
- Enable account creation with `managed_radius`.
- Keep existing router-backed accounts unchanged.
- Add source-aware apply and reconcile flows.

### Phase 4: Controlled Migration
- Introduce tooling to copy equivalent account state from billing/router-backed records into managed RADIUS.
- Migrate per router, not globally.
- Verify password, profile, disabled state, and IP mapping before cutover.

### Phase 5: Operational Expansion
- Add accounting ingestion and tenant-aware reporting based on `radacct`.
- Consider session dashboards and online user views after auth stability is proven.

## Rollback Strategy
- Rollback should happen per router / tenant pilot group.
- If a managed RADIUS cutover fails, the router can be returned to local auth configuration.
- Existing router-backed accounts remain available until migration is explicitly completed.
- Avoid deleting router-local auth state until managed RADIUS auth is proven stable for the migration window.

## Error Handling / Observability
- Managed RADIUS sync failures must be truthful and visible in the PPPoE UI.
- `radius_last_error` should store the latest provisioning or reconciliation failure.
- Audit logs should distinguish:
  - PPPoE create/update/delete
  - router apply / reconcile
  - managed RADIUS apply / reconcile
  - NAS/client provisioning changes

Recommended future telemetry:
- provisioning latency
- failed sync count
- auth rejects by tenant/router
- accounting ingestion lag

## Security Considerations
- Encrypt all database and shared secrets at rest using the existing secret-handling approach where practical.
- Use one unique RADIUS shared secret per router/NAS.
- Never rely on username uniqueness alone across tenants.
- Restrict access to the RADIUS database and configuration volumes.
- Prefer internal Docker networking and minimal port exposure.

## Testing Strategy
- Unit tests:
  - source routing logic in `PppoeService`
  - tenant-aware username resolution rules
  - validation for managed RADIUS configuration
  - NAS/client uniqueness and tenant mapping rules
- Integration tests:
  - create/update/apply/delete for `router`
  - create/update/apply/delete for `managed_radius`
  - reconciliation behavior for both sources
  - migration behavior from `router` to `managed_radius`
- End-to-end validation:
  - pilot router authenticates successfully against FreeRADIUS
  - overlapping usernames across two tenants resolve correctly via NAS mapping

## Risks / Mitigations
- **Risk:** Username collision across tenants causes incorrect auth resolution.
  - **Mitigation:** Resolve tenant from NAS/client first; never query by username alone.
- **Risk:** RADIUS accounting load impacts billing database performance.
  - **Mitigation:** Use a separate PostgreSQL database for RADIUS.
- **Risk:** Shared secret reuse increases blast radius.
  - **Mitigation:** Generate and store unique secrets per router/NAS.
- **Risk:** Migration causes customer downtime if cutover is global.
  - **Mitigation:** Migrate per router and keep hybrid support during rollout.
- **Risk:** Business logic becomes tightly coupled to FreeRADIUS table details.
  - **Mitigation:** Introduce `ManagedRadiusService` and keep infrastructure-specific logic behind a boundary.

## Acceptance Criteria
- A PPPoE account can be marked as either `router` or `managed_radius`.
- Existing router-backed accounts remain functional without migration.
- Managed RADIUS design supports shared multi-tenant FreeRADIUS safely.
- Router/NAS secrets are unique per router.
- Billing and RADIUS databases are separated.
- Deployment target supports Docker Compose on aaPanel VPS.
- Hybrid migration from router-backed PPPoE to managed RADIUS is supported at the design level.
