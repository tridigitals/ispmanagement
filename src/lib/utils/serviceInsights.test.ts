import { describe, expect, it } from 'vitest';
import {
  convertPrice,
  currencyDigits,
  friendlyDeleteError,
  isInternetType,
  isPppoeProvisioning,
  mappingAllowed,
  normalizeProvisioningType,
  normalizeServiceType,
  provisioningTypeLabel,
  roundForCurrency,
  serviceTypeLabel,
  serviceTypeTone,
  usageSummary,
  validatePackageDraft,
} from './serviceInsights';

describe('normalizeServiceType', () => {
  it('memetakan nilai dikenal, default internet_pppoe', () => {
    expect(normalizeServiceType('hotspot')).toBe('hotspot');
    expect(normalizeServiceType('VPN')).toBe('vpn');
    expect(normalizeServiceType('internet_pppoe')).toBe('internet_pppoe');
    expect(normalizeServiceType(null)).toBe('internet_pppoe');
    expect(normalizeServiceType('unknown')).toBe('internet_pppoe');
  });
});

describe('normalizeProvisioningType', () => {
  it('hanya dhcp_static yang berbeda, sisanya pppoe', () => {
    expect(normalizeProvisioningType('dhcp_static')).toBe('dhcp_static');
    expect(normalizeProvisioningType('DHCP_STATIC')).toBe('dhcp_static');
    expect(normalizeProvisioningType('pppoe')).toBe('pppoe');
    expect(normalizeProvisioningType(undefined)).toBe('pppoe');
    expect(normalizeProvisioningType('sampah')).toBe('pppoe');
  });
});

describe('mappingAllowed', () => {
  it('hanya Internet + PPPoE', () => {
    expect(mappingAllowed('internet_pppoe', 'pppoe')).toBe(true);
    expect(mappingAllowed('internet_pppoe', 'dhcp_static')).toBe(false);
    expect(mappingAllowed('hotspot', 'pppoe')).toBe(false);
    expect(mappingAllowed('vpn', null)).toBe(false);
  });
  it('konsisten dengan helper atomik', () => {
    for (const st of ['internet_pppoe', 'hotspot', 'vpn']) {
      for (const pt of ['pppoe', 'dhcp_static']) {
        expect(mappingAllowed(st, pt)).toBe(isInternetType(st) && isPppoeProvisioning(pt));
      }
    }
  });
});

describe('label & tone', () => {
  it('label tipe menggabungkan provisioning', () => {
    expect(serviceTypeLabel('internet_pppoe', 'dhcp_static')).toBe('Internet / DHCP Static');
    expect(serviceTypeLabel('internet_pppoe', 'pppoe')).toBe('Internet / PPPoE');
    expect(serviceTypeLabel('hotspot')).toBe('Hotspot');
    expect(serviceTypeLabel('vpn')).toBe('VPN');
  });
  it('label provisioning', () => {
    expect(provisioningTypeLabel('dhcp_static')).toBe('DHCP Static');
    expect(provisioningTypeLabel('pppoe')).toBe('PPPoE');
  });
  it('tone valid terhadap StatusTone ds', () => {
    expect(serviceTypeTone('internet_pppoe')).toBe('positive');
    expect(serviceTypeTone('hotspot')).toBe('info');
    expect(serviceTypeTone('vpn')).toBe('neutral');
  });
});

describe('mata uang', () => {
  it('digit per kode', () => {
    expect(currencyDigits('IDR')).toBe(0);
    expect(currencyDigits('idr')).toBe(0);
    expect(currencyDigits('JPY')).toBe(0);
    expect(currencyDigits('USD')).toBe(2);
  });
  it('roundForCurrency IDR membulatkan utuh', () => {
    expect(roundForCurrency(150000.6, 'IDR')).toBe(150001);
    expect(roundForCurrency(10.005, 'USD')).toBe(10.01);
  });
  it('convertPrice: mata uang sama -> basis apa adanya', () => {
    expect(convertPrice(150000, 'IDR', 'IDR', null)).toEqual({ amount: 150000, currency: 'IDR' });
  });
  it('convertPrice: tanpa kurs -> fallback basis', () => {
    expect(convertPrice(150000, 'IDR', 'USD', null)).toEqual({ amount: 150000, currency: 'IDR' });
  });
  it('convertPrice: dengan kurs -> konversi + pembulatan', () => {
    const r = convertPrice(150000, 'IDR', 'USD', 0.000061);
    expect(r.currency).toBe('USD');
    expect(r.amount).toBe(9.15);
  });
  it('convertPrice: nol -> 0 tenant', () => {
    expect(convertPrice(0, 'IDR', 'USD', 0.0001)).toEqual({ amount: 0, currency: 'USD' });
  });
});

describe('validatePackageDraft', () => {
  it('draft kosong -> dua error (nama + bulanan)', () => {
    const errs = validatePackageDraft({ name: '  ', priceMonthly: 0, priceYearly: 0, yearlyEnabled: false });
    expect(errs).toHaveLength(2);
  });
  it('tahunan aktif tanpa harga -> error', () => {
    const errs = validatePackageDraft({ name: 'X', priceMonthly: 100000, priceYearly: 0, yearlyEnabled: true });
    expect(errs.some((e) => /tahunan/i.test(e))).toBe(true);
  });
  it('draft valid -> nol error', () => {
    const errs = validatePackageDraft({ name: 'X', priceMonthly: 100000, priceYearly: 1100000, yearlyEnabled: true });
    expect(errs).toEqual([]);
  });
  it('tahunan nonaktif mengabaikan harga tahunan', () => {
    const errs = validatePackageDraft({ name: 'X', priceMonthly: 100000, priceYearly: 0, yearlyEnabled: false });
    expect(errs).toEqual([]);
  });
});

describe('usageSummary', () => {
  it('lookup aman untuk paket tanpa data', () => {
    expect(usageSummary({ a: 5 }, 'a')).toBe(5);
    expect(usageSummary({ a: 5 }, 'b')).toBe(0);
    expect(usageSummary({}, 'x')).toBe(0);
  });
});

describe('friendlyDeleteError', () => {
  it('menerjemahkan guard referensi server', () => {
    const msg =
      "Package 'Fiber 20Mbps' is still in use: 551 subscriptions, 546 pppoe_accounts. Move or cancel the related records before deleting.";
    const out = friendlyDeleteError(msg);
    expect(out).toContain('masih dipakai');
    expect(out).toContain('551 subscriptions');
    expect(out).not.toMatch(/Move or cancel/);
  });
  it('error lain diteruskan apa adanya', () => {
    expect(friendlyDeleteError('boom')).toBe('boom');
  });
  it('kosong -> pesan generik', () => {
    expect(friendlyDeleteError(null)).toBe('Gagal menghapus paket.');
  });
});
