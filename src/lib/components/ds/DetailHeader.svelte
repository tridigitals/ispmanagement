<!--
  Ds/DetailHeader — kepala halaman detail entitas (pelanggan, router, tiket).

  Halaman detail pelanggan lama memakai "hero" 90 baris markup + ~180 baris CSS
  scoped: avatar besar, dua baris pill, dan 7 tombol aksi sejajar. Di layar
  1600px hero itu memakan 168px tinggi sebelum konten pertama terlihat.

  Versi ini tingginya tetap (avatar 40px, satu baris meta) dan aksinya dibatasi:
  satu tombol utama + sisanya di menu, memakai RowActions yang sama seperti di
  tabel — jadi cuma ada satu pola "aksi banyak" di seluruh aplikasi.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Badge from './Badge.svelte';
  import Icon from './Icon.svelte';
  import type { StatusTone } from './tokens';

  export interface MetaItem {
    label: string;
    value: string;
  }

  interface Props {
    title: string;
    /** Nomor/identifier yang bisa disalin, ditampilkan mono di bawah judul. */
    subtitle?: string;
    /** Status utama entitas: dipetakan ke tone lewat tokens.ts. */
    status?: string | null;
    statusTone?: StatusTone;
    statusLabel?: string;
    /** Pasangan label:nilai di kanan judul. Maksimal 4 supaya tetap satu baris. */
    meta?: MetaItem[];
    backHref?: string;
    backLabel?: string;
    actions?: Snippet;
  }

  let {
    title,
    subtitle,
    status = null,
    statusTone,
    statusLabel,
    meta = [],
    backHref,
    backLabel = 'Kembali',
    actions,
  }: Props = $props();

  const initials = $derived(
    title
      .split(' ')
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase() ?? '')
      .join(''),
  );
</script>

<div class="mb-5">
  {#if backHref}
    <a
      href={backHref}
      class="focus-ring mb-2.5 inline-flex items-center gap-1 rounded text-sm text-ink-500 hover:text-ink-900"
    >
      <Icon name="chevronLeft" size={14} />
      {backLabel}
    </a>
  {/if}

  <div class="flex flex-wrap items-start justify-between gap-4">
    <div class="flex min-w-0 items-center gap-3">
      <div
        aria-hidden="true"
        class="grid size-10 shrink-0 place-items-center rounded-full bg-brand-100 text-sm font-semibold text-brand-700"
      >
        {initials || '?'}
      </div>
      <div class="min-w-0">
        <div class="flex flex-wrap items-center gap-2">
          <h1 class="truncate text-xl leading-tight font-semibold tracking-tight text-ink-900">
            {title}
          </h1>
          {#if status || statusTone}
            <Badge {status} tone={statusTone} label={statusLabel} />
          {/if}
        </div>
        {#if subtitle}
          <div class="num mt-0.5 truncate text-sm text-ink-500">{subtitle}</div>
        {/if}
      </div>
    </div>

    {#if actions}
      <div class="flex flex-wrap items-center gap-2">{@render actions()}</div>
    {/if}
  </div>

  {#if meta.length > 0}
    <!-- Meta dipisah dari judul supaya baris judul tidak pernah terpotong. -->
    <dl class="mt-3 flex flex-wrap gap-x-7 gap-y-1.5 border-t border-ink-100 pt-3">
      {#each meta as item}
        <div class="min-w-0">
          <dt class="text-2xs font-medium tracking-wide text-ink-400 uppercase">{item.label}</dt>
          <dd class="truncate text-base text-ink-800">{item.value}</dd>
        </div>
      {/each}
    </dl>
  {/if}
</div>
