# Message Template Builder Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build tenant-scoped WhatsApp/email message templates with granular metadata, RBAC, preview rendering, and customer-page WhatsApp send integration.

**Architecture:** Add a `message_templates` backend module with SQL persistence, a strict variable renderer, HTTP/Tauri commands, and TS API wrapper. UI gets an admin template builder surface plus customer compose integration that renders/sends saved templates through the existing WhatsApp gateway.

**Tech Stack:** SvelteKit/Svelte 5, TypeScript API wrappers, Tauri commands, Axum HTTP routes, Rust services, SQLx, existing RBAC/permission seed patterns.

---

## Chunk 1: Backend Contract And Renderer

### Task 1: Add message template models and renderer tests

**Files:**
- Create: `src-tauri/src/models/message_template.rs`
- Modify: `src-tauri/src/models/mod.rs`
- Create: `src-tauri/src/services/message_template_renderer.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] Write failing Rust tests for rendering known variables and rejecting unknown variables.
- [ ] Run `rtk cargo test message_template_renderer` and verify RED.
- [ ] Implement model enums/DTOs and renderer.
- [ ] Run `rtk cargo test message_template_renderer` and verify GREEN.

### Task 2: Add persistence service and migration

**Files:**
- Create migration in `src-tauri/migrations/`
- Create: `src-tauri/src/services/message_template_service.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] Write failing tests/source checks for CRUD function names and permission strings.
- [ ] Add `message_templates` table with tenant indexes and unique `(tenant_id, key)`.
- [ ] Implement list/create/update/delete/preview helpers.
- [ ] Verify with `rtk cargo check`.

## Chunk 2: API And RBAC

### Task 3: Register backend HTTP and Tauri commands

**Files:**
- Create: `src-tauri/src/http/message_templates.rs`
- Create: `src-tauri/src/commands/message_templates.rs`
- Modify: `src-tauri/src/http/mod.rs`
- Modify: `src-tauri/src/bootstrap/http.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify backend bootstrap/app state to manage `MessageTemplateService`

- [ ] Write source tests for HTTP routes, commands, and RBAC strings.
- [ ] Add routes under `/api/message-templates`.
- [ ] Add Tauri commands with matching safeInvoke names.
- [ ] Verify `communication_templates.read` for list/preview and `communication_templates.manage` for mutations.

### Task 4: Update RBAC seed/default permissions

**Files:**
- Locate and modify existing RBAC seed/default permission files.

- [ ] Add `communication_templates.read`.
- [ ] Add `communication_templates.manage`.
- [ ] Include both for admin/owner roles using the repo's existing pattern.
- [ ] Add/adjust tests if permission seed source tests exist.

## Chunk 3: Frontend API And Builder UI

### Task 5: Add TS API wrapper and types

**Files:**
- Modify: `src/lib/api/types.ts`
- Create: `src/lib/api/messageTemplates.ts`
- Modify: `src/lib/api/core.ts`
- Modify: `src/lib/api/client.ts`
- Create: `src/lib/api/messageTemplates.test.ts`

- [ ] Write failing wrapper tests for list/create/update/delete/preview.
- [ ] Implement types and safeInvoke wrapper.
- [ ] Verify target tests pass.

### Task 6: Add template builder page

**Files:**
- Create: `src/routes/(app)/admin/message-templates/+page.svelte`
- Add source/UI tests in existing admin UI test file or a new route test.

- [ ] Write failing source test for dark responsive layout, filters, editor, preview, and RBAC guard.
- [ ] Implement list filters, create/edit modal, variable picker, preview panel, delete/archive action.
- [ ] Keep UI restrained and mobile responsive.
- [ ] Verify Svelte check.

## Chunk 4: Customer WhatsApp Integration

### Task 7: Use saved templates in customer compose

**Files:**
- Modify: `src/routes/(app)/admin/customers/+page.svelte`
- Modify: `src-tauri/src/commands/whatsapp.rs`
- Modify: `src-tauri/src/http/whatsapp.rs`
- Modify: `src/lib/api/whatsapp.ts`
- Adjust tests in `src/lib/api/whatsapp.test.ts` and `src/routes/admin-operational-ui.test.ts`

- [ ] Write failing tests for `templateId` send payload and customer page source using `messageTemplates.list`.
- [ ] Backend accepts optional `template_id`; if present, render tenant active manual WhatsApp template with customer context.
- [ ] UI loads active manual WhatsApp customer templates and shows them in compose modal.
- [ ] Custom message remains fallback.
- [ ] Verify targeted tests.

## Chunk 5: Verification

- [ ] Run `rtk npm run test:unit -- src/lib/api/messageTemplates.test.ts src/lib/api/whatsapp.test.ts src/routes/admin-operational-ui.test.ts`.
- [ ] Run `rtk cargo test message_template`.
- [ ] Run `rtk cargo check`.
- [ ] Run `rtk npm run check`.
- [ ] Run `rtk npm run build`.
- [ ] Document any pre-existing warnings/failures separately.
