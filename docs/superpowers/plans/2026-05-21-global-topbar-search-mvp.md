# Global Topbar Search MVP Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a provider-based global search in the shared topbar that shows RBAC-filtered results across key admin and superadmin resources.

**Architecture:** Keep the search UI attached to the existing topbar input, but move behavior into a global search store plus a provider registry. Each provider is enabled only when the current user has the matching permission, and it queries existing APIs that already enforce token and permission checks.

**Tech Stack:** Svelte 5 runes, Svelte stores, existing `$lib/api/*` wrappers, Vitest.

---

## Chunk 1: Core Search Model

### Task 1: Define shared search contracts

**Files:**
- Create: `src/lib/search/globalSearchModel.ts`
- Test: `src/lib/search/globalSearchModel.test.ts`

- [ ] **Step 1: Write the failing test**
- [ ] **Step 2: Run test to verify it fails**
- [ ] **Step 3: Write minimal implementation for result/provider/store-facing types**
- [ ] **Step 4: Run test to verify it passes**

### Task 2: Define RBAC-aware provider registry helpers

**Files:**
- Create: `src/lib/search/globalSearchProviders.ts`
- Test: `src/lib/search/globalSearchProviders.test.ts`

- [ ] **Step 1: Write the failing test for provider visibility and scope ordering**
- [ ] **Step 2: Run test to verify it fails**
- [ ] **Step 3: Implement minimal provider registry helpers**
- [ ] **Step 4: Run test to verify it passes**

## Chunk 2: Search Execution

### Task 3: Add topbar search orchestrator

**Files:**
- Create: `src/lib/search/globalSearchService.ts`
- Test: `src/lib/search/globalSearchService.test.ts`

- [ ] **Step 1: Write the failing test for fan-out search, trimming, dedupe, and per-provider limits**
- [ ] **Step 2: Run test to verify it fails**
- [ ] **Step 3: Implement minimal orchestrator using existing APIs**
- [ ] **Step 4: Run test to verify it passes**

### Task 4: Add global search store state

**Files:**
- Create: `src/lib/stores/globalSearch.ts`
- Test: `src/lib/stores/globalSearch.test.ts`

- [ ] **Step 1: Write the failing test for open/close/query/loading/result state**
- [ ] **Step 2: Run test to verify it fails**
- [ ] **Step 3: Implement minimal store actions**
- [ ] **Step 4: Run test to verify it passes**

## Chunk 3: UI Wiring

### Task 5: Build the topbar results panel

**Files:**
- Create: `src/lib/components/layout/TopbarGlobalSearchPanel.svelte`
- Modify: `src/lib/components/layout/Topbar.svelte`
- Test: `src/routes/admin-network-map-topbar-search-ui.test.ts`

- [ ] **Step 1: Write or update the failing UI-source test for topbar search wiring**
- [ ] **Step 2: Run test to verify it fails**
- [ ] **Step 3: Implement the panel and connect the shared topbar input to the global search store**
- [ ] **Step 4: Run test to verify it passes**

### Task 6: Add keyboard and navigation wiring

**Files:**
- Modify: `src/lib/components/layout/Topbar.svelte`
- Modify: `src/routes/(app)/+layout.svelte`
- Modify: `src/routes/superadmin/+layout.svelte`

- [ ] **Step 1: Add keyboard open/close handling and safe navigation behavior**
- [ ] **Step 2: Verify RBAC-sensitive pages still behave normally**

## Chunk 4: Verification

### Task 7: Run focused verification

**Files:**
- Test: `src/lib/search/*.test.ts`
- Test: `src/lib/stores/globalSearch.test.ts`
- Test: `src/routes/admin-network-map-topbar-search-ui.test.ts`

- [ ] **Step 1: Run targeted Vitest suites**
- [ ] **Step 2: Run `svelte-check` if the changed surface is stable enough**
- [ ] **Step 3: Summarize remaining gaps or deferred providers**
