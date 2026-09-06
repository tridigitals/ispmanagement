import { describe, expect, it } from 'vitest';
import {
  INVOICE_REJECT_REASONS,
  invoicePaymentMethodLabel,
  invoiceStatusLabel,
  invoiceStatusTone,
  isManualPaymentInvoice,
} from './invoiceDetailInsights';

describe('invoiceStatusLabel + Tone', () => {
  it('4 status dikenal + fallback', () => {
    expect(invoiceStatusLabel('pending')).toBe('Menunggu bayar');
    expect(invoiceStatusLabel('verification_pending')).toBe('Menunggu verifikasi');
    expect(invoiceStatusLabel('paid')).toBe('Lunas');
    expect(invoiceStatusLabel('failed')).toBe('Gagal');
    expect(invoiceStatusLabel('x')).toBe('x');
    expect(invoiceStatusTone('paid')).toBe('positive');
    expect(invoiceStatusTone('failed')).toBe('negative');
  });
});

describe('isManualPaymentInvoice', () => {
  it('verification_pending / bukti / bank / manual = manual', () => {
    expect(isManualPaymentInvoice({ status: 'verification_pending' })).toBe(true);
    expect(isManualPaymentInvoice({ status: 'pending', proof_attachment: 'f1' })).toBe(true);
    expect(isManualPaymentInvoice({ status: 'pending', payment_method: 'bank_transfer' })).toBe(true);
    expect(isManualPaymentInvoice({ status: 'pending', payment_method: 'midtrans' })).toBe(false);
    expect(isManualPaymentInvoice(null)).toBe(false);
  });
  it('label metode + 5 alasan tolak', () => {
    expect(invoicePaymentMethodLabel({ status: 'pending', payment_method: 'midtrans' })).toBe('Pembayaran online');
    expect(invoicePaymentMethodLabel({ status: 'pending', payment_method: 'bank_transfer' })).toBe('Transfer bank');
    expect(INVOICE_REJECT_REASONS.length).toBe(5);
  });
});
