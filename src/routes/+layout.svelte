<script lang="ts">
  import '$lib/styles/global.css';
  import '$lib/i18n'; // Init i18n
  import { waitLocale, t } from 'svelte-i18n';
  import { checkAuth, isAuthenticated, isSuperAdmin } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { appLogo } from '$lib/stores/logo';
  import { theme } from '$lib/stores/theme';
  import { install } from '$lib/api/client';
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { connectWebSocket, disconnectWebSocket } from '$lib/stores/websocket';
  import { refreshUnreadCount } from '$lib/stores/notifications';
  import { Toaster } from 'svelte-sonner';
  import GlobalUploads from '$lib/components/layout/GlobalUploads.svelte';
  import { getSlugFromDomain } from '$lib/utils/domain';

  let loading = true;
  let i18nReady = false;

  onMount(async () => {
    try {
      // 1. Validate Auth & Session first
      // This ensures we have the correct tenant context before fetching data
      await checkAuth();

      // Apply saved theme
      theme.init();

      // Load global settings & logo from cache immediately
      // Logo is only refreshed from backend when user logs in (handled by auth store)
      await Promise.all([appSettings.init(), appLogo.init()]);

      // Wait for i18n to be ready (locale set in appSettings.init)
      // Wait for i18n to be ready (locale set in appSettings.init)
      await waitLocale();
      i18nReady = true;

      // --- Dynamic Custom Domain Lookup ---
      const hostname = window.location.hostname;
      const knownSlug = getSlugFromDomain(hostname);

      // If we are on a custom domain (not localhost/IP) and we DON'T know the slug yet
      // We need to ask the backend.
      const isLocal =
        hostname.includes('localhost') ||
        hostname.includes('127.0.0.1') ||
        hostname.includes('tauri');

      // Service Worker Registration
      if ('serviceWorker' in navigator) {
        try {
          await navigator.serviceWorker.register('/sw.js');
        } catch (e) {
          console.error('[SW] Registration failed:', e);
        }
      }

      if (!knownSlug && !isLocal) {
        try {
          // We use fetch directly here to avoid auth store dependencies loop if possible
          // or use the 'api' client but it might not be ready? 'api.public' is stateless usually.
          // Let's use fetch for safety and simplicity on this "boot" phase.
          const apiUrl = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';
          const res = await fetch(`${apiUrl}/public/tenant-lookup?domain=${hostname}`);

          if (res.ok) {
            const tenant = await res.json();
            if (tenant && tenant.slug) {
              console.log(
                `[Domain] Found tenant '${tenant.slug}' for '${hostname}'. Caching and reloading...`,
              );
              // Cache it
              await import('$lib/utils/domain').then((m) =>
                m.cacheDomainMapping(hostname, tenant.slug),
              );

              // RELOAD to let hooks.ts reroute handle it
              window.location.reload();
              return; // Stop initialization
            }
          }
        } catch (e) {
          console.warn('[Domain] Failed to lookup custom domain:', e);
        }
      }
      // ------------------------------------

      // Check if app is installed
      const isInstalled = await install.checkIsInstalled();
      const currentPath = $page.url.pathname;

      if (!isInstalled) {
        if (currentPath !== '/install') {
          // console.log("App not installed, redirecting to /install");
          goto('/install');
        }
      } else {
        if (currentPath === '/install') {
          console.log('App installed, leaving /install page for /login');
          goto('/login');
        }
        await checkAuth();

        // Check maintenance mode - redirect non-superadmin users
        const settings = $appSettings as any;
        const isMaintenanceMode =
          settings.maintenance_mode === true || settings.maintenance_mode === 'true';
        const allowedPaths = ['/login', '/maintenance', '/install', '/superadmin'];
        const isAllowedPath = allowedPaths.some((p) => currentPath.startsWith(p));

        if (isMaintenanceMode && !$isSuperAdmin && !isAllowedPath) {
          goto('/maintenance');
          return;
        }

        // Connect to WebSocket for real-time updates (only if authenticated)
        if ($isAuthenticated) {
          connectWebSocket();
          refreshUnreadCount();
        }
      }
    } catch (e) {
      console.error('Critical Error during app initialization in +layout.svelte:', e);
      // We stop here to prevent loops. The user will see a loading screen,
      // and the real error will be in the console.
    } finally {
      loading = false;
    }
  });

  // Disconnect WebSocket when app unloads
  onDestroy(() => {
    disconnectWebSocket();
  });
</script>

<svelte:head>
  {#if $appLogo}
    <link rel="icon" type="image/png" href={$appLogo} />
  {/if}
</svelte:head>

{#if loading}
  <div class="loading-container">
    <div class="spinner"></div>
    <p>{i18nReady ? $t('common.loading') || 'Loading...' : 'Loading...'}</p>
  </div>
{:else}
  <Toaster />
  <GlobalUploads />
  <slot />
{/if}

<style>
  .loading-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    gap: 1rem;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid var(--bg-tertiary);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
