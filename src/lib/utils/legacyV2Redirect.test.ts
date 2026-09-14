import { describe, expect, it } from 'vitest';

import { v2RedirectFor } from './legacyV2Redirect';

describe('v2RedirectFor (cutover v2)', () => {
  it('memetakan admin utama ke v2', () => {
    expect(v2RedirectFor('/admin')).toBe('/v2/admin');
    expect(v2RedirectFor('/admin/')).toBe('/v2/admin');
    expect(v2RedirectFor('/admin/customers')).toBe('/v2/admin/customers');
    expect(v2RedirectFor('/admin/network/noc/wallboard/settings')).toBe(
      '/v2/admin/network/noc/wallboard/settings',
    );
  });

  it('memetakan parameter halaman detail', () => {
    expect(v2RedirectFor('/admin/customers/abc-123')).toBe('/v2/admin/customers/abc-123');
    expect(v2RedirectFor('/admin/invoices/inv-9')).toBe('/v2/admin/invoices/inv-9');
    expect(v2RedirectFor('/support/tkt-1')).toBe('/v2/support/tkt-1');
    expect(v2RedirectFor('/announcements/ann-2')).toBe('/v2/announcements/ann-2');
  });

  it('memetakan alias lama ke padanan v2', () => {
    expect(v2RedirectFor('/admin/servicers')).toBe('/v2/admin/services');
    expect(v2RedirectFor('/admin/network/packages')).toBe('/v2/admin/services');
    expect(v2RedirectFor('/dashboard/packages')).toBe('/v2/dashboard/services');
    expect(v2RedirectFor('/storage')).toBe('/v2/dashboard');
  });

  it('memetakan portal pelanggan ke v2', () => {
    expect(v2RedirectFor('/dashboard')).toBe('/v2/dashboard');
    expect(v2RedirectFor('/dashboard/services/order/internet')).toBe(
      '/v2/dashboard/services/order/internet',
    );
    expect(v2RedirectFor('/announcements')).toBe('/v2/announcements');
  });

  it('tidak memetakan path tanpa padanan, /v2 itu sendiri, superadmin, dan root', () => {
    expect(v2RedirectFor('/')).toBeNull();
    expect(v2RedirectFor('')).toBeNull();
    expect(v2RedirectFor('/login')).toBeNull();
    expect(v2RedirectFor('/unauthorized')).toBeNull();
    expect(v2RedirectFor('/maintenance')).toBeNull();
    expect(v2RedirectFor('/profile')).toBeNull();
    expect(v2RedirectFor('/v2/admin')).toBeNull();
    expect(v2RedirectFor('/v2/admin/customers/x')).toBeNull();
    expect(v2RedirectFor('/superadmin')).toBeNull();
    expect(v2RedirectFor('/superadmin/tenants')).toBeNull();
    expect(v2RedirectFor('/admin/tidak-ada')).toBeNull();
  });
});

describe('v2RedirectFor — hash panel settings legacy-only', () => {
  it('/admin/settings#<panel> TIDAK dipantul ke v2 (belum dimigrasi)', () => {
    for (const panel of ['branding','billing_plan','email','payment','service','whatsapp','event_notifications']) {
      expect(v2RedirectFor('/admin/settings', `#${panel}`), panel).toBeNull();
    }
    // variasi dengan encoding/whitespace
    expect(v2RedirectFor('/admin/settings', ' %23email ')).toBeNull();
  });
  it('hash tab yang SUDAH ada padanan v2-nya tetap redirect', () => {
    expect(v2RedirectFor('/admin/settings', '#general')).toBe('/v2/admin/settings');
    expect(v2RedirectFor('/admin/settings', '#network')).toBe('/v2/admin/settings');
    expect(v2RedirectFor('/admin/settings', '#storage')).toBe('/v2/admin/settings');
  });
  it('tanpa hash tetap redirect seperti sebelumnya', () => {
    expect(v2RedirectFor('/admin/settings')).toBe('/v2/admin/settings');
    expect(v2RedirectFor('/admin/settings', '')).toBe('/v2/admin/settings');
  });
});
