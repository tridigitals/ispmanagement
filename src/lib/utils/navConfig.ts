/**
 * Konfigurasi navigasi shell v2.
 *
 * Dipisah dari komponen supaya bisa diuji tanpa merender Svelte, dan supaya
 * aturan izinnya persis sama dengan Sidebar lama (Sidebar.svelte:241-501)
 * selama masa transisi. Fungsi ini murni: terima checker izin, kembalikan
 * daftar grup.
 */

import type { RailGroup } from '$lib/components/ds/nav-types';
import { canAccessNetworkMap } from './adminNetworkAccess';
import { canAccessServiceCatalog } from './serviceCatalogAccess';

type Can = (action: string, resource: string) => boolean;

type NavUser = {
  role?: string | null;
  tenant_role?: string | null;
  is_super_admin?: boolean | null;
} | null;

/** Badge dinamis yang ditempel ke item tertentu. */
export type NavBadges = {
  /** Jumlah invoice tertunggak → ditempel ke Tagihan. */
  invoicesOverdue?: number;
  /** Jumlah insiden aktif → ditempel ke Insiden. */
  incidentsActive?: number;
  /** Tiket support terbuka → ditempel ke Support. */
  supportOpen?: number;
};

/**
 * Halaman yang SUDAH dimigrasi ke shell v2.
 *
 * Selama migrasi bertahap, rail di shell v2 harus menunjuk ke versi v2 untuk
 * halaman yang sudah ada, dan tetap menunjuk ke halaman lama untuk sisanya —
 * kalau tidak, pengguna yang mengklik menu akan terlempar keluar dari shell
 * baru tanpa penjelasan, atau lebih buruk, mendarat di 404.
 *
 * Tambahkan path ke daftar ini SETIAP KALI sebuah halaman selesai dimigrasi.
 */
export const V2_MIGRATED: readonly string[] = [
  '/admin',
  '/admin/customers',
  '/admin/invoices',
  '/admin/network/pppoe',
  '/admin/settings',
  '/admin/network/routers',
  '/admin/support',
  '/admin/team',
  '/admin/roles',
  '/admin/billing',
];

/** Ubah href legacy ke padanan v2 bila halaman itu sudah dimigrasi. */
export function v2Href(href: string): string {
  return V2_MIGRATED.includes(href) ? `/v2${href}` : href;
}

export function buildAdminNav(
  can: Can,
  user: NavUser,
  badges: NavBadges = {},
  options: { v2?: boolean } = {},
): RailGroup[] {
  const billing = can('read', 'billing') || can('manage', 'billing');
  const customers = can('read', 'customers') || can('manage', 'customers');
  const workOrders = can('read', 'work_orders') || can('manage', 'work_orders');
  const pppoe = can('read', 'pppoe') || can('manage', 'pppoe');
  const packages = canAccessServiceCatalog(
    user,
    can('read', 'isp_packages'),
    can('manage', 'isp_packages'),
  );

  const groups: RailGroup[] = [
    {
      title: 'Ringkasan',
      items: [
        { label: 'Beranda', icon: 'grid', href: '/admin' },
        ...(customers
          ? [{ label: 'Pelanggan', icon: 'users' as const, href: '/admin/customers' }]
          : []),
      ],
    },
    {
      title: 'Tagihan',
      items: [
        ...(billing
          ? [
              {
                label: 'Tagihan',
                icon: 'receipt' as const,
                href: '/admin/invoices',
                badge: badges.invoicesOverdue || undefined,
              },
              { label: 'Penagihan', icon: 'card' as const, href: '/admin/invoices/collection' },
              { label: 'Analitik', icon: 'chart' as const, href: '/admin/billing' },
            ]
          : []),
        ...(packages ? [{ label: 'Paket', icon: 'box' as const, href: '/admin/services' }] : []),
      ],
    },
    {
      title: 'Jaringan',
      items: [
        ...(can('read', 'network_noc')
          ? [{ label: 'NOC', icon: 'activity' as const, href: '/admin/network/noc' }]
          : []),
        ...(canAccessNetworkMap(can)
          ? [{ label: 'Topologi', icon: 'map' as const, href: '/admin/network/map' }]
          : []),
        ...(can('read', 'network_incidents')
          ? [
              {
                label: 'Insiden',
                icon: 'alert' as const,
                href: '/admin/network/incidents',
                badge: badges.incidentsActive || undefined,
              },
            ]
          : []),
        ...(pppoe ? [{ label: 'PPPoE', icon: 'key' as const, href: '/admin/network/pppoe' }] : []),
        ...(can('read', 'router_inventory')
          ? [{ label: 'Router', icon: 'router' as const, href: '/admin/network/routers' }]
          : []),
        ...(pppoe ? [{ label: 'OLT', icon: 'radio' as const, href: '/admin/network/olts' }] : []),
      ],
    },
    {
      title: 'Lapangan',
      items: [
        ...(workOrders
          ? [
              {
                label: 'Instalasi',
                icon: 'clipboard' as const,
                href: '/admin/network/installations',
              },
            ]
          : []),
        ...(can('read', 'support') || can('read_all', 'support')
          ? [
              {
                label: 'Support',
                icon: 'lifebuoy' as const,
                href: '/admin/support',
                badge: badges.supportOpen || undefined,
              },
            ]
          : []),
      ],
    },
    {
      title: 'Pengaturan',
      items: [
        ...(can('read', 'team') ? [{ label: 'Tim', icon: 'users' as const, href: '/admin/team' }] : []),
        ...(can('read', 'roles') ? [{ label: 'Peran', icon: 'lock' as const, href: '/admin/roles' }] : []),
        ...(can('read', 'audit_logs')
          ? [{ label: 'Audit', icon: 'shield' as const, href: '/admin/audit-logs' }]
          : []),
        ...(can('read', 'settings')
          ? [{ label: 'Setelan', icon: 'cog' as const, href: '/admin/settings' }]
          : []),
      ],
    },
  ];

  // Buang grup kosong supaya rail tidak menampilkan pemisah menggantung.
  const nonEmpty = groups.filter((g) => g.items.length > 0);

  /* Di shell v2, href halaman yang sudah dimigrasi diarahkan ke /v2/... supaya
     navigasi tidak melompat keluar dari shell baru di tengah pekerjaan. */
  if (!options.v2) return nonEmpty;

  return nonEmpty.map((g) => ({
    ...g,
    items: g.items.map((i) => ({ ...i, href: v2Href(i.href) })),
  }));
}
