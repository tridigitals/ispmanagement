# 📱 Cara Build APK — Customer App

APK belum bisa di-build di sandbox tempat AI saya jalan (sandbox **tidak punya Java** dan **tidak bisa install paket sistem** karena tidak ada hak root). Tapi ada **2 cara** untuk Anda dapatkan APK hari ini juga:

---

## ⭐ Opsi A: Build di Cloud (GitHub Actions) — PALING CEPAT, zero setup

Anda **tidak perlu install apa-apa** di laptop. Cukup push kode ke GitHub, APK auto-build di server mereka, download dari web.

### Langkah:

1. **Buat repo GitHub** untuk project ini (atau pakai yang sudah ada)
2. **Push kode** + workflow yang sudah saya kerjakan:
   ```bash
   cd /path/to/ISPMANAGEMENT
   git init  # kalau belum
   git add .
   git commit -m "Add customer app + GitHub Actions workflow"
   git branch -M main
   git remote add origin https://github.com/USERNAME/REPO.git
   git push -u origin main
   ```
3. **Buka tab Actions** di repo GitHub Anda → workflow `Build Customer App APK` akan jalan otomatis
4. **Tunggu 5-8 menit** (first build lebih lama, berikutnya cached)
5. **Download APK**: di halaman workflow run, scroll ke bawah → bagian **Artifacts** → klik `customer-app-debug-apk` → download zip → extract → ada `app-debug.apk`

### Trigger manual:

Buka `.github/workflows/build-customer-apk.yml` di GitHub → klik **Run workflow** → **Run workflow** (tombol hijau). Workflow jalan setiap kali Anda klik tombol ini.

### Screenshot path:
```
GitHub repo → tab "Actions" → kiri klik "Build Customer App APK" 
            → klik run yang sudah selesai (centang hijau)
            → scroll ke bawah "Artifacts"
            → download "customer-app-debug-apk"
            → extract zip → app-debug.apk
```

---

## 🛠️ Opsi B: Build di Laptop Sendiri (10-15 menit, perlu install toolchain)

### 1. Install Java 17
```bash
sudo apt update
sudo apt install -y openjdk-17-jdk
```

### 2. Install Flutter SDK
```bash
# Cara cepat (saya sudah siapkan script otomatisnya)
cd /path/to/ISPMANAGEMENT
chmod +x setup_flutter_nosudo.sh  # tidak butuh sudo, install ke ~/
./setup_flutter_nosudo.sh

# ATAU manual:
wget https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_3.24.5-stable.tar.xz
mkdir -p ~/flutter && tar xf flutter_linux_3.24.5-stable.tar.xz -C ~/flutter --strip-components=1
echo 'export PATH="$PATH:$HOME/flutter/bin"' >> ~/.bashrc
source ~/.bashrc
```

### 3. Install Android command-line tools
```bash
# Otomatis oleh setup script di step 2
# ATAU manual:
wget https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip
mkdir -p ~/Android/Sdk/cmdline-tools
unzip commandlinetools-linux-11076708_latest.zip -d ~/Android/Sdk/cmdline-tools/
mv ~/Android/Sdk/cmdline-tools/cmdline-tools ~/Android/Sdk/cmdline-tools/latest
echo 'export ANDROID_SDK_ROOT=$HOME/Android/Sdk' >> ~/.bashrc
echo 'export PATH="$PATH:$ANDROID_SDK_ROOT/cmdline-tools/latest/bin:$ANDROID_SDK_ROOT/platform-tools"' >> ~/.bashrc
source ~/.bashrc

# Accept licenses + install platform
yes | sdkmanager --licenses
sdkmanager "platform-tools" "platforms;android-34" "build-tools;34.0.0"
yes | flutter doctor --android-licenses
```

### 4. Verifikasi
```bash
flutter doctor -v
# Yang penting ✅:
#   - Flutter SDK
#   - Android toolchain
#   - Connected device (kalau HP dicolok USB)
```

### 5. Build APK
```bash
cd /path/to/ISPMANAGEMENT
./build_apk.sh
```

APK ada di: `apps/mobile-customer/build/app/outputs/flutter-apk/app-debug.apk`

---

## 📲 Install APK ke HP

### Cara 1: Copy manual
1. Copy `app-debug.apk` ke HP (USB / WA / Google Drive / email)
2. Buka file APK di HP
3. Tap **Install** (Android akan minta izin "Install dari sumber tidak dikenal" → Allow)
4. Buka app dari drawer

### Cara 2: Via ADB (HP USB Debugging ON)
```bash
adb install build/app/outputs/flutter-apk/app-debug.apk
# Update app yang sudah ada:
adb install -r build/app/outputs/flutter-apk/app-debug.apk
```

### Cara 3: Upload ke Firebase App Distribution / Diawi
- **Firebase**: free, max 150 testers, https://firebase.google.com/products/app-distribution
- **Diawi**: instant, tanpa daftar, https://www.diawi.com — upload APK → dapat link

---

## 🎨 Branding (opsional, kalau mau ganti icon & splash)

File branding ada di `apps/mobile-customer/assets/branding/`:
- `app_icon_android.png` (1024×1024 px) → icon launcher
- `app_icon_ios.png` (1024×1024 px) → icon iOS
- `app_icon_foreground.png` (1024×1024 px, padding ~25% tiap sisi) → adaptive icon
- `splash_logo.png` (512×512 px, background transparan) → logo di splash

Ganti file-file ini, lalu re-generate:
```bash
cd apps/mobile-customer
dart run flutter_launcher_icons
dart run flutter_native_splash:create
flutter build apk --debug
```

---

## ❓ Troubleshooting

| Error | Solusi |
|---|---|
| `flutter: command not found` | `source ~/.bashrc` atau buka terminal baru |
| `Android license status unknown` | `yes \| flutter doctor --android-licenses` |
| `BUILD FAILED: Could not determine java version` | `export JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64` |
| `cmdline-tools not found` | `export ANDROID_SDK_ROOT=$HOME/Android/Sdk` lalu `source ~/.bashrc` |
| `No space left on device` | Butuh minimal **5 GB** kosong (Flutter ~700MB + Android SDK ~3GB + build cache) |
| `Gradle build daemon disappeared` | `flutter clean` lalu coba lagi |
| `minSdkVersion < 23` | Edit `android/app/build.gradle`: `minSdk = 23` |

---

## 🏗️ Build untuk release (production)

```bash
# 1. Buat keystore (cuma sekali)
keytool -genkey -v -keystore ~/isp-customer-key.jks -keyalg RSA -keysize 2048 -validity 10000 -alias isp-customer

# 2. Encode base64 untuk GitHub secret
base64 ~/isp-customer-key.jks | tr -d '\n' > ~/isp-customer-key.b64

# 3. Set secrets di GitHub repo: Settings → Secrets → New repository secret
#    - SIGNING_KEY   (isi dari isp-customer-key.b64)
#    - KEY_ALIAS     = isp-customer
#    - KEY_PASSWORD  = <password>
#    - STORE_PASSWORD = <password>

# 4. Build release
flutter build apk --release
# Output: build/app/outputs/flutter-apk/app-release.apk (~25MB)
```

Atau App Bundle (untuk Play Store):
```bash
flutter build appbundle --release
# Output: build/app/outputs/bundle/release/app-release.aab
```

---

## 🆚 Opsi A vs Opsi B — mana yang lebih cocok?

| Aspek | Opsi A (GitHub Actions) | Opsi B (Laptop sendiri) |
|---|---|---|
| **Setup awal** | 5 menit (push ke GitHub) | 15 menit (install toolchain) |
| **Waktu build** | 5-8 menit (di server) | 5-8 menit (di laptop Anda) |
| **Butuh internet** | Ya (push + download) | Tidak setelah install |
| **Butuh install** | Tidak | Ya (Java + Flutter + Android SDK ~4GB) |
| **Paling cocok untuk** | Coba-coba cepat, share ke tester | Development harian, build lokal |

**Saran**: pakai Opsi A untuk **coba pertama kali** (lebih cepat), lalu kalau puas dan mau development serius, install toolchain di laptop.
