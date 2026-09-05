import { describe, expect, it } from 'vitest';
import {
  filterInterfaceRows,
  formatBps,
  formatBytes,
  formatUptime,
  friendlyRouterError,
  interfaceRows,
  interfaceStatus,
  pctUsed,
  snapshotHealthStats,
} from './routerDetailInsights';

describe('formatBps', () => {
  it('memformat satuan desimal (1000)', () => {
    expect(formatBps(0)).toBe('0 bps');
    expect(formatBps(500)).toBe('500 bps');
    expect(formatBps(1500)).toBe('1.5 Kbps');
    expect(formatBps(2_500_000)).toBe('2.5 Mbps');
    expect(formatBps(3_000_000_000)).toBe('3.0 Gbps');
  });
  it('membulatkan >= 10 tanpa desimal', () => {
    expect(formatBps(12_000)).toBe('12 Kbps');
    expect(formatBps(15_500_000)).toBe('16 Mbps');
  });
  it('menangani negatif dan null', () => {
    expect(formatBps(-1_500_000)).toBe('-1.5 Mbps');
    expect(formatBps(null)).toBe('—');
    expect(formatBps(undefined)).toBe('—');
  });
});

describe('formatBytes', () => {
  it('memakai basis 1024', () => {
    expect(formatBytes(1024)).toBe('1.0 KB');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(1048576)).toBe('1.0 MB');
    expect(formatBytes(1073741824)).toBe('1.0 GB');
  });
  it('null -> dash', () => {
    expect(formatBytes(null)).toBe('—');
  });
});

describe('formatUptime', () => {
  it('menyusun d/h/m', () => {
    expect(formatUptime(0)).toBe('0m');
    expect(formatUptime(90)).toBe('1m');
    expect(formatUptime(7200)).toBe('2h 0m');
    expect(formatUptime(90061)).toBe('1d 1h');
  });
  it('null -> dash dan menolak negatif', () => {
    expect(formatUptime(null)).toBe('—');
    expect(formatUptime(-5)).toBe('0m');
  });
});

describe('pctUsed', () => {
  it('menghitung persen terpakai', () => {
    expect(pctUsed(100, 25)).toBe(75);
    expect(pctUsed(100, 0)).toBe(100);
    expect(pctUsed(100, 100)).toBe(0);
  });
  it('null bila total tidak valid', () => {
    expect(pctUsed(0, 0)).toBe(null);
    expect(pctUsed(null, 10)).toBe(null);
    expect(pctUsed(10, null)).toBe(null);
  });
});

describe('interfaceStatus & rows', () => {
  it('disabled menang atas running', () => {
    expect(interfaceStatus({ name: 'e1', disabled: true, running: true })).toBe('disabled');
  });
  it('running bila running=true, down selainnya', () => {
    expect(interfaceStatus({ name: 'e1', running: true })).toBe('running');
    expect(interfaceStatus({ name: 'e1', running: false })).toBe('down');
    expect(interfaceStatus({ name: 'e1' })).toBe('down');
  });
  it('interfaceRows menyuntik status tiap baris', () => {
    const rows = interfaceRows([
      { name: 'ether1', running: true },
      { name: 'ether2', disabled: true },
    ]);
    expect(rows.map((r) => r.status)).toEqual(['running', 'disabled']);
  });
  it('menangani list null', () => {
    expect(interfaceRows(null)).toEqual([]);
    expect(interfaceRows(undefined)).toEqual([]);
  });
});

describe('filterInterfaceRows', () => {
  const rows = interfaceRows([
    { name: 'e1', running: true },
    { name: 'e2', running: false },
    { name: 'e3', disabled: true },
  ]);
  it('all mengembalikan semua', () => {
    expect(filterInterfaceRows(rows, 'all')).toHaveLength(3);
  });
  it('running/down/disabled memfilter', () => {
    expect(filterInterfaceRows(rows, 'running').map((r) => r.name)).toEqual(['e1']);
    expect(filterInterfaceRows(rows, 'down').map((r) => r.name)).toEqual(['e2']);
    expect(filterInterfaceRows(rows, 'disabled').map((r) => r.name)).toEqual(['e3']);
  });
});

describe('snapshotHealthStats', () => {
  it('mengagregasi kesehatan snapshot', () => {
    const s = snapshotHealthStats({
      isOnline: true,
      cpuLoad: 42,
      totalMemoryBytes: 1000,
      freeMemoryBytes: 250,
      totalHddBytes: 1000,
      freeHddBytes: 500,
      uptimeSeconds: 90061,
    });
    expect(s.online).toBe(true);
    expect(s.cpu).toBe(42);
    expect(s.memPct).toBe(75);
    expect(s.diskPct).toBe(50);
    expect(s.uptime).toBe('1d 1h');
  });
  it('menormalkan cpu dan null-safe', () => {
    const s = snapshotHealthStats({ isOnline: false, cpuLoad: 150 });
    expect(s.cpu).toBe(100);
    expect(s.memPct).toBe(null);
    expect(s.uptime).toBe('—');
  });
});

describe('friendlyRouterError', () => {
  it('menerjemahkan error umum', () => {
    expect(friendlyRouterError('Router not found')).toContain('tidak ditemukan');
    expect(friendlyRouterError('Connection timed out')).toContain('timeout');
    expect(friendlyRouterError('connection refused')).toContain('offline');
  });
  it('loloskan pesan lain apa adanya, null -> generik', () => {
    expect(friendlyRouterError('disk hampir penuh')).toBe('disk hampir penuh');
    expect(friendlyRouterError(null)).toBe('Gagal memuat data router.');
    expect(friendlyRouterError('')).toBe('Gagal memuat data router.');
  });
});
