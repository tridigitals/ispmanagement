/**
 * Ambil seluruh halaman dari endpoint ber-paginate.
 *
 * LATAR BELAKANG (ini bug nyata, bukan optimasi prematur):
 * Dashboard admin lama memanggil `listCustomerPackageInvoices({ sort_by, sort_dir })`
 * tanpa `per_page`, jadi backend memakai default 25 (commands/payment.rs), lalu
 * `summarizeInvoices()` menjumlahkan 25 baris itu dan menampilkannya sebagai
 * total tenant. Untuk tenant isp-management yang punya 489 invoice, angka
 * "tertunggak" di dashboard hanya mewakili 25 baris pertama.
 *
 * Menaikkan `per_page` saja tidak cukup: payment_service melakukan
 * `per_page.clamp(1, 100)`, jadi permintaan 1000 tetap dipotong jadi 100.
 *
 * Helper ini membaca `total` dari respons halaman pertama lalu mengambil sisa
 * halaman sampai lengkap, sehingga agregat dihitung atas seluruh baris.
 *
 * CATATAN: ini solusi sisi klien. Solusi yang benar jangka panjang adalah
 * endpoint agregat di backend (SUM/COUNT di SQL) supaya cukup satu permintaan.
 */

export interface Paginated<T> {
  data: T[];
  total: number;
  page: number;
  per_page: number;
}

export interface FetchAllOptions {
  /** Ukuran halaman yang diminta. Backend boleh memotongnya. */
  perPage?: number;
  /** Batas keras jumlah permintaan supaya tidak membanjiri backend. */
  maxPages?: number;
}

export async function fetchAllPages<T>(
  fetchPage: (page: number, perPage: number) => Promise<Paginated<T>>,
  options: FetchAllOptions = {},
): Promise<{ rows: T[]; total: number; complete: boolean }> {
  const requested = options.perPage ?? 100;
  const maxPages = options.maxPages ?? 25;

  const first = await fetchPage(1, requested);
  const rows = [...(first.data ?? [])];
  const total = first.total ?? rows.length;

  /* Pakai per_page yang BENAR-BENAR dipakai backend, bukan yang kita minta —
     inilah sumber kesalahan hitung kalau diabaikan (minta 1000, dapat 100). */
  const effective = first.per_page || rows.length || requested;
  if (effective <= 0) return { rows, total, complete: rows.length >= total };

  const pages = Math.ceil(total / effective);
  const limit = Math.min(pages, maxPages);

  for (let p = 2; p <= limit; p++) {
    const res = await fetchPage(p, requested);
    const batch = res.data ?? [];
    if (batch.length === 0) break;
    rows.push(...batch);
  }

  return { rows, total, complete: rows.length >= total };
}
