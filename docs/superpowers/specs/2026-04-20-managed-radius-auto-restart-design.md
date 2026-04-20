# Managed RADIUS Auto-Restart On Mapping Edit Design

## Goal

When a superadmin edits a NAS mapping in `/superadmin/radius/mappings`, automatically restart FreeRADIUS if the edit changes client-facing NAS fields that are only loaded at FreeRADIUS startup.

## Scope

This design only covers the existing superadmin NAS mapping edit flow. It does not attempt to react to direct SQL edits, imports, or other backend writers.

## Current State

- NAS mapping CRUD already exists in the superadmin UI and backend.
- FreeRADIUS loads NAS clients from SQL on startup.
- Updating `managed_radius_nas` in the database is not enough to refresh the in-memory client list for source IP / secret changes.

## Proposed Behavior

- Keep the existing edit flow and validation.
- After `update_mapping` saves the new NAS mapping, compare old vs new values for fields that change the effective FreeRADIUS client entry:
  - `nas_name`
  - `nas_ip_or_cidr`
  - `shortname`
  - `shared_secret`
  - `is_active`
- If none of those fields changed, do nothing extra.
- If any of those fields changed, run a configured restart command for FreeRADIUS.

## Operational Model

- Restart execution is environment-driven:
  - `MANAGED_RADIUS_RESTART_COMMAND`
  - `MANAGED_RADIUS_RESTART_WORKDIR`
- If the command is not configured, mapping edits continue to work without automatic restart.
- If the command is configured but fails, the edit request fails so the operator sees the restart problem immediately.

## Rationale

- Triggering restart in the service keeps the behavior close to the domain rule instead of coupling it to HTTP/UI code.
- Using an environment-configured command avoids hardcoding host-specific Docker assumptions into the binary.
- Restricting the behavior to mapping edits keeps the change small and predictable.

## Testing

- Unit test pure change-detection logic.
- Unit test env command resolution.
- Regression test that `update_mapping` still syncs runtime NAS state and now includes the restart hook.
- Keep the existing JS config/documentation regression test green.
