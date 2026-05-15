# Network Assets Modal And Map Picker Design

## Goal

Memperbaiki pengalaman halaman `/admin/network/assets` agar:

- modal asset terasa lebih rapi dan lebih operasional untuk tim ISP
- asset memiliki koordinat peta yang berdiri sendiri
- koordinat asset bisa dipakai untuk topology map/network map
- UI tetap tenang, tidak terlalu ramai

## Current State

- Modal asset saat ini masih berupa form linear biasa.
- Belum ada section khusus untuk koordinat asset.
- Asset masih bergantung pada relasi customer/location untuk konteks lokasi operasional.
- Repo sudah memiliki pola `MapCanvasShell` yang dipakai pada router form dan order/customer location flow.

## Product Decision

### 1. Asset coordinates are standalone

Setiap asset menyimpan koordinatnya sendiri.

Alasan:

- posisi fisik asset backbone seperti `ODP`, `ODC`, `FAT`, `switch`, `router`, `ODF`, `UPS` sering tidak sama dengan titik customer
- topology map perlu koordinat asset yang akurat
- satu customer location tidak boleh “menarik” semua asset ke satu titik

### 2. Map picker is available for all asset types

Map picker tersedia untuk semua asset type, tetapi prioritas visualnya berbeda:

- asset lapangan/infrastruktur: section lokasi peta tampil lebih penting
- `ONT/ONU`: tetap tersedia, namun bersifat opsional dan lebih ringan

### 3. Customer/location link remains separate

Relasi `customer`, `location`, `work_order`, dan `parent_asset` tetap dipertahankan sebagai konteks bisnis.

Koordinat asset tidak otomatis bergantung pada customer location, tetapi user bisa memakai shortcut `Use Customer Location` jika customer location punya latitude/longitude.

## UX Direction

### Modal layout

Modal asset diubah menjadi layout dua kolom:

- kolom kiri: form utama
- kolom kanan: section lokasi peta

Tujuan:

- data identitas dan operasional tetap terbaca
- map picker selalu terlihat tanpa membuat modal terasa penuh

### Form sections

Kolom kiri dibagi menjadi section yang jelas:

1. `Asset Identity`
   - type
   - status
   - name
   - code
   - serial number
   - vendor
   - model

2. `Asset Detail & Capacity`
   - dynamic detail fields per asset type
   - termasuk `total_port_capacity` untuk `ODP`
   - jika tersedia, tampilkan occupancy preview ringan

3. `Operational Links`
   - customer
   - location
   - work order
   - parent asset

4. `Notes`

### Map section

Kolom kanan berisi section `Lokasi Peta Asset`:

- embedded map preview / picker
- field latitude
- field longitude
- tombol `Pick on Map`
- tombol `Use Customer Location`
- tombol `Clear Point`

Section ini menjelaskan bahwa koordinat adalah posisi asset fisik yang akan dipakai di topology map.

### Visual style

Gaya yang diinginkan:

- lebih rapi, lebih padat makna
- tetap ringan, tidak “rame”
- hierarchy jelas dengan card section lembut
- warna netral hangat, mengikuti arah mockup
- map section terlihat penting tapi tidak mendominasi

## Data Model Direction

### Storage

Asset perlu menyimpan koordinat sendiri.

Recommended shape:

- tambah `latitude`
- tambah `longitude`

di model `network_assets`, bukan ditaruh di `metadata`.

Alasan:

- koordinat adalah properti inti asset, bukan detail opsional per tipe
- lebih mudah dipakai untuk query, filtering, dan integrasi topology map
- lebih konsisten dengan pola data router/location yang sudah ada

### Shortcut behavior

Jika customer location terpilih dan punya koordinat:

- tombol `Use Customer Location` meng-copy nilai latitude/longitude ke asset draft
- copy ini hanya aksi helper, bukan binding permanen

## Map Picker Behavior

### Open/interaction model

Ada dua opsi implementasi yang tetap kompatibel dengan design:

1. embedded compact map di modal
2. tombol yang membuka picker/expanded map state

Untuk tahap pertama, lebih aman memakai embedded compact map dengan interaksi sederhana:

- klik pada map untuk set point
- kalau sudah ada koordinat, map fokus ke titik itu
- kalau belum ada koordinat tapi customer location tersedia, map fokus ke customer location
- fallback terakhir ke default viewport umum

### Validation

- latitude harus di antara `-90` dan `90`
- longitude harus di antara `-180` dan `180`
- keduanya boleh kosong bersama-sama
- tidak boleh terisi salah satu saja

## Integration Targets

### Page polish

Halaman `/admin/network/assets` perlu dipoles di area:

- modal form
- ringkasan asset row bila koordinat tersedia
- kemungkinan chip kecil koordinat pada table/detail

### Future topology support

Tahap ini belum harus langsung menghubungkan asset ke topology map, tetapi data dan UI harus disiapkan agar tahap berikutnya mudah:

- asset coordinates tersedia di API response
- koordinat mudah dibaca oleh page atau map layer lain

## Testing Strategy

Minimal tests:

- helper validation untuk koordinat asset
- state helper untuk copy dari customer location ke asset coordinate draft
- frontend typecheck untuk modal baru
- regression check pada network assets page

## Out Of Scope

Tahap ini belum mencakup:

- mapping port-per-port ODP ke pelanggan
- visual layer topology map asset penuh
- auto-routing atau nearest-asset suggestion
- sinkronisasi dua arah antara asset coordinate dan customer location

## Recommended Implementation Order

1. tambahkan field koordinat asset ke backend model/API
2. tambahkan state/form support di page asset
3. polish modal menjadi sectioned two-column layout
4. integrasikan map picker reuse dari `MapCanvasShell`
5. tambahkan shortcut `Use Customer Location`
6. tampilkan koordinat secara ringan di registry asset
7. verifikasi test, `svelte-check`, dan `cargo check`
