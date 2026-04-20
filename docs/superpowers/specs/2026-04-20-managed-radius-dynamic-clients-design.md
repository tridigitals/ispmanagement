# Managed RADIUS Dynamic Clients Design

## Goal

Make FreeRADIUS resolve NAS clients dynamically from the managed RADIUS PostgreSQL database so changes to `managed_radius_nas` apply without restarting the `freeradius` container.

## Problem

The current deployment uses `read_clients = yes` with `client_query` in `sql.template`. That loads the NAS client list during FreeRADIUS startup. Database edits to:

- `nas_ip_or_cidr`
- `shortname`
- `nas_name`
- `shared_secret`
- `is_active`

do not fully affect the running FreeRADIUS process until the container is restarted.

User credential changes already work dynamically because the auth queries are executed at request time. NAS client changes do not.

## Current Constraints

- PPP authentication is PAP-oriented and uses the `default` virtual server.
- Tenant isolation currently depends on joining `managed_radius_accounts` to `managed_radius_nas` and matching `COALESCE(n.shortname, n.nas_name) = '%{client:shortname}'`.
- `require_message_authenticator = yes` should remain enforced for runtime clients.
- The deployment should stay database-driven; we do not want static NAS client files or restart hooks as the primary mechanism.

## Proposed Architecture

### 1. Replace startup SQL clients with runtime dynamic clients

- Stop relying on `read_clients = yes` and `client_query`.
- Enable FreeRADIUS dynamic client resolution so unknown packet source IPs are looked up against `managed_radius_nas` during request handling.
- Resolve the client by `Packet-Src-IP-Address` against active NAS rows in `managed_radius_nas`.

### 2. Build the runtime client from the NAS row

For a matching active row, populate runtime client properties from the database:

- `FreeRADIUS-Client-IP-Address`
- `FreeRADIUS-Client-Secret`
- `FreeRADIUS-Client-Shortname`
- `FreeRADIUS-Client-NAS-Type = other`
- `FreeRADIUS-Client-Virtual-Server = default`
- `FreeRADIUS-Client-Require-MA = yes`

This keeps BlastRADIUS protection in place while allowing DB-driven client updates to apply without restart.

### 3. Keep tenant-aware auth lookup tied to the resolved client

- Continue using SQL for `authorize_check_query` and `authorize_reply_query`.
- Keep tenant scoping by joining to `managed_radius_nas`.
- Continue matching the auth row to the resolved runtime client via `%{client:shortname}`.

That preserves the existing behavior where the same username can exist in different tenants, as long as the packet source maps to the correct NAS.

### 4. De-scope restart hooks from the primary path

- Dynamic clients make restart unnecessary for normal NAS mapping edits.
- The earlier restart hook can remain optional as a compatibility fallback, but the new target behavior is “DB edit is enough”.

## FreeRADIUS Configuration Changes

Expected config direction:

- add a `sites-enabled/dynamic-clients` virtual server in repo
- disable `read_clients = yes`
- remove dependency on startup `client_query`
- configure `dynamic_clients` lookup against PostgreSQL using unlang + SQL/xlat

## Operational Behavior

After this change:

- editing a NAS mapping in `/superadmin/radius/mappings` updates the DB
- the next Access-Request from the new source IP is resolved against the DB
- no FreeRADIUS restart is required for NAS IP/secret changes

## Risks

### Dynamic client cache lifetime

If FreeRADIUS caches dynamic clients indefinitely, DB changes may still appear delayed. The dynamic client lifetime must be kept short enough that changes converge quickly without restart.

### Overly broad dynamic matching

The dynamic client lookup must only create a client when an active `managed_radius_nas` row matches the packet source IP. Unknown sources must stay rejected.

### Tenant isolation regression

Auth queries must continue scoping by the resolved client identity, not only by username.

## Testing Strategy

- Add regression tests for FreeRADIUS repo config:
  - `read_clients = no`
  - `dynamic-clients` site exists and is enabled
  - dynamic client config requires Message-Authenticator
- Keep existing auth query expectations intact
- Rebuild the FreeRADIUS image and verify:
  - container starts healthy
  - `freeradius -XC` shows the dynamic clients site loaded
  - an auth request from an existing NAS still returns `Access-Accept`
  - changing NAS IP in DB no longer requires a restart to authenticate from the new source
