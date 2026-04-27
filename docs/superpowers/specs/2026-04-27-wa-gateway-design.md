# WhatsApp Gateway Design

Date: 2026-04-27

## Goal

Add WhatsApp delivery as a configurable notification channel for both platform operations and tenant operations.

The platform must support a fast Fonnte setup and a flexible custom HTTP gateway without coupling notification events to a single provider.

## Scope

In scope:

- Platform-level WhatsApp gateway configuration for superadmin use.
- Tenant-level WhatsApp gateway configuration for each tenant.
- Provider selector with `disabled`, `fonnte`, and `custom_http`.
- Test-send action from both platform and tenant settings.
- Event-channel preferences at platform and tenant scope.
- User/customer opt-in preference for receiving WhatsApp notifications.
- Delivery logging for success and failure.

Out of scope for the first implementation:

- Full WhatsApp conversation inbox.
- Incoming webhook processing.
- Per-message template designer.
- Rich media delivery.
- Per-event user preference matrix beyond broad opt-in and optional category toggles.

## Existing System Fit

The app already has tenant-aware settings:

- Superadmin reads and writes global settings where `tenant_id IS NULL`.
- Tenant admin reads and writes tenant settings where `tenant_id = claims.tenant_id`.

WhatsApp gateway configuration should reuse this settings model. It avoids a new configuration table and preserves the current authorization model.

Notification delivery should be added behind a dedicated service boundary so existing notification producers do not need to know about Fonnte or custom HTTP details.

## Architecture

Add a backend `WhatsAppGatewayService` with a provider adapter interface:

- `FonnteAdapter`
- `CustomHttpAdapter`

The service resolves configuration by scope:

- Platform events use global settings.
- Tenant events use tenant settings.
- Optional fallback to global settings can be added later, but the first implementation should keep tenant and platform gateways explicit to avoid accidental cross-tenant delivery.

The notification/event layer calls one stable method:

```text
send_whatsapp_notification(scope, recipient, event_code, message)
```

That method handles preference checks, provider resolution, delivery, and logging.

## Settings Keys

Gateway keys:

- `wa_gateway_enabled`
- `wa_gateway_provider`
- `wa_gateway_fonnte_token`
- `wa_gateway_fonnte_base_url`
- `wa_gateway_fonnte_sender`
- `wa_gateway_custom_url`
- `wa_gateway_custom_method`
- `wa_gateway_custom_headers`
- `wa_gateway_custom_body_template`
- `wa_gateway_custom_success_statuses`

Event preference keys:

- `wa_events_platform`
- `wa_events_tenant`

These can be JSON objects keyed by event code:

```json
{
  "customer_invoice_due": {
    "whatsapp": true,
    "email": true,
    "in_app": true
  }
}
```

Sensitive setting keys such as token, headers, and body templates containing secrets must be redacted from audit details.

## Event Registry

Platform event examples:

- `tenant_invoice_created`
- `tenant_invoice_due`
- `tenant_subscription_expiring`
- `system_alert`
- `backup_failed`

Tenant event examples:

- `customer_invoice_created`
- `customer_invoice_due`
- `payment_received`
- `installation_scheduled`
- `installation_completed`
- `support_ticket_replied`
- `network_router_down`

The first implementation should create a typed registry in code so UI, tests, and backend validation share the same event codes.

## User Preferences

Add user/customer notification preference fields through settings or a dedicated preference model, depending on existing user profile patterns discovered during implementation.

Minimum first version:

- WhatsApp phone number.
- WhatsApp notifications enabled.

Optional category toggles:

- Billing.
- Support.
- Network.
- System.

Per-event user preferences can be added later without changing the gateway provider API.

## UI

Superadmin:

- Add a WhatsApp Gateway tab to `/superadmin/settings`.
- Show provider selector, provider-specific fields, event toggles for platform events, and test-send form.

Tenant admin:

- Add a WhatsApp Gateway tab to `/{tenant}/admin/settings`.
- Show provider selector, provider-specific fields, event toggles for tenant events, and test-send form.

User/customer:

- Add WhatsApp phone and opt-in toggle to profile notification preferences.

The UI should follow existing settings page patterns and lazy-load the new tab where the current settings page already uses deferred modules.

## Delivery Flow

1. A platform or tenant event is emitted.
2. The notification layer identifies the scope and event code.
3. The event channel settings are loaded for that scope.
4. If WhatsApp is disabled for the event, delivery stops.
5. The recipient preference is checked.
6. The recipient phone is normalized.
7. The active gateway provider is loaded for the same scope.
8. The provider adapter sends the message.
9. A delivery log records provider, event code, recipient, status, and error summary.

## Error Handling

- Missing gateway config returns a clear validation error in test-send.
- Runtime event delivery failures are logged but must not block the original business event.
- Provider responses should be summarized without storing full secrets.
- Custom HTTP JSON parsing errors should fail validation before saving or before test-send.

## Testing

Frontend tests:

- Provider form state and required fields.
- Event toggle serialization.
- User preference validation.

Backend tests:

- Settings redaction for WhatsApp secrets.
- Provider config validation.
- Fonnte request building.
- Custom HTTP request building from templates.
- Preference gate behavior.
- Delivery failure logging without propagating failures into business flow.

## Implementation Notes

Use TDD for behavior changes. Start with narrow tests around config normalization and provider request building before wiring UI.

Keep the first release focused on outbound text notifications. Incoming messages and templates should be separate follow-up work.
