# Network Asset Topology Layer Design

## Goal

Menampilkan asset FTTH terpilih di topology/network map dengan icon yang dibedakan per tipe, sambil menjaga peta tetap bersih dan mudah dibaca operator.

## Scope

Layer asset tahap pertama hanya menampilkan:

- `OLT`
- `ODC`
- `ODP`
- `FAT`
- `NAP`
- `switch`

Tidak termasuk:

- `ONT`
- `ONU`
- asset customer terminal lain

## Why This Scope

- kebutuhan operasional utama untuk peta FTTH adalah melihat distribusi backbone dan titik access/distribution
- data pelanggan dan subscription sudah punya konteks alamat sendiri
- memunculkan `ONT/ONU` terlalu dini akan membuat map cepat ramai
- `switch` masih relevan secara lapangan untuk node/box tertentu, tetapi tetap dibatasi ke layer asset terpilih ini

## Product Decisions

### 1. Asset markers use standalone asset coordinates

Marker asset di map dibaca dari `network_assets.latitude` dan `network_assets.longitude`.

Koordinat ini tidak bergantung pada lokasi customer, walaupun asset tetap bisa punya relasi customer/location untuk konteks bisnis.

### 2. Icons differ by asset type

Setiap tipe asset memiliki icon yang berbeda langsung:

- `OLT`
- `ODC`
- `ODP`
- `FAT`
- `NAP`
- `switch`

Alasan:

- operator bisa scan map lebih cepat
- perbedaan fungsi lapangan langsung terbaca
- jumlah tipe masih cukup sedikit sehingga tidak membuat visual terlalu bising

### 3. Color is secondary, not the primary differentiator

Perbedaan utama adalah shape/icon.

Warna hanya membantu:

- penguatan visual
- kemungkinan status/group nuance

tetapi warna tidak boleh menjadi satu-satunya pembeda tipe.

## UX Direction

### Asset layer behavior

- asset layer tampil di topology map sebagai marker point
- marker hanya dibuat jika asset punya koordinat valid
- marker asset tidak menimpa flow customer/pelanggan

### Popup content

Klik marker menampilkan ringkasan singkat:

- nama asset
- tipe asset
- status
- kode atau serial bila ada
- customer/location bila terkait

Popup tetap ringkas dan operasional.

### Legend

Tambahkan legend kecil untuk asset layer:

- `OLT`
- `ODC`
- `ODP`
- `FAT`
- `NAP`
- `switch`

Legend harus ringan dan tidak mendominasi map.

### Layer toggle

Asset layer perlu bisa dihidupkan/dimatikan.

Tujuan:

- operator bisa fokus ke link/node default bila perlu
- asset marker tidak selalu memenuhi map

## Technical Direction

### Data flow

Frontend map memuat asset registry terfilter dari API yang sudah memiliki:

- `asset_type`
- `name`
- `status`
- `code`
- `serial_number`
- `latitude`
- `longitude`
- relasi konteks lain bila tersedia

### Filtering before rendering

Sebelum marker dibangun:

1. filter asset type hanya yang ada di scope
2. filter koordinat valid
3. ubah ke format marker/layer internal map

### Icon mapping

Buat helper terpisah untuk:

- memetakan `asset_type -> icon token`
- memetakan `asset_type -> badge/legend label`
- memetakan `asset_type -> fallback accent`

Ini dipakai bersama oleh:

- topology map layer
- legend map
- kemungkinan chip/summary di registry asset

## Suggested Visual Language

- `OLT`: icon core/headend
- `ODC`: icon cabinet/backbone
- `ODP`: icon distribution box
- `FAT`: icon fiber access terminal
- `NAP`: icon access/drop point
- `switch`: icon network switch

Semua icon sebaiknya masih berada dalam satu keluarga visual agar konsisten.

## Testing Strategy

Minimal:

- helper test untuk filter asset map scope
- helper test untuk transform asset ke marker model
- helper test untuk icon mapping per tipe
- `npm run check`
- `cargo check`

## Out Of Scope

Tahap ini belum mencakup:

- visual asset customer terminal (`ONT/ONU`) di map
- port-per-port ODP occupancy di map
- auto-link asset layer ke line topology
- drag/edit marker langsung dari map
- infra layer terpisah lain di luar scope ini

## Recommended Implementation Order

1. tambahkan helper scope + icon mapping asset map
2. tambahkan transform asset -> marker model
3. ambil asset berkoordinat untuk halaman map
4. render asset layer point di topology map
5. tambahkan popup ringkas
6. tambahkan toggle + legend kecil
7. final polish pada halaman asset registry agar selaras
