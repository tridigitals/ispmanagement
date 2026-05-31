# Bulk Send Invoice — Implementation Plan

**Goal:** Let admins select multiple invoices in the invoice list and send them to customers in one action — **email with PDF attachment** (primary) + in-app notification (always) — with per-invoice success/skip/fail reporting.

**Tech Stack:** Rust, SQLx, Axum, Tauri, SvelteKit, TypeScript, svelte-i18n, **printpdf** (new dep)

---

## 1. Context & Existing Building Blocks

Confirmed by code inspection (`2026-05-31`):

- **Single send already exists:** `http/customer_communication.rs::send_customer_email` — RBAC `customers:manage`, renders via `message_template_service.render_customer_email`, sends via `notification_service.force_send_email`.
- **Email infra:** `notification_service::force_send_email` / `force_send_email_with_html` → `email_outbox` → `EmailService::send_via_{smtp,resend,sendgrid,webhook}`. **No attachment support anywhere in this chain** — neither in the queue table nor in any of the 4 providers.
- **`lettre` is configured with `builder` feature** → SMTP can attach via `MultiPart::mixed()` + `Attachment::new()`.
- **In-app notify exists:** `payment_service::notify_subscription_invoice_created` (notification + `/pay/{id}` action URL).
- **Bulk UI pattern exists:** invoice list `+page.svelte` already has `generateDueInvoicesBulk()` + `bulkGenerating` + result toast (`created/skipped/failed`). No row checkboxes yet.
- **Existing PDF is client-side only** (`invoicePdf.ts` uses html2canvas + jsPDF) — cannot be reused server-side. Backend needs its own PDF generator.

## 2. Architecture Decision: PDF Generation = `printpdf` (Pure Rust)

Chosen over headless Chromium (Opsi B) because:
- **Server-friendly:** no Chromium install, no system deps beyond what's already shipped.
- **Reliable:** programmatic layout, no browser crashes, predictable performance for bulk runs.
- **Self-contained:** PDF built from invoice data directly, fits invoices' tabular nature.
- **Tradeoff accepted:** layout will not be pixel-identical to the client-side html2canvas output. We'll match branding (logo, company name, colors) and the same data fields, but page composition is built in code. User confirmed this tradeoff.

## 3. Non-Goals (v1)

- **No WhatsApp bulk** in v1. Phase-2 add-on once email path is proven.
- **No new scheduler.** On-demand admin action, not cron.
- **No template editor changes** for the PDF body (the PDF layout is code-driven; the email body still uses `render_customer_email` for personalization).
- **No CC/BCC** on bulk send.

## 4. Channels (v1)

1. **In-app notification** — always sent to customer's linked users (reuse existing notification path).
2. **Email with PDF attachment** — sent only if customer email present; PDF generated server-side from invoice data.

Per-invoice result: `sent` (at least one channel delivered) | `skipped` (already paid/cancelled, or no email + no notifiable user) | `failed` (error). Bulk run returns aggregated counts + per-item details.

## 5. Phase 1 — PDF Generator (`services/invoice_pdf_service.rs`)

### 5.1 Cargo dep
```toml
printpdf = "0.7"   # pure Rust, A4, fonts via built-in or embedded
```

### 5.2 New service `InvoicePdfService`
- `pub fn render_invoice(&self, ctx: InvoicePdfContext) -> AppResult<Vec<u8>>` — returns raw PDF bytes.
- `InvoicePdfContext` carries: company info (name, address, NPWP, logo path optional), invoice (number, dates, status, currency, line items, subtotal, tax, total, payment URL), customer (name, address, email).

### 5.3 Layout (single page, A4 portrait)
- Header: company logo (if present) + name + address + NPWP — left; "INVOICE" big + invoice_number + status badge — right.
- Sub-header: bill-to (customer name + address + email) — left; dates (issue, due) — right.
- Body: items table (Description, Qty, Unit Price, Subtotal). Currency-aware formatting via existing money helpers (port the relevant formatter to Rust if not present, or pass pre-formatted strings from Rust money utils).
- Footer: subtotal, tax (PPN if set), total — right-aligned. Below: short payment instructions + clickable URL `/pay/{id}` (rendered as text — printpdf supports clickable URI annotations via the `Link` annotation API).
- Fonts: built-in Helvetica family (no external font files needed for v1).

### 5.4 Tests
- Unit: `render_invoice` produces non-empty bytes starting with `%PDF-` magic.
- Snapshot-light: extract first 1KB header + a couple of expected text fragments (invoice_number, total) using a minimal PDF parser or regex on the raw bytes.

## 6. Phase 2 — Email Attachment Plumbing

This is the cross-cutting change. `email_outbox` and all 4 providers must learn attachments.

### 6.1 Schema migration — `email_outbox` attachments
```sql
-- src-tauri/migrations/<ts>_add_email_outbox_attachments.up.sql
CREATE TABLE IF NOT EXISTS email_outbox_attachments (
    id              TEXT PRIMARY KEY,
    outbox_id       TEXT NOT NULL REFERENCES email_outbox(id) ON DELETE CASCADE,
    filename        TEXT NOT NULL,
    content_type    TEXT NOT NULL,
    content_bytes   BYTEA NOT NULL,         -- inline storage; bulk PDFs ~50-200KB each
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_email_outbox_attachments_outbox_id ON email_outbox_attachments(outbox_id);
```
DOWN drops table + index.

Inline BYTEA chosen over filesystem refs to keep retries idempotent and avoid orphaned files. Add an outbox-side cap (e.g. 5 attachments, 5MB total) enforced at enqueue time.

### 6.2 Models
- `EmailOutboxAttachment { id, outbox_id, filename, content_type, content_bytes }`
- New enqueue helper `EmailOutboxItem` doesn't need a column — attachments fetched in sender loop by `outbox_id`.

### 6.3 `EmailOutboxService` extension
- `enqueue_with_attachments(...)` that inserts the outbox row and the attachment rows in one transaction.
- `send_or_enqueue_with_attachments(tenant_id, to, subject, body_text, body_html, attachments)` — when `email_outbox_enabled=true` queues; otherwise calls `EmailService::send_email_with_attachments_for_tenant` directly.
- `process_batch` extended: after fetching the outbox row, fetch its attachments (LEFT JOIN or follow-up query) and pass to provider.

### 6.4 `EmailService` extension — 4 providers
New shape: `EmailAttachment { filename: String, content_type: String, content: Vec<u8> }`.

- `send_via_smtp_with_attachments`: build `Message` with `MultiPart::mixed()` — alternative-text/html part + one `SinglePart::builder().header(ContentType::parse(&att.content_type)).header(ContentDisposition::attachment(&att.filename)).body(att.content)` per attachment.
- `send_via_resend_with_attachments`: Resend JSON `attachments: [{filename, content: base64}]`.
- `send_via_sendgrid_with_attachments`: SendGrid v3 `attachments: [{content: base64, type, filename, disposition: "attachment"}]`.
- `send_via_webhook_with_attachments`: pass `attachments: [{filename, content_type, content_base64}]` in the JSON payload (non-breaking for existing custom hooks — they ignore unknown fields).

Public umbrella: `send_email_with_attachments_for_tenant(tenant_id, to, subject, body_text, body_html, attachments)`.

### 6.5 NotificationService thin wrapper
`force_send_email_with_attachments(tenant_id, to, subject, body_text, body_html, attachments)` → `email_outbox.send_or_enqueue_with_attachments(...)`.

## 7. Phase 3 — Bulk Send API + Service

### 7.1 DTOs (`payment_service/dto.rs`)
```rust
#[derive(Debug, Deserialize)]
pub struct BulkSendInvoiceRequest {
    pub invoice_ids: Vec<String>,
    #[serde(default)]
    pub channels: Option<Vec<String>>,   // ["email","notification"]; default both
    #[serde(default)]
    pub template_id: Option<String>,     // optional email template override
    #[serde(default = "default_attach_pdf")]
    pub attach_pdf: bool,                // default true
}

fn default_attach_pdf() -> bool { true }

#[derive(Debug, Serialize)]
pub struct BulkSendInvoiceItemResult {
    pub invoice_id: String,
    pub invoice_number: String,
    pub status: String,                  // "sent" | "skipped" | "failed"
    pub email_sent: bool,
    pub notification_sent: bool,
    pub pdf_attached: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BulkSendInvoiceResult {
    pub sent_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub items: Vec<BulkSendInvoiceItemResult>,
}
```

### 7.2 Service method (`payment_service/mod.rs`)
`pub async fn bulk_send_invoices(&self, actor_user_id: &str, tenant_id: &str, req: BulkSendInvoiceRequest) -> AppResult<BulkSendInvoiceResult>`

Flow per invoice:
1. Cap input length (max 200) → `Validation` if exceeded.
2. Tenant-scoped fetch invoice + customer + subscription.
3. Skip if status is `paid`/`cancelled` (reason `already_settled`).
4. Build `InvoicePdfContext` from invoice + tenant company settings; render PDF if `attach_pdf=true`.
5. Render email body via `message_template_service.render_customer_email` (or default builder).
6. Email channel (if customer.email present): `notification_service.force_send_email_with_attachments(...)`.
7. Notification channel: existing `notify_subscription_invoice_created`-style helper.
8. Audit-log: one entry per bulk run summary + per-invoice debug entry.
9. Aggregate counts.

Need `InvoicePdfService` injected into `PaymentService` constructor → update `bootstrap/app.rs` + `bin/server.rs` (both entrypoints, per skill rules).

### 7.3 HTTP route (`http/payment.rs`)
- `POST /invoices/bulk-send` → `bulk_send_invoices` handler.
- Auth: token validate + tenant + permission `billing:write` (verify against existing billing-collection routes).
- Register in `Router::new()...` chain.

## 8. Phase 4 — Frontend

### 8.1 API client (`src/lib/api/payment.ts`)
```ts
bulkSendInvoices: (
  invoiceIds: string[],
  opts?: { channels?: string[]; templateId?: string; attachPdf?: boolean }
) => safeInvoke('bulk_send_invoices', { token: getTokenOrThrow(), invoiceIds, ...opts }),
```

### 8.2 commandMap (`src/lib/api/core.ts`) — CRITICAL
```ts
bulk_send_invoices: { method: 'POST', path: '/payment/invoices/bulk-send' },
```

### 8.3 Invoice list page (`src/routes/(app)/admin/invoices/+page.svelte`)
- `selectedInvoiceIds = $state<Set<string>>(new Set())` + checkbox column (header = select-all-on-filtered, exclude paid/cancelled rows).
- "Kirim Invoice" toolbar button (visible when `selectedInvoiceIds.size > 0`, gated `can('billing','write')`), busy state mirrors `bulkGenerating`.
- Confirm dialog → `api.payment.bulkSendInvoices([...])` → result toast → clear selection + reload.
- Optional toggle in confirm dialog: "Lampirkan PDF" (default ON).

### 8.4 i18n (DUAL-FILE, en + id)
Add under `admin.package_invoices.list.bulk_send.*`: `button`, `confirm_title`, `confirm_body`, `attach_pdf`, `sending`, `result_summary`, `select_all`, `none_selected`, `skipped_paid`, `error_partial`. Update both namespace and consolidated locale files.

### 8.5 Icons
Register `send` (or `mail-plus`) in `iconModules.ts` if missing.

## 9. Verification

- `cargo check` — backend compiles (all features).
- `npm run check` — TS/Svelte 0 errors.
- `npm run i18n:check` — 0 missing keys (en + id).
- `npm run test:unit` — existing 594 stay green.
- New tests:
  - Rust: `invoice_pdf_service::render_invoice` produces valid PDF bytes (header check + key text fragments).
  - Rust: `bulk_send_invoices` skips paid/cancelled, respects cap, tenant isolation, partial-failure aggregation.
  - Rust: each provider's `_with_attachments` path round-trips a small PDF (smtp uses a stub transport; others use mock HTTP).
  - Frontend: selection state util test (select-all-on-filtered, exclude paid).

## 10. Phases (commit per phase)

1. **`feat: invoice PDF generator (printpdf)`** — Cargo dep + `InvoicePdfService` + tests.
2. **`feat: email attachments — outbox schema + 4 providers`** — migration + models + outbox enqueue/sender + EmailService 4 provider extensions + NotificationService wrapper + tests.
3. **`feat: bulk send invoice backend`** — DTOs + `bulk_send_invoices` + route + bootstrap wiring + tests.
4. **`feat: bulk send invoice UI`** — selection state, send button, confirm dialog with attach-pdf toggle, result toast.
5. **`chore: i18n + icons for bulk send invoice`** — keys (dual-file en+id) + icon registration.
6. **`chore: verify bulk send invoice gates`** — gate run + fix fallout if any.

## 11. Risks & Mitigations

- **PDF size in Postgres:** typical invoice PDF ~50–200KB. Bulk of 200 = up to ~40MB temporarily in `email_outbox_attachments`. Cleaned up by existing outbox retention (or add explicit cleanup if not present). Acceptable.
- **printpdf font support:** built-in Helvetica covers ASCII; for Indonesian (Rp, é, etc.) Helvetica + Latin-1 should suffice. If non-ASCII characters appear in customer names/items and don't render, embed a TTF font (e.g. DejaVu Sans). Verify in tests with sample Indonesian data.
- **SendGrid/Resend payload size:** base64-encoding a 200KB PDF → ~270KB JSON. Both providers allow far more. Fine.
- **Bulk concurrency:** 200 invoices × PDF render is CPU work. Run sequentially to start; if too slow, switch to a small `tokio::task::JoinSet` with concurrency cap (e.g. 4). Measure first.
- **Tenant isolation regression:** add explicit assertion test that an invoice from another tenant included in `invoice_ids` is rejected (not silently skipped).

## 12. Open Decisions (defaults chosen)

- **Channels default:** both email + notification. [chosen]
- **Attach PDF default:** true. [chosen]
- **Permission:** `billing:write`. [confirm in phase 3]
- **Max batch:** 200. [chosen]
- **Concurrency:** sequential v1. [chosen]
- **Font:** built-in Helvetica v1; embed DejaVu if needed for non-ASCII. [conditional]
