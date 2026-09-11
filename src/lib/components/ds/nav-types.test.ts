import { describe, expect, it } from 'vitest';
import { activeRailHref } from './nav-types';

const HREFS = [
  '/v2/admin',
  '/v2/admin/customers',
  '/v2/admin/invoices',
  '/v2/admin/invoices/collection',
  '/v2/admin/network/noc',
  '/v2/admin/settings/profile',
];

describe('activeRailHref', () => {
  it('hanya SATU item aktif: Beranda tidak menang di sub-route', () => {
    expect(activeRailHref(HREFS, '/v2/admin/customers')).toBe('/v2/admin/customers');
    expect(activeRailHref(HREFS, '/v2/admin/invoices/collection')).toBe(
      '/v2/admin/invoices/collection',
    );
    expect(activeRailHref(HREFS, '/v2/admin/network/noc/wallboard')).toBe('/v2/admin/network/noc');
  });

  it('cocok persis utk root', () => {
    expect(activeRailHref(HREFS, '/v2/admin')).toBe('/v2/admin');
  });

  it('halaman tanpa padanan menu -> fallback root Beranda (bukan item lain)', () => {
    expect(activeRailHref(HREFS, '/v2/admin/email-outbox/123')).toBe('/v2/admin');
  });

  it('prefix mirip tapi bukan anak (invoice vs invoices) tidak tersorot', () => {
    expect(activeRailHref(['/v2/admin/invoice'], '/v2/admin/invoices')).toBeNull();
  });
});
