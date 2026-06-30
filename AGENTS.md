# ISPMANAGEMENT Project

## Overview
ISP Management monorepo — desktop (Tauri + SvelteKit), mobile apps (Flutter 3.24), shared packages.
- **Path:** `/home/xtrabit/ISPMANAGEMENT`
- **Stack:** Rust (Tauri) + SvelteKit + Flutter 3.24 + Dart + PostgreSQL/PostGIS
- **Managed by:** Melos

## Structure
```
ISPMANAGEMENT/
├── apps/
│   ├── mobile-customer/   # Flutter — Customer-facing app (full features)
│   ├── mobile-technician/ # Flutter — Technician app (limited features)
│   └── mobile-admin/      # Flutter — Admin app (analytics, monitoring)
├── packages/
│   ├── api-client/        # Shared Dart API client (used by all Flutter apps)
│   ├── config/            # Shared configuration
│   └── ui-kit/            # Shared Flutter UI components
├── build/                 # Build outputs
├── deploy/                # Deployment configs (Docker, etc.)
├── docs/                  # Documentation
├── e2e/                   # End-to-end tests
└── pubspec.yaml           # Workspace root (Flutter workspace mode)
```

## Key Commands

### Build APKs
Use the `ispmanagement-mobile-build` skill for detailed build instructions. Quick reference:
- **SDK paths** (not on default PATH):
  - Flutter: `~/sdk/flutter/bin`
  - Android SDK: `~/sdk/android-sdk`
  - Java 17: `~/sdk/java17`
- **Build workdir:** `~/ISPMANAGEMENT` (each app: `apps/mobile-{app}`)
- **Output APKs** → copy to `~/apk-server/mobile-{type}-arm64.apk`

### Version Control
- Git repo at `/home/xtrabit/ISPMANAGEMENT/.git`
- Branch: check with `git branch`
- Common: `git status`, `git diff`, `git log --oneline -10`

## App Feature Gating
- **Customer app** = full feature set (ratings, surveys, payments, create tickets, etc.)
- **Technician app** = no ratings, surveys, marketing, engagement hooks, NO create ticket
- **Admin app** = analytics, monitoring, system controls
- When removing features: comment out imports/usage blocks (NEVER delete) so other apps keep them

## APK Server
- Serves APKs at: `http://103.190.112.214:9999/`
- Files at: `/home/xtrabit/apk-server/`

## CI/CD
- GitHub Actions workflows in `.github/workflows/`

## Signing
- Keystore: `release-key.jks` (alias: `ispcustomer`)
- Check signature: `apksigner verify --print-certs <apk>`
