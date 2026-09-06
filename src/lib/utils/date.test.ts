import { describe, expect, it } from 'vitest';
import { timeAgo } from './date';

describe('timeAgo id', () => {
  it('relatif Indonesia', () => {
    expect(timeAgo(new Date(Date.now() - 5_000))).toBe('baru saja');
    expect(timeAgo(new Date(Date.now() - 90_000))).toBe('2 mnt lalu');
    expect(timeAgo(new Date(Date.now() - 3 * 3_600_000))).toBe('3 jam lalu');
  });
  it('masa depan + invalid dijaga', () => {
    expect(timeAgo(new Date(Date.now() + 86_400_000))).toBe('baru saja');
    expect(timeAgo('bukan-tanggal')).toBe('—');
  });
});
