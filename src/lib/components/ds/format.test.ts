import { describe, expect, it } from 'vitest';
import {
  formatRupiah,
  formatCompactRupiah,
  formatDate,
  formatRelative,
  formatPercent,
} from './format';

describe('formatRupiah', () => {
  it('memformat angka dengan pemisah ribuan gaya Indonesia', () => {
    expect(formatRupiah(56485000)).toBe('Rp56.485.000');
    expect(formatRupiah(125000)).toBe('Rp125.000');
  });

  it('tidak menyisipkan spasi tak-putus setelah Rp', () => {
    // Ini regresi nyata: Intl id-ID default menghasilkan "Rp\u00a0125.000",
    // sehingga di layar bercampur dengan "Rp56,8jt" dari versi ringkas.
    expect(formatRupiah(125000)).not.toContain('\u00a0');
    expect(formatRupiah(125000).startsWith('Rp1')).toBe(true);
  });

  it('menerima string angka dari API', () => {
    expect(formatRupiah('75000')).toBe('Rp75.000');
  });

  it('mengembalikan em dash untuk nilai kosong atau bukan angka', () => {
    expect(formatRupiah(null)).toBe('—');
    expect(formatRupiah(undefined)).toBe('—');
    expect(formatRupiah('bukan angka')).toBe('—');
  });

  it('memformat nol sebagai nilai, bukan kosong', () => {
    expect(formatRupiah(0)).toBe('Rp0');
  });
});

describe('formatCompactRupiah', () => {
  it('meringkas jutaan dan miliaran', () => {
    expect(formatCompactRupiah(56485000)).toBe('Rp56,5jt');
    expect(formatCompactRupiah(1_500_000_000)).toBe('Rp1,5M');
    expect(formatCompactRupiah(125000)).toBe('Rp125rb');
  });

  it('memakai gaya penulisan yang sama dengan formatRupiah untuk nilai kecil', () => {
    expect(formatCompactRupiah(500)).toBe('Rp500');
    expect(formatCompactRupiah(500)).not.toContain('\u00a0');
  });
});

describe('formatPercent', () => {
  it('memakai koma sebagai pemisah desimal', () => {
    expect(formatPercent(2.6)).toBe('2,6%');
    expect(formatPercent(97.9)).toBe('97,9%');
  });

  it('menghormati jumlah digit yang diminta', () => {
    expect(formatPercent(54.321, 0)).toBe('54%');
  });

  it('menangani nilai kosong', () => {
    expect(formatPercent(null)).toBe('—');
  });
});

describe('formatDate', () => {
  it('memformat tanggal tanpa jam secara default', () => {
    const out = formatDate('2026-04-16T00:00:00Z');
    expect(out).toMatch(/2026/);
    expect(out).not.toMatch(/\d{2}\.\d{2}/);
  });

  it('menyertakan jam ketika withTime true', () => {
    expect(formatDate('2026-04-16T13:45:00Z', true)).toMatch(/\d{2}\.\d{2}/);
  });

  it('menangani tanggal tidak valid', () => {
    expect(formatDate('bukan-tanggal')).toBe('—');
    expect(formatDate(null)).toBe('—');
  });
});

describe('formatRelative', () => {
  it('menyebut "lalu" untuk masa lalu dan "lagi" untuk masa depan', () => {
    const duaJamLalu = new Date(Date.now() - 2 * 3600_000);
    const tigaHariLagi = new Date(Date.now() + 3 * 86_400_000);

    expect(formatRelative(duaJamLalu)).toBe('2 jam lalu');
    expect(formatRelative(tigaHariLagi)).toBe('3 hari lagi');
  });

  it('menyebut "baru saja" untuk selisih di bawah satu menit', () => {
    expect(formatRelative(new Date())).toBe('baru saja');
  });
});
