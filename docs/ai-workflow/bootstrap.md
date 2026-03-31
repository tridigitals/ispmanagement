# AI Workflow Bootstrap

Use this page as the contributor quickstart for the integrated workflow.

## 5 minute setup checklist

- [ ] Read [`.roo/README.md`](../../.roo/README.md)
- [ ] Review guardrail baseline in [`.roo/prompts/no-feature-change-system.md`](../../.roo/prompts/no-feature-change-system.md)
- [ ] Open index in [`AGENTS.md`](../../AGENTS.md)
- [ ] Confirm repo surfaces in [`docs/ai-workflow/sveltekit-tauri-compatibility.md`](./sveltekit-tauri-compatibility.md)
- [ ] Use mode selection in [`docs/ai-workflow/modes.md`](./modes.md)

## Default operating rule

Start from **no feature change** unless a task explicitly authorizes exceptions.

Policy details: [`docs/ai-workflow/no-feature-change-policy.md`](./no-feature-change-policy.md)

## Practical contributor flow

1. **Scope first**
   - Clarify objective and explicit non-goals
   - Identify touched surfaces in [`src/routes`](../../src/routes), [`src/lib/api`](../../src/lib/api), [`src-tauri/src/http`](../../src-tauri/src/http), [`src-tauri/src/services`](../../src-tauri/src/services), [`src-tauri/src/models`](../../src-tauri/src/models), [`src-tauri/migrations`](../../src-tauri/migrations)
2. **Select mode**
   - Planning heavy: [`.roo/modes/architect.md`](../../.roo/modes/architect.md)
   - Implementation: [`.roo/modes/code.md`](../../.roo/modes/code.md)
   - Diagnosis: [`.roo/modes/debug.md`](../../.roo/modes/debug.md)
3. **Attach skill**
   - Start with [`.roo/skills/repo-audit.md`](../../.roo/skills/repo-audit.md)
   - Add stack skill for touched surface
4. **Validate and close**
   - Run [`.roo/prompts/review-checklist.md`](../../.roo/prompts/review-checklist.md)
   - Confirm behavior preserved outside approved scope

## Command posture for contributors

- Prefer minimal diffs and localized edits
- Keep API/UI contracts stable unless exception is approved
- If schema or contract must change, synchronize frontend and backend updates in one scoped change set

Next references:
- Modes: [`docs/ai-workflow/modes.md`](./modes.md)
- Skills: [`docs/ai-workflow/skills.md`](./skills.md)
- Prompts: [`docs/ai-workflow/prompts.md`](./prompts.md)