# PPP Profile CRUD Design (2026-04-02)

## Background / Current State
- Tenant admin already has a dedicated PPP Profile page at `/[tenant]/admin/network/ppp-profiles`.
- The current page is read-oriented: it can list PPP profiles per router and sync them from RouterOS into PostgreSQL.
- Backend support currently exists only for:
  - listing PPP profiles for a router
  - syncing PPP profiles from a router into `mikrotik_ppp_profiles`
- The `mikrotik_ppp_profiles` table already stores a tenant-scoped, router-scoped snapshot of RouterOS profile data.
- Other tenant features already depend on profile names, especially:
  - PPPoE accounts via `router_profile_name`
  - ISP package router mappings via `router_profile_name`
  - installation and customer flows that surface or reuse those mappings

## Goals / Non-goals
### Goals
- Add full tenant-admin CRUD for PPP profiles from `/[tenant]/admin/network/ppp-profiles`.
- Make RouterOS the source of truth for create, update, and delete.
- Keep PostgreSQL as a synchronized mirror for fast reads, filtering, dependency checks, and cross-module references.
- Support only the standard, low-risk profile fields in phase one:
  - `name`
  - `local_address`
  - `remote_address`
  - `rate_limit`
  - `dns_server`
  - `comment`
- Prevent destructive deletes when the profile is still referenced by internal tenant records.
- Reuse existing MikroTik router permissions and page patterns already present in network admin.

### Non-goals
- No PPP profile rename behavior in phase one.
- No advanced RouterOS PPP profile fields in phase one:
  - `only_one`
  - `change_tcp_mss`
  - `use_compression`
  - `use_encryption`
  - `use_ipv6`
  - `bridge`
- No bulk import/export workflow changes.
- No cross-router profile copy or template system.
- No background reconciliation daemon beyond the existing manual sync behavior.
- No migration of dependent modules from `router_profile_name` string references to profile foreign keys in this phase.

## Product Decision
PPP profile CRUD will be a `router-first` workflow.

That means:
- create writes to RouterOS first
- update writes to RouterOS first
- delete removes from RouterOS first, if dependencies allow it
- PostgreSQL is updated only as a mirror after the RouterOS operation succeeds

This preserves one operational truth source while keeping the app fast and relationally useful.

## Authority and Data Model
### Source of truth
- RouterOS is authoritative.
- `mikrotik_ppp_profiles` is a cache/snapshot table, not the canonical configuration owner.

### Why still store rows in PostgreSQL
- the page can list quickly without reconnecting to a router each time
- the app can search, filter, and show sync metadata
- delete safeguards can check dependencies before touching RouterOS
- other modules can continue using profile-name-based relationships

### Consistency rule
- Never report a CRUD operation as successful unless the RouterOS write succeeded.
- After a successful RouterOS write, refresh the PostgreSQL mirror immediately.
- If the RouterOS write succeeds but mirror refresh fails, return a dedicated sync error so the UI can warn the user that RouterOS changed but the local cache still needs refresh.

## Route and UX Design
Keep the existing route:
- `/[tenant]/admin/network/ppp-profiles`

Keep the existing router filter as the main working context:
- every CRUD action is scoped to the currently selected router
- no create/edit/delete is allowed unless a router is selected

### Page actions
- `Refresh` reloads the current router's mirrored PPP profile list from PostgreSQL only; it must not contact RouterOS
- `Sync from router`
- `Add profile`

### Row actions
- `Edit`
- `Delete`

### Form shape
Use a modal or dialog form with standard-safe fields:
- `name` (required)
- `local_address`
- `remote_address`
- `rate_limit`
- `dns_server`
- `comment`

Edit-mode behavior:
- show `name` as read-only so the current profile identity remains visible
- do not allow editing the `name` field in phase one
- edit submissions should send only mutable fields plus the row ID/router context

The page should continue showing:
- profile name
- local address
- remote address
- rate limit
- DNS
- router presence state
- last synced time

### Delete UX
- User clicks `Delete`.
- UI first requests dependency information for the selected profile.
- If dependencies exist, the confirmation action is disabled and the dialog explains what is still using the profile.
- If no dependencies exist, the dialog requires confirmation and then performs router-first delete.
- Backend must re-run dependency validation immediately before the RouterOS delete so the safeguard is not only based on the preflight response.

## API Design
Extend the existing MikroTik admin surface with PPP profile CRUD endpoints.

Recommended HTTP routes:
- `GET /admin/mikrotik/routers/:routerId/ppp-profiles`
- `POST /admin/mikrotik/routers/:routerId/ppp-profiles`
- `PUT /admin/mikrotik/routers/:routerId/ppp-profiles/:id`
- `DELETE /admin/mikrotik/routers/:routerId/ppp-profiles/:id`
- `GET /admin/mikrotik/routers/:routerId/ppp-profiles/:id/dependencies`
- `POST /admin/mikrotik/routers/:routerId/ppp-profiles/sync`

The Tauri command layer should mirror the same operations.
Tauri commands must translate service failures into the same logical error envelope used by HTTP consumers:
- `code`
- `message`
- `details` (optional)

Route identity contract:
- `:routerId` always identifies the selected router context
- `:id` always means the local PostgreSQL mirror row ID from `mikrotik_ppp_profiles.id`
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
- return the final mirrored profile row or dependency payload

### MikroTik service PPP profile CRUD unit
Responsibility:
- resolve router ownership within tenant scope
- connect to RouterOS
- execute RouterOS create/update/delete commands
- synchronize PostgreSQL mirror after successful router writes
- enforce dependency blocking before delete

This should stay separate from page/UI concerns and separate from generic sync-only behavior.

### Mirror refresh strategy
Phase-one implementation must use one path only:
- run router-scoped PPP profile sync after every successful create/update/delete

Do not add a separate single-profile refresh helper in phase one.

This keeps the implementation simpler, avoids diverging refresh code paths, and matches the existing sync-first service pattern already in the codebase.

### Dependency query unit
Responsibility:
- return whether a PPP profile name is referenced by tenant records on the same router
- produce human-readable counts for blocking UI

Initial dependency sources:
- `pppoe_accounts` where `router_id` and `router_profile_name` match
- `isp_package_router_mappings` where `router_id` and `router_profile_name` match

Dependency coverage for phase one is considered complete when those two sources are checked, because the current tenant flows that indirectly depend on profile names do so through one of these tables:
- direct PPPoE account assignment uses `pppoe_accounts.router_profile_name`
- package-driven provisioning and installation/customer flows derive profile usage from `isp_package_router_mappings.router_profile_name`

## RouterOS Operation Rules
### Shared request payload for create and update
```json
{
  "name": "Basic-10M",
  "local_address": "10.10.10.1",
  "remote_address": "pool-basic-10m",
  "rate_limit": "10M/10M",
  "dns_server": "1.1.1.1",
  "comment": "Standard residential package"
}
```

Shared response payload for successful create and update:
- return the refreshed mirrored row from `mikrotik_ppp_profiles`

### Create
Input:
- selected router ID
- standard field payload

Behavior:
- reject empty or whitespace-only `name`
- reject duplicate profile name on the same router
- send RouterOS add command for `/ppp/profile`
- after router success, sync router PPP profiles into PostgreSQL
- return the created profile row from the mirrored dataset

### Update
Input:
- selected router ID
- local mirrored row ID
- standard field payload

Behavior:
- load the mirrored row first to obtain the current router-scoped profile identity
- use the current RouterOS profile name to locate the router record being edited
- treat `name` as immutable in phase one
- reject update requests that attempt to change `name`
- apply only the supported standard fields in phase one
- if the mirrored row exists locally but RouterOS lookup by the stored current name fails, return a conflict-style error and require `Sync from router` before retrying
- after router success, sync router PPP profiles into PostgreSQL
- return the updated profile row from the mirrored dataset

### Delete
Input:
- selected router ID
- local mirrored row ID

Behavior:
- load the mirrored row first
- run dependency lookup before touching RouterOS
- re-run dependency lookup immediately before the RouterOS delete
- if dependencies exist, return a blocking validation error with dependency counts/details
- if no dependencies exist, delete the RouterOS profile
- after router success, sync router PPP profiles into PostgreSQL
- return a delete result payload:
  - `ok`
  - `deleted_profile_id`
  - `deleted_profile_name`
  - `router_id`

### Dependency lookup response
Return:
- `profile_id`
- `profile_name`
- `router_id`
- `can_delete`
- `dependencies`

### Standard error contract
Use a stable error envelope for all CRUD endpoints:
- `code`
- `message`
- `details` (optional)

Initial error codes:
- `validation_error`
- `not_found`
- `dependency_blocked`
- `rename_not_allowed`
- `router_conflict`
- `router_write_failed`
- `mirror_sync_failed`

Examples:
- `dependency_blocked`: delete attempted while counts exist in `pppoe_accounts` or `isp_package_router_mappings`
- `router_conflict`: mirrored row exists locally but the RouterOS profile cannot be found by the last-synced name
- `mirror_sync_failed`: RouterOS mutation succeeded but the mirror refresh failed immediately after
- `validation_error`: malformed or disallowed input such as blank `name` on create or attempted rename on update
- `router_write_failed`: RouterOS rejected an otherwise valid request or the router operation could not be completed

## Validation Rules
### Required
- `name`

### Optional
- `local_address`
- `remote_address`
- `rate_limit`
- `dns_server`
- `comment`

### Validation behavior
- `name` must be trimmed and non-empty
- `name` must be unique per router
- `local_address` should accept empty or a single IPv4/IPv6 host value as plain text
- `dns_server` should accept empty, a single IPv4/IPv6 host value, or a comma-separated list of host values
- `remote_address` stays loosely validated because it can be either an address value or a RouterOS pool name
- `rate_limit` remains a freeform RouterOS-compatible string in phase one
- blank optional strings should be normalized to `null`

Validation examples:
- valid `local_address`: `10.10.10.1`
- valid `local_address`: `2001:db8::1`
- invalid `local_address`: `10.10.10.1,10.10.10.2`
- valid `dns_server`: `1.1.1.1`
- valid `dns_server`: `1.1.1.1,8.8.8.8`
- valid `dns_server`: `2001:4860:4860::8888`
- invalid `dns_server`: `pool-basic`

## Dependency Check Contract
Recommended response shape:
- `profile_id`
- `profile_name`
- `router_id`
- `can_delete`
- `dependencies`

Each dependency item should include:
- `type`
- `label`
- `count`

Initial dependency types:
- `pppoe_accounts`
- `isp_package_router_mappings`

Example behavior:
- `can_delete = false` when any count is greater than zero
- UI shows the dependency list in the delete dialog and blocks the destructive action

## Error Handling
### Router write failures
- Return actionable errors from RouterOS operations whenever available.
- Do not mutate the local mirror as if the action succeeded.

### Mirror sync failures after router success
- Return a dedicated error message explaining that RouterOS changed but local data refresh failed.
- UI should show a warning toast and keep `Sync from router` available as the recovery action.

### Stale row targeting
- If the local row ID no longer exists in PostgreSQL for the selected tenant/router, return `not found`.
- If the row exists locally but the RouterOS profile no longer exists, return a conflict-like error and suggest sync.
- If the row exists locally but the profile was renamed externally on the router after the last sync, treat it the same as a missing RouterOS target: reject the update/delete attempt and require sync before retry.

### Dependency violations
- Return a structured validation error rather than a generic failure.
- UI should surface counts and labels instead of only saying delete failed.

## Frontend Design
### Page structure
- Keep the existing page shell, router selector, list table, and sync action.
- Add an `Add profile` button near the existing actions.
- Add row-level action controls for `Edit` and `Delete`.

### State handling
- Keep router selection as top-level page state.
- Keep table load and sync states separate from form submit state if possible.
- After successful create/update/delete, refresh the table from the API response rather than mutating rows optimistically.

### Form component
- Prefer a dedicated reusable PPP profile form dialog component instead of growing the page file with all form logic inline.
- Use a single form component for both create and edit modes.
- Centralize payload normalization so blank strings become `null` consistently.

### Copy and messaging
- Make it explicit in the UI that changes are applied directly to the router.
- Distinguish:
  - router operation failure
  - blocked delete because of dependencies
  - successful router write but failed local refresh

## Testing Strategy
### Backend tests
- create rejects blank name
- create rejects duplicate name on the same router
- update can change non-name standard fields for an existing profile
- update rejects any attempt to rename the profile in phase one
- delete is blocked when PPPoE accounts depend on the profile
- delete is blocked when package mappings depend on the profile
- successful create/update/delete triggers mirror refresh behavior
- authorization rules are enforced for read vs manage actions

### Frontend tests
- add button disabled or blocked until a router is selected
- create/edit dialog sends normalized payload
- delete dialog shows dependency counts and blocks confirmation when needed
- successful CRUD reloads table state
- sync-error messaging is distinct from router-write failure messaging

## Rollout
1. Add backend dependency query and router-first CRUD service methods.
2. Expose new HTTP routes and Tauri commands.
3. Add frontend API wrappers and types.
4. Add form dialog and row actions on the PPP Profiles page.
5. Verify CRUD against a real router and confirm PostgreSQL mirror behavior.

## Acceptance Criteria
- Tenant admin can create a PPP profile with the standard-safe fields from `/admin/network/ppp-profiles`.
- Tenant admin can edit the same standard-safe fields for an existing profile.
- Tenant admin cannot rename an existing PPP profile in phase one.
- Tenant admin can delete a profile only when no internal dependencies exist.
- Every successful create/update/delete applies to RouterOS first and then refreshes the PostgreSQL mirror.
- The page continues to support manual `Sync from router`.
- Delete failures clearly explain which internal records are blocking the action.
- The implementation does not expose advanced PPP profile fields in phase one.
