/**
 * Kelas utilitas bersama untuk permukaan detail instalasi.
 *
 * Kenapa ada: permukaan detail instalasi dilayani sebagai satu komponen
 * (`InstallationQuickAssetPanel.svelte`) dari DUA halaman — halaman lama
 * `(app)` yang memakai kelas gaya kustom, dan halaman v2 yang memakai Tailwind.
 * Kalau gaya diambil dari satu sisi saja, sisi lain akan tampak salah.
 *
 * Semua kelas di sini adalah kelas Tailwind yang SUDAH dipakai di halaman v2,
 * jadi halaman v2 tidak berubah tampilannya sama sekali. Halaman lama
 * memuat stylesheet Tailwind juga, sehingga kelas yang sama tetap bekerja.
 */
export const IQA = {
  /** Baris tombol "Buat aset FTTH" saat form tertutup. */
  triggerRow: 'mt-2',
  /** Teks bantuan kecil di bawah tombol. */
  hint: 'mt-1 text-xs text-ink-500',

  /** Kartu form. */
  card: 'mt-3 rounded-lg border border-ink-200 p-3',
  cardTitle: 'mb-2 font-medium text-ink-900',
  cardNote: 'mb-2 text-xs text-ink-500',

  /** Grid dua kolom untuk field. */
  grid: 'grid grid-cols-2 gap-2',

  error: 'mt-1 text-xs text-rose-600',
  actions: 'mt-3 flex gap-2',
} as const;
