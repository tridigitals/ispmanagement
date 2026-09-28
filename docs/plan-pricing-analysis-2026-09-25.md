# Analisa Setting Plan — Harga & Pembatasan

> **STATUS IMPLEMENTASI (2026-09-25):** P0 (limit resource) + P1 (api_access & audit_logs → Pro)
> **sudah dieksekusi** — lihat bagian 6 di bawah. P2–P5 masih rekomendasi.

Tanggal: 2026-09-25
Sumber harga: halaman resmi vendor (curl langsung), kurs USD/IDR 17.921 (exchangerate-api, 28 Sep 2026).
Data internal: DB produksi `ispmanagement` @127.0.0.1:55432.

---

## 1. Kondisi plan saat ini

| Plan | Harga/bln | Tahunan (diskon) | Efektif/bln |
|---|---|---|---|
| Free | 0 | 0 | 0 |
| Pro | 290.000 | 2.900.000 (16,7%) | 241.667 |
| Enterprise | 990.000 | 9.900.000 (16,7%) | 825.000 |

Diskon tahunan = 2 bulan gratis (16,7%) — lebih agresif dari MixRadius (5–10%). Bagus untuk retensi, pertahankan.

### Matriks fitur aktual

| Fitur | Free | Pro | Enterprise | Di-enforce? |
|---|---|---|---|---|
| max_users (akun tim) | 3 | 20 | unlimited | ✅ team_service |
| max_members | 2 | 10 | unlimited | ❌ tidak dipakai |
| max_storage_gb | 1 | 50 | 500 | ✅ storage_service |
| api_access | false | false | true | ❌ tidak ada implementasi |
| custom_domain | false | true | true | ✅ tenant.rs |
| remove_branding | false | false | true | ❌ tidak ada branding dinamis |
| audit_logs | false | false | true | ✅ audit_service |
| managed_radius | false | false | true | ✅ mikrotik/superadmin (4 tempat) |
| sso_support | false | false | true | ❌ tidak ada implementasi SSO |
| support_level | community | priority | dedicated | ❌ hanya label |

**Hanya 5 dari 10 fitur yang punya efek nyata.** 5 sisanya dijual/ditampilkan tanpa pengaruh.

### Skala nyata tenant

| Tenant | Plan | Pelanggan | Router | OLT | Anggota |
|---|---|---|---|---|---|
| ISP Management | enterprise | **548** | 3 | 3 | 6 |
| JMK | enterprise | 5 | 1 | 0 | 7 |
| Default Tenant | free | 2 | 0 | 0 | 2 |

Tidak ada limit untuk **pelanggan, router, OLT, concurrent session** — padahal inilah yang membedakan skala ISP.

---

## 2. Perbandingan kompetitor (harga terverifikasi)

### MixRadius Cloud (Indonesia, per instance ISP)

| Paket | Rp/bln | User online | Router | Pelanggan |
|---|---|---|---|---|
| CM LITE 1 | 50.000 | 150 | 1 | 250 |
| CM LITE 2 | 100.000 | 300 | 2 | 500 |
| CM MED 1 | 150.000 | 450 | 3 | 750 |
| CM MED 2 | 200.000 | 600 | 4 | 1.000 |
| CM PRO 1800 | 600.000 | 1.800 | 20 | **unlimited** |

Termasuk: voucher gratis (15.000–65.000/paket), **WA Gateway gratis**, VPN RADIUS, update.

### MixRadius Evolution (multi-tenant 4 tingkat — pembanding langsung Enterprise)

| Paket | Rp/bln | User online | Router |
|---|---|---|---|
| Lite | 50.000 | 150 | 1 |
| Starter | 100.000 | 300 | 2 |
| Basic | 150.000 | 450 | 3 |
| Standard | 200.000 | 600 | 4 |
| Pro | 400.000 | 1.200 | 10 |

### Cloud BillSpot (basis Mikrotik API, bukan RADIUS): 50.000 → 600.000 (struktur sama)

### Global

| Vendor | Biaya | Model |
|---|---|---|
| Splynx | $160/bln (≈Rp 2.868.000) untuk 1.700 pelanggan, lalu $0,095/pelanggan | per subscriber |
| Sonar | mulai $500/bln (≈Rp 8.960.000) dengan kontrak; atau $1,25/pelanggan/bln | per subscriber |

Biaya Sonar pada skala Anda: $1,25 × 548 = $685 ≈ **Rp 12.278.000/bln**.

### Posisi Anda

| Pembanding | Biaya/bln setara skala 548 pelanggan |
|---|---|
| MixRadius CM MED 2 | Rp 200.000 |
| MixRadius CM PRO | Rp 600.000 |
| **Anda (Enterprise)** | **Rp 990.000** |
| Splynx | Rp 2.868.000 |
| Sonar | Rp 8.960.000 – 12.278.000 |

**Kesimpulan posisi:** mahal **1,7×–5×** vs kompetitor Indonesia; murah **3×–12×** vs global.

### Biaya per pelanggan — inilah angka pentingnya

| | Rp/pelanggan/bln |
|---|---|
| Anda (Rp 990.000 ÷ 548) | **1.807** |
| MixRadius PRO (Rp 600.000, 1.000 pel.) | 600 |
| MixRadius MED 2 | 200 |

Tampak mahal. **Tapi lihat dari sisi ekonomi pelanggan:** tenant 548 pelanggan dengan ARPU ~Rp 250.000 = omzet ±Rp 137 juta/bln. Biaya software Rp 990.000 = **0,72% dari omzet**. Benchmark SaaS industri: 1–3% dari revenue. Jadi **harga Anda masih sehat** — bukan alasan untuk turun harga.

---

## 3. Masalah sebenarnya (bukan angkanya)

1. **Unit limit salah.** Anda membatasi "akun tim" (10 fitur), yang tidak relevan untuk ISP — mereka jalan dengan 1–3 admin. Yang membedakan skala adalah **pelanggan, router, OLT, concurrent**. Semua itu **tanpa batas**: tenant 10 pelanggan dan 10.000 pelanggan bayar sama. Tidak ada alasan natural upgrade Free→Pro→Enterprise, dan Anda kehilangan kemampuan menaikkan harga seiring pertumbuhan.

2. **Pro tidak punya pembeda berarti.** Free = 3 user/1 GB; Pro = 20 user/50 GB. Untuk ISP, itu praktis hanya storage + custom domain. Bandingkan MixRadius Rp 150.000 = 750 pelanggan + 3 router.

3. **Batas 3 router di Enterprise sangat ketat.** Tenant Anda sendiri punya 3 router (pas mentok). MixRadius Rp 600.000 memberi 20 router. Tidak di-enforce sekarang, tapi kalau di-enforce akan menyakiti pelanggan terbesar Anda.

4. **5 fitur kosong dijual.** `sso_support` di Enterprise padahal belum ada implementasi SSO. `remove_branding` padahal tidak ada branding dinamis. Ini risiko kepercayaan saat pelanggan minta.

5. **Enterprise tidak benar-benar "unlimited".** Ada cap 500 GB. Pada 548 pelanggan, berkas/berkas tiket bisa mendekati. "Unlimited" di marketing vs 500 GB nyata = janji tidak konsisten.

6. **Free praktis tidak terpakai.** Tidak ada registrasi tenant publik (hanya superadmin yang bisa membuat tenant), jadi Free/Pro tidak akan diambil orang luar kecuali Anda undang manual.

7. **Fitur kompetitor yang Anda belum punya:** voucher (tabel `vouchers` tidak ada), TR-069/GenieACS, SmartOLT, reseller/outlet multi-tingkat.

   **Yang Anda unggul:** WA Gateway **BYO key** (Fonnte/Triwax) — nol COGS untuk Anda, sementara MixRadius membagikan WA gratis (biaya mereka). Anda juga sudah punya portal pelanggan + APK pelanggan.

---

## 4. Rekomendasi ber-rank

### P0 — Ganti unit limit ke metrik ISP

| Plan | Harga/bln | Pelanggan | Router | OLT | Concurrent | Storage | Tim |
|---|---|---|---|---|---|---|---|
| Free | 0 | 25 | 1 | 0 | 25 | 1 GB | 2 |
| **Growth (baru)** | **450.000** | 300 | 3 | 1 | 150 | 20 GB | 5 |
| Pro | 750.000 | 1.000 | 10 | 3 | 500 | 100 GB | 10 |
| Enterprise | 1.500.000 | unlimited | 25 | unlimited | unlimited | 500 GB | unlimited |

Tier **Growth** mengisi jurang Free→Enterprise sekarang. Untuk tenant Anda (548 pelanggan, 3 router, 3 OLT) → jatuh di **Pro Rp 750.000**. Secara omzet naik dari Rp 990.000 per tenant besar... tunggu: Enterprise 548 pelanggan tetap masuk Enterprise jika unlimited. Alternatif: Pro = 1.000 pelanggan menampung 548 → Rp 750.000 (turun dari 990.000 untuk tenant Anda sendiri, tapi membuka 3 tier yang bisa naik).

Catatan: kalau tidak ingin menurunkan pendapatan tenant sendiri, set Pro = 500 pelanggan dan Enterprise = unlimited dengan harga Rp 990.000 tetap. **Rekomendasi saya: pertahankan Rp 990.000 untuk Enterprise, tambah Growth Rp 450.000, dan beri Pro Rp 750.000 @ 1.000 pelanggan.** Tenant Anda boleh tetap di Enterprise (grandfathered) — jangan turunkan harga yang sudah ada.

### P1 — Beri Pro pembeda nyata

Pindahkan **`api_access`** dan **`audit_logs`** dari Enterprise ke Pro. `managed_radius` **tetap Enterprise** — itu pembeda terkuat Anda (dan sudah di-enforce di 4 tempat).

### P2 — Bersihkan fitur kosong

| Fitur | Aksi |
|---|---|
| `sso_support` | Implementasi dulu, baru jual. Atau hapus dari matriks. |
| `remove_branding` | Hapus — tidak ada branding dinamis. |
| `max_members` | Gabung dengan `max_users` (duplikat, membingungkan). |
| `support_level` | Pindah ke metadata SLA/deskripsi, bukan fitur teknis. |
| `api_access` | Kalau dipertahankan, wajib ada rate limit per plan. |

### P3 — Jujur soal storage Enterprise

Pilih satu: naikkan ke 2 TB, atau ganti label "unlimited" jadi angka nyata. Rekomendasi: **500 GB → 1 TB** dan tetap tulis angkanya (jangan "unlimited") supaya klaim konsisten. Pantau `tenants.storage_usage` — saat ini tenant terbesar 38,9 MB, jadi masih sangat lega.

### P4 — Bangun yang menjual harga

Supaya Rp 990.000 bisa dibenarkan, prioritas fitur: **voucher**, **TR-069/GenieACS**, **reseller/outlet multi-tingkat** (ini yang MixRadius Evolution jual Rp 400.000), dan **billing otomatis**. Tanpa ini, harga Anda sulit dipertahankan di pasar ID.

### P5 — Buka self-serve

Tanpa registrasi tenant publik, Free/Pro mustahil jadi funnel. Ini masalah produk, bukan harga.

---

## 5. Jawaban singkat

- **Apakah harganya wajar?** Terhadap kompetitor global: murah (3–12×). Terhadap kompetitor Indonesia: mahal (1,7–5×). Terhadap omzet pelanggan: wajar (0,72% dari revenue, benchmark 1–3%). **Jangan turunkan harga** — yang kurang adalah pembeda, bukan harga.
- **Apakah pembatasannya sudah oke?** **Belum.** Unit limit salah (akun tim, bukan skala pelanggan); 5 dari 10 fitur tanpa efek; tidak ada batas pelanggan/router/OLT/concurrent; Enterprise tidak benar-benar unlimited.


---

## 6. Yang SUDAH diimplementasikan (2026-09-25)

### 6.1 Limit resource per-plan — P0

Sebelumnya hanya `max_users` + `max_storage_gb` yang membatasi, dan keduanya bukan satuan
skala ISP. Sekarang ada tiga fitur limit baru yang **benar-benar di-enforce**:

| Fitur | Free | Pro | Enterprise |
|---|---|---|---|
| `max_customers` | 25 | 300 | unlimited |
| `max_routers` | 1 | 10 | 25 |
| `max_olts` | 0 | 3 | unlimited |

Enterprise sengaja `unlimited` untuk pelanggan & OLT supaya tenant terbesar yang sudah ada
(548 pelanggan, 3 router, 3 OLT) tidak pernah terkunci oleh limitnya sendiri.

**Titik enforcement (3 jalur pembuatan resource + 1 jalur registrasi publik):**
- `customer_service/core.rs::create_customer_with_portal`
- `customer_service/core.rs::create_customer_from_public_registration` (registrasi mandiri)
- `mikrotik_service.rs::create_router`
- `olt_service/mod.rs::create_olt`

Implementasi: `services/resource_limit.rs` — modul baru berisi `effective_limit`,
`current_resource_count`, dan `enforce`. Dipilih berbasis `DbPool` (bukan menyuntikkan
`PlanService` ke CustomerService/MikrotikService/OltService) supaya blast radius kecil.
Nama tabel di-whitelist lewat `sql_ident::is_safe_sql_ident`.

### 6.2 api_access & audit_logs → Pro — P1

Pro sebelumnya hanya punya pembeda yang tidak di-enforce. Sekarang:

| Fitur | Free | Pro | Enterprise |
|---|---|---|---|
| `api_access` | false | **true** | true |
| `audit_logs` | false | **true** | true |
| `managed_radius` | false | false | true |

`audit_logs` sudah di-enforce di `audit_service.rs`; `managed_radius` tetap Enterprise-only
(pembeda terkuat, di-enforce di 4 tempat).

### 6.3 Batas 3 router Enterprise diperlebar → 25

Temuan: tenant sendiri punya tepat 3 router, jadi cap lama akan langsung menyakiti
pelanggan terbesar. Enterprise sekarang 25 router.

### 6.4 Yang BELUM dikerjakan

- **Growth tier Rp 450.000** (P0 lanjutan) — butuh keputusan harga, belum dieksekusi.
- Naikkan harga Pro/Enterprise — **tidak disarankan**; yang kurang pembeda, bukan harga.
- `sso_support` & `remove_branding` masih dijual tanpa implementasi (P2).
- Enterprise `max_storage_gb` masih 500 GB (P3) — saat ini pemakaian terbesar 38,9 MB.
- Concurrent session (`max_concurrent`) belum dibatasi, padahal
  `radius_accounting_sessions` sudah menyediakan datanya (P1 lanjutan).
- Voucher / TR-069 / reseller multi-tingkat (P4), self-serve signup (P5).
