<!--
  Ds/Tabs — navigasi tab untuk halaman detail.

  Menggantikan ResponsiveTabs (216 baris + CSS scoped). Beda perilakunya:
  - Tab yang tidak muat TIDAK dipindah ke menu overflow, tapi ikut discroll
    horizontal. Menu overflow lama menyembunyikan tab di balik tombol "..."
    sehingga di layar sempit orang tidak tahu tab itu ada.
  - Indikator aktif garis bawah 2px, bukan latar penuh: batas antar tab tetap
    terbaca saat labelnya panjang.
  - `role="tab"` + `aria-selected` + panah kiri/kanan sesuai pola ARIA tabs,
    jadi bisa dijalankan tanpa mouse.
-->
<script lang="ts">
  export interface TabItem {
    id: string;
    label: string;
    /** Angka kecil di kanan label, misal jumlah baris pada tab itu. */
    count?: number | null;
  }

  interface Props {
    items: TabItem[];
    active: string;
    /** Id panel yang dikendalikan, untuk aria-controls. */
    panelId?: string;
    onselect: (id: string) => void;
  }

  let { items, active, panelId, onselect }: Props = $props();

  let listEl = $state<HTMLDivElement | null>(null);

  /* Panah kiri/kanan memindah fokus sekaligus memilih, sesuai pola ARIA
     "tabs with automatic activation" — pengguna keyboard tidak perlu Enter. */
  function onKeydown(event: KeyboardEvent) {
    const delta = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
    if (delta === 0) return;
    event.preventDefault();

    const index = items.findIndex((i) => i.id === active);
    const next = items[(index + delta + items.length) % items.length];
    if (!next) return;

    onselect(next.id);
    listEl?.querySelector<HTMLButtonElement>(`[data-tab="${next.id}"]`)?.focus();
  }
</script>

<div
  bind:this={listEl}
  role="tablist"
  tabindex="-1"
  onkeydown={onKeydown}
  class="-mx-1 mb-5 flex gap-1 overflow-x-auto border-b border-ink-200 px-1"
>
  {#each items as item (item.id)}
    {@const on = item.id === active}
    <button
      role="tab"
      data-tab={item.id}
      aria-selected={on}
      aria-controls={panelId}
      tabindex={on ? 0 : -1}
      onclick={() => onselect(item.id)}
      class="focus-ring -mb-px flex h-9 shrink-0 items-center gap-1.5 border-b-2 px-3 text-base whitespace-nowrap
        {on
        ? 'border-brand-600 font-medium text-ink-900'
        : 'border-transparent text-ink-500 hover:border-ink-300 hover:text-ink-700'}"
    >
      {item.label}
      {#if item.count != null}
        <span class="num text-sm {on ? 'text-ink-500' : 'text-ink-400'}">{item.count}</span>
      {/if}
    </button>
  {/each}
</div>
