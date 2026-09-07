import { describe, expect, it } from 'vitest';
import { buildPortalNav, buildPortalNavGroups } from './portalNav';

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

describe('buildPortalNavGroups', () => {
  it('satu grup "Menu" berisi hasil buildPortalNav', () => {
    const groups = buildPortalNavGroups(allowAll);
    expect(groups).toHaveLength(1);
    expect(groups[0].title).toBe('Menu');
    expect(groups[0].items.map((i) => i.href)).toEqual(
      buildPortalNav(allowAll).map((i) => i.href),
    );
  });

  it('izin minim → 4 item di grup tunggal', () => {
    const groups = buildPortalNavGroups(denyAll);
    expect(groups[0].items).toHaveLength(4);
  });
});
