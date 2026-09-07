<!--
  Ds/UserMenu — menu akun di kanan topbar (AppShell & PortalShell).

  Cutover v2: shell lama punya menu user (profil, 2FA, logout); AppShell/
  PortalShell v2 belum, jadi setelah cutover user tidak bisa logout. Komponen
  ini mengisi gap itu dengan pola DS: tombol avatar -> dropdown sederhana.

  - "Profil & keamanan" membuka ProfileModal lama (store profileModal) —
    modalnya dirender oleh (app)/+layout.svelte, jadi hanya berfungsi saat
    layout itu ikut ter-mount. Fallback: jika modal tidak tersedia, tidak
    ada error (tombol tetap membuka store; halaman v2 berada di bawah root
    layout yang sama).
  - "Keluar" memanggil logout() dari store auth.
-->
<script lang="ts">
  import { page } from '$app/stores';
  import { user, logout } from '$lib/stores/auth';
  import { openProfileModal } from '$lib/stores/profileModal';
  import Icon from './Icon.svelte';

  interface Props {
    /** Teks kecil di bawah nama, mis. role atau "Portal Pelanggan". */
    subtitle?: string;
  }

  let { subtitle }: Props = $props();

  let open = $state(false);
  let wrap = $state<HTMLDivElement | undefined>();

  function close() {
    open = false;
  }

  function onDocClick(e: MouseEvent) {
    if (open && wrap && !wrap.contains(e.target as Node)) close();
  }

  function toggle() {
    open = !open;
  }

  async function handleLogout() {
    close();
    await logout();
    window.location.assign('/login');
  }

  function handleProfile() {
    close();
    openProfileModal({ tab: 'general', reason: 'manual' });
  }

  const initials = $derived(($user?.name ?? $user?.email ?? '?').slice(0, 2).toUpperCase());
  const displayName = $derived($user?.name ?? $user?.email ?? '—');
  const roleLabel = $derived(subtitle ?? $user?.tenant_role ?? $user?.role ?? '');
</script>

<svelte:window onclick={onDocClick} />

<div class="relative" bind:this={wrap}>
  <button
    type="button"
    onclick={toggle}
    aria-haspopup="menu"
    aria-expanded={open}
    class="focus-ring flex items-center gap-2 rounded-lg py-1 pl-1 pr-1.5 hover:bg-ink-100"
  >
    <span
      class="grid size-7 shrink-0 place-items-center rounded-full bg-brand-100 text-2xs font-semibold text-brand-700"
    >
      {initials}
    </span>
    <span class="hidden min-w-0 text-left sm:block">
      <span class="block max-w-[160px] truncate text-sm font-medium text-ink-900">{displayName}</span>
      <span class="block max-w-[160px] truncate text-2xs text-ink-400">{roleLabel}</span>
    </span>
    <Icon name="chevronDown" size={14} class="hidden text-ink-400 sm:block" />
  </button>

  {#if open}
    <div
      role="menu"
      class="absolute right-0 top-full z-50 mt-1.5 w-56 overflow-hidden rounded-xl border border-ink-200 bg-white py-1 shadow-lg"
    >
      <div class="border-b border-ink-100 px-3 py-2">
        <div class="truncate text-sm font-medium text-ink-900">{displayName}</div>
        <div class="truncate text-2xs text-ink-400">{$user?.email ?? ''}</div>
      </div>
      <button
        type="button"
        role="menuitem"
        onclick={handleProfile}
        class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm text-ink-700 hover:bg-ink-50"
      >
        <Icon name="users" size={15} class="text-ink-400" />
        Profil &amp; keamanan
      </button>
      <button
        type="button"
        role="menuitem"
        onclick={handleLogout}
        class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm text-ink-700 hover:bg-ink-50"
      >
        <Icon name="logout" size={15} class="text-ink-400" />
        Keluar
      </button>
    </div>
  {/if}
</div>
