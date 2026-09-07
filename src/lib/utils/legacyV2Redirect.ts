/**
 * Cutover v2 — peta redirect route lama (app) ke pratinjau v2.
 *
 * Sengaja satu utilitas murni: diinjeksi di (app)/+layout.svelte SATU tempat
 * (bukan +layout.ts per route), jadi tidak ada 40 file tambahan. Rollback =
 * hapus injeksi satu blok di layout + revert utilitas ini.
 *
 * Path dibandingkan sebagai path TEPAT (bukan prefix), kecuali aturan
 * parameter di bawah. `/superadmin/**` sengaja TIDAK dipetakan — superadmin
 * punya area sendiri yang belum dimigrasi.
 */

/**
 * Cari padanan v2 untuk path lama. Return null = tidak ada padanan, biarkan
 * halaman lama dirender (fallback aman).
 */
export function v2RedirectFor(path: string): string | null {
  if (!path || path === '/') return null;
  if (path.startsWith('/v2')) return null; // sudah di v2
  if (path.startsWith('/superadmin')) return null; // area superadmin, tidak dimigrasi

  // Halaman dengan parameter: /admin/customers/[id]/..., /admin/invoices/[id]
  // /admin/announcements/[id], /support/[id], /announcements/[id]
  const paramMap: Array<[RegExp, (rest: string) => string]> = [
    [/^\/admin\/customers\/(.+)$/, (rest) => `/v2/admin/customers/${rest}`],
    [/^\/admin\/invoices\/(.+)$/, (rest) => `/v2/admin/invoices/${rest}`],
    [/^\/admin\/announcements\/(.+)$/, (rest) => `/v2/admin/announcements/${rest}`],
    [/^\/support\/(.+)$/, (rest) => `/v2/support/${rest}`],
    [/^\/announcements\/(.+)$/, (rest) => `/v2/announcements/${rest}`],
  ];
  for (const [re, map] of paramMap) {
    const m = path.match(re);
    if (m) return map(m[1]);
  }

  const table: Record<string, string> = {
    // admin (43 halaman + 2 alias lama yang redirect ke services)
    '/admin': '/v2/admin',
    '/admin/': '/v2/admin',
    '/admin/announcements': '/v2/admin/announcements',
    '/admin/audit-logs': '/v2/admin/audit-logs',
    '/admin/backups': '/v2/admin/backups',
    '/admin/billing': '/v2/admin/billing',
    '/admin/customers': '/v2/admin/customers',
    '/admin/customers/lifecycle-reconciliation': '/v2/admin/customers/lifecycle-reconciliation',
    '/admin/customers/orders/new': '/v2/admin/customers/orders/new',
    '/admin/email-outbox': '/v2/admin/email-outbox',
    '/admin/invoices': '/v2/admin/invoices',
    '/admin/invoices/collection': '/v2/admin/invoices/collection',
    '/admin/message-templates': '/v2/admin/message-templates',
    '/admin/network/alerts': '/v2/admin/network/alerts',
    '/admin/network/assets': '/v2/admin/network/assets',
    '/admin/network/dhcp-static': '/v2/admin/network/dhcp-static',
    '/admin/network/import': '/v2/admin/network/import',
    '/admin/network/import/mixradius': '/v2/admin/network/import/mixradius',
    '/admin/network/incidents': '/v2/admin/network/incidents',
    '/admin/network/installations': '/v2/admin/network/installations',
    '/admin/network/ip-pools': '/v2/admin/network/ip-pools',
    '/admin/network/logs': '/v2/admin/network/logs',
    '/admin/network/map': '/v2/admin/network/map',
    '/admin/network/noc': '/v2/admin/network/noc',
    '/admin/network/noc/wallboard': '/v2/admin/network/noc/wallboard',
    '/admin/network/noc/wallboard/settings': '/v2/admin/network/noc/wallboard/settings',
    '/admin/network/olts': '/v2/admin/network/olts',
    '/admin/network/pppoe': '/v2/admin/network/pppoe',
    '/admin/network/pppoe/import': '/v2/admin/network/pppoe/import',
    '/admin/network/ppp-profiles': '/v2/admin/network/ppp-profiles',
    '/admin/network/routers': '/v2/admin/network/routers',
    '/admin/network/packages': '/v2/admin/services', // alias lama (sudah redirect ke services)
    '/admin/roles': '/v2/admin/roles',
    '/admin/servicers': '/v2/admin/services', // alias lama (sudah redirect ke services)
    '/admin/services': '/v2/admin/services',
    '/admin/settings': '/v2/admin/settings',
    '/admin/storage': '/v2/admin/storage',
    '/admin/subscription': '/v2/admin/subscription',
    '/admin/support': '/v2/admin/support',
    '/admin/team': '/v2/admin/team',

    // portal pelanggan
    '/dashboard': '/v2/dashboard',
    '/dashboard/': '/v2/dashboard',
    '/dashboard/invoices': '/v2/dashboard/invoices',
    '/dashboard/locations': '/v2/dashboard/locations',
    '/dashboard/packages': '/v2/dashboard/services', // alias lama (sudah redirect ke services)
    '/dashboard/services': '/v2/dashboard/services',
    '/dashboard/services/order': '/v2/dashboard/services/order',
    '/dashboard/services/order/dedicated-link': '/v2/dashboard/services/order/dedicated-link',
    '/dashboard/services/order/hotspot': '/v2/dashboard/services/order/hotspot',
    '/dashboard/services/order/internet': '/v2/dashboard/services/order/internet',
    '/dashboard/services/order/vpn': '/v2/dashboard/services/order/vpn',
    '/dashboard/settings': '/v2/dashboard/settings',
    '/dashboard/tickets': '/v2/dashboard/tickets',
    '/announcements': '/v2/announcements',
    '/announcements/': '/v2/announcements',
    '/support': '/v2/support',
    '/support/': '/v2/support',
    '/storage': '/v2/dashboard', // alias lama (sudah redirect ke dashboard)
  };

  return table[path] ?? null;
}
