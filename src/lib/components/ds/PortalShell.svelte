<!--
  PortalShell — kerangka portal pelanggan v2 (gelombang 25).

  Beda dari AppShell (nav admin via NavRail): portal pelanggan hanya butuh
  topbar + nav tab horizontal (Beranda, Lokasi, Layanan, Tagihan, Pengumuman,
  Bantuan). Dipakai oleh halaman /v2/dashboard/**, /v2/support, /v2/announcements.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { page } from '$app/stores';
  import { can } from '$lib/stores/auth';
  import Icon from './Icon.svelte';
  import { buildPortalNav } from '$lib/utils/portalNav';

  interface Props {
    title?: string;
    children: Snippet;
  }

  let { title, children }: Props = $props();

  const items = $derived(buildPortalNav($can));
  const current = $derived($page.url.pathname);

  function isActive(href: string) {
    if (href === '/v2/dashboard') return current === href || current === '/v2/dashboard/';
    return current === href || current.startsWith(href + '/');
  }
</script>

<div class="ds-scope flex min-h-dvh flex-col bg-ink-50 text-ink-900">
  <header class="sticky top-0 z-20 border-b border-ink-200 bg-white">
    <div class="mx-auto flex h-14 max-w-6xl items-center gap-3 px-4">
      {#if title}
        <div class="truncate text-md font-semibold text-ink-900">{title}</div>
      {/if}
    </div>
    <nav class="mx-auto flex max-w-6xl items-center gap-1 overflow-x-auto px-4 pb-2" aria-label="Navigasi portal">
      {#each items as item (item.href)}
        <a
          href={item.href}
          class="flex shrink-0 items-center gap-1.5 rounded-lg px-3 py-1.5 text-sm transition-colors {isActive(item.href)
            ? 'bg-ink-900 font-medium text-white'
            : 'text-ink-500 hover:bg-ink-100 hover:text-ink-900'}"
          aria-current={isActive(item.href) ? 'page' : undefined}
        >
          <Icon name={item.icon} size={15} />
          {item.label}
        </a>
      {/each}
    </nav>
  </header>

  <main class="mx-auto w-full max-w-6xl flex-1 px-4 py-6">
    {@render children()}
  </main>
</div>
