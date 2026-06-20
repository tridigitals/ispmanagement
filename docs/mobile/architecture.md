# Mobile Apps Architecture

> **Last updated:** Juni 2026
> **Stack:** Flutter 3.22+ (Dart 3.5+) · Riverpod 2.5 · go_router 14 · Dio 5
> **Target:** Android (arm64-v8a) + iOS · Backend: Tauri 2 + Rust (port 3000)

---

## 1. Big Picture

Tiga APK, satu backend, satu monorepo:

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Single Backend (Rust + Tauri)                   │
│                     Multi-tenant · Single port :3000                │
│  /api/auth/* · /api/customers/* · /api/work-orders/* · /api/...    │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
              ┌─────────────────┼─────────────────┐
              │                 │                 │
       ┌──────▼──────┐   ┌──────▼──────┐   ┌──────▼──────┐
       │  customer   │   │  technician │   │   admin     │
       │     APK     │   │     APK     │   │     APK     │
       │  (~32 MB)   │   │  (~35 MB)   │   │  (later)    │
       └──────┬──────┘   └──────┬──────┘   └──────┬──────┘
              │                 │                 │
              └────────┬────────┴────────┬────────┘
                       │                 │
                ┌──────▼──────┐   ┌──────▼──────┐
                │ api-client  │   │   ui-kit    │
                │ (shared)    │   │  (shared)   │
                └─────────────┘   └─────────────┘
                       │
                ┌──────▼──────┐
                │   config    │
                │  (shared)   │
                └─────────────┘
```

**Prinsip utama:**
- **Satu backend, banyak klien** — backend tidak peduli siapa yang manggil, role-based access di-handle di sana
- **Role gate di masing-masing APK** — `mobile-customer` reject staff, `mobile-technician` reject customer
- **Shared packages** (`api-client`, `ui-kit`, `config`) untuk konsistensi dan DRY
- **Per-APK tema** (logo, splash, accent color, app name) untuk branding berbeda

---

## 2. Repository Structure

```
ISPMANAGEMENT/
├── apps/
│   ├── mobile-customer/        # Customer APK (production)
│   │   ├── lib/src/
│   │   │   ├── features/       # 14 fitur spesifik customer
│   │   │   ├── router/
│   │   │   └── services/       # AuthController, dll
│   │   ├── android/ios/        # Branding: customer
│   │   └── pubspec.yaml        # name: mobile_customer
│   │
│   ├── mobile-technician/      # Technician APK (akan dibuat)
│   │   ├── lib/src/
│   │   │   ├── features/
│   │   │   │   ├── work_orders/    # Jadwal & detail
│   │   │   │   ├── customers/      # Quick view customer info
│   │   │   │   ├── tickets/        # Bisa respon tiket
│   │   │   │   ├── network/       # OLT status, ping, dll
│   │   │   │   └── profile/        # Profile + absen
│   │   │   ├── router/
│   │   │   └── services/
│   │   ├── android/ios/        # Branding: technician
│   │   └── pubspec.yaml        # name: mobile_technician
│   │
│   └── mobile-admin/           # (existing, untuk staff admin via web)
│       └── (mobile admin tidak jadi APK — pakai web Tauri)
│
├── packages/
│   ├── api-client/             # SHARED · Network + models + realtime
│   │   ├── lib/src/
│   │   │   ├── api/            # Dio + interceptors
│   │   │   ├── auth/           # AuthTokenStorage
│   │   │   ├── services/       # AuthService, dll
│   │   │   ├── models/         # Data classes
│   │   │   └── realtime/       # WebSocket client
│   │   └── pubspec.yaml
│   │
│   ├── ui-kit/                 # SHARED · Widget library
│   │   ├── lib/src/
│   │   │   ├── theme/          # IspThemeColors, IspSpacing
│   │   │   └── widgets/        # IspCard, IspButton, dll
│   │   └── pubspec.yaml
│   │
│   └── config/                 # SHARED · Build-time config
│       └── pubspec.yaml
│
├── docs/mobile/
│   ├── architecture.md         # ← file ini
│   └── plans/
│       └── technician-apk.md   # Plan teknis untuk mobile-technician
│
└── scripts/
    └── build-apk.sh           # Build script per APK
```

---

## 3. Shared Packages — Apa yang bisa di-reuse

### `packages/api-client/` — 100% reusable

Semua backend integration. Role tidak relevan di sini, hanya endpoint URL dan models.

| Komponen | Fungsi | Reuse |
|----------|--------|-------|
| `ApiConfig`, `buildDio` | HTTP client config | ✅ |
| `AuthInterceptor`, `setGlobalAuthToken` | Auth header | ✅ |
| `AuthTokenStorage` | Secure storage + cache | ✅ |
| `AuthService`, `*Service` classes | Backend calls | ✅ |
| `*Model` classes | Data models | ✅ |
| `RealtimeClient` | WebSocket | ✅ |
| `Result`, `Success`, `Failure` | Result wrapper | ✅ |

### `packages/ui-kit/` — 100% reusable

Widget library theme-aware. Semua pakai `IspThemeColors` jadi bisa di-skin ulang per APK.

| Widget | Reuse |
|--------|-------|
| `IspButton`, `IspCard`, `IspTextField` | ✅ |
| `IspEmptyState`, `IspErrorState`, `IspShimmer` | ✅ |
| `IspStatusBadge`, `IspAvatar`, `IspListItem` | ✅ |
| `IspProgressBar`, `IspSectionHeader`, `IspStatCard` | ✅ |
| `IspToastOverlay` | ✅ |
| `IspThemeColors`, `IspRadii`, `IspSpacing` | ✅ (override accent per APK) |

### Yang TIDAK shared (per-AK)

- **Screens** — beda fitur
- **Bottom navigation** — beda tab
- **Branding** — beda logo/splash
- **Role gate** — beda role yang diizinkan
- **Routing** — beda routes

---

## 4. Code Reuse Strategy

### Opsi A: Copy folder (chosen untuk MVP)
```
apps/mobile-customer/        ← copy → apps/mobile-technician/
```

**Pros:**
- Cepat, APK independen
- Tidak ada coupling antar APK
- Different app ID, different branding
- Bisa beda AndroidManifest, permission, signing

**Cons:**
- Duplikasi kode auth/settings/profile (~2-3K LOC)
- Bug fix harus sync ke kedua APK

**Mitigasi:** Extract shared auth/settings/profile ke `packages/shared/` setelah MVP stabil.

### Opsi B: Shared screens package (future)
```
packages/shared/
├── auth/         # LoginScreen, AuthController
├── settings/     # SettingsScreen, ChangePasswordScreen
├── profile/      # ProfileScreen, EditProfileScreen
└── notifications/ # NotificationInboxScreen + FCM
```

**Status:** Belum dibuat. Ditambahkan setelah mobile-technician stabil.

---

## 5. Backend Compatibility

Backend Tauri 2 sudah multi-tenant + RBAC. Yang sudah ada:

| Endpoint | Customer | Technician | Admin |
|----------|----------|------------|-------|
| `/api/auth/login` | ✅ | ✅ | ✅ |
| `/api/work-orders/*` | read own | CRUD own | CRUD all |
| `/api/customers/*` | read self | read assigned | CRUD |
| `/api/tickets/*` | CRUD own | CRUD assigned | CRUD all |
| `/api/network/olts/*` | ❌ | read+reboot | CRUD |
| `/api/notifications/*` | ✅ | ✅ | ✅ |
| `/api/storage/*` | upload/download | upload/download | ✅ |

**Field `role` di User:**
- `customer` → mobile-customer
- `technician` → mobile-technician
- `admin`, `super_admin`, `staff` → web admin only (semua APK reject)

---

## 6. Authentication & Token Flow

Sama untuk semua APK — pakai `api-client`. Sudah battle-tested dengan fix race condition.

```
Login → POST /api/auth/login
       ↓
       Backend return { token, refreshToken, user: { role, ... } }
       ↓
       AuthController.apply()
       ├─ Role gate: kalau role tidak sesuai → reject
       ├─ setGlobalAuthToken(token)  ← sync, sebelum state
       ├─ dio.options.headers['Authorization'] = 'Bearer $token'
       └─ state = AuthState(user)  ← trigger GoRouter redirect
       ↓
       AuthInterceptor.onRequest()
       ├─ Step 1: _globalLatestToken (fastest, no storage read)
       ├─ Step 2: existing header
       └─ Step 3: storage read with 5s timeout
```

Detail: `docs/mobile/plans/technician-apk.md` section "Auth Reuse".

---

## 7. Build & Deployment

### Build script per APK
```bash
# Customer
bash apps/mobile-customer/scripts/build-apk.sh
# → /tmp/app-customer-release.apk (32 MB arm64)

# Technician (akan dibuat)
bash apps/mobile-technician/scripts/build-apk.sh
# → /tmp/app-technician-release.apk (35 MB arm64)
```

### Build flags
- `--dart-define=API_BASE_URL=http://103.190.112.214:3000`
- `--target-platform android-arm64`
- `--release --no-pub`

### Signing
- Customer: SHA-1 `33:B9:F9:3F:...` alias `ispcustomer`
- Technician: SHA-1 berbeda (generate baru, alias `isptechnician`)

### Distribution
- Customer: Telegram channel `Home` (existing)
- Technician: Telegram channel baru atau direct ke tim lapangan
- Both served via Python http.server port 9999 di VPS

---

## 8. Roadmap

| Phase | Scope | Status |
|-------|-------|--------|
| 1 | Customer APK stabilization | ✅ Done (v0.1.0+58) |
| 2 | **Technician APK MVP** | 🟡 Next |
| 3 | Extract shared screens (`packages/shared/`) | Future |
| 4 | iOS build + TestFlight | Future |
| 5 | Admin mobile (kalau perlu) | Future |

---

## 9. Decision Log

| Date | Decision | Reason |
|------|----------|--------|
| 2026-06 | Copy folder strategy (Opsi A) | Cepat, independent APK, no coupling |
| 2026-06 | Backend tetap single port :3000 | Sudah jalan, jangan duplikasi |
| 2026-06 | Role `admin/super_admin/staff` tidak boleh login di APK manapun | Web admin Tauri sudah handle |
| 2026-06 | Technician work order: pakai endpoint `/api/work-orders/*` yang sudah ada | Backend sudah support |
| 2026-06 | Mobile-admin dihapus dari rencana | Staff pakai web Tauri lebih lengkap |