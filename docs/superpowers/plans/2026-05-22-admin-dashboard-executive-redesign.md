# Admin Dashboard Executive Redesign Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Memoles halaman `/admin` menjadi dashboard dark-first yang lebih profesional, lebih lapang, dan terasa production-ready tanpa mengubah data source atau alur klik.

**Architecture:** Perubahan dibatasi pada presentasi di satu route Svelte. Struktur section dipertahankan secara fungsional, tetapi hierarchy visual, spacing, cards, dan trend presentation ditata ulang agar lebih executive dan lebih matang.

**Tech Stack:** Svelte 5, TypeScript, route-local CSS, Vitest source guardrails

---

## Chunk 1: Guardrails

### Task 1: Add failing tests for the redesigned admin dashboard shell

**Files:**
- Create or Modify: `src/routes/admin-dashboard-ui.test.ts`

- [ ] Add source-level expectations for the new executive shell hooks.
- [ ] Run the focused Vitest suite and confirm it fails first.
- [ ] Keep the assertions aligned with the approved design direction.

## Chunk 2: Dashboard Redesign

### Task 2: Implement the new presentation in `/admin`

**Files:**
- Modify: `src/routes/(app)/admin/+page.svelte`

- [ ] Rework the masthead into a calmer executive header.
- [ ] Restyle the KPI row for a more premium, denser scan pattern.
- [ ] Redesign the focus cards into a more editorial action band.
- [ ] Rebuild the bottom decision layer with stronger quick actions and lightweight trend visuals.
- [ ] Preserve existing data and click behavior.

## Chunk 3: Verification

### Task 3: Prove the redesign is valid

**Files:**
- No extra files required unless verification reveals needed fixes

- [ ] Run focused Vitest suites for the new UI guardrails.
- [ ] Run `npm run check`.
- [ ] Review the diff to ensure the change stayed presentation-only.
