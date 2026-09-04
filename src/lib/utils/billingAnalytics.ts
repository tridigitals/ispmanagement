/**
 * Helper untuk halaman /v2/admin/billing.
 *
 * KENAPA INI ADA
 *
 * Halaman lama `(app)/admin/billing/+page.svelte` (396 baris) menghitung
 * turunan langsung di markup, dan tiga di antaranya salah atau menyesatkan
 * pada data produksi:
 *
 *   1. `agingTotal` dijumlahkan di frontend dari empat bucket. Begitu server
 *      menambah bucket kelima (`not_due`, yang memang hilang sebelumnya),
 *      total di layar langsung lebih kecil dari isinya tanpa ada yang gagal.
 *      Server sekarang mengirim `aging_total`; helper ini memverifikasi
 *      keduanya cocok dan menandai selisihnya kalau tidak.
 *
 *   2. `maxTrendRevenue` memakai `Math.max(...revenue)` lalu setiap batang
 *      dibagi nilai itu. Kalau seluruh bulan nol — kondisi nyata untuk
 *      Juli–September 2026 — pembaginya jadi 1 dan setiap batang digambar
 *      dengan tinggi 0 tanpa keterangan apa pun.
 *
 *   3. Persentase ditampilkan tanpa basis sampel. `collection_rate` 0% yang
 *      lahir dari 2 invoice tidak bisa dibedakan dari 0% atas 500 invoice.
 *
 * Semua fungsi di sini murni supaya bisa diuji tanpa merender komponen.
 */

import type { BillingAnalytics } from '$lib/api/types';

/** Satu batang pada grafik tren. */
export interface TrendBar {
  month: string;
  /** Label ringkas "Sep", dipakai di sumbu. */
  label: string;
  revenue: number;
  /** 0–100. Nol untuk semua batang ketika tidak ada pemasukan sama sekali. */
  heightPct: number;
  /** true kalau bulan ini tidak punya pemasukan. */
  empty: boolean;
}

/** Satu baris pada laporan umur piutang. */
export interface AgingRow {
  key: 'not_due' | 'current' | 'days_31_60' | 'days_61_90' | 'over_90';
  label: string;
  amount: number;
  /** 0–100, porsi terhadap total piutang. */
  sharePct: number;
  /** Semakin tua semakin gawat; dipakai memilih warna. */
  severity: 'neutral' | 'info' | 'warning' | 'danger' | 'critical';
}

const MONTH_LABELS = [
  'Jan',
  'Feb',
  'Mar',
  'Apr',
  'Mei',
  'Jun',
  'Jul',
  'Agu',
  'Sep',
  'Okt',
  'Nov',
  'Des'
];

/** "2026-09" → "Sep". String yang tidak dikenal dikembalikan apa adanya. */
export function monthLabel(month: string): string {
  const parts = month.split('-');
  if (parts.length !== 2) return month;
  const index = Number.parseInt(parts[1], 10) - 1;
  return MONTH_LABELS[index] ?? month;
}

/**
 * Ubah `revenue_trend` menjadi batang siap render.
 *
 * Ketika seluruh bulan nol, `heightPct` semuanya 0 dan setiap batang ditandai
 * `empty` — halaman wajib menampilkan keterangan, bukan grafik kosong yang
 * ambigu. Versi lama memakai pembagi 1 sehingga hasilnya identik dengan
 * "ada data tapi sangat kecil".
 */
export function buildTrendBars(trend: BillingAnalytics['revenue_trend']): TrendBar[] {
  const rows = trend ?? [];
  const peak = rows.reduce((max, point) => Math.max(max, point.revenue), 0);
  return rows.map((point) => ({
    month: point.month,
    label: monthLabel(point.month),
    revenue: point.revenue,
    heightPct: peak > 0 ? Math.round((point.revenue / peak) * 1000) / 10 : 0,
    empty: point.revenue <= 0
  }));
}

/** true kalau tidak ada satu pun bulan dengan pemasukan. */
export function trendIsEmpty(bars: TrendBar[]): boolean {
  return bars.length > 0 && bars.every((bar) => bar.empty);
}

const AGING_LABELS: Record<AgingRow['key'], string> = {
  not_due: 'Belum jatuh tempo',
  current: '0-30 hari',
  days_31_60: '31-60 hari',
  days_61_90: '61-90 hari',
  over_90: 'Lebih dari 90 hari'
};

const AGING_SEVERITY: Record<AgingRow['key'], AgingRow['severity']> = {
  not_due: 'neutral',
  current: 'info',
  days_31_60: 'warning',
  days_61_90: 'danger',
  over_90: 'critical'
};

const AGING_ORDER: AgingRow['key'][] = [
  'not_due',
  'current',
  'days_31_60',
  'days_61_90',
  'over_90'
];

/**
 * Susun baris aging beserta porsinya.
 *
 * Pembagi memakai `aging_total` dari server, bukan hasil penjumlahan di sini,
 * supaya angka di kartu dan panjang bar berasal dari satu sumber.
 */
export function buildAgingRows(analytics: BillingAnalytics): AgingRow[] {
  const aging = analytics.aging;
  const total = analytics.aging_total > 0 ? analytics.aging_total : 0;
  return AGING_ORDER.map((key) => {
    const amount = aging?.[key] ?? 0;
    return {
      key,
      label: AGING_LABELS[key],
      amount,
      sharePct: total > 0 ? Math.round((amount / total) * 1000) / 10 : 0,
      severity: AGING_SEVERITY[key]
    };
  });
}

/**
 * Selisih antara total yang dikirim server dan penjumlahan bucket.
 *
 * Nol berarti konsisten. Nilai bukan nol berarti server menambah bucket yang
 * belum dikenal `AGING_ORDER` — persis mode kegagalan yang membuat versi lama
 * menampilkan total lebih kecil dari isinya.
 */
export function agingReconciliation(analytics: BillingAnalytics): {
  serverTotal: number;
  bucketSum: number;
  drift: number;
  consistent: boolean;
} {
  const bucketSum = buildAgingRows(analytics).reduce((sum, row) => sum + row.amount, 0);
  const serverTotal = analytics.aging_total ?? 0;
  const drift = Math.round((serverTotal - bucketSum) * 100) / 100;
  return { serverTotal, bucketSum, drift, consistent: Math.abs(drift) < 0.01 };
}

/**
 * Kalimat penjelas untuk collection rate.
 *
 * Persentase tanpa penyebut tidak bisa dinilai; 0% dari 2 invoice adalah
 * kabar yang sangat berbeda dari 0% atas 500 invoice.
 */
export function collectionCaption(analytics: BillingAnalytics): string {
  const sample = analytics.collection_sample;
  if (!sample || sample.invoices_considered <= 0) {
    return `Tidak ada tagihan pelanggan dalam ${sample?.window_days ?? 90} hari terakhir`;
  }
  return `${sample.paid_on_time} dari ${sample.invoices_considered} tagihan lunas tepat waktu (${sample.window_days} hari terakhir)`;
}

/** Kalimat penjelas untuk rata-rata hari pelunasan. */
export function avgDaysCaption(analytics: BillingAnalytics): string {
  if (!analytics.avg_days_sample || analytics.avg_days_sample <= 0) {
    return 'Belum ada tagihan lunas pada periode ini';
  }
  const plural = analytics.avg_days_sample === 1 ? 'tagihan' : 'tagihan';
  return `Rata-rata dari ${analytics.avg_days_sample} ${plural} lunas`;
}

/**
 * Alasan MRR bernilai nol, diambil dari rincian status langganan.
 *
 * Mengembalikan null kalau MRR wajar (ada langganan aktif), sehingga halaman
 * tidak memasang peringatan tanpa sebab.
 */
export function mrrExplanation(analytics: BillingAnalytics): string | null {
  if (analytics.active_subscriptions > 0) return null;
  const rows = analytics.subscription_breakdown ?? [];
  if (rows.length === 0) {
    return 'Belum ada langganan pelanggan yang tercatat';
  }
  const total = rows.reduce((sum, row) => sum + row.count, 0);
  const dominant = rows.reduce((top, row) => (row.count > top.count ? row : top), rows[0]);
  return `Tidak ada langganan berstatus aktif dari ${total} langganan; terbanyak ${subscriptionStatusLabel(dominant.status)} (${dominant.count})`;
}

const SUBSCRIPTION_STATUS_LABELS: Record<string, string> = {
  active: 'Aktif',
  grace_active: 'Masa tenggang',
  suspended: 'Ditangguhkan',
  pending_installation: 'Menunggu instalasi',
  installation_done_awaiting_payment: 'Terpasang, menunggu bayar',
  cancelled: 'Dibatalkan',
  terminated: 'Diakhiri'
};

/** Label Indonesia untuk status langganan; status tak dikenal tetap tampil. */
export function subscriptionStatusLabel(status: string): string {
  return SUBSCRIPTION_STATUS_LABELS[status] ?? status;
}

/**
 * Ringkas rincian status jadi teks pendek untuk subjudul kartu.
 * Contoh: "542 ditangguhkan, 7 menunggu instalasi, 2 dibatalkan".
 */
export function subscriptionSummary(analytics: BillingAnalytics, limit = 3): string {
  const rows = [...(analytics.subscription_breakdown ?? [])]
    .sort((a, b) => b.count - a.count)
    .slice(0, limit);
  if (rows.length === 0) return 'Belum ada langganan';
  return rows
    .map((row) => `${row.count} ${subscriptionStatusLabel(row.status).toLowerCase()}`)
    .join(', ');
}

/**
 * Apakah ada tagihan platform yang belum dibayar tenant ini.
 * Dipisah dari piutang pelanggan karena arah uangnya berlawanan.
 */
export function hasPlatformDues(analytics: BillingAnalytics): boolean {
  return (analytics.platform_dues?.outstanding_count ?? 0) > 0;
}
