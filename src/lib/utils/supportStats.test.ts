import { describe, expect, it } from 'vitest';
import {
  ACTIVE_STATUSES,
  STALE_TICKET_MS,
  bucketByAge,
  buildStatCards,
  isStale,
  shouldShowPending,
  unaccounted,
  waitingLabel,
} from './supportStats';

const NOW = new Date('2026-09-04T12:00:00Z').getTime();
const hariLalu = (n: number) => new Date(NOW - n * 86_400_000).toISOString();
const jamLalu = (n: number) => new Date(NOW - n * 3_600_000).toISOString();

/* Angka produksi tenant "ISP Management" 2026-09-04 setelah backend diperbaiki. */
const PROD = { all: 20, open: 18, pending: 0, closed: 1, resolved: 1, unassigned: 7 };

describe('unaccounted', () => {
  it('mendeteksi tiket yang hilang dari semua ember', () => {
    // Bentuk stats LAMA: resolved tidak ada, jadi 1 tiket menguap.
    const lama = { all: 20, open: 18, pending: 0, closed: 1 };
    expect(unaccounted(lama)).toBe(1);
  });

  it('nol setelah resolved ikut dihitung', () => {
    expect(unaccounted(PROD)).toBe(0);
  });

  it('tidak pernah negatif walau data tidak konsisten', () => {
    expect(unaccounted({ all: 2, open: 5, pending: 0, closed: 0, resolved: 0 })).toBe(0);
  });

  it('menangani field yang hilang atau null tanpa NaN', () => {
    expect(unaccounted({ all: 5 })).toBe(5);
    expect(unaccounted({ all: null, open: null })).toBe(0);
    expect(unaccounted({})).toBe(0);
  });
});

describe('buildStatCards', () => {
  it('kartu "Belum ditugaskan" membawa angka nyata, bukan em-dash', () => {
    const kartu = buildStatCards(PROD);
    const un = kartu.find((c) => c.key === 'unassigned');

    expect(un?.value).toBe(7);
    expect(un?.hint).toBe('dari 18 tiket aktif');
    expect(un?.tone).toBe('negative'); // ada pekerjaan tanpa penerima tugas
    expect(un?.filter).toBe('unassigned');
  });

  it('setiap kartu punya hint tidak kosong', () => {
    for (const c of buildStatCards(PROD)) {
      expect(c.hint.trim().length).toBeGreaterThan(0);
    }
  });

  it('kartu resolved muncul dan bisa difilter', () => {
    const kartu = buildStatCards(PROD);
    const res = kartu.find((c) => c.key === 'resolved');
    expect(res?.value).toBe(1);
    expect(res?.filter).toBe('resolved');
  });

  it('belum ditugaskan nol berarti tone positif', () => {
    const kartu = buildStatCards({ ...PROD, unassigned: 0 });
    expect(kartu.find((c) => c.key === 'unassigned')?.tone).toBe('positive');
  });

  it('kartu total menjelaskan komposisi aktif vs selesai', () => {
    const total = buildStatCards(PROD).find((c) => c.key === 'all');
    expect(total?.value).toBe(20);
    expect(total?.hint).toBe('18 masih aktif · 2 selesai');
  });

  it('daftar kosong tidak menghasilkan NaN atau persen aneh', () => {
    const kartu = buildStatCards({});
    for (const c of kartu) {
      expect(Number.isFinite(c.value)).toBe(true);
      expect(c.hint).not.toContain('NaN');
    }
    expect(kartu.find((c) => c.key === 'open')?.hint).toBe('belum ada tiket');
  });

  it('persen dihitung terhadap total, bukan terhadap tiket aktif', () => {
    const kartu = buildStatCards({ all: 20, open: 18, pending: 0, closed: 1, resolved: 1 });
    expect(kartu.find((c) => c.key === 'open')?.hint).toBe('90% dari seluruh tiket');
  });
});

describe('shouldShowPending', () => {
  it('menyembunyikan ember pending yang tidak pernah diisi backend', () => {
    expect(shouldShowPending(PROD)).toBe(false);
  });

  it('menampilkannya kalau memang ada isinya', () => {
    expect(shouldShowPending({ ...PROD, pending: 3 })).toBe(true);
  });
});

describe('bucketByAge', () => {
  it('hanya menghitung tiket aktif', () => {
    const t = [
      { status: 'open', created_at: hariLalu(1) },
      { status: 'closed', created_at: hariLalu(200) },
      { status: 'resolved', created_at: hariLalu(200) },
    ];
    const b = bucketByAge(t, NOW);
    expect(b.reduce((s, x) => s + x.count, 0)).toBe(1);
  });

  it('memisahkan tiket terlantar dari yang baru', () => {
    const t = [
      { status: 'open', created_at: jamLalu(5) },
      { status: 'open', created_at: hariLalu(4) },
      { status: 'open', created_at: hariLalu(20) },
      { status: 'open', created_at: hariLalu(195) }, // tiket tertua di produksi
    ];
    const b = bucketByAge(t, NOW);
    expect(b.map((x) => x.count)).toEqual([1, 1, 1, 1]);
    expect(b[3].label).toBe('> 30 hari');
  });

  it('mengabaikan created_at yang tidak valid tanpa melempar', () => {
    const b = bucketByAge([{ status: 'open', created_at: null }, { status: 'open' }], NOW);
    expect(b.reduce((s, x) => s + x.count, 0)).toBe(0);
  });
});

describe('waitingLabel', () => {
  it('memakai satuan yang wajar', () => {
    expect(waitingLabel(jamLalu(2), NOW)).toBe('2 jam');
    expect(waitingLabel(hariLalu(3), NOW)).toBe('3 hari');
    expect(waitingLabel(hariLalu(195), NOW)).toBe('6 bulan');
    expect(waitingLabel(null, NOW)).toBe('—');
  });

  it('waktu di masa depan tidak jadi angka negatif', () => {
    expect(waitingLabel(new Date(NOW + 86_400_000).toISOString(), NOW)).toBe('—');
  });
});

describe('isStale', () => {
  it('tiket aktif lebih dari 30 hari disebut terlantar', () => {
    expect(isStale({ status: 'open', created_at: hariLalu(31) }, NOW)).toBe(true);
    expect(isStale({ status: 'open', created_at: hariLalu(29) }, NOW)).toBe(false);
  });

  it('tiket selesai tidak pernah terlantar walau tua', () => {
    expect(isStale({ status: 'closed', created_at: hariLalu(500) }, NOW)).toBe(false);
    expect(isStale({ status: 'resolved', created_at: hariLalu(500) }, NOW)).toBe(false);
  });

  it('ambang sesuai konstanta yang diekspor', () => {
    expect(STALE_TICKET_MS).toBe(30 * 86_400_000);
    expect(ACTIVE_STATUSES).toEqual(['open', 'pending']);
  });
});
