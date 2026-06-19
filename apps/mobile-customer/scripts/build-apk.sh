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
  # Fix ALL relative paths to absolute file:// URIs (not just api-client + ui-kit)
  python3 -c "
import json
from pathlib import Path
config_path = Path('$APP_DART_TOOL/package_config.json')
root_dot = Path('$MONOREPO_DART_TOOL')
with open(config_path) as f:
    config = json.load(f)
fixed = 0
for pkg in config['packages']:
    uri = pkg['rootUri']
    if not uri.startswith('file://'):
        abs_path = (root_dot / uri).resolve()
        pkg['rootUri'] = abs_path.as_uri()
        fixed += 1
with open(config_path, 'w') as f:
    json.dump(config, f, indent=2)
print(f'   Fixed {fixed} relative paths → absolute file://')
"

  # Fix .flutter-plugins-dependencies — root's resolution:workspace mode means
  # sub-app doesn't generate its own plugin list, so Gradle misses Firebase + other
  # native plugins. Copy from root to sub-app.
  if [[ -f "$PROJECT_ROOT/.flutter-plugins-dependencies" ]]; then
    cp "$PROJECT_ROOT/.flutter-plugins-dependencies" "$APP_DIR/.flutter-plugins-dependencies"
    echo "   .flutter-plugins-dependencies copied from monorepo root"
  fi
  if [[ -f "$PROJECT_ROOT/.flutter-plugins" ]]; then
    cp "$PROJECT_ROOT/.flutter-plugins" "$APP_DIR/.flutter-plugins"
  fi

  # Patch `gal` package's android/build.gradle — it references
  # `flutter.compileSdkVersion` inside the `android {}` block, but with the
  # monorepo's plugin-loader pattern that extension isn't injected for
  # subprojects, so the build fails with "Could not get unknown property
  # 'flutter'". Hardcode to 35 (matches the SDK we set in this app's
  # subprojects override). This patch is idempotent — re-running on an
  # already-patched file is a no-op.
  GAL_BG="$HOME/.pub-cache/hosted/pub.dev/gal-2.3.2/android/build.gradle"
  if [[ -f "$GAL_BG" ]] && grep -q "compileSdk flutter\.compileSdkVersion" "$GAL_BG"; then
    sed -i 's/compileSdk flutter\.compileSdkVersion/compileSdk 35/' "$GAL_BG"
    echo "   gal/android/build.gradle patched (compileSdk → 35)"
  fi
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
# API_BASE_URL is baked in at compile time so the app knows where to connect.
API_BASE_URL="${API_BASE_URL:-http://103.190.112.214:3000}"
WS_BASE_URL="${WS_BASE_URL:-ws://103.190.112.214:3000}"
echo "   API_BASE_URL=$API_BASE_URL"

flutter build apk --release --no-pub \
  --target-platform android-arm64 \
  --build-number="$BUILD_NUMBER" \
  --build-name="$BUILD_NAME" \
  --dart-define=API_BASE_URL="$API_BASE_URL" \
  --dart-define=WS_BASE_URL="$WS_BASE_URL"

# arm64-only build produces a single APK at app-release.apk (smaller than universal)
APK_SRC="$APP_DIR/build/app/outputs/flutter-apk/app-release.apk"
APK_DST="/tmp/app-release.apk"

if [[ -f "$APK_SRC" ]]; then
  cp "$APK_SRC" "$APK_DST"
  APK_SIZE=$(du -h "$APK_DST" | cut -f1)
  echo "✅ APK ready: $APK_DST ($APK_SIZE)"
  echo "   Download: http://103.190.112.214:9999/app-release.apk"
  echo "   Architecture: arm64-v8a only (smaller than universal)"
else
  echo "❌ Build failed — APK not found at $APK_SRC"
  exit 1
fi
