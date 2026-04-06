# IP Pool CRUD Design (2026-04-03)

## Background / Current State
- Tenant admin already has a dedicated IP Pool page at `/[tenant]/admin/network/ip-pools`.
- The current page is read-oriented: it can list IP pools per router and sync them from RouterOS into PostgreSQL.
- Backend support currently exists only for:
  - listing IP pools for a router
  - syncing IP pools from a router into `mikrotik_ip_pools`
- The `mikrotik_ip_pools` table already stores a tenant-scoped, router-scoped snapshot of RouterOS pool data.
- Other tenant features already depend on pool names, especially:
  - PPPoE accounts via `address_pool`
  - ISP package router mappings via `address_pool`
  - installation and customer flows that surface or reuse those mappings

## Goals / Non-goals
### Goals
- Add full tenant-admin CRUD for IP pools from `/[tenant]/admin/network/ip-pools`.
- Make RouterOS the source of truth for create, update, and delete.
- Keep PostgreSQL as a synchronized mirror for fast reads, filtering, warning checks, and cross-module references.
- Support only the standard phase-one fields:
  - `name`
  - `ranges`
  - `next_pool`
  - `comment`
- Reuse existing MikroTik router permissions and page patterns already present in network admin.

### Non-goals
- No IP pool rename behavior in phase one.
- No bulk import/export workflow changes.
- No cross-router pool copy or template system.
- No background reconciliation daemon beyond the existing manual sync behavior.
- No migration of dependent modules from `address_pool` string references to pool foreign keys in this phase.

## Product Decision
IP pool CRUD will be a `router-first` workflow.

That means:
- create writes to RouterOS first
- update writes to RouterOS first
- delete removes from RouterOS first
- PostgreSQL is updated only as a mirror after the RouterOS operation succeeds

Delete behavior differs from PPP profile CRUD:
- dependency checks produce warnings, not a hard backend block

## Authority and Data Model
### Source of truth
- RouterOS is authoritative.
- `mikrotik_ip_pools` is a cache/snapshot table, not the canonical configuration owner.

### Why still store rows in PostgreSQL
- the page can list quickly without reconnecting to a router each time
- the app can search, filter, and show sync metadata
- delete warnings can show dependency impact before touching RouterOS
- other modules can continue using pool-name-based relationships

### Consistency rule
- Never report a CRUD operation as successful unless the RouterOS write succeeded.
- After a successful RouterOS write, refresh the PostgreSQL mirror immediately.
- If the RouterOS write succeeds but mirror refresh fails, return a dedicated sync error so the UI can warn the user that RouterOS changed but the local cache still needs refresh.

## Route and UX Design
Keep the existing route:
- `/[tenant]/admin/network/ip-pools`

Keep the existing router filter as the main working context:
- every CRUD action is scoped to the currently selected router
- no create/edit/delete is allowed unless a router is selected

### Page actions
- `Refresh`
- `Sync from router`
- `Add pool`

`Refresh` reloads the current router's mirrored IP pool list from PostgreSQL only and must not contact RouterOS.

### Row actions
- `Edit`
- `Delete`

### Form shape
Use a modal or dialog form with phase-one fields:
- `name` (required)
- `ranges`
- `next_pool`
- `comment`

Edit-mode behavior:
- show `name` as read-only so the current pool identity remains visible
- do not allow editing the `name` field in phase one
- edit submissions should send only mutable fields plus the row ID/router context

The page should continue showing:
- pool name
- ranges
- next pool
- router presence state
- last synced time

### Delete UX
- User clicks `Delete`.
- UI first requests dependency information for the selected pool.
- If dependencies exist, the dialog shows a strong warning with dependency counts, but still allows delete confirmation.
- If no dependencies exist, the dialog behaves like a normal delete confirmation.
- Backend should re-run dependency lookup immediately before the RouterOS delete and include the latest warning counts in the delete result when applicable.

## API Design
Recommended HTTP routes:
- `GET /admin/mikrotik/routers/:routerId/ip-pools`
- `POST /admin/mikrotik/routers/:routerId/ip-pools`
- `PUT /admin/mikrotik/routers/:routerId/ip-pools/:id`
- `DELETE /admin/mikrotik/routers/:routerId/ip-pools/:id`
- `GET /admin/mikrotik/routers/:routerId/ip-pools/:id/dependencies`
- `POST /admin/mikrotik/routers/:routerId/ip-pools/sync`

The Tauri command layer should mirror the same operations.

Route identity contract:
- `:routerId` always identifies the selected router context
- `:id` always means the local PostgreSQL mirror row ID from `mikrotik_ip_pools.id`
- router operations must never treat `:id` as a RouterOS name or internal RouterOS handle

### Authorization
- `read` permission on `network_routers` for list and dependency lookup
- `manage` permission on `network_routers` for create, update, delete, and sync

## Backend Unit Design
### HTTP / command handlers
Responsibility:
- authorize tenant user
- parse route and payload
- call the MikroTik service
- return the final mirrored pool row, dependency payload, or delete result

### MikroTik service IP pool CRUD unit
Responsibility:
- resolve router ownership within tenant scope
- connect to RouterOS
- execute RouterOS create/update/delete commands
- synchronize PostgreSQL mirror after successful router writes
- compute dependency warnings before delete

### Mirror refresh strategy
Phase-one implementation must use one path only:
- run router-scoped IP pool sync after every successful create/update/delete

## Dependency Warning Unit
Responsibility:
- return whether an IP pool name is referenced by tenant records on the same router
- produce human-readable counts for warning UI

Phase-one dependency sources:
- `pppoe_accounts` where `router_id` and `address_pool` match
- `isp_package_router_mappings` where `router_id` and `address_pool` match

## RouterOS Operation Rules
### Shared request payload for create and update
```json
{
  "name": "pool-basic-10m",
  "ranges": "10.10.10.10-10.10.10.200",
  "next_pool": "pool-overflow",
  "comment": "Standard PPPoE pool"
}
```

### Create
Behavior:
- reject empty or whitespace-only `name`
- reject duplicate pool name on the same router
- send RouterOS add command for `/ip/pool`
- after router success, sync router IP pools into PostgreSQL
- return the created pool row from the mirrored dataset

### Update
Behavior:
- load the mirrored row first to obtain the current router-scoped pool identity
- use the current RouterOS pool name to locate the router record being edited
- treat `name` as immutable in phase one
- reject update requests that attempt to change `name`
- apply only `ranges`, `next_pool`, and `comment`
- if the mirrored row exists locally but RouterOS lookup by the stored current name fails, return a conflict-style error and require `Sync from router` before retrying
- after router success, sync router IP pools into PostgreSQL
- return the updated pool row from the mirrored dataset

### Delete
Behavior:
- load the mirrored row first
- run dependency lookup before touching RouterOS
- re-run dependency lookup immediately before the RouterOS delete
- never hard-block delete solely because dependencies exist in phase one
- if dependencies exist, include them in the response so the UI can surface that the delete happened with warnings
- after router success, sync router IP pools into PostgreSQL
- return:
  - `ok`
  - `deleted_pool_id`
  - `deleted_pool_name`
  - `router_id`
  - `warnings`

## Validation Rules
### Required
- `name`

### Optional
- `ranges`
- `next_pool`
- `comment`

### Validation behavior
- `name` must be trimmed and non-empty
- `name` must be unique per router
- `ranges` stays loosely validated as a RouterOS-compatible string in phase one
- `next_pool` stays loosely validated as a RouterOS pool name string in phase one
- blank optional strings should be normalized to `null`

## Error Handling
- Router write failures should return actionable errors whenever available.
- Mirror sync failures after router success should return a dedicated sync error.
- If the local row ID no longer exists in PostgreSQL for the selected tenant/router, return `not found`.
- If the row exists locally but the RouterOS pool no longer exists, return a conflict-like error and suggest sync.

## Frontend Design
- Keep the existing page shell, router selector, list table, and sync action.
- Add an `Add pool` button near the existing actions.
- Add row-level action controls for `Edit` and `Delete`.
- Use a dedicated reusable IP pool form dialog component.
- Make it explicit in the UI that changes are applied directly to the router.
- Distinguish:
  - router operation failure
  - delete with dependency warnings
  - successful router write but failed local refresh

## Testing Strategy
### Backend tests
- create rejects blank name
- create rejects duplicate name on the same router
- update can change non-name standard fields for an existing pool
- update rejects any attempt to rename the pool in phase one
- delete returns warning metadata when PPPoE accounts depend on the pool
- delete returns warning metadata when package mappings depend on the pool
- successful create/update/delete triggers mirror refresh behavior
- authorization rules are enforced for read vs manage actions

### Frontend tests
- add button disabled or blocked until a router is selected
- create/edit dialog sends normalized payload
- delete dialog shows dependency warning counts and still allows confirmation
- successful CRUD reloads table state
- sync-error messaging is distinct from router-write failure messaging

## Acceptance Criteria
- Tenant admin can create an IP pool with the phase-one fields from `/[tenant]/admin/network/ip-pools`.
- Tenant admin can edit `ranges`, `next_pool`, and `comment` for an existing pool.
- Tenant admin cannot rename an existing IP pool in phase one.
- Tenant admin can delete a pool even when internal dependencies exist, but the UI warns clearly before confirmation.
- Every successful create/update/delete applies to RouterOS first and then refreshes the PostgreSQL mirror.
- The page continues to support manual `Sync from router`.
