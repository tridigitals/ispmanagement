import { describe, expect, it } from 'vitest';
import {
  featureIsOn,
  featureValueLabel,
  formatBytesIEC,
  friendlyPlanError,
  groupFeaturesByCategory,
  usagePercent,
  usageTone,
} from './subscriptionInsights';

describe('featureValueLabel', () => {
  it('unlimited -> Tanpa batas (case-insensitive)', () => {
    expect(featureValueLabel({ value_type: 'number', value: 'unlimited' })).toBe('Tanpa batas');
    expect(featureValueLabel({ value_type: 'number', value: 'Unlimited' })).toBe('Tanpa batas');
  });
  it('boolean -> Ya/Tidak', () => {
    expect(featureValueLabel({ value_type: 'boolean', value: 'true' })).toBe('Ya');
    expect(featureValueLabel({ value_type: 'boolean', value: 'false' })).toBe('Tidak');
  });
  it('number diformat id-ID', () => {
    expect(featureValueLabel({ value_type: 'number', value: '50' })).toBe('50');
    expect(featureValueLabel({ value_type: 'number', value: '1000000' })).toBe('1.000.000');
  });
  it('kosong -> em-dash; text apa adanya', () => {
    expect(featureValueLabel({ value_type: 'text', value: '' })).toBe('—');
    expect(featureValueLabel({ value_type: 'text', value: 'priority' })).toBe('priority');
  });
});

describe('featureIsOn', () => {
  it('boolean true/false', () => {
    expect(featureIsOn({ value_type: 'boolean', value: 'true' })).toBe(true);
    expect(featureIsOn({ value_type: 'boolean', value: 'false' })).toBe(false);
  });
  it('number > 0 on, 0 off, unlimited on', () => {
    expect(featureIsOn({ value_type: 'number', value: '10' })).toBe(true);
    expect(featureIsOn({ value_type: 'number', value: '0' })).toBe(false);
    expect(featureIsOn({ value_type: 'number', value: 'unlimited' })).toBe(true);
  });
  it('text non-kosong dianggap aktif', () => {
    expect(featureIsOn({ value_type: 'text', value: 'custom_domain' })).toBe(true);
    expect(featureIsOn({ value_type: 'text', value: '  ' })).toBe(false);
  });
});

describe('groupFeaturesByCategory', () => {
  const f = (code: string, category: string) => ({
    feature_id: code,
    code,
    name: code,
    value_type: 'boolean',
    value: 'true',
    category,
  });
  it('mengelompokkan + kategori asc, urutan asal dipertahankan', () => {
    const g = groupFeaturesByCategory([f('b1', 'B'), f('a1', 'A'), f('b2', 'B')]);
    expect(g.map((x) => x.category)).toEqual(['A', 'B']);
    expect(g[1].items.map((x) => x.code)).toEqual(['b1', 'b2']);
  });
  it('kategori kosong -> Lainnya', () => {
    const g = groupFeaturesByCategory([f('x', '')]);
    expect(g[0].category).toBe('Lainnya');
  });
});

describe('usagePercent / usageTone', () => {
  it('limit null/0 -> 0 (unlimited)', () => {
    expect(usagePercent(500, null)).toBe(0);
    expect(usagePercent(500, 0)).toBe(0);
  });
  it('dibatasi 100 dan dibulatkan', () => {
    expect(usagePercent(50, 100)).toBe(50);
    expect(usagePercent(150, 100)).toBe(100);
    expect(usagePercent(1, 3)).toBe(33);
  });
  it('tone ambang 80/100', () => {
    expect(usageTone(50)).toBe('positive');
    expect(usageTone(85)).toBe('warning');
    expect(usageTone(100)).toBe('negative');
  });
});

describe('formatBytesIEC', () => {
  it('satuan naik pangkat 1024', () => {
    expect(formatBytesIEC(0)).toBe('0 B');
    expect(formatBytesIEC(1024)).toBe('1 KB');
    expect(formatBytesIEC(5 * 1024 ** 3)).toBe('5 GB');
  });
  it('negatif/NaN -> em-dash', () => {
    expect(formatBytesIEC(-1)).toBe('—');
    expect(formatBytesIEC(Number.NaN)).toBe('—');
  });
});

describe('friendlyPlanError', () => {
  it('still used by -> daftar tenant', () => {
    expect(
      friendlyPlanError("plan 'pro' is still used by: jmk, isp-management"),
    ).toContain('jmk, isp-management');
  });
  it('slug duplikat', () => {
    expect(friendlyPlanError('a plan with slug x already exists')).toContain('sudah dipakai');
  });
  it('free plan invoice ditolak', () => {
    expect(friendlyPlanError('free plans do not require an invoice')).toContain('Paket gratis');
  });
  it('kosong -> pesan umum', () => {
    expect(friendlyPlanError('')).toContain('kesalahan');
  });
});
