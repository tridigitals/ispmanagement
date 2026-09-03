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
        >
          <Icon name="bell" size={17} />
        </button>
        <div class="ml-1 flex items-center gap-2 border-l border-ink-200 pl-2.5">
          <div
            class="grid size-7 place-items-center rounded-full bg-brand-100 text-2xs font-semibold text-brand-700"
          >
            {($user?.name ?? $user?.email ?? '?').slice(0, 2).toUpperCase()}
          </div>
          <div class="hidden min-w-0 sm:block">
            <div class="truncate text-sm font-medium text-ink-900">
              {$user?.name ?? $user?.email ?? '—'}
            </div>
            <div class="truncate text-2xs text-ink-400">{$user?.tenant_role ?? $user?.role ?? ''}</div>
          </div>
        </div>
      {/snippet}
    </Topbar>

    <main class="min-w-0 flex-1 overflow-y-auto">
      <div class="mx-auto max-w-[1400px] p-5 lg:p-7">
        {@render children()}
      </div>
    </main>
  </div>
</div>
