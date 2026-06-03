#!/usr/bin/env bash
###############################################################################
# Build Customer App APK (debug) — untuk ISP Management
# Usage: chmod +x build_apk.sh && ./build_apk.sh
###############################################################################

set -e
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0m'
NC='\033[0m'
step() { echo -e "\n${GREEN}▶ $1${NC}"; }
warn() { echo -e "${YELLOW}⚠ $1${NC}"; }
fail() { echo -e "${RED}✗ $1${NC}"; exit 1; }

# Cari root project
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"
if [[ ! -d "$PROJECT_ROOT/apps/mobile-customer" ]]; then
  PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
fi
APP_DIR="$PROJECT_ROOT/apps/mobile-customer"

if [[ ! -d "$APP_DIR" ]]; then
  fail "Folder apps/mobile-customer tidak ditemukan di $PROJECT_ROOT"
fi

step "Project root: $PROJECT_ROOT"
step "App dir     : $APP_DIR"

# Source bashrc kalau ada
[[ -f "$HOME/.bashrc" ]] && source "$HOME/.bashrc" 2>/dev/null || true
export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}"
export PATH="$PATH:/opt/flutter/bin:$ANDROID_SDK_ROOT/cmdline-tools/latest/bin:$ANDROID_SDK_ROOT/platform-tools"

command -v flutter >/dev/null || fail "Flutter tidak ada di PATH. Jalankan setup_flutter.sh dulu."

# =========================================================================
# 1. Install dependencies untuk semua packages
# =========================================================================
step "[1/5] Install dependencies (root, api-client, ui-kit, mobile-customer)..."

# Root melos (kalau ada)
if [[ -f "$PROJECT_ROOT/pubspec.yaml" ]] && grep -q "melos" "$PROJECT_ROOT/pubspec.yaml" 2>/dev/null; then
  cd "$PROJECT_ROOT"
  flutter pub get
  dart run melos bootstrap || warn "Melos bootstrap gagal (mungkin tidak critical)"
fi

# Per package
for pkg in api-client ui-kit; do
  if [[ -d "$PROJECT_ROOT/packages/$pkg" ]]; then
    echo "  → $pkg"
    cd "$PROJECT_ROOT/packages/$pkg"
    flutter pub get
  fi
done

cd "$APP_DIR"
flutter pub get

# =========================================================================
# 2. Buat branding placeholder kalau belum ada
# =========================================================================
step "[2/5] Siapkan branding assets (placeholder)..."
mkdir -p assets/branding
for f in app_icon_android.png app_icon_ios.png app_icon_foreground.png splash_logo.png; do
  if [[ ! -s "assets/branding/$f" ]]; then
    # Buat PNG 1x1 transparan sebagai placeholder
    # (PNG terkecil: 67 byte)
    printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\rIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\x0d\n-\xb4\x00\x00\x00\x00IEND\xaeB`\x82' \
      > "assets/branding/$f"
    echo "  → placeholder assets/branding/$f dibuat (67 byte)"
  else
    echo "  → assets/branding/$f sudah ada, skip"
  fi
done

# =========================================================================
# 3. Generate launcher icon & native splash
# =========================================================================
step "[3/5] Generate app icon & splash screen..."
dart run flutter_launcher_icons -f flutter_launcher_icons.yaml || warn "Icon generate gagal (skip, APK masih bisa built)"
dart run flutter_native_splash:create || warn "Splash generate gagal (skip)"

# =========================================================================
# 4. Build APK
# =========================================================================
step "[4/5] Build APK debug (5-8 menit untuk first build)..."
flutter clean
flutter build apk --debug

# =========================================================================
# 5. Tampilkan hasil
# =========================================================================
APK_PATH="$APP_DIR/build/app/outputs/flutter-apk/app-debug.apk"
step "[5/5] ✅ BUILD SELESAI!"
echo ""
if [[ -f "$APK_PATH" ]]; then
  SIZE=$(du -h "$APK_PATH" | cut -f1)
  echo -e "${GREEN}📦 APK: $APK_PATH ($SIZE)${NC}"
  echo ""
  echo "Cara install ke HP:"
  echo "  1. Copy file APK ke HP (USB / WA / Drive / email)"
  echo "  2. Buka file APK di HP"
  echo "  3. Tap 'Install' (izinkan 'Install dari sumber tidak dikenal' kalau diminta)"
  echo ""
  echo "Atau via ADB (HP terhubung USB dengan USB Debugging ON):"
  echo "  adb install $APK_PATH"
else
  fail "APK tidak ditemukan di $APK_PATH — cek error di atas"
fi
