<!--
  Ds/PageHeader — kepala halaman standar.

  `.page-head` sebelumnya ditulis ulang di 20 file dengan ukuran judul yang
  beda-beda. Satu definisi di sini.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    /** Baris kecil di atas judul: konteks periode, tanggal, atau modul. */
    eyebrow?: string;
    desc?: string;
    mode?: 'light' | 'dark';
    actions?: Snippet;
  }

  let { title, eyebrow, desc, mode = 'light', actions }: Props = $props();
  const dark = $derived(mode === 'dark');
</script>

<div class="mb-6 flex flex-wrap items-end justify-between gap-4">
  <div class="min-w-0">
    {#if eyebrow}
      <div
        class="mb-1.5 font-mono text-2xs tracking-[0.14em] uppercase {dark
          ? 'text-sky-400/80'
          : 'text-ink-400'}"
      >
        {eyebrow}
      </div>
    {/if}
    <h1
      class="text-xl leading-tight font-semibold tracking-tight {dark
        ? 'text-slate-50'
        : 'text-ink-900'}"
    >
      {title}
    </h1>
    {#if desc}
      <p class="mt-1 max-w-2xl text-base {dark ? 'text-slate-400' : 'text-ink-500'}">{desc}</p>
    {/if}
  </div>
  {#if actions}
    <div class="flex flex-wrap items-center gap-2">{@render actions()}</div>
  {/if}
</div>
