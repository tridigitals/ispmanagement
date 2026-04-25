# Duitku Payment Gateway Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Duitku as an online payment gateway alongside Midtrans and manual bank transfer.

**Architecture:** Reuse the existing `PaymentService` invoice lifecycle and checkout page. Backend creates a Duitku transaction and returns `paymentUrl`; frontend redirects the customer; Duitku callback verifies the MD5 signature before applying the same paid/failed transition path used by Midtrans.

**Tech Stack:** Rust/Tauri/Axum backend, Svelte checkout UI, SQL settings, reqwest external API client, Vitest/Cargo tests.

---

## Chunk 1: Backend Gateway Support

### Task 1: Duitku Service Helpers

**Files:**
- Modify: `src-tauri/src/services/payment_service/mod.rs`
- Modify: `src-tauri/src/services/payment_service/tests.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Write failing tests** for Duitku create/callback signatures and result-code status mapping.
- [ ] **Step 2: Run `cargo test --manifest-path src-tauri/Cargo.toml payment_service::tests::duitku --lib` and verify failure.**
- [ ] **Step 3: Implement helper functions and add `md5` dependency.**
- [ ] **Step 4: Run the same test and verify pass.**

### Task 2: Duitku HTTP/Command Flow

**Files:**
- Modify: `src-tauri/src/services/payment_service/mod.rs`
- Modify: `src-tauri/src/http/payment.rs`
- Modify: `src-tauri/src/commands/payment.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/api/core.ts`
- Modify: `src/lib/api/payment.ts`

- [ ] **Step 1: Add `initiate_duitku` returning a redirect URL.**
- [ ] **Step 2: Add `/payment/invoices/:id/duitku` and `/payment/duitku/callback`.**
- [ ] **Step 3: Add Tauri command and frontend API mapping.**
- [ ] **Step 4: Run focused Rust/TS checks.**

## Chunk 2: Settings And Checkout UI

### Task 3: Settings Surface

**Files:**
- Modify: `src-tauri/src/db/connection/seed.rs`
- Modify: `src-tauri/src/http/settings.rs`
- Modify: `src-tauri/src/commands/settings.rs`
- Modify: `src/lib/api/settings.ts`
- Modify: `src/routes/[tenant]/(app)/admin/settings/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/settings/SettingsPaymentTab.svelte`
- Modify: `src/routes/superadmin/settings/+page.svelte`

- [ ] **Step 1: Seed Duitku settings with disabled defaults.**
- [ ] **Step 2: Expose non-secret public Duitku enabled/production state.**
- [ ] **Step 3: Add admin settings fields for merchant code, API key, payment method, and production mode.**

### Task 4: Checkout Flow

**Files:**
- Modify: `src/routes/pay/[id]/+page.svelte`

- [ ] **Step 1: Show Duitku as an online provider when enabled.**
- [ ] **Step 2: Redirect browser to Duitku `paymentUrl` on pay.**
- [ ] **Step 3: Reuse existing status polling/check button.**

