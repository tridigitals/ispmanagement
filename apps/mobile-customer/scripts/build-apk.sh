#!/usr/bin/env bash
#
# build-apk.sh — build release APK for mobile-customer with proper monorepo path resolution.
#
# Required env (export before running):
#   PATH must include $HOME/sdk/flutter/bin
#   ANDROID_HOME=$HOME/sdk/android-sdk
#   JAVA_HOME=$HOME/sdk/java17
#
# Optional env:
#   BUILD_NUMBER (default: timestamp % 100000)
#   BUILD_NAME (default: read from pubspec.yaml)
#   SKIP_FIX=1 to skip package_config.json path fix
#
# Usage: bash build-apk.sh
#
set -euo pipefail

# Resolve script dir + project root (apps/mobile-customer/scripts/build-apk.sh)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
APP_DIR="$PROJECT_ROOT/apps/mobile-customer"
MONOREPO_DART_TOOL="$PROJECT_ROOT/.dart_tool"
APP_DART_TOOL="$APP_DIR/.dart_tool"

# Sanity check
[[ -d "$APP_DIR" ]] || { echo "❌ App dir not found: $APP_DIR"; exit 1; }
[[ -d "$MONOREPO_DART_TOOL" ]] || { echo "❌ Run 'flutter pub get' at repo root first"; exit 1; }

# Tooling
export PATH="$PATH:$HOME/sdk/flutter/bin"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/sdk/android-sdk}"
export JAVA_HOME="${JAVA_HOME:-$HOME/sdk/java17}"
command -v flutter >/dev/null || { echo "❌ flutter not in PATH"; exit 1; }

cd "$APP_DIR"

# Step 1: Fix package_config.json — monorepo uses relative paths that break when
# Flutter builds from apps/mobile-customer/. Copy + rewrite to absolute file:// URIs.
if [[ "${SKIP_FIX:-0}" != "1" ]]; then
  echo "🔧 Patching package_config.json paths..."
  mkdir -p "$APP_DART_TOOL"
  cp "$MONOREPO_DART_TOOL/package_config.json" "$APP_DART_TOOL/package_config.json"
  sed -i \
    -e 's|"../packages/api-client"|"file://'"$PROJECT_ROOT"'/packages/api-client"|g' \
    -e 's|"../packages/ui-kit"|"file://'"$PROJECT_ROOT"'/packages/ui-kit"|g' \
    "$APP_DART_TOOL/package_config.json"
  echo "   ui-kit + api-client paths → absolute file://"
fi

# Step 2: Determine build number + name
if [[ -z "${BUILD_NUMBER:-}" ]]; then
  # Read pubspec.yaml version (e.g., "0.1.0+11") and bump +N
  PUBSPEC_VERSION=$(grep '^version:' "$APP_DIR/pubspec.yaml" | head -1 | sed 's/version: //')
  CUR_NAME=$(echo "$PUBSPEC_VERSION" | cut -d+ -f1)
  CUR_CODE=$(echo "$PUBSPEC_VERSION" | cut -d+ -f2)
  BUILD_NAME="${BUILD_NAME:-$CUR_NAME}"
  BUILD_NUMBER="${BUILD_NUMBER:-$((CUR_CODE + 1))}"
fi

echo "📦 Building mobile-customer v${BUILD_NAME}+${BUILD_NUMBER}"

# Step 3: Build APK
flutter build apk --release --no-pub \
  --build-number="$BUILD_NUMBER" \
  --build-name="$BUILD_NAME"

APK_SRC="$APP_DIR/build/app/outputs/flutter-apk/app-release.apk"
APK_DST="/home/xtrabit/app-release.apk"

if [[ -f "$APK_SRC" ]]; then
  cp "$APK_SRC" "$APK_DST"
  echo "✅ APK ready: $APK_DST ($(du -h "$APK_DST" | cut -f1))"
  echo "   Download: http://103.190.112.214:9999/app-release.apk"
else
  echo "❌ Build failed — APK not found at $APK_SRC"
  exit 1
fi
