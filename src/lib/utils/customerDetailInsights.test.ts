import { describe, expect, it } from 'vitest';
import {
  customerHealthChips,
  formatLocationLine,
  friendlyCustomerError,
  invoicesForSubscriptions,
  subStatusLabel,
  subStatusTone,
  subscriptionIdFromInvoice,
} from './customerDetailInsights';

describe('subStatusLabel / subStatusTone', () => {
  it('label status lengkap', () => {
    expect(subStatusLabel('active')).toBe('Aktif');
    expect(subStatusLabel('grace_active')).toBe('Aktif sementara');
    expect(subStatusLabel('pending_installation')).toBe('Menunggu instalasi');
    expect(subStatusLabel('suspended')).toBe('Ditangguhkan');
    expect(subStatusLabel('weird')).toBe('weird');
  });
  it('tone per status', () => {
    expect(subStatusTone('active')).toBe('positive');
    expect(subStatusTone('pending_installation')).toBe('warning');
    expect(subStatusTone('suspended')).toBe('negative');
    expect(subStatusTone('cancelled')).toBe('neutral');
  });
});

describe('friendlyCustomerError', () => {
  it('menerjemahkan guard FK backend', () => {
    expect(
      friendlyCustomerError('cannot delete: still referenced by 3 subscriptions, 1 work orders'),
    ).toBe('Tidak bisa dihapus — masih dipakai oleh 3 langganan, 1 work order.');
  });
  it('menerjemahkan 404 known entities', () => {
    expect(friendlyCustomerError('Customer not found')).toBe('Pelanggan tidak ditemukan.');
    expect(friendlyCustomerError('Subscription not found')).toBe('Langganan tidak ditemukan.');
    expect(friendlyCustomerError('Location not found')).toBe('Lokasi tidak ditemukan.');
  });
  it('kosong dan generik', () => {
    expect(friendlyCustomerError('')).toBe('Terjadi kesalahan.');
    expect(friendlyCustomerError(null)).toBe('Terjadi kesalahan.');
    expect(friendlyCustomerError('boom')).toBe('boom');
  });
});

describe('customerHealthChips', () => {
  it('campuran status', () => {
    const chips = customerHealthChips({
      is_active: true,
      subscriptions: [
        { status: 'active' },
        { status: 'grace_active' },
        { status: 'suspended' },
        { status: 'pending_installation' },
      ],
      pendingInstallations: 0,
    });
    expect(chips.map((c) => c.key)).toEqual(['active', 'pending', 'suspended']);
    expect(chips[0].label).toBe('2 langganan aktif');
  });
  it('pelanggan nonaktif tanpa langganan', () => {
    const chips = customerHealthChips({ is_active: false, subscriptions: [], pendingInstallations: 0 });
    expect(chips.map((c) => c.key)).toEqual(['inactive']);
  });
  it('aktif tanpa langganan -> chip kosong', () => {
    const chips = customerHealthChips({ is_active: true, subscriptions: [], pendingInstallations: 0 });
    expect(chips[0].key).toBe('none');
  });
});

describe('formatLocationLine', () => {
  it('gabung alamat', () => {
    expect(formatLocationLine({ label: 'Rumah', address_line1: 'Jl. Mawar 1', city: 'Jogja' })).toBe(
      'Rumah — Jl. Mawar 1, Jogja',
    );
  });
  it('hanya label kalau alamat kosong', () => {
    expect(formatLocationLine({ label: 'Kantor' })).toBe('Kantor');
  });
});

describe('subscriptionIdFromInvoice / invoicesForSubscriptions', () => {
  it('parse pkgsub:<id>:<ts>', () => {
    expect(subscriptionIdFromInvoice({ external_id: 'pkgsub:sub-9:1700000000' })).toBe('sub-9');
  });
  it('tolak format asing', () => {
    expect(subscriptionIdFromInvoice({ external_id: 'manual' })).toBeNull();
    expect(subscriptionIdFromInvoice({ external_id: 'pkgsub:' })).toBeNull();
    expect(subscriptionIdFromInvoice({ external_id: null })).toBeNull();
  });
  it('filter invoice ke langganan milik pelanggan', () => {
    const ids = new Set(['sub-1']);
    const rows = [
      { external_id: 'pkgsub:sub-1:1' },
      { external_id: 'pkgsub:sub-2:2' },
      { external_id: 'other' },
    ];
    expect(invoicesForSubscriptions(rows, ids)).toEqual([{ external_id: 'pkgsub:sub-1:1' }]);
  });
});
