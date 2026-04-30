# Tenant Clean URL Hybrid Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make clean tenant app URLs canonical while keeping `src/routes/[tenant]/(app)` as the temporary physical route tree.

**Architecture:** Preserve `reroute` as a compatibility layer for internal route matching, then move link generation to explicit canonical clean-path helpers. Migrate shared navigation first, then page-level redirects and cross-links, leaving physical route migration for a later plan.

**Tech Stack:** SvelteKit SPA, Svelte 5, TypeScript, Vitest, existing `rtk` command wrapper.

---

## File Structure

- Modify: `src/lib/utils/tenantRouting.ts`
  - Add canonical clean path helpers and legacy path helpers.
  - Keep `resolveTenantContext` for compatibility.

- Create: `src/lib/utils/tenantRouting.test.ts`
  - Lock helper behavior before touching consumers.

- Modify: `src/hooks.ts`
  - Keep internal reroute behavior, clarify canonical/legacy handling if needed.

- Modify: `src/hooks.reroute.test.ts`
  - Add tests for clean canonical behavior and legacy normalization.

- Modify: `src/lib/utils/appLanding.ts`
  - Add or adapt landing helpers to prefer clean paths.

- Modify: `src/lib/utils/adminBillingNavigation.ts`
  - Use clean canonical app paths by default.

- Modify: `src/lib/utils/adminCustomerNavigation.ts`
  - Use clean canonical app paths by default.

- Modify: `src/lib/utils/announcementRouting.ts`
  - Preserve internal/external announcement behavior while using canonical defaults.

- Modify tests:
  - `src/lib/utils/adminBillingNavigation.test.ts`
  - `src/lib/utils/adminCustomerNavigation.test.ts`
  - `src/lib/utils/announcementRouting.test.ts`
  - `src/lib/utils/adminDashboard.test.ts`

- Modify shared components:
  - `src/lib/components/layout/Sidebar.svelte`
  - `src/lib/components/layout/AnnouncementBanner.svelte`
  - `src/lib/components/layout/NotificationDropdown.svelte`

- Modify low-risk pages:
  - `src/routes/login/+page.svelte`
  - `src/routes/dashboard/+page.svelte`
  - `src/routes/[tenant]/(app)/+layout.svelte`
  - `src/routes/[tenant]/(app)/dashboard/+page.svelte`
  - `src/routes/[tenant]/(app)/profile/+page.svelte`
  - `src/routes/[tenant]/(app)/notifications/+page.svelte`
  - `src/routes/[tenant]/(app)/announcements/[id]/+page.svelte`
  - `src/routes/[tenant]/(app)/storage/+page.svelte`

- Later chunks modify admin pages incrementally. Do not delete or move `[tenant]` routes in this plan.

---

## Chunk 1: Canonical Routing Helpers

### Task 1: Add Tests for Canonical Path Helpers

**Files:**
- Create: `src/lib/utils/tenantRouting.test.ts`
- Modify: `src/lib/utils/tenantRouting.ts`

- [ ] **Step 1: Write failing tests**

Add tests for clean path normalization:

```ts
import { describe, expect, it } from 'vitest';
import { canonicalTenantPath, legacyTenantPath } from './tenantRouting';

describe('tenant routing helpers', () => {
  it('builds clean canonical tenant app paths', () => {
    expect(canonicalTenantPath('admin/settings')).toBe('/admin/settings');
    expect(canonicalTenantPath('/dashboard')).toBe('/dashboard');
  });

  it('preserves query strings', () => {
    expect(canonicalTenantPath('/profile?tab=security')).toBe('/profile?tab=security');
  });

  it('normalizes duplicate slashes', () => {
    expect(canonicalTenantPath('//admin//settings')).toBe('/admin/settings');
  });

  it('builds explicit legacy paths only when requested', () => {
    expect(legacyTenantPath('demo', '/admin/settings')).toBe('/demo/admin/settings');
  });
});
```

- [ ] **Step 2: Run tests and verify failure**

Run: `rtk npm test -- src/lib/utils/tenantRouting.test.ts`

Expected: FAIL because `canonicalTenantPath` and `legacyTenantPath` do not exist yet.

- [ ] **Step 3: Implement minimal helpers**

Add helpers to `src/lib/utils/tenantRouting.ts`:

```ts
function splitPathAndSuffix(path: string): { pathname: string; suffix: string } {
  const raw = String(path || '/').trim() || '/';
  const match = raw.match(/^([^?#]*)(.*)$/);
  return {
    pathname: match?.[1] || '/',
    suffix: match?.[2] || '',
  };
}

function normalizePath(path: string): string {
  const { pathname, suffix } = splitPathAndSuffix(path);
  const normalized = `/${pathname.split('/').filter(Boolean).join('/')}`;
  return `${normalized === '/' ? '/' : normalized}${suffix}`;
}

export function canonicalTenantPath(path: string): string {
  return normalizePath(path);
}

export function legacyTenantPath(slug: string, path: string): string {
  const cleanSlug = normalize(slug).replace(/^\/+|\/+$/g, '');
  const cleanPath = canonicalTenantPath(path);
  return cleanSlug ? `/${cleanSlug}${cleanPath === '/' ? '' : cleanPath}` : cleanPath;
}
```

- [ ] **Step 4: Run helper tests**

Run: `rtk npm test -- src/lib/utils/tenantRouting.test.ts`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add src/lib/utils/tenantRouting.ts src/lib/utils/tenantRouting.test.ts
rtk git commit -m "test: lock canonical tenant path helpers"
```

### Task 2: Extend Reroute Tests

**Files:**
- Modify: `src/hooks.reroute.test.ts`
- Modify: `src/hooks.ts` only if tests reveal behavior drift.

- [ ] **Step 1: Add tests for canonical platform behavior**

Add cases:

```ts
it('rewrites clean platform admin path internally when tenant slug exists', () => {
  installBrowserTenantSlug('demo');
  expect(run('billing.tridigitals.com', '/admin/settings')).toBe('/demo/admin/settings');
});

it('keeps platform login clean even when tenant slug exists', () => {
  installBrowserTenantSlug('demo');
  expect(run('billing.tridigitals.com', '/login')).toBeUndefined();
});

it('canonicalizes legacy slug-prefixed platform app paths before internal rewrite', () => {
  installBrowserTenantSlug('demo');
  expect(run('billing.tridigitals.com', '/oldslug/admin/settings')).toBe('/demo/admin/settings');
});
```

- [ ] **Step 2: Run reroute tests**

Run: `rtk npm test -- src/hooks.reroute.test.ts`

Expected: existing behavior may already pass. If not, adjust `src/hooks.ts` minimally.

- [ ] **Step 3: Commit**

```bash
rtk git add src/hooks.ts src/hooks.reroute.test.ts
rtk git commit -m "test: cover tenant clean url rerouting"
```

---

## Chunk 2: Helper Consumers

### Task 3: Update Landing and Navigation Helpers

**Files:**
- Modify: `src/lib/utils/appLanding.ts`
- Modify: `src/lib/utils/adminBillingNavigation.ts`
- Modify: `src/lib/utils/adminCustomerNavigation.ts`
- Modify: `src/lib/utils/announcementRouting.ts`
- Modify tests for those modules.

- [ ] **Step 1: Update tests to expect clean defaults**

Change tests so normal/default path outputs use `/admin`, `/dashboard`, `/admin/invoices`, etc. Keep one explicit legacy test for each helper that still needs legacy behavior.

- [ ] **Step 2: Run tests and verify failures**

Run:

```bash
rtk npm test -- \
  src/lib/utils/adminBillingNavigation.test.ts \
  src/lib/utils/adminCustomerNavigation.test.ts \
  src/lib/utils/announcementRouting.test.ts
```

Expected: FAIL where helpers still return `/demo/...`.

- [ ] **Step 3: Replace default `tenantPrefix` concatenation**

Use `canonicalTenantPath('/admin/...')` for normal links. Keep legacy slug generation behind explicit options only.

Example direction:

```ts
import { canonicalTenantPath } from './tenantRouting';

export function buildAdminBillingNavigation() {
  return {
    tenantPrefix: '',
    billingPath: canonicalTenantPath('/admin/invoices'),
    collectionsPath: canonicalTenantPath('/admin/invoices/collection'),
    billingPlanSettingsPath: canonicalTenantPath('/admin/settings?tab=billing_plan'),
    subscriptionPath: canonicalTenantPath('/admin/subscription'),
  };
}
```

- [ ] **Step 4: Run helper tests**

Run:

```bash
rtk npm test -- \
  src/lib/utils/appLanding.test.ts \
  src/lib/utils/adminBillingNavigation.test.ts \
  src/lib/utils/adminCustomerNavigation.test.ts \
  src/lib/utils/announcementRouting.test.ts \
  src/lib/utils/adminDashboard.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add src/lib/utils
rtk git commit -m "refactor: default tenant navigation helpers to clean urls"
```

---

## Chunk 3: Shared Layout Navigation

### Task 4: Migrate Shared Components to Clean Paths

**Files:**
- Modify: `src/lib/components/layout/Sidebar.svelte`
- Modify: `src/lib/components/layout/AnnouncementBanner.svelte`
- Modify: `src/lib/components/layout/NotificationDropdown.svelte`

- [ ] **Step 1: Replace high-level link construction**

In `Sidebar.svelte`, keep `resolveTenantContext` only if still needed for display or compatibility, but make link values clean:

```ts
href: canonicalTenantPath('/dashboard')
href: canonicalTenantPath('/admin/network/noc')
```

For dynamic `goto`, use:

```ts
goto(canonicalTenantPath('/profile'))
```

- [ ] **Step 2: Keep active-state checks compatible**

Any `isActive` logic should compare normalized visible path against clean item href. If the current visible path is legacy slug-prefixed, normalize it before comparison.

- [ ] **Step 3: Update announcement and notification routes**

Replace `${tenantPrefix}/notifications`, `${tenantPrefix}/profile`, and announcement detail links with canonical clean paths unless the helper explicitly needs internal legacy behavior.

- [ ] **Step 4: Run focused checks**

Run:

```bash
rtk npm run check
rtk npm test -- src/lib/utils/tenantRouting.test.ts src/hooks.reroute.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add src/lib/components/layout src/lib/utils src/hooks.reroute.test.ts
rtk git commit -m "refactor: use clean urls in shared tenant navigation"
```

---

## Chunk 4: Low-Risk Page Redirects and Links

### Task 5: Migrate Login and Dashboard Redirects

**Files:**
- Modify: `src/routes/login/+page.svelte`
- Modify: `src/routes/dashboard/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/+layout.svelte`

- [ ] **Step 1: Update redirect expectations mentally before editing**

Expected redirect targets:

- authenticated internal/admin users: `/admin`
- customer users: `/dashboard`
- 2FA requirement: `/profile?2fa_required=true`
- platform legacy slug path: clean path without slug

- [ ] **Step 2: Replace `ctx.tenantPrefix` target generation**

Use clean landing path generation for normal redirects. If `getDefaultTenantLandingPath` still accepts a prefix, pass `''` during this hybrid phase or add a clean-specific helper.

- [ ] **Step 3: Preserve mismatch guard behavior**

In `[tenant]/(app)/+layout.svelte`, do not remove mismatch checks. Only ensure redirect destinations on platform domain remain clean.

- [ ] **Step 4: Run checks**

Run:

```bash
rtk npm run check
rtk npm test -- src/hooks.reroute.test.ts src/lib/utils/tenantRouting.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add src/routes/login/+page.svelte src/routes/dashboard/+page.svelte 'src/routes/[tenant]/(app)/+layout.svelte'
rtk git commit -m "refactor: canonicalize tenant redirect targets"
```

### Task 6: Migrate Dashboard/Profile/Notifications Area

**Files:**
- Modify: `src/routes/[tenant]/(app)/dashboard/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/profile/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/notifications/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/announcements/[id]/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/storage/+page.svelte`

- [ ] **Step 1: Replace page-level `tenantPrefix` links**

Use canonical clean paths for:

- `/admin`
- `/dashboard/services`
- `/profile`
- `/profile?tab=security`
- `/profile?tab=notifications`
- `/notifications`
- `/announcements`

- [ ] **Step 2: Keep props compatible**

Where child components still accept `tenantPrefix`, pass `''` or update the child component in the same task if it only uses the prop for navigation.

- [ ] **Step 3: Run checks**

Run:

```bash
rtk npm run check
rtk npm test -- src/lib/utils/announcementRouting.test.ts src/lib/utils/tenantRouting.test.ts
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
rtk git add 'src/routes/[tenant]/(app)/dashboard/+page.svelte' 'src/routes/[tenant]/(app)/profile/+page.svelte' 'src/routes/[tenant]/(app)/notifications/+page.svelte' 'src/routes/[tenant]/(app)/announcements/[id]/+page.svelte' 'src/routes/[tenant]/(app)/storage/+page.svelte'
rtk git commit -m "refactor: use clean urls in tenant portal pages"
```

---

## Chunk 5: Admin Areas

### Task 7: Migrate Admin Overview and Billing Links

**Files:**
- Modify: `src/routes/[tenant]/(app)/admin/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/invoices/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/invoices/[id]/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/invoices/collection/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/subscription/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/customers/[id]/+page.svelte`

- [ ] **Step 1: Update helper tests first**

Ensure `adminBillingNavigation` and `adminCustomerNavigation` tests already expect clean paths.

- [ ] **Step 2: Replace local path constants**

Replace `${tenantPrefix}/admin/...` with clean helper outputs.

- [ ] **Step 3: Run checks**

Run:

```bash
rtk npm run check
rtk npm test -- src/lib/utils/adminBillingNavigation.test.ts src/lib/utils/adminCustomerNavigation.test.ts
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
rtk git add 'src/routes/[tenant]/(app)/admin/+page.svelte' 'src/routes/[tenant]/(app)/admin/invoices' 'src/routes/[tenant]/(app)/admin/subscription/+page.svelte' 'src/routes/[tenant]/(app)/admin/customers/[id]/+page.svelte'
rtk git commit -m "refactor: use clean urls in admin billing navigation"
```

### Task 8: Migrate Admin Network Links Last

**Files:**
- Modify: `src/routes/[tenant]/(app)/admin/network/**/*.svelte`
- Modify: `src/lib/components/network/**/*.svelte`
- Modify: `src/lib/components/network/**/*.ts`

- [ ] **Step 1: Inventory network tenantPrefix use**

Run:

```bash
rtk rg -n "tenantPrefix|resolveTenantContext|\\$page\\.params\\.tenant" 'src/routes/[tenant]/(app)/admin/network' src/lib/components/network
```

Expected: list all remaining network references.

- [ ] **Step 2: Split changes by area**

Do not edit all network pages at once. Use this order:

- import center
- alerts/incidents/noc
- PPPoE/import
- map
- wallboard/settings

- [ ] **Step 3: Run checks after each area**

Run after each area:

```bash
rtk npm run check
```

Expected: PASS.

- [ ] **Step 4: Commit each area separately**

Use messages like:

```bash
rtk git commit -m "refactor: use clean urls in network import navigation"
rtk git commit -m "refactor: use clean urls in network operations navigation"
```

---

## Chunk 6: Verification and Future Route Prep

### Task 9: Add No-New-Slug-Link Audit

**Files:**
- Optional create: `src/lib/utils/tenantRouting.audit.test.ts`
- Or add to existing tests if project conventions prefer that.

- [ ] **Step 1: Add a lightweight audit test**

Add a test that scans key navigation helper outputs and asserts they do not start with `/demo/` unless explicitly using `legacyTenantPath`.

- [ ] **Step 2: Run full relevant test suite**

Run:

```bash
rtk npm test -- src/hooks.reroute.test.ts src/lib/utils
rtk npm run check
```

Expected: PASS.

- [ ] **Step 3: Manual browser verification**

Start the app using the repo's normal dev command, then verify:

- `/login` works.
- Login redirects to `/admin` or `/dashboard`.
- Sidebar links do not include `/:tenant`.
- `/demo/admin` on platform domain normalizes to `/admin`.
- Custom domain path `/admin` still loads when a cached domain mapping exists.
- Superadmin paths are untouched.

- [ ] **Step 4: Document next-phase route move**

Add a short note to this plan or create a follow-up TODO for moving from:

```text
src/routes/[tenant]/(app)
```

to:

```text
src/routes/(app)
```

after clean navigation is stable.

- [ ] **Step 5: Final commit**

```bash
rtk git add src docs
rtk git commit -m "docs: record tenant clean url migration follow-up"
```

## Notes

- Do not delete `src/routes/[tenant]/(app)` in this implementation plan.
- Do not remove auth or mismatch guards during the hybrid phase.
- Prefer one area per commit because many files already contain unrelated work in active development.
- Automated plan subagent review was not run because this Codex session is configured to spawn subagents only when explicitly requested by the user.
