# Plan: Mobile Technician APK

> **Status:** Planning
> **Target:** v1.0.0 MVP
> **Estimasi:** 6 sprint (~3-4 minggu)

---

## 1. Goals & Non-Goals

### Goals
- APK untuk teknisi lapangan yang install internet di rumah pelanggan
- Login dengan email/password + 2FA (sama seperti customer)
- Lihat work order yang di-assign atau di-claim
- Update status work order: `assigned → in_progress → completed`
- Foto dokumentasi (sebelum/sesudah)
- GPS check-in/out
- Quick view customer info (alamat, paket)
- Bisa buka & respon tiket yang di-assign

### Non-Goals (v1)
- Routing/optimisasi urutan kunjungan
- Chat real-time dengan admin
- Billing/payment (tidak perlu)
- Manajemen tim/asset
- Laporan performance teknisi
- Offline mode (coming v2)

---

## 2. Sprint Plan

### Sprint 1 — Skeleton + Auth (3 hari)
| Task | Output |
|------|--------|
| Copy `mobile-customer/` → `mobile-technician/` | Folder baru |
| Rename package, app name, signing | `id: com.tridigitals.technician`, alias `isptechnician` |
| Update `pubspec.yaml` name + version 0.1.0+1 | File updated |
| Bikin `apply()` role gate: only `technician` role | `auth_providers.dart` updated |
| Branding: logo, splash, accent color (#1565C0 biru engineer) | Android resources |
| Generate release keystore `isptechnician` | `~/.android/keystore.jks` |
| First build APK v0.1.0+1 | Build success |

### Sprint 2 — Bottom Nav + Home (3 hari)
| Task | Output |
|------|--------|
| Bottom nav 4 tab: **Jadwal**, **Pekerjaan Saya**, **Notifikasi**, **Akun** | `home_shell.dart` |
| Hapus semua fitur customer-only dari nav | Removed |
| Hapus screens: home/invoices/subscriptions/payments | Deleted |
| Home tab = work order list (filter: hari ini, minggu ini, semua) | `work_order_list_screen.dart` |
| Empty state "Belum ada pekerjaan" | Widget |
| Logout button di tab Akun | Button |

### Sprint 3 — Work Order Detail + Action (4 hari)
| Task | Output |
|------|--------|
| Screen detail work order | `work_order_detail_screen.dart` |
| Tampilkan: customer info, alamat (map), paket, catatan | UI |
| Action button: Claim (kalau belum assigned ke saya) | Button + API call |
| Action button: Start (set status in_progress, GPS check-in) | Button + API call |
| Action button: Complete (upload foto, signature, GPS check-out) | Button + API |
| Reschedule request (form tanggal + alasan) | Bottom sheet |
| Realtime: status berubah dari admin → push notification | FCM hook |

### Sprint 4 — Camera + Signature + Map (4 hari)
| Task | Output |
|------|--------|
| Camera integration (image_picker) | Plugin added |
| Upload foto dokumentasi ke `/api/storage/upload` | API call |
| Signature pad (signature package) | Widget |
| Map preview (maplibre_gl) di detail work order | Map view |
| Share location (kirim koordinat ke backend) | API call |
| Compress foto sebelum upload (max 1MB) | Utility |

### Sprint 5 — Network Quick View (3 hari)
| Task | Output |
|------|--------|
| Tab khusus network: OLT status, ONU list | `network_tab.dart` |
| Ping test ke OLT IP | API call |
| Reboot ONU (kalau ada permission) | API call |
| Tampilkan signal strength terakhir | Read-only |

### Sprint 6 — Polish + Release (3 hari)
| Task | Output |
|------|--------|
| Loading states + error states di semua screen | ✅ |
| Pagination di work order list | ✅ |
| Search/filter di work order list | ✅ |
| App icon + splash final | Assets |
| Localization (id, en) | `.arb` files |
| APK v1.0.0 release | Tagged build |

---

## 3. Tech Stack

| Layer | Package | Source |
|-------|---------|--------|
| State management | `flutter_riverpod ^2.5.1` | `packages/api-client` |
| Navigation | `go_router ^14.0.0` | `packages/api-client` |
| HTTP | Dio (via api-client) | `packages/api-client` |
| Storage | `flutter_secure_storage` | `packages/api-client` |
| Realtime | WebSocket (via api-client) | `packages/api-client` |
| Theme | ui-kit | `packages/ui-kit` |
| Camera | `image_picker ^1.1.2` | NEW |
| Signature | `signature ^5.4.0` | NEW |
| Map | `maplibre_gl ^0.20.0` | NEW (jika perlu) |
| GPS | `geolocator ^11.0.0` | NEW |
| Path | `path_provider ^2.1.5` | NEW |
| HTTP multipart | `dio` (sudah ada) | `packages/api-client` |

---

## 4. Backend Endpoints Needed

Existing endpoints di Rust Tauri (port 3000):

| Endpoint | Method | Used For |
|----------|--------|----------|
| `/api/auth/login` | POST | Login (sama seperti customer) |
| `/api/auth/me` | GET | Verify session |
| `/api/work-orders?assigned_to=me&status=*` | GET | List my work orders |
| `/api/work-orders/{id}` | GET | Detail work order |
| `/api/work-orders/{id}/claim` | POST | Claim available WO |
| `/api/work-orders/{id}/start` | POST | Start (set in_progress) |
| `/api/work-orders/{id}/complete` | POST | Complete (with photos, notes) |
| `/api/work-orders/{id}/reschedule` | POST | Request reschedule |
| `/api/customers/{id}` | GET | Quick view customer |
| `/api/storage/upload` | POST | Upload foto |
| `/api/network/olts` | GET | OLT list |
| `/api/network/olts/{id}/onus` | GET | ONU list per OLT |
| `/api/network/olts/{id}/onus/{onu_id}/reboot` | POST | Reboot ONU |

**Semua sudah ada di backend**, tinggal pakai.

---

## 5. Role Gate — Apa yang berubah

Di `mobile-customer`, `AuthController.apply()` reject non-customer:
```dart
if (!auth.user.isCustomer) {
    return Failure(ApiException(
        message: 'Akun ini bukan akun pelanggan...',
    ));
}
```

Untuk technician, reverse logic:
```dart
if (!auth.user.isTechnician) {
    return Failure(ApiException(
        message: 'Akun ini bukan akun teknisi...',
    ));
}
```

UserModel perlu expose `isTechnician` getter (cek `role == 'technician'`).

---

## 6. UI Theme Override

`mobile-customer` pakai accent oranye (warna ISP Tridigitals).

`mobile-technician` override ke biru engineer:

```dart
// Di main.dart technician
IspThemeColors.of(context).copyWith(
    accent: Color(0xFF1565C0),  // biru engineer
    accentDark: Color(0xFF0D47A1),
);
```

Atau via theme override di `ui-kit` jika support.

---

## 7. Bottom Nav — Definitive

```
┌────────┬────────┬────────┬────────┐
│ Jadwal │ Kerja  │ Notif  │  Akun  │
│ 📅     │ 🛠️     │ 🔔     │  👤    │
└────────┴────────┴────────┴────────┘
```

4 tab (sesuai preferensi user: no profile terisah).

---

## 8. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Backend endpoint `/api/work-orders/*` schema berubah | Medium | Pin Rust API version, add integration tests |
| Camera/gps permission di Android 13+ | Low | Standard Android permissions, tested di Android 12/13 |
| APK size membengkak (camera + map + signature) | Medium | Tree-shake, exclude unused locales |
| Code duplikasi dengan customer | Low | Acceptable untuk MVP, refactor ke shared package di v2 |

---

## 9. Build & Release

```bash
# Build script akan meng-copy pola dari mobile-customer
cp apps/mobile-customer/scripts/build-apk.sh apps/mobile-technician/scripts/

# Modify script:
# - PKG_NAME=mobile_technician
# - KEYSTORE_ALIAS=isptechnician
# - APK_DST=/tmp/app-technician-release.apk

bash apps/mobile-technician/scripts/build-apk.sh
```

Output:
- `/tmp/app-technician-release.apk` (estimasi 35 MB arm64)
- Serve via Python http.server port 9999
- Kirim ke Telegram channel khusus tim lapangan

---

## 10. Definition of Done (MVP)

- [ ] Login sebagai teknisi → bisa, role teknisi lain/customer/admin reject
- [ ] Bisa lihat work order yang di-assign
- [ ] Bisa claim work order yang available
- [ ] Bisa start (dengan GPS)
- [ ] Bisa complete (dengan foto + signature + notes)
- [ ] Bisa request reschedule
- [ ] Notifikasi realtime saat admin update status
- [ ] APK release v1.0.0 + served di port 9999
- [ ] Tested di Android 12 + 13 (perangkat teknisi)