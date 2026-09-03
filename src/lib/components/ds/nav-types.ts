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
