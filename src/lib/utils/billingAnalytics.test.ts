import { describe, expect, it } from 'vitest';
import type { BillingAnalytics } from '$lib/api/types';
import {
  agingReconciliation,
  avgDaysCaption,
  buildAgingRows,
  buildTrendBars,
  collectionCaption,
  hasPlatformDues,
  monthLabel,
  mrrExplanation,
  subscriptionStatusLabel,
  subscriptionSummary,
  trendIsEmpty
} from './billingAnalytics';

/**
 * Fixture menyalin GET /api/payment/billing/analytics tenant f4ba7f24 pada
 * 2026-09-04, diverifikasi ulang lewat psql:
 *   piutang pelanggan  Rp 56.780.000 (476 tagihan, 474 di antaranya >90 hari)
 *   tagihan platform   Rp    990.000 (1 tagihan, arah uang berlawanan)
 *   langganan          542 suspended / 5 pending_installation / 2 cancelled
 *   pemasukan          hanya Apr (Rp 480rb) dan Jun (Rp 125rb) dari 6 bulan
 */
function analytics(overrides: Partial<BillingAnalytics> = {}): BillingAnalytics {
  return {
    mrr: 0,
    arr: 0,
    total_revenue: 0,
    collection_rate: 0,
    collection_sample: {
      invoices_considered: 2,
      paid_on_time: 0,
      paid_total: 0,
      window_days: 90
    },
    avg_days_to_pay: 0,
    avg_days_sample: 0,
    aging: {
      not_due: 0,
      current: 0,
      days_31_60: 5_000,
      days_61_90: 125_000,
      over_90: 56_650_000
    },
    aging_total: 56_780_000,
    churn_rate: 0,
    active_subscriptions: 0,
    total_customers: 545,
    subscription_breakdown: [
      { status: 'suspended', count: 542 },
      { status: 'pending_installation', count: 5 },
      { status: 'cancelled', count: 2 }
    ],
    revenue_trend: [
      { month: '2026-04', revenue: 480_000 },
      { month: '2026-05', revenue: 0 },
      { month: '2026-06', revenue: 125_000 },
      { month: '2026-07', revenue: 0 },
      { month: '2026-08', revenue: 0 },
      { month: '2026-09', revenue: 0 }
    ],
    platform_dues: { outstanding_amount: 990_000, outstanding_count: 1 },
    ...overrides
  };
}

describe('monthLabel', () => {
  it('memakai nama bulan Indonesia', () => {
    expect(monthLabel('2026-05')).toBe('Mei');
    expect(monthLabel('2026-08')).toBe('Agu');
    expect(monthLabel('2026-12')).toBe('Des');
  });

  it('mengembalikan input apa adanya bila formatnya tak dikenal', () => {
    expect(monthLabel('bukan-bulan-2')).toBe('bukan-bulan-2');
    expect(monthLabel('2026')).toBe('2026');
    expect(monthLabel('2026-13')).toBe('2026-13');
  });
});

describe('buildTrendBars', () => {
  it('mempertahankan seluruh 6 bulan termasuk yang nol', () => {
    const bars = buildTrendBars(analytics().revenue_trend);
    expect(bars).toHaveLength(6);
    expect(bars.map((b) => b.label)).toEqual(['Apr', 'Mei', 'Jun', 'Jul', 'Agu', 'Sep']);
    expect(bars.filter((b) => b.empty).map((b) => b.month)).toEqual([
      '2026-05',
      '2026-07',
      '2026-08',
      '2026-09'
    ]);
  });

  it('menskalakan tinggi terhadap bulan tertinggi', () => {
    const bars = buildTrendBars(analytics().revenue_trend);
    expect(bars[0].heightPct).toBe(100);
    expect(bars[2].heightPct).toBeCloseTo(26, 0);
    expect(bars[1].heightPct).toBe(0);
  });

  it('tidak memakai pembagi 1 palsu ketika semua bulan nol', () => {
    // Versi lama: Math.max(...revenue, 1) -> pembagi 1 -> semua batang 0%
    // tanpa cara membedakannya dari "ada data sangat kecil".
    const bars = buildTrendBars([
      { month: '2026-07', revenue: 0 },
      { month: '2026-08', revenue: 0 },
      { month: '2026-09', revenue: 0 }
    ]);
    expect(bars.every((bar) => bar.heightPct === 0)).toBe(true);
    expect(trendIsEmpty(bars)).toBe(true);
  });

  it('tren dengan pemasukan tidak dianggap kosong', () => {
    expect(trendIsEmpty(buildTrendBars(analytics().revenue_trend))).toBe(false);
  });

  it('tahan terhadap tren kosong', () => {
    expect(buildTrendBars([])).toEqual([]);
    expect(trendIsEmpty([])).toBe(false);
  });
});

describe('buildAgingRows', () => {
  it('menampilkan lima bucket termasuk belum jatuh tempo', () => {
    const rows = buildAgingRows(analytics());
    expect(rows.map((r) => r.key)).toEqual([
      'not_due',
      'current',
      'days_31_60',
      'days_61_90',
      'over_90'
    ]);
    expect(rows[0].label).toBe('Belum jatuh tempo');
  });

  it('memakai angka nyata: >90 hari menguasai 99,8% piutang', () => {
    const rows = buildAgingRows(analytics());
    expect(rows.find((r) => r.key === 'over_90')?.sharePct).toBeCloseTo(99.8, 1);
    expect(rows.find((r) => r.key === 'days_61_90')?.sharePct).toBeCloseTo(0.2, 1);
  });

  it('menghitung porsi dari total server, bukan penjumlahan sendiri', () => {
    const rows = buildAgingRows(
      analytics({
        aging: {
          not_due: 250_000,
          current: 250_000,
          days_31_60: 0,
          days_61_90: 0,
          over_90: 500_000
        },
        aging_total: 1_000_000
      })
    );
    expect(rows.find((r) => r.key === 'not_due')?.sharePct).toBe(25);
    expect(rows.find((r) => r.key === 'over_90')?.sharePct).toBe(50);
  });

  it('tidak membagi nol ketika tak ada piutang', () => {
    const rows = buildAgingRows(
      analytics({
        aging: { not_due: 0, current: 0, days_31_60: 0, days_61_90: 0, over_90: 0 },
        aging_total: 0
      })
    );
    expect(rows.every((row) => row.sharePct === 0)).toBe(true);
    expect(rows.every((row) => Number.isFinite(row.sharePct))).toBe(true);
  });

  it('memberi tingkat kegawatan meningkat sesuai umur', () => {
    const rows = buildAgingRows(analytics());
    expect(rows.map((r) => r.severity)).toEqual([
      'neutral',
      'info',
      'warning',
      'danger',
      'critical'
    ]);
  });
});

describe('agingReconciliation', () => {
  it('menyatakan konsisten ketika total server sama dengan isi bucket', () => {
    const result = agingReconciliation(analytics());
    expect(result.bucketSum).toBe(56_780_000);
    expect(result.serverTotal).toBe(56_780_000);
    expect(result.drift).toBe(0);
    expect(result.consistent).toBe(true);
  });

  it('menandai selisih ketika server punya bucket yang tak dikenal klien', () => {
    // Inilah mode kegagalan versi lama: bucket kelima ditambahkan di server
    // sementara klien menjumlahkan empat, dan tidak ada yang gagal.
    const result = agingReconciliation(
      analytics({
        aging: {
          not_due: 0,
          current: 0,
          days_31_60: 5_000,
          days_61_90: 125_000,
          over_90: 56_650_000
        },
        // Server menghitung satu invoice verification_pending Rp 165.000 yang
        // belum punya bucket di klien.
        aging_total: 56_945_000
      })
    );
    expect(result.drift).toBe(165_000);
    expect(result.consistent).toBe(false);
  });
});

describe('collectionCaption', () => {
  it('menyebut pembilang dan penyebut', () => {
    expect(collectionCaption(analytics())).toBe(
      '0 dari 2 tagihan lunas tepat waktu (90 hari terakhir)'
    );
  });

  it('menyatakan tidak ada data saat sampel kosong', () => {
    const caption = collectionCaption(
      analytics({
        collection_sample: {
          invoices_considered: 0,
          paid_on_time: 0,
          paid_total: 0,
          window_days: 90
        }
      })
    );
    expect(caption).toBe('Tidak ada tagihan pelanggan dalam 90 hari terakhir');
  });
});

describe('avgDaysCaption', () => {
  it('membedakan nol karena tidak ada data', () => {
    expect(avgDaysCaption(analytics())).toBe('Belum ada tagihan lunas pada periode ini');
  });

  it('menyebut jumlah sampel ketika ada pelunasan', () => {
    expect(avgDaysCaption(analytics({ avg_days_sample: 4, avg_days_to_pay: 12.5 }))).toBe(
      'Rata-rata dari 4 tagihan lunas'
    );
  });
});

describe('mrrExplanation', () => {
  it('menjelaskan MRR nol lewat status dominan', () => {
    expect(mrrExplanation(analytics())).toBe(
      'Tidak ada langganan berstatus aktif dari 549 langganan; terbanyak Ditangguhkan (542)'
    );
  });

  it('diam ketika ada langganan aktif', () => {
    expect(mrrExplanation(analytics({ active_subscriptions: 12, mrr: 1_500_000 }))).toBeNull();
  });

  it('menangani tenant tanpa langganan sama sekali', () => {
    expect(mrrExplanation(analytics({ subscription_breakdown: [] }))).toBe(
      'Belum ada langganan pelanggan yang tercatat'
    );
  });
});

describe('subscriptionStatusLabel', () => {
  it('menerjemahkan status yang dikenal', () => {
    expect(subscriptionStatusLabel('suspended')).toBe('Ditangguhkan');
    expect(subscriptionStatusLabel('grace_active')).toBe('Masa tenggang');
    expect(subscriptionStatusLabel('pending_installation')).toBe('Menunggu instalasi');
  });

  it('menampilkan status asing apa adanya, bukan menyembunyikannya', () => {
    expect(subscriptionStatusLabel('status_baru_dari_migrasi')).toBe('status_baru_dari_migrasi');
  });
});

describe('subscriptionSummary', () => {
  it('meringkas tiga status terbanyak', () => {
    expect(subscriptionSummary(analytics())).toBe(
      '542 ditangguhkan, 5 menunggu instalasi, 2 dibatalkan'
    );
  });

  it('menghormati batas jumlah', () => {
    expect(subscriptionSummary(analytics(), 1)).toBe('542 ditangguhkan');
  });

  it('memberi teks jelas saat kosong', () => {
    expect(subscriptionSummary(analytics({ subscription_breakdown: [] }))).toBe(
      'Belum ada langganan'
    );
  });
});

describe('hasPlatformDues', () => {
  it('true ketika tenant punya tagihan platform belum lunas', () => {
    expect(hasPlatformDues(analytics())).toBe(true);
  });

  it('false ketika tidak ada', () => {
    expect(
      hasPlatformDues(analytics({ platform_dues: { outstanding_amount: 0, outstanding_count: 0 } }))
    ).toBe(false);
  });
});
