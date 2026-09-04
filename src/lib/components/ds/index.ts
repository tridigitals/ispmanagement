/**
 * Titik masuk tunggal design system.
 *
 * Halaman mengimpor dari '$lib/components/ds' saja, bukan per file, supaya
 * refactor internal tidak menyentuh halaman.
 */

export { default as Icon } from './Icon.svelte';
export { default as Badge } from './Badge.svelte';
export { default as Button } from './Button.svelte';
export { default as Card } from './Card.svelte';
export { default as PageHeader } from './PageHeader.svelte';
export { default as StatTile } from './StatTile.svelte';
export { default as TableSkeleton } from './TableSkeleton.svelte';
export { default as RowActions } from './RowActions.svelte';
export { default as AttentionPanel } from './AttentionPanel.svelte';
export { default as DataTable } from './DataTable.svelte';
export { default as Tabs } from './Tabs.svelte';
export { default as DetailHeader } from './DetailHeader.svelte';
export { default as FieldRow } from './FieldRow.svelte';
export { default as Field } from './Field.svelte';
export { default as SaveBar } from './SaveBar.svelte';
export { default as NavRail } from './NavRail.svelte';
export { default as Topbar } from './Topbar.svelte';
export { default as AppShell } from './AppShell.svelte';

export type { RailItem, RailGroup } from './nav-types';
export type { FieldType, FieldOption } from './Field.svelte';
export type { Column } from './table-types';

export { icons, type IconName } from './icons';
export {
  formatRupiah,
  formatCompactRupiah,
  formatDate,
  formatRelative,
  formatPercent,
} from './format';
export { badgeClass, toneOf, toneDot, type StatusTone } from './tokens';
