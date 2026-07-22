# Build Mobile APK — Panduan Lengkap

Panduan ini mencakup persiapan environment sampai build release APK untuk **Customer App** dan **Technician App** di monorepo ISPMANAGEMENT.

---

## 1. Prasyarat Sistem

### 1.1. Toolchain yang dibutuhkan

| Tool | Versi | Lokasi default |
|---|---|---|
| Flutter | 3.24.x | `~/sdk/flutter/bin/flutter` |
| Java (JDK) | 17 (Temurin) | `~/sdk/java17` |
| Android SDK | Build-tools 34.0.0 | `~/sdk/android-sdk` |
| Android NDK | 27.0.12077973 | `~/sdk/android-sdk/ndk/27.0.12077973` |
| Gradle | 8.9 | (embedded wrapper) |
| Kotlin | 2.1.0 | (via Gradle plugin) |
| Python 3 | 3.x | System default |

### 1.2. Install SDK (server baru / clean)

```bash
# Flutter 3.24
cd ~/sdk
git clone https://github.com/flutter/flutter.git -b stable flutter
~/sdk/flutter/bin/flutter precache
~/sdk/flutter/bin/flutter config --no-analytics

# Android SDK + NDK (via sdkmanager atau commandline tools)
# Pastikan build-tools 34.0.0 terinstall

# Java 17
sudo apt install openjdk-17-jdk  # atau download Temurin
```

### 1.3. Environment variables (wajib di-export sebelum build)

```bash
export ANDROID_HOME="$HOME/sdk/android-sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export JAVA_HOME="$HOME/sdk/java17"
export PATH="$HOME/sdk/flutter/bin:$ANDROID_HOME/build-tools/34.0.0:$JAVA_HOME/bin:$PATH"
```

> 💡 Tambahkan ke `~/.bashrc` agar persistent antar session.

---

## 2. Clone Repository

```bash
git clone https://github.com/tridigitals/ispmanagement.git ~/ISPMANAGEMENT
cd ~/ISPMANAGEMENT
```

**PENTING:** repo ini monorepo Flutter. Root `pubspec.yaml` mendefinisikan `workspace:` yang mencakup semua apps di `apps/` dan packages di `packages/`.

---

## 3. Persiapan Awal (sekali saja)

### 3.1. Install dependency seluruh workspace

```bash
cd ~/ISPMANAGEMENT
flutter pub get
```

Ini akan menghasilkan `.dart_tool/package_config.json` di root — **wajib ada** sebelum build APK.

### 3.2. Pastikan `pubspec_overrides.yaml` tidak ada

```bash
rm -f apps/mobile-customer/pubspec_overrides.yaml
rm -f apps/mobile-technician/pubspec_overrides.yaml
rm -f apps/mobile-admin/pubspec_overrides.yaml
```

> File ini (hasil `dart pub add` manual atau `melos`) menyebabkan error `Cannot override workspace packages` saat build.

### 3.3. Verifikasi struktur package_config

```bash
ls -la .dart_tool/package_config.json
```

### 3.4. Keystore signing

Copy `release-key.jks` ke folder android masing-masing app:

```bash
# Dari server utama ke server baru
scp user@server-utama:/path/ke/release-key.jks ~/ISPMANAGEMENT/apps/mobile-customer/android/release-key.jks
scp user@server-utama:/path/ke/release-key.jks ~/ISPMANAGEMENT/apps/mobile-technician/android/release-key.jks
```

Detail keystore:
- **Alias:** `ispcustomer`
- **Cert SHA-256:** `1c1f8d036144163aaeea1feab65a7272f821dc700a3ac0685042caa6d81e30d4`

---

## 4. Konfigurasi Environment

`.env.example` sudah berisi key yang dibutuhkan. Copy dan isi:

```bash
cp .env.example .env
```

**Minimal 2 env wajib untuk build mobile:**

```bash
# di .env (atau export langsung sebelum build)
API_BASE_URL=https://api-isp.najahababy.com
WS_BASE_URL=wss://api-isp.najahababy.com
```

> ⚠️ **Tidak boleh kosong.** Default value di semua config Dart sudah `''` (empty). Build script akan exit error jika `API_BASE_URL` tidak diset.
>
> Ini sengaja: app akan crash explicit → developer dipaksa set env. Tidak ada lagi koneksi ke URL production lama secara diam-diam.

---

## 5. Build APK

### 5.1. Customer App

```bash
cd ~/ISPMANAGEMENT/apps/mobile-customer

# Set env + build
API_BASE_URL="https://api-isp.najahababy.com" \
WS_BASE_URL="wss://api-isp.najahababy.com" \
./scripts/build-apk.sh
```

### 5.2. Technician App

```bash
cd ~/ISPMANAGEMENT/apps/mobile-technician

# Set env + build
API_BASE_URL="https://api-isp.najahababy.com" \
WS_BASE_URL="wss://api-isp.najahababy.com" \
./scripts/build-apk.sh
```

### 5.3. Custom build number / version

```bash
BUILD_NAME="1.2.0" BUILD_NUMBER="42" \
API_BASE_URL="https://api-isp.najahababy.com" \
WS_BASE_URL="wss://api-isp.najahababy.com" \
./scripts/build-apk.sh
```

Tanpa `BUILD_NUMBER`, script akan auto-bump dari `pubspec.yaml` (`versionCode + 1`).

---

## 6. Output APK & Deploy

### 6.1. Lokasi output

Build script menaruh APK di `/tmp/app-release.apk`. Untuk production:

```bash
# Customer — rename & copy ke APK server
cp apps/mobile-customer/build/app/outputs/flutter-apk/app-release.apk \
   ~/apk-server/mobile-customer-arm64.apk

# Technician
cp apps/mobile-technician/build/app/outputs/flutter-apk/app-release.apk \
   ~/apk-server/mobile-technician-arm64.apk
```

### 6.2. Jalankan server APK (jika belum)

```bash
cd ~/apk-server
python3 -m http.server 9999 --bind 0.0.0.0 &
```

Akses:
- `http://<IP>:9999/mobile-customer-arm64.apk`
- `http://<IP>:9999/mobile-technician-arm64.apk`

---

## 7. Verifikasi APK

### 7.1. Cek versionCode

```bash
$ANDROID_HOME/build-tools/34.0.0/aapt dump badging ~/apk-server/mobile-customer-arm64.apk | grep "package:"
# → package: name='com.tridigitals.customer' versionCode='68' versionName='0.1.0'
```

### 7.2. Cek signing

```bash
$ANDROID_HOME/build-tools/34.0.0/apksigner verify --print-certs ~/apk-server/mobile-customer-arm64.apk
# → CN=ISP Customer, OU=Tridigitals
# → SHA-256: 1c1f8d036144163aaeea1feab65a7272f821dc700a3ac0685042caa6d81e30d4
```

### 7.3. Cek ukuran

```bash
du -h ~/apk-server/mobile-customer-arm64.apk
# → ~35MB (arm64-v8a only)
```

---

## 8. Workflow Build Script (internal)

Script `build-apk.sh` melakukan:

1. **Patch `package_config.json`** — copy dari root `.dart_tool/`, rewrite relative paths ke absolute `file://` URIs
2. **Copy plugin registry** — `.flutter-plugins` + `.flutter-plugins-dependencies` dari root ke app dir (Gradle perlu ini)
3. **Patch gal** — hardcode `compileSdk 35` (workspace mode tidak inject `flutter.compileSdkVersion`)
4. **Validasi `API_BASE_URL`** — exit error jika kosong
5. **`flutter build apk --release --no-pub --target-platform android-arm64`** — dengan `--dart-define`

> `--no-pub` digunakan karena `flutter pub get` di workspace mode akan error. Dependencies sudah resolved dari step 1.

---

## 9. CI/CD (GitHub Actions)

Workflow `.github/workflows/build-customer-apk.yml` build **debug APK** otomatis tiap push ke branch `main`/`master`/`develop`.

**Release APK** (signed) hanya jalan via `workflow_dispatch` (manual trigger) jika secret GitHub `SIGNING_KEY` diset.

---

## 10. Troubleshooting

| Error | Penyebab | Solusi |
|---|---|---|
| `flutter: command not found` | PATH belum diset | Export `PATH=$HOME/sdk/flutter/bin:$PATH` |
| `pub did not create .dart_tools/package_config.json` | Belum `flutter pub get` di root | `cd ~/ISPMANAGEMENT && flutter pub get` |
| `Cannot override workspace packages` | `pubspec_overrides.yaml` masih ada | `rm apps/*/pubspec_overrides.yaml` |
| `Could not get unknown property 'flutter'` | gal build.gradle | Sudah di-patch oleh script (idempotent) |
| `API_BASE_URL not set` | Env kosong | Export `API_BASE_URL` sebelum build |
| `versionCode must be higher` | VersionCode APK ≤ yang terinstall | Bump di `pubspec.yaml` atau override `BUILD_NUMBER` |
| `App not installed` di device | Signing key mismatch | Uninstall APK lama dulu (key berubah di commit `8b8c86a`) |
| `MissingPluginException` | Native plugin tidak terdaftar | `flutter clean` + jalankan ulang workspace workaround |
| Gradle NDK error | `ndkVersion = flutter.ndkVersion` gagal resolve | Sudah hardcode `27.0.12077973` di `build.gradle` |
| L10n getter undefined | File `.arb` belum diregenerate | `flutter gen-l10n`, jangan edit file `app_localizations*.dart` |

---

## 11. Perbedaan App

| Fitur | Customer | Technician |
|---|---|---|
| Rating & Survey | ✅ | ❌ |
| Create Ticket | ✅ | ❌ |
| Payment | ✅ | ❌ |
| Work Order | ❌ | ✅ |
| Scanner | ❌ | ✅ |

---

## 12. Struktur Direktori

```
ISPMANAGEMENT/
├── apps/
│   ├── mobile-customer/    # Flutter — Customer app
│   ├── mobile-technician/  # Flutter — Technician app
│   └── mobile-admin/       # Flutter — Admin app
├── packages/
│   ├── api-client/         # Shared Dart API client
│   ├── config/             # Shared BuildConfig (dart-define)
│   └── ui-kit/             # Shared UI components
├── pubspec.yaml            # Workspace root
├── .env.example            # Template environment
└── release-key.jks         # Signing keystore (salin dari server utama)
```
