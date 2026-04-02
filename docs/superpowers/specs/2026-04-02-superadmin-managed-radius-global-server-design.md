# Superadmin Managed RADIUS Global Server Design (2026-04-02)

## Background / Current State
- Managed RADIUS foundation and hybrid PPPoE provisioning already exist.
- Current control-plane implementation uses `managed_radius_servers` as a tenant-scoped table.
- That model works for pilots, but it does not match the intended SaaS operating model.
- In the desired SaaS model, superadmin owns the centralized RADIUS infrastructure and tenants consume it as a shared platform service.
- The likely production operating model is simple:
  - one or a few shared RADIUS servers for the whole platform
  - one active server assignment per tenant
  - multiple routers per tenant mapped to that assigned server

## Product Decision
Refactor Managed RADIUS from tenant-scoped server records to global server records.

Target operating model:
- superadmin can create multiple global RADIUS servers
- one global RADIUS server can be used by multiple tenants
- one tenant can have exactly one active server assignment at a time
- one tenant can have multiple routers
- each router keeps its own NAS/shared-secret mapping

This preserves operational clarity while aligning the feature with a real SaaS control plane.

## Goals / Non-goals
### Goals
- Make RADIUS servers truly platform-owned and reusable across tenants.
- Keep tenant-side operations simple: a tenant uses one assigned RADIUS server at a time.
- Preserve per-router NAS/shared-secret isolation.
- Keep tenant router detail CLI and masked-secret UX.
- Support safe migration from the current tenant-scoped server model.
- Minimize ambiguity in superadmin UI by separating infrastructure, tenant assignment, and router mapping.

### Non-goals
- No multi-active server assignment per tenant in this phase.
- No automatic failover or load balancing between RADIUS servers in this phase.
- No per-tenant dedicated FreeRADIUS instance lifecycle management in this phase.
- No redesign of PPPoE business ownership; billing DB remains the source of truth.
- No bulk end-user migration workflow beyond schema/data transition needed for this refactor.

## Recommended Architecture
Split the model into three layers:

1. Global RADIUS infrastructure
   - owned by superadmin
   - reusable by many tenants

2. Tenant assignment
   - each tenant is assigned exactly one active global server

3. Router NAS mapping
   - each router maps to the tenant’s assigned server
   - each router keeps its own shared secret and NAS identity

This creates clean boundaries:
- server infrastructure is platform-scoped
- tenant consumption is tenant-scoped
- NAS client auth is router-scoped

## Data Model

### 1. Global Server Table
Replace tenant-scoped `managed_radius_servers` with a global table, for example `radius_servers`.

Recommended fields:
- `id`
- `name`
- `db_host`
- `db_port`
- `db_name`
- `db_user`
- `db_password_enc`
- `is_active`
- `notes`
- `created_at`
- `updated_at`

Rules:
- `name` must be unique globally
- database password remains encrypted at rest
- inactive servers cannot receive new tenant assignments

Design note:
- this table represents platform infrastructure, not tenant ownership
- `tenant_id` is removed from this layer

### 2. Tenant Assignment Table
Add a new table, for example `tenant_radius_assignments`.

Recommended fields:
- `id`
- `tenant_id`
- `radius_server_id`
- `is_active`
- `assigned_at`
- `created_at`
- `updated_at`

Rules:
- one tenant may have many historical rows
- only one row may be active per tenant at a time
- an active assignment points to exactly one active global server

Recommended DB constraints:
- unique partial index on `(tenant_id)` where `is_active = true`

Operational meaning:
- this is the authoritative “tenant uses server X” record

### 3. NAS / Router Mapping Table
Keep `managed_radius_nas`, but make it reference the global server table.

Recommended fields:
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

Rules:
- router must belong to tenant
- radius server must match the tenant’s active assignment
- one router has one active NAS mapping
- secret remains unique per router/NAS

Recommended validation:
- prevent creating a NAS mapping to a server that is not actively assigned to that tenant

## Authority Model

### Superadmin
Superadmin controls:
- global server CRUD
- tenant-to-server assignment CRUD
- NAS mapping CRUD
- shared secret reveal/rotation

### Tenant Admin
Tenant admin controls:
- PPPoE accounts and apply/reconcile flows
- router operations
- router detail RADIUS CLI copy
- masked secret visibility

Tenant admin does not control:
- global server inventory
- cross-tenant assignment
- raw secret reveal unless explicitly permission-gated

Recommended permission gate remains:
- `network_routers.manage_radius_secret`

## Validation Rules

### Global Server
- `name`, `db_host`, `db_name`, and `db_user` are required
- `db_port` defaults to `5432`
- `is_active=false` blocks new active assignments

### Tenant Assignment
- `tenant_id` is required
- `radius_server_id` is required
- assigned server must exist and be active
- activating one assignment deactivates previous active assignment for the same tenant in the same transaction

### Router NAS Mapping
- router must belong to tenant
- server must equal the tenant’s active assigned server
- `nas_ip_or_cidr` is required
- shared secret is required or auto-generated on create
- rotating secret updates encrypted value and invalidates previously copied CLI operationally

## Service Boundaries

### `ManagedRadiusService`
Extend the existing service with three responsibilities:

1. Global server management
2. Tenant assignment management
3. Router NAS mapping and RADIUS store provisioning

Suggested internal service boundaries:
- `RadiusInfrastructureService` behavior inside `ManagedRadiusService`
- `TenantRadiusAssignmentService` behavior inside `ManagedRadiusService`
- `RadiusNasService` behavior inside `ManagedRadiusService`

This can be one service class initially, but the code should be organized around those concerns.

## Runtime Resolution Model
At provisioning/runtime:

1. PPPoE account is marked `managed_radius`
2. Account belongs to tenant
3. Tenant resolves to its one active assigned global RADIUS server
4. Router resolves to its NAS mapping and shared secret
5. Managed RADIUS provisioning writes into the execution store for that server

This keeps auth resolution tenant-safe while allowing infrastructure reuse.

## UX Design

## `/superadmin/radius`
Refactor the page into three sections:

### 1. Global Servers
Purpose:
- manage actual RADIUS infrastructure nodes

Each row shows:
- server name
- DB host
- DB name
- active/inactive
- tenant count
- router count
- updated at

Actions:
- create
- edit
- activate/deactivate
- filter assignments and mappings by this server

### 2. Tenant Assignments
Purpose:
- show which tenant is attached to which global server

Each row shows:
- tenant
- assigned server
- active/inactive
- router count
- updated at

Actions:
- create assignment
- change assigned server
- activate/deactivate assignment
- open router mappings filtered to this tenant

Simplification:
- keep one active assignment per tenant
- UI should make this obvious and constrained

### 3. Router / NAS Mappings
Purpose:
- configure router-to-RADIUS auth details

Each row shows:
- tenant
- server
- router
- NAS IP/CIDR
- shortname
- masked secret
- active/inactive
- updated at

Actions:
- create
- edit
- rotate secret
- reveal secret
- copy CLI
- activate/deactivate

### Mapping Workflow
Recommended create flow:
1. choose tenant
2. auto-resolve that tenant’s active assigned server
3. choose router from that tenant
4. set NAS details
5. generate or provide secret

This avoids ambiguous server selection and reinforces the one-server-per-tenant model.

## Tenant Router Detail UX
Tenant router detail should continue to show:
- whether Managed RADIUS is configured
- assigned server name
- copy-ready RouterOS CLI
- masked secret by default

Adjustments:
- raw secret must be omitted unless reveal permission is present
- copy CLI remains available without reveal permission

## Migration Strategy

### Phase A: Add New Global Schema
Create:
- `radius_servers`
- `tenant_radius_assignments`

Update:
- `managed_radius_nas.radius_server_id` to reference global server IDs

### Phase B: Migrate Existing Tenant-Scoped Servers
For each row in existing `managed_radius_servers`:
- create or match a global `radius_servers` row by normalized connection identity
- create tenant assignment row pointing to that global server

Expected near-term reality:
- many tenants may collapse onto the same one real server
- migration should support deduplication by connection tuple:
  - `db_host`
  - `db_port`
  - `db_name`
  - `db_user`

### Phase C: Re-point NAS Mappings
Update `managed_radius_nas.radius_server_id` to the new global server IDs.

### Phase D: Remove Legacy Tenant-Scoped Server Table
After verification:
- drop old `managed_radius_servers`
- or rename/archive temporarily if rollback safety is preferred

## Rollback Strategy
Safe rollback path:
- keep old server table until migration is verified
- migrate forward with reversible mapping table between old and new server IDs
- if rollback is needed, restore NAS references and tenant assignment state from migration metadata

Because the current expected deployment is likely a single shared server, rollback complexity is manageable.

## API Design

### Global Servers
- list global servers
- create global server
- update global server
- activate/deactivate global server

### Tenant Assignments
- list tenant assignments
- create tenant assignment
- update tenant assignment
- activate/deactivate tenant assignment

### NAS Mappings
- list mappings
- create mapping
- update mapping
- activate/deactivate mapping
- rotate secret
- reveal secret

Important design rule:
- NAS mapping create/update should not freely select any server
- it should use the tenant’s active assigned server or fail clearly if tenant has no active assignment

## Recommended Implementation Order
1. add spec-approved new schema
2. build backend for global servers
3. build backend for tenant assignments
4. update NAS backend validation to rely on active tenant assignment
5. migrate superadmin UI to the three-section model
6. add migration script from current tenant-scoped servers
7. remove old tenant-scoped server model

## Recommendation
Adopt the global-server refactor now, before tenant adoption grows.

Why:
- it matches the real SaaS operational model
- it keeps superadmin as the owner of centralized infrastructure
- it simplifies tenant understanding: one tenant, one assigned RADIUS server
- it keeps secrets isolated where they belong: per router/NAS
- it reduces future rework before wider rollout
