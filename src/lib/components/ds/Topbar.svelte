<!--
  Ds/Topbar — topbar shell v2.

  Berbeda dari Topbar lama (590 baris CSS scoped), di sini hanya utility dan
  tinggi dikunci 56px supaya sejajar dengan header NavRail.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  interface Props {
    /** Judul konteks, biasanya nama halaman aktif. */
    title?: string;
    /** Isi kanan: notifikasi, menu user, dsb. */
    right?: Snippet;
    onMenuClick?: () => void;
    searchPlaceholder?: string;
    onSearch?: (q: string) => void;
  }

  let {
    title,
    right,
    onMenuClick,
    searchPlaceholder = 'Cari pelanggan, invoice, PPPoE…',
    onSearch,
  }: Props = $props();

  let q = $state('');
</script>

<header
  class="flex h-14 shrink-0 items-center gap-3 border-b border-ink-200 bg-white px-4"
>
  {#if onMenuClick}
    <button
      onclick={onMenuClick}
      aria-label="Buka menu"
      class="focus-ring grid size-8 place-items-center rounded-lg text-ink-500 hover:bg-ink-100 lg:hidden"
    >
      <Icon name="menu" size={18} />
    </button>
  {/if}

  {#if title}
    <div class="truncate text-md font-semibold text-ink-900">{title}</div>
  {/if}

  <div class="relative ml-auto hidden w-full max-w-sm md:block">
    <span class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-400">
      <Icon name="search" size={15} />
    </span>
    <input
      bind:value={q}
      oninput={() => onSearch?.(q)}
      type="search"
      placeholder={searchPlaceholder}
      aria-label={searchPlaceholder}
      class="h-8 w-full rounded-lg bg-ink-50 pr-14 pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400 focus:bg-white focus:ring-brand-600 focus:outline-none"
    />
    <kbd
      class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 rounded border border-ink-200 bg-white px-1 font-mono text-2xs text-ink-400"
    >
      Ctrl K
    </kbd>
  </div>

  <div class="ml-auto flex items-center gap-1 md:ml-0">
    {#if right}{@render right()}{/if}
  </div>
</header>
