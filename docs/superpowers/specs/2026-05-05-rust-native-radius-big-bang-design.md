# Rust Native RADIUS Big-Bang Replacement Design

## Background / Current State
- The current managed RADIUS topology uses the Rust billing app as a control plane and FreeRADIUS as the AAA runtime.
- PPPoE accounts with `account_source = managed_radius` are provisioned by Rust into the managed RADIUS PostgreSQL database.
- FreeRADIUS handles:
  - UDP auth on `1812`
  - UDP accounting on `1813`
  - NAS/shared-secret lookup by source IP/CIDR
  - PAP, CHAP, and MS-CHAP auth routing
  - MikroTik reply attributes such as `Mikrotik-Group`, `Framed-IP-Address`, and `Framed-Pool`
- The current implementation keeps a separate runtime RADIUS database and deployment stack:
  - `freeradius`
  - `radius-postgres`
- The product is still in development, so a breaking architectural cleanup is acceptable if it produces a cleaner long-term system.

## Goals / Non-goals
### Goals
- Remove FreeRADIUS entirely from the project runtime.
- Remove the separate managed RADIUS PostgreSQL runtime store.
- Replace the external RADIUS runtime with a Rust-native RADIUS server implemented inside the existing backend codebase.
- Keep the product focused on MikroTik PPPoE first.
- Preserve the current PPPoE operator workflow as much as possible:
  - create/update PPPoE accounts
  - choose router or managed RADIUS source
  - apply account
  - copy RouterOS RADIUS setup CLI from router detail page
- Keep the new runtime database-driven and tenant-aware.
- Keep a path open for MikroTik hotspot support later.

### Non-goals
- No attempt to reach full FreeRADIUS parity in phase 1.
- No EAP/802.1X feature work in phase 1.
- No LDAP, external identity provider, or generic enterprise AAA integrations.
- No multi-node high-availability cluster design in phase 1.
- No per-tenant independent RADIUS runtimes in phase 1.
- No production-grade multi-region failover design in phase 1.

## Product Decision
Use a big-bang replacement strategy:
- build a Rust-native RADIUS runtime in the existing backend
- migrate managed RADIUS data into the main app database
- delete FreeRADIUS and `radius-postgres` from the target architecture

This is intentionally a clean architectural break, not a transitional hybrid.

To keep the big-bang scope under control, phase 1 should be explicitly limited to:
1. MikroTik PPPoE auth
2. MikroTik PPPoE accounting
3. tenant/router-aware NAS client resolution
4. current managed RADIUS reply attributes
5. internal observability and auditability

Hotspot support is a planned follow-up, not a phase-1 blocker.

## Proposed Architecture

### 1. New bounded context
Add a dedicated `radius_service` bounded context under `src-tauri/src/services/`.

Recommended structure:
- `src-tauri/src/services/radius_service/mod.rs`
- `src-tauri/src/services/radius_service/server.rs`
- `src-tauri/src/services/radius_service/packet.rs`
- `src-tauri/src/services/radius_service/client_registry.rs`
- `src-tauri/src/services/radius_service/auth.rs`
- `src-tauri/src/services/radius_service/accounting.rs`
- `src-tauri/src/services/radius_service/reply.rs`
- `src-tauri/src/services/radius_service/repository.rs`
- `src-tauri/src/services/radius_service/models.rs`
- `src-tauri/src/services/radius_service/config.rs`

Responsibilities:
- `server.rs`
  - own UDP listeners for auth/accounting
  - run async receive/dispatch loop
  - handle runtime startup and shutdown
- `packet.rs`
  - isolate crate-specific packet decode/encode logic
  - normalize request/response data into internal models
- `client_registry.rs`
  - resolve NAS/shared secret from source IP/CIDR
  - enforce active/inactive client rules
- `auth.rs`
  - implement Access-Request auth flow
- `accounting.rs`
  - implement Accounting-Request flow
- `reply.rs`
  - build `Access-Accept`, `Access-Reject`, and `Accounting-Response`
  - map DB-backed account state to MikroTik reply attributes
- `repository.rs`
  - own all DB queries for NAS, account lookup, and accounting/session persistence
- `config.rs`
  - parse runtime configuration, enable flags, ports, and limits

### 2. Runtime model
The backend remains one application, but two protocols:
- HTTP/Axum for product APIs
- UDP RADIUS for MikroTik AAA

The HTTP app should not parse or handle RADIUS packets directly. The RADIUS runtime should be started during backend bootstrap and attached as an application-managed background service.

### 3. Library choice
Use the `radius` crate as the protocol foundation, not as a full application framework.

Why:
- it is a more credible protocol-level foundation than `radius-server`
- it already covers packet parsing, dictionaries, secrets, request validation, and basic server/client primitives
- it still leaves room for application-owned business logic and repository design

Design stance:
- the crate is a transport/protocol primitive
- all AAA policy remains owned by this codebase

## Data Model

### 1. Collapse runtime RADIUS data into the main app database
The target system should not keep a separate `radius-postgres` database.

Recommended target:
- keep managed-RADIUS metadata in the main database
- keep PPPoE account source of truth in the main database
- persist RADIUS accounting/session rows in the main database

### 2. Keep and adapt existing tables
Current app-side tables should remain the source of truth:
- `pppoe_accounts`
- `radius_servers`
- `tenant_radius_assignments`
- `managed_radius_nas`

But their meaning changes:
- `radius_servers`
  - no longer describes an external runtime DB server
  - becomes logical RADIUS endpoint configuration for tenant assignment and operational identity
- `tenant_radius_assignments`
  - remains useful as tenant enablement/gating state
- `managed_radius_nas`
  - remains the NAS registry for router/source IP/CIDR/shared secret mapping

### 3. Remove external runtime account mirror
The external `managed_radius_accounts` runtime mirror should be deleted from the target architecture.

Instead:
- the RADIUS runtime should authenticate directly against `pppoe_accounts`
- account state such as username, password, disabled flag, profile name, remote address, pool, and `radius_identity` should live only once in the main database

### 4. Add accounting/session tables
Add app-owned tables for RADIUS runtime persistence:
- `radius_accounting_sessions`
- `radius_auth_log` or equivalent lightweight auth audit table

`radius_accounting_sessions` should store:
- tenant id
- router id
- NAS identity
- username / radius identity
- acct session id
- status type
- framed ip if present
- calling station id if present
- started at
- last update at
- ended at
- input octets
- output octets
- terminate cause
- raw attribute snapshot if needed for debugging

This table should support:
- current online session visibility
- future customer/session diagnostics
- hotspot compatibility groundwork

## Auth Flow Design

### Access-Request pipeline
1. Receive UDP packet on auth port.
2. Resolve the NAS client from request source IP/CIDR.
3. Load the NAS shared secret.
4. Validate request authenticity.
5. Parse request attributes:
   - `User-Name`
   - `User-Password` when PAP
   - `CHAP-Password`
   - `CHAP-Challenge`
   - `NAS-IP-Address`
   - `NAS-Port`
   - `Service-Type`
   - `Calling-Station-Id`
6. Determine tenant/router context from resolved NAS, not from username alone.
7. Lookup PPPoE account in the main DB:
   - only active mapping
   - only `account_source = managed_radius`
   - router/tenant constrained
8. Check account disabled state and assignment validity.
9. Verify password.
10. Build response:
   - `Access-Accept` on success
   - `Access-Reject` on invalid credentials or inactive state
11. Attach reply attributes:
   - `Mikrotik-Group`
   - `Framed-IP-Address`
   - `Framed-Pool`
12. Persist auth log / observability event.

### Authentication scope in phase 1
- `PAP` is mandatory.
- `CHAP` should be supported in phase 1 if library integration is stable.
- `MS-CHAP` should be designed for, but may be deferred if real MikroTik PPPoE usage does not require it immediately.

Product rule:
- do not let `MS-CHAP` become the reason the whole replacement stalls
- but do not block future implementation with a PAP-only architecture that would need to be rewritten

## Accounting Flow Design

### Accounting-Request pipeline
1. Receive UDP packet on accounting port.
2. Resolve NAS client from source IP/CIDR.
3. Validate packet authenticity with the NAS secret.
4. Parse core attributes:
   - `User-Name`
   - `Acct-Status-Type`
   - `Acct-Session-Id`
   - `NAS-IP-Address`
   - `Framed-IP-Address`
   - `Calling-Station-Id`
   - `Acct-Session-Time`
   - `Acct-Input-Octets`
   - `Acct-Output-Octets`
   - `Acct-Terminate-Cause`
5. Upsert accounting/session record in the main DB.
6. Return `Accounting-Response`.
7. Emit internal event hooks for future dashboards and troubleshooting.

### Accounting scope in phase 1
Support:
- `Start`
- `Stop`
- `Interim-Update`

Nice to have but not phase-1 critical:
- `Accounting-On`
- `Accounting-Off`
- detailed operator-facing live session screens

## Authority Model

### Tenant-facing behavior
Tenant admins continue to:
- manage PPPoE accounts
- choose provisioning target
- apply managed RADIUS accounts
- copy RouterOS RADIUS setup CLI

### Superadmin-facing behavior
Superadmin continues to:
- manage managed RADIUS assignments
- manage NAS mappings
- inspect infrastructure observability

The authority model does not fundamentally change. What changes is the backend runtime owner:
- before: FreeRADIUS runtime
- after: Rust runtime

## Configuration Model
Add dedicated app configuration for the native runtime:
- `RADIUS_ENABLED`
- `RADIUS_BIND_ADDR`
- `RADIUS_AUTH_PORT`
- `RADIUS_ACCT_PORT`
- `RADIUS_WORKER_CONCURRENCY`
- `RADIUS_REQUEST_TIMEOUT_MS`
- `RADIUS_MAX_PACKET_SIZE`
- `RADIUS_REQUIRE_MESSAGE_AUTHENTICATOR`

Recommended defaults:
- enabled only when explicitly configured in development first
- bind `0.0.0.0`
- auth `1812`
- acct `1813`

## Startup / Shutdown Behavior
- Start the RADIUS runtime during backend bootstrap after DB connectivity is ready.
- If the RADIUS runtime cannot bind its ports, backend startup should fail loudly when `RADIUS_ENABLED=true`.
- Shutdown should close both UDP listeners cleanly.
- Runtime failures should be surfaced into logs and, if the app has health endpoints, into platform health indicators.

## Error Handling

### Reject rules
Reject requests when:
- source IP does not match an active NAS mapping
- shared secret validation fails
- packet decode fails
- username is missing
- account is missing for resolved tenant/router
- account is disabled
- password validation fails

### Internal error rules
If internal DB or runtime errors occur:
- do not panic the process for a single request
- log structured error context
- return the correct RADIUS failure behavior for the packet type

Recommended behavior:
- auth path internal failure:
  - prefer reject/deny response over silent success
- accounting path internal failure:
  - prefer logging + best-effort response strategy only if protocol safety allows

### Logging
Structured logs should include:
- request type
- remote source IP
- resolved tenant id
- router id
- username if present
- accept/reject result
- failure reason classification
- latency

Never log:
- plaintext PPPoE passwords
- shared secrets

## Observability
Add internal observability for:
- auth requests per minute
- accounting requests per minute
- accepts
- rejects
- invalid NAS source packets
- secret/authenticator failures
- average request latency
- active sessions count

If the app already has a superadmin health or observability surface, extend it later to show native RADIUS runtime status instead of FreeRADIUS container status.

## Migration Strategy

### Schema migration
1. stop using external runtime `managed_radius_accounts`
2. add accounting/session tables to the main DB
3. ensure `pppoe_accounts` contains every field required for runtime reply building
4. deprecate external DB connection fields once the runtime no longer needs them

### Service migration
1. add `radius_service`
2. refactor `managed_radius_service` so it no longer provisions to external runtime DB
3. make `pppoe_service` apply flow update local source-of-truth only
4. remove FreeRADIUS restart hooks and runtime DB sync logic

### Deployment migration
1. remove `docker-compose.radius.yml` from the target architecture
2. remove `deploy/freeradius/`
3. remove FreeRADIUS operational docs and replace them with native runtime docs

## Testing Strategy

### Unit tests
- NAS CIDR resolution
- PAP password validation
- CHAP validation if implemented in phase 1
- reply attribute construction
- disabled/missing account rejection
- accounting upsert behavior for `Start`, `Stop`, `Interim-Update`

### Integration tests
- startup with runtime enabled
- auth request from known NAS returns `Access-Accept`
- auth request with bad secret is rejected
- auth request with wrong password is rejected
- accounting request persists session rows
- duplicate usernames across tenants stay isolated by NAS resolution

### Compatibility tests
Use `radclient`-style request fixtures or a Rust test harness to validate:
- request authenticator handling
- response encoding
- MikroTik-oriented reply attributes

If possible, add a small manual verification checklist for a real MikroTik PPPoE router:
- add router as NAS
- point router to app RADIUS endpoint
- authenticate valid user
- reject invalid user
- confirm accounting rows appear

## Risks

### Protocol completeness risk
The project may underestimate the work needed for protocol-correct CHAP/MS-CHAP behavior.

Mitigation:
- keep auth methods explicitly phased
- validate against real router traffic early

### Boundary creep risk
RADIUS runtime code may leak into generic PPPoE or HTTP services.

Mitigation:
- keep `radius_service` isolated
- restrict protocol handling to the new bounded context

### Data-model confusion risk
Trying to preserve the external runtime DB mirror after native runtime introduction would create duplicated truth.

Mitigation:
- remove the mirror decisively
- use the main app DB as the only runtime truth

### Operational blind spots
Replacing a mature AAA runtime removes a lot of hidden protocol hardening.

Mitigation:
- build request classification logs and auth/accounting metrics early
- verify behavior against MikroTik before layering more product features

## Recommended Phase Breakdown

### Phase 1
- native auth listener
- NAS resolution by IP/CIDR
- PAP auth
- basic Access-Accept/Reject
- MikroTik reply attributes

### Phase 2
- accounting listener
- accounting persistence
- active session visibility groundwork

### Phase 3
- CHAP and any required advanced auth methods
- cleanup of old managed-RADIUS sync and restart code

### Phase 4
- hotspot-oriented extensions
- optional CoA/Disconnect support if product needs it

## Final Recommendation
Proceed with the big-bang replacement because the product is still in development and the architectural payoff is meaningful.

However, keep the implementation target narrow:
- remove FreeRADIUS entirely
- do not chase full parity before the PPPoE MikroTik path is stable
- make the main database the only source of truth
- treat the `radius` crate as a protocol primitive, not a finished AAA product
