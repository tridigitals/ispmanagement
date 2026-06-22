#!/bin/bash
# Build release APK for ISP Technician mobile app.
# Usage:
#   ./scripts/build-release.sh                 # Sentry disabled (dev)
#   ./scripts/build-release.sh --with-sentry   # Sentry enabled (production)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$PROJECT_DIR/../.." && pwd)"

cd "$PROJECT_DIR"

# Sentry DSN — embedded in the binary
# Public key, safe to commit (Sentry DSNs are designed for client-side use)
SENTRY_DSN="https://75e881638b52b75b31b35fa92e82d834@o4511507498401792.ingest.us.sentry.io/4511507503513600"
SENTRY_ENV="production"
SENTRY_RELEASE="mobile-technician@0.2.0+12"

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

# ── Workspace workaround ──────────────────────────────────────────
# Flutter workspace mode (Dart 3.5+) skips writing package_config.json
# into each member dir. flutter_tools then exits with "LocalDirectory:
# pub did not create .dart_tools/package_config.json file" on
# `flutter build apk`. Fix: temporarily remove this member from the
# workspace, generate a per-member package_config.json, then restore
# the workspace and build with --no-pub so flutter doesn't re-run pub
# get (which would delete the per-member config).
echo "→ Generating per-member package_config.json (workspace workaround)"
cp "$REPO_ROOT/pubspec.yaml" /tmp/pubspec.yaml.bak
sed -i 's|^  - apps/mobile-technician$|  #- apps/mobile-technician|' "$REPO_ROOT/pubspec.yaml"

cleanup() {
  cp /tmp/pubspec.yaml.bak "$REPO_ROOT/pubspec.yaml"
}
trap cleanup EXIT

(
  cd "$REPO_ROOT"
  flutter pub get > /dev/null 2>&1
)

# Inside technician dir: drop workspace resolution, run dart pub get,
# then restore so the file is committed-friendly.
sed -i 's|^resolution: workspace|#resolution: workspace|' pubspec.yaml
(
  dart pub get > /dev/null 2>&1
)
sed -i 's|^#resolution: workspace|resolution: workspace|' pubspec.yaml

# Restore workspace BEFORE flutter build apk (--no-pub skips its pub step).
cleanup
trap - EXIT

flutter build apk --release \
  --target-platform android-arm64 \
  --no-pub \
  "${DART_DEFINES[@]}"

APK="build/app/outputs/flutter-apk/app-arm64-v8a-release.apk"
if [[ -f "$APK" ]]; then
  cp "$APK" /home/xtrabit/isp-management-technician-arm64.apk
  echo
  echo "✓ Built and copied to /home/xtrabit/isp-management-technician-arm64.apk"
  ls -la /home/xtrabit/isp-management-technician-arm64.apk
fi
