# Managed RADIUS Dynamic Clients Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make FreeRADIUS resolve NAS clients dynamically from PostgreSQL so `managed_radius_nas` changes apply without restarting the container.

**Architecture:** Replace startup SQL client loading with FreeRADIUS dynamic client resolution driven by `managed_radius_nas`. Keep tenant-aware auth queries tied to the resolved runtime client and verify the container still starts healthy and authenticates existing users.

**Tech Stack:** FreeRADIUS 3.x config, Docker Compose, PostgreSQL, Vitest, Rust backend

---

## Chunk 1: Config Regression Guards

### Task 1: Extend repo-level regression tests for dynamic clients

**Files:**
- Modify: `src/lib/utils/freeradiusConfig.test.ts`

- [ ] **Step 1: Write the failing test**
Add assertions that:
- `sql.template` no longer uses startup SQL client loading
- a repo `dynamic-clients` site file exists
- docs no longer describe NAS changes as restart-required

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test:unit -- src/lib/utils/freeradiusConfig.test.ts`

- [ ] **Step 3: Write minimal implementation**
Update tests only; do not change production config yet.

- [ ] **Step 4: Run test to verify it fails for the right reason**

Run: `npm run test:unit -- src/lib/utils/freeradiusConfig.test.ts`

## Chunk 2: FreeRADIUS Dynamic Client Config

### Task 2: Add dynamic client site and wire it into the image

**Files:**
- Create: `deploy/freeradius/raddb/sites-available/dynamic-clients`
- Modify: `deploy/freeradius/docker-entrypoint.sh`
- Modify: `deploy/freeradius/raddb/mods-available/sql.template`
- Modify: `deploy/freeradius/README.md`

- [ ] **Step 1: Implement minimal FreeRADIUS config**
Add a repo-managed `dynamic-clients` site that:
- looks up active NAS rows by packet source IP
- sets runtime client secret / shortname / require-MA

- [ ] **Step 2: Disable startup SQL client loading**
Update `sql.template` so NAS clients are not loaded only at startup.

- [ ] **Step 3: Enable the dynamic-clients site at container start**
Update the entrypoint to symlink the site if needed.

- [ ] **Step 4: Update operational docs**
Document that NAS client edits should no longer require restart after the dynamic client migration.

## Chunk 3: Backend Cleanup

### Task 3: Remove restart-first operational bias

**Files:**
- Modify: `src-tauri/src/services/managed_radius_service.rs`
- Modify: `deploy/systemd/server.env.example`
- Modify: `scripts/restart-freeradius.sh`

- [ ] **Step 1: Review restart hook behavior**
Keep or remove the optional restart hook based on whether it still serves as a fallback only.

- [ ] **Step 2: Minimize backend behavior**
Do not require restart for normal NAS mapping edits once dynamic clients are in place.

- [ ] **Step 3: Keep fallback docs honest**
If the wrapper remains, document it as optional fallback instead of required path.

## Chunk 4: Verification

### Task 4: Run focused verification

**Files:**
- Modify: `deploy/freeradius/raddb/sites-available/dynamic-clients`
- Modify: `deploy/freeradius/docker-entrypoint.sh`
- Modify: `deploy/freeradius/raddb/mods-available/sql.template`
- Modify: `deploy/freeradius/README.md`
- Modify: `src/lib/utils/freeradiusConfig.test.ts`

- [ ] **Step 1: Run JS regression test**

Run: `npm run test:unit -- src/lib/utils/freeradiusConfig.test.ts`

- [ ] **Step 2: Rebuild FreeRADIUS container**

Run: `docker compose -f docker-compose.radius.yml up -d --build freeradius`

- [ ] **Step 3: Check FreeRADIUS config parse**

Run: `docker exec isp_freeradius sh -lc 'freeradius -XC'`
Expected: config parses successfully and dynamic-clients site is loaded

- [ ] **Step 4: Check auth still works**

Run a controlled auth probe against the running stack.
Expected: existing test account still returns `Access-Accept`

- [ ] **Step 5: Check NAS DB change path**
Change a NAS mapping in DB and verify the new source can authenticate without restarting `freeradius`.
