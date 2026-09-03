<!--
  Ds/TableSkeleton — placeholder baris tabel saat data dimuat.

  Sebelum redesign: 0 dari 7 halaman tabel punya skeleton, semuanya spinner,
  sehingga tinggi konten melompat ketika data masuk.
-->
<script lang="ts">
  interface Props {
    rows?: number;
    cols?: number;
    mode?: 'light' | 'dark';
  }

  let { rows = 8, cols = 5, mode = 'light' }: Props = $props();
  const dark = $derived(mode === 'dark');

  /* Lebar bervariasi supaya tidak terlihat seperti grid kosong. */
  const widths = ['w-40', 'w-24', 'w-32', 'w-20', 'w-28', 'w-16'];
</script>

<div class={dark ? 'dark' : ''} aria-hidden="true">
  {#each Array(rows) as _, r}
    <div
      class="flex items-center gap-6 border-b px-4 py-3 {dark ? 'border-white/6' : 'border-ink-100'}"
    >
      {#each Array(cols) as _, c}
        <div class="skeleton h-3 {widths[(r + c) % widths.length]}"></div>
      {/each}
    </div>
  {/each}
</div>
