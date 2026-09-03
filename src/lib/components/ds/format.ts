/**
 * Format angka & tanggal terpusat.
 *
 * Sebelumnya tiap halaman memformat rupiah dengan caranya sendiri, sehingga
 * ada campuran "Rp 150.000", "150000", dan "Rp150.000,00" di layar yang sama.
 */

const rupiah = new Intl.NumberFormat('id-ID', {
  style: 'currency',
  currency: 'IDR',
  minimumFractionDigits: 0,
  maximumFractionDigits: 0,
});

/**
 * Contoh: 56485000 -> "Rp56.485.000".
 *
 * Intl id-ID menyisipkan U+00A0 (spasi tak-putus) setelah "Rp", sementara
 * formatCompactRupiah menulis "Rp56,8jt" tanpa spasi. Di satu layar hasilnya
 * bercampur "Rp 125.000" dan "Rp56,8jt" — audit render menemukan 10 kejadian
 * bergaya spasi dan 3 tanpa spasi di dashboard yang sama. Spasi dibuang di
 * sini supaya kedua fungsi seragam.
 */
export function formatRupiah(value: number | string | null | undefined): string {
  const n = typeof value === 'string' ? Number(value) : value;
  if (n == null || Number.isNaN(n)) return '—';
  return rupiah.format(n).replace(/\u00a0/g, '');
}

/** Untuk ruang sempit: 56485000 -> "Rp56,5jt". */
export function formatCompactRupiah(value: number | null | undefined): string {
  if (value == null || Number.isNaN(value)) return '—';
  const abs = Math.abs(value);
  if (abs >= 1_000_000_000) return `Rp${(value / 1_000_000_000).toFixed(1).replace('.', ',')}M`;
  if (abs >= 1_000_000) return `Rp${(value / 1_000_000).toFixed(1).replace('.', ',')}jt`;
  if (abs >= 1_000) return `Rp${Math.round(value / 1_000)}rb`;
  return formatRupiah(value);
}

const dateFmt = new Intl.DateTimeFormat('id-ID', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
});

/** Persen gaya Indonesia: 2.6 -> "2,6%". */
export function formatPercent(value: number | null | undefined, digits = 1): string {
  if (value == null || Number.isNaN(value)) return '—';
  return `${value.toFixed(digits).replace('.', ',')}%`;
}

const dateTimeFmt = new Intl.DateTimeFormat('id-ID', {
  day: '2-digit',
  month: 'short',
  hour: '2-digit',
  minute: '2-digit',
});

/** withTime=true dipakai untuk penanda "diperbarui pukul ...". */
export function formatDate(
  value: string | Date | null | undefined,
  withTime = false,
): string {
  if (!value) return '—';
  const d = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(d.getTime())) return '—';
  return withTime ? dateTimeFmt.format(d) : dateFmt.format(d);
}

/** Selisih waktu ringkas: "3 hari lagi", "2 jam lalu". */
export function formatRelative(value: string | Date | null | undefined): string {
  if (!value) return '—';
  const d = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(d.getTime())) return '—';

  const diffMs = d.getTime() - Date.now();
  const abs = Math.abs(diffMs);
  const suffix = diffMs >= 0 ? 'lagi' : 'lalu';

  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;

  if (abs < minute) return 'baru saja';
  if (abs < hour) return `${Math.round(abs / minute)} mnt ${suffix}`;
  if (abs < day) return `${Math.round(abs / hour)} jam ${suffix}`;
  return `${Math.round(abs / day)} hari ${suffix}`;
}
