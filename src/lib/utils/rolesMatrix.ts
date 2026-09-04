/**
 * KENAPA INI ADA
 *
 * Halaman lama `(app)/admin/roles/+page.svelte` (977 baris) menampilkan matriks izin
 * dengan tiga masalah yang terbukti di data produksi:
 *
 * 1. Grup izin dibangun dari `permission.resource` mentah, sehingga daftar menampilkan
 *    34 resource teknis (`storage_console`, `storage_files`, `ppp_profiles`,
 *    `router_inventory`, `olt_onu_history`, ...) sebagai judul kolom. Operator tidak bisa
 *    menebak bahwa "storage_files" dan "storage_console" adalah satu fitur.
 *
 * 2. Kategori diurutkan alfabetis, jadi `admin` dan `audit_logs` muncul sebelum
 *    `customers`/`billing` — izin yang paling sering diubah terkubur di bawah.
 *
 * 3. Tidak ada indikator apa pun bahwa role adalah role sistem (`is_system = true`).
 *    Sembilan role di DB ini SEMUANYA sistem, jadi setiap penyuntingan oleh non-super-admin
 *    ditolak backend — tapi UI baru memberi tahu setelah tombol Simpan gagal.
 *
 * Helper ini murni (tanpa Svelte, tanpa fetch) supaya aturannya bisa dites tanpa DOM.
 * Lihat `rolesMatrix.test.ts`.
 */

export interface PermissionLike {
  id?: string | null;
  resource: string;
  action: string;
  description?: string | null;
}

export interface RoleLike {
  id: string;
  name: string;
  description?: string | null;
  level: number;
  is_system?: boolean | null;
  tenant_id?: string | null;
  permissions?: string[] | null;
}

export interface PermissionGroup {
  /** Kunci stabil untuk `{#each}` dan state buka/tutup. */
  key: string;
  /** Label bahasa Indonesia yang dibaca operator. */
  label: string;
  /** Resource teknis yang digabung ke grup ini. */
  resources: string[];
  items: PermissionEntry[];
}

export interface PermissionEntry {
  key: string;
  resource: string;
  action: string;
  label: string;
  description: string | null;
}

/**
 * Peta resource DB → grup yang dilihat operator, dalam urutan yang dipakai UI.
 *
 * Urutan array ini SENGAJA bukan alfabetis: pekerjaan harian (pelanggan, tagihan,
 * layanan) di atas, administrasi sistem (audit, penyimpanan, admin) di bawah.
 * Resource yang tidak terdaftar di sini tetap muncul lewat grup "Lainnya" — daftar
 * ini tidak boleh menyembunyikan izin yang ada di backend.
 */
const GROUPS: { key: string; label: string; resources: string[] }[] = [
  { key: 'pelanggan', label: 'Pelanggan', resources: ['customers', 'customer_locations', 'orders'] },
  { key: 'tagihan', label: 'Tagihan & Paket', resources: ['billing', 'isp_packages'] },
  { key: 'dukungan', label: 'Dukungan & Pekerjaan', resources: ['support', 'work_orders'] },
  {
    key: 'jaringan',
    label: 'Jaringan',
    resources: [
      'network_noc',
      'network_routers',
      'router_inventory',
      'network_topology',
      'network_incidents',
      'network_alerts',
      'network_logs',
      'coverage',
      'service_zones',
      'ftth_assets',
      'olt',
      'olt_onu_history',
      'dhcp_static',
    ],
  },
  { key: 'pppoe', label: 'PPPoE & IP', resources: ['pppoe', 'ppp_profiles', 'ip_pools'] },
  {
    key: 'komunikasi',
    label: 'Komunikasi',
    resources: ['communication_templates', 'announcements', 'email_outbox'],
  },
  { key: 'tim', label: 'Tim & Akses', resources: ['team', 'roles'] },
  {
    key: 'sistem',
    label: 'Sistem',
    resources: ['settings', 'dashboard', 'audit_logs', 'backups', 'admin'],
  },
  {
    key: 'penyimpanan',
    label: 'Penyimpanan',
    resources: ['storage', 'storage_console', 'storage_files'],
  },
];

const ACTION_LABELS: Record<string, string> = {
  read: 'Lihat',
  read_own: 'Lihat milik sendiri',
  create: 'Tambah',
  update: 'Ubah',
  delete: 'Hapus',
  manage: 'Kelola penuh',
  upload: 'Unggah',
  manage_radius_secret: 'Kelola secret RADIUS',
};

export function permissionKey(p: PermissionLike): string {
  return `${p.resource}:${p.action}`;
}

export function actionLabel(action: string): string {
  return ACTION_LABELS[action] ?? action.replace(/_/g, ' ');
}

/** Nama resource jadi label yang bisa dibaca kalau tidak ada di peta grup. */
export function resourceLabel(resource: string): string {
  return resource.replace(/_/g, ' ').replace(/^\w/, (c) => c.toUpperCase());
}

/**
 * Susun izin ke grup berlabel. Resource yang tidak dikenal masuk ke grup "Lainnya"
 * supaya tidak ada izin yang hilang dari layar hanya karena peta di atas belum diperbarui.
 */
export function groupPermissions(permissions: PermissionLike[]): PermissionGroup[] {
  const byResource = new Map<string, PermissionEntry[]>();

  for (const p of permissions) {
    const entry: PermissionEntry = {
      key: permissionKey(p),
      resource: p.resource,
      action: p.action,
      label: actionLabel(p.action),
      description: p.description ?? null,
    };
    const list = byResource.get(p.resource);
    if (list) list.push(entry);
    else byResource.set(p.resource, [entry]);
  }

  const used = new Set<string>();
  const groups: PermissionGroup[] = [];

  for (const g of GROUPS) {
    const items: PermissionEntry[] = [];
    const resources: string[] = [];
    for (const r of g.resources) {
      const found = byResource.get(r);
      if (!found) continue;
      used.add(r);
      resources.push(r);
      items.push(...found);
    }
    if (items.length > 0) groups.push({ key: g.key, label: g.label, resources, items });
  }

  const leftovers = [...byResource.keys()].filter((r) => !used.has(r)).sort();
  if (leftovers.length > 0) {
    groups.push({
      key: 'lainnya',
      label: 'Lainnya',
      resources: leftovers,
      items: leftovers.flatMap((r) => byResource.get(r) ?? []),
    });
  }

  return groups;
}

/** Jumlah izin dalam grup yang dimiliki role. Dipakai untuk badge "3/8". */
export function groupCoverage(
  group: PermissionGroup,
  granted: Iterable<string>
): { granted: number; total: number } {
  const set = granted instanceof Set ? granted : new Set(granted);
  let n = 0;
  for (const item of group.items) if (set.has(item.key)) n += 1;
  return { granted: n, total: group.items.length };
}

/**
 * Bisakah aktor menyunting role ini?
 *
 * Mencerminkan `role_service.rs`: role sistem hanya boleh diubah super admin. Selain itu
 * aktor tidak boleh menyunting role yang levelnya setara atau di atas levelnya sendiri —
 * kalau tidak, Admin (level 50) bisa memberi dirinya izin Owner (level 100) lewat role lain.
 */
export function canEditRole(
  role: RoleLike,
  actor: { level: number; isSuperAdmin?: boolean; canUpdate: boolean }
): { allowed: boolean; reason?: string } {
  if (!actor.canUpdate) return { allowed: false, reason: 'Tidak punya izin mengubah role' };
  if (role.is_system && !actor.isSuperAdmin)
    return { allowed: false, reason: 'Role sistem hanya bisa diubah Super Admin' };
  if (!actor.isSuperAdmin && role.level >= actor.level)
    return { allowed: false, reason: 'Level role sama atau lebih tinggi dari level Anda' };
  return { allowed: true };
}

/**
 * Bisakah role dihapus?
 *
 * `tenant_members.role_id` tidak punya ON DELETE, jadi menghapus role yang masih dipakai
 * memicu foreign key violation. Backend kini mengembalikan 409 dengan pesan jelas
 * (`delete_role`), dan layar menonaktifkan tombolnya lebih dulu.
 */
export function canDeleteRole(
  role: RoleLike,
  actor: { level: number; isSuperAdmin?: boolean; canDelete: boolean },
  memberCount: number
): { allowed: boolean; reason?: string } {
  if (!actor.canDelete) return { allowed: false, reason: 'Tidak punya izin menghapus role' };
  if (role.is_system && !actor.isSuperAdmin)
    return { allowed: false, reason: 'Role sistem hanya bisa dihapus Super Admin' };
  if (memberCount > 0)
    return {
      allowed: false,
      reason: `Masih dipakai ${memberCount} anggota — pindahkan dulu ke role lain`,
    };
  if (!actor.isSuperAdmin && role.level >= actor.level)
    return { allowed: false, reason: 'Level role sama atau lebih tinggi dari level Anda' };
  return { allowed: true };
}

/**
 * Warna badge menurut level, mengikuti tangga level di seed_roles().
 * Nilainya harus anggota `StatusTone` design system ('negative', bukan 'danger') —
 * Badge menolak nama lain saat type-check.
 */
export function levelTone(level: number): 'negative' | 'warning' | 'info' | 'neutral' {
  if (level >= 100) return 'negative';
  if (level >= 50) return 'warning';
  if (level >= 20) return 'info';
  return 'neutral';
}

export function summarizeRoles(
  roles: RoleLike[],
  memberCounts: Record<string, number> = {}
): { total: number; system: number; custom: number; unused: number; totalAssigned: number } {
  let system = 0;
  let unused = 0;
  let totalAssigned = 0;

  for (const r of roles) {
    if (r.is_system) system += 1;
    const n = memberCounts[r.id] ?? 0;
    totalAssigned += n;
    if (n === 0) unused += 1;
  }

  return { total: roles.length, system, custom: roles.length - system, unused, totalAssigned };
}
