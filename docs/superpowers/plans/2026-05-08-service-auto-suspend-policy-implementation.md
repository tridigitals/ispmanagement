# Service Auto Suspend Policy Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a dedicated `Service` settings tab, support global auto-suspend mode selection (`grace_period` or `fixed_day`), and expose computed service lifecycle policy details in customer subscriptions.

**Architecture:** Keep existing settings keys and billing collection flow intact, then extend them with two new global settings and a small shared frontend derivation layer for preview labels/dates. Frontend settings UI is split so `Payments` handles gateway configuration and `Service` handles customer service lifecycle policy.

**Tech Stack:** SvelteKit, Svelte 5, TypeScript, Rust, sqlx, Vitest, cargo test

---

## Chunk 1: Backend Billing Policy

### Task 1: Add failing Rust tests for billing settings resolution and suspend date helpers

**Files:**
- Modify: `src-tauri/src/services/payment_service/tests.rs`
- Modify: `src-tauri/src/services/payment_service/mod.rs`

- [ ] **Step 1: Write failing tests for new billing mode defaults and fixed-day clamping**
- [ ] **Step 2: Run targeted Rust test command and verify failure**
Run: `rtk cargo test payment_service::tests -- --nocapture`
- [ ] **Step 3: Implement minimal backend support for new setting keys and helper logic**
- [ ] **Step 4: Re-run targeted Rust test command and verify pass**
Run: `rtk cargo test payment_service::tests -- --nocapture`

### Task 2: Wire billing collection settings to new mode fields

**Files:**
- Modify: `src-tauri/src/services/payment_service/mod.rs`
- Modify: `src-tauri/src/db/connection/seed.rs`

- [ ] **Step 1: Extend `BillingCollectionSettings` with mode and fixed-day fields**
- [ ] **Step 2: Read global or tenant overrides defensively and clamp fixed day to `1..28`**
- [ ] **Step 3: Preserve existing grace-period suspend behavior while preparing fixed-day scheduling path**
- [ ] **Step 4: Re-run Rust tests**
Run: `rtk cargo test payment_service::tests -- --nocapture`

## Chunk 2: Admin Settings UI

### Task 3: Add failing frontend tests for the new settings module layout

**Files:**
- Modify: `src/routes/(app)/admin/settings/adminSettingsPageModules.test.ts`
- Create: `src/routes/(app)/admin/settings/SettingsServiceTab.test.ts`

- [ ] **Step 1: Write failing tests for `Service` tab loader and service policy rendering**
- [ ] **Step 2: Run targeted Vitest commands and verify failure**
Run: `rtk npm test -- adminSettingsPageModules.test.ts SettingsServiceTab.test.ts`
- [ ] **Step 3: Implement the new deferred module and service tab component**
- [ ] **Step 4: Re-run targeted Vitest commands and verify pass**
Run: `rtk npm test -- adminSettingsPageModules.test.ts SettingsServiceTab.test.ts`

### Task 4: Move lifecycle settings out of Payments and into Service

**Files:**
- Modify: `src/routes/(app)/admin/settings/+page.svelte`
- Modify: `src/routes/(app)/admin/settings/adminSettingsPageModules.ts`
- Modify: `src/routes/(app)/admin/settings/SettingsPaymentTab.svelte`
- Create: `src/routes/(app)/admin/settings/SettingsServiceTab.svelte`
- Modify: `src/lib/i18n/namespaces/en/admin.json`
- Modify: `src/lib/i18n/namespaces/id/admin.json`

- [ ] **Step 1: Add `service` category and deferred tab loading to settings page**
- [ ] **Step 2: Remove customer service lifecycle controls from `Payments`**
- [ ] **Step 3: Add `Service` tab UI with conditional mode inputs and helper copy**
- [ ] **Step 4: Re-run targeted Vitest commands**
Run: `rtk npm test -- adminSettingsPageModules.test.ts SettingsServiceTab.test.ts`

## Chunk 3: Customer Detail Observability

### Task 5: Add failing frontend tests for subscription policy preview helpers

**Files:**
- Create: `src/lib/utils/customerSubscriptionPolicy.test.ts`
- Create: `src/lib/utils/customerSubscriptionPolicy.ts`

- [ ] **Step 1: Write failing tests for labels, fallback states, and preview date calculation**
- [ ] **Step 2: Run targeted Vitest command and verify failure**
Run: `rtk npm test -- customerSubscriptionPolicy.test.ts`
- [ ] **Step 3: Implement minimal policy helper**
- [ ] **Step 4: Re-run targeted Vitest command and verify pass**
Run: `rtk npm test -- customerSubscriptionPolicy.test.ts`

### Task 6: Surface policy details in customer subscriptions

**Files:**
- Modify: `src/routes/(app)/admin/customers/[id]/+page.svelte`
- Modify: `src/routes/(app)/admin/customers/[id]/CustomerSubscriptionsTab.svelte`

- [ ] **Step 1: Load settings-backed policy inputs into the customer detail page**
- [ ] **Step 2: Add subscription columns or cell content for active-until, policy, and estimated suspend**
- [ ] **Step 3: Re-run targeted customer detail and helper tests**
Run: `rtk npm test -- customerSubscriptionPolicy.test.ts customerDetailModules.test.ts customerDetailTabModules.test.ts`

## Chunk 4: Final Verification

### Task 7: Run full relevant verification for touched areas

**Files:**
- Modify: `docs/superpowers/plans/2026-05-08-service-auto-suspend-policy-implementation.md`

- [ ] **Step 1: Run Rust verification**
Run: `rtk cargo test payment_service::tests -- --nocapture`
- [ ] **Step 2: Run frontend verification**
Run: `rtk npm test -- adminSettingsPageModules.test.ts SettingsServiceTab.test.ts customerSubscriptionPolicy.test.ts customerDetailModules.test.ts customerDetailTabModules.test.ts`
- [ ] **Step 3: If available and fast enough, run lint for touched TS/Svelte files**
Run: `rtk npm run check`
