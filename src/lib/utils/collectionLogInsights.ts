/**
 * Helper murni log penagihan v2 (gelombang 24c).
 *
 * Label aksi/hasil/status/pengingat + tone pill dulu inline `$t()` di
 * halaman legacy — kini pemetaan murni + tes.
 */
export function collectionActionLabel(action: string): string {
  const x = String(action || '').toLowerCase();
  if (x === 'reminder') return 'Pengingat';
  if (x === 'suspend') return 'Suspend';
  if (x === 'grace_expire_suspend') return 'Suspend grace habis';
  if (x === 'resume') return 'Aktif lagi';
  if (x === 'installation') return 'Aktivasi instalasi';
  if (x === 'assignment') return 'Assign layanan';
  if (x === 'payment_callback') return 'Callback pembayaran';
  return action || '—';
}

export function collectionActionTone(action: string): 'positive' | 'negative' | 'neutral' | 'warning' {
  const x = String(action || '').toLowerCase();
  if (x === 'resume' || x === 'installation' || x === 'assignment') return 'positive';
  if (x === 'suspend' || x === 'grace_expire_suspend') return 'negative';
  if (x === 'payment_callback') return 'neutral';
  return 'warning';
}

export function collectionActionHint(action: string): string {
  const x = String(action || '').toLowerCase();
  if (x === 'reminder') return 'Pengingat invoice dikirim atau dijadwalkan';
  if (x === 'suspend') return 'Suspend sesuai policy billing aktif';
  if (x === 'grace_expire_suspend') return 'Suspend setelah masa tenggang berakhir';
  if (x === 'resume') return 'Layanan aktif lagi setelah pembayaran diterima';
  if (x === 'installation') return 'Invoice instalasi dibayar, layanan siap diaktifkan';
  if (x === 'assignment') return 'Runner menemukan assignment atau relasi layanan';
  if (x === 'payment_callback') return 'Status invoice diperbarui dari callback pembayaran';
  return 'Aktivitas scheduler billing';
}

export function collectionResultLabel(result: string): string {
  const x = String(result || '').toLowerCase();
  if (x === 'success' || x === 'sent') return 'Berhasil';
  if (x === 'failed') return 'Gagal';
  if (x === 'skipped' || x === 'queued') return 'Dilewati';
  return result || '—';
}

export function collectionResultTone(result: string): 'positive' | 'negative' | 'warning' {
  const x = String(result || '').toLowerCase();
  if (x === 'success' || x === 'sent') return 'positive';
  if (x === 'failed') return 'negative';
  return 'warning';
}

export function collectionReminderLabel(code: string): string {
  const x = String(code || '').toLowerCase();
  if (x === 'd7') return 'H-7';
  if (x === 'd3') return 'H-3';
  if (x === 'd0') return 'H-0';
  return code || '—';
}

export function toIsoUtc(input: string): string | undefined {
  const value = input.trim();
  if (!value) return undefined;
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return undefined;
  return parsed.toISOString();
}
