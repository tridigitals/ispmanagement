# AI Workflow Prompts

Prompt definitions are in [`.roo/prompts`](../../.roo/prompts).

## Prompt set

- Default policy: [`.roo/prompts/no-feature-change-system.md`](../../.roo/prompts/no-feature-change-system.md)
- Subtask handoff format: [`.roo/prompts/subtask-brief-template.md`](../../.roo/prompts/subtask-brief-template.md)
- Final gate: [`.roo/prompts/review-checklist.md`](../../.roo/prompts/review-checklist.md)

## Practical execution order

1. Start every task with no feature change policy
2. For split work, package each handoff with subtask brief template
3. Before completion, run review checklist and resolve gaps

## What each prompt enforces

### no feature change system

- Preserve existing behavior by default
- Keep UI and API surface stable unless explicitly authorized
- Prefer minimal localized edits

### subtask brief template

- Objective and scope boundaries
- Non-goals and constraints
- Required validations and expected handoff artifact

### review checklist

- Scope adherence
- Contract safety across frontend and backend
- Data and migration safety
- Validation evidence quality

## Contributor rule of thumb

If a task touches [`src/lib/api`](../../src/lib/api), [`src/routes`](../../src/routes), [`src-tauri/src/http`](../../src-tauri/src/http), [`src-tauri/src/services`](../../src-tauri/src/services), [`src-tauri/src/models`](../../src-tauri/src/models), or [`src-tauri/migrations`](../../src-tauri/migrations), run the checklist before claiming completion.