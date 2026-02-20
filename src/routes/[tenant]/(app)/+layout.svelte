<script lang="ts">
  import Sidebar from '$lib/components/layout/Sidebar.svelte';
  import Topbar from '$lib/components/layout/Topbar.svelte';
  import AnnouncementBanner from '$lib/components/layout/AnnouncementBanner.svelte';
  import { isAuthenticated, isSuperAdmin, is2FARequiredButDisabled } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { page } from '$app/stores';
  import { user } from '$lib/stores/auth';
  import { getSlugFromDomain, isPlatformDomain } from '$lib/utils/domain';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  let { children } = $props();

  // Determine if we are on a custom domain that matches the current tenant
  let domainSlug = $derived(getSlugFromDomain($page.url.hostname));
  let isCustomDomain = $derived(domainSlug && domainSlug === $user?.tenant_slug);
  let onPlatformDomainForPrefix = $derived(isPlatformDomain($page.url.hostname));

  // If on custom domain, prefix is empty. Otherwise, use slug.
  let tenantPrefix = $derived(
    $user?.tenant_slug && !isCustomDomain && !onPlatformDomainForPrefix
      ? `/${$user.tenant_slug}`
      : '',
  );
  const RESERVED_APP_SEGMENTS = new Set([
    'admin',
    'dashboard',
    'profile',
    'support',
    'notifications',
    'announcements',
    'storage',
  ]);

  function isDebugEnabled() {
    if (typeof window === 'undefined') return false;
    const qs = new URLSearchParams(window.location.search);
    return qs.get('debug') === '1' || localStorage.getItem('debug_routing') === '1';
  }

  function debugLog(message: string, meta?: Record<string, unknown>) {
    if (!isDebugEnabled()) return;
    console.log(`[tenant-layout] ${message}`, meta || {});
  }

  // Reactive Auth Guard & Tenant Scoping
  $effect(() => {
    if (!$isAuthenticated || !$user) return;

    const currentHost = $page.url.hostname;
    const userCustomDomain = ($user as any)?.tenant_custom_domain || ($user as any)?.custom_domain;
    const currentSlug = $page.params.tenant;
    const userSlug = $user?.tenant_slug;
    const onPlatformDomain = isPlatformDomain(currentHost);
    const canonicalPath = $page.url.pathname.replace(/^\/[^/]+/, '') || '/';
    const currentSlugLooksLikeAppRoot = !!currentSlug && RESERVED_APP_SEGMENTS.has(currentSlug);

    debugLog('guard-check', {
      host: currentHost,
      path: $page.url.pathname,
      currentSlug,
      userSlug,
      onPlatformDomain,
      tenantPrefix,
    });

    // Keep main domain URL clean: never expose /:tenant/... in browser URL.
    if (
      onPlatformDomain &&
      currentSlug &&
      !currentSlugLooksLikeAppRoot &&
      $page.url.pathname.startsWith(`/${currentSlug}`)
    ) {
      debugLog('canonicalize-main-domain-path', {
        from: $page.url.pathname,
        to: canonicalPath,
      });
      goto(canonicalPath);
      return;
    }

    if (userCustomDomain && currentHost !== userCustomDomain && !$isSuperAdmin && !onPlatformDomain) {
      debugLog('domain-mismatch-logout', {
        currentHost,
        expectedDomain: userCustomDomain,
        isSuperAdmin: $isSuperAdmin,
      });
      console.warn(`[Layout] Domain Mismatch! User belongs to ${userCustomDomain}. Logging out.`);
      // Domain Mismatch -> Logout and redirect to login
      import('$lib/stores/auth').then((m) => m.logout());
      goto('/login');
      return;
    }

    // 2FA Enforcement
    if ($is2FARequiredButDisabled && !$page.url.pathname.includes('/profile')) {
      debugLog('redirect-2fa-required', {
        to: `${tenantPrefix}/profile?2fa_required=true`,
      });
      goto(`${tenantPrefix}/profile?2fa_required=true`);
      return;
    }

    if (
      currentSlug &&
      userSlug &&
      currentSlug.toLowerCase() !== userSlug.toLowerCase() &&
      !currentSlugLooksLikeAppRoot
    ) {
      console.warn(`[Layout] Tenant Mismatch! User ${userSlug} tried to access ${currentSlug}`);
      // Keep session and normalize route.
      const restPath = $page.url.pathname.replace(/^\/[^/]+/, '') || '/';
      if (onPlatformDomain) {
        debugLog('tenant-mismatch-normalize-platform', {
          from: $page.url.pathname,
          to: restPath,
        });
        goto(restPath);
      } else {
        debugLog('tenant-mismatch-normalize-tenant', {
          from: $page.url.pathname,
          to: `/${userSlug}${restPath}`,
        });
        goto(`/${userSlug}${restPath}`);
      }
    }
  });

  onMount(() => {
    const hasStoredToken =
      (typeof localStorage !== 'undefined' &&
        (localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token'))) ||
      false;

    // Avoid false redirect while auth store is still hydrating/validating.
    if (!$isAuthenticated && !hasStoredToken) {
      debugLog('redirect-login-no-session', {
        isAuthenticated: $isAuthenticated,
        hasStoredToken: !!hasStoredToken,
        path: $page.url.pathname,
      });
      goto('/login');
      return;
    }

    // Check maintenance mode on mount
    const settings = $appSettings as any;
    const isMaintenanceMode =
      settings.maintenance_mode === true || settings.maintenance_mode === 'true';

    if (isMaintenanceMode && !$isSuperAdmin) {
      goto('/maintenance');
    }
  });

  let mobileOpen = $state(false);
</script>

<div class="app-shell">
  <!-- Sidebar sits on the base layer -->
  <Sidebar bind:isMobileOpen={mobileOpen} />

  <!-- Main Area is a floating card -->
  <div class="main-viewport">
    <div class="content-surface">
      <Topbar onMobileMenuClick={() => (mobileOpen = !mobileOpen)} />
      <AnnouncementBanner />
      <div class="scroll-area">
        {@render children()}
      </div>
    </div>
  </div>
</div>

<style>
  .app-shell {
    display: flex;
    height: calc(100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom));
    min-height: calc(
      100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom)
    ); /* Prevent body scrolling caused by global safe-area padding */
    width: 100%;
    background: var(--bg-app); /* Background dasar aplikasi */
    overflow: hidden;
  }

  .main-viewport {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: clamp(6px, 1vw, 12px);
    padding-left: 0; /* Sidebar occupies the left edge */
    min-height: 0; /* allow .scroll-area to be the scroller */
  }

  .content-surface {
    flex: 1;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg); /* Sudut membulat modern */
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: var(--shadow-sm);
    position: relative;
    min-height: 0; /* allow .scroll-area to be the scroller */
  }

  .scroll-area {
    flex: 1;
    overflow-y: auto;
    position: relative;
    padding-bottom: env(safe-area-inset-bottom);
    min-height: 0;
    overscroll-behavior: contain;
  }

  @media (max-width: 900px) {
    .main-viewport {
      padding: clamp(4px, 2vw, 10px);
    }

    .content-surface {
      border-radius: var(--radius-md);
      border-left: none;
      border-right: none;
    }
  }
</style>
