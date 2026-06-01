# Network Topology Map Refactoring Plan (REVISED)

> **Revised 2026-06-01** after empirical verification. The original plan's "Phase 1 = CSS,
> lowest risk" premise was **wrong** for this Svelte setup. See "Verified Findings" below.
>
> **Progress 2026-06-01:** Phase A ✅ done, Phase B ✅ done. `+page.svelte` 4,313 → 3,831
> lines (−482, ~11%). 145 unit tests pass, svelte-check 0 errors. See "Execution Log" below.

## Current State
- **File:** `src/routes/(app)/admin/network/map/+page.svelte`
- **Total lines:** 3,831 (was 4,313 at plan start)
- **Original baseline:** 4,313 lines
  - Script: lines 1–2740 (~2,740 lines)
  - Template: lines 2742–3068 (~327 lines)
  - Style: lines 3070–4313 (~1,244 lines)
- **Inline functions:** 88
- **State variables:** 107 (many `$state(...)`)
- **Imports:** 44
- **`:global()` selectors in style block:** 142

## Verified Findings (tested, not assumed)

### ❌ Finding 1 — `<style src="./networkMap.css">` does NOT work here
- Setup uses `vitePreprocess()` (svelte.config.js) with `@sveltejs/vite-plugin-svelte@^5`.
- Confirmed in `node_modules/@sveltejs/vite-plugin-svelte/src/preprocess.js` (lines 68–86):
  the style preprocessor only reads `content`, **ignores the `src` attribute**.
- Result: external `src` CSS is silently dropped, not compiled. Do not use this approach.

### ❌ Finding 2 — `import './networkMap.css'` makes ALL styles GLOBAL
- Svelte `<style>` is auto-scoped (classes get `.svelte-xxx` hash) — no leakage today.
- A plain CSS import loses that scoping → styles become global.
- The map uses generic class names: `.btn`, `.btn.ghost`, `.btn.danger`, `.btn-xs`,
  `.page-content`, `.page-content.compact-mode`.
- **50+ other files** define those same class names with different styles
  (invoices, settings, dashboard, profile tabs, superadmin, etc.).
- Global import → map styles override buttons/layout across the entire app. HIGH RISK.

### ✅ Finding 3 — Modals are already extracted (Phase 2 mostly done)
- `LinkModalComponent`, `ZoneModalComponent`, `ConfirmDialogComponent` are lazy-loaded
  (`$state<Component | null>`, dynamically imported around lines 197–200, 544–551).
- `AssetFormModalComponent` (`NetworkAssetFormModalComponent`) lazy-loaded ~line 1353–1355.
- Their markup/styles already live in separate components. Remaining inline modal:
  `asset-customer-drop-modal` (template ~2969–3050) — still inline.

### ✅ Finding 4 — 18 `networkMap*.ts` helper modules already exist
- File already imports from networkMapData, networkMapInit, networkMapLayers,
  networkMapPopups, networkMapActions, networkMapCrud, networkMapDrafts,
  networkMapLinkPicking, networkMapAssets, networkMapAssetConnect, etc.
- A clear extraction pattern is already established + each has a `.test.ts`. Follow it.

## Revised Execution Order (safest → riskiest)

### Phase A — Extract large pure functions to TS (was Phase 3) — SAFEST ✅ DONE
- **Status:** ✅ Complete (2026-06-01). Result: −235 lines, not the estimated ~500.
- **Target module(s):** extend existing `networkMap*.ts` or add `networkMapRuntime.ts`.
- **Functions handled** (by their actual nature, not blindly extracted):
  - `buildTopologyAssetPopupHtml` (~112 lines) ✅ → extracted to `networkMapAssets.ts` as
    a pure function taking `{ row, buttonIds, canManageFtthAssets, translate }`. Helpers
    `escapeTopologyAssetPopupValue` + `popupToneForAssetStatus` moved with it (were inline
    duplicates). +13 new tests in `networkMapAssetPopup.test.ts` (incl. XSS escaping).
  - `replaceTopologyAssetOverlay` (~120 lines) ✅ → turned out to be a **verbatim duplicate**
    of `replaceTopologyAssetSourceData` already in `networkMapLayers.ts`. NOT re-extracted —
    collapsed to a 3-line guard wrapper that delegates to the module. Dead-code removal.
    Target fn already covered by 3 tests.
  - `refreshMapData` (~154 lines) ⛔ → **intentionally LEFT in component.** On reading, it's
    an orchestrator that reads/writes 25+ `$state` vars; heavy logic already delegated to
    helper modules. Forcing it out would mean passing 25+ state refs as params — more fragile,
    less readable. This kind of function belongs in the component.
- **Why safe:** logic only, no CSS scoping concerns; established pattern + vitest harness.
- **Caution learned:** the i18n translator type must match svelte-i18n's `MessageFormatter`
  (values are `InterpolationValues` primitives, not `Record<string, unknown>`). Caught by
  svelte-check, fixed.
- **Savings:** −235 lines (4,313 → 4,078). Lower than estimate because 1 fn stays in
  component and 1 was a duplicate (cleanup, not extraction).
- **Verify:** ✅ 145 tests pass, svelte-check 0 errors.

### Phase B — Finish modal extraction (was Phase 2) — LOW RISK ✅ DONE
- **Status:** ✅ Complete (2026-06-01). Result: −247 lines (4,078 → 3,831).
- Extracted remaining inline `asset-customer-drop-modal` → `NetworkMapAssetCustomerDropModal.svelte`
  (own scoped style). Props: `show`, `title`, `items`, `onClose`, `onView`.
- **Split done right:** markup + styles + `stateLabel` (presentation) moved into the
  component; `closeCustomerDropModalAndFocus` (map/state/navigation coupling) stays in the
  parent, passed as the `onView` callback. The component knows nothing about the map instance.
- **Pitfall hit + fixed (exactly as Finding 2 predicted):** the "View" button uses
  `class="btn ghost btn-xs"`. `global.css` has `.btn` and `.btn-ghost` (hyphen) but NOT the
  compound `.btn.ghost` or `.btn-xs` — those came from the parent's *scoped* `<style>`.
  Moving markup alone would have lost the button styling. Fix: replicated those 4 rules in
  the new component's `<style>` (same approach as the existing extracted modals), with a
  comment explaining why.
- **Savings:** −247 lines (template + migrated styles + `stateLabel`).
- **Verify:** ✅ 145 tests pass, svelte-check 0 errors, no new warnings in changed files.
- **Still pending:** visual smoke test (open modal in browser, dark mode + mobile). Low risk
  since markup/styles are byte-identical, but not yet done.

### Phase C — CSS extraction with NAMESPACE WRAPPER (was Phase 1) — MEDIUM RISK, DO LAST
- **Do NOT** use `<style src>` (Finding 1) or bare global `import` (Finding 2).
- **Safe approach:**
  1. Add a unique root class on the page wrapper:
     `<div class="page-content fade-in network-map-root" ...>`.
  2. Move the `<style>` content into `networkMap.css`.
  3. The 142 `:global(...)` selectors are already global → move as-is (safe).
  4. Every **scoped** generic selector must be prefixed/namespaced under the root:
     `.btn` → `.network-map-root .btn`, `.page-content` → `.network-map-root.page-content`, etc.
  5. Import the namespaced CSS. Verify nothing leaks to other pages.
- **Alternative (lower effort, keeps scoping):** leave CSS in the `.svelte` `<style>`
  block as-is. It's the only place Svelte scoping is free. Line count in `.svelte` is
  cosmetic — the real maintainability win is Phases A/B. Consider whether CSS extraction
  is worth the namespacing risk at all.
- **Savings:** ~1,200 lines moved out of `.svelte` (cosmetic, not behavioral).
- **Verify:** manual visual check — light AND dark mode, desktop AND mobile (user tests
  on mobile + dark mode). Diff button/layout styling on invoices/settings/dashboard.

### Phase D — State consolidation (was Phase 4) — HIGHEST COMPLEXITY, OPTIONAL
- 107 `$state` vars. Group by feature into typed state holders if it improves clarity.
- High risk of reactivity regressions in Svelte 5 runes. Only attempt if Phases A–C
  leave clear seams. May not be worth it.

## Per-Phase Verification (every phase)
- [ ] `npm run check` (svelte-check) passes — no new type errors.
- [ ] `npm run test:unit` passes (extend tests for extracted logic).
- [ ] Map renders; pan/zoom/click/drag work; all modals open + submit.
- [ ] No new console errors.
- [ ] (Phase C only) Visual regression check on light/dark + mobile, and on other pages
      that share `.btn`/`.page-content`.

## Notes
- Preserve all existing functionality and the lazy-loading pattern for modal components.
- Keep extracted helpers pure where possible; keep `$state` mutation in the component.
- Line-count reduction in the `.svelte` file is a cosmetic goal; prioritize real
  maintainability wins (pure logic extraction, tested modules) over raw line savings.

## Execution Log

### 2026-06-01 — Phase A ✅
- Extracted `buildTopologyAssetPopupHtml` + `escapeTopologyAssetPopupValue` +
  `popupToneForAssetStatus` → `networkMapAssets.ts` (pure, param-injected translator).
- New: `networkMapAssetPopup.test.ts` (13 tests incl. XSS escaping).
- Collapsed `replaceTopologyAssetOverlay` (duplicate) → wrapper over
  `replaceTopologyAssetSourceData` in `networkMapLayers.ts`.
- Decided `refreshMapData` stays in component (orchestrator, 25+ `$state` deps).
- Result: 4,313 → 4,078 lines. 145 tests pass, svelte-check 0 errors.

### 2026-06-01 — Phase B ✅
- New component: `NetworkMapAssetCustomerDropModal.svelte` (markup + scoped styles +
  `stateLabel`). Parent keeps `closeCustomerDropModalAndFocus`, passes as `onView`.
- Replicated `.btn`/`.btn.ghost`/`.btn:disabled`/`.btn-xs` locally (not in global.css).
- Result: 4,078 → 3,831 lines. 145 tests pass, svelte-check 0 errors.

### Cumulative
- `+page.svelte`: 4,313 → 3,831 lines (−482, ~11%). No regressions.
- Not yet committed (working on `master`, was 7 commits ahead at start).
- Visual smoke test (dark mode + mobile) still pending for Phase B modal.

## Next Up
- Optional: visual smoke test of the customer-drop modal (dark mode + mobile).
- Phase C (CSS) — only if line reduction in `.svelte` is genuinely wanted; the namespacing
  effort + risk may not be worth it (see Phase C "Alternative").
- Commit checkpoint recommended before starting Phase C.
