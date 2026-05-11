# Superadmin Managed RADIUS Filter Toolbar Design

**Context**

Halaman [src/routes/superadmin/radius/+page.svelte](/home/xtrabit/ISPMANAGEMENT/src/routes/superadmin/radius/+page.svelte) sudah kaya fungsi, tetapi area filter per tab masih memakai input mentah yang selalu terbuka. Saat jumlah filter bertambah, tampilan cepat terasa padat dan sulit dipindai. Halaman `Users` dan `Audit Logs` sudah lebih terarah dengan toolbar, jadi `Managed RADIUS` perlu dinaikkan ke pola yang lebih ringkas dan konsisten.

**Goal**

Merapikan pengalaman filter di halaman `Managed RADIUS` dengan pola:
- `Search` selalu terlihat.
- `Tenant` menjadi filter utama yang tetap terlihat pada tab yang relevan.
- Filter lanjutan dibuka lewat tombol `Filter`.
- Jumlah filter aktif terlihat langsung pada tombol.
- Tombol `Reset` tersedia di panel lanjutan, tidak dominan.

**Design**

Toolbar per tab dibagi menjadi tiga lapis:
- Baris utama: judul ringkas tab, jumlah item hasil, search, primary select, tombol `Filter`, dan tombol aksi tab.
- Panel lanjutan: muncul inline di bawah toolbar saat dibuka, berisi filter sekunder sesuai tab aktif.
- Tabel/empty state: tetap di bawah panel tanpa perubahan perilaku data.

Filter per tab:
- `Assignments`: visible `search + tenant`, advanced `status`
- `Mappings`: visible `search + tenant`, advanced `server + status`
- `Users`: visible `search + tenant`, advanced `router + provision status`
- `Sessions`: visible `search + tenant`, advanced `router`

**UX Rules**

- Search dan select memakai tinggi dan radius yang sama agar terasa satu keluarga.
- Panel filter lanjutan inline, bukan modal atau drawer, supaya konteks tabel tetap terlihat.
- Badge `Filter (n)` hanya menghitung filter lanjutan yang aktif.
- `Reset` mereset seluruh filter tab aktif, termasuk search dan tenant.
- Di mobile, toolbar menumpuk vertikal tanpa menyembunyikan fungsi utama.

**Implementation Notes**

- Buat komponen toolbar lokal yang reusable untuk halaman `Managed RADIUS`, bukan refactor global dulu.
- Simpan state buka/tutup panel filter per tab supaya perilakunya stabil saat pindah tab.
- Tambahkan helper hitung filter aktif dan reset filter per tab di page component.
- Pertahankan seluruh logika filter data yang sudah ada; perubahan fokus pada presentasi UI dan wiring filter.

**Testing**

- `npm run check` harus tetap hijau.
- Uji manual tiap tab:
  - search tetap memfilter hasil
  - primary filter tetap bekerja
  - panel filter lanjutan bisa dibuka/tutup
  - badge jumlah filter aktif berubah sesuai input
  - reset mengembalikan seluruh filter ke default
