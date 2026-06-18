#!/usr/bin/env bash
#
# build-admin-apk.sh — build release APK for mobile-admin (admin app) with proper monorepo path resolution.
#
# Required env (export before running):
#   PATH must include $HOME/sdk/flutter/bin
#   ANDROID_HOME=$HOME/sdk/android-sdk
#   JAVA_HOME=$HOME/sdk/java17
#
# Usage: bash build-admin-apk.sh
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
APP_DIR="$PROJECT_ROOT/apps/mobile-admin"
MONOREPO_DART_TOOL="$PROJECT_ROOT/.dart_tool"
APP_DART_TOOL="$APP_DIR/.dart_tool"

[[ -d "$APP_DIR" ]] || { echo "❌ App dir not found: $APP_DIR"; exit 1; }
[[ -d "$MONOREPO_DART_TOOL" ]] || { echo "❌ Run 'flutter pub get' at repo root first"; exit 1; }

export PATH="$PATH:$HOME/sdk/flutter/bin"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/sdk/android-sdk}"
export JAVA_HOME="${JAVA_HOME:-$HOME/sdk/java17}"
command -v flutter >/dev/null || { echo "❌ flutter not in PATH"; exit 1; }

cd "$APP_DIR"

if [[ "${SKIP_FIX:-0}" != "1" ]]; then
  echo "🔧 Patching package_config.json paths..."
  mkdir -p "$APP_DART_TOOL"
  cp "$MONOREPO_DART_TOOL/package_config.json" "$APP_DART_TOOL/package_config.json"
  sed -i \
    -e 's|\"../packages/api-client\"|\"file://'"$PROJECT_ROOT"'/packages/api-client\"|g' \
    -e 's|\"../packages/ui-kit\"|\"file://'"$PROJECT_ROOT"'/packages/ui-kit\"|g' \
    "$APP_DART_TOOL/package_config.json"
  echo "   ui-kit + api-client paths → absolute file://"

  if [[ -f "$PROJECT_ROOT/.flutter-plugins-dependencies" ]]; then
    cp "$PROJECT_ROOT/.flutter-plugins-dependencies" "$APP_DIR/.flutter-plugins-dependencies"
    echo "   .flutter-plugins-dependencies copied from monorepo root"
  fi
  if [[ -f "$PROJECT_ROOT/.flutter-plugins" ]]; then
    cp "$PROJECT_ROOT/.flutter-plugins" "$APP_DIR/.flutter-plugins"
  fi
fi

if [[ -z "${BUILD_NUMBER:-}" ]]; then
  PUBSPEC_VERSION=$(grep '^version:' "$APP_DIR/pubspec.yaml" | head -1 | sed 's/version: //')
  CUR_NAME=$(echo "$PUBSPEC_VERSION" | cut -d+ -f1)
  CUR_CODE=$(echo "$PUBSPEC_VERSION" | cut -d+ -f2)
  BUILD_NAME="${BUILD_NAME:-$CUR_NAME}"
  BUILD_NUMBER="${BUILD_NUMBER:-$((CUR_CODE + 1))}"
fi

echo "📦 Building mobile-admin v${BUILD_NAME}+${BUILD_NUMBER}"

API_BASE_URL="${API_BASE_URL:-http://103.190.112.214:3000}"
WS_BASE_URL="${WS_BASE_URL:-ws://103.190.112.214:3000}"
echo "   API_BASE_URL=$API_BASE_URL"

flutter build apk --release --no-pub \
  --target-platform android-arm64 \
  --build-number="$BUILD_NUMBER" \
  --build-name="$BUILD_NAME" \
  --dart-define=API_BASE_URL="$API_BASE_URL" \
  --dart-define=WS_BASE_URL="$WS_BASE_URL"

APK_SRC="$APP_DIR/build/app/outputs/flutter-apk/app-release.apk"
APK_DST="/tmp/app-admin-release.apk"

if [[ -f "$APK_SRC" ]]; then
  cp "$APK_SRC" "$APK_DST"
  APK_SIZE=$(du -h "$APK_DST" | cut -f1)
  echo "✅ Admin APK ready: $APK_DST ($APK_SIZE)"
  echo "   Download: http://103.190.112.214:9999/app-admin-release.apk"
  echo "   Architecture: arm64-v8a only"
else
  echo "❌ Build failed — APK not found at $APK_SRC"
  exit 1
fi
