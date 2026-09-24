<!--
  Ds/RowActions — satu aksi utama + menu untuk sisanya.

  Sebelum redesign: halaman Customers menampilkan 7 tombol ikon per baris =
  70 tombol terlihat sekaligus (170 per halaman), dan itu penyumbang utama
  lebar minimum tabel 1.114px yang bikin overflow horizontal di bawah 1.150px.

  PENTING — menu overflow pakai `position: fixed`, bukan `absolute`:

  `DataTable` membungkus tabel dengan `overflow-hidden` + `overflow-auto`
  (supaya tabel lebar tidak merusak layout). Konsekuensinya elemen
  `position: absolute` di dalam sel TERPOTONG di batas wrapper scroll: menu
  ikut terpangkas dan memunculkan scrollbar di dalam tabel. Karena itu menu di
  sini dirender fixed lalu diposisikan manual dari `getBoundingClientRect()`
  (rata kanan ke tombol, flip ke atas bila ruang bawah kurang), dan mengikuti
  posisi tombol saat tabel/halaman di-scroll. Posisi fixed valid di sini —
  tidak ada ancestor ber-`transform`/`filter`/`contain` pada layout tabel.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';

  export interface RowAction {
    label: string;
    icon?: IconName;
    danger?: boolean;
    onclick?: () => void;
    href?: string;
    /**
     * Nonaktifkan aksi yang pasti ditolak backend, misal mengubah anggota
     * dengan level role sama atau lebih tinggi. Lebih jujur daripada
     * membiarkan pengguna mengklik lalu menerima 403.
     */
    disabled?: boolean;
    /**
     * Alasan aksi dinonaktifkan. Wajib diisi bersama `disabled` — tombol mati
     * tanpa penjelasan memaksa pengguna menebak. Dipakai sebagai title dan
     * masuk ke aria-label supaya terbaca screen reader.
     */
    disabledReason?: string;
  }

  interface Props {
    /** Aksi utama, tampil sebagai tombol. */
    primary: RowAction;
    /** Sisanya masuk menu overflow. */
    rest?: RowAction[];
  }

  let { primary, rest = [] }: Props = $props();

  const MENU_WIDTH = 208; // selaras w-52
  const MENU_GAP = 4;
  const EDGE = 8;

  let open = $state(false);
  let wrap: HTMLDivElement | undefined = $state();
  let trigger: HTMLButtonElement | undefined = $state();
  let menu: HTMLDivElement | undefined = $state();
  let menuStyle = $state('visibility:hidden');

  function position() {
    if (!trigger) return;
    const r = trigger.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;

    /* Tombol keluar viewport (baris di-scroll jauh) → tutup saja. */
    if (r.bottom < 0 || r.top > vh || r.right < 0 || r.left > vw) {
      open = false;
      return;
    }

    /* Rata kanan ke tombol, lalu di-clamp supaya tidak keluar layar. */
    const left = Math.max(EDGE, Math.min(r.right - MENU_WIDTH, vw - MENU_WIDTH - EDGE));

    const menuH = menu?.offsetHeight ?? 0;
    let top = r.bottom + MENU_GAP;
    /* Ruang bawah tidak cukup → flip ke atas tombol. */
    if (menuH && top + menuH > vh - EDGE) {
      const above = r.top - MENU_GAP - menuH;
      top = above >= EDGE ? above : Math.max(EDGE, vh - menuH - EDGE);
    }

    menuStyle = `top:${Math.round(top)}px;left:${Math.round(left)}px;width:${MENU_WIDTH}px`;
  }

  async function toggle(e: MouseEvent) {
    e.stopPropagation();
    open = !open;
    if (!open) return;
    position();
    /* Ukur ulang setelah render supaya flip-atas pakai tinggi menu sebenarnya. */
    await tick();
    position();
  }

  function close() {
    open = false;
  }

  function run(a: RowAction) {
    close();
    a.onclick?.();
  }

  function onWindowClick(e: MouseEvent) {
    if (!open) return;
    const t = e.target as Node;
    if (wrap?.contains(t) || menu?.contains(t)) return;
    close();
  }

  /* Semua scroll harus ikut menggerakkan menu. `scroll` TIDAK bubble, jadi
     listener window biasa tidak menangkap scroll container di dalam
     (`main.overflow-y-auto`, wrapper `overflow-auto` milik DataTable).
     Karena itu dipasang di document dengan fase capture. */
  function onAnyScroll() {
    if (open) position();
  }

  $effect(() => {
    document.addEventListener('scroll', onAnyScroll, true);
    return () => document.removeEventListener('scroll', onAnyScroll, true);
  });

  function onKeydown(e: KeyboardEvent) {
    if (open && e.key === 'Escape') close();
  }
</script>

<svelte:window onclick={onWindowClick} onresize={onAnyScroll} onkeydown={onKeydown} />

<div class="flex items-center justify-end gap-1" bind:this={wrap}>
  {#if primary.href}
    <a
      href={primary.href}
      class="focus-ring inline-flex h-7 items-center gap-1 rounded-md px-2 text-sm font-medium text-ink-700 ring-1 ring-inset ring-ink-200 hover:bg-ink-50"
    >
      {#if primary.icon}<Icon name={primary.icon} size={13} />{/if}
      {primary.label}
    </a>
  {:else}
    <button
      onclick={() => run(primary)}
      disabled={primary.disabled}
      title={primary.disabled ? primary.disabledReason : undefined}
      aria-label={primary.disabled && primary.disabledReason
        ? `${primary.label} — ${primary.disabledReason}`
        : undefined}
      class="focus-ring inline-flex h-7 items-center gap-1 rounded-md px-2 text-sm font-medium text-ink-700 ring-1 ring-inset ring-ink-200 hover:bg-ink-50 disabled:cursor-not-allowed disabled:bg-ink-50 disabled:text-ink-400 disabled:hover:bg-ink-50"
    >
      {#if primary.icon}<Icon name={primary.icon} size={13} />{/if}
      {primary.label}
    </button>
  {/if}

  {#if rest.length}
    <button
      bind:this={trigger}
      onclick={toggle}
      aria-haspopup="menu"
      aria-expanded={open}
      aria-label="Aksi lain"
      class="focus-ring inline-flex size-7 items-center justify-center rounded-md text-ink-400 hover:bg-ink-100 hover:text-ink-900"
    >
      <Icon name="more" size={15} />
    </button>

    {#if open}
      <div
        bind:this={menu}
        role="menu"
        style={menuStyle}
        class="fixed z-50 overflow-hidden rounded-lg bg-white py-1 shadow-lg ring-1 ring-ink-200"
      >
        {#each rest as a}
          <button
            role="menuitem"
            onclick={() => run(a)}
            disabled={a.disabled}
            title={a.disabled ? a.disabledReason : undefined}
            class="flex w-full items-center gap-2.5 px-3 py-1.5 text-left text-base hover:bg-ink-50 disabled:cursor-not-allowed disabled:text-ink-400 disabled:hover:bg-transparent {a.danger
              ? 'text-red-700'
              : 'text-ink-700'}"
          >
            {#if a.icon}<Icon name={a.icon} size={14} />{/if}
            {a.label}
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>
