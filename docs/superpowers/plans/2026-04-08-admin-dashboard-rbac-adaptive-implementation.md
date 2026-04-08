# Admin Dashboard RBAC Adaptive Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rework `/[tenant]/admin` into a permission-aware dashboard that shows adaptive summary cards, focus cards, and quick actions aligned to granular RBAC.

**Architecture:** Keep a single Svelte page for the admin dashboard, but replace its static card layout with a widget-based composition model driven by `$can(...)` capability checks. Reuse existing APIs where possible, fetch only the data needed for visible widget groups, and keep the visuals compact and role-first.

**Tech Stack:** SvelteKit, Svelte 5 runes, existing tenant auth/permission stores, existing API client, Tauri/Rust backend endpoints already protected by RBAC.

---

## File Map

### Primary implementation files
- Modify: `src/routes/[tenant]/(app)/admin/+page.svelte`
  - Replace the static owner/admin-only dashboard with adaptive widgets and grouped sections.
- Modify: `src/lib/i18n/locales/en.json`
  - Add strings for new dashboard section names, widget titles, hints, and empty states.

### Likely supporting files
- Modify: `src/lib/i18n/locales/id.json`
  - Keep locale parity for newly added dashboard strings if the repo expects mirrored translations.
- Inspect only: `src/lib/api/client.ts`
- Inspect only: `src/lib/api/payment.ts`
- Inspect only: `src/lib/components/layout/Sidebar.svelte`
- Inspect only: `src/routes/[tenant]/(app)/+layout.svelte`

### Verification files / touchpoints
- Verify permissions still align with:
  - `src-tauri/src/services/role_service.rs`
  - `src-tauri/src/http/*`
  - `src-tauri/src/commands/*`

---

## Chunk 1: Dashboard Capability Model

### Task 1: Capture current dashboard behavior in a focused refactor target

**Files:**
- Modify: `src/routes/[tenant]/(app)/admin/+page.svelte`

- [ ] **Step 1: Write the failing test mindset checkpoint**

Define expected behavior before changing code:
- Technician should not primarily see team/settings/subscription cards
- Owner/Admin should still see broad summary cards
- Widget visibility must depend on permission checks, not role names

- [ ] **Step 2: Identify capability groups in the page**

Create capability booleans in `+page.svelte`, for example:
- team access
- settings access
- customer access
- billing access
- work order access
- PPPoE access
- network operations access

- [ ] **Step 3: Refactor current static dashboard conditionals into explicit capabilities**

Move direct `$can(...)` checks out of markup where repeated and into top-level derived values.

- [ ] **Step 4: Verify page still compiles before widget expansion**

Run: `npm exec prettier -- --write 'src/routes/[tenant]/(app)/admin/+page.svelte'`

- [ ] **Step 5: Commit**

```bash
git add src/routes/[tenant]/(app)/admin/+page.svelte
git commit -m "refactor: prepare admin dashboard capability model"
```

## Chunk 2: Primary Stats and Focus Cards

### Task 2: Replace static summary cards with permission-aware primary stats

**Files:**
- Modify: `src/routes/[tenant]/(app)/admin/+page.svelte`

- [ ] **Step 1: Define a `primaryStats` builder**

Create a derived array of cards with:
- `title`
- `value`
- `icon`
- `tone`
- `href`
- `show`
- optional `meta`

- [ ] **Step 2: Add data-loading branches only for visible domains**

Reuse existing API calls and only fetch what visible cards need.
Examples:
- team count only if team widget visible
- settings count only if settings widget visible
- subscription details only if subscription widget visible
- customer totals / billing summaries / work-order summaries only if those widgets render

- [ ] **Step 3: Add a new `My Focus Today` section**

Render a second adaptive card row with smaller operational cards.
Examples by capability:
- work orders
- PPPoE
- billing follow-up
- incidents / alerts

- [ ] **Step 4: Add contextual empty states**

Examples:
- `No active work orders`
- `No invoice follow-up needed`
- `No active incidents`

- [ ] **Step 5: Verify visual hierarchy**

Check that:
- top row = high-level summary
- focus row = actionable operational emphasis

- [ ] **Step 6: Commit**

```bash
git add src/routes/[tenant]/(app)/admin/+page.svelte
git commit -m "feat: add adaptive admin stats and focus cards"
```

## Chunk 3: Quick Actions and Trend Cards

### Task 3: Rebuild quick actions around permissions and useful next steps

**Files:**
- Modify: `src/routes/[tenant]/(app)/admin/+page.svelte`

- [ ] **Step 1: Replace static quick action list with a filtered action registry**

Include only actionable destinations:
- team
- roles
- settings
- customers
- invoices
- support
- installations
- PPPoE
- NOC / incidents / alerts / routers

- [ ] **Step 2: Add one compact trend / distribution section**

Implement small, low-risk visual summaries such as:
- invoice status bar distribution
- work-order status strip
- incident severity strip

Use simple local rendering with existing CSS rather than introducing a heavy charting library.

- [ ] **Step 3: Gate trend rendering by permission**

Examples:
- billing strip only with `billing` access
- work-order strip only with work-order access
- incident strip only with incident/alert access

- [ ] **Step 4: Keep the dashboard concise**

If too many widgets are visible:
- cap cards per section
- order by operational priority

- [ ] **Step 5: Commit**

```bash
git add src/routes/[tenant]/(app)/admin/+page.svelte
git commit -m "feat: add permission-aware quick actions and trend cards"
```

## Chunk 4: Copy, i18n, and Polish

### Task 4: Add translations and polish adaptive states

**Files:**
- Modify: `src/lib/i18n/locales/en.json`
- Modify: `src/lib/i18n/locales/id.json`
- Modify: `src/routes/[tenant]/(app)/admin/+page.svelte`

- [ ] **Step 1: Add English strings for new dashboard sections**

Add keys for:
- primary stats labels
- focus card titles
- quick action descriptions
- trend labels
- empty states

- [ ] **Step 2: Add matching Indonesian strings**

Keep parity with `en.json` so the page remains clean in both locales.

- [ ] **Step 3: Polish loading and no-capability states**

Add:
- better loading copy
- fallback state if user only has one minimal permission cluster

- [ ] **Step 4: Commit**

```bash
git add src/lib/i18n/locales/en.json src/lib/i18n/locales/id.json src/routes/[tenant]/(app)/admin/+page.svelte
git commit -m "feat: polish admin dashboard copy and localization"
```

## Chunk 5: Verification

### Task 5: Verify RBAC alignment and build safety

**Files:**
- Inspect: `src/routes/[tenant]/(app)/admin/+page.svelte`
- Inspect: `src/lib/components/layout/Sidebar.svelte`
- Inspect: `src/routes/[tenant]/(app)/+layout.svelte`

- [ ] **Step 1: Format modified frontend files**

Run:

```bash
npm exec prettier -- --write 'src/routes/[tenant]/(app)/admin/+page.svelte' 'src/lib/i18n/locales/en.json' 'src/lib/i18n/locales/id.json'
```

- [ ] **Step 2: Run backend compile safety check**

Run:

```bash
CARGO_TARGET_DIR=/tmp/ispmgmt-rbac-check cargo check --manifest-path src-tauri/Cargo.toml
```

Expected:
- success
- only known existing warnings are acceptable

- [ ] **Step 3: Manual permission walkthrough**

Verify with representative roles:
- Owner/Admin sees broad dashboard
- Technician sees operations-focused dashboard
- NOC sees network-focused dashboard
- CS sees customer/billing/support-focused dashboard

- [ ] **Step 4: Commit final polish**

```bash
git add src/routes/[tenant]/(app)/admin/+page.svelte src/lib/i18n/locales/en.json src/lib/i18n/locales/id.json
git commit -m "feat: adapt admin dashboard to granular rbac"
```

---

Plan complete and saved to `docs/superpowers/plans/2026-04-08-admin-dashboard-rbac-adaptive-implementation.md`. Ready to execute?
