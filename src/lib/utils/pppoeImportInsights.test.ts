import { describe, expect, it } from 'vitest';
import {
  pppoeActionLabel,
  pppoeActionTone,
  pppoeDefaultSelection,
  pppoeMappingError,
  pppoeSummary,
} from './pppoeImportInsights';

describe('pppoe label + tone', () => {
  it('3 aksi', () => {
    expect(pppoeActionLabel('new')).toBe('Baru');
    expect(pppoeActionLabel('update')).toBe('Perbarui');
    expect(pppoeActionLabel('same')).toBe('Sama');
    expect(pppoeActionTone('new')).toBe('positive');
    expect(pppoeActionTone('same')).toBe('neutral');
  });
});

describe('mapping + seleksi + ringkasan', () => {
  it('pasangan wajib lengkap', () => {
    expect(pppoeMappingError('c1', '')).toContain('dua-duanya');
    expect(pppoeMappingError('', '')).toBeNull();
    expect(pppoeMappingError('c1', 'l1')).toBeNull();
  });
  it('default new+update; ringkasan benar', () => {
    const rows = [
      { action: 'new' as const, username: 'a' },
      { action: 'update' as const, username: 'b' },
      { action: 'same' as const, username: 'c' },
    ];
    expect(pppoeDefaultSelection(rows)).toEqual(['a', 'b']);
    expect(pppoeSummary(rows)).toEqual({ total: 3, fresh: 1, updates: 1, same: 1 });
  });
});
