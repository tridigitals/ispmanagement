# Message Template Builder Design

## Goal

Build a granular communication template system for WhatsApp and email so admins can create reusable message templates for manual customer communication now and system-triggered notifications later.

## Recommended Approach

Use a hybrid model:

- Admin UI groups templates by operational use case: billing, installation, support, outage, customer lifecycle, and custom.
- Backend stores granular metadata: channel, target, trigger mode, event key, variable context, status, and version.
- The same renderer is used by manual sends and future automatic events, so template behavior stays consistent.

## Scope

Initial implementation includes:

- Template CRUD for tenant admins.
- RBAC permissions for template read/manage.
- WhatsApp body templates.
- Email subject/body templates in the same data model and API.
- Variable validation for known context fields.
- Preview rendering from supplied sample data.
- Customer page WhatsApp send using templates from the new library.

Deferred:

- Full automatic event wiring for every billing/install event.
- Rich email designer with drag/drop blocks.
- Per-language template variants beyond storing `locale`.
- Approval workflow.

## Data Model

`message_templates`:

- `id`
- `tenant_id`
- `key`
- `name`
- `description`
- `use_case`
- `target`
- `trigger_mode`
- `event_key`
- `channel`
- `locale`
- `status`
- `whatsapp_body`
- `email_subject`
- `email_body`
- `variables`
- `version`
- `created_at`
- `updated_at`

Channels:

- `whatsapp`
- `email`
- `both`

Trigger mode:

- `manual`
- `automatic`
- `both`

Status:

- `draft`
- `active`
- `archived`

## Variable Context

Initial customer-send context supports:

- `tenant.name`
- `customer.id`
- `customer.name`
- `customer.email`
- `customer.phone`
- `customer.status`
- `customer.notes`

The renderer must reject unknown variables instead of silently sending broken content.

## RBAC

Add permissions:

- `communication_templates.read`
- `communication_templates.manage`

Admin/owner roles should receive both. Read-only operational roles can receive read only if the current seed pattern supports it.

Manual customer sends still require `customers.manage` and WhatsApp gateway readiness. Template management requires `communication_templates.manage`.

## UI

Add a new settings/admin communication surface for templates:

- Template list with filters by use case/channel/status.
- Editor modal with metadata, WhatsApp body, email subject/body, variable picker, preview, and validation.
- Keep visual style dark, clean, restrained, and mobile responsive.

Customer page integration:

- Load active manual WhatsApp templates for customer context.
- Compose modal selects a saved template.
- Render preview with selected customer before send.
- Allow custom message fallback only when no saved template is selected.

## Backend Flow

Manual customer WhatsApp send:

1. UI selects customer and template.
2. Backend validates `customers.manage`.
3. Backend validates gateway readiness.
4. Backend loads active template owned by tenant.
5. Backend builds customer context.
6. Backend renders template.
7. Backend sends rendered body through WA gateway.
8. Delivery is logged by gateway service.

Template preview:

1. UI sends template body and sample context.
2. Backend validates variables.
3. Backend returns rendered channel output and missing/unknown variable errors.

## Testing

- TypeScript API wrapper tests for list/create/update/delete/preview.
- Source tests for UI route and customer integration.
- Rust unit tests for renderer variable replacement and unknown variable rejection.
- Rust source tests for HTTP/Tauri command registration and RBAC permission names.
- `npm run check`, `npm run build`, `cargo check`, targeted `cargo test message_template`.
