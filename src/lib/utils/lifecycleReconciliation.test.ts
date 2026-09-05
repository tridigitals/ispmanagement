import { describe, expect, it } from 'vitest';
import {
  lifecycleActionLabel,
  lifecycleIssueLabel,
  lifecyclePeriod,
  lifecycleServiceLabel,
} from './lifecycleReconciliation';

describe('lifecycleIssueLabel', () => {
  it('dua tipe dikenal + fallback', () => {
    expect(lifecycleIssueLabel('missing_bootstrap_invoice')).toBe('Belum ada invoice awal');
    expect(lifecycleIssueLabel('invalid_active_lifecycle')).toBe('Lifecycle aktif tidak valid');
    expect(lifecycleIssueLabel('aneh')).toBe('aneh');
  });
});

describe('lifecycleActionLabel', () => {
  it('tiga aksi dikenal + fallback', () => {
    expect(lifecycleActionLabel('bootstrap_invoice')).toBe('Buat invoice awal');
    expect(lifecycleActionLabel('review_lifecycle_data')).toBe('Tinjau data lifecycle');
    expect(lifecycleActionLabel('suspend_invalid_active_lifecycle')).toBe('Suspend layanan');
    expect(lifecycleActionLabel('x')).toBe('x');
  });
});

describe('lifecycleServiceLabel + lifecyclePeriod', () => {
  it('gabung paket + lokasi, fallback aman', () => {
    expect(lifecycleServiceLabel('10 Mbps', 'Kalianda')).toBe('10 Mbps • Kalianda');
    expect(lifecycleServiceLabel(null, null)).toBe('Paket • Lokasi tak dikenal');
  });
  it('periode YYYY-MM-DD → YYYY-MM-DD', () => {
    expect(lifecyclePeriod('2026-01-01T00:00:00Z', '2026-02-01T00:00:00Z')).toBe('2026-01-01 → 2026-02-01');
    expect(lifecyclePeriod(null, null)).toBe('— → —');
  });
});
