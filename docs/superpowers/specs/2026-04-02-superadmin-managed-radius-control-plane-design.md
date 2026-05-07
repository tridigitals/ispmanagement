# Superadmin Managed RADIUS Control Plane Design (2026-04-02)

## Background / Current State
- Managed RADIUS hybrid provisioning already exists for tenant PPPoE operations.
- `/superadmin/radius` currently acts as a read-only observability surface for global servers and managed-RADIUS-backed users.
- Managed RADIUS infrastructure metadata lives in the billing database:
  - `managed_radius_servers`
  - `managed_radius_nas`
- Tenant admins can already copy MikroTik RADIUS setup CLI from router detail pages.
- Today, server and NAS bootstrap still depend on manual setup or one-off scripts rather than a proper SaaS control plane UI.

## Goals / Non-goals
### Goals
- Add a superadmin control plane for managed RADIUS infrastructure.
- Let superadmin create, edit, activate/deactivate, and inspect tenant-scoped managed RADIUS server configuration.
- Let superadmin create, edit, and rotate router-to-server NAS mappings.
- Keep the control model safe for SaaS operations by scoping infrastructure changes through tenant context.
- Preserve tenant admin usability by keeping router detail CLI visible and copyable.
- Keep secrets protected by default while still supporting operational reveal for authorized actors.

### Non-goals
- No direct superadmin CRUD for PPPoE end-user accounts in this phase.
- No multi-server failover orchestration in this phase.
- No per-tenant dedicated external RADIUS runtime lifecycle automation in this phase.
- No secret exposure by default in list views.
- No bulk migration workflow from router-local PPP secrets in this phase.

## Product Decision
Phase 2 should add:
1. Managed RADIUS server CRUD in `/superadmin/radius`
2. Managed RADIUS NAS/router mapping CRUD in `/superadmin/radius`

Recommended operating model:
- one active managed RADIUS server per tenant for phase one of the control plane
- mapping flow starts from tenant context, then router selection inside that tenant
- tenant admin can still access copy-ready CLI on router detail pages
- tenant admin sees secret masked by default
- raw secret reveal is restricted and permission-gated

## Authority Model
### Superadmin
Superadmin owns infrastructure control:
- create server config
- edit server config
- activate/deactivate server config
- create NAS/router mapping
- edit NAS/router mapping
- rotate shared secret
- reveal full secret

### Tenant Admin
Tenant admin keeps tenant operations:
- create/update/apply PPPoE accounts
- inspect whether router has managed RADIUS setup
- copy RouterOS CLI from router detail page
- see masked secret

Raw secret reveal for tenant admins should not be default behavior.

Recommended permission gate:
- `network_routers.manage_radius_secret`

## Server Model
`managed_radius_servers` remains tenant-scoped.

Phase 2 rule:
- allow many rows historically or operationally
- but only one row may be active per tenant at a time

This keeps the schema future-friendly while keeping the UI simple and safe now.

### Server fields exposed in CRUD
- `tenant_id`
- `name`
- `db_host`
- `db_port`
- `db_name`
- `db_user`
- `db_password`
- `is_active`

### Validation rules
- `tenant_id` is required
- `name` is required and unique within tenant
- `db_host`, `db_name`, and `db_user` are required
- `db_port` defaults to `5432`
- when activating one server, any previously active server for that tenant must be deactivated in the same transaction

### Security rules
- `db_password` must remain encrypted at rest
- list pages must never expose decrypted database passwords
- edit forms should show password fields as empty placeholders unless rotated or replaced

## NAS / Router Mapping Model
`managed_radius_nas` remains tenant-scoped and router-scoped.

Mapping flow:
1. choose tenant
2. choose that tenant’s active or target server
3. choose router from that tenant
4. set NAS identity fields
5. generate or set shared secret

### Mapping fields exposed in CRUD
- `tenant_id`
- `radius_server_id`
- `router_id`
- `nas_name`
- `nas_ip_or_cidr`
- `shortname`
- `shared_secret`
- `is_active`

### Validation rules
- router must belong to selected tenant
- server must belong to selected tenant
- one router maps to one NAS record
- `nas_ip_or_cidr` is required
- shared secret is required on create
- secret rotation updates encrypted value and invalidates stale CLI copies operationally

### Secret behavior
- secret stored encrypted in billing DB
- masked by default in UI
- reveal only on explicit action
- reveal permission:
  - superadmin always allowed
  - tenant admin only if they have `network_routers.manage_radius_secret`

## UX Design
## `/superadmin/radius`
The page evolves from read-only observability into a control plane with two layers:

### Layer 1: Observability Summary
- keep current stats cards
- keep global users table read-only
- keep global server overview

### Layer 2: Infrastructure Management
- add server management section
- add router mapping section

Recommended top-level actions:
- `New Server`
- `New Mapping`

### Server List UX
Each server row should show:
- tenant
- server name
- DB host
- DB name
- active/inactive badge
- mapped router count
- updated at

Actions:
- edit
- activate/deactivate
- open mappings filtered to this server

Avoid showing DB password in list rows.

### Mapping List UX
Each mapping row should show:
- tenant
- server
- router
- NAS IP/CIDR
- shortname
- secret masked
- active/inactive badge
- updated at

Actions:
- edit
- rotate secret
- reveal secret
- copy CLI
- activate/deactivate

### Form UX
#### Server form
- tenant selector
- name
- db host
- db port
- db name
- db user
- db password
- active toggle

#### Mapping form
- tenant selector first
- server selector filtered by tenant
- router selector filtered by tenant
- nas name
- nas ip/cidr
- shortname
- shared secret input or generate button
- active toggle

## Router Detail UX
Router detail page in tenant admin should continue showing:
- managed RADIUS configuration status
- copyable RouterOS CLI
- masked secret

Phase 2 adjustment:
- secret reveal behavior should be permission-gated
- if reveal is not allowed, copy CLI still works

This preserves operational speed while reducing unnecessary secret exposure.

## API Design
Add superadmin-only operations for:

### Managed RADIUS Servers
- list servers
- create server
- update server
- activate/deactivate server

### Managed RADIUS Mappings
- list mappings
- create mapping
- update mapping
- activate/deactivate mapping
- rotate mapping secret
- reveal mapping secret

### Read models
Current observability list endpoints can stay and expand if needed.

### Write models
Use dedicated request DTOs rather than overloading list DTOs.

## Transaction Rules
### Activating a server
Inside one transaction:
- verify tenant ownership
- deactivate other active servers for that tenant
- activate selected server

### Creating/updating mapping
Inside one transaction:
- verify tenant ownership of server and router
- upsert or insert mapping
- if secret changed, re-encrypt and store new value

### Rotating secret
Inside one transaction:
- generate or accept new secret
- encrypt and persist
- update `updated_at`

## Audit Requirements
Superadmin changes should create audit entries for:
- server created
- server updated
- server activated/deactivated
- mapping created
- mapping updated
- mapping secret rotated
- secret revealed

Audit detail should include IDs and tenant context, but not raw secret values.

## Error Handling
- if tenant has no routers, mapping form should explain that router inventory must exist first
- if tenant has no server yet, mapping creation should point user to create server first
- if activation conflicts occur, transactional logic should resolve them deterministically
- if reveal is denied, return authorization error rather than hiding silently

## Testing Strategy
### Backend
- only superadmin can mutate server or mapping records
- activation enforces one active server per tenant
- mapping create/update rejects cross-tenant server/router combinations
- secret rotation persists encrypted values
- reveal endpoint never leaks secret to unauthorized roles

### Frontend
- server form tenant filtering works
- mapping form server/router filtering works
- masked secret displays by default
- reveal control respects permission state
- copy CLI continues to work from router detail

## Rollout
1. Add superadmin server CRUD
2. Add superadmin mapping CRUD
3. Add secret reveal/rotation protections
4. Update tenant router detail permission behavior
5. Validate with pilot tenant before wider rollout

## Acceptance Criteria
- Superadmin can create and edit managed RADIUS server configs in the UI.
- Only one managed RADIUS server can be active per tenant at a time.
- Superadmin can create and edit NAS/router mappings from tenant context.
- Superadmin can rotate and reveal mapping secrets.
- Tenant admin can still copy router CLI from router detail.
- Tenant admin sees masked secret by default.
- Raw secret reveal is permission-gated.
