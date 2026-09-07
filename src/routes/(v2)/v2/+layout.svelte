<!--
  Layout shell v2 (entry utama sejak cutover).

  Berada di route group sendiri dengan prefix URL /v2; route lama (app) kini
  otomatis redirect ke padanannya lewat v2RedirectFor di (app)/+layout.svelte.
  Rollback cutover: hapus blok redirect itu — halaman lama tetap utuh.

  Guard auth di sini adalah versi ringkas dari (app)/+layout.svelte: memeriksa
  token, memvalidasi sesi, lalu cek peran (admin vs portal). Cek izin per
  halaman tetap dilakukan oleh masing-masing halaman lewat store `can`.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { isAuthenticated, checkAuth, user } from '$lib/stores/auth';
  import { hasInternalAppAccess } from '$lib/utils/appLanding';
  import { secureGetItem } from '$lib/utils/tauri-store';
  import ProfileModal from '$lib/components/profile/ProfileModal.svelte';
  import NotificationModal from '$lib/components/notifications/NotificationModal.svelte';
  import { refreshUnreadCount } from '$lib/stores/notifications';
  import '$lib/styles/design-system.css';

  let { children } = $props();
  let ready = $state(false);

  onMount(() => {
    let cancelled = false;

    (async () => {
      const hasToken = typeof window !== 'undefined' && !!secureGetItem('auth_token');

      if (!$isAuthenticated && !hasToken) {
        goto('/login');
        return;
      }

      if (hasToken) {
        const valid = await checkAuth();
        if (cancelled) return;
        if (!valid) {
          goto('/login?reason=expired');
          return;
        }
      }

      if (!cancelled) {
        ready = true;
        // Badge unread di bell butuh angka sejak halaman pertama; modalnya
        // sendiri me-refresh saat dibuka.
        refreshUnreadCount().catch(() => {});
      }
    })();

    return () => {
      cancelled = true;
    };
  });

  // Reaktif: navigasi client-side antar area tidak me-remount layout ini,
  // jadi cek peran tiap path/user berubah (setelah sesi valid).
  $effect(() => {
    if (!ready) return;
    const path = $page.url.pathname;
    const internal = hasInternalAppAccess($user);
    if (path.startsWith('/v2/admin') && !internal) {
      goto('/unauthorized');
    } else if (path.startsWith('/v2/dashboard') && internal) {
      goto('/v2/admin');
    }
  });
</script>

{#if ready}
  <!--
    `v2-light` meng-override token warna gelap legacy supaya komponen warisan
    (ui/Modal, RichTextEditor, Select, modal profil/notifikasi) yang dirender
    DI LUAR .ds-scope ikut terang. `contents` = tidak membuat box, jadi layout
    h-dvh AppShell tidak berubah; custom property tetap diwarisi lewat DOM.
    PENTING: modal WAJIB di dalam wrapper ini — di luar = tema gelap bocor.
  -->
  <div class="contents v2-light">
    {@render children()}
    <!-- Dipanggil dari UserMenu (Profil & keamanan) di AppShell/PortalShell. -->
    <ProfileModal />
    <!-- Dipanggil dari bell notifikasi di AppShell/PortalShell. -->
    <NotificationModal />
  </div>
{:else}
  <div class="grid h-dvh place-items-center bg-ink-50">
    <div class="text-base text-ink-500">Memuat…</div>
  </div>
{/if}
