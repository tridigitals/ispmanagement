<!--
  Ds/NavRail — sidebar ikon yang melebar saat hover.

  Menggantikan Sidebar.svelte (1.139 baris) untuk shell v2. Perbedaan pokok:
  - Lebar 56px saat diam, 240px saat hover/fokus. Isi utama dapat ruang lebih.
  - Ikon sinkron (bukan dynamic import), jadi tidak ada pop-in.
  - Label section tidak pakai accordion state yang perlu disimpan ke storage.
-->
<script lang="ts">
  import Icon from './Icon.svelte';
  import type { RailGroup } from './nav-types';

  interface Props {
    groups: RailGroup[];
    /** Path aktif saat ini untuk menandai item terpilih. */
    current: string;
    brand?: string;
    tenant?: string;
  }

  let { groups, current, brand = 'ISP Management', tenant }: Props = $props();

  let expanded = $state(false);

  function isActive(href: string) {
    if (href === current) return true;
    // Cocokkan prefix, tapi jangan sampai '/admin' menyorot semua submenu.
    return href !== '/admin' && current.startsWith(href + '/');
  }
</script>

<nav
  onmouseenter={() => (expanded = true)}
  onmouseleave={() => (expanded = false)}
  onfocusin={() => (expanded = true)}
  onfocusout={() => (expanded = false)}
  aria-label="Navigasi utama"
  class="group/rail flex h-full shrink-0 flex-col border-r border-ink-200 bg-white transition-[width] duration-150 ease-out
    {expanded ? 'w-60' : 'w-14'}"
>
  <!-- Identitas tenant -->
  <div class="flex h-14 items-center gap-2.5 border-b border-ink-100 px-3.5">
    <div
      class="grid size-7 shrink-0 place-items-center rounded-md bg-ink-900 text-2xs font-bold text-white"
    >
      ISP
    </div>
    {#if expanded}
      <div class="min-w-0">
        <div class="truncate text-sm font-semibold text-ink-900">{brand}</div>
        {#if tenant}
          <div class="truncate font-mono text-2xs text-ink-400">{tenant}</div>
        {/if}
      </div>
    {/if}
  </div>

  <div class="flex-1 overflow-x-hidden overflow-y-auto py-2">
    {#each groups as g}
      <div class="mb-1">
        {#if expanded}
          <div class="px-3.5 pt-2 pb-1 text-2xs font-semibold tracking-[0.12em] text-ink-400 uppercase">
            {g.title}
          </div>
        {:else}
          <div class="mx-3.5 my-2 border-t border-ink-100"></div>
        {/if}

        {#each g.items as it}
          <a
            href={it.href}
            aria-current={isActive(it.href) ? 'page' : undefined}
            title={expanded ? undefined : it.label}
            class="focus-ring relative mx-1.5 flex h-9 items-center gap-3 rounded-lg px-2 text-base
              {isActive(it.href)
              ? 'bg-ink-100 font-medium text-ink-900'
              : 'text-ink-500 hover:bg-ink-50 hover:text-ink-900'}"
          >
            {#if isActive(it.href)}
              <span class="absolute top-1.5 -left-1.5 h-6 w-[3px] rounded-r bg-ink-900"></span>
            {/if}
            <span class="grid size-5 shrink-0 place-items-center">
              <Icon name={it.icon} size={16} />
            </span>
            {#if expanded}
              <span class="truncate">{it.label}</span>
              {#if it.badge}
                <span class="num ml-auto rounded bg-red-100 px-1.5 py-px text-2xs font-semibold text-red-700">
                  {it.badge}
                </span>
              {/if}
            {:else if it.badge}
              <span class="absolute top-1.5 right-1.5 size-1.5 rounded-full bg-red-500"></span>
            {/if}
          </a>
        {/each}
      </div>
    {/each}
  </div>
</nav>
