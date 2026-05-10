# Domain Tenant Routing And Custom Domain Design

## Summary

The product should use host-based tenant resolution as the production source of truth. Platform traffic stays on one canonical control-plane domain, while tenant traffic can run on either platform subdomains or verified tenant custom domains. Web clients should call the API through the current origin using relative `/api` paths so custom-domain traffic does not depend on a hardcoded remote API host. Desktop/Tauri clients may continue using an explicit API base override.

This design keeps the current multi-tenant application model, but tightens the boundaries between:

- platform domain behavior
- tenant domain behavior
- backend tenant resolution
- auth/session routing
- custom domain lifecycle and verification

## Current State

The codebase already contains the main building blocks:

- `tenants.custom_domain` exists and is unique.
- Public APIs already expose tenant lookup by domain.
- `src/routes/+layout.svelte` already attempts custom-domain lookup and caches the mapping.
- Backend public handlers already validate some flows against the request host.
- Backend HTTP bootstrap already refreshes custom domains into CORS-related runtime state.
- Auth responses already include tenant slug and optional tenant custom domain.

The biggest architectural mismatch today is API base URL behavior in the web client. `getApiBaseUrl()` currently prioritizes `VITE_API_URL` when present, which is safe for desktop or explicitly remote deployments, but is fragile for browser-based multi-domain production because it couples all browser requests to one configured origin. That creates CORS pressure and weakens custom-domain behavior.

## Goals

- Make tenant resolution authoritative on the backend from the request host.
- Support platform domain, platform subdomain, and tenant custom domain cleanly.
- Remove browser dependence on a hardcoded remote API host.
- Keep Tauri/native flexibility with explicit API URL overrides.
- Isolate superadmin/control-plane behavior to the platform domain.
- Require custom-domain verification before activation.
- Ensure auth/session flows cannot silently cross tenants or domains.
- Preserve the existing SPA routing direction and existing tenant/domain foundations where practical.

## Non-Goals

- Do not split the product into per-tenant app deployments.
- Do not redesign the core tenant data model beyond what is needed for domain lifecycle.
- Do not move all route trees again as part of this work.
- Do not redesign RBAC itself.
- Do not introduce wildcard trust of arbitrary domains.

## Domain Model

The system should recognize three domain modes:

1. Platform control-plane domain
   - Example: `app.isp.com`
   - Used for superadmin, tenant onboarding, platform billing, and global operations.

2. Platform tenant subdomain
   - Example: `acme.app.isp.com`
   - Optional tenant-facing domain managed under the platform namespace.

3. Tenant custom domain
   - Example: `portal.acme.net`
   - Tenant-owned branded domain for admin/customer access.

Backend request context should resolve one of:

- `Platform`
- `TenantSubdomain { tenant_id, tenant_slug }`
- `TenantCustomDomain { tenant_id, tenant_slug, custom_domain }`

Frontend storage may cache domain-to-slug mappings for UX, but it must not be the security source of truth.

## Canonical Routing Rules

### Platform Domain

- Superadmin routes are allowed only on the platform domain.
- Platform domain may host tenant app routes only when the current session and redirect policy explicitly allow it.
- Platform domain remains the fallback when a tenant custom domain is not yet active.

### Tenant Domains

- Tenant admin and customer-facing routes should be clean root app paths such as:
  - `/admin`
  - `/dashboard`
  - `/profile`
  - `/support`
  - `/notifications`
  - `/announcements`
- The tenant is derived from host first, not from a client slug.

### Redirect Policy

- A tenant-authenticated user on the wrong tenant domain should be redirected to the correct active tenant domain when possible.
- A superadmin session should not remain active on a tenant custom domain.
- A disabled or unverified tenant custom domain should not resolve to tenant data.

## API Base URL Strategy

### Browser Web Clients

Web clients should default to relative API access:

- `window.location.origin + /api`
- or simply `/api`

This means:

- `https://app.isp.com` calls `https://app.isp.com/api`
- `https://portal.acme.net` calls `https://portal.acme.net/api`

Benefits:

- avoids cross-origin browser dependency for normal production traffic
- reduces CORS complexity
- keeps custom domains first-class
- better matches reverse-proxy/CDN production layouts

### Tauri / Native Clients

Desktop/Tauri clients may keep an explicit API base override through environment configuration because they are not bound to browser-origin rules in the same way.

### Proposed Rule For `getApiBaseUrl()`

- If runtime is Tauri/native and explicit API URL is configured, use it.
- If runtime is standard browser web, prefer relative `/api`.
- Only use explicit configured browser API base in clearly intentional special modes, not as the default production behavior.

## Backend Tenant Resolution

Backend should centralize tenant resolution into one shared resolver instead of duplicating direct `custom_domain` lookups in many handlers.

### Resolver Inputs

- trusted host value:
  - `X-Forwarded-Host` only from known reverse proxies
  - otherwise standard `Host`
- optional port stripping
- lowercase normalization
- protection against empty/invalid host values

### Resolver Outputs

- domain mode
- resolved tenant metadata if applicable
- whether the host is active and verified

### Resolver Responsibilities

- detect platform domain
- detect tenant platform subdomain
- detect tenant custom domain
- reject unknown or inactive domains
- avoid mixing hostname parsing logic across unrelated handlers

Public and tenant-sensitive handlers should consume the shared resolver instead of each doing raw domain SQL.

## Custom Domain Lifecycle

`tenants.custom_domain` alone is not enough as the operational state model. The system should track lifecycle state explicitly.

### Proposed States

- `pending`
- `dns_verified`
- `ssl_ready`
- `active`
- `disabled`
- `failed`

### Proposed Domain Record Fields

This can be a separate table or an expanded tenant-domain model:

- `tenant_id`
- `domain`
- `type` (`platform_subdomain` or `custom_domain`)
- `status`
- `dns_last_checked_at`
- `dns_target_value`
- `ssl_status`
- `verified_at`
- `activated_at`
- `disabled_at`
- `failure_reason`

If the team wants the smallest initial change, the status model can be introduced first while keeping `custom_domain` on tenants for backward compatibility, then later extracted into a full tenant-domain table.

## Domain Verification Flow

1. Tenant enters a custom domain.
2. System stores it in normalized form with `pending` status.
3. UI shows DNS instructions.
4. User triggers verification.
5. Backend validates:
   - normalized domain format
   - uniqueness
   - DNS target match
   - tenant eligibility/plan feature access
6. If valid:
   - mark `dns_verified`
   - trigger SSL issuance
7. When SSL is ready:
   - mark `ssl_ready`
8. When fully usable:
   - mark `active`
   - include domain in runtime allowlists/cache

Only `active` domains should serve tenant traffic.

## Auth And Session Rules

### Tenant Users

- Tenant users should only access tenant data for the resolved tenant context.
- Login success should evaluate:
  - current host
  - user tenant
  - tenant active domain availability
- If host and tenant do not match:
  - redirect to the correct tenant domain if active
  - otherwise redirect to the platform fallback

### Superadmin Users

- Superadmin login should be valid only on the platform domain.
- If a superadmin session appears on a tenant custom domain, the app should move the user back to the platform domain or block the session there.

### Session Storage

- Browser storage can keep convenience values like `active_tenant_slug`, but these values must be treated as hints only.
- Cookie/session scope should not be shared loosely across arbitrary tenant custom domains.
- Host-specific browser auth state is safer than trying to share one session across many tenant-owned domains.

## CORS And Reverse Proxy Rules

### For Browser Traffic

If web uses relative `/api` on the same origin, most browser API traffic becomes same-origin and CORS is greatly simplified.

### For Dynamic Domains

When CORS is still needed for special cases:

- allow only exact active domains
- never allow arbitrary wildcard tenant origins
- refresh allowlists from verified active domain records

### Reverse Proxy

Production proxy/CDN should:

- terminate TLS for platform and active tenant domains
- forward trusted host headers consistently
- route both app pages and `/api` to the same application stack

This is what makes relative `/api` viable and clean for custom domains.

## Frontend Responsibilities

- Detect host context, but do not make host context authoritative for security.
- Avoid hardcoded remote browser API URLs.
- Keep tenant-domain cache only as UX acceleration.
- On app boot:
  - detect host
  - ask backend/public lookup when needed
  - cache mapping only after successful resolution
- Redirect tenant users to the correct domain when backend-authenticated tenant identity does not match the current host.

## Backend Responsibilities

- Resolve tenant context from host on every tenant-sensitive request path where domain matters.
- Enforce domain activation and tenant activation.
- Restrict custom-domain-only public flows to valid tenant domains.
- Restrict superadmin/control-plane flows to the platform domain.
- Record host/domain context in audit logging for sensitive operations.

## Migration Plan

### Phase 1: API Base URL Hardening

- Change browser API base behavior to prefer relative `/api`.
- Keep Tauri explicit override support intact.
- Verify existing dev and production assumptions.

### Phase 2: Shared Backend Domain Resolver

- Introduce one reusable resolver for platform/subdomain/custom-domain detection.
- Migrate public domain-sensitive handlers to use it.

### Phase 3: Custom Domain Lifecycle

- Add explicit domain lifecycle status.
- Add verification and activation flow.
- Gate routing/CORS on active verified domains only.

### Phase 4: Auth Redirect Hardening

- Enforce platform-only superadmin behavior.
- Enforce tenant-domain correctness after login/boot.
- Stop relying on client slug state as authority.

### Phase 5: Operational UX

- Expose tenant domain status, verification actions, and failure reasons in admin UI.
- Add observability around domain verification, activation, and request resolution.

## Risks

- Relative `/api` requires production proxy routing to be correct on every served domain.
- Existing dev habits using remote API origins may need adjustment.
- Cached client-side tenant-domain mappings may hide bugs if not carefully downgraded to hint-only behavior.
- Platform subdomain conventions and custom domain ownership must not conflict.

## Acceptance Criteria

- Browser production traffic works on platform and tenant custom domains without depending on a hardcoded remote API base URL.
- Backend tenant resolution is centralized and host-based.
- Only active verified tenant domains can resolve tenant traffic.
- Superadmin access is isolated to the platform domain.
- Tenant login and boot flows redirect to the correct tenant domain when needed.
- Public tenant-domain registration/invite flows validate the current host against an active tenant domain.
- CORS/domain allowlists are based only on active verified domains.

## Review Note

Automated spec subagent review was not run because this Codex session is configured to spawn subagents only when explicitly requested by the user. This document should be user-reviewed before implementation.
