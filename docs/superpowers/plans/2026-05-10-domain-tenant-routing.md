# Domain Tenant Routing Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make tenant/domain resolution host-driven and production-safe, switch browser web API access to relative `/api`, and add verified custom-domain lifecycle and auth/domain guardrails without breaking Tauri/native behavior.

**Architecture:** Implement the change in four layers: browser API base hardening, backend shared domain resolver, tenant custom-domain lifecycle, and auth/redirect enforcement. Keep the current tenant/custom-domain foundations, but move authority to the backend request host and reduce frontend reliance on cached slug/domain hints.

**Tech Stack:** SvelteKit SPA, TypeScript, Vitest, Rust/Axum backend, SQLx migrations, existing `rtk` command wrapper.

---

## File Structure

- Modify: `src/lib/utils/apiUrl.ts`
  - Make browser web default to relative `/api`.
  - Preserve explicit override behavior for Tauri/native runtime only.

- Create: `src/lib/utils/apiUrl.test.ts`
  - Lock browser vs Tauri API-base behavior before implementation.

- Modify: `.env.example`
  - Document the new browser/Tauri API base rules clearly.

- Modify: `src/lib/utils/domain.ts`
  - Reduce domain fallback behavior that currently guesses tenant slug from host too aggressively.
  - Keep client cache as hint-only behavior.

- Modify: `src/routes/+layout.svelte`
  - Use domain lookup only as boot-time UX optimization.
  - Stop treating cached slug/domain data as authoritative for tenant resolution.

- Modify: `src/routes/login/+page.svelte`
  - Enforce platform-domain vs tenant-domain redirect rules after login.
  - Isolate superadmin behavior to platform domain.

- Modify: `src/lib/api/public.ts`
  - Keep domain-aware public calls explicit and consistent with current host behavior.

- Create: `src-tauri/src/http/domain_resolver.rs`
  - Central shared host/domain resolver for platform domain, platform subdomain, and custom domain.

- Modify: `src-tauri/src/http/mod.rs` or the relevant parent module file if needed
  - Export the new resolver module.

- Modify: `src-tauri/src/http/public.rs`
  - Replace repeated raw custom-domain lookups with the shared resolver.
  - Restrict tenant-domain-only flows to active verified domains.

- Modify: `src-tauri/src/http/tenant.rs`
  - Add normalized domain updates and lifecycle-aware tenant domain behavior.

- Modify: `src-tauri/src/http/superadmin.rs`
  - Enforce custom-domain feature/lifecycle behavior in superadmin tenant CRUD/update flows.

- Modify: `src-tauri/src/bootstrap/http.rs`
  - Refresh only active verified domains into CORS/runtime caches.
  - Keep platform static origins separate from tenant domain runtime allowlists.

- Modify: `src-tauri/src/models/tenant.rs`
  - Add lifecycle fields if using the smallest in-place schema change first.

- Create: `src-tauri/migrations/20260510000100_tenant_domain_lifecycle.up.sql`
- Create: `src-tauri/migrations/20260510000100_tenant_domain_lifecycle.down.sql`
  - Add lifecycle fields or a new tenant-domain table, depending on the chosen minimal schema path.

- Modify: `src-tauri/src/services/auth_service/dto.rs`
  - Ensure main-domain/platform-domain metadata is explicit enough for frontend auth redirect decisions.

- Modify: `src-tauri/src/services/auth_service/repository.rs`
  - Keep global auth settings retrieval aligned with platform-domain configuration.

- Modify: `src-tauri/src/services/auth_service/mod.rs`
  - Harden login success mapping and tenant/custom-domain metadata enrichment.

- Modify: `src-tauri/src/services/auth_service/mapper.rs`
  - Keep returned user tenant/domain data aligned with the new lifecycle-aware domain model.

- Modify / Create tests:
  - `src/lib/utils/apiUrl.test.ts`
  - `src/lib/utils/domain.test.ts` if needed
  - `src/lib/utils/appBoot.test.ts`
  - `src-tauri/src/http/public.rs` test module or new focused resolver tests
  - `src-tauri/src/services/auth_service/tests.rs`
  - `src-tauri/src/services/auth_service/integration.rs`

- Verification commands:
  - `rtk npm test -- src/lib/utils/apiUrl.test.ts src/lib/utils/appBoot.test.ts`
  - `rtk npm run check`
  - `rtk cargo test domain_resolver`
  - `rtk cargo test auth_service`

---

## Chunk 1: Browser API Base Hardening

### Task 1: Lock browser vs Tauri API base behavior with tests

**Files:**
- Create: `src/lib/utils/apiUrl.test.ts`
- Modify: `src/lib/utils/apiUrl.ts`

- [ ] **Step 1: Write the failing tests**

Add tests that document the desired rules:

```ts
import { describe, expect, it, vi } from 'vitest';

describe('getApiBaseUrl', () => {
  it('uses relative /api in standard browser web runtime even when VITE_API_URL is set', async () => {
    vi.stubGlobal('window', {
      location: { origin: 'https://portal.acme.net' },
      __TAURI_INTERNALS__: undefined,
    });
    const { getApiBaseUrl } = await import('./apiUrl');
    expect(getApiBaseUrl()).toBe('https://portal.acme.net/api');
  });

  it('uses explicit API URL in tauri runtime', async () => {
    vi.stubGlobal('window', {
      location: { origin: 'https://portal.acme.net' },
      __TAURI_INTERNALS__: {},
    });
    const { getApiBaseUrl } = await import('./apiUrl');
    expect(getApiBaseUrl()).toBe('https://api.example.com/api');
  });
});
```

- [ ] **Step 2: Run the focused test and verify failure**

Run:

```bash
rtk npm test -- src/lib/utils/apiUrl.test.ts
```

Expected: FAIL because browser runtime still prefers `VITE_API_URL`.

- [ ] **Step 3: Implement the minimal API base rule**

Adjust `src/lib/utils/apiUrl.ts` so that:

- browser web returns `${window.location.origin}/api`
- Tauri/native keeps env override support
- fallback remains `http://localhost:3000/api` only for non-browser/non-window cases

Keep the implementation small and explicit rather than adding multiple environment toggles immediately.

- [ ] **Step 4: Run the focused test and `svelte-check`**

Run:

```bash
rtk npm test -- src/lib/utils/apiUrl.test.ts
rtk npm run check
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add src/lib/utils/apiUrl.ts src/lib/utils/apiUrl.test.ts
rtk git commit -m "feat: harden browser api base resolution"
```

### Task 2: Document browser/Tauri API base rules

**Files:**
- Modify: `.env.example`

- [ ] **Step 1: Update env comments**

Document:

- browser web uses relative `/api` by default
- `VITE_API_URL` is for Tauri/native or special remote debugging modes
- production custom domains should terminate app and API on the same origin

- [ ] **Step 2: Run formatting-safe validation**

Run:

```bash
rtk npm run check
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
rtk git add .env.example
rtk git commit -m "docs: clarify browser and tauri api base rules"
```

---

## Chunk 2: Shared Backend Domain Resolver

### Task 3: Add a shared host/domain resolver with focused backend tests

**Files:**
- Create: `src-tauri/src/http/domain_resolver.rs`
- Modify: `src-tauri/src/http/mod.rs` or the equivalent module export file
- Create or extend tests in `src-tauri/src/http/domain_resolver.rs`

- [ ] **Step 1: Write failing resolver tests**

Add unit tests for:

- platform-domain detection
- host normalization (`Host`, lowercase, strip port)
- local/IP detection
- unknown domain rejection
- custom-domain active match
- platform subdomain match

Example skeleton:

```rust
#[test]
fn normalizes_host_and_strips_port() {
    assert_eq!(normalize_host("Portal.Acme.Net:443"), Some("portal.acme.net".into()));
}

#[test]
fn rejects_empty_host() {
    assert_eq!(normalize_host(""), None);
}
```

- [ ] **Step 2: Run the focused backend tests and verify failure**

Run:

```bash
rtk cargo test domain_resolver
```

Expected: FAIL because the resolver module does not exist yet.

- [ ] **Step 3: Implement the minimal resolver**

Create a focused module that exposes:

- `normalize_host(...)`
- `is_platform_domain(...)`
- `ResolvedDomainContext`
- `resolve_request_domain(...)`

The resolver should:

- accept trusted host input and app state/dependencies
- detect platform domain
- detect platform tenant subdomain
- detect active verified custom domain
- return explicit enum variants instead of ad hoc booleans

- [ ] **Step 4: Run focused backend tests**

Run:

```bash
rtk cargo test domain_resolver
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add src-tauri/src/http/domain_resolver.rs src-tauri/src/http/mod.rs
rtk git commit -m "feat: add shared backend domain resolver"
```

### Task 4: Migrate public domain-sensitive endpoints to the resolver

**Files:**
- Modify: `src-tauri/src/http/public.rs`
- Modify tests in `src-tauri/src/http/public.rs` or associated test files

- [ ] **Step 1: Add failing tests for host validation behavior**

Cover:

- invite validation rejected on platform domain
- invite validation rejected on local/IP hosts
- tenant registration rejected on inactive domain
- tenant registration accepted only when resolver returns active tenant domain

- [ ] **Step 2: Run focused tests and verify failure**

Run:

```bash
rtk cargo test public::
```

Expected: FAIL or partially fail where old direct lookup logic differs.

- [ ] **Step 3: Replace repeated direct domain SQL with resolver usage**

Refactor `public.rs` so handlers obtain domain context from the shared resolver and only proceed for the correct active tenant domain variants.

- [ ] **Step 4: Run focused tests**

Run:

```bash
rtk cargo test public::
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add src-tauri/src/http/public.rs
rtk git commit -m "refactor: use shared resolver in public domain flows"
```

---

## Chunk 3: Tenant Custom-Domain Lifecycle

### Task 5: Add lifecycle schema for tenant domains

**Files:**
- Create: `src-tauri/migrations/20260510000100_tenant_domain_lifecycle.up.sql`
- Create: `src-tauri/migrations/20260510000100_tenant_domain_lifecycle.down.sql`
- Modify: `src-tauri/src/models/tenant.rs`

- [ ] **Step 1: Write failing schema/model tests or assertions**

If an existing migration verification workflow exists, add the smallest useful regression around reading new fields from `Tenant`. Otherwise write compile-level model usage tests in Rust.

- [ ] **Step 2: Run migration/model-focused checks and verify failure**

Run:

```bash
rtk cargo test tenant
```

Expected: FAIL or missing-field compilation gap.

- [ ] **Step 3: Add minimal lifecycle schema**

Choose the smallest safe first step:

- keep `custom_domain` on `tenants`
- add lifecycle columns such as:
  - `custom_domain_status`
  - `custom_domain_verified_at`
  - `custom_domain_failure_reason`

Do not jump to a full separate domain table unless the migration clearly benefits from it immediately.

- [ ] **Step 4: Update Rust tenant model**

Add matching optional fields to `src-tauri/src/models/tenant.rs`.

- [ ] **Step 5: Run backend checks**

Run:

```bash
rtk cargo test tenant
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
rtk git add src-tauri/migrations/20260510000100_tenant_domain_lifecycle.* src-tauri/src/models/tenant.rs
rtk git commit -m "feat: add tenant custom domain lifecycle fields"
```

### Task 6: Gate CORS/runtime allowlists on active verified domains

**Files:**
- Modify: `src-tauri/src/bootstrap/http.rs`

- [ ] **Step 1: Add a failing regression test or at minimum a focused logic extraction**

If `bootstrap/http.rs` is too integration-heavy for direct tests, first extract the domain-list refresh filtering into a small pure helper inside the same file or a nearby utility, then test that helper.

- [ ] **Step 2: Run focused test and verify failure**

Run:

```bash
rtk cargo test cors_domain
```

Expected: FAIL before the filtering helper or logic exists.

- [ ] **Step 3: Implement active-domain filtering**

Only include:

- static platform origins from env
- tenant domains whose lifecycle status is `active`

Do not include pending, failed, or disabled domains in the runtime allowlist.

- [ ] **Step 4: Run focused backend tests**

Run:

```bash
rtk cargo test cors_domain
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add src-tauri/src/bootstrap/http.rs
rtk git commit -m "feat: restrict cors cache to active tenant domains"
```

### Task 7: Enforce lifecycle-aware custom-domain updates in tenant/superadmin endpoints

**Files:**
- Modify: `src-tauri/src/http/tenant.rs`
- Modify: `src-tauri/src/http/superadmin.rs`

- [ ] **Step 1: Add failing tests for domain normalization and status transitions**

Cover:

- lowercasing and stripping protocol/trailing slash
- duplicate domain rejection
- feature-gated custom-domain updates
- setting a changed domain back to non-active status until re-verified

- [ ] **Step 2: Run focused backend tests and verify failure**

Run:

```bash
rtk cargo test tenant:: 
rtk cargo test superadmin::
```

Expected: FAIL where lifecycle rules are not yet enforced.

- [ ] **Step 3: Implement normalization and lifecycle resets**

When `custom_domain` changes:

- normalize the domain
- enforce uniqueness
- set lifecycle back to pending/non-active until verified

- [ ] **Step 4: Run focused backend tests**

Run:

```bash
rtk cargo test tenant::
rtk cargo test superadmin::
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add src-tauri/src/http/tenant.rs src-tauri/src/http/superadmin.rs
rtk git commit -m "feat: enforce tenant custom domain lifecycle rules"
```

---

## Chunk 4: Auth And Redirect Hardening

### Task 8: Reduce frontend authority of cached domain slug state

**Files:**
- Modify: `src/lib/utils/domain.ts`
- Modify: `src/routes/+layout.svelte`
- Modify: `src/lib/utils/appBoot.test.ts` if needed

- [ ] **Step 1: Add failing frontend tests**

Cover:

- localhost/dev keeps using stored tenant hint
- platform domain does not guess tenant slug from arbitrary host pieces
- non-platform custom domain uses cached mapping only after successful backend lookup

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
rtk npm test -- src/lib/utils/appBoot.test.ts src/lib/utils/domain.test.ts
```

Expected: FAIL where current `domain.ts` still falls back to subdomain guessing too aggressively.

- [ ] **Step 3: Implement minimal hardening**

Adjust client logic so that:

- platform domain uses auth/session hint only for UX
- arbitrary browser host does not become authoritative tenant context
- custom-domain cache is populated only after backend success

- [ ] **Step 4: Run frontend checks**

Run:

```bash
rtk npm test -- src/lib/utils/appBoot.test.ts src/lib/utils/domain.test.ts
rtk npm run check
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add src/lib/utils/domain.ts src/routes/+layout.svelte src/lib/utils/appBoot.test.ts
rtk git commit -m "refactor: reduce frontend domain authority"
```

### Task 9: Enforce platform-domain vs tenant-domain auth redirects

**Files:**
- Modify: `src/routes/login/+page.svelte`
- Modify: `src-tauri/src/services/auth_service/dto.rs`
- Modify: `src-tauri/src/services/auth_service/repository.rs`
- Modify: `src-tauri/src/services/auth_service/mod.rs`
- Modify: `src-tauri/src/services/auth_service/mapper.rs`
- Modify tests:
  - `src-tauri/src/services/auth_service/tests.rs`
  - `src-tauri/src/services/auth_service/integration.rs`

- [ ] **Step 1: Add failing auth tests**

Cover:

- superadmin login on platform domain lands on `/superadmin`
- superadmin session on tenant custom domain is rejected or redirected out
- tenant user on wrong custom domain is redirected to the correct active tenant domain
- tenant user without active custom domain falls back to platform-domain tenant landing

- [ ] **Step 2: Run focused tests and verify failure**

Run:

```bash
rtk cargo test auth_service
```

Expected: FAIL where auth/domain guard rules are incomplete.

- [ ] **Step 3: Implement minimal backend metadata support**

Ensure auth settings and auth response payloads expose enough platform-domain and tenant-domain metadata for frontend redirect decisions without extra fragile lookups.

- [ ] **Step 4: Implement login page redirect enforcement**

Update `src/routes/login/+page.svelte` so:

- superadmin is platform-domain only
- tenant user/custom-domain mismatch redirects correctly
- incorrect tenant domain access is rejected clearly

- [ ] **Step 5: Run full focused checks**

Run:

```bash
rtk cargo test auth_service
rtk npm run check
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
rtk git add \
  src/routes/login/+page.svelte \
  src-tauri/src/services/auth_service/dto.rs \
  src-tauri/src/services/auth_service/repository.rs \
  src-tauri/src/services/auth_service/mod.rs \
  src-tauri/src/services/auth_service/mapper.rs \
  src-tauri/src/services/auth_service/tests.rs \
  src-tauri/src/services/auth_service/integration.rs
rtk git commit -m "feat: harden auth domain redirect rules"
```

---

## Chunk 5: Final Verification And Rollout Safety

### Task 10: Run cross-layer verification

**Files:**
- No code changes unless checks fail

- [ ] **Step 1: Run focused frontend tests**

Run:

```bash
rtk npm test -- src/lib/utils/apiUrl.test.ts src/lib/utils/appBoot.test.ts src/lib/utils/tenantRouting.test.ts
```

Expected: PASS.

- [ ] **Step 2: Run backend focused tests**

Run:

```bash
rtk cargo test domain_resolver
rtk cargo test auth_service
rtk cargo test public::
```

Expected: PASS.

- [ ] **Step 3: Run app-wide checks**

Run:

```bash
rtk npm run check
```

Expected: PASS with `0 errors` and `0 warnings`.

- [ ] **Step 4: Manually verify these scenarios after deployment to a staging-like environment**

Check:

- platform domain login works
- superadmin only works on platform domain
- tenant on active custom domain loads `/admin` and `/dashboard`
- inactive custom domain does not resolve tenant data
- browser dev with local origin no longer hard-depends on a remote `VITE_API_URL`

- [ ] **Step 5: Commit if verification-only docs/notes changed**

If no files changed, skip commit.

---

## Review Note

Automated plan subagent review was not run because this Codex session is configured to spawn subagents only when explicitly requested by the user. The plan should be user-reviewed before execution.

Plan complete and saved to `docs/superpowers/plans/2026-05-10-domain-tenant-routing.md`. Ready to execute?
