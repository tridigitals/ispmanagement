import { describe, expect, it, vi } from 'vitest';
import { fetchAllRows, type Paginated } from './fetchAllPages';

/**
 * Regresi untuk bug yang ditemukan di produksi: pemanggil lama menulis
 * `listCustomerPackageInvoices().then((r) => r.data)`, sehingga hanya menerima
 * 25 baris pertama (default backend) dan menganggapnya data lengkap.
 *
 * Backend nyata: `per_page.unwrap_or(25)` lalu `clamp(1, 100)`.
 */
function backendTiruan(total: number, defaultPerPage = 25) {
  return vi.fn(async (page: number, perPage?: number): Promise<Paginated<number>> => {
    const effective = Math.min(Math.max(perPage ?? defaultPerPage, 1), 100);
    const start = (page - 1) * effective;
    return {
      data: Array.from({ length: Math.max(0, Math.min(effective, total - start)) }, (_, i) => start + i),
      total,
      page,
      per_page: effective,
    };
  });
}

describe('fetchAllRows', () => {
  it('mengembalikan seluruh baris, bukan halaman pertama saja', async () => {
    const backend = backendTiruan(485);
    const rows = await fetchAllRows(backend);

    expect(rows).toHaveLength(485);
    // 485 baris / 100 per halaman = 5 permintaan
    expect(backend).toHaveBeenCalledTimes(5);
  });

  it('bentuk kembaliannya array, sepadan dengan `.then((r) => r.data)` yang lama', async () => {
    const rows = await fetchAllRows(backendTiruan(3));
    expect(Array.isArray(rows)).toBe(true);
    expect(rows).toEqual([0, 1, 2]);
  });

  it('tidak terpengaruh default 25 milik backend', async () => {
    // Inilah inti bugnya: tanpa per_page eksplisit, satu permintaan hanya 25 baris.
    const backend = backendTiruan(485);
    const satuHalaman = await backend(1, undefined);
    expect(satuHalaman.data).toHaveLength(25);

    const semua = await fetchAllRows(backendTiruan(485));
    expect(semua).toHaveLength(485);
  });

  it('menghormati batas maxPages agar tidak membanjiri backend', async () => {
    const backend = backendTiruan(10_000);
    const rows = await fetchAllRows(backend, { maxPages: 3 });

    expect(backend).toHaveBeenCalledTimes(3);
    expect(rows).toHaveLength(300);
  });

  it('menangani hasil kosong tanpa permintaan tambahan', async () => {
    const backend = backendTiruan(0);
    const rows = await fetchAllRows(backend);

    expect(rows).toEqual([]);
    expect(backend).toHaveBeenCalledTimes(1);
  });
});
