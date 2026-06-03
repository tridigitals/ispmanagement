# Mobile App Development

Customer, Admin, and Superadmin mobile apps for the ISP Management platform. All built with **Flutter** (one codebase → iOS + Android).

## Quick start

```bash
# 1. Install melos (monorepo tool)
dart pub global activate melos

# 2. Bootstrap all packages
cd /home/xtrabit/ISPMANAGEMENT
melos bootstrap

# 3. Generate code (api-client models, l10n)
melos run codegen

# 4. Run the customer app on a connected device / emulator
cd apps/mobile-customer
flutter run
```

## Project layout

```
apps/
├── mobile-customer/      # Pelanggan ISP app
├── mobile-admin/         # Admin ISP app
└── mobile-superadmin/    # Superadmin platform app

packages/
├── api-client/           # Dio + Auth + WebSocket (shared)
├── ui-kit/               # Dark theme + reusable widgets
└── config/               # Build-time env (API URLs, feature flags)
```

## Why Flutter, not Tauri Mobile?

| Factor | Tauri Mobile | Flutter |
|---|---|---|
| iOS build | Requires macOS + Xcode | Same |
| App Store review | WebView can be rejected | Standard native, fast |
| Plugin ecosystem | Thin (Mikrotik, biometric, FCM immature) | Rich |
| Hot reload | Slow | Sub-second |
| UX for complex admin UI | Laggy WebView | Smooth native |

The desktop admin (Tauri) remains untouched — its file system / local agent / Mikrotik integration would be expensive to rebuild in Flutter.

## Architecture

```
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ mobile-customer │  │  mobile-admin   │  │ mobile-super    │
│  (Flutter)      │  │  (Flutter)      │  │  (Flutter)      │
└────────┬────────┘  └────────┬────────┘  └────────┬────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              │ packages/api-client (Dio + WS)
                              ▼
                  ┌──────────────────────────┐
                  │ Axum REST API (existing) │
                  │ src-tauri/src/http/      │
                  └──────────────────────────┘
                              │
                              ▼
                  ┌──────────────────────────┐
                  │  Postgres multi-tenant   │
                  └──────────────────────────┘
```

The Axum router already serves the web app. The mobile apps consume the same REST API.

## Auth flow

1. POST `/api/auth/login` with email + password
2. If `requires_2fa: true` → POST `/api/auth/2fa/verify` with `temp_token` + code
3. Token stored in `flutter_secure_storage` (Android EncryptedSharedPrefs, iOS Keychain)
4. Dio `AuthInterceptor` attaches `Bearer` header; on 401, auto-refresh and retry
5. Optional biometric (Face ID / fingerprint) wraps the secure storage

## API contract generation

Backend → OpenAPI → Dart client:

```bash
./tools/generate-openapi.sh     # Rust → JSON
./tools/generate-dart-client.sh # JSON → Dart
melos run codegen               # run build_runner
```

The OpenAPI export uses `utoipa` annotations on the Axum router. Start with `auth.rs` and `customer_service/portal.rs` (the most-used customer endpoints).

## CI/CD

GitHub Actions builds Android (APK) and iOS (.app) artifacts on every push to `main`/`develop`. See `.github/workflows/mobile-build.yml`.

For app store releases, extend with:
- Android: `upload-playstore` step using `r0adkll/upload-google-play@v1`
- iOS: `xcrun altool` or `fastlane`

## Testing

```bash
# Unit tests for packages
melos run test

# Integration tests for a specific app
cd apps/mobile-customer
flutter test integration_test/

# Generate coverage
flutter test --coverage
genhtml coverage/lcov.info -o coverage/html
```

## Next steps (priority order)

1. ☐ Add `utoipa` annotations to `src-tauri/src/http/auth.rs` and `customer_service/portal.rs`
2. ☐ Generate OpenAPI spec → review the JSON for any private fields to redact
3. ☐ Generate Dart client → wrap in a `Dio` instance with auth interceptor
4. ☐ Wire `mySubscriptionsProvider` / `myInvoicesProvider` / `myTicketsProvider` to the generated client
5. ☐ Build subscription/invoice/ticket detail screens (replace the current placeholders)
6. ☐ Add 2FA setup flow with QR code (use `mobile_scanner` + `otp` package)
7. ☐ Add payment flow (Midtrans/Xendit/Stripe deep link integration)
8. ☐ Add FCM for push notifications (use `firebase_messaging`)
9. ☐ Add speed-test mini-feature (HTTP-based for MVP, no raw socket needed)
10. ☐ Repeat for `mobile-admin` and `mobile-superadmin`
