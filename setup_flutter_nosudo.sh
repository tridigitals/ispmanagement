#!/usr/bin/env bash
###############################################################################
# Setup Flutter + Android SDK di ~/ (TANPA sudo) — untuk user biasa
# Install ke $HOME/flutter dan $HOME/Android/Sdk
# Usage: bash setup_flutter_nosudo.sh
###############################################################################

set -e
set -o pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0m'
NC='\033[0m'
step() { echo -e "\n${GREEN}▶ $1${NC}"; }
warn() { echo -e "${YELLOW}⚠ $1${NC}"; }
fail() { echo -e "${RED}✗ $1${NC}"; exit 1; }

HOME_DIR="$HOME"
FLUTTER_DIR="$HOME_DIR/flutter"
SDK_DIR="$HOME_DIR/Android/Sdk"

# Detect java
JAVA_HOME_CANDIDATE=$(readlink -f "$(command -v java 2>/dev/null)" 2>/dev/null | sed 's|/bin/java||')
if [[ -z "$JAVA_HOME_CANDIDATE" || ! -d "$JAVA_HOME_CANDIDATE" ]]; then
  fail "Java tidak ada. Install Java 17 dulu: sudo apt install -y openjdk-17-jdk\n  Atau kalau pakai SDKMAN: sdk install java 17.0.13-tem"
fi
export JAVA_HOME="$JAVA_HOME_CANDIDATE"
echo "Using JAVA_HOME=$JAVA_HOME"
java -version 2>&1 | head -1

# Disk space check
AVAIL=$(df -BG "$HOME_DIR" | tail -1 | awk '{print $4}' | tr -d 'G')
if [[ "${AVAIL:-0}" -lt 8 ]]; then
  fail "Butuh minimal 8GB kosong di $HOME_DIR. Available: ${AVAIL}G"
fi

# =========================================================================
# STEP 1: Download Flutter SDK ke ~/flutter
# =========================================================================
step "[1/5] Download Flutter SDK ke $FLUTTER_DIR (~700MB)..."
if [[ ! -x "$FLUTTER_DIR/bin/flutter" ]]; then
  cd /tmp
  wget -q --show-progress "https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_3.24.5-stable.tar.xz" \
       -O flutter.tar.xz
  mkdir -p "$FLUTTER_DIR"
  tar -xJf flutter.tar.xz -C "$FLUTTER_DIR" --strip-components=1
  rm flutter.tar.xz
  echo "✅ Flutter SDK terinstall di $FLUTTER_DIR"
else
  echo "Flutter SDK sudah ada di $FLUTTER_DIR, skip"
fi

# =========================================================================
# STEP 2: Set environment variables
# =========================================================================
step "[2/5] Set environment variables di ~/.bashrc..."
BASHRC="$HOME_DIR/.bashrc"
# Bersihkan entry lama
sed -i '/flutter\/bin/d' "$BASHRC" 2>/dev/null
sed -i '/ANDROID_SDK_ROOT/d' "$BASHRC" 2>/dev/null
sed -i '/JAVA_HOME=/d' "$BASHRC" 2>/dev/null
# Tambah entry baru
cat >> "$BASHRC" <<EOF

# --- Flutter & Android SDK (added by setup_flutter_nosudo.sh) ---
export FLUTTER_HOME="$FLUTTER_DIR"
export ANDROID_SDK_ROOT="$SDK_DIR"
export ANDROID_HOME="\$ANDROID_SDK_ROOT"
export JAVA_HOME="$JAVA_HOME"
export PATH="\$FLUTTER_HOME/bin:\$ANDROID_SDK_ROOT/cmdline-tools/latest/bin:\$ANDROID_SDK_ROOT/platform-tools:\$PATH"
EOF
echo "✅ ~/.bashrc updated"

# Export juga untuk sesi script ini
export PATH="$FLUTTER_DIR/bin:$SDK_DIR/cmdline-tools/latest/bin:$SDK_DIR/platform-tools:$PATH"
export ANDROID_SDK_ROOT="$SDK_DIR"
export ANDROID_HOME="$SDK_DIR"

# =========================================================================
# STEP 3: Download Android command-line tools
# =========================================================================
step "[3/5] Download Android cmdline-tools ke $SDK_DIR..."
mkdir -p "$SDK_DIR/cmdline-tools"
if [[ ! -x "$SDK_DIR/cmdline-tools/latest/bin/sdkmanager" ]]; then
  cd /tmp
  wget -q --show-progress "https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip" \
       -O cmdtools.zip
  unzip -q cmdtools.zip -d "$SDK_DIR/cmdline-tools/"
  # Struktur sdkmanager expects: $SDK/cmdline-tools/latest/bin/sdkmanager
  if [[ -d "$SDK_DIR/cmdline-tools/cmdline-tools" && ! -d "$SDK_DIR/cmdline-tools/latest" ]]; then
    mv "$SDK_DIR/cmdline-tools/cmdline-tools" "$SDK_DIR/cmdline-tools/latest"
  fi
  rm cmdtools.zip
  echo "✅ Android cmdline-tools terinstall"
else
  echo "Android cmdline-tools sudah ada, skip"
fi

# =========================================================================
# STEP 4: Accept licenses + install SDK
# =========================================================================
step "[4/5] Accept licenses & install Android SDK platform 34..."
yes 2>/dev/null | "$SDK_DIR/cmdline-tools/latest/bin/sdkmanager" --licenses >/dev/null 2>&1 || warn "License accept (may already be accepted)"

"$SDK_DIR/cmdline-tools/latest/bin/sdkmanager" \
  "platform-tools" \
  "platforms;android-34" \
  "build-tools;34.0.0" \
  >/dev/null 2>&1

echo "✅ Android SDK platform 34, build-tools 34.0.0, platform-tools terinstall"

# Flutter Android licenses
yes 2>/dev/null | "$FLUTTER_DIR/bin/flutter" doctor --android-licenses >/dev/null 2>&1 || warn "Flutter licenses (skip)"

# =========================================================================
# STEP 5: Verifikasi
# =========================================================================
step "[5/5] Verifikasi instalasi (flutter doctor)..."
echo ""
"$FLUTTER_DIR/bin/flutter" config --no-analytics >/dev/null 2>&1 || true
"$FLUTTER_DIR/bin/flutter" doctor 2>&1 | head -30 || true

echo ""
echo -e "${GREEN}============================================================${NC}"
echo -e "${GREEN}✅ SETUP SELESAI! (no-sudo mode)${NC}"
echo -e "${GREEN}============================================================${NC}"
echo ""
echo "📁 Lokasi install:"
echo "   Flutter SDK : $FLUTTER_DIR"
echo "   Android SDK : $SDK_DIR"
echo "   Java        : $JAVA_HOME"
echo ""
echo "Langkah selanjutnya:"
echo "  1. Buka terminal BARU, atau ketik:  source ~/.bashrc"
echo "  2. Pergi ke project:"
echo "     cd /home/xtrabit/ISPMANAGEMENT"
echo "  3. Build APK:"
echo "     ./build_apk.sh"
echo ""
echo "APK akan muncul di:"
echo "  apps/mobile-customer/build/app/outputs/flutter-apk/app-debug.apk"
echo ""
