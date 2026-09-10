/**
 * Helper murni detail invoice v2 (gelombang 24c).
 *
 * Label status, klasifikasi pembayaran manual vs online, dan label
 * metode bayar dulu inline `$t()` di halaman legacy — kini murni + tes.
 */
export function invoiceStatusLabel(status: string, tt?: (k: string) => string): string {
  const map: Record<string, string> = {
    pending: 'Menunggu bayar',
    verification_pending: 'Menunggu verifikasi',
    paid: 'Lunas',
    failed: 'Gagal',
  };
  if (tt && map[status]) return tt('admin.invoices.detail.st_' + status);
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
} | null, tt?: (k: string) => string): string {
  if (!row) return '-';
  if (isManualPaymentInvoice(row)) return tt ? tt('admin.invoices.detail.method_bank') : 'Transfer bank';
  const method = String(row.payment_method || '').toLowerCase();
  if (method.includes('midtrans') || !method) return tt ? tt('admin.invoices.detail.method_online') : 'Pembayaran online';
  return row.payment_method || '-';
}

export const INVOICE_REJECT_REASONS = [
  'Bukti transfer tidak jelas',
  'Nominal transfer tidak sesuai total invoice',
  'Rekening tujuan transfer salah',
  'Bukti tidak valid atau tidak terkait',
  'Bukti duplikat sudah dipakai',
];

/** Key i18n sejajar INVOICE_REJECT_REASONS (halaman memetakan dengan $t). */
export const INVOICE_REJECT_REASON_KEYS = [
  'admin.invoices.detail.r1',
  'admin.invoices.detail.r2',
  'admin.invoices.detail.r3',
  'admin.invoices.detail.r4',
  'admin.invoices.detail.r5',
];
