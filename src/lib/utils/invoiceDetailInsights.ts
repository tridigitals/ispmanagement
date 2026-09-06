/**
 * Helper murni detail invoice v2 (gelombang 24c).
 *
 * Label status, klasifikasi pembayaran manual vs online, dan label
 * metode bayar dulu inline `$t()` di halaman legacy — kini murni + tes.
 */
export function invoiceStatusLabel(status: string): string {
  const map: Record<string, string> = {
    pending: 'Menunggu bayar',
    verification_pending: 'Menunggu verifikasi',
    paid: 'Lunas',
    failed: 'Gagal',
  };
  return map[status] || status;
}

export function invoiceStatusTone(status: string): 'warning' | 'info' | 'positive' | 'negative' | 'neutral' {
  if (status === 'paid') return 'positive';
  if (status === 'failed') return 'negative';
  if (status === 'verification_pending') return 'info';
  if (status === 'pending') return 'warning';
  return 'neutral';
}

export function isManualPaymentInvoice(row: {
  status?: string;
  proof_attachment?: string | null;
  payment_method?: string | null;
} | null): boolean {
  if (!row) return false;
  const method = String(row.payment_method || '').toLowerCase();
  return (
    row.status === 'verification_pending' ||
    !!row.proof_attachment ||
    method.includes('bank') ||
    method.includes('manual')
  );
}

export function invoicePaymentMethodLabel(row: {
  status?: string;
  proof_attachment?: string | null;
  payment_method?: string | null;
} | null): string {
  if (!row) return '-';
  if (isManualPaymentInvoice(row)) return 'Transfer bank';
  const method = String(row.payment_method || '').toLowerCase();
  if (method.includes('midtrans') || !method) return 'Pembayaran online';
  return row.payment_method || '-';
}

export const INVOICE_REJECT_REASONS = [
  'Bukti transfer tidak jelas',
  'Nominal transfer tidak sesuai total invoice',
  'Rekening tujuan transfer salah',
  'Bukti tidak valid atau tidak terkait',
  'Bukti duplikat sudah dipakai',
];
