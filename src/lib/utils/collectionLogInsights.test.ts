import { describe, expect, it } from 'vitest';
import {
  collectionActionHint,
  collectionActionLabel,
  collectionActionTone,
  collectionReminderLabel,
  collectionResultLabel,
  collectionResultTone,
  toIsoUtc,
} from './collectionLogInsights';

describe('collectionActionLabel + Tone + Hint', () => {
  it('7 aksi dikenal + fallback', () => {
    expect(collectionActionLabel('reminder')).toBe('Pengingat');
    expect(collectionActionLabel('grace_expire_suspend')).toBe('Suspend grace habis');
    expect(collectionActionLabel('payment_callback')).toBe('Callback pembayaran');
    expect(collectionActionLabel('x')).toBe('x');
    expect(collectionActionTone('resume')).toBe('positive');
    expect(collectionActionTone('suspend')).toBe('negative');
    expect(collectionActionTone('payment_callback')).toBe('neutral');
    expect(collectionActionHint('resume')).toContain('aktif lagi');
  });
});

describe('collectionResult + Reminder + toIsoUtc', () => {
  it('hasil & pengingat', () => {
    expect(collectionResultLabel('sent')).toBe('Berhasil');
    expect(collectionResultLabel('queued')).toBe('Dilewati');
    expect(collectionResultTone('failed')).toBe('negative');
    expect(collectionReminderLabel('d7')).toBe('H-7');
    expect(collectionReminderLabel('d0')).toBe('H-0');
  });
  it('toIsoUtc valid/kosong/tak valid', () => {
    expect(toIsoUtc('')).toBeUndefined();
    expect(toIsoUtc('ngaco')).toBeUndefined();
    expect(toIsoUtc('2026-09-01T10:00')).toContain('2026-09-01');
  });
});
