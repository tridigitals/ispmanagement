<!--
  Ds/FieldRow — satu baris label:nilai untuk panel ringkasan.

  Halaman detail lama menampilkan data ini sebagai tabel dua kolom dengan CSS
  scoped berbeda di tiap tab. Komponen ini menyeragamkan: label kecil di atas,
  nilai di bawah, dan nilai kosong SELALU jadi "—" (bukan string kosong yang
  membuat baris tampak rusak).
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    value?: string | number | null;
    /** Pakai untuk nomor/ID/uang supaya sejajar dan mudah dibandingkan. */
    mono?: boolean;
    children?: Snippet;
  }

  let { label, value = null, mono = false, children }: Props = $props();

  const shown = $derived(
    value === null || value === undefined || value === '' ? '—' : String(value),
  );
</script>

<div class="min-w-0 py-2">
  <dt class="text-2xs font-medium tracking-wide text-ink-400 uppercase">{label}</dt>
  <dd class="mt-0.5 text-base break-words text-ink-800 {mono ? 'num' : ''}">
    {#if children}{@render children()}{:else}{shown}{/if}
  </dd>
</div>
