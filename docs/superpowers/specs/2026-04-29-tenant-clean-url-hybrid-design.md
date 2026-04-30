# Tenant Clean URL Hybrid Migration Design

## Summary

This migration keeps `src/routes/[tenant]/(app)` as the temporary physical route tree while making clean app URLs the product-facing canonical contract. Platform-domain and custom-domain users should navigate with `/admin`, `/dashboard`, `/profile`, `/support`, `/notifications`, `/announcements`, and `/storage`. Slug-prefixed tenant URLs remain supported only as legacy compatibility and should normalize back to clean URLs.

## Current State

The app runs as an SPA (`ssr = false`) and relies on SvelteKit's `reroute` hook to map clean URLs to physical `[tenant]` routes when a tenant slug is available in browser storage or from a custom domain. For example, `https://billing.tridigitals.com/admin/storage` can be internally resolved to `/xtrabit/admin/storage` while keeping the visible browser URL clean.

The complexity comes from two responsibilities being mixed together:

- Route matching still depends on `/:tenant/...`.
- User-visible navigation increasingly expects clean root app paths.

Many pages and shared components still compute `tenantPrefix` from `$page.params.tenant`, then build links with `${tenantPrefix}/admin` or `${tenantPrefix}/dashboard`. That is useful for the temporary internal route tree, but it keeps slug-prefixed URLs alive in normal navigation and makes route guards perform extra canonicalization.

## Goals

- Keep the migration low-risk by retaining `[tenant]` as the internal route tree for now.
- Make clean URLs canonical for platform and custom domains.
- Preserve existing auth, RBAC, 2FA, maintenance, domain mismatch, and tenant mismatch behavior.
- Keep legacy slug-prefixed links functional long enough to avoid breaking bookmarks.
- Add tests around canonical URL behavior before changing navigation helpers.
- Prepare the route tree for a later move from `[tenant]/(app)` to root `(app)`.

## Non-Goals

- Do not move all app routes out of `[tenant]` in this phase.
- Do not redesign auth, tenant membership, or permission checks.
- Do not change backend tenant scoping.
- Do not remove custom-domain support.
- Do not remove legacy slug URLs until telemetry/manual QA confirms they are no longer needed.

## Canonical Routing Rules

1. Platform domain app paths are clean:
   - `/admin`
   - `/dashboard`
   - `/profile`
   - `/support`
   - `/notifications`
   - `/announcements`
   - `/storage`

2. Custom-domain app paths are also clean:
   - `https://customer-domain.example/admin`
   - `https://customer-domain.example/dashboard`

3. Slug-prefixed app paths are legacy:
   - `/demo/admin/settings` should normalize to `/admin/settings` on the platform domain.
   - On a non-platform tenant domain, a mismatched slug should normalize to that user's expected tenant route or logout according to the existing guard behavior.

4. Public/platform routes are never tenant-rerouted:
   - `/login`
   - `/register`
   - `/forgot-password`
   - `/verify-email`
   - `/install`
   - `/maintenance`
   - `/unauthorized`
   - `/pay/:id`
   - `/superadmin/...`

## Architecture

### Compatibility Layer

`src/hooks.ts` remains the temporary compatibility layer. It keeps mapping clean tenant-aware app paths to physical `[tenant]` routes when a slug can be resolved. It also keeps canonicalizing legacy slug-prefixed platform URLs back to clean app URLs.

This layer should be explicitly documented as temporary. Its job is route matching, not product navigation.

### URL Builder Layer

`src/lib/utils/tenantRouting.ts` should become the source of truth for canonical app URL behavior. During the hybrid phase, it can still return a `tenantPrefix` for compatibility, but new code should prefer canonical app path helpers that return clean paths for normal browser navigation.

Proposed helper direction:

- `canonicalTenantPath(path: string): string` returns normalized clean app paths like `/admin/settings`.
- `legacyTenantPath(slug: string, path: string): string` exists only for compatibility tests or explicit legacy redirects.
- `resolveTenantContext(...)` remains for domain/tenant detection but should stop being the primary link-building API.

### Navigation Consumers

Shared navigation components should be migrated first:

- `src/lib/components/layout/Sidebar.svelte`
- `src/lib/components/layout/AnnouncementBanner.svelte`
- `src/lib/components/layout/NotificationDropdown.svelte`
- tenant navigation helper modules under `src/lib/utils`

Page-level migrations can then happen by area. Dashboard/profile/notifications are lower risk than admin network pages, so they should be moved first.

### Route Tree

The physical route tree remains:

- `src/routes/[tenant]/(app)/...`

The future route tree is prepared but not completed in this phase:

- `src/routes/(app)/...`

The future move should happen only after canonical link generation is stable and covered by tests.

## Data Flow

1. User visits `/admin`.
2. `reroute` checks whether the path is a tenant-aware app path.
3. `getSlugFromDomain` resolves the tenant slug from custom domain mapping or browser auth storage.
4. If a slug exists, `reroute` internally resolves `/admin` to `/:slug/admin`.
5. `[tenant]/(app)/+layout.svelte` runs existing auth and permission guards.
6. Components render links using clean canonical paths.
7. If any legacy `/:slug/...` path is reached on the platform domain, the guard normalizes visible navigation back to the clean path.

## Error Handling

- If no tenant slug is available for a clean protected app path, the route should fall through to the existing login/session handling instead of inventing a fake slug.
- If a logged-in user reaches a mismatched tenant slug, the existing mismatch guard should keep normalizing rather than silently switching account context.
- If custom-domain lookup fails, the app should continue using existing fallback/cache behavior and show login or protected-route redirects as it does today.
- If canonicalization would redirect to the same path, navigation must be skipped to avoid loops.

## Testing Strategy

Unit tests should be added or extended before behavior changes:

- `src/hooks.reroute.test.ts`
  - clean platform app path reroutes internally when browser tenant slug exists.
  - slug-prefixed platform app path normalizes internally to clean app path before reroute.
  - public routes and superadmin routes never reroute.
  - custom domain clean app path reroutes internally.

- `src/lib/utils/tenantRouting.test.ts`
  - canonical helper returns clean paths.
  - duplicate slashes are normalized.
  - query strings are preserved.
  - legacy helper is explicit and not used by default.

- Navigation helper tests:
  - `adminBillingNavigation`
  - `adminCustomerNavigation`
  - `announcementRouting`
  - `appLanding`

Component-level browser verification should cover:

- Login redirect lands at clean app URL.
- Sidebar links are clean on platform domain.
- Dashboard quick actions are clean.
- Admin billing/customer links are clean.
- Custom domain still loads app pages.
- Legacy `/:tenant/admin` normalizes to `/admin`.

## Migration Phases

### Phase 1: Lock Canonical Rules

Add tests and helper APIs that define clean URLs as the default. Do not change large page files yet.

### Phase 2: Migrate Shared Navigation

Move shared navigation and helper modules to use canonical clean path builders. This removes most repeated `tenantPrefix` propagation from high-traffic UI.

### Phase 3: Migrate Low-Risk Tenant Pages

Update dashboard, profile, notifications, announcements, storage redirects, login, and root dashboard redirects. Keep `[tenant]` physical routes intact.

### Phase 4: Migrate Admin Areas Gradually

Update admin overview and billing/customer pages before network pages. Network pages have heavier cross-links and should be last.

### Phase 5: Prepare Root Route Tree

After navigation is clean and stable, introduce `src/routes/(app)` route groups area by area. Keep `[tenant]` as compatibility until every app page has a root route equivalent.

## Rollback Strategy

Because `[tenant]` remains as the physical route tree during this hybrid migration, rollback is straightforward:

- Revert canonical helper usage in navigation consumers.
- Keep `reroute` behavior unchanged.
- Legacy slug-prefixed URLs continue to work.

Avoid deleting `[tenant]` routes or route guards in this phase.

## Acceptance Criteria

- Normal user navigation on the platform domain never produces `/:tenant/...`.
- Custom-domain app navigation stays clean.
- Legacy `/:tenant/...` links do not break and normalize where appropriate.
- Existing auth, permission, 2FA, maintenance, and domain mismatch flows still work.
- Unit tests document canonical routing behavior.
- The codebase has a clear next step toward moving app routes into root `(app)` routes.

## Review Note

Automated spec subagent review was not run because this Codex session is configured to spawn subagents only when explicitly requested by the user. This document should be user-reviewed before implementation.
