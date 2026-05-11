# Superadmin Managed RADIUS Filter Toolbar Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the always-open filter rows on the superadmin Managed RADIUS page with a compact search-first toolbar and inline advanced filter panel per tab.

**Architecture:** Keep filtering logic in the page, but extract the repetitive toolbar shell into a focused local component under the superadmin radius area. The page remains the source of truth for tab-specific filter state, active-count calculation, and reset behavior.

**Tech Stack:** Svelte 5, existing UI primitives (`Icon`), current page-local filtering logic, `svelte-check`

---

## Chunk 1: Documents

### Task 1: Save the approved design

**Files:**
- Create: `docs/superpowers/specs/2026-05-11-superadmin-radius-filter-toolbar-design.md`

- [ ] **Step 1: Write the approved design**
- [ ] **Step 2: Save the spec file**

## Chunk 2: Toolbar Component

### Task 2: Build a compact toolbar shell for the page

**Files:**
- Create: `src/lib/components/superadmin/radius/ManagedRadiusFilterToolbar.svelte`

- [ ] **Step 1: Define toolbar props for title, count, search binding, primary filter, active filter count, panel visibility, snippets, and actions**
- [ ] **Step 2: Implement main toolbar row with search, primary select, filter toggle, and actions slot**
- [ ] **Step 3: Implement inline advanced filter panel and mobile stacking styles**

## Chunk 3: Page Integration

### Task 3: Replace the per-tab raw filter rows

**Files:**
- Modify: `src/routes/superadmin/radius/+page.svelte`

- [ ] **Step 1: Add imports and per-tab advanced-panel open state**
- [ ] **Step 2: Add helpers for active advanced-filter counts**
- [ ] **Step 3: Add helpers to reset filters by tab**
- [ ] **Step 4: Replace each tab header filter markup with the new toolbar component**
- [ ] **Step 5: Keep existing filter behavior and empty states intact**

## Chunk 4: Verification

### Task 4: Verify the frontend

**Files:**
- Test: `src/routes/superadmin/radius/+page.svelte`
- Test: `src/lib/components/superadmin/radius/ManagedRadiusFilterToolbar.svelte`

- [ ] **Step 1: Run `rtk timeout 120s npm run check`**
- [ ] **Step 2: Fix any Svelte or TypeScript issues**
- [ ] **Step 3: Re-run `rtk timeout 120s npm run check` until clean**
