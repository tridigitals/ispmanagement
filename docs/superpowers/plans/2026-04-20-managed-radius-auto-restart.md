# Managed RADIUS Auto-Restart Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Automatically restart FreeRADIUS after superadmin NAS mapping edits that change runtime client-facing fields.

**Architecture:** Add a small restart hook in `ManagedRadiusService::update_mapping`, guarded by pure change-detection logic and environment-driven command resolution. Keep the existing UI/API contract unchanged and document the required server environment for operators.

**Tech Stack:** Rust, Axum/Tauri backend, Vitest for repo-level config regression tests

---

## Chunk 1: Service Hook

### Task 1: Add failing tests for mapping change detection

**Files:**
- Modify: `src-tauri/src/services/managed_radius_service.rs`

- [ ] **Step 1: Write the failing test**
Add unit tests for:
- unchanged mappings do not require restart
- changed NAS IP / shortname / secret / active flag require restart

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml mapping_change_detection_ -- --nocapture`

- [ ] **Step 3: Write minimal implementation**
Add a pure helper that compares current and next runtime client-facing values.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml mapping_change_detection_ -- --nocapture`

### Task 2: Add restart command resolution and hook

**Files:**
- Modify: `src-tauri/src/services/managed_radius_service.rs`

- [ ] **Step 1: Write the failing test**
Add unit test for trimming and resolving:
- `MANAGED_RADIUS_RESTART_COMMAND`
- `MANAGED_RADIUS_RESTART_WORKDIR`

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml restart_command_resolution_treats_blank_values_as_missing -- --nocapture`

- [ ] **Step 3: Write minimal implementation**
Add:
- env resolution helpers
- async restart command runner
- `update_mapping` hook after runtime NAS sync

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml restart_command_resolution_treats_blank_values_as_missing -- --nocapture`

## Chunk 2: Docs And Regression Coverage

### Task 3: Document server environment

**Files:**
- Modify: `deploy/systemd/server.env.example`
- Modify: `deploy/freeradius/README.md`
- Modify: `src/lib/utils/freeradiusConfig.test.ts`

- [ ] **Step 1: Write the failing test**
Extend the existing Vitest regression file to cover the restart-related docs expectation.

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test:unit -- src/lib/utils/freeradiusConfig.test.ts`

- [ ] **Step 3: Write minimal implementation**
Document the restart command env vars and operator requirement that the API user can execute the command.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test:unit -- src/lib/utils/freeradiusConfig.test.ts`

### Task 4: Final verification

**Files:**
- Modify: `src-tauri/src/services/managed_radius_service.rs`
- Modify: `deploy/systemd/server.env.example`
- Modify: `deploy/freeradius/README.md`
- Modify: `src/lib/utils/freeradiusConfig.test.ts`

- [ ] **Step 1: Run targeted Rust tests**

Run:
- `cargo test --manifest-path src-tauri/Cargo.toml mapping_change_detection_ -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml restart_command_resolution_treats_blank_values_as_missing -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml mapping_mutations_sync_runtime_nas_state -- --nocapture`

- [ ] **Step 2: Run targeted JS regression test**

Run: `npm run test:unit -- src/lib/utils/freeradiusConfig.test.ts`

- [ ] **Step 3: Review runtime prerequisite**
Confirm the final message explicitly states that automatic restart needs `MANAGED_RADIUS_RESTART_COMMAND` plus host-level permission for the service user to run it.
