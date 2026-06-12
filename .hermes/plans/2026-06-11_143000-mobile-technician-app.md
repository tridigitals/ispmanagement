# Mobile Technician App — Implementation Plan

> **Goal:** Repurpose `apps/mobile-admin` → `apps/mobile-technician` (field technician app for work order management, ticket handling, and installation workflows)
>
> **Stack:** Flutter 3.24+ / Dart 3.5+ / Riverpod / GoRouter / Dio
>
> **Monorepo:** `ISPMANAGEMENT` (melos workspace)

---

## Context

### What exists
| Component | Status | Notes |
|-----------|--------|-------|
| `apps/mobile-admin` | 🔴 Skeleton | UI stubs, no API integration, admin-focused nav |
| `packages/api-client` | 🟡 Partial | Has auth, tickets, notifications, storage services. **Missing:** work order models + service |
| `packages/ui-kit` | ✅ Ready | 14 reusable widgets (cards, badges, buttons, etc.) |
| Backend work orders API | ✅ Ready | `/api/admin/work-orders` — list, assign, claim, release, start, complete, cancel, reopen, reschedule |
| Backend support tickets API | ✅ Ready | `/api/support/tickets` — list, create, reply, stats, satisfaction |
| Backend storage API | ✅ Ready | `/api/storage/files/{id}/content` — upload/serve |
| Backend notifications API | ✅ Ready | `/api/notifications` — list, read, unread-count |

### Backend API endpoints (technician needs)
```
# Work Orders (requires tenant JWT + support:read permission or assigned_to filter)
GET    /api/admin/work-orders?assigned_to={user_id}&status={status}
POST   /api/admin/work-orders/{id}/claim
POST   /api/admin/work-orders/{id}/start
POST   /api/admin/work-orders/{id}/complete   {notes, terminal_asset_id, parent_asset_id}
POST   /api/admin/work-orders/{id}/cancel     {notes}
GET    /api/admin/work-orders/{id}/reschedule-request

# Support Tickets (requires support:read or support:read_all)
GET    /api/support/tickets?status={status}
GET    /api/support/tickets/stats
GET    /api/support/tickets/{id}
POST   /api/support/tickets/{id}/messages     {message, is_internal, attachment_ids}

# Auth
POST   /api/auth/login
GET    /api/auth/me
POST   /api/auth/change-password

# Storage
POST   /api/storage/files                     (multipart upload)
GET    /api/storage/files/{id}/content

# Notifications
GET    /api/notifications
GET    /api/notifications/unread-count
POST   /api/notifications/{id}/read
```

### Work Order status flow
```
pending → (claim) → assigned → (start) → in_progress → (complete) → completed
                                                           ↕
                                                       (cancel) → cancelled
                                                       (reopen) → pending
```

---

## Phased Plan

### Phase 0: Rename + Restructure
Rename `mobile-admin` → `mobile-technician`, update pubspec, melos, and asset references.

| Task | Status | Notes |
|------|--------|-------|
| 0.1 Rename app directory `mobile-admin` → `mobile-technician` | ⏳ | |
| 0.2 Update `pubspec.yaml` (name: `mobile_technician`, description) | ⏳ | |
| 0.3 Update `melos.yaml` build scripts | ⏳ | Add `build:technician:android` |
| 0.4 Update Android `applicationId` + iOS bundle ID | ⏳ | `com.isp.technician` |
| 0.5 Clean up admin-only imports in `app.dart` | ⏳ | |

### Phase 1: API Client — Work Order Models + Service
Add work order support to `packages/api-client`.

| Task | Status | Notes |
|------|--------|-------|
| 1.1 Create `work_order_model.dart` | ⏳ | `WorkOrderModel`, `WorkOrderView` (mirror Rust `InstallationWorkOrderView`) |
| 1.2 Create `work_order_service.dart` | ⏳ | Dio service: list, get, claim, start, complete, cancel, reschedule |
| 1.3 Add work order endpoints to `api_endpoints.dart` | ⏳ | `/api/admin/work-orders/*` |
| 1.4 Run `build_runner` for JSON serialization | ⏳ | `dart run build_runner build` |
| 1.5 Add `storage_service.dart` upload method | ⏳ | Multipart upload for photos |

**Files:**
- `packages/api-client/lib/src/models/work_order_model.dart` (new)
- `packages/api-client/lib/src/services/work_order_service.dart` (new)
- `packages/api-client/lib/src/services/storage_service.dart` (update)
- `packages/api-client/lib/src/api/api_endpoints.dart` (update)
- `packages/api-client/lib/api_client.dart` (update barrel)

### Phase 2: App Shell + Navigation
Replace admin shell with technician-focused bottom nav.

| Task | Status | Notes |
|------|--------|-------|
| 2.1 New `TechnicianShell` with 4 tabs | ⏳ | Home, Work Orders, Tickets, Profile |
| 2.2 Update `app_router.dart` | ⏳ | Remove `/customers`, `/announcements` routes. Add `/work-orders`, `/work-orders/:id` |
| 2.3 Update `app.dart` theme + branding | ⏳ | Technician-specific colors/naming |

**Tab structure:**
```
🏠 Home           → DashboardScreen (tech-focused)
📋 Work Orders    → WorkOrderListScreen → WorkOrderDetailScreen
🎫 Tickets        → TicketListScreen → TicketDetailScreen
👤 Profil         → ProfileScreen + SettingsScreen
```

**Files:**
- `apps/mobile-technician/lib/src/features/home/technician_shell.dart` (new)
- `apps/mobile-technician/lib/src/router/app_router.dart` (rewrite)
- `apps/mobile-technician/lib/src/app.dart` (update)

### Phase 3: Auth + Profile
Connect login to real API, add profile management.

| Task | Status | Notes |
|------|--------|-------|
| 3.1 Connect `LoginScreen` to `AuthService.login()` | ⏳ | Riverpod providers for auth state |
| 3.2 Token storage via `flutter_secure_storage` | ⏳ | Reuse `AuthTokenStorage` from api-client |
| 3.3 Update `auth_providers.dart` | ⏳ | Proper auth state with Riverpod AsyncNotifier |
| 3.4 Profile screen — show user info from `/api/auth/me` | ⏳ | |
| 3.5 Change password flow | ⏳ | `/api/auth/change-password` |

**Files:**
- `apps/mobile-technician/lib/src/features/auth/login_screen.dart` (rewrite)
- `apps/mobile-technician/lib/src/features/profile/` (update)
- `apps/mobile-technician/lib/src/services/auth_providers.dart` (rewrite)

### Phase 4: Dashboard (Home Tab)
Technician-focused home screen with today's schedule and stats.

| Task | Status | Notes |
|------|--------|-------|
| 4.1 Dashboard stats cards | ⏳ | Assigned WO count, pending tickets, completed today |
| 4.2 Today's schedule list | ⏳ | Work orders with `scheduled_at` = today |
| 4.3 Recent tickets widget | ⏳ | Last 3 assigned tickets |
| 4.4 Pull-to-refresh | ⏳ | |

**Data sources:**
- `GET /api/admin/work-orders?assigned_to=me` → filter `scheduled_at` for today
- `GET /api/support/tickets/stats` → ticket counts

**Files:**
- `apps/mobile-technician/lib/src/features/dashboard/dashboard_screen.dart` (rewrite)
- `apps/mobile-technician/lib/src/features/dashboard/` (add providers, widgets)

### Phase 5: Work Orders (Core Feature)
The main feature — list, view, and manage work orders.

| Task | Status | Notes |
|------|--------|-------|
| 5.1 `WorkOrderListScreen` with status tabs | ⏳ | Tabs: Semua, Menunggu, Dikerjakan, Selesai |
| 5.2 `WorkOrderDetailScreen` | ⏳ | Full WO info: customer, location, package, schedule, status |
| 5.3 Status action buttons | ⏳ | Contextual: Claim, Start, Complete, Cancel |
| 5.4 Work order providers (Riverpod) | ⏳ | List provider, detail provider, action mutations |
| 5.5 Google Maps / location link | ⏳ | Open customer location in maps via `url_launcher` |
| 5.6 Notes input for status changes | ⏳ | Bottom sheet with text field for complete/cancel notes |

**Status actions mapping:**
```
pending   → [Claim]
assigned  → [Start, Cancel]
in_progress → [Complete, Cancel]
completed → [Reopen] (if allowed)
```

**Files:**
- `apps/mobile-technician/lib/src/features/work_orders/work_order_list_screen.dart` (new)
- `apps/mobile-technician/lib/src/features/work_orders/work_order_detail_screen.dart` (new)
- `apps/mobile-technician/lib/src/features/work_orders/widgets/` (new)
- `apps/mobile-technician/lib/src/services/work_order_providers.dart` (new)

### Phase 6: Installation Completion Flow
The critical field workflow — complete an installation with proof.

| Task | Status | Notes |
|------|--------|-------|
| 6.1 Photo capture (camera) | ⏳ | `image_picker` for camera capture |
| 6.2 Photo gallery picker | ⏳ | Multiple photos support |
| 6.3 Photo preview + upload | ⏳ | Upload to `/api/storage/files`, attach file IDs |
| 6.4 Notes field | ⏳ | Required notes for completion |
| 6.5 Equipment selection (ONT/ONU) | ⏳ | Select `terminal_asset_id` from available assets |
| 6.6 Customer signature canvas | ⏳ | `signature` package — draw + save as image |
| 6.7 Completion summary screen | ⏳ | Review all data before submit |
| 6.8 Submit to `/api/admin/work-orders/{id}/complete` | ⏳ | With notes + asset IDs |

**Completion payload:**
```json
{
  "notes": "Instalasi berhasil, ONT terpasang di ruang tamu",
  "terminal_asset_id": "uuid-of-ont",
  "parent_asset_id": null
}
```

**New dependency:** `signature: ^5.4.0` (signature pad widget)

**Files:**
- `apps/mobile-technician/lib/src/features/work_orders/installation_complete_screen.dart` (new)
- `apps/mobile-technician/lib/src/features/work_orders/widgets/photo_capture_widget.dart` (new)
- `apps/mobile-technician/lib/src/features/work_orders/widgets/signature_pad_widget.dart` (new)

### Phase 7: Tickets (Technician View)
View and reply to assigned support tickets.

| Task | Status | Notes |
|------|--------|-------|
| 7.1 Update `TicketListScreen` with real API | ⏳ | Connect to `ticket_service.dart`, show assigned tickets |
| 7.2 Update `TicketDetailScreen` with real data | ⏳ | Load messages, show chat thread |
| 7.3 Reply functionality | ⏳ | POST `/api/support/tickets/{id}/messages` |
| 7.4 Photo attachment in replies | ⏳ | Upload photo → attach to message |
| 7.5 Ticket status chips | ⏳ | Use `IspStatusBadge` from ui-kit |
| 7.6 Internal notes toggle | ⏳ | Staff-only notes vs customer-visible replies |

**Files:**
- `apps/mobile-technician/lib/src/features/tickets/ticket_list_screen.dart` (rewrite)
- `apps/mobile-technician/lib/src/features/tickets/ticket_detail_screen.dart` (rewrite)
- `apps/mobile-technician/lib/src/services/ticket_providers.dart` (new)

### Phase 8: Notifications + FCM
Push notifications for new assignments and ticket replies.

| Task | Status | Notes |
|------|--------|-------|
| 8.1 FCM setup (reuse pattern from mobile-customer) | ⏳ | `firebase_core` + `firebase_messaging` |
| 8.2 Notification inbox screen | ⏳ | Connect to `/api/notifications` |
| 8.3 Deep linking from notification | ⏳ | Navigate to WO/ticket detail |
| 8.4 Unread badge on tab | ⏳ | Badge on Tickets tab |

**Files:**
- `apps/mobile-technician/lib/src/services/fcm_service.dart` (new)
- `apps/mobile-technician/lib/src/features/notifications/notification_inbox_screen.dart` (update)
- `apps/mobile-technician/lib/src/services/notifications_providers.dart` (new)

### Phase 9: Polish + Production Readiness

| Task | Status | Notes |
|------|--------|-------|
| 9.1 Dark mode support | ⏳ | Verify all screens work in dark mode |
| 9.2 Error states + empty states | ⏳ | Use `IspErrorState`, `IspEmptyState` from ui-kit |
| 9.3 Loading states (shimmer) | ⏳ | Use `IspShimmer` from ui-kit |
| 9.4 L10n (id + en) | ⏳ | Reuse existing l10n pattern |
| 9.5 Offline support (basic) | ⏳ | Cache last work order list, show offline banner |
| 9.6 APK build script update | ⏳ | Add `build:technician:android` to `melos.yaml` |
| 9.7 App icon + splash screen | ⏳ | Technician-specific branding |

---

## Dependency Graph

```
Phase 0 (rename)
    ↓
Phase 1 (api-client) ← can start immediately after Phase 0
    ↓
Phase 2 (shell) + Phase 3 (auth) ← parallel after Phase 1
    ↓
Phase 4 (dashboard) ← after Phase 2 + 3
    ↓
Phase 5 (work orders) ← after Phase 4
    ↓
Phase 6 (completion flow) ← after Phase 5
    ↓
Phase 7 (tickets) ← parallel with Phase 5/6
    ↓
Phase 8 (notifications) ← after Phase 5 + 7
    ↓
Phase 9 (polish) ← after all
```

---

## Files to Change (Summary)

### New files
```
packages/api-client/lib/src/models/work_order_model.dart
packages/api-client/lib/src/services/work_order_service.dart
apps/mobile-technician/lib/src/features/home/technician_shell.dart
apps/mobile-technician/lib/src/features/work_orders/work_order_list_screen.dart
apps/mobile-technician/lib/src/features/work_orders/work_order_detail_screen.dart
apps/mobile-technician/lib/src/features/work_orders/installation_complete_screen.dart
apps/mobile-technician/lib/src/features/work_orders/widgets/photo_capture_widget.dart
apps/mobile-technician/lib/src/features/work_orders/widgets/signature_pad_widget.dart
apps/mobile-technician/lib/src/services/work_order_providers.dart
apps/mobile-technician/lib/src/services/ticket_providers.dart
apps/mobile-technician/lib/src/services/fcm_service.dart
apps/mobile-technician/lib/src/services/notifications_providers.dart
```

### Modified files
```
packages/api-client/lib/src/api/api_endpoints.dart
packages/api-client/lib/src/services/storage_service.dart
packages/api-client/lib/api_client.dart
apps/mobile-technician/pubspec.yaml
apps/mobile-technician/lib/src/app.dart
apps/mobile-technician/lib/src/router/app_router.dart
apps/mobile-technician/lib/src/features/auth/login_screen.dart
apps/mobile-technician/lib/src/features/dashboard/dashboard_screen.dart
apps/mobile-technician/lib/src/features/tickets/ticket_list_screen.dart
apps/mobile-technician/lib/src/features/tickets/ticket_detail_screen.dart
apps/mobile-technician/lib/src/features/profile/admin_profile_screen.dart
apps/mobile-technician/lib/src/services/auth_providers.dart
apps/mobile-technician/lib/src/features/notifications/notification_inbox_screen.dart
melos.yaml
```

### Deleted/renamed
```
apps/mobile-admin/ → apps/mobile-technician/
apps/mobile-technician/lib/src/features/home/admin_shell.dart (→ technician_shell.dart)
apps/mobile-technician/lib/src/features/customers/ (remove — not needed for tech)
apps/mobile-technician/lib/src/features/announcements/ (remove — not needed for tech)
```

---

## New Dependencies (pubspec.yaml)

```yaml
dependencies:
  # ... existing ...
  signature: ^5.4.0          # Customer signature capture
  geolocator: ^13.0.1        # GPS location for check-in (optional Phase 2)
  google_maps_flutter: ^2.10.0  # Map view (optional Phase 2)
```

---

## Risks + Tradeoffs

1. **Backend permission model** — Technician users need `support:read` + work order access. Verify role setup in backend before building.
2. **Work order list API** — Current `list_work_orders` uses `has_permission` check. Technicians may only see assigned WOs via `assigned_to` filter. Confirm backend behavior.
3. **Photo upload size** — Camera photos can be 5-10MB. Need compression before upload (use `flutter_image_compress`).
4. **Signature as proof** — Not legally binding, but useful for internal records. Store as PNG in storage service.
5. **Offline** — Technicians often work in areas with poor connectivity. Phase 9 basic offline is a stretch goal; real offline sync is a future feature.

---

## Decisions (confirmed 2026-06-11)

1. ✅ **Technicians CAN create tickets** for customers (not just reply)
2. ✅ **Show customer contact + maps** — phone/WhatsApp + Google Maps link for subscription location
3. ✅ **Equipment selection: BOTH** — barcode/QR scan + dropdown fallback
4. ✅ **Performance stats visible** — technician sees own completion rate, avg time, counts

---

## Verification Steps

After each phase:
1. `cd apps/mobile-technician && flutter analyze` — no errors
2. `flutter build apk --debug` — builds successfully
3. Manual test on emulator/device — screens render, API calls work
4. Dark mode toggle — all screens readable

Final:
1. `melos run analyze` — all packages pass
2. `melos run build:technician:android` — release APK builds
3. End-to-end flow: login → see dashboard → open WO → claim → start → add photos → complete with signature
