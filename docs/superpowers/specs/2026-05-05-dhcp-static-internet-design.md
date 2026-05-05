# DHCP Static Internet Design (2026-05-05)

## Background / Current State
- The product already supports tenant-managed `PPPoE` internet provisioning end to end:
  - backend model, service, HTTP, and Tauri command support
  - tenant admin page at `/[tenant]/admin/network/pppoe`
  - installation-scoped provisioning rules in `pppoe_service`
  - customer detail exposure through a dedicated PPPoE tab
- Internet ordering already starts from the shared package catalog at `/[tenant]/dashboard/services/order/internet`.
- ISP packages currently default `service_type` to `internet_pppoe`, which hard-codes PPPoE as the only internet provisioning path.
- Installation and subscription flows already treat internet activation as a work-order-driven lifecycle, so the new access method should plug into the same package, subscription, and installation structure instead of bypassing it.

## Goals / Non-goals
### Goals
- Add a second internet provisioning path for MikroTik `DHCP static lease`.
- Keep DHCP static under the internet service family while separating it from PPPoE operationally.
- Preserve the business model:
  - internet packages are still the sellable product
  - subscriptions still drive activation and billing
  - `1 subscription = 1 access service record`
- Support deterministic provisioning with required:
  - `router`
  - `dhcp_server`
  - `mac_address`
  - `ip_address`
- Support optional automatic `simple queue` provisioning per lease using package-derived bandwidth settings.
- Integrate DHCP static into installation and customer management flows without overloading PPPoE forms.

### Non-goals
- No generic “all access methods in one table” refactor in phase one.
- No dynamic DHCP reservation mode where IP is allocated from a pool automatically.
- No hotspot, bridge, VLAN, ONU, or other non-PPPoE/non-DHCP access methods in this phase.
- No public self-service collection of MAC/IP details during customer ordering.
- No advanced queue tree, burst, parent queue, or firewall-mark orchestration in phase one.

## Product Decision
DHCP static will be implemented as a separate access-service module that sits alongside PPPoE inside the internet domain.

That means:
- package catalog remains shared
- provisioning method is selected by package metadata
- tenant admin gets a dedicated DHCP static page
- customer detail gets a dedicated DHCP static tab
- installation forms branch by provisioning type instead of mixing PPPoE and DHCP fields together

Recommended information architecture:
- `Admin > Network > Internet > PPPoE`
- `Admin > Network > Internet > DHCP Static`

Customer detail:
- keep `PPPoE` as its own tab
- add `DHCP Static` as a sibling tab

## Package and Subscription Model
### Package structure
Internet packages should stop relying on `service_type = internet_pppoe` as the provisioning discriminator.

Recommended phase-one package contract:
- keep `service_type` at the business level, still representing an internet product family
- add a new `provisioning_type` enum for technical activation:
  - `pppoe`
  - `dhcp_static`

Recommended default behavior:
- existing records without an explicit value should resolve to `pppoe`
- new DHCP-based internet packages should set `provisioning_type = dhcp_static`

### Visibility model
Package visibility should remain a package concern, not a provisioning-module concern.

That means:
- public orderability continues to use the package’s visibility rules
- internal-only DHCP packages can stay hidden from customer ordering
- public DHCP packages remain possible later without changing the DHCP module itself

### Subscription rule
One subscription owns one active provisioning record.

For DHCP static in phase one:
- one subscription may have at most one DHCP static service record
- one DHCP static record belongs to exactly one subscription
- if a customer needs another router/CPE, they must place another subscription/order

## Domain Model
### New DHCP static service record
Create a dedicated tenant-scoped model and storage table for DHCP static services.

Recommended core fields:
- `id`
- `tenant_id`
- `subscription_id`
- `router_id`
- `customer_id`
- `location_id`
- `package_id`
- `dhcp_server_name`
- `mac_address`
- `ip_address`
- `comment`
- `disabled`
- `lease_present`
- `lease_router_ref` or equivalent cached RouterOS identity handle when useful
- `lease_last_sync_at`
- `lease_last_error`
- `queue_mode`
- `queue_name`
- `queue_target`
- `queue_present`
- `queue_last_sync_at`
- `queue_last_error`
- `created_at`
- `updated_at`

### Queue mode
Phase-one queue mode can stay intentionally small:
- `none`
- `simple_queue_from_package`

`simple_queue_from_package` means:
- the queue exists because the service package says this access line should be rate-limited
- queue bandwidth values are derived from package or package-router mapping data
- technician/admin does not type arbitrary queue rates during installation

### Suggested uniqueness rules
- unique active record per `subscription_id`
- unique per router for `mac_address`
- unique per router for `ip_address`

Phase-one normalization:
- trim and uppercase/canonicalize `mac_address`
- trim `ip_address`
- blank strings become `null` for optional fields only

## Operational Model
### Source of truth
DHCP static service records are owned by the application database.

RouterOS is the provisioning target for:
- static lease creation/update
- optional simple queue creation/update

This matches the PPPoE module’s operator mental model:
- save service data locally
- apply to router deliberately
- track presence and sync failures explicitly

### Draft-first workflow
Recommended default:
- save the DHCP static record to PostgreSQL first
- mark it as not yet applied
- expose a manual `Apply` action

Optional later enhancement:
- tenant setting similar to PPPoE auto-apply can enable apply-on-save

### Why draft-first
- avoids accidental writes to the wrong router during data entry
- fits the current PPPoE operating pattern
- makes installation and admin correction workflows safer

## RouterOS Provisioning Design
### Static lease resource
Each DHCP static record should provision one RouterOS `/ip dhcp-server lease` static entry with:
- `server = dhcp_server_name`
- `mac-address = mac_address`
- `address = ip_address`
- `comment = comment`
- `disabled = disabled`

Behavior:
- create if missing
- update if already mapped to the local service record
- detect conflicts if another router lease already owns the same IP or MAC

### Simple queue resource
When `queue_mode = simple_queue_from_package`, provision one RouterOS `/queue simple` entry:
- `name = deterministic queue name`
- `target = ip_address/32`
- `max-limit` derived from package/package-router mapping
- `comment` includes enough service identity for troubleshooting

Recommended deterministic queue naming:
- based on service type plus customer/subscription identity
- stable across re-apply
- not dependent on mutable free-text comments

Example shape:
- `dhcp-{subscription_or_service_key}`

### Lease and queue are separate sync surfaces
Track lease sync and queue sync independently.

Reason:
- lease may succeed while queue fails
- queue may be manually changed or deleted without affecting the lease
- operational troubleshooting is much clearer when status is not collapsed into one boolean

### Reconcile behavior
Provide a router-scoped reconcile flow similar to PPPoE.

Recommended scope:
- compare local DHCP static records against RouterOS lease entries for the selected router
- refresh `lease_present`, `lease_last_sync_at`, and `lease_last_error`
- if queue mode is enabled, also refresh queue status fields

Phase-one reconcile does not need to auto-import unknown router leases into billing/subscription data.

## Installation and Work Order Integration
### Installation branching
Installation should branch by package `provisioning_type`.

If package provisioning type is:
- `pppoe`: keep the current PPPoE creation/apply flow
- `dhcp_static`: render a dedicated DHCP static installation form

This is intentionally not a single mixed “internet credential” form.

### DHCP installation form fields
Recommended installation form fields:
- `router`
- `dhcp_server`
- `mac_address`
- `ip_address`
- `comment`
- `queue_mode`

Form behavior:
- `mac_address` required
- `ip_address` required
- `dhcp_server` required
- package and subscription context should already be known from the work order

### Work order linkage
The DHCP service create/apply flow should accept `work_order_id` where the current PPPoE flow already does, so installation-scoped permissions stay enforceable.

Recommended rule:
- technicians with installation-manage scope may create/apply DHCP static only for the assigned work-order context
- the router, customer, location, and subscription must match the work-order scope or package-router mapping rules

### Installation completion
Completing the installation work order should not invent a second activation path.

The work-order completion flow should:
- validate that the required provisioning record exists for the subscription
- confirm that its required router apply step has succeeded, or surface a clear warning/block depending on the current installation policy

Phase-one recommendation:
- require the DHCP record to exist before installation completion
- strongly prefer requiring successful lease apply before completion
- queue failure can remain a visible warning if the business wants install completion to proceed when connectivity is already live

## Admin UX Design
### New page
Add a dedicated page for DHCP static services under the internet/network cluster.

Recommended route:
- `/[tenant]/admin/network/dhcp-static`

If the sidebar/navigation is upgraded to an `Internet` subgroup, both PPPoE and DHCP static should move under that same cluster together.

### Page content
Follow the PPPoE page pattern closely enough to feel familiar.

Recommended summary cards:
- total
- provisioned lease
- missing lease
- disabled
- queue issues

Recommended filters:
- search
- router
- DHCP server
- lease sync status
- queue sync status
- disabled/enabled

Recommended table columns:
- customer
- subscription/package
- router
- DHCP server
- MAC
- IP
- lease status
- queue status
- disabled
- updated at

Recommended row actions:
- `Edit`
- `Apply`
- `Reconcile`
- `Delete`
- `Open customer`
- `Open billing/subscription`

### Form behavior
Create/edit form should expose:
- customer
- location
- subscription
- package
- router
- DHCP server
- MAC address
- IP address
- comment
- disabled
- queue mode

Phase-one form should not ask the operator to key in manual queue limits when queue mode uses package-driven values.

## Customer Detail UX
Add a dedicated `DHCP Static` tab in customer detail.

Phase-one tab content can be lighter than the admin page but should still show:
- package/subscription
- router
- DHCP server
- MAC
- IP
- lease status
- queue status
- latest sync/error state

Keep `PPPoE` and `DHCP Static` separate tabs.

Reason:
- each access type has different identifiers and actions
- operators read them faster when not mixed

## Ordering UX
The public/customer order page can remain package-driven.

Phase-one recommendation:
- do not collect MAC/IP at customer order time
- create the subscription and installation work order as usual
- collect DHCP-specific technical details during admin/technician installation handling

Reason:
- MAC/IP is operational installation data
- customer-entered values would be unreliable and hard to validate against onsite reality

## API and Command Design
Recommended API shape mirroring PPPoE:
- `GET /admin/dhcp-static/services`
- `GET /admin/dhcp-static/services/:id`
- `POST /admin/dhcp-static/services`
- `PUT /admin/dhcp-static/services/:id`
- `DELETE /admin/dhcp-static/services/:id`
- `POST /admin/dhcp-static/services/:id/apply`
- `POST /admin/dhcp-static/routers/:routerId/reconcile`

The Tauri command layer should mirror those operations the same way the PPPoE module does.

Recommended DTO concerns:
- create/update request with required router, DHCP server, MAC, and IP
- apply request optionally accepting `work_order_id`
- list filters for router, DHCP server, search text, lease status, queue status, and disabled state

## Authorization Design
Recommended phase-one permission model:
- new resource key: `dhcp_static`
- `read`
- `manage`

Installation-scoped exceptions should mirror PPPoE behavior:
- users with installation-manage authority may operate only inside the assigned work-order scope

This keeps DHCP static operationally separate from PPPoE while preserving the same field-technician workflow model.

## Validation Rules
### Required
- `subscription_id`
- `router_id`
- `customer_id`
- `location_id`
- `package_id`
- `dhcp_server_name`
- `mac_address`
- `ip_address`

### Validation behavior
- reject malformed MAC address
- reject malformed IP address
- reject duplicate MAC on the same router for another active service
- reject duplicate IP on the same router for another active service
- reject service creation when the subscription already has another active DHCP static record
- reject mismatch between subscription/package and requested provisioning type

### Package validation
- the selected package must resolve to `provisioning_type = dhcp_static`
- PPPoE packages must not create DHCP static records

## Error Handling
- RouterOS lease conflict should return a clear MAC/IP conflict error.
- RouterOS queue failure should populate queue-specific error fields.
- If lease apply succeeds but queue apply fails, return a partial-success response that the UI can explain.
- If the local record exists but RouterOS identity can no longer be found, reconcile should surface that as drift instead of silently recreating resources unexpectedly.

## Testing Strategy
### Backend tests
- create rejects malformed MAC address
- create rejects malformed IP address
- create rejects subscription/package when provisioning type is not `dhcp_static`
- create rejects duplicate active record for the same subscription
- apply creates lease without queue when queue mode is `none`
- apply creates or updates both lease and queue when queue mode is `simple_queue_from_package`
- apply returns partial success when lease succeeds and queue fails
- installation-scope access allows assigned technician within matching work-order scope
- installation-scope access rejects unrelated customer/router/subscription context

### Frontend tests
- package selection branches installation form by provisioning type
- DHCP static admin page loads its deferred modal/module correctly
- customer detail shows separate DHCP static tab when permission is available
- admin list filters and row status rendering distinguish lease issues from queue issues

### Integration tests
- customer order for DHCP package still creates a normal subscription + installation work order
- technician can complete installation after creating/applying the DHCP static record
- customer detail, subscription context, and admin list all reflect the same DHCP service record

## Rollout Recommendation
Implement in slices:
1. Package model support for `provisioning_type`
2. DHCP static backend model/service/API/commands
3. Admin DHCP static page
4. Customer detail tab
5. Installation/work-order branching
6. Optional auto-apply setting after the manual apply baseline is stable

This sequence minimizes risk because package semantics land first, the new provisioning domain stays isolated, and installation flow changes happen after the standalone DHCP static module is already working.
