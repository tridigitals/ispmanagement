# Managed RADIUS Default Assignment And Plan Gating Design (2026-04-02)

## Background / Current State
- Managed RADIUS already supports:
  - global RADIUS servers
  - per-tenant active server assignments
  - per-router NAS mappings
  - superadmin observability and control-plane management
- Today, tenant assignment to a RADIUS server is still manual.
- There is no plan-level gate that decides whether a tenant is entitled to use Managed RADIUS.
- Tenant router detail pages show RADIUS setup information, but they do not yet explain when the tenant’s subscription plan does not include the feature.

## Product Decision
Add three related behaviors:

1. One platform-wide default Managed RADIUS server
2. Automatic assignment for new tenants when their plan allows Managed RADIUS
3. Plan-aware upgrade messaging on tenant router pages when Managed RADIUS is not included

Explicit rules:
- Only one global RADIUS server can be marked as the platform default at a time.
- Auto-assignment applies only to newly created tenants.
- Existing tenant assignments are not automatically migrated when the default server changes.
- Plan access controls whether Managed RADIUS is available at all.
- Router NAS mapping creation remains manual in this phase.

## Goals / Non-goals
### Goals
- Reduce superadmin operational work for common onboarding.
- Keep Managed RADIUS entitlement aligned with subscription plans.
- Give tenant admins clear upgrade guidance when their plan does not include Managed RADIUS.
- Preserve superadmin override capability for special tenants.

### Non-goals
- No automatic NAS mapping creation on router create.
- No automatic reassignment of existing tenants when the default server changes.
- No per-plan default server matrix in this phase.
- No dedicated-server orchestration per plan in this phase.

## Recommended Architecture
Keep the current control plane, and add one entitlement layer plus one platform default setting:

1. Platform default server
   - one `radius_servers` record may be the default

2. Plan entitlement
   - a plan feature flag decides whether the tenant may use Managed RADIUS

3. Tenant auto-assignment
   - tenant creation checks plan entitlement
   - if eligible and a default server exists, create one active `tenant_radius_assignment`

4. Tenant router UX
   - if feature is disabled by plan, router detail page shows an upgrade notice instead of actionable Managed RADIUS setup

This keeps infrastructure ownership, tenant assignment, and plan entitlement separate.

## Data Model Changes

### 1. Global Default Server Flag
Extend `radius_servers` with:
- `is_default boolean not null default false`

Rules:
- only one row may have `is_default = true`
- inactive servers should not be allowed to become default

Recommended DB constraint:
- partial unique index on `(is_default)` where `is_default = true`

### 2. Plan Feature Gate
Use the existing plan feature system.

Add a new feature definition:
- `code = managed_radius`
- `value_type = boolean`
- default value = `false`

Operational meaning:
- `true` means the tenant is entitled to use Managed RADIUS
- `false` means tenant UI should treat Managed RADIUS as unavailable and show upgrade guidance

No separate boolean column on plans is needed because the project already has feature-based entitlement checks.

## Runtime Rules

### Tenant Creation
When a superadmin creates a tenant:
1. tenant is created normally
2. requested plan is assigned
3. backend checks plan feature access for `managed_radius`
4. backend resolves the one default global RADIUS server
5. if feature access is `true` and a default server exists:
   - create active `tenant_radius_assignment`
6. if no default server exists:
   - tenant creation still succeeds
   - no assignment is created
7. if feature access is `false`:
   - tenant creation still succeeds
   - no assignment is created

### Default Server Change
When superadmin marks a different server as default:
- previous default is cleared in the same transaction
- only future tenant creation uses the new default
- existing tenant assignments are unchanged

### Router Detail Behavior
On tenant router detail page:
- if tenant plan allows Managed RADIUS:
  - show existing Managed RADIUS setup block as today
- if tenant plan does not allow Managed RADIUS:
  - show an informational upgrade card
  - CTA links to `/admin/subscription`
  - do not show misleading operational setup guidance

## UX Design

## `/superadmin/radius`

### Global Server Tab
Add default-server controls:
- column/badge showing `Default`
- action to set a server as default
- only one server can display `Default`

Recommended behavior:
- if server is inactive, disable or reject “Set as default”

### Assignment Behavior
No visible onboarding wizard is required in this phase.
The result is observable through the existing `Assignments` tab.

## `/superadmin/plans/[id]`
Add feature control using the existing feature editor:
- expose `Managed RADIUS` as a boolean feature under features/limits

No special-case plan UI is required beyond making sure the feature definition exists and is understandable.

## `/{tenant}/admin/network/routers/[id]`
Add a plan-aware notice:
- title: plan does not include Managed RADIUS
- body: explain that the current subscription must be upgraded
- CTA: open `/admin/subscription`

If the tenant is eligible but not configured:
- retain the current “not configured yet” style behavior

This distinction is important:
- `not entitled` should suggest upgrade
- `entitled but not configured` should suggest admin setup

## Backend Service Changes

### ManagedRadiusService
Add support for:
- setting one server as default
- resolving the current default server

### PlanService
Reuse existing feature access checks for:
- `managed_radius`

No new entitlement subsystem is needed.

### Superadmin Tenant Creation Flow
Extend tenant creation so that after plan assignment it can:
- check plan feature access
- create tenant assignment automatically when eligible

This logic should be transactional where practical, or at minimum fail soft:
- tenant creation must not be rolled back solely because default RADIUS assignment is unavailable
- failures should be captured in logs/audit so operators can resolve them

## API Design

### Superadmin RADIUS Server Surface
Extend server responses with:
- `is_default`

Add a superadmin-only action:
- `set default server`

### Tenant Router Setup Surface
Extend router setup response with plan entitlement fields, for example:
- `plan_allows_managed_radius`
- `plan_upgrade_required`
- `upgrade_path`

This keeps the frontend simple and avoids duplicating entitlement resolution logic in Svelte.

## Error Handling
- If no default server exists, tenant creation still succeeds.
- If auto-assignment fails unexpectedly, tenant creation still succeeds, but backend should log and audit the failure.
- If plan feature definition is missing in an environment, treat feature as disabled and surface a supportable configuration warning.

## Security / Authorization
- default server mutation is superadmin only
- plan feature editing remains superadmin only
- tenant admins only see upgrade guidance, not plan internals beyond the fact that the feature is unavailable

## Testing Strategy

### Backend
- only one server can be default
- setting new default clears old default
- tenant creation with `managed_radius=true` and default server creates assignment
- tenant creation with `managed_radius=false` does not create assignment
- tenant creation with no default server does not fail
- inactive server cannot become default

### Frontend
- server tab renders default badge/state
- router detail page shows upgrade card when plan does not include Managed RADIUS
- router detail page still shows normal setup when entitlement exists

## Rollout
1. Add default-server support to RADIUS servers
2. Add `managed_radius` plan feature definition/seed
3. Add auto-assignment on tenant creation
4. Add router-page upgrade guidance
5. Verify with:
   - one eligible plan
   - one non-eligible plan
   - one default server

## Acceptance Criteria
- exactly one global RADIUS server can be default
- superadmin can change the default server
- new tenant with plan feature `managed_radius=true` gets an active assignment automatically when default exists
- new tenant with plan feature `managed_radius=false` does not get an assignment automatically
- changing the default server does not change existing tenant assignments
- tenant router detail page shows upgrade guidance when the plan does not include Managed RADIUS
