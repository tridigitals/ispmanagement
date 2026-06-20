#!/usr/bin/env bash
# Build release APK for mobile-technician
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
APP_DIR="$PROJECT_ROOT/apps/mobile-technician"
MONOREPO_DART_TOOL="$PROJECT_ROOT/.dart_tool"
APP_DART_TOOL="$APP_DIR/.dart_tool"
export PATH="$PATH:$HOME/sdk/flutter/bin"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/sdk/android-sdk}"
export JAVA_HOME="${JAVA_HOME:-$HOME/sdk/java17}"
command -v flutter >/dev/null || { echo "❌ flutter not in PATH"; exit 1; }
[[ -d "$MONOREPO_DART_TOOL" ]] || { echo "❌ Run 'flutter pub get' at repo root first"; exit 1; }
echo "🔧 Patching package_config.json paths..."
mkdir -p "$APP_DART_TOOL"
cp "$MONOREPO_DART_TOOL/package_config.json" "$APP_DART_TOOL/package_config.json"
python3 -c "
import json
from pathlib import Path
config_path = Path('$APP_DART_TOOL/package_config.json')
root_dot = Path('$MONOREPO_DART_TOOL')
config = json.loads(config_path.read_text())
fixed = 0
for pkg in config['packages']:
    uri = pkg['rootUri']
    if not uri.startswith('file://'):
        pkg['rootUri'] = (root_dot / uri).resolve().as_uri()
        fixed += 1
config_path.write_text(json.dumps(config, indent=2))
print(f'   Fixed {fixed} relative paths → absolute file://')
"
[[ -f "$PROJECT_ROOT/.flutter-plugins-dependencies" ]] && cp "$PROJECT_ROOT/.flutter-plugins-dependencies" "$APP_DIR/.flutter-plugins-dependencies" && echo "   .flutter-plugins-dependencies copied"
[[ -f "$PROJECT_ROOT/.flutter-plugins" ]] && cp "$PROJECT_ROOT/.flutter-plugins" "$APP_DIR/.flutter-plugins"
PUBSPEC_VERSION=$(grep '^version:' "$APP_DIR/pubspec.yaml" | head -1 | sed 's/version: //')
CUR_NAME=$(echo "$PUBSPEC_VERSION" | cut -d+ -f1)
CUR_CODE=$(echo "$PUBSPEC_VERSION" | cut -d+ -f2)
BUILD_NAME="${BUILD_NAME:-$CUR_NAME}"
BUILD_NUMBER="${BUILD_NUMBER:-$((CUR_CODE + 1))}"
echo "📦 Building mobile-technician v${BUILD_NAME}+${BUILD_NUMBER}"
API_BASE_URL="${API_BASE_URL:-http://103.190.112.214:3000}"
WS_BASE_URL="${WS_BASE_URL:-ws://103.190.112.214:3000}"
cd "$APP_DIR"
flutter build apk --release --no-pub --target-platform android-arm64 --build-number="$BUILD_NUMBER" --build-name="$BUILD_NAME" --dart-define=API_BASE_URL="$API_BASE_URL" --dart-define=WS_BASE_URL="$WS_BASE_URL"
APK_SRC="$APP_DIR/build/app/outputs/flutter-apk/app-release.apk"
APK_DST="/tmp/app-technician-release.apk"
[[ -f "$APK_SRC" ]] && cp "$APK_SRC" "$APK_DST" && echo "✅ APK ready: $APK_DST ($(du -h "$APK_DST" | cut -f1))" && echo "   Download: http://103.190.112.214:9999/app-technician-release.apk" || { echo "❌ Build failed"; exit 1; }
