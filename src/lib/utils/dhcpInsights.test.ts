import { describe, expect, it } from 'vitest';
import {
  dhcpSyncLabel,
  dhcpSyncState,
  dhcpSyncSummary,
  dhcpSyncTone,
  friendlyDhcpError,
  type DhcpSyncRow,
} from './dhcpInsights';

const base: DhcpSyncRow = {
  lease_present: true,
  lease_last_error: null,
  lease_last_sync_at: null,
  queue_mode: 'none',
  queue_present: false,
  queue_last_error: null,
  queue_rate_limit: null,
  disabled: false,
};

const row = (over: Partial<DhcpSyncRow>): DhcpSyncRow => ({ ...base, ...over });

describe('dhcpSyncState', () => {
  it('lease ada tanpa queue = synced', () => {
    expect(dhcpSyncState(row({}))).toBe('synced');
  });

  it('disabled menang atas segalanya', () => {
    expect(dhcpSyncState(row({ disabled: true, lease_present: false }))).toBe('disabled');
  });

  it('lease hilang = missing', () => {
    expect(dhcpSyncState(row({ lease_present: false }))).toBe('missing');
  });

  it('queue mode none tidak dianggap masalah walau queue_present false', () => {
    expect(dhcpSyncState(row({ queue_mode: 'none', queue_present: false }))).toBe('synced');
  });

  it('queue diminta tapi belum ada = partial (lease sudah ada)', () => {
    expect(dhcpSyncState(row({ queue_mode: 'simple_queue', queue_present: false }))).toBe(
      'partial',
    );
  });

  it('error lease saja = missing (lease dianggap bermasalah)', () => {
    expect(dhcpSyncState(row({ lease_present: false, lease_last_error: 'boom' }))).toBe('missing');
  });

  it('lease + queue bermasalah sekaligus = error', () => {
    expect(
      dhcpSyncState(
        row({
          lease_present: false,
          lease_last_error: 'boom',
          queue_mode: 'simple_queue',
          queue_present: false,
          queue_last_error: 'trap',
        }),
      ),
    ).toBe('error');
  });
});

describe('dhcpSyncTone / label', () => {
  it('setiap state punya tone dan label', () => {
    for (const s of ['synced', 'partial', 'missing', 'error', 'disabled'] as const) {
      expect(dhcpSyncTone(s)).toBeTruthy();
      expect(dhcpSyncLabel(s).length).toBeGreaterThan(2);
    }
    expect(dhcpSyncTone('synced')).toBe('positive');
    expect(dhcpSyncTone('missing')).toBe('negative');
    expect(dhcpSyncLabel('partial')).toBe('Sebagian');
  });
});

describe('dhcpSyncSummary', () => {
  it('menggabungkan dua dimensi', () => {
    const s = dhcpSyncSummary(row({ queue_mode: 'simple_queue', queue_present: true, queue_rate_limit: '10M/10M' }));
    expect(s).toContain('Lease ada di router');
    expect(s).toContain('Queue aktif (10M/10M)');
  });

  it('menampilkan error lease', () => {
    const s = dhcpSyncSummary(row({ lease_present: false, lease_last_error: 'timeout' }));
    expect(s).toContain('Lease gagal: timeout');
  });

  it('mode none disebut eksplisit', () => {
    expect(dhcpSyncSummary(row({}))).toContain('tanpa pembatas bandwidth');
  });
});

describe('friendlyDhcpError', () => {
  it('duplicate', () => {
    expect(friendlyDhcpError('Duplicate DHCP static subscription, MAC address, or IP address detected')).toContain(
      'sudah terdaftar',
    );
  });

  it('MAC & IP', () => {
    expect(friendlyDhcpError('mac_address must use a valid MAC format like AA:BB:CC:DD:EE:FF')).toContain(
      'AA:BB:CC:DD:EE:FF',
    );
    expect(friendlyDhcpError('ip_address must contain a valid IPv4 or IPv6 address')).toContain(
      'IP address tidak valid',
    );
  });

  it('queue failure diterjemahkan', () => {
    expect(friendlyDhcpError('Lease applied, but queue failed: no such command')).toContain(
      'Lease berhasil, tapi queue gagal',
    );
  });

  it('kosong & unknown', () => {
    expect(friendlyDhcpError('')).toBe('Gagal memproses permintaan.');
    expect(friendlyDhcpError('weird')).toBe('weird');
  });
});
