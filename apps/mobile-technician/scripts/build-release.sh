#!/bin/bash
# Build release APK for ISP Customer mobile app.
# Usage:
#   ./scripts/build-release.sh                 # Sentry disabled (dev)
#   ./scripts/build-release.sh --with-sentry   # Sentry enabled (production)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_DIR"

# Sentry DSN — embedded in the binary
# Public key, safe to commit (Sentry DSNs are designed for client-side use)
SENTRY_DSN="https://75e881638b52b75b31b35fa92e82d834@o4511507498401792.ingest.us.sentry.io/4511507503513600"
SENTRY_ENV="production"
SENTRY_RELEASE="mobile-technician@0.2.0+1"

DART_DEFINES=()
if [[ "${1:-}" == "--with-sentry" ]]; then
  echo "→ Building with Sentry enabled"
  DART_DEFINES+=(
    "--dart-define=SENTRY_DSN=$SENTRY_DSN"
    "--dart-define=SENTRY_ENV=$SENTRY_ENV"
    "--dart-define=SENTRY_RELEASE=$SENTRY_RELEASE"
  )
else
  echo "→ Building without Sentry (dev build)"
fi

flutter build apk --release \
  --target-platform android-arm64 \
  "${DART_DEFINES[@]}"

APK="build/app/outputs/flutter-apk/app-arm64-v8a-release.apk"
if [[ -f "$APK" ]]; then
  cp "$APK" /home/xtrabit/isp-management-customer-arm64.apk
  echo
  echo "✓ Built and copied to /home/xtrabit/isp-management-customer-arm64.apk"
  ls -la /home/xtrabit/isp-management-customer-arm64.apk
fi
