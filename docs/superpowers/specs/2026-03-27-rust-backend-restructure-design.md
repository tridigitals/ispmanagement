# Rust Backend Restructure Design Spec (Approved)

**Date:** 2026-03-27  
**Scope:** `src-tauri` Rust backend structure only  
**Status:** Approved design for phased execution

## 1) Objective

Reorganize backend Rust code structure to improve maintainability **without changing existing functionality** (behavior, API responses, status semantics, and side effects remain unchanged).

## 2) Execution Strategy (Approved)

- **Execution style:** Full backend pass in phases (analysis + design + implementation per module), not a single big-bang refactor.
- **Chosen strategy:** **Option 2 (balanced)** — for each module, do:
  1. split-by-concern,
  2. local deduplication within the same phase.
- **Rollback rule:** If a phase fails verification gates, revert that phase and do not proceed.

## 3) Phase Architecture

Each phase must follow this architecture:

1. **Baseline**
   - Confirm current behavior contract for touched module.
   - Capture touched file list and intended internal boundaries.
2. **Refactor (split + local dedup)**
   - Split internals by concern.
   - Deduplicate only within local phase scope/module scope.
   - Keep public contract stable.
3. **Verify** *(mandatory gate)*
   - Run targeted tests for touched module.
   - Run workspace-wide check under `src-tauri`.
4. **Freeze**
   - No extra opportunistic refactors after passing gates.
   - Record completion artifacts and move to next module.

## 4) Component Boundaries

Per module, use a `mod.rs` facade and split internals as needed into focused units:

- `core/`
- `repository/`
- `integration/`
- `scheduler/`
- `dto/`
- `mapper/`
- `validation/`

Boundary rules:

- `mod.rs` is the stable module surface.
- command and HTTP layers remain **thin adapters** (transport orchestration only).
- business logic moves behind module-internal boundaries.
- no new command -> http dependency introduced.

## 5) Known Hotspots and Risks

### 5.1 Hotspot files from analysis

- `src-tauri/src/services/payment_service.rs` (~4879 lines)
- `src-tauri/src/services/mikrotik_service.rs` (~4137 lines)
- `src-tauri/src/services/customer_service.rs` (~3816 lines)
- `src-tauri/src/services/network_mapping_service.rs` (~2883 lines)
- `src-tauri/src/services/auth_service.rs` (~2583 lines)
- `src-tauri/src/db/connection.rs` (~1659 lines)
- Duplication hotspots:
  - `src-tauri/src/commands/*` (announcements/support paths)
  - `src-tauri/src/http/*` (announcements/support paths)

### 5.2 Risk notes

- command -> http coupling risk during adapter extraction
- scheduler single-start side effect regressions
- visibility leaks while splitting modules/files
- PostgreSQL/SQLite parity divergence during repository split
- dedup drift (same logic reappearing after phased extraction)

## 6) Quality Gates and Acceptance Criteria

A phase is complete only if all criteria pass:

1. **Behavior contract unchanged**
   - API payload/shape and HTTP status semantics remain unchanged for touched routes/commands.
2. **Architecture integrity**
   - no new command -> http coupling.
   - command/http remain thin adapters.
3. **Verification gate (mandatory)**
   - run targeted module tests under `src-tauri` with explicit selectors, for example:
     - `cargo test announcements`
     - `cargo test support`
     - `cargo test payment_service`
   - run workspace-wide backend validation:
     - from `src-tauri`: `cargo check --workspace`
4. **Rollback discipline**
   - if any gate fails, rollback phase changes before proceeding.

## 7) Recommended Module Order

Apply phases in this order:

1. announcements/support dedup (`commands/*` + `http/*` overlap)
2. DB bootstrap split (`src-tauri/src/db/connection.rs`)
3. app/http bootstrap split (startup and HTTP bootstrap wiring)
4. large services in sequence:
   - `payment`
   - `mikrotik`
   - `customer`
   - `network_mapping`
   - `auth`

## 8) Non-goals / Out of Scope

To prevent accidental feature changes, the following are out of scope:

- new features or endpoint additions
- API contract redesign
- auth policy changes
- scheduler behavior redesign (beyond structural extraction preserving behavior)
- database schema or migration semantics changes unless strictly required for parity bug prevention and approved separately
- cross-module dedup not required by current phase scope

## 9) Deliverables Per Phase

Each phase must produce:

1. **Design artifact**
   - module split map (old path -> new internal structure)
2. **Implementation artifact**
   - refactored files with stable `mod.rs` facade
3. **Verification artifact**
   - targeted `cargo test` command + pass output
   - `cargo check --workspace` command + pass output from `src-tauri`
4. **Risk artifact**
   - note any mitigations applied for identified risks
5. **Freeze artifact**
   - phase summary documenting no behavior-contract changes

## 10) Phase Completion Checklist

Use this checklist at the end of every module phase:

- [ ] Baseline documented for touched module
- [ ] Internal split done using `mod.rs` facade and needed subcomponents
- [ ] Local dedup completed within phase scope
- [ ] No new command -> http coupling introduced
- [ ] Targeted module `cargo test` passed
- [ ] `cargo check --workspace` passed under `src-tauri`
- [ ] Behavior contract unchanged (responses/status semantics)
- [ ] Rollback performed if any gate failed (or marked N/A if all passed)
- [ ] Freeze summary recorded

---

## 11) Self-Review (Inline)

This spec has been self-reviewed for:

- **Placeholders:** none (`TBD/TODO` not present)
- **Ambiguity:** mandatory gates, boundaries, and order are explicit
- **Contradictions:** none found between strategy, quality gates, and non-goals

Approved for phased backend restructuring execution under Option 2 (balanced).
