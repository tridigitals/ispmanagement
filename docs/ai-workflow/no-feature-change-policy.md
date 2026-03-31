# No Feature Change Policy

Default policy source: [`.roo/prompts/no-feature-change-system.md`](../../.roo/prompts/no-feature-change-system.md)

## Default rule

Contributors must preserve existing product behavior unless a task **explicitly** authorizes feature changes.

## What is in policy scope

- UI and route behavior in [`src/routes`](../../src/routes)
- Frontend API contracts in [`src/lib/api`](../../src/lib/api)
- Backend endpoint service model behavior in [`src-tauri/src/http`](../../src-tauri/src/http), [`src-tauri/src/services`](../../src-tauri/src/services), [`src-tauri/src/models`](../../src-tauri/src/models)
- Schema behavior in [`src-tauri/migrations`](../../src-tauri/migrations)

## Defaults contributors must follow

- Keep external contracts stable
- Prefer minimal and localized diffs
- State non-goals before making changes
- Document assumptions and risks
- Validate the affected regression path before completion

## Exception policy

Exceptions are allowed only when all conditions are met:

- The task explicitly requests a feature or contract expansion
- Scope names the allowed changed surfaces
- Cross-layer sync is included where needed
- Validation criteria are defined up front

If exception conditions are not met, fallback is strict no feature change.

## Required controls when exception is approved

- Sync frontend and backend contract changes in one scoped change set
- Update affected call sites and types
- If schema changes, include safe migration strategy and rollback handling
- Run review gate in [`.roo/prompts/review-checklist.md`](../../.roo/prompts/review-checklist.md)

## Practical acceptance checklist

- [ ] Approved objective is reflected exactly
- [ ] Non-goals are explicit
- [ ] Unrelated feature behavior remains unchanged
- [ ] Validation evidence covers touched layers