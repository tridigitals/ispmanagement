# AI Workflow Skills

Skill definitions are in [`.roo/skills`](../../.roo/skills).

## Which skill to start with

- Always start with [`.roo/skills/repo-audit.md`](../../.roo/skills/repo-audit.md)
- Add one stack skill based on touched files
- Add contract sync when payload or response shapes move

## Skill map by repository surface

- [`src/routes`](../../src/routes) or UI behavior
  - [`.roo/skills/sveltekit-ui-safe-change.md`](../../.roo/skills/sveltekit-ui-safe-change.md)
- [`src-tauri/src/http`](../../src-tauri/src/http), [`src-tauri/src/services`](../../src-tauri/src/services), [`src-tauri/src/models`](../../src-tauri/src/models)
  - [`.roo/skills/tauri-rust-service-safe-change.md`](../../.roo/skills/tauri-rust-service-safe-change.md)
- [`src-tauri/migrations`](../../src-tauri/migrations)
  - [`.roo/skills/db-migration-safety.md`](../../.roo/skills/db-migration-safety.md)
- Contract movement across frontend and backend
  - [`.roo/skills/api-contract-sync.md`](../../.roo/skills/api-contract-sync.md)

## Practical sequence

1. Run repo audit
2. Pick one primary stack skill
3. Add contract sync if API shapes are touched
4. Validate with review checklist prompt

## Minimal output expected from skill usage

- Affected files by layer
- Invariants and non-goals
- Validation set
- Rollback note if risk is medium or high