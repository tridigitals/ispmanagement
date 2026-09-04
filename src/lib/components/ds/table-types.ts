/**
 * Tipe tabel design system.
 *
 * Dipisahkan dari `DataTable.svelte` karena komponen itu memakai atribut
 * `generics=`. svelte2tsx membungkus script generik ke dalam sebuah fungsi,
 * jadi `export interface` di dalamnya tidak bisa diimpor dari luar
 * ("has no exported member 'Column'"). Pola yang sama sudah dipakai
 * `nav-types.ts` untuk RailItem/RailGroup.
 */

export interface Column {
  key: string;
  label: string;
  align?: 'left' | 'right';
  /** Set true untuk kolom angka: mono + tabular-nums. */
  num?: boolean;
  /** Sembunyikan di bawah breakpoint md. */
  hideSm?: boolean;
  width?: string;
}
