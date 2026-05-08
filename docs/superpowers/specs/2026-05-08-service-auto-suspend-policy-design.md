# Service Auto Suspend Policy Design

## Goal

Rapikan pengaturan lifecycle layanan pelanggan dengan menambahkan tab `Service` di admin settings, lalu mendukung policy auto-suspend global yang bisa memakai `grace period` atau `fixed day`.

## Problem

Saat ini pengaturan yang terkait lifecycle layanan pelanggan masih tersebar atau tercampur di area `Payments`, padahal sebagian besar setting tersebut bukan konfigurasi payment gateway, melainkan aturan operasional subscription pelanggan.

Kondisi yang ada sekarang:
- auto invoice customer sudah ada
- auto suspend sudah ada, tetapi masih berbasis grace period
- auto resume on payment sudah ada
- billing reminder sudah ada
- customer detail belum menampilkan hasil policy secara jelas, terutama `masa aktif akhir`, `policy auto suspend`, dan `perkiraan suspend`

Akibatnya:
- user admin sulit menemukan pengaturan service lifecycle
- `Payments` menjadi terlalu padat dan domainnya tercampur
- policy auto-suspend belum cukup fleksibel untuk kebutuhan operasional global

## Product Decision

Tambahkan tab settings baru bernama `Service` dan jadikan tab itu sebagai pusat pengaturan lifecycle layanan pelanggan.

Policy auto-suspend berlaku global untuk semua customer dan semua layanan customer subscription. Customer detail tidak menjadi sumber pengaturan, hanya menjadi tempat observability hasil policy.

## Scope

Desain ini mencakup:
- tab `Service` baru di admin settings
- pemindahan setting lifecycle layanan dari `Payments` ke `Service`
- dukungan mode auto-suspend global:
  - `grace_period`
  - `fixed_day`
- batas `fixed_day` ke rentang `1-28`
- penampilan `masa aktif`, `policy auto suspend`, dan `perkiraan suspend` di customer detail subscription view

Desain ini tidak mencakup:
- override per customer
- override per subscription
- policy berbeda per service type
- perubahan besar pada billing collection flow

## Settings Information Architecture

### Tab `Payments`

Tab `Payments` hanya berisi:
- payment gateway settings
- manual transfer settings
- payment provider operational credentials

### Tab `Service`

Tab `Service` menjadi rumah untuk setting berikut:
- `customer_invoice_auto_generate_enabled`
- `customer_invoice_generate_days_before_due`
- `customer_invoice_scheduler_interval_minutes`
- `billing_auto_suspend_enabled`
- `billing_auto_suspend_mode`
- `billing_auto_suspend_grace_days`
- `billing_auto_suspend_fixed_day`
- `billing_auto_resume_on_payment`
- `billing_reminder_enabled`
- `billing_reminder_schedule`

Tab ini perlu menampilkan ringkasan singkat:
- `Policy ini berlaku global untuk semua customer dan layanan.`

## Auto Suspend Policy

### Setting keys

Setting existing yang tetap dipakai:
- `billing_auto_suspend_enabled`
- `billing_auto_suspend_grace_days`
- `billing_auto_resume_on_payment`

Setting baru:
- `billing_auto_suspend_mode`
- `billing_auto_suspend_fixed_day`

### Allowed values

`billing_auto_suspend_mode`:
- `grace_period`
- `fixed_day`

`billing_auto_suspend_fixed_day`:
- integer `1..28`

### Mode behavior

#### `grace_period`

Jika mode `grace_period` aktif:
- sistem menghitung tanggal suspend dari `subscription.ends_at + billing_auto_suspend_grace_days`
- subscription hanya boleh disuspend otomatis jika tanggal hitung tersebut sudah tercapai

Jika `ends_at` tidak ada:
- tanggal suspend tidak dapat dihitung
- UI harus menampilkan bahwa data masa aktif belum lengkap

#### `fixed_day`

Jika mode `fixed_day` aktif:
- sistem memakai tanggal bulanan global untuk suspend otomatis
- nilai tanggal dibatasi ke `1..28` agar aman di semua bulan

Aturan hitung yang direkomendasikan:
- jika subscription masih aktif dan belum melewati `ends_at`, tampilkan tanggal suspend terdekat yang relevan setelah masa aktif
- backend scheduler hanya boleh menyuspend subscription yang sudah melewati `ends_at` dan sudah mencapai hari suspend bulanan yang berlaku

Tujuannya:
- fixed day tetap mengikuti lifecycle layanan
- tidak menyuspend subscription sebelum masa aktifnya berakhir

## Customer Detail Observability

Di `Customer Detail > Subscriptions`, setiap row atau card subscription perlu menampilkan:
- `Masa aktif sampai`
- `Policy auto suspend`
- `Perkiraan suspend`

Perilaku UI:
- jika `ends_at` ada, tampilkan tanggal masa aktif akhir
- jika mode global `grace_period`, tampilkan contoh `Grace 3 hari setelah masa aktif`
- jika mode global `fixed_day`, tampilkan contoh `Suspend tanggal 20 setiap bulan`
- jika `ends_at` tidak ada dan mode membutuhkan basis masa aktif, tampilkan:
  - `Belum ada masa aktif akhir`
  - `Suspend otomatis tidak bisa dihitung`

Customer detail tidak boleh menjadi tempat edit policy global. Informasi di sini read-only agar tidak membingungkan user.

## Backend Shape

### Seed defaults

Tambahkan default setting:
- `billing_auto_suspend_mode = grace_period`
- `billing_auto_suspend_fixed_day = 1`

Key lama harus tetap dipertahankan agar tenant existing tidak rusak.

### Scheduler logic

Logika backend auto-suspend perlu membaca:
- enabled state
- selected mode
- grace days jika mode `grace_period`
- fixed day jika mode `fixed_day`

Backend harus tetap defensif:
- abaikan fixed day di luar `1..28`
- fallback ke `grace_period` jika mode tidak dikenal
- jangan suspend subscription tanpa `ends_at` bila mode butuh basis masa aktif

### Resume behavior

`billing_auto_resume_on_payment` tetap global dan tidak berubah secara konsep:
- invoice dibayar
- subscription suspended karena policy billing
- subscription bisa diaktifkan kembali otomatis sesuai aturan existing

## Validation Rules

- `billing_auto_suspend_grace_days` harus integer `>= 0`
- `billing_auto_suspend_fixed_day` harus integer `1..28`
- jika `billing_auto_suspend_enabled = false`, mode dan nilai turunannya tetap boleh disimpan tetapi tidak dipakai scheduler
- jika mode `grace_period`, UI tetap boleh menampilkan field fixed day tetapi dalam kondisi disabled atau tersembunyi
- jika mode `fixed_day`, UI tetap boleh menampilkan grace days tetapi dalam kondisi disabled atau tersembunyi

## UX Recommendation

Urutan section di tab `Service`:
1. `Auto Invoice`
2. `Auto Suspend`
3. `Auto Resume`
4. `Reminder`

Bagian `Auto Suspend` disarankan berisi:
- toggle enable
- radio/select mode
- conditional numeric input:
  - grace days
  - fixed day `1..28`
- helper text yang menjelaskan efek global

## Testing Strategy

Minimum backend coverage:
- mode `grace_period` menghitung suspend sesuai `ends_at + grace_days`
- mode `fixed_day` hanya menghasilkan tanggal valid `1..28`
- mode invalid fallback ke default aman
- subscription tanpa `ends_at` tidak ikut auto-suspend saat basis masa aktif dibutuhkan

Minimum frontend coverage:
- settings category `Service` muncul
- keys lifecycle service tidak lagi tampil di `Payments`
- switching mode menampilkan input yang sesuai
- customer detail menampilkan `masa aktif`, `policy`, dan `perkiraan suspend`
- fallback observability muncul saat `ends_at` kosong

## Risks and Constraints

- Bila fixed-day scheduler tidak didefinisikan terhadap `ends_at`, sistem bisa menyuspend terlalu cepat. Karena itu fixed day harus tetap menghormati akhir masa aktif.
- Pemindahan setting dari `Payments` ke `Service` harus hati-hati agar tidak menghilangkan key existing dari form save/load.
- Customer detail perlu cukup informatif tanpa membuka peluang edit policy di lokasi yang salah.

## Recommended Implementation Shape

Frontend:
- tambah kategori settings `Service`
- pindahkan rendering field lifecycle layanan dari `Payments` ke `Service`
- tambah field baru mode dan fixed day
- tambah observability policy di customer detail subscription list

Backend:
- tambah default key baru di seed settings
- perluas pembacaan config auto-suspend di payment/billing service
- tambahkan helper untuk menghitung preview tanggal suspend agar frontend dan backend tidak menyimpang bila helper bersama memungkinkan
