# Plan: Bulk Send Invoice via WhatsApp (Channel ke-3)

## Status: ✅ SELESAI (2026-05-31)
- Phase 1 backend — commit `59ab6bc` (force_send_whatsapp + phone resolution + WA branch + DTO). cargo check clean.
- Phase 2 frontend — commit `b97d92d` (channel-picker Modal + types + 7 i18n keys x4 file). svelte-check 0 error, i18n:check 0 missing.
- Phase 3 verify — contract test live: API terima `channels:["whatsapp"]`, return `whatsapp_sent` field, HTTP 200 (bukan 400/deserialize). End-to-end real-send NOT dijalankan karena `wa_gateway_enabled=false` (master switch) + hindari kirim WA beneran tanpa konfirmasi. Provider terkonfigurasi: triwax.

**Goal:** Tambah channel WhatsApp ke fitur bulk-send invoice yang sudah ada. Admin bisa pilih kirim invoice ke pelanggan via WhatsApp (selain email + in-app notification yang sudah jalan). Per-invoice result tetap `sent`/`skipped`/`failed` dengan flag `whatsapp_sent`.

**Status infra existing (verified 2026-05-31):**
- ✅ `WhatsappGatewayService` lengkap: `send_text(tenant_id, event_code, recipient_user_id, phone, message)` + Fonnte/Triwax/custom-HTTP provider + auto delivery logging (`whatsapp_delivery_logs`).
- ✅ `NotificationService` SUDAH membungkus `WhatsappGatewayService` via `new_with_whatsapp` (field `whatsapp_gateway: Option<...>`). Dipakai di bootstrap/app.rs + bin/server.rs.
- ✅ `PaymentService` sudah punya field `notification_service`. **TIDAK perlu ubah `PaymentService::new`** (dipanggil 6 tempat) — WA dijangkau lewat notification_service.
- ✅ bulk-send backend (`bulk_send_invoices` → `send_one_invoice`) + frontend (checkbox, toolbar, confirm) sudah jalan untuk email+notif.
- ✅ `normalize_phone()` handle `08xxx` → `628xxx` dan `+62` → `62`.

---

## Arsitektur Keputusan

**Jangan** ubah `PaymentService::new` (6 call sites). **Jangan** inject `WhatsappGatewayService` langsung ke PaymentService.
**Pakai** jalur: `PaymentService` → `self.notification_service` → thin method baru `force_send_whatsapp(...)` → `whatsapp_gateway.send_text(...)`.

Ini konsisten dengan pola email yang sudah ada (`force_send_email_with_attachments` ada di NotificationService, bukan di PaymentService).

Event code untuk bulk-send invoice WA: `customer_invoice_due` (sudah dipakai `whatsapp_event_code_for_category` untuk kategori billing). **Catatan penting:** `send_text` TIDAK gate by `is_event_whatsapp_enabled` — pengiriman eksplisit dari admin action harus tetap terkirim walau toggle event WA off (itu untuk auto-notification, bukan untuk explicit bulk action). Jadi panggil `send_text` langsung, bukan lewat `deliver_whatsapp_notification`.

---

## Phase 1 — Backend: NotificationService pass-through + phone resolution

### 1.1 `notification_service.rs` — tambah method publik
```rust
/// Explicit WhatsApp send for admin-triggered actions (bulk invoice send).
/// Unlike deliver_whatsapp_notification, this does NOT gate on the per-event
/// WhatsApp toggle — an explicit admin action should always attempt delivery.
/// Returns Ok(false) if WA gateway not configured / disabled at provider level.
pub async fn force_send_whatsapp(
    &self,
    tenant_id: Option<&str>,
    event_code: &str,
    recipient_user_id: Option<&str>,
    phone: &str,
    message: &str,
) -> AppResult<bool> {
    let Some(gw) = &self.whatsapp_gateway else { return Ok(false); };
    match gw.send_text(tenant_id, event_code, recipient_user_id, phone, message).await {
        Ok(()) => Ok(true),
        Err(e) => { tracing::warn!("force_send_whatsapp failed: {e}"); Ok(false) }
    }
}
```
- `send_text` sendiri sudah return `AppResult<()>` (sent/failed sudah di-log). Kalau provider return non-2xx, `send_text` map ke status "failed" di log tapi tetap Ok-kah? **CEK saat implementasi**: `send_text_response` selalu `Ok(WhatsappTestSendResponse{ ok, ... })` kecuali config/build error. `send_text` map `.map(|_| ())` — jadi Ok bahkan kalau delivery gagal di provider. Untuk akurasi `whatsapp_sent`, panggil `send_text_response` dan baca `.ok` field, bukan `send_text`.

**Revisi 1.1:** pakai `send_text_response` supaya tahu hasil delivery sebenarnya:
```rust
let resp = gw.send_text_response(tenant_id, event_code, recipient_user_id, phone, message).await?;
Ok(resp.ok)
```

### 1.2 `payment_service/mod.rs` — `InvoiceCustomerLink` + phone
- Tambah field `customer_phone: Option<String>` ke struct `InvoiceCustomerLink` (line ~6705).
- Path 1 query (CSA): `SELECT csa.subscription_id, csa.customer_id, c.email, c.name, c.phone` — tambah `c.phone`, ubah tuple jadi 5 elemen.
- Path 2 fallback (external_id → customer): `SELECT email, name, phone FROM customers ...` — tambah `phone`, ubah tuple jadi 3 elemen.
- Update kedua `return Ok(InvoiceCustomerLink { ... })` untuk isi `customer_phone`.

### 1.3 DTO — tambah channel + flag
File `payment_service/dto.rs`:
- `BulkSendInvoiceItemResult`: tambah `pub whatsapp_sent: bool,`.
- (channels sudah `Option<Vec<String>>` — "whatsapp" string baru, tidak perlu ubah tipe.)
- Default channels saat omitted: pertimbangkan apakah WA ikut default. **Keputusan:** default tetap `["email","notification"]` (tidak ubah perilaku existing). WA hanya terkirim kalau admin eksplisit pilih. Hindari kejutan kirim WA massal.

### 1.4 `send_one_invoice` — WA channel branch
- Tambah param `want_whatsapp: bool` ke signature.
- Ambil `customer_phone` dari link (sejajar `customer_email`).
- Setelah blok notification, tambah:
```rust
let mut whatsapp_sent = false;
if want_whatsapp {
    let phone = customer_phone.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if let Some(p) = phone {
        let msg = format!(
            "Halo {cust},\n\nInvoice {num} sebesar {cur} {amt:.2} sudah terbit.\nJatuh tempo: {due}\nBayar online: /pay/{id}\n\nTerima kasih.",
            cust = customer_name.as_deref().unwrap_or("Pelanggan"),
            num = invoice.invoice_number, cur = invoice.currency_code,
            amt = invoice.amount, due = invoice.due_date.format("%Y-%m-%d"), id = invoice.id,
        );
        whatsapp_sent = self.notification_service
            .force_send_whatsapp(Some(tenant_id), "customer_invoice_due", None, p, &msg)
            .await.unwrap_or(false);
    }
}
```
- Update status resolution: `sent` kalau `email_sent || notification_sent || whatsapp_sent`. Tambah `no_wa_target = want_whatsapp && customer_phone kosong` ke kondisi skipped.
- Update semua `BulkSendInvoiceItemResult { ... }` literal (ada beberapa early-return) untuk isi `whatsapp_sent` (false di early returns).

### 1.5 `bulk_send_invoices` — teruskan want_whatsapp
- Parse `want_whatsapp = channels.iter().any(|c| c == "whatsapp")`.
- Validasi minimal 1 channel: update jadi `!want_email && !want_notification && !want_whatsapp`.
- Teruskan `want_whatsapp` ke `send_one_invoice`.
- Tambah `want_whatsapp` ke audit summary JSON.

### 1.6 Verify backend
```bash
cd ~/ISPMANAGEMENT/src-tauri && cargo check
# tunggu cargo watch recompile via tauri dev, atau touch file
```

---

## Phase 2 — Frontend: pilihan channel di confirm dialog

### 2.1 API client (`src/lib/api/payment.ts`)
- `BulkSendInvoiceRequest` type: tambah `channels?: string[]`.
- `BulkSendInvoiceItemResult` type: tambah `whatsapp_sent: boolean`.

### 2.2 Invoice page (`src/routes/(app)/admin/invoices/+page.svelte`)
- Saat ini `bulkSendSelectedInvoices` hardcode `{ invoice_ids, attach_pdf: true }` (default channels email+notif di backend).
- **Opsi A (minimal, direkomendasikan untuk v1):** tambah tombol kedua "Kirim via WhatsApp" di toolbar yang panggil `bulkSendInvoices({ invoice_ids, channels: ['whatsapp'], attach_pdf: false })`. Tombol existing tetap email+notif.
- **Opsi B (lebih kaya):** ganti `window.confirm` dengan `Modal.svelte` berisi checkbox channel (Email / In-app / WhatsApp) + toggle Lampirkan PDF. Lebih sesuai preferensi user (dynamic editable forms) tapi lebih besar.
- **Default plan: Opsi B** — user lebih suka form dinamis. Bikin `Modal` dengan 3 checkbox channel (default email+notif checked, WA unchecked) + toggle attach PDF (gated: hanya relevan kalau email checked). Submit kirim `channels` array sesuai pilihan.
- Toast summary: tambah hitungan WA kalau channel WA dipakai.

### 2.3 i18n (DUAL-FILE: namespaces + locales, id + en)
Tambah keys di `admin.package_invoices.list.actions` / `.bulk_send`:
- `channel_email`, `channel_notification`, `channel_whatsapp`
- `bulk_send_modal_title`, `bulk_send_modal_body`
- `attach_pdf_label`
- `bulk_send_via_wa` (kalau Opsi A)
- toast: `bulk_sent_stats_wa` kalau perlu format dengan WA count
Update **4 file**: `namespaces/{id,en}/admin.json` + `locales/{id,en}.json`. Verify `npm run i18n:check` = 0 missing.

### 2.4 Icon
Pastikan icon `message-circle` (atau `send`) terdaftar di `iconModules.ts` untuk tombol WA. `message-circle` ada di daftar alias targets — cek dual-registration.

---

## Phase 3 — Verifikasi

### 3.1 Gates
```bash
npm run check            # svelte-check, 0 new error
npm run i18n:check       # 0 missing (en+id)
cd src-tauri && cargo check
```

### 3.2 Live test via curl (pakai pola yang terbukti)
- Login → token (`requests.post` via execute_code, hindari curl token masking).
- Cek WA gateway config aktif untuk tenant test (`wa_gateway_provider`, token). Kalau belum dikonfigurasi, test akan return `whatsapp_sent:false` — itu BUKAN bug, perlu provider Fonnte/Triwax disetel dulu di Settings.
- Query 1-2 invoice pending yang customernya punya `phone` non-null.
- `POST /api/payment/invoices/bulk-send` body `{"invoice_ids":["..."],"channels":["whatsapp"],"attach_pdf":false}`.
- Verifikasi response `whatsapp_sent:true` + cek tabel `whatsapp_delivery_logs` ada row baru status `sent`.

### 3.3 Cek data prasyarat
```sql
-- invoice pending dengan customer phone
SELECT i.id, i.invoice_number, c.phone
FROM invoices i
-- linkage via external_id pkgsub: → subscription → customer
WHERE i.status='pending' AND c.phone IS NOT NULL LIMIT 5;
```
Kalau mayoritas customer `phone` NULL → WA bulk akan banyak `skipped:no_contact_path`. Itu data issue, bukan bug (sama seperti email kemarin).

---

## Phases (commit per phase)
1. `feat(bulk-send): add whatsapp channel — backend (notif passthrough + phone resolution + send_one_invoice branch)`
2. `feat(bulk-send): whatsapp channel picker UI + i18n`
3. `chore(bulk-send): verify whatsapp channel gates`

## Pitfalls (dari skill isp-management-dev)
- **`customers` pakai kolom `phone`** (bukan `phone_number`) — verifikasi `\d customers` sebelum query. Skema: `id, tenant_id, name, email, phone, notes, is_active, ...`.
- **DUAL-FILE i18n**: update `namespaces/` DULU (runtime) lalu `locales/` (lint). Verify both via search_files.
- **Tauri-IPC vs HTTP body shape**: handler `bulk_send_invoices` sudah handle wrapped `{request:...}` + flat. Tidak ubah handler signature.
- **`send_text` vs `send_text_response`**: `send_text` return Ok walau provider gagal. Pakai `send_text_response().ok` untuk akurasi flag `whatsapp_sent`.
- **Jangan gate explicit admin send dengan `is_event_whatsapp_enabled`** — itu untuk auto-notification. Bulk action eksplisit harus selalu attempt.
- **Recompile ~3 menit** via cargo watch (tauri dev). Verify binary mtime + "Finished dev profile".
- **Pisahkan cargo fmt noise** dari commit feature.
- **Report check/i18n delta, bukan absolute** — ada pre-existing debt dari parallel agents.
