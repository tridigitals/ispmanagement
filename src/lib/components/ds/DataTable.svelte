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
  import { t } from 'svelte-i18n';

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
    /** 0 = nonaktif. >0: pager tampil; rows di-slice internal (mode klien). */
    pageSize?: number;
    pageSizeOptions?: number[];
    /** Mode server: halaman aktif (1-based). Isi bersama `onpage` + `total`. */
    page?: number;
    /** Mode server: bila diisi, rows TIDAK di-slice — parent yang fetch. */
    onpage?: (p: number) => void;
    /** Mode server: dipanggil saat user mengganti ukuran halaman; parent
     *  fetch ulang dengan per_page baru + reset ke halaman 1. */
    onpagesize?: (n: number) => void;
    /** Total baris untuk mode server (default: rows.length). */
    total?: number;
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
    pageSize = 0,
    pageSizeOptions = [10, 25, 50, 100],
    page,
    onpage,
    onpagesize,
    total,
  }: Props = $props();

  const dark = $derived(mode === 'dark');

  // ── Pagination ──
  let sizeOverride = $state<number | null>(null);
  let innerPage = $state(1);
  const serverMode = $derived(onpage !== undefined);
  const size = $derived(sizeOverride ?? pageSize);
  const effectiveTotal = $derived(total ?? rows.length);
  const currentPage = $derived(
    serverMode ? Math.max(1, page ?? 1) : Math.max(1, innerPage),
  );
  const pageCount = $derived(
    !size ? 1 : Math.max(1, Math.ceil(effectiveTotal / size)),
  );
  const visibleRows = $derived(
    !size || serverMode
      ? rows
      : rows.slice((currentPage - 1) * size, currentPage * size),
  );
  const rangeStart = $derived(
    effectiveTotal === 0 ? 0 : (currentPage - 1) * size + 1,
  );
  const rangeEnd = $derived(Math.min(currentPage * size, effectiveTotal));

  // Clamp saat filter menyusutkan rows (mode klien).
  $effect(() => {
    if (!serverMode && innerPage > pageCount) innerPage = pageCount;
  });

  function goto(p: number) {
    if (p < 1 || p > pageCount) return;
    if (serverMode) onpage?.(p);
    else innerPage = p;
  }

  function changeSize(ev: Event) {
    const v = Number((ev.target as HTMLSelectElement).value);
    if (serverMode) {
      if (onpagesize && Number.isFinite(v) && v > 0) onpagesize(v);
      return;
    }
    sizeOverride = Number.isFinite(v) && v > 0 ? v : pageSize;
    innerPage = 1;
  }

  /** Terjemahan dgn fallback — re-aktif terhadap pergantian locale. */
  const tr = $derived($t);
  function tt(key: string, fallback: string) {
    const v = tr(key);
    return typeof v === 'string' && v ? v : fallback;
  }
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
          {#each visibleRows as row}
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
    {:else if visibleRows.length === 0}
      <div class="flex flex-col items-center gap-2 px-4 py-14 text-center">
        <Icon name="inbox" size={26} class={dark ? 'text-slate-600' : 'text-ink-300'} />
        <div class="text-base font-medium {dark ? 'text-slate-300' : 'text-ink-700'}">
          {emptyTitle}
        </div>
        <div class="text-sm {dark ? 'text-slate-400' : 'text-ink-500'}">{emptyHint}</div>
      </div>
    {/if}
  </div>

  {#if size > 0 && !loading && effectiveTotal > Math.min(size, ...pageSizeOptions)}
    {@const showSizeSelect = serverMode ? onpagesize !== undefined : true}
    {@const rangeText = tt(
      'components.pagination.range',
      '{start}–{end} dari {count}',
    )
      .replace('{start}', String(rangeStart))
      .replace('{end}', String(rangeEnd))
      .replace('{count}', String(effectiveTotal))}
    <div
      class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-t px-4 py-2.5 text-sm
        {dark
        ? 'border-white/8 bg-night-100 text-slate-400'
        : 'border-ink-200 bg-ink-50 text-ink-500'}"
    >
      {#if showSizeSelect}
        <label class="flex items-center gap-2 whitespace-nowrap">
          <span>{tt('components.pagination.rows_per_page', 'Baris / hal')}</span>
          <select
            class="rounded-lg border px-2 py-1 text-sm font-medium outline-none
              {dark
              ? 'border-white/10 bg-night-200 text-slate-200'
              : 'border-ink-200 bg-white text-ink-700'}"
            value={size}
            onchange={changeSize}
            aria-label={tt('components.pagination.rows_per_page_aria', 'Baris per halaman')}
          >
            {#each pageSizeOptions as n}
              <option value={n}>{n}</option>
            {/each}
          </select>
        </label>
      {:else}
        <span class="whitespace-nowrap">{rangeText}</span>
      {/if}
      <div class="flex items-center gap-3">
        {#if showSizeSelect}
          <span class="whitespace-nowrap tabular-nums">{rangeText}</span>
        {/if}
        <div class="flex items-center gap-1">
          <button
            type="button"
            class="grid h-8 w-8 place-items-center rounded-lg border transition-colors
              disabled:cursor-not-allowed disabled:opacity-35
              {dark
              ? 'border-white/10 text-slate-300 hover:bg-white/5'
              : 'border-ink-200 text-ink-600 hover:bg-ink-100'}"
            disabled={currentPage <= 1}
            onclick={() => goto(currentPage - 1)}
            aria-label={tt('components.pagination.previous_page', 'Halaman sebelumnya')}
            title={tt('components.pagination.previous_page', 'Halaman sebelumnya')}
          >
            <Icon name="chevronLeft" size={16} />
          </button>
          <span class="min-w-16 text-center text-xs font-semibold tabular-nums">
            {currentPage} / {pageCount}
          </span>
          <button
            type="button"
            class="grid h-8 w-8 place-items-center rounded-lg border transition-colors
              disabled:cursor-not-allowed disabled:opacity-35
              {dark
              ? 'border-white/10 text-slate-300 hover:bg-white/5'
              : 'border-ink-200 text-ink-600 hover:bg-ink-100'}"
            disabled={currentPage >= pageCount}
            onclick={() => goto(currentPage + 1)}
            aria-label={tt('components.pagination.next_page', 'Halaman berikutnya')}
            title={tt('components.pagination.next_page', 'Halaman berikutnya')}
          >
            <Icon name="chevronRight" size={16} />
          </button>
        </div>
      </div>
    </div>
  {/if}

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
