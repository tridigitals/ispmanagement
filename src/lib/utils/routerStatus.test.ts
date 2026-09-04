import { describe, expect, it } from 'vitest';
import {
  POLL_INTERVAL_MS,
  STALE_AFTER_MS,
  humanAge,
  routerStatus,
  statusTone,
  summarize,
} from './routerStatus';

const NOW = new Date('2026-09-04T12:00:00Z').getTime();
const menitLalu = (n: number) => new Date(NOW - n * 60_000).toISOString();
const hariLalu = (n: number) => new Date(NOW - n * 86_400_000).toISOString();

describe('routerStatus', () => {
  it('router dinonaktifkan TIDAK PERNAH tampil online walau is_online masih true', () => {
    // Ini kasus nyata "Solikin": poller melewatkannya (WHERE enabled = true),
    // jadi is_online membeku pada true selamanya.
    const s = routerStatus(
      { enabled: false, is_online: true, latency_ms: 65, last_seen_at: hariLalu(28) },
      NOW,
    );

    expect(s.state).toBe('disabled');
    expect(s.label).toBe('Dinonaktifkan');
    expect(s.reason).toContain('28 hari');
    expect(s.metricsTrustworthy).toBe(false);
  });

  it('router aktif tanpa kabar > 3 siklus poll ditandai usang, bukan online', () => {
    const s = routerStatus(
      { enabled: true, is_online: true, last_seen_at: new Date(NOW - STALE_AFTER_MS - 1000).toISOString() },
      NOW,
    );

    expect(s.state).toBe('stale');
    expect(s.metricsTrustworthy).toBe(false);
    expect(s.reason).toContain('Poller berjalan tiap 5 menit');
  });

  it('satu siklus poll terlewat masih dianggap online', () => {
    const s = routerStatus(
      { enabled: true, is_online: true, last_seen_at: new Date(NOW - POLL_INTERVAL_MS - 1000).toISOString() },
      NOW,
    );

    expect(s.state).toBe('online');
    expect(s.metricsTrustworthy).toBe(true);
  });

  it('pemeliharaan menang atas online tapi tidak atas dinonaktifkan', () => {
    const maint = routerStatus(
      {
        enabled: true,
        is_online: true,
        last_seen_at: menitLalu(1),
        maintenance_until: new Date(NOW + 2 * 3_600_000).toISOString(),
        maintenance_reason: 'Ganti perangkat',
      },
      NOW,
    );
    expect(maint.state).toBe('maintenance');
    expect(maint.reason).toContain('Ganti perangkat');
    expect(maint.reason).toContain('2 jam');

    const disabledSaatMaint = routerStatus(
      {
        enabled: false,
        is_online: true,
        last_seen_at: menitLalu(1),
        maintenance_until: new Date(NOW + 2 * 3_600_000).toISOString(),
      },
      NOW,
    );
    expect(disabledSaatMaint.state).toBe('disabled');
  });

  it('jendela pemeliharaan yang sudah lewat tidak menahan status sebenarnya', () => {
    const s = routerStatus(
      {
        enabled: true,
        is_online: false,
        last_seen_at: menitLalu(2),
        maintenance_until: hariLalu(3),
      },
      NOW,
    );
    expect(s.state).toBe('offline');
  });

  it('offline memakai last_error sebagai alasan bila ada', () => {
    const s = routerStatus(
      { enabled: true, is_online: false, last_seen_at: menitLalu(4), last_error: 'connection refused' },
      NOW,
    );
    expect(s.state).toBe('offline');
    expect(s.reason).toBe('connection refused');
  });

  it('router yang belum pernah terhubung punya alasan eksplisit, bukan tanda hubung', () => {
    const s = routerStatus({ enabled: true, is_online: false, last_seen_at: null }, NOW);
    expect(s.ageMs).toBeNull();
    expect(s.reason).toBe('Belum pernah berhasil terhubung.');
  });

  it('setiap status selalu membawa alasan tidak kosong', () => {
    const kasus = [
      { enabled: false, is_online: true, last_seen_at: hariLalu(28) },
      { enabled: true, is_online: true, last_seen_at: menitLalu(1) },
      { enabled: true, is_online: false, last_seen_at: menitLalu(30) },
      { enabled: true, is_online: true, last_seen_at: hariLalu(2) },
      { enabled: true, is_online: true, last_seen_at: menitLalu(1), maintenance_until: new Date(NOW + 60_000).toISOString() },
    ];
    for (const k of kasus) {
      expect(routerStatus(k, NOW).reason.trim().length).toBeGreaterThan(0);
    }
  });
});

describe('summarize', () => {
  it('memisahkan dipantau dari dinonaktifkan (versi lama menggabungkannya)', () => {
    // Persis komposisi tenant ISP Management 2026-09-04.
    const rows = [
      { enabled: true, is_online: true, last_seen_at: menitLalu(1) }, // DEV Router
      { enabled: true, is_online: true, last_seen_at: menitLalu(1) }, // Xtrabit
      { enabled: false, is_online: true, last_seen_at: hariLalu(28) }, // Solikin
    ];
    const s = summarize(rows, NOW);

    expect(s.total).toBe(3);
    expect(s.monitored).toBe(2);
    expect(s.online).toBe(2); // versi lama melaporkan 3
    expect(s.disabled).toBe(1);
    expect(s.offline).toBe(0);
  });

  it('jumlah semua ember sama dengan total', () => {
    const rows = [
      { enabled: true, is_online: true, last_seen_at: menitLalu(1) },
      { enabled: true, is_online: false, last_seen_at: menitLalu(9) },
      { enabled: true, is_online: true, last_seen_at: hariLalu(5) },
      { enabled: false, is_online: true, last_seen_at: hariLalu(30) },
      { enabled: true, is_online: true, last_seen_at: menitLalu(1), maintenance_until: new Date(NOW + 3_600_000).toISOString() },
    ];
    const s = summarize(rows, NOW);
    expect(s.online + s.offline + s.stale + s.disabled + s.maintenance).toBe(s.total);
    expect(s.total).toBe(5);
  });

  it('daftar kosong tidak menghasilkan NaN', () => {
    const s = summarize([], NOW);
    expect(s).toEqual({
      total: 0,
      monitored: 0,
      online: 0,
      offline: 0,
      stale: 0,
      disabled: 0,
      maintenance: 0,
    });
  });
});

describe('humanAge', () => {
  it('memakai satuan yang wajar', () => {
    expect(humanAge(30_000)).toBe('baru saja');
    expect(humanAge(5 * 60_000)).toBe('5 menit');
    expect(humanAge(3 * 3_600_000)).toBe('3 jam');
    expect(humanAge(28 * 86_400_000)).toBe('28 hari');
  });
});

describe('statusTone', () => {
  it('usang memakai tone peringatan, bukan negatif — bedakan dari benar-benar mati', () => {
    expect(statusTone('stale')).toBe('warning');
    expect(statusTone('offline')).toBe('negative');
    expect(statusTone('online')).toBe('positive');
    expect(statusTone('disabled')).toBe('neutral');
  });
});
