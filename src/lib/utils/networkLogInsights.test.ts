import { describe, expect, it } from 'vitest';
import { logFilterKey, logLevelLabel, logLevelTone } from './networkLogInsights';

describe('logLevelTone', () => {
  it('critical/error -> negative', () => {
    expect(logLevelTone('critical')).toBe('negative');
    expect(logLevelTone('ERROR')).toBe('negative');
  });
  it('warning -> warning', () => {
    expect(logLevelTone('warning')).toBe('warning');
  });
  it('debug -> neutral', () => {
    expect(logLevelTone('debug')).toBe('neutral');
  });
  it('info/kosong/tak dikenal -> info', () => {
    expect(logLevelTone('info')).toBe('info');
    expect(logLevelTone(null)).toBe('info');
    expect(logLevelTone('something')).toBe('info');
  });
});

describe('logLevelLabel', () => {
  it('lowercase + default info', () => {
    expect(logLevelLabel('Warning')).toBe('warning');
    expect(logLevelLabel(undefined)).toBe('info');
  });
});

describe('logFilterKey', () => {
  it('stabil utk filter sama, beda bila berubah', () => {
    const a = { routerId: '', level: '', topic: '', q: '', month: '9', year: '2026' };
    expect(logFilterKey(a)).toBe(logFilterKey({ ...a }));
    expect(logFilterKey(a)).not.toBe(logFilterKey({ ...a, q: 'error' }));
  });
});
