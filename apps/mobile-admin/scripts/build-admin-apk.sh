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
ANDROID_DART_TOOL="$APP_DIR/android/.dart_tool"

[[ -d "$APP_DIR" ]] || { echo "❌ App dir not found: $APP_DIR"; exit 1; }
[[ -d "$MONOREPO_DART_TOOL" ]] || { echo "❌ Run 'flutter pub get' at repo root first"; exit 1; }

export PATH="$PATH:$HOME/sdk/flutter/bin"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/sdk/android-sdk}"
export JAVA_HOME="${JAVA_HOME:-$HOME/sdk/java17}"
command -v flutter >/dev/null || { echo "❌ flutter not in PATH"; exit 1; }

cd "$APP_DIR"

# Rewrite the monorepo-root package_config.json in-place so its `../packages/*`
# and `../apps/*` URIs resolve to absolute file:// URIs. Gradle's flutter task
# spawns the flutter CLI from $PROJECT_ROOT (workspace root), and relative URIs
# would resolve outside the repo.
rewrite_root_pkg_config() {
  local cfg="$MONOREPO_DART_TOOL/package_config.json"
  [[ -f "$cfg" ]] || return 0
  python3 - "$cfg" "$PROJECT_ROOT" <<'PYEOF'
import json, sys
from pathlib import Path
cfg_path = Path(sys.argv[1])
project_root = Path(sys.argv[2]).resolve()
project_root_uri = project_root.as_uri() + '/'
data = json.loads(cfg_path.read_text())
rewrite = {
    '../packages/api-client': project_root_uri + 'packages/api-client',
    '../packages/ui-kit': project_root_uri + 'packages/ui-kit',
    '../apps/mobile-admin': project_root_uri + 'apps/mobile-admin',
    '../apps/mobile-customer': project_root_uri + 'apps/mobile-customer',
}
changed = False
for pkg in data.get('packages', []):
    if pkg.get('rootUri') in rewrite:
        new = rewrite[pkg['rootUri']]
        if pkg['rootUri'] != new:
            pkg['rootUri'] = new
            changed = True
if changed:
    cfg_path.write_text(json.dumps(data, indent=2))
    print('   Rewrote root package_config.json relative → absolute URIs')
PYEOF
}

# ALSO copy the (now-rewritten) root file into apps/mobile-admin/.dart_tool/.
# The Gradle Flutter plugin invokes `flutter assemble` from the app dir, and
# `findProjectRoot` finds apps/mobile-admin/pubspec.yaml FIRST (it's a valid
# project root), so Flutter reads apps/mobile-admin/.dart_tool/package_config.json.
# If we don't put a file there, the build fails with "package_config.json does
# not exist" — workspace mode never writes per-app config files.
copy_pkg_config_to_app() {
  local src="$MONOREPO_DART_TOOL/package_config.json"
  local dst_dir="$APP_DART_TOOL"
  local dst="$dst_dir/package_config.json"
  [[ -f "$src" ]] || return 0
  mkdir -p "$dst_dir"
  cp "$src" "$dst"
  # Also copy the supplementary files Flutter reads (package_config_subset,
  # version, flutter-plugins) so dart_plugin_registrant target can find them.
  for f in package_config_subset version; do
    [[ -f "$MONOREPO_DART_TOOL/$f" ]] && cp "$MONOREPO_DART_TOOL/$f" "$dst_dir/$f"
  done
  if [[ -f "$PROJECT_ROOT/.flutter-plugins-dependencies" ]]; then
    cp "$PROJECT_ROOT/.flutter-plugins-dependencies" "$APP_DIR/.flutter-plugins-dependencies"
  fi
  if [[ -f "$PROJECT_ROOT/.flutter-plugins" ]]; then
    cp "$PROJECT_ROOT/.flutter-plugins" "$APP_DIR/.flutter-plugins"
  fi
  echo "   Copied package_config.json (+subset+version) → apps/mobile-admin/.dart_tool/"
}

if [[ "${SKIP_FIX:-0}" != "1" ]]; then
  rewrite_root_pkg_config
  copy_pkg_config_to_app
fi

if [[ -z "${BUILD_NUMBER:-}" ]]; then
  PUBSPEC_VERSION=$(grep '^version:' "$APP_DIR/pubspec.yaml" | head -1 | sed 's/version: //')
  CUR_NAME=$(echo "$PUBSPEC_VERSION" | cut -d+ -f1)
  CUR_CODE=$(echo "$PUBSPEC_VERSION" | cut -d+ -f2)
  BUILD_NAME="${BUILD_NAME:-$CUR_NAME}"
  BUILD_NUMBER="${BUILD_NUMBER:-$((CUR_CODE + 1))}"
fi

echo "📦 Building mobile-admin v${BUILD_NAME}+${BUILD_NUMBER}"
echo "   API_BASE_URL=${API_BASE_URL:-http://103.190.112.214:3000}"

# IMPORTANT: pass --no-pub. We rewrite the workspace root's package_config.json
# in-place above (relative → absolute file:// URIs), and we do NOT want
# `flutter pub get` to overwrite it from the app dir — in workspace mode that
# re-emits the file with `../packages/api-client` relative URIs that resolve
# outside the repo.
flutter build apk --release --no-pub \
  --target-platform android-arm64 \
  --build-number="$BUILD_NUMBER" \
  --build-name="$BUILD_NAME" \
  --dart-define=API_BASE_URL="${API_BASE_URL:-http://103.190.112.214:3000}" \
  --dart-define=WS_BASE_URL="${WS_BASE_URL:-ws://103.190.112.214:3000}"

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
