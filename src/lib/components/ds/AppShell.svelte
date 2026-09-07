<!--
  Shell v2 — kerangka aplikasi berbasis design system.

  Dipakai hanya oleh halaman yang sudah dimigrasi. Halaman lama tetap memakai
  (app)/+layout.svelte dengan Sidebar/Topbar lama, jadi migrasi bisa bertahap
  tanpa periode "setengah rusak".

  Perbedaan dari shell lama:
  - NavRail 56px (hover 240px) menggantikan Sidebar 1.139 baris.
  - Latar terang; kontras teks utama 17,72:1 (sebelumnya primary 2,53:1 GAGAL).
  - Tidak ada CSS scoped: semua utility, jadi tidak menambah 25.547 baris CSS.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { page } from '$app/stores';
  import { user, can } from '$lib/stores/auth';
  import NavRail from './NavRail.svelte';
  import Topbar from './Topbar.svelte';
  import Icon from './Icon.svelte';
  import UserMenu from './UserMenu.svelte';
  import { unreadCount } from '$lib/stores/notifications';
  import { openNotificationModal } from '$lib/stores/notificationModal';
  import { buildAdminNav, type NavBadges } from '$lib/utils/navConfig';

  interface Props {
    /** Judul di topbar. */
    title?: string;
    badges?: NavBadges;
    children: Snippet;
  }

  let { title, badges = {}, children }: Props = $props();

  let mobileOpen = $state(false);

  const groups = $derived(buildAdminNav($can, $user, badges, { v2: true }));
  const current = $derived($page.url.pathname);
  const tenant = $derived($user?.tenant_slug ?? undefined);
</script>

<div class="ds-scope flex h-dvh overflow-hidden bg-ink-50 text-ink-900">
  <!-- Rail desktop -->
  <div class="hidden lg:block">
    <NavRail {groups} {current} {tenant} />
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
        <NavRail {groups} {current} {tenant} />
      </div>
    </div>
  {/if}

  <div class="flex min-w-0 flex-1 flex-col">
    <Topbar {title} onMenuClick={() => (mobileOpen = !mobileOpen)}>
      {#snippet right()}
        <button
          aria-label="Notifikasi"
          class="focus-ring relative grid size-8 place-items-center rounded-lg text-ink-500 hover:bg-ink-100"
          onclick={() => openNotificationModal()}
        >
          <Icon name="bell" size={17} />
          {#if $unreadCount > 0}
            <span
              class="absolute -right-0.5 -top-0.5 grid min-w-[15px] place-items-center rounded-full bg-red-500 px-[3px] text-[9px] font-semibold leading-[15px] text-white"
            >{$unreadCount > 99 ? '99+' : $unreadCount}</span>
          {/if}
        </button>
        <UserMenu />
      {/snippet}
    </Topbar>

    <main class="min-w-0 flex-1 overflow-y-auto">
      <div class="mx-auto max-w-[1400px] p-5 lg:p-7">
        {@render children()}
      </div>
    </main>
  </div>
</div>
