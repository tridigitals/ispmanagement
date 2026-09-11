/**
 * Tipe navigasi dipisah dari komponen Svelte.
 *
 * TypeScript tidak bisa mengimpor `export interface` dari file .svelte, jadi
 * tipe rail hidup di sini dan dipakai bersama oleh NavRail.svelte serta
 * utils/navConfig.ts.
 */

import type { IconName } from './icons';

export interface RailItem {
  label: string;
  icon: IconName;
  href: string;
  /** Angka kecil di kanan label, contoh jumlah invoice tertunggak. */
  badge?: number | string;
}

export interface RailGroup {
  title: string;
  items: RailItem[];
}

/**
 * Satu-satunya item nav yang boleh dianggap aktif untuk `current`.
 *
 * Aturan: cocokkan persis, ATAU cocokkan prefix terpanjang. Dulu NavRail
 * menandai semua item yang jadi prefix path, sehingga 'Beranda' (/v2/admin)
 * ikut nyala di seluruh sub-route, dan 'Tagihan' nyala berbarengan dengan
 * 'Penagihan' di /v2/admin/invoices/collection.
 */
export function activeRailHref(
  hrefs: readonly string[],
  current: string,
): string | null {
  let best: string | null = null;
  for (const href of hrefs) {
    const exact = href === current;
    const prefix = current.startsWith(href + '/');
    if (!exact && !prefix) continue;
    if (best === null || href.length > best.length) best = href;
  }
  return best;
}
