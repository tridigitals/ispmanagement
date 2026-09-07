/**
 * Navigasi portal pelanggan v2 (gelombang 25).
 *
 * Cerminan menu portal di Sidebar legacy, tapi murni + prefix /v2
 * agar bisa diuji tanpa merender Svelte.
 */

export type PortalNavItem = {
  label: string;
  icon: 'grid' | 'pin' | 'box' | 'receipt' | 'megaphone' | 'lifebuoy';
  href: string;
};

type Can = (action: string, resource: string) => boolean;

/** Daftar menu portal pelanggan. Urutan = urutan Sidebar legacy. */
export function buildPortalNav(can: Can, v2Prefix = '/v2'): PortalNavItem[] {
  const items: PortalNavItem[] = [
    { label: 'Beranda', icon: 'grid', href: `${v2Prefix}/dashboard` },
    { label: 'Lokasi', icon: 'pin', href: `${v2Prefix}/dashboard/locations` },
  ];
  if (can('read_own', 'customers')) {
    items.push({ label: 'Layanan', icon: 'box', href: `${v2Prefix}/dashboard/services` });
  }
  items.push(
    { label: 'Tagihan', icon: 'receipt', href: `${v2Prefix}/dashboard/invoices` },
    { label: 'Pengumuman', icon: 'megaphone', href: `${v2Prefix}/announcements` },
  );
  if (can('read', 'support') || can('create', 'support')) {
    items.push({ label: 'Bantuan', icon: 'lifebuoy', href: `${v2Prefix}/support` });
  }
  return items;
}

export interface PortalNavGroup {
  title: string;
  items: PortalNavItem[];
}

/**
 * Menu portal dalam bentuk grup RailGroup (dipakai NavRail di PortalShell).
 * Satu grup "Menu" — portal hanya punya 5-6 item, tidak perlu section.
 */
export function buildPortalNavGroups(can: Can, v2Prefix = '/v2'): PortalNavGroup[] {
  return [{ title: 'Menu', items: buildPortalNav(can, v2Prefix) }];
}
