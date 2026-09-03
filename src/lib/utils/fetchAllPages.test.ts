import { describe, expect, it, vi } from 'vitest';
import { fetchAllPages, type Paginated } from './fetchAllPages';

/** Backend palsu yang meniru clamp(1, 100) di payment_service. */
function fakeBackend(total: number, cap = 100) {
  const calls: Array<{ page: number; perPage: number }> = [];

  const fetchPage = async (page: number, perPage: number): Promise<Paginated<number>> => {
    calls.push({ page, perPage });
    const effective = Math.min(Math.max(perPage, 1), cap);
    const start = (page - 1) * effective;
    const data = Array.from({ length: Math.max(0, Math.min(effective, total - start)) }, (_, i) => start + i);
    return { data, total, page, per_page: effective };
  };

  return { fetchPage, calls };
}

describe('fetchAllPages', () => {
  it('mengambil seluruh baris meski backend memotong per_page jadi 100', async () => {
    const { fetchPage, calls } = fakeBackend(489);

    const res = await fetchAllPages(fetchPage, { perPage: 1000 });

    expect(res.rows.length).toBe(489);
    expect(res.total).toBe(489);
    expect(res.complete).toBe(true);
    expect(calls.length).toBe(5); // ceil(489 / 100)
  });

  it('cukup satu permintaan kalau semua baris masuk halaman pertama', async () => {
    const { fetchPage, calls } = fakeBackend(42);

    const res = await fetchAllPages(fetchPage, { perPage: 100 });

    expect(res.rows.length).toBe(42);
    expect(calls.length).toBe(1);
  });

  it('menandai complete=false ketika maxPages menghentikan pengambilan', async () => {
    const { fetchPage } = fakeBackend(1000);

    const res = await fetchAllPages(fetchPage, { perPage: 100, maxPages: 3 });

    expect(res.rows.length).toBe(300);
    expect(res.total).toBe(1000);
    expect(res.complete).toBe(false);
  });

  it('tidak melakukan permintaan tambahan saat data kosong', async () => {
    const { fetchPage, calls } = fakeBackend(0);

    const res = await fetchAllPages(fetchPage);

    expect(res.rows).toEqual([]);
    expect(calls.length).toBe(1);
  });

  it('berhenti kalau halaman berikutnya balik kosong (mencegah loop tak berujung)', async () => {
    const fetchPage = vi
      .fn<(page: number, perPage: number) => Promise<Paginated<number>>>()
      .mockResolvedValueOnce({ data: [1, 2, 3], total: 99, page: 1, per_page: 3 })
      .mockResolvedValue({ data: [], total: 99, page: 2, per_page: 3 });

    const res = await fetchAllPages(fetchPage);

    expect(res.rows.length).toBe(3);
    expect(res.complete).toBe(false);
    expect(fetchPage).toHaveBeenCalledTimes(2);
  });
});
