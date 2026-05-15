# Network Assets Modal Map Picker Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Menambahkan koordinat asset yang berdiri sendiri dan memoles modal `/admin/network/assets` dengan map picker yang rapi dan operasional.

**Architecture:** Koordinat asset ditambahkan sebagai field inti `latitude` dan `longitude` di backend `network_assets`, diteruskan ke API types/frontend state, lalu dipakai oleh modal asset dua kolom yang me-reuse `MapCanvasShell`. Pola validasi koordinat mengikuti flow router/order yang sudah ada, dan UI tetap ringan dengan section map yang jelas.

**Tech Stack:** Rust + SQLx, Svelte 5, TypeScript, Vitest, MapLibre via `MapCanvasShell`

---

## Chunk 1: Asset Coordinate Data Model

### Task 1: Add schema and model support

**Files:**
- Create: `src-tauri/migrations/20260512100000_add_network_asset_coordinates.up.sql`
- Create: `src-tauri/migrations/20260512100000_add_network_asset_coordinates.down.sql`
- Modify: `src-tauri/src/models/network_asset.rs`
- Modify: `src-tauri/src/services/network_asset_service.rs`
- Modify: `src/lib/api/types.ts`
- Modify: `src/lib/api/networkAssets.test.ts`

- [ ] Write failing API/backend-facing tests for coordinate payload fields and validation coverage.
- [ ] Run focused tests to verify they fail for missing `latitude/longitude` support.
- [ ] Add DB columns, Rust model fields, DTO fields, list/select queries, and coordinate validation.
- [ ] Update TS API types and wrapper tests for create/update payloads carrying coordinates.
- [ ] Re-run focused tests until green.

## Chunk 2: Frontend Asset State And Helpers

### Task 2: Add asset coordinate draft/state helpers

**Files:**
- Modify: `src/routes/(app)/admin/network/assets/+page.svelte`
- Modify: `src/routes/(app)/admin/network/assets/networkAssetsPageState.ts`
- Modify: `src/routes/(app)/admin/network/assets/networkAssetsPageState.test.ts`
- Create or Modify: helper file under `src/routes/(app)/admin/network/assets/` if coordinate parsing/copy logic needs extraction

- [ ] Write failing tests for coordinate parsing/copy behavior if extracted helper is added.
- [ ] Add `formLatitude/formLongitude` equivalent state into asset draft handling.
- [ ] Prefill edit flow from asset coordinates.
- [ ] Add `Use Customer Location` behavior that copies chosen customer location coordinates when available.
- [ ] Verify tests pass.

## Chunk 3: Modal Polish And Map Picker UI

### Task 3: Rebuild the modal into a two-column sectioned layout

**Files:**
- Modify: `src/routes/(app)/admin/network/assets/NetworkAssetFormModal.svelte`
- Reference: `src/routes/(app)/admin/network/routers/RouterFormModal.svelte`
- Reference: `src/routes/(app)/admin/customers/orders/new/+page.svelte`

- [ ] Write/adjust failing UI-related tests only if existing coverage supports this component path; otherwise rely on `svelte-check` and focused helper tests.
- [ ] Refactor modal layout into sections: identity, detail/capacity, operational links, notes.
- [ ] Add right-side map location section with coordinate fields and actions.
- [ ] Reuse `MapCanvasShell` picker flow for `Pick on Map`.
- [ ] Add `Clear Point` and lightweight contextual guidance by asset type.
- [ ] Verify with `svelte-check`.

## Chunk 4: Registry Surface And Final Verification

### Task 4: Show coordinate context in asset registry and finish verification

**Files:**
- Modify: `src/routes/(app)/admin/network/assets/+page.svelte`
- Modify: any related formatting helper only if needed

- [ ] Show coordinate summary lightly in registry rows when available.
- [ ] Keep ODP occupancy summary behavior intact.
- [ ] Run focused Vitest suites.
- [ ] Run `npm run check`.
- [ ] Run `cargo check --manifest-path src-tauri/Cargo.toml --lib --bin server`.
