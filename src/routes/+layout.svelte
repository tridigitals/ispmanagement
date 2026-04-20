<script lang="ts">
  import '$lib/styles/global.css';
  import '$lib/i18n'; // Init i18n
  import { waitLocale, t } from 'svelte-i18n';
  import { checkAuth, isAuthenticated, isSuperAdmin, logout } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { appLogo } from '$lib/stores/logo';
  import { theme } from '$lib/stores/theme';
  import { install } from '$lib/api/install';
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { connectWebSocket, disconnectWebSocket } from '$lib/stores/websocket';
  import { refreshUnreadCount, resetNotificationsState } from '$lib/stores/notifications';
  import { getSlugFromDomain, isPlatformDomain } from '$lib/utils/domain';
  import { browser } from '$app/environment';
  import { getApiBaseUrl } from '$lib/utils/apiUrl';
  import { normalizeLegacyBasePath, shouldLookupCustomDomain } from '$lib/utils/appBoot';
  import type { Component } from 'svelte';

  let loading = true;
  let i18nReady = false;
  let authExpiredHandled = false;
  let keepAliveHandle: ReturnType<typeof setInterval> | null = null;
  let lastUserActivityAt = Date.now();
  let ToasterComponent: Component | null = null;
  let GlobalUploadsComponent: Component | null = null;

  function markUserActivity() {
    lastUserActivityAt = Date.now();
  }

  function resolveSessionTimeoutMs() {
    const raw = Number(($appSettings as any)?.auth?.session_timeout_minutes ?? 60);
    const minutes = Number.isFinite(raw) && raw > 0 ? raw : 60;
    return minutes * 60 * 1000;
  }

  function isWallboardPath(pathname: string) {
    return pathname.includes('/admin/network/noc/wallboard');
  }

  async function keepSessionAliveIfActive() {
    if (!browser || authExpiredHandled) return;
    if (!$isAuthenticated) return;

    const isWallboard = isWallboardPath($page.url.pathname);
    if (document.hidden && !isWallboard) return;

    const timeoutMs = resolveSessionTimeoutMs();
    const idleMs = Date.now() - lastUserActivityAt;
    // Outside wallboard, let truly idle sessions expire naturally.
    if (!isWallboard && idleMs >= timeoutMs) return;

    try {
      await checkAuth();
    } catch {
      // Any auth failure is handled by existing auth-expired flow.
    }
  }

  function handleAuthExpired(event: Event) {
    if (typeof window === 'undefined') return;
    if (authExpiredHandled) return;
    authExpiredHandled = true;

    const detail = (event as CustomEvent<{ reason?: string }>)?.detail;
    debugLog('auth-expired-event', {
      reason: detail?.reason || null,
      path: window.location.pathname,
    });

    // Ensure in-memory state is reset immediately.
    disconnectWebSocket();
    resetNotificationsState();
    logout();

    if (!window.location.pathname.startsWith('/login')) {
      goto('/login?reason=expired');
    }
  }

  function isDebugEnabled() {
    if (typeof window === 'undefined') return false;
    const qs = new URLSearchParams(window.location.search);
    return qs.get('debug') === '1' || localStorage.getItem('debug_routing') === '1';
  }

  function debugLog(message: string, meta?: Record<string, unknown>) {
    if (!isDebugEnabled()) return;
    console.log(`[root-layout] ${message}`, meta || {});
  }

  async function loadDeferredGlobals() {
    const [{ Toaster }, { default: GlobalUploads }] = await Promise.all([
      import('svelte-sonner'),
      import('$lib/components/layout/GlobalUploads.svelte'),
    ]);
    ToasterComponent = Toaster;
    GlobalUploadsComponent = GlobalUploads;
  }

  async function initializeLocale() {
    try {
      await appSettings.init();
      await waitLocale();
      i18nReady = true;
    } catch (error) {
      console.error('[root-layout] Failed to initialize locale:', error);
      i18nReady = true;
    }
  }

  async function lookupCustomDomainIfNeeded(hostname: string) {
    const knownSlug = getSlugFromDomain(hostname);
    const isMainPlatformDomain = isPlatformDomain(hostname);
    debugLog('domain-check', { hostname, knownSlug, isMainPlatformDomain });

    if (
      !shouldLookupCustomDomain({
        hostname,
        knownSlug,
        isPlatformDomain: isMainPlatformDomain,
      })
    ) {
      return;
    }

    try {
      const apiUrl = getApiBaseUrl();
      let res = await fetch(`${apiUrl}/public/domains/${encodeURIComponent(hostname)}`);
      if (res.status === 404) {
        res = await fetch(`${apiUrl}/public/tenant-lookup?domain=${encodeURIComponent(hostname)}`);
      }

      if (!res.ok) return;
      const tenant = await res.json();
      if (tenant && tenant.slug) {
        debugLog('domain-lookup-success-reload', {
          hostname,
          slug: tenant.slug,
        });
        await import('$lib/utils/domain').then((m) => m.cacheDomainMapping(hostname, tenant.slug));
        window.location.reload();
      }
    } catch (error) {
      console.warn('[Domain] Failed to lookup custom domain:', error);
    }
  }

  function registerServiceWorker() {
    if (!('serviceWorker' in navigator)) return;
    void navigator.serviceWorker.register('/sw.js').catch((error) => {
      console.error('[SW] Registration failed:', error);
    });
  }

  function syncRealtimeConnections() {
    if (!$isAuthenticated) return;
    debugLog('ws-connect', { isAuthenticated: $isAuthenticated });
    connectWebSocket();
    refreshUnreadCount();
  }

  function applyMaintenanceRedirect(currentPath: string) {
    const settings = $appSettings as any;
    const isMaintenanceMode =
      settings.maintenance_mode === true || settings.maintenance_mode === 'true';
    const allowedPaths = ['/login', '/maintenance', '/install', '/superadmin'];
    const isAllowedPath = allowedPaths.some((p) => currentPath.startsWith(p));

    if (isMaintenanceMode && !$isSuperAdmin && !isAllowedPath) {
      debugLog('maintenance-redirect', { path: currentPath });
      goto('/maintenance');
      return true;
    }

    return false;
  }

  onMount(async () => {
    if (typeof window !== 'undefined') {
      const legacyPath = normalizeLegacyBasePath($page.url.pathname, $page.url.search);
      if (legacyPath) {
        goto(legacyPath, { replaceState: true });
        return;
      }

      // Track real user activity, so idle users still expire by server timeout.
      const events = ['mousemove', 'mousedown', 'keydown', 'touchstart', 'scroll'];
      for (const ev of events) {
        window.addEventListener(ev, markUserActivity, { passive: true });
      }
      markUserActivity();
      keepAliveHandle = setInterval(() => {
        void keepSessionAliveIfActive();
      }, 30_000);
    }

    if (typeof window !== 'undefined') {
      window.addEventListener('app:auth-expired', handleAuthExpired as EventListener);
    }
    try {
      debugLog('boot-start', { path: $page.url.pathname, host: window.location.hostname });
      theme.init();
      const hostname = window.location.hostname;
      appLogo.init();
      const settingsTask = initializeLocale();
      void loadDeferredGlobals();
      registerServiceWorker();

      const [authOk, isInstalled] = await Promise.all([
        checkAuth(),
        install.checkIsInstalled(),
        lookupCustomDomainIfNeeded(hostname),
      ]);
      const currentPath = $page.url.pathname;

      if (!isInstalled) {
        debugLog('install-state', { isInstalled, path: currentPath });
        if (currentPath !== '/install') {
          goto('/install');
        }
      } else {
        debugLog('app-installed-check-auth', { path: currentPath });
        if (currentPath === '/install') {
          console.log('App installed, leaving /install page for /login');
          goto('/login');
        }
        if (authOk) {
          syncRealtimeConnections();
        }
      }

      loading = false;
      await settingsTask;
      if (isInstalled) {
        if (applyMaintenanceRedirect(currentPath)) return;
        if (authOk) {
          syncRealtimeConnections();
        }
      }
    } catch (e) {
      console.error('Critical Error during app initialization in +layout.svelte:', e);
    } finally {
      loading = false;
    }
  });

  // Disconnect WebSocket when app unloads
  onDestroy(() => {
    if (typeof window !== 'undefined') {
      const events = ['mousemove', 'mousedown', 'keydown', 'touchstart', 'scroll'];
      for (const ev of events) {
        window.removeEventListener(ev, markUserActivity as EventListener);
      }
    }
    if (keepAliveHandle) {
      clearInterval(keepAliveHandle);
      keepAliveHandle = null;
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('app:auth-expired', handleAuthExpired as EventListener);
    }
    disconnectWebSocket();
  });

  // Keep WS connection in sync with auth state (important after login without full reload).
  $: if (browser && $isAuthenticated) {
    connectWebSocket();
    refreshUnreadCount();
  } else if (browser && !$isAuthenticated) {
    disconnectWebSocket();
  }
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
  {#if ToasterComponent}
    <svelte:component this={ToasterComponent} />
  {/if}
  {#if GlobalUploadsComponent}
    <svelte:component this={GlobalUploadsComponent} />
  {/if}
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
