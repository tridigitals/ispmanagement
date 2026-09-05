import type { StatusTone } from '$lib/components/ds/tokens';

/**
 * Helper murni untuk halaman Email Outbox v2.
 * Server (models/email_outbox.rs) sudah mengirim retryable, next_retry_at,
 * dan delivery_status_summary — layar legacy tidak pernah menampilkannya.
 */

export type OutboxStatus = 'queued' | 'sending' | 'sent' | 'failed' | string;

/** Guard FE — server menegakkan hal yang sama via SQL (retry_allowed_status). */
export function isRetryable(status: OutboxStatus, attempts: number, maxAttempts: number): boolean {
  const s = String(status || '').toLowerCase();
  if (s !== 'queued' && s !== 'failed') return false;
  return attempts < maxAttempts;
}

export function outboxStatusTone(status: OutboxStatus): StatusTone {
  const s = String(status || '').toLowerCase();
  if (s === 'sent') return 'positive';
  if (s === 'sending') return 'info';
  if (s === 'failed') return 'negative';
  return 'neutral';
}

export function outboxStatusLabel(status: OutboxStatus): string {
  const s = String(status || '').toLowerCase();
  if (s === 'queued') return 'Antri';
  if (s === 'sending') return 'Mengirim';
  if (s === 'sent') return 'Terkirim';
  if (s === 'failed') return 'Gagal';
  return s || 'Antri';
}

/** Ringkasan pengiriman dalam bahasa Indonesia (mirror derive_delivery_status_summary). */
export function deliverySummary(
  status: OutboxStatus,
  attempts: number,
  maxAttempts: number,
  retryable: boolean
): string {
  const s = String(status || '').toLowerCase();
  const a = Math.max(attempts, 1);
  const m = Math.max(maxAttempts, 1);
  if (s === 'sent') return `Terkirim setelah ${attempts || 1} percobaan`;
  if (s === 'sending') return `Sedang dikirim (percobaan ${a} dari ${m})`;
  if (s === 'failed') {
    return retryable
      ? `Gagal di percobaan ${a} dari ${m}; bisa dicoba lagi`
      : `Gagal setelah ${a} dari ${m} percobaan`;
  }
  if (s === 'queued') {
    return attempts > 0
      ? `Antrian ulang setelah percobaan ${attempts} dari ${m}`
      : `Mengantri untuk pengiriman pertama (0 dari ${m} percobaan)`;
  }
  return `${s} (${attempts} dari ${m} percobaan)`;
}

/** Pesan error ramah dari API retry/delete. */
export function friendlyOutboxError(message?: string | null): string {
  const m = String(message || '');
  if (/already sent|currently sending|not found/i.test(m)) {
    return 'Email ini sudah terkirim atau sedang dikirim — tidak bisa diulang.';
  }
  return m || 'Gagal memproses permintaan.';
}

/** Batas bulk yang sama dengan clamp server-side export (20.000). */
export function clampBulkIds(ids: string[], limit = 500): string[] {
  const clean = ids.map((s) => String(s || '').trim()).filter(Boolean);
  return clean.slice(0, limit);
}
