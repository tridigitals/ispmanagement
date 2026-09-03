<!--
  Ds/Card — pembungkus panel standar.

  `.card` sebelumnya didefinisikan ulang di 30 file dengan radius dan border
  yang berbeda-beda. Komponen ini menggantikan semuanya.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title?: string;
    mode?: 'light' | 'dark';
    /** Set false kalau isi perlu menempel ke tepi, misal tabel. */
    padded?: boolean;
    class?: string;
    /** Konten di kanan header, misal link "Lihat semua". */
    aside?: Snippet;
    children: Snippet;
  }

  let { title, mode = 'light', padded = true, class: cls = '', aside, children }: Props = $props();

  const dark = $derived(mode === 'dark');
</script>

<section
  class="rounded-xl {dark
    ? 'bg-white/[0.03] ring-1 ring-inset ring-white/8'
    : 'bg-white ring-1 ring-inset ring-ink-200'} {cls}"
>
  {#if title}
    <div
      class="flex h-12 items-center justify-between gap-3 border-b px-5 {dark
        ? 'border-white/8'
        : 'border-ink-100'}"
    >
      <h2 class="text-base font-semibold {dark ? 'text-slate-200' : 'text-ink-900'}">{title}</h2>
      {#if aside}{@render aside()}{/if}
    </div>
  {/if}
  <div class={padded ? 'p-5' : ''}>
    {@render children()}
  </div>
</section>
