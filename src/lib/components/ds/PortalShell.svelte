<!--
  PortalShell — kerangka portal pelanggan v2 (gelombang 25, revisi 25d).

  Layout disamakan dengan AppShell (admin): NavRail sidebar + Topbar, bukan
  nav tab horizontal. Alasannya:
  - Legacy portal pelanggan juga pakai sidebar (Sidebar.svelte), jadi portal
    v2 top-nav justru deviasi dari dua-duanya.
  - Admin v2 dan portal jadi satu bahasa layout: sidebar kiri, konten kanan.

  Beda dari AppShell:
  - Nav dari buildPortalNavGroups (6 item pelanggan), bukan buildAdminNav.
  - Topbar tanpa search (portal tidak punya global search).
  - Brand rail "Portal", tanpa tenant slug.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { page } from '$app/stores';
  import { can } from '$lib/stores/auth';
  import NavRail from './NavRail.svelte';
  import Topbar from './Topbar.svelte';
  import Icon from './Icon.svelte';
  import UserMenu from './UserMenu.svelte';
  import { buildPortalNavGroups } from '$lib/utils/portalNav';

  interface Props {
    /** Judul di topbar. */
    title?: string;
    children: Snippet;
  }

  let { title, children }: Props = $props();

  let mobileOpen = $state(false);

  const groups = $derived(buildPortalNavGroups($can));
  const current = $derived($page.url.pathname);
</script>

<div class="ds-scope flex h-dvh overflow-hidden bg-ink-50 text-ink-900">
  <!-- Rail desktop -->
  <div class="hidden lg:block">
    <NavRail {groups} {current} brand="Portal" />
  </div>

  <!-- Drawer mobile -->
  {#if mobileOpen}
    <div class="fixed inset-0 z-40 lg:hidden">
      <button
        class="absolute inset-0 bg-ink-900/40"
        aria-label="Tutup menu"
        onclick={() => (mobileOpen = false)}
      ></button>
      <div class="relative h-full w-60">
        <NavRail {groups} {current} brand="Portal" />
      </div>
    </div>
  {/if}

  <div class="flex min-w-0 flex-1 flex-col">
    <Topbar {title} search={false} onMenuClick={() => (mobileOpen = !mobileOpen)}>
      {#snippet right()}
        <UserMenu subtitle="Portal Pelanggan" />
      {/snippet}
    </Topbar>

    <main class="min-w-0 flex-1 overflow-y-auto">
      <div class="mx-auto max-w-6xl p-5 lg:p-7">
        {@render children()}
      </div>
    </main>
  </div>
</div>
