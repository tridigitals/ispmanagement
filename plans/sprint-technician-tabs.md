# Sprint — Technician Tab Restructuring ✅ PLANNING

> **For Hermes:** Execute sequentially — commit per task.

**Goal:** Ganti tab APK teknisi dari copy-an customer (Home, Subscriptions, Invoices, Support) menjadi tab khusus teknisi: Home (dashboard stats) + Tickets (work orders).

**Architecture:** 2-tab HomeShell dengan IndexedStack. Tab Home menampilkan statistik tiket + recent tickets. Tab Tickets menampilkan list tiket yang di-assigned + full filter (status/priority). Router tetap nested di bawah `/` HomeShell.

**Tech Stack:** Flutter/Dart, Riverpod, GoRouter, api_client package

---

## Current State Audit

| File | Current | Target |
|------|---------|--------|
| `home_shell.dart` | 4 tabs: Home, Subscriptions, Invoices, Support | 2 tabs: Home, Tickets |
| `home_tab.dart` | Subscription card + invoices + network + announcement | Ticket stats cards + recent tickets + network + announcement |
| `support_tab.dart` | List all tickets (customer view) | ❌ Hapus, ganti `tickets_tab.dart` |
| `subscriptions_tab.dart` | Customer subscriptions | ❌ Hapus |
| `invoices_tab.dart` | Customer invoices | ❌ Hapus |
| `app_router.dart` | Full routes incl subscriptions/invoices/payments | Hapus route subscription/invoice/payment |
| `auth_providers.dart` | Role gate `isStaff` ✅ | Tidak perlu ubah |
| `ticket_service.dart` | `list()`, `stats()`, `update()`, `reply()` ✅ | Tidak perlu ubah |
| `TicketStats` model | `all`, `open`, `pending`, `closed` ✅ | Tidak perlu ubah |

---

## Tasks

### Task 1: Rewrite `home_tab.dart` — Technician Dashboard

**Files:**
- Modify: `apps/mobile-technician/lib/src/features/home/home_tab.dart`

**What changes:**
- Hapus `_PrimarySubscription` widget, subscription card, invoices section
- Tambah `_TicketStatsRow` — 4 card kecil (Semua, Open, Pending, Closed) dari `ticketService.stats()`
- Tambah `_RecentTickets` — list 5 tiket terbaru yang assigned ke teknisi, dengan tap → `/tickets/:id`
- Keep: `NetworkStatusBanner`, `AnnouncementBanner`
- Provider: `ticketServiceProvider`, `ticketStatsProvider` (future provider), inline FutureBuilder atau stateful

### Task 2: Create `tickets_tab.dart` — Technician Work Orders

**Files:**
- Create: `apps/mobile-technician/lib/src/features/home/tickets_tab.dart`

**Spec:**
- List tiket dengan filter chips: Semua, Open, In Progress, Waiting Customer, Resolved
- Filter priority: Low, Medium, High, Critical
- Tiap tiket: subject, status badge, priority color, customer name, last update
- Tap → push `/tickets/:id`
- Pull-to-refresh + infinite scroll (pagination)
- FAB → `/tickets/new` (teknisi bisa buka tiket baru)
- Gunakan `TicketService.list()` — backend sudah handle filter `assigned_to` untuk role staff

### Task 3: Edit `home_shell.dart` — 2 Tabs

**Files:**
- Modify: `apps/mobile-technician/lib/src/features/home/home_shell.dart`

**Changes:**
- Hapus import `invoices_tab.dart`, `subscriptions_tab.dart`
- Tambah import `tickets_tab.dart`
- `pages` = `const [HomeTab(), TicketsTab()]` (dari 4 jadi 2)
- `tabTitles` = `[greeting, l10n.ticketsLabel]` (dari 5 jadi 2)
- Guard: `tabIdx < 4` → `tabIdx < 2`
- Destinations: Home icon + Tickets icon ( `Icons.assignment` / `Icons.confirmation_number_outlined` )
- FAB: `tab == 1` (bukan tab == 3)
- `_normalizeAction`: hapus referensi `/pay/`, `/invoices`, `/subscriptions` — ganti ke `/tickets/:id`

### Task 4: Update Router — Remove Customer Routes

**Files:**
- Modify: `apps/mobile-technician/lib/src/router/app_router.dart`

**Changes:**
- Hapus import `subscription_detail_screen.dart`, `invoice_detail_screen.dart`, `payment_screen.dart`, `payment_webview_screen.dart`, `payment_instruction_screen.dart`
- Hapus route: `subscriptions/:id`, `invoices/:id`, `payments/:invoiceId`, `payments/:invoiceId/webview`, `payments/:invoiceId/:transactionId/instructions`
- Keep: semua route lain (profile, settings, notifications, tickets, announcements, faq, contact)

### Task 5: Update l10n — Tambah Label Baru

**Files:**
- Modify: `apps/mobile-technician/lib/src/l10n/app_en.arb`
- Modify: `apps/mobile-technician/lib/src/l10n/app_id.arb`

**New strings:**
- `ticketsLabel`: "Tickets" / "Tiket"
- `ticketStatsAll`: "All" / "Semua"  
- `ticketStatsOpen`: "Open" / "Buka"
- `ticketStatsPending`: "Pending" / "Tertunda"
- `ticketStatsClosed`: "Closed" / "Selesai"
- `recentTickets`: "Recent Tickets" / "Tiket Terbaru"
- `noAssignedTickets`: "No assigned tickets" / "Tidak ada tiket"

### Task 6: Cleanup — Hapus File Tidak Dipakai

**Files:**
- Delete: `apps/mobile-technician/lib/src/features/home/subscriptions_tab.dart`
- Delete: `apps/mobile-technician/lib/src/features/home/invoices_tab.dart`
- Keep: `support_tab.dart` (untuk referensi, bisa dihapus nanti)

### Task 7: Regenerate l10n + Build & Deploy APK

**Commands:**
```bash
cd /home/xtrabit/ISPMANAGEMENT/apps/mobile-technician
flutter gen-l10n
cd /home/xtrabit/ISPMANAGEMENT/apps/mobile-technician/scripts
./build-release.sh
cp build/app/outputs/flutter-apk/app-arm64-v8a-release.apk ~/apk-server/mobile-technician-arm64.apk
```

### Task 8: Commit per Feature

```
feat(technician): redesign home tab for technician dashboard
feat(technician): add tickets tab with status/priority filters
refactor(technician): switch to 2-tab HomeShell (Home + Tickets)
refactor(technician): remove customer routes from router
chore(technician): add technician l10n labels
chore(technician): remove unused customer tab files
```

---

## Verification Checklist

- [ ] APK terinstall tanpa crash
- [ ] Login teknisi berhasil, masuk ke Home
- [ ] Home tab menampilkan statistik tiket + recent tickets
- [ ] Tickets tab menampilkan list tiket dengan filter
- [ ] Tap tiket → detail screen
- [ ] FAB → new ticket screen
- [ ] Notifikasi bell tetap berfungsi
- [ ] Profile & Settings bisa diakses dari header
- [ ] Session persist setelah app kill (regression check)
- [ ] FCM notifikasi tetap masuk

---

## Scope Exclusions (Future)

- GPS tracking toggle/location recording
- Ticket photo upload dari teknisi
- Map view untuk lokasi pelanggan
- Offline mode / queue
