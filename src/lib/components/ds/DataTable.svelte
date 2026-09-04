<!--
  Ds/DataTable — tabel data standar.

  Menyatukan pola yang sebelumnya ditulis ulang di 7+ halaman tabel:
  header sticky, kolom angka rata kanan + tabular-nums, skeleton saat loading,
  empty state, dan pembungkus scroll horizontal supaya tabel lebar tidak
  merusak layout halaman (masalah overflow 1.114px sebelumnya).
-->
<script lang="ts" generics="TRow extends object">
  import type { Snippet } from 'svelte';
  import TableSkeleton from './TableSkeleton.svelte';
  import Icon from './Icon.svelte';
  import type { Column } from './table-types';

  interface Props {
    columns: Column[];
    rows: TRow[];
    loading?: boolean;
    mode?: 'light' | 'dark';
    /** Tinggi maksimum area scroll; header tetap sticky. */
    maxHeight?: string;
    emptyTitle?: string;
    emptyHint?: string;
    /** Baris ringkasan di bawah tabel. */
    footNote?: string;
    /** Render sel kustom: dipanggil dengan (row, column). */
    cell?: Snippet<[TRow, Column]>;
  }

  let {
    columns,
    rows,
    loading = false,
    mode = 'light',
    maxHeight,
    emptyTitle = 'Belum ada data',
    emptyHint = 'Coba ubah filter atau rentang tanggal.',
    footNote,
    cell,
  }: Props = $props();

  const dark = $derived(mode === 'dark');
</script>

<div class="overflow-hidden rounded-xl {dark ? 'ring-1 ring-inset ring-white/8' : 'ring-1 ring-inset ring-ink-200'}">
  <div class="overflow-auto" style={maxHeight ? `max-height:${maxHeight}` : undefined}>
    <table class="w-full border-collapse text-base">
      <thead class="sticky top-0 z-10">
        <tr class={dark ? 'bg-night-100' : 'bg-ink-50'}>
          {#each columns as c}
            <th
              class="border-b px-4 py-2.5 text-xs font-semibold tracking-wide whitespace-nowrap uppercase
                {c.align === 'right' ? 'text-right' : 'text-left'}
                {c.hideSm ? 'hidden md:table-cell' : ''}
                {dark ? 'border-white/8 text-slate-400' : 'border-ink-200 text-ink-500'}"
              style={c.width ? `width:${c.width}` : undefined}
            >
              {c.label}
            </th>
          {/each}
        </tr>
      </thead>

      {#if !loading}
        <tbody>
          {#each rows as row}
            <tr class={dark ? 'hover:bg-white/[0.04]' : 'hover:bg-ink-50/70'}>
              {#each columns as c}
                <td
                  class="border-b px-4 py-2.5 align-middle
                    {c.align === 'right' ? 'text-right' : 'text-left'}
                    {c.num ? 'num' : ''}
                    {c.hideSm ? 'hidden md:table-cell' : ''}
                    {dark ? 'border-white/6 text-slate-200' : 'border-ink-100 text-ink-700'}"
                >
                  {#if cell}
                    {@render cell(row, c)}
                  {:else}
                    <!-- `TRow extends object` (bukan Record) supaya interface
                         seperti RouterLike bisa dipakai: TS tidak memberi index
                         signature implisit ke interface. Cast hanya di sini. -->
                    {(row as Record<string, unknown>)[c.key] ?? '—'}
                  {/if}
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      {/if}
    </table>

    {#if loading}
      <TableSkeleton rows={8} cols={columns.length} {mode} />
    {:else if rows.length === 0}
      <div class="flex flex-col items-center gap-2 px-4 py-14 text-center">
        <Icon name="inbox" size={26} class={dark ? 'text-slate-600' : 'text-ink-300'} />
        <div class="text-base font-medium {dark ? 'text-slate-300' : 'text-ink-700'}">
          {emptyTitle}
        </div>
        <div class="text-sm {dark ? 'text-slate-400' : 'text-ink-500'}">{emptyHint}</div>
      </div>
    {/if}
  </div>

  {#if footNote && !loading}
    <div
      class="border-t px-4 py-2 text-sm {dark
        ? 'border-white/8 bg-night-100 text-slate-400'
        : 'border-ink-200 bg-ink-50 text-ink-500'}"
    >
      {footNote}
    </div>
  {/if}
</div>
