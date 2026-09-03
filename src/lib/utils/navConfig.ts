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

export function buildAdminNav(can: Can, user: NavUser, badges: NavBadges = {}): RailGroup[] {
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
  return groups.filter((g) => g.items.length > 0);
}
