<!--
  Ds/RowActions — satu aksi utama + menu untuk sisanya.

  Sebelum redesign: halaman Customers menampilkan 7 tombol ikon per baris =
  70 tombol terlihat sekaligus (170 per halaman), dan itu penyumbang utama
  lebar minimum tabel 1.114px yang bikin overflow horizontal di bawah 1.150px.
-->
<script lang="ts">
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

  let open = $state(false);
  let wrap: HTMLDivElement | undefined = $state();

  function onWindowClick(e: MouseEvent) {
    if (open && wrap && !wrap.contains(e.target as Node)) open = false;
  }

  function run(a: RowAction) {
    open = false;
    a.onclick?.();
  }
</script>

<svelte:window onclick={onWindowClick} />

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
    <div class="relative">
      <button
        onclick={(e) => {
          e.stopPropagation();
          open = !open;
        }}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label="Aksi lain"
        class="focus-ring inline-flex size-7 items-center justify-center rounded-md text-ink-400 hover:bg-ink-100 hover:text-ink-900"
      >
        <Icon name="more" size={15} />
      </button>

      {#if open}
        <div
          role="menu"
          class="absolute right-0 z-20 mt-1 w-52 overflow-hidden rounded-lg bg-white py-1 shadow-lg ring-1 ring-ink-200"
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
    </div>
  {/if}
</div>
