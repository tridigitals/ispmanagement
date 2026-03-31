# SvelteKit Tauri Compatibility Guide

Use this page to keep cross-layer behavior stable in this stack.

## Stack boundaries in this repo

- UI routes and page behavior: [`src/routes`](../../src/routes)
- Frontend API client and TS types: [`src/lib/api`](../../src/lib/api)
- Backend HTTP handlers: [`src-tauri/src/http`](../../src-tauri/src/http)
- Backend service logic: [`src-tauri/src/services`](../../src-tauri/src/services)
- Backend models: [`src-tauri/src/models`](../../src-tauri/src/models)
- Schema and rollback scripts: [`src-tauri/migrations`](../../src-tauri/migrations)

## Compatibility rules

- Do not change route params or load return keys unintentionally
- Keep frontend API signatures aligned with backend request and response shapes
- Keep backend error and auth semantics stable unless explicitly approved
- Keep model fields consistent with active migration state

## Safe change checklist by scenario

### UI-only polish

- Touched files mainly in [`src/routes`](../../src/routes)
- Confirm no API payload shape changes in [`src/lib/api`](../../src/lib/api)

### Backend logic fix

- Touched files mainly in [`src-tauri/src/services`](../../src-tauri/src/services) or [`src-tauri/src/http`](../../src-tauri/src/http)
- Confirm client assumptions in [`src/lib/api`](../../src/lib/api) still hold

### Contract change required

- Update backend endpoint plus model impact
- Sync frontend client and call sites in same scoped change
- Apply [`.roo/skills/api-contract-sync.md`](../../.roo/skills/api-contract-sync.md)

### Migration involved

- Keep migration additive when possible
- Provide compatible read path during transition
- Apply [`.roo/skills/db-migration-safety.md`](../../.roo/skills/db-migration-safety.md)

## Minimal validation matrix

- Frontend type checks for touched API calls and routes
- Backend compile or tests for touched handlers services models
- Migration up down sanity if SQL changed
- Targeted regression path for changed user flow