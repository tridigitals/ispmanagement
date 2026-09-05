import { describe, expect, it } from 'vitest';
import { buildAdminNav, v2Href, V2_MIGRATED } from './navConfig';
import { badgeClass, toneOf } from '$lib/components/ds/tokens';

/** Checker izin yang hanya mengizinkan pasangan yang didaftarkan. */
function allow(...pairs: string[]) {
  const set = new Set(pairs);
  return (action: string, resource: string) => set.has(`${action}:${resource}`);
}

const owner = { role: 'owner', tenant_role: 'owner', is_super_admin: false };

describe('buildAdminNav', () => {
  it('tidak pernah mengembalikan grup kosong', () => {
    const groups = buildAdminNav(allow(), null);
    for (const g of groups) {
      expect(g.items.length).toBeGreaterThan(0);
    }
  });

  it('menyembunyikan Tagihan ketika izin billing tidak ada', () => {
    const groups = buildAdminNav(allow('read:customers'), owner);
    const hrefs = groups.flatMap((g) => g.items.map((i) => i.href));

    expect(hrefs).toContain('/admin/customers');
    expect(hrefs).not.toContain('/admin/invoices');
    expect(hrefs).not.toContain('/admin/billing');
  });

  it('menampilkan Tagihan beserta badge tertunggak ketika izin billing ada', () => {
    const groups = buildAdminNav(allow('read:billing'), owner, { invoicesOverdue: 473 });
    const invoices = groups.flatMap((g) => g.items).find((i) => i.href === '/admin/invoices');

    expect(invoices).toBeDefined();
    expect(invoices?.badge).toBe(473);
  });

  it('menghilangkan badge ketika hitungannya nol, bukan menampilkan angka 0', () => {
    const groups = buildAdminNav(allow('read:billing'), owner, { invoicesOverdue: 0 });
    const invoices = groups.flatMap((g) => g.items).find((i) => i.href === '/admin/invoices');

    expect(invoices?.badge).toBeUndefined();
  });

  it('hanya menampilkan Paket untuk peran yang boleh mengakses katalog layanan', () => {
    const can = allow('read:isp_packages');

    const asOwner = buildAdminNav(can, owner)
      .flatMap((g) => g.items)
      .map((i) => i.href);
    const asCustomer = buildAdminNav(can, { role: 'customer', tenant_role: 'customer' })
      .flatMap((g) => g.items)
      .map((i) => i.href);

    expect(asOwner).toContain('/admin/services');
    expect(asCustomer).not.toContain('/admin/services');
  });

  it('selalu menyertakan beranda admin', () => {
    const hrefs = buildAdminNav(allow(), null).flatMap((g) => g.items.map((i) => i.href));
    expect(hrefs).toContain('/admin');
  });

  it('tidak menghasilkan href duplikat', () => {
    const hrefs = buildAdminNav(
      allow(
        'read:billing',
        'read:customers',
        'read:network_noc',
        'read:network_incidents',
        'read:pppoe',
        'read:router_inventory',
        'read:work_orders',
        'read:support',
        'read:team',
        'read:roles',
        'read:audit_logs',
        'read:settings',
        'read:isp_packages',
        'read:network_topology',
      ),
      owner,
    ).flatMap((g) => g.items.map((i) => i.href));

    expect(new Set(hrefs).size).toBe(hrefs.length);
  });
});

describe('pemetaan href v2 selama migrasi bertahap', () => {
  const all = allow(
    'read:billing',
    'read:customers',
    'read:network_noc',
    'read:pppoe',
    'read:roles',
    'read:router_inventory',
    'read:settings',
    'read:support',
    'read:team',
    'read:audit_logs',
    'manage:announcements',
    'read:isp_packages',
    'manage:isp_packages',
    'read:email_outbox',
  );

  it('mengarahkan halaman yang sudah dimigrasi ke /v2 saat opsi v2 aktif', () => {
    const hrefs = buildAdminNav(all, owner, {}, { v2: true }).flatMap((g) =>
      g.items.map((i) => i.href),
    );

    expect(hrefs).toContain('/v2/admin');
    expect(hrefs).toContain('/v2/admin/customers');
    expect(hrefs).toContain('/v2/admin/invoices');
    expect(hrefs).toContain('/v2/admin/network/pppoe');
    expect(hrefs).toContain('/v2/admin/settings');
    expect(hrefs).toContain('/v2/admin/network/routers');
    expect(hrefs).toContain('/v2/admin/support');
    expect(hrefs).toContain('/v2/admin/team');
    expect(hrefs).toContain('/v2/admin/network/olts');
    expect(hrefs).toContain('/v2/admin/email-outbox');
  });

  it('membiarkan halaman yang BELUM dimigrasi tetap menunjuk ke rute lama', () => {
    const hrefs = buildAdminNav(all, owner, {}, { v2: true }).flatMap((g) =>
      g.items.map((i) => i.href),
    );

    // Belum ada versi v2-nya: harus tetap legacy supaya tidak 404.
    expect(hrefs).toContain('/admin/invoices/collection');
    expect(hrefs).not.toContain('/v2/admin/invoices/collection');
    expect(hrefs).toContain('/admin/network/noc');
    expect(hrefs).not.toContain('/v2/admin/network/noc');
  });

  it('tidak mengubah href sama sekali di shell lama (opsi v2 mati)', () => {
    const hrefs = buildAdminNav(all, owner).flatMap((g) => g.items.map((i) => i.href));

    expect(hrefs).toContain('/admin/invoices');
    expect(hrefs.some((h) => h.startsWith('/v2'))).toBe(false);
  });

  it('v2Href hanya menyentuh path yang terdaftar', () => {
    expect(v2Href('/admin/invoices')).toBe('/v2/admin/invoices');
    expect(v2Href('/admin/team')).toBe('/v2/admin/team');
    expect(v2Href('/admin/roles')).toBe('/v2/admin/roles');
    expect(v2Href('/admin/billing')).toBe('/v2/admin/billing');
    expect(v2Href('/admin/announcements')).toBe('/v2/admin/announcements');
    expect(v2Href('/admin/audit-logs')).toBe('/v2/admin/audit-logs');
    expect(v2Href('/admin/services')).toBe('/v2/admin/services');
    expect(v2Href('/admin/network/olts')).toBe('/v2/admin/network/olts');
    // Rute yang BELUM dimigrasi harus lewat tanpa perubahan.
    expect(v2Href('/admin/olts')).toBe('/admin/olts');
    // Tidak boleh menumpuk prefix kalau dipanggil dua kali.
    expect(v2Href(v2Href('/admin/invoices'))).toBe('/v2/admin/invoices');
  });

  it('setiap entri V2_MIGRATED memang muncul di nav (mencegah daftar basi)', () => {
    const legacy = buildAdminNav(all, owner).flatMap((g) => g.items.map((i) => i.href));
    for (const path of V2_MIGRATED) {
      expect(legacy).toContain(path);
    }
  });
});

describe('token status', () => {
  it('memetakan status domain ISP ke tone yang benar', () => {
    expect(toneOf('paid')).toBe('positive');
    expect(toneOf('suspended')).toBe('negative');
    expect(toneOf('pending')).toBe('warning');
    expect(toneOf('PENDING')).toBe('warning');
    expect(toneOf('pending_installation')).toBe('info');
  });

  it('mengembalikan neutral untuk status kosong atau tak dikenal', () => {
    expect(toneOf(null)).toBe('neutral');
    expect(toneOf('')).toBe('neutral');
    expect(toneOf('status-yang-belum-ada')).toBe('neutral');
  });

  it('badge selalu punya warna teks eksplisit, tidak mewarisi', () => {
    for (const tone of ['positive', 'negative', 'warning', 'info', 'neutral'] as const) {
      expect(badgeClass(tone, 'light')).toMatch(/text-/);
      expect(badgeClass(tone, 'dark')).toMatch(/text-/);
    }
  });
});
