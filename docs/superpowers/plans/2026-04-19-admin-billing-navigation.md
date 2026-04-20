# Admin Billing Navigation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganize admin navigation so customer billing is the Billing menu and tenant plan/subscription lives under Settings.

**Architecture:** Update sidebar IA, add a Settings category entry, keep the existing tenant subscription page as a safe detail route. Avoid moving the 1k-line subscription page in this change.

**Tech Stack:** Svelte 5, SvelteKit routes, TypeScript, svelte-i18n.

---

### Task 1: Sidebar IA

**Files:**
- Modify: `src/lib/components/layout/Sidebar.svelte`

- [ ] Remove `Subscription` from the `Billing` sidebar section.
- [ ] Rename customer invoices entry to `Billing`.
- [ ] Rename `Billing Logs` to `Collections`.

### Task 2: Settings Billing & Plan Category

**Files:**
- Modify: `src/routes/[tenant]/(app)/admin/settings/+page.svelte`

- [ ] Add `billing_plan` category with `credit-card` icon.
- [ ] Render a card explaining tenant platform billing and link to `/admin/subscription`.
- [ ] Update the existing custom-domain upgrade button to open Settings `Billing & Plan`.

### Task 3: Verification

**Commands:**
- `npm run check`

- [ ] Confirm Svelte diagnostics pass.
