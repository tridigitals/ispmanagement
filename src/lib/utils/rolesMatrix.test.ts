import { describe, expect, it } from 'vitest';
import {
  actionLabel,
  canDeleteRole,
  canEditRole,
  groupCoverage,
  groupPermissions,
  levelTone,
  permissionKey,
  resourceLabel,
  summarizeRoles,
  type PermissionLike,
  type RoleLike,
} from './rolesMatrix';

/* Sampel resource nyata dari DB produksi (34 resource, 83 baris permissions).
   Yang penting di sini: storage_console/storage_files/storage adalah tiga resource
   berbeda yang harus tampil sebagai SATU grup, dan communication_templates bukan
   'communication'. */
const perms: PermissionLike[] = [
  { resource: 'customers', action: 'read', description: 'Lihat pelanggan' },
  { resource: 'customers', action: 'manage' },
  { resource: 'customers', action: 'read_own' },
  { resource: 'billing', action: 'read' },
  { resource: 'billing', action: 'manage' },
  { resource: 'storage', action: 'read' },
  { resource: 'storage_console', action: 'read' },
  { resource: 'storage_files', action: 'upload' },
  { resource: 'communication_templates', action: 'read' },
  { resource: 'network_routers', action: 'manage_radius_secret' },
  { resource: 'roles', action: 'update' },
  { resource: 'settings', action: 'update' },
  { resource: 'audit_logs', action: 'read' },
  { resource: 'zzz_unknown_module', action: 'read' },
];

describe('groupPermissions', () => {
  it('menggabungkan tiga resource penyimpanan ke satu grup', () => {
    const g = groupPermissions(perms).find((x) => x.key === 'penyimpanan');
    expect(g?.label).toBe('Penyimpanan');
    expect(g?.resources.sort()).toEqual(['storage', 'storage_console', 'storage_files']);
    expect(g?.items).toHaveLength(3);
  });

  it('tidak pernah menjatuhkan resource yang belum dipetakan', () => {
    const groups = groupPermissions(perms);
    const lainnya = groups.find((x) => x.key === 'lainnya');
    expect(lainnya?.resources).toEqual(['zzz_unknown_module']);

    // Total item harus sama dengan jumlah izin yang masuk — tidak ada yang hilang.
    const totalItems = groups.reduce((n, g) => n + g.items.length, 0);
    expect(totalItems).toBe(perms.length);
  });

  it('menaruh pekerjaan harian sebelum administrasi sistem', () => {
    const keys = groupPermissions(perms).map((g) => g.key);
    expect(keys.indexOf('pelanggan')).toBeLessThan(keys.indexOf('sistem'));
    expect(keys.indexOf('tagihan')).toBeLessThan(keys.indexOf('penyimpanan'));
  });

  it('grup kosong tidak dirender', () => {
    const groups = groupPermissions([{ resource: 'customers', action: 'read' }]);
    expect(groups.map((g) => g.key)).toEqual(['pelanggan']);
  });
});

describe('label', () => {
  it('menerjemahkan aksi ke bahasa Indonesia', () => {
    expect(actionLabel('read')).toBe('Lihat');
    expect(actionLabel('manage')).toBe('Kelola penuh');
    expect(actionLabel('manage_radius_secret')).toBe('Kelola secret RADIUS');
  });

  it('aksi tak dikenal tetap terbaca, bukan kosong', () => {
    expect(actionLabel('force_sync')).toBe('force sync');
  });

  it('resourceLabel merapikan garis bawah', () => {
    expect(resourceLabel('olt_onu_history')).toBe('Olt onu history');
  });

  it('permissionKey memakai format resource:action yang sama dengan can()', () => {
    expect(permissionKey({ resource: 'billing', action: 'manage' })).toBe('billing:manage');
  });
});

describe('groupCoverage', () => {
  it('menghitung izin yang dimiliki per grup', () => {
    const g = groupPermissions(perms).find((x) => x.key === 'tagihan')!;
    expect(groupCoverage(g, ['billing:read'])).toEqual({ granted: 1, total: 2 });
    expect(groupCoverage(g, new Set(['billing:read', 'billing:manage']))).toEqual({
      granted: 2,
      total: 2,
    });
    expect(groupCoverage(g, [])).toEqual({ granted: 0, total: 2 });
  });
});

const systemOwner: RoleLike = { id: 'r1', name: 'Owner', level: 100, is_system: true };
const systemAdmin: RoleLike = { id: 'r2', name: 'Admin', level: 50, is_system: true };
const customRole: RoleLike = { id: 'r3', name: 'Dispatcher', level: 15, is_system: false };

describe('canEditRole', () => {
  it('menolak role sistem untuk non-super-admin', () => {
    // Sembilan role di DB produksi semuanya is_system=true, jadi jalur ini yang
    // sebenarnya dipakai — bukan kasus tepi.
    const r = canEditRole(systemAdmin, { level: 100, canUpdate: true });
    expect(r.allowed).toBe(false);
    expect(r.reason).toMatch(/Super Admin/);
  });

  it('mengizinkan super admin menyunting role sistem', () => {
    expect(canEditRole(systemAdmin, { level: 100, isSuperAdmin: true, canUpdate: true }).allowed).toBe(
      true
    );
  });

  it('menolak role dengan level sama atau lebih tinggi', () => {
    expect(canEditRole(customRole, { level: 15, canUpdate: true }).allowed).toBe(false);
    expect(canEditRole(customRole, { level: 50, canUpdate: true }).allowed).toBe(true);
  });

  it('menolak kalau izin roles:update tidak ada', () => {
    const r = canEditRole(customRole, { level: 100, canUpdate: false });
    expect(r.allowed).toBe(false);
    expect(r.reason).toMatch(/izin/);
  });
});

describe('canDeleteRole', () => {
  it('menolak role yang masih dipakai anggota dan menyebut jumlahnya', () => {
    const r = canDeleteRole(customRole, { level: 100, isSuperAdmin: true, canDelete: true }, 3);
    expect(r.allowed).toBe(false);
    expect(r.reason).toContain('3');
  });

  it('mengizinkan hapus role kustom yang tidak dipakai', () => {
    expect(canDeleteRole(customRole, { level: 50, canDelete: true }, 0).allowed).toBe(true);
  });

  it('role sistem tetap ditolak walau tanpa anggota', () => {
    expect(canDeleteRole(systemOwner, { level: 100, canDelete: true }, 0).allowed).toBe(false);
  });
});

describe('levelTone', () => {
  it('memetakan tangga level seed_roles', () => {
    expect(levelTone(100)).toBe('negative');
    expect(levelTone(50)).toBe('warning');
    expect(levelTone(25)).toBe('info');
    expect(levelTone(0)).toBe('neutral');
  });
});

describe('summarizeRoles', () => {
  it('memisahkan role sistem dari kustom dan menandai yang tak terpakai', () => {
    // Angka produksi: 9 role, SEMUANYA sistem, 6 keanggotaan aktif.
    const roles: RoleLike[] = [systemOwner, systemAdmin, customRole];
    const s = summarizeRoles(roles, { r1: 6, r2: 0, r3: 0 });
    expect(s).toEqual({ total: 3, system: 2, custom: 1, unused: 2, totalAssigned: 6 });
  });

  it('tanpa data keanggotaan semua role dihitung tak terpakai', () => {
    expect(summarizeRoles([systemOwner]).unused).toBe(1);
  });
});
