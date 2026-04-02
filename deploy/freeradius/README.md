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
  - NAS/client definitions via `read_clients = yes`
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
- Query behavior is aligned to the tenant-aware tables created in `radius-postgres`.
- Tenant isolation relies on NAS lookup first, then PPP username lookup.
- Usernames do not need to be globally unique across tenants.
- Active NAS clients are read from PostgreSQL during FreeRADIUS startup.

## Router Expectations

- One MikroTik router/NAS should map to one unique shared secret.
- Router IP / CIDR in `managed_radius_nas.nas_ip_or_cidr` must match the request source seen by FreeRADIUS.
- Configure MikroTik PPP authentication to use the shared FreeRADIUS server and the router-specific secret.

## Important Operational Note

Because this setup uses SQL client loading on startup, adding a brand-new router/NAS mapping usually requires a FreeRADIUS restart or container recreate before that router can authenticate:

```bash
docker compose -f docker-compose.radius.yml restart freeradius
```

Updating user credentials does not require a restart; billing writes those into PostgreSQL and FreeRADIUS reads them during auth.
