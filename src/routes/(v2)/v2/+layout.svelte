<!--
  Layout pratinjau shell v2.

  Sengaja berada di route group sendiri dengan prefix URL /v2 supaya:
  - Tidak bertabrakan dengan (app)/admin/** yang masih memakai shell lama.
  - Halaman produksi tidak berubah sama sekali selama pratinjau berjalan.

  Guard auth di sini adalah versi ringkas dari (app)/+layout.svelte: hanya
  memeriksa token lalu memvalidasi sesi. Cek izin per halaman tetap dilakukan
  oleh masing-masing halaman lewat store `can`.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { isAuthenticated, checkAuth } from '$lib/stores/auth';
  import { secureGetItem } from '$lib/utils/tauri-store';
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

      if (!cancelled) ready = true;
    })();

    return () => {
      cancelled = true;
    };
  });
</script>

{#if ready}
  {@render children()}
{:else}
  <div class="grid h-dvh place-items-center bg-ink-50">
    <div class="text-base text-ink-500">Memuat…</div>
  </div>
{/if}
