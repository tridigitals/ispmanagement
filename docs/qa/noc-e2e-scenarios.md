# NOC E2E Scenarios

Dokumen ini jadi checklist regression untuk flow kritikal NOC di web browser.

## Persiapan

1. Jalankan backend API.
2. Jalankan frontend (`npm run dev`) atau deploy environment staging.
3. Login sebagai admin tenant yang punya akses menu network.

## Skenario 1: Token Expired Auto Redirect

Tujuan: pastikan sesi invalid langsung keluar ke login.

Langkah:

1. Masuk ke halaman `admin/network/noc`.
2. Buka DevTools, hapus/invalidasi token (atau paksa backend balas 401).
3. Trigger request API (misalnya klik refresh atau pindah tab network lain).

Ekspektasi:

1. Session local dibersihkan.
2. User diarahkan ke `/login?reason=expired`.
3. Tidak stuck di loading screen.

## Skenario 1b: Idle Timeout (Tidak Aktif)

Tujuan: pastikan sesi **tidak** expired saat user aktif, dan expired saat benar-benar idle.

Langkah:

1. Set `auth_session_timeout_minutes` ke nilai kecil (contoh 1-2 menit) di superadmin settings.
2. Login, lalu tetap aktif (klik menu/pindah halaman) lebih lama dari timeout.
3. Verifikasi user tetap login.
4. Setelah itu biarkan tab benar-benar idle tanpa akses halaman sampai lewat timeout.
5. Trigger 1 request API (refresh/pindah halaman).

Ekspektasi:

1. Saat aktif, sesi tetap valid.
2. Saat idle lewat timeout, request berikutnya diarahkan ke `/login?reason=expired`.

## Skenario 2: Wallboard Long-Run

Tujuan: pastikan monitor tetap jalan lama.

Langkah:

1. Masuk ke `admin/network/noc/wallboard`.
2. Biarkan halaman berjalan minimal 15 menit.
3. Ganti layout, ubah poll interval, buka/close fullscreen.

Ekspektasi:

1. Tile traffic tetap update.
2. Tidak ada error loop di console.
3. State toolbar/layout tetap konsisten.

## Skenario 3: Incident Detail Drawer

Tujuan: pastikan detail insiden usable penuh setelah refactor komponen.

Langkah:

1. Buka `admin/network/incidents`.
2. Klik salah satu incident untuk buka detail drawer.
3. Ubah assignee + notes, simpan.
4. Coba action `Acknowledge`/`Resolve`.

Ekspektasi:

1. Drawer tampil normal (runbook + timeline + SLA + metrik ringkas).
2. Save notes berhasil dan data list ikut update.
3. Acknowledge/Resolve reflected di tabel.

## Skenario 4: Simulate Incident Drawer

Tujuan: pastikan drawer simulasi tetap bekerja.

Langkah:

1. Klik tombol `Simulate` di halaman incidents.
2. Pilih router/type/severity, isi message optional.
3. Submit simulasi.

Ekspektasi:

1. Incident baru muncul di list.
2. Tidak ada error deserialisasi payload.
3. Drawer bisa ditutup dengan tombol close/cancel.

## Skenario 5: Export Data

Tujuan: validasi export incidents.

Langkah:

1. Di halaman incidents, klik `Export`.
2. Pilih `CSV` lalu `Excel`.

Ekspektasi:

1. File terdownload.
2. Header dan nilai kolom sesuai filter aktif.
