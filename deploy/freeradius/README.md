# Managed RADIUS Deployment Notes

This project now supports PPPoE accounts with `account_source = managed_radius`.

## Recommended Topology

- Billing app stack stays on its existing PostgreSQL database.
- Managed RADIUS runs as a separate Docker Compose stack using:
  - `freeradius`
  - `radius-postgres`
- Billing provisions tenant-aware records into the managed RADIUS PostgreSQL database.

## What This Stack Does

- Builds a local FreeRADIUS image from the official upstream image.
- Configures a minimal PAP/PPPoE-oriented `default` virtual server.
- Uses PostgreSQL as the source for:
  - PPP credentials / reply attributes
  - NAS/client definitions via dynamic client lookup at request time
- Lets the billing app render a copy-paste MikroTik CLI snippet when `MANAGED_RADIUS_HOST` is set in the app environment.

## Bring Up the Stack

```bash
docker compose -f docker-compose.radius.yml up -d
```

Useful checks:

```bash
docker compose -f docker-compose.radius.yml ps
docker compose -f docker-compose.radius.yml logs -f freeradius
docker compose -f docker-compose.radius.yml logs -f radius-postgres
bash scripts/test-radius-stack.sh
```

## Required Billing-Side Setup

- Create one `managed_radius_servers` row per tenant environment.
- Create one `managed_radius_nas` row per router that should authenticate through managed RADIUS.
- Store:
  - RADIUS PostgreSQL password encrypted with the app's `managed_radius_db` purpose
  - NAS shared secret encrypted with the app's `managed_radius_shared_secret` purpose

The current implementation wires billing-side provisioning first. Admin CRUD screens for those records are still a follow-up step, so initial setup may be done via SQL or seed tooling.

## FreeRADIUS Query Model

- The FreeRADIUS image reads SQL configuration from:
  - `deploy/freeradius/raddb/mods-available/sql.template`
  - `deploy/freeradius/raddb/sites-enabled/default`
  - `deploy/freeradius/raddb/sites-available/dynamic-clients`
- Query behavior is aligned to the tenant-aware tables created in `radius-postgres`.
- Tenant isolation relies on NAS lookup first, then PPP username lookup.
- Usernames do not need to be globally unique across tenants.
- Active NAS clients are resolved from PostgreSQL at request time through FreeRADIUS dynamic clients.

## Router Expectations

- One MikroTik router/NAS should map to one unique shared secret.
- Router IP / CIDR in `managed_radius_nas.nas_ip_or_cidr` must match the request source seen by FreeRADIUS.
- Configure MikroTik PPP authentication to use the shared FreeRADIUS server and the router-specific secret.

## Runtime Behavior

Managed RADIUS account changes continue to apply directly from PostgreSQL during auth.

Managed RADIUS NAS mapping changes also apply without restarting the container because FreeRADIUS now resolves NAS clients dynamically from PostgreSQL. Dynamic client entries are cached briefly by FreeRADIUS, so source IP or shared-secret changes should converge within a few seconds instead of requiring a container restart.

## Optional Fallback Restart Hook

If you still want `/superadmin/radius/mappings` edits to force a FreeRADIUS restart as an operational fallback, configure the API server environment:

```bash
MANAGED_RADIUS_RESTART_COMMAND="/opt/isp-management/scripts/restart-freeradius.sh"
MANAGED_RADIUS_RESTART_WORKDIR="/opt/isp-management"
MANAGED_RADIUS_COMPOSE_FILE="docker-compose.radius.yml"
MANAGED_RADIUS_SERVICE_NAME="freeradius"
```

Operational requirement:

- The API server process user must be allowed to execute the restart command successfully.
- In the provided systemd unit, that means the `ispmanagement` user needs access to Docker or a tightly-scoped wrapper/sudoers rule.
- The provided wrapper script lives at `scripts/restart-freeradius.sh`.
