# WhatsApp Gateway Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add configurable WhatsApp notification delivery for superadmin platform events, tenant events, and user opt-in preferences.

**Architecture:** Reuse the existing tenant-aware `settings` table for gateway and event-channel configuration, add a focused Rust `WhatsAppGatewayService` for provider validation/request building/delivery logging, and expose test-send APIs through Axum plus the existing frontend API bridge. UI changes stay inside current settings/profile lazy-loading patterns.

**Tech Stack:** Rust + Axum + SQLx backend, SvelteKit 5 frontend, Vitest for TS unit tests, Cargo tests for Rust behavior tests.

---

## File Structure

Backend:

- Create `src-tauri/src/models/whatsapp.rs`: provider config, event registry, test-send DTOs, delivery log model.
- Modify `src-tauri/src/models/mod.rs`: export WhatsApp models.
- Create `src-tauri/src/services/whatsapp_gateway_service.rs`: config loading, validation, phone normalization, Fonnte/custom request construction, delivery logging.
- Modify `src-tauri/src/services/mod.rs`: export `WhatsAppGatewayService`.
- Create `src-tauri/src/http/whatsapp.rs`: authenticated test-send and event-config endpoints.
- Modify `src-tauri/src/http/mod.rs`: add service to `AppState`.
- Modify `src-tauri/src/bootstrap/http.rs`: construct state and mount `/whatsapp/*` routes.
- Modify `src-tauri/src/bin/server.rs` and `src-tauri/src/bootstrap/app.rs`: construct and pass service where required.
- Modify `src-tauri/src/services/settings_service.rs`: redact WhatsApp sensitive settings in audit.
- Add migrations `src-tauri/migrations/20260427130000_add_whatsapp_delivery_logs.up.sql` and `.down.sql`.

Frontend:

- Create `src/lib/api/whatsapp.ts`: typed API wrapper.
- Modify `src/lib/api/client.ts`: export `whatsapp`.
- Modify `src/lib/api/core.ts`: command map entries.
- Modify `src/lib/api/types.ts`: WhatsApp types.
- Create `src/lib/utils/whatsappGateway.ts` and `src/lib/utils/whatsappGateway.test.ts`: provider form normalization and event preference helpers.
- Create `src/lib/components/settings/WhatsAppGatewayTab.svelte`: shared gateway/event/test-send UI.
- Modify `src/routes/superadmin/settings/settingsPageModules.ts`: lazy-load tab.
- Modify `src/routes/superadmin/settings/+page.svelte`: add tab state and settings keys.
- Modify `src/routes/[tenant]/(app)/admin/settings/adminSettingsPageModules.ts`: lazy-load tab.
- Modify `src/routes/[tenant]/(app)/admin/settings/+page.svelte`: add tab state and settings keys.
- Modify profile notification components under `src/lib/components/profile/` after locating the current notifications tab implementation.

## Chunk 1: Backend Gateway Core

### Task 1: Add WhatsApp model and config tests

**Files:**

- Create: `src-tauri/src/models/whatsapp.rs`
- Modify: `src-tauri/src/models/mod.rs`
- Test: `src-tauri/src/services/whatsapp_gateway_service.rs`

- [ ] **Step 1: Write failing Rust tests**

Add tests for:

- Fonnte config requires token when enabled.
- Custom HTTP config requires URL and valid method.
- Phone normalization converts `08123456789` to `628123456789`.
- Event registry separates platform and tenant event codes.

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml whatsapp_gateway
```

Expected: FAIL because module/types do not exist.

- [ ] **Step 2: Implement minimal models**

Create model enums/structs:

- `WhatsAppGatewayProvider`
- `WhatsAppGatewayConfig`
- `WhatsAppEventScope`
- `WhatsAppEventDefinition`
- `WhatsAppTestSendRequest`
- `WhatsAppTestSendResponse`

- [ ] **Step 3: Run tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml whatsapp_gateway
```

Expected: PASS for the new pure model/config tests.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/models/whatsapp.rs src-tauri/src/models/mod.rs src-tauri/src/services/whatsapp_gateway_service.rs
git commit -m "feat: add whatsapp gateway config model"
```

### Task 2: Add request builders and delivery log migration

**Files:**

- Create: `src-tauri/src/services/whatsapp_gateway_service.rs`
- Create: `src-tauri/migrations/20260427130000_add_whatsapp_delivery_logs.up.sql`
- Create: `src-tauri/migrations/20260427130000_add_whatsapp_delivery_logs.down.sql`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: Write failing tests**

Test request construction without network calls:

- Fonnte builder uses configured base URL or default.
- Fonnte builder sends token as provider expects.
- Custom HTTP builder substitutes `{{phone}}` and `{{message}}`.
- Custom HTTP headers JSON rejects malformed data.

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml whatsapp_gateway_service
```

Expected: FAIL because builders are missing.

- [ ] **Step 2: Implement minimal service**

Add pure helper methods first:

- `normalize_phone`
- `validate_config`
- `build_fonnte_request`
- `build_custom_http_request`

Add `whatsapp_delivery_logs` migration with:

- `id`
- `tenant_id`
- `scope`
- `event_code`
- `provider`
- `recipient_user_id`
- `recipient_phone`
- `status`
- `error_summary`
- `created_at`

- [ ] **Step 3: Run tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml whatsapp_gateway_service
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/services/whatsapp_gateway_service.rs src-tauri/src/services/mod.rs src-tauri/migrations/20260427130000_add_whatsapp_delivery_logs.*
git commit -m "feat: add whatsapp gateway service core"
```

### Task 3: Add authenticated HTTP API

**Files:**

- Create: `src-tauri/src/http/whatsapp.rs`
- Modify: `src-tauri/src/http/mod.rs`
- Modify: `src-tauri/src/bootstrap/http.rs`
- Modify: `src-tauri/src/bin/server.rs`
- Modify: `src-tauri/src/bootstrap/app.rs`
- Modify: `src-tauri/src/services/settings_service.rs`

- [ ] **Step 1: Write failing tests**

Add focused source/API tests following existing repository style:

- Route map includes `/whatsapp/test-send`.
- Superadmin resolves test-send scope to global settings.
- Tenant admin requires `settings:update` or equivalent permission.
- Audit redaction treats `wa_gateway_fonnte_token`, custom headers, and body template as sensitive.

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml whatsapp
```

Expected: FAIL for missing routes/redaction.

- [ ] **Step 2: Implement endpoints**

Add endpoints:

- `POST /api/whatsapp/test-send`
- `GET /api/whatsapp/events`

Use existing auth extraction pattern from `settings.rs`. Test-send should validate config and attempt delivery; event registry is safe to return after auth.

- [ ] **Step 3: Wire service into app state**

Construct `WhatsAppGatewayService` with DB pool/settings dependencies and add it to `AppState`.

- [ ] **Step 4: Run backend checks**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml whatsapp
cargo fmt --manifest-path src-tauri/Cargo.toml --all
```

Expected: PASS and formatted.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/http/whatsapp.rs src-tauri/src/http/mod.rs src-tauri/src/bootstrap/http.rs src-tauri/src/bin/server.rs src-tauri/src/bootstrap/app.rs src-tauri/src/services/settings_service.rs
git commit -m "feat: expose whatsapp gateway api"
```

## Chunk 2: Frontend Settings UI

### Task 4: Add API wrapper and utility tests

**Files:**

- Create: `src/lib/api/whatsapp.ts`
- Modify: `src/lib/api/client.ts`
- Modify: `src/lib/api/core.ts`
- Modify: `src/lib/api/types.ts`
- Create: `src/lib/utils/whatsappGateway.ts`
- Create: `src/lib/utils/whatsappGateway.test.ts`

- [ ] **Step 1: Write failing Vitest tests**

Test:

- Settings map converts to form state for disabled/Fonnte/custom.
- Form state serializes back to settings keys.
- Event preference JSON defaults unknown events to disabled for WhatsApp.
- Test-send wrapper calls `safeInvoke('send_test_whatsapp', ...)`.

Run:

```bash
npm run test:unit -- src/lib/utils/whatsappGateway.test.ts src/lib/api/whatsapp.test.ts
```

Expected: FAIL because files are missing.

- [ ] **Step 2: Implement API/types/utils**

Add command map entries:

- `send_test_whatsapp`: `POST /whatsapp/test-send`
- `list_whatsapp_events`: `GET /whatsapp/events`

Export `api.whatsapp`.

- [ ] **Step 3: Run tests**

Run:

```bash
npm run test:unit -- src/lib/utils/whatsappGateway.test.ts src/lib/api/whatsapp.test.ts
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/lib/api/whatsapp.ts src/lib/api/client.ts src/lib/api/core.ts src/lib/api/types.ts src/lib/utils/whatsappGateway.ts src/lib/utils/whatsappGateway.test.ts src/lib/api/whatsapp.test.ts
git commit -m "feat: add whatsapp frontend api helpers"
```

### Task 5: Add shared WhatsApp settings tab

**Files:**

- Create: `src/lib/components/settings/WhatsAppGatewayTab.svelte`
- Modify: `src/routes/superadmin/settings/settingsPageModules.ts`
- Modify: `src/routes/superadmin/settings/+page.svelte`
- Modify: `src/routes/[tenant]/(app)/admin/settings/adminSettingsPageModules.ts`
- Modify: `src/routes/[tenant]/(app)/admin/settings/+page.svelte`

- [ ] **Step 1: Write failing component/helper tests**

Prefer utility-level tests for tab data contract:

- Superadmin passes `scope="platform"` and platform event keys.
- Tenant admin passes `scope="tenant"` and tenant event keys.
- Dirty state changes when provider fields update.

Run:

```bash
npm run test:unit -- src/lib/utils/whatsappGateway.test.ts
```

Expected: FAIL for missing tab integration helper cases.

- [ ] **Step 2: Implement shared component**

Build a compact settings form:

- Provider segmented/select control: Disabled, Fonnte, Custom HTTP.
- Fonnte fields: token, base URL, sender.
- Custom fields: method, URL, headers JSON, body template, success statuses.
- Event toggles table for WhatsApp/email/in-app.
- Test send: phone + message + submit.

Use existing `Icon`, `Input`, `Select`, button, toast, and settings page visual patterns.

- [ ] **Step 3: Integrate superadmin settings**

Add `whatsapp` tab to `SettingsTabId`, loaders, categories, settings application, and save flow.

- [ ] **Step 4: Integrate tenant admin settings**

Add `whatsapp` category and lazy loader. Include gateway keys in local settings build/save.

- [ ] **Step 5: Run frontend checks**

Run:

```bash
npm run test:unit -- src/lib/utils/whatsappGateway.test.ts
npm run check
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/settings/WhatsAppGatewayTab.svelte src/routes/superadmin/settings/settingsPageModules.ts src/routes/superadmin/settings/+page.svelte src/routes/[tenant]/\\(app\\)/admin/settings/adminSettingsPageModules.ts src/routes/[tenant]/\\(app\\)/admin/settings/+page.svelte
git commit -m "feat: add whatsapp gateway settings ui"
```

## Chunk 3: User Preferences and Event Delivery

### Task 6: Add user WhatsApp opt-in preference

**Files:**

- Inspect/modify: `src/lib/components/profile/*`
- Modify: `src/routes/[tenant]/(app)/profile/+page.svelte`
- Backend path to determine during implementation: existing user profile/preferences handlers.

- [ ] **Step 1: Locate existing profile preference API**

Run:

```bash
rg -n "NotificationPreference|UpdatePreferenceRequest|profile|preferences" src src-tauri/src
```

Expected: identify current notification preference components and handlers.

- [ ] **Step 2: Write failing tests**

Test:

- User can store WhatsApp opt-in.
- Empty phone blocks opt-in or shows validation.
- Phone normalization preview matches backend rules.

- [ ] **Step 3: Implement minimum preference UI/API**

Add:

- WhatsApp phone input.
- WhatsApp notifications enabled toggle.
- Optional category toggles only if existing preference shape supports them cleanly.

- [ ] **Step 4: Run checks**

Run:

```bash
npm run test:unit
npm run check
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/profile src/routes/[tenant]/\\(app\\)/profile/+page.svelte src-tauri/src
git commit -m "feat: add whatsapp notification preferences"
```

### Task 7: Wire WhatsApp delivery into notification flow

**Files:**

- Modify: `src-tauri/src/services/notification_service.rs`
- Modify related event producers only where clear first-use events already exist.
- Test: Rust service tests in `notification_service.rs` or a focused module.

- [ ] **Step 1: Write failing tests**

Test:

- WhatsApp disabled for event skips delivery.
- User opt-out skips delivery.
- Delivery failure logs error and does not fail notification creation.

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml whatsapp_notification_delivery
```

Expected: FAIL because notification service is not wired.

- [ ] **Step 2: Implement delivery gate**

Add a non-blocking call after in-app notification creation for categories that map to event codes.

Keep initial mapping conservative:

- `payment` -> billing event.
- `support` -> support event.
- `system` -> system event.

- [ ] **Step 3: Run backend tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml whatsapp_notification_delivery
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/services/notification_service.rs src-tauri/src/services/whatsapp_gateway_service.rs
git commit -m "feat: send whatsapp notifications from event preferences"
```

## Chunk 4: Verification

### Task 8: Full verification

**Files:** no intentional edits unless fixing failures.

- [ ] **Step 1: Run unit tests**

```bash
npm run test:unit
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 2: Run static checks**

```bash
npm run check
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
```

Expected: PASS.

- [ ] **Step 3: Run migration check**

```bash
npm run db:migrate
```

Expected: migration applies cleanly in the configured local database.

- [ ] **Step 4: Manual smoke test**

Start the app:

```bash
npm run tauri dev
```

Verify:

- Superadmin can open WhatsApp Gateway tab and send a test message.
- Tenant admin can open WhatsApp Gateway tab and send a test message.
- User can opt in/out from profile.
- Disabling event toggle prevents WhatsApp delivery.

- [ ] **Step 5: Final commit**

```bash
git status --short
git commit -m "test: verify whatsapp gateway integration"
```

Only commit if there are verification-only fixes or docs updates.
