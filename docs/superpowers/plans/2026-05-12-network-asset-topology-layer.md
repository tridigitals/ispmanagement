# Network Asset Topology Layer Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Menampilkan asset FTTH terpilih di topology map dengan icon per tipe, popup ringkas, toggle layer, dan legend ringan.

**Architecture:** Asset topology layer dibangun sebagai overlay frontend terpisah dari node/link topology yang sudah ada. Data asset diambil dari registry asset berkoordinat, difilter ke tipe `olt/odc/odp/fat/nap/switch`, ditransform ke marker model, lalu dirender di map dengan icon token per tipe dan popup ringkas.

**Tech Stack:** Svelte 5, TypeScript, MapLibre, existing `networkAssets` API + network map page

---

## Chunk 1: Helpers And Tests

### Task 1: Add asset map scope and marker helpers

**Files:**
- Create: `src/lib/components/network/networkMapAssets.ts`
- Create: `src/lib/components/network/networkMapAssets.test.ts`

- [ ] Write failing tests for asset scope filtering, icon mapping, and marker transformation.
- [ ] Run focused tests and confirm they fail first.
- [ ] Implement minimal helper code to pass tests.
- [ ] Re-run focused tests until green.

## Chunk 2: Map Page Integration

### Task 2: Load asset markers and render a topology asset overlay

**Files:**
- Modify: `src/routes/(app)/admin/network/map/+page.svelte`
- Optionally modify: `src/lib/components/network/networkMapInsights.ts` if search grouping needs asset entries now or later

- [ ] Add map page state for asset rows, visibility toggle, and selected asset popup state.
- [ ] Load asset registry rows and transform to map markers using the helper.
- [ ] Render asset markers with distinct per-type icons and lightweight popups.
- [ ] Add layer toggle and compact legend.
- [ ] Keep existing node/link/router behavior intact.

## Chunk 3: Final Polish And Verification

### Task 3: Validate UI and registry polish stays coherent

**Files:**
- Modify: `src/routes/(app)/admin/network/assets/+page.svelte` only if needed for minor alignment

- [ ] Run focused Vitest suites.
- [ ] Run `npm run check`.
- [ ] Run `cargo check --manifest-path src-tauri/Cargo.toml --lib --bin server`.
