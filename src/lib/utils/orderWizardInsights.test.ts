import { describe, expect, it } from 'vitest';
import {
  orderPackagePriceLabel,
  validateOrderStep1,
  validateOrderStep2,
} from './orderWizardInsights';

describe('validateOrderStep1', () => {
  it('existing tanpa pilihan / baru tanpa nama / tanpa kontak', () => {
    expect(validateOrderStep1({ customerMode: 'existing', existingCustomerId: '', customer: { name: '', email: '', phone: '' } })).toContain('Pilih pelanggan');
    expect(validateOrderStep1({ customerMode: 'new', existingCustomerId: '', customer: { name: '', email: '', phone: '' } })).toContain('Nama');
    expect(validateOrderStep1({ customerMode: 'new', existingCustomerId: '', customer: { name: 'A', email: '', phone: '' } })).toContain('Email');
    expect(validateOrderStep1({ customerMode: 'new', existingCustomerId: '', customer: { name: 'A', email: '', phone: '081' } })).toBeNull();
  });
});

describe('validateOrderStep2 + harga', () => {
  it('alamat & paket wajib', () => {
    const base = { locationMode: 'new' as const, existingLocationId: '', location: { label: 'Rumah', address_line1: 'Jl A' }, packageId: 'p1' };
    expect(validateOrderStep2(base)).toBeNull();
    expect(validateOrderStep2({ ...base, packageId: '' })).toContain('paket');
    expect(validateOrderStep2({ ...base, location: { label: '', address_line1: 'Jl A' } })).toContain('Label');
  });
  it('label harga bulanan/tahunan', () => {
    expect(orderPackagePriceLabel(150000, 1500000, 'monthly')).toContain('150.000');
    expect(orderPackagePriceLabel(150000, 1500000, 'yearly')).toContain('1.500.000');
    expect(orderPackagePriceLabel(150000, 0, 'yearly')).toContain('150.000');
  });
});
