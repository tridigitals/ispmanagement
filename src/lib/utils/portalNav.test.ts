import { describe, expect, it } from 'vitest';
import { buildPortalNav } from './portalNav';

const allowAll = () => true;
const denyAll = () => false;

describe('buildPortalNav', () => {
  it('6 item + href /v2 bila izin penuh', () => {
    const nav = buildPortalNav(allowAll);
    expect(nav).toHaveLength(6);
    expect(nav.map((i) => i.href)).toEqual([
      '/v2/dashboard',
      '/v2/dashboard/locations',
      '/v2/dashboard/services',
      '/v2/dashboard/invoices',
      '/v2/announcements',
      '/v2/support',
    ]);
  });

  it('tanpa Layanan/Bantuan bila izin minim', () => {
    const nav = buildPortalNav(denyAll);
    expect(nav.map((i) => i.label)).toEqual(['Beranda', 'Lokasi', 'Tagihan', 'Pengumuman']);
  });
});
