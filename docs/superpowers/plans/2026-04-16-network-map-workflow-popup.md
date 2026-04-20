# Network Map Workflow Popup Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Workflow First customer/service popups on the network map with `Open Customer` as the primary action.

**Architecture:** Extend the existing pure popup model in `networkMapUtils.ts`, keep HTML rendering in `networkMapInteractionUtils.ts`, and wire navigation from `networkMapPopups.ts` into the Svelte map page. Avoid a new popup component because current popups are MapLibre HTML strings.

**Tech Stack:** Svelte 5, TypeScript, Vitest, MapLibre popup HTML.

---

### Task 1: Popup Model

**Files:**
- Modify: `src/lib/components/network/networkMapUtils.ts`
- Test: `src/lib/components/network/networkMapUtils.test.ts`

- [ ] Add `open-customer` and optional `open-service` popup action keys.
- [ ] Update service popup actions so `Open Customer` is first when metadata has `customer_id`.
- [ ] Add summary/detail data that supports Workflow First quick facts.
- [ ] Run `npm run test:unit -- src/lib/components/network/networkMapUtils.test.ts`.

### Task 2: Popup Action Wiring

**Files:**
- Modify: `src/lib/components/network/networkMapPopups.ts`
- Modify: `src/routes/[tenant]/(app)/admin/network/map/+page.svelte`

- [ ] Add `onOpenCustomer(customerId)` to node popup arguments.
- [ ] Dispatch `open-customer` clicks to that callback using node metadata.
- [ ] Route to `${tenantPrefix}/admin/customers/${customerId}`.

### Task 3: Popup Visual Polish

**Files:**
- Modify: `src/lib/components/network/networkMapInteractionUtils.ts`
- Modify: `src/routes/[tenant]/(app)/admin/network/map/+page.svelte`

- [ ] Add semantic CSS classes for Workflow First service popup content.
- [ ] Make the context strip, quick facts, and action row more readable.
- [ ] Keep existing router/link/node popups compatible.
- [ ] Run `npm run check`.
