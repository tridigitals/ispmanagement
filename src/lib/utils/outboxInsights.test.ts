import { describe, expect, it } from 'vitest';
import {
  clampBulkIds,
  deliverySummary,
  friendlyOutboxError,
  isRetryable,
  outboxStatusLabel,
  outboxStatusTone,
} from './outboxInsights';

describe('isRetryable', () => {
  it('queued dengan sisa percobaan boleh', () => {
    expect(isRetryable('queued', 0, 3)).toBe(true);
    expect(isRetryable('queued', 2, 3)).toBe(true);
  });
  it('failed dengan sisa percobaan boleh', () => {
    expect(isRetryable('failed', 4, 5)).toBe(true);
  });
  it('sent tidak pernah boleh (email sudah sampai)', () => {
    expect(isRetryable('sent', 1, 3)).toBe(false);
    expect(isRetryable('SENT', 0, 3)).toBe(false);
  });
  it('sending tidak boleh', () => {
    expect(isRetryable('sending', 1, 3)).toBe(false);
  });
  it('kuota percobaan habis tidak boleh', () => {
    expect(isRetryable('failed', 5, 5)).toBe(false);
    expect(isRetryable('queued', 3, 3)).toBe(false);
  });
  it('status kosong/tidak dikenal tidak boleh', () => {
    expect(isRetryable('', 0, 3)).toBe(false);
    expect(isRetryable('unknown', 0, 3)).toBe(false);
  });
});

describe('outboxStatusTone', () => {
  it('memetakan empat status', () => {
    expect(outboxStatusTone('sent')).toBe('positive');
    expect(outboxStatusTone('sending')).toBe('info');
    expect(outboxStatusTone('failed')).toBe('negative');
    expect(outboxStatusTone('queued')).toBe('neutral');
    expect(outboxStatusTone('apapun')).toBe('neutral');
  });
});

describe('outboxStatusLabel', () => {
  it('label Indonesia', () => {
    expect(outboxStatusLabel('queued')).toBe('Antri');
    expect(outboxStatusLabel('sending')).toBe('Mengirim');
    expect(outboxStatusLabel('sent')).toBe('Terkirim');
    expect(outboxStatusLabel('failed')).toBe('Gagal');
    expect(outboxStatusLabel('')).toBe('Antri');
  });
});

describe('deliverySummary', () => {
  it('sent', () => {
    expect(deliverySummary('sent', 2, 5, false)).toBe('Terkirim setelah 2 percobaan');
    expect(deliverySummary('sent', 0, 5, false)).toBe('Terkirim setelah 1 percobaan');
  });
  it('sending', () => {
    expect(deliverySummary('sending', 1, 3, false)).toBe('Sedang dikirim (percobaan 1 dari 3)');
  });
  it('failed dengan sisa vs habis', () => {
    expect(deliverySummary('failed', 2, 5, true)).toBe('Gagal di percobaan 2 dari 5; bisa dicoba lagi');
    expect(deliverySummary('failed', 5, 5, false)).toBe('Gagal setelah 5 dari 5 percobaan');
  });
  it('queued awal vs antrian ulang', () => {
    expect(deliverySummary('queued', 0, 3, true)).toBe(
      'Mengantri untuk pengiriman pertama (0 dari 3 percobaan)'
    );
    expect(deliverySummary('queued', 2, 3, true)).toBe('Antrian ulang setelah percobaan 2 dari 3');
  });
  it('max_attempts nol tidak membagi nol', () => {
    expect(deliverySummary('queued', 0, 0, true)).toContain('0 dari 1');
  });
});

describe('friendlyOutboxError', () => {
  it('menormalkan pesan retry ditolak', () => {
    expect(friendlyOutboxError('Item not found, already sent, or currently sending')).toBe(
      'Email ini sudah terkirim atau sedang dikirim — tidak bisa diulang.'
    );
  });
  it('mempertahankan pesan lain', () => {
    expect(friendlyOutboxError('Forbidden')).toBe('Forbidden');
    expect(friendlyOutboxError(null)).toBe('Gagal memproses permintaan.');
  });
});

describe('clampBulkIds', () => {
  it('membuang kosong dan memotong ke limit', () => {
    const ids = ['a', '', '  ', 'b', 'c'];
    expect(clampBulkIds(ids, 2)).toEqual(['a', 'b']);
  });
  it('trim whitespace', () => {
    expect(clampBulkIds([' x '])).toEqual(['x']);
  });
});
