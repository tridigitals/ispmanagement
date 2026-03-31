# AI Workflow Modes

Mode definitions come from [`.roo/modes`](../../.roo/modes).

## Quick mode selector

- Use [`.roo/modes/ask.md`](../../.roo/modes/ask.md) when you need analysis and impact mapping
- Use [`.roo/modes/architect.md`](../../.roo/modes/architect.md) when sequencing or scope is non-trivial
- Use [`.roo/modes/code.md`](../../.roo/modes/code.md) when implementing approved minimal edits
- Use [`.roo/modes/debug.md`](../../.roo/modes/debug.md) when root cause is unclear
- Use [`.roo/modes/orchestrator.md`](../../.roo/modes/orchestrator.md) for multi-step coordination

## Responsibilities by mode

### Ask

- Summarize current behavior
- Identify blast radius
- Output explicit non-goals and validation expectations

### Architect

- Define boundaries and ordered execution
- Check cross-layer effects across frontend, backend, and schema
- Output rollout and rollback notes

### Code

- Apply smallest safe patch
- Keep contracts stable unless exception approved
- Output changed files and validation evidence

### Debug

- Reproduce then isolate
- Patch only defect path
- Output root cause and regression target

### Orchestrator

- Split into scoped subtasks
- Assign mode plus skill per subtask
- Merge outputs with checklist completion

## Standard handoff checklist

Before switching modes, provide:

- Objective and non-goals
- Allowed files or surfaces
- Expected validations
- Completion artifact

Use [`.roo/prompts/subtask-brief-template.md`](../../.roo/prompts/subtask-brief-template.md).

## Repo surfaces every mode should watch

- [`src/routes`](../../src/routes)
- [`src/lib/api`](../../src/lib/api)
- [`src-tauri/src/http`](../../src-tauri/src/http)
- [`src-tauri/src/services`](../../src-tauri/src/services)
- [`src-tauri/src/models`](../../src-tauri/src/models)
- [`src-tauri/migrations`](../../src-tauri/migrations)