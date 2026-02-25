<script lang="ts">
  import Sidebar from '$lib/components/layout/Sidebar.svelte';
  import Topbar from '$lib/components/layout/Topbar.svelte';
  import AnnouncementBanner from '$lib/components/layout/AnnouncementBanner.svelte';
  import { isAuthenticated, isSuperAdmin, is2FARequiredButDisabled, can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { page } from '$app/stores';
  import { user } from '$lib/stores/auth';
  import { resolveTenantContext, APP_ROOT_SEGMENTS } from '$lib/utils/tenantRouting';
  import { isPlatformDomain } from '$lib/utils/domain';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  let { children } = $props();

  let tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  let tenantPrefix = $derived(tenantCtx.tenantPrefix);
  const RESERVED_APP_SEGMENTS = new Set<string>(APP_ROOT_SEGMENTS as readonly string[]);

  function isDebugEnabled() {
    if (typeof window === 'undefined') return false;
    const qs = new URLSearchParams(window.location.search);
    return qs.get('debug') === '1' || localStorage.getItem('debug_routing') === '1';
  }

  function debugLog(message: string, meta?: Record<string, unknown>) {
    if (!isDebugEnabled()) return;
    console.log(`[tenant-layout] ${message}`, meta || {});
  }

  function hasAnyAdminCapability() {
    if ($isSuperAdmin) return true;
    return (
      $can('access', 'admin') ||
      $can('read', 'network_routers') ||
      $can('manage', 'network_routers') ||
      $can('read', 'work_orders') ||
      $can('manage', 'work_orders') ||
      $can('read', 'customers') ||
      $can('manage', 'customers') ||
      $can('read', 'billing') ||
      $can('manage', 'billing') ||
      $can('read', 'team') ||
      $can('read', 'roles') ||
      $can('read', 'settings') ||
      $can('read', 'audit_logs') ||
      $can('read_all', 'support') ||
      $can('read', 'email_outbox')
    );
  }

  function canAccessAdminPath(path: string) {
    if (!path.startsWith('/admin')) return true;
    if ($isSuperAdmin) return true;

    // /admin home
    if (path === '/admin' || path === '/admin/') {
      return hasAnyAdminCapability();
    }

    if (path === '/admin/network' || path === '/admin/network/') {
      return (
        $can('read', 'network_routers') ||
        $can('manage', 'network_routers') ||
        $can('read', 'pppoe') ||
        $can('manage', 'pppoe') ||
        $can('read', 'isp_packages') ||
        $can('manage', 'isp_packages') ||
        $can('read', 'work_orders') ||
        $can('manage', 'work_orders')
      );
    }
    if (path.startsWith('/admin/network/pppoe')) {
      return $can('read', 'pppoe') || $can('manage', 'pppoe');
    }
    if (path.startsWith('/admin/network/packages')) {
      return $can('read', 'isp_packages') || $can('manage', 'isp_packages');
    }
    if (path.startsWith('/admin/network/installations')) {
      return $can('read', 'work_orders') || $can('manage', 'work_orders');
    }
    if (
      path.startsWith('/admin/network/noc') ||
      path.startsWith('/admin/network/alerts') ||
      path.startsWith('/admin/network/incidents') ||
      path.startsWith('/admin/network/logs') ||
      path.startsWith('/admin/network/routers') ||
      path.startsWith('/admin/network/ppp-profiles') ||
      path.startsWith('/admin/network/ip-pools')
    ) {
      return $can('read', 'network_routers') || $can('manage', 'network_routers');
    }
    if (path.startsWith('/admin/customers')) {
      return $can('read', 'customers') || $can('manage', 'customers');
    }
    if (path.startsWith('/admin/invoices')) {
      return $can('read', 'billing') || $can('manage', 'billing');
    }
    if (path.startsWith('/admin/subscription')) {
      return $can('read', 'billing') || $can('manage', 'billing');
    }
    if (path.startsWith('/admin/billing-logs')) {
      return $can('read', 'billing') || $can('manage', 'billing');
    }
    if (path.startsWith('/admin/announcements')) {
      return $can('read', 'announcements') || $can('manage', 'announcements');
    }
    if (path.startsWith('/admin/backups')) {
      return (
        $can('read', 'backups') ||
        $can('create', 'backups') ||
        $can('download', 'backups') ||
        $can('restore', 'backups') ||
        $can('delete', 'backups')
      );
    }
    if (path.startsWith('/admin/team')) {
      return (
        $can('read', 'team') ||
        $can('create', 'team') ||
        $can('update', 'team') ||
        $can('delete', 'team')
      );
    }
    if (path.startsWith('/admin/roles')) {
      return (
        $can('read', 'roles') ||
        $can('create', 'roles') ||
        $can('update', 'roles') ||
        $can('delete', 'roles')
      );
    }
    if (path.startsWith('/admin/settings')) {
      return $can('read', 'settings') || $can('update', 'settings') || $can('delete', 'settings');
    }
    if (path.startsWith('/admin/audit-logs')) {
      return $can('read', 'audit_logs');
    }
    if (path.startsWith('/admin/support')) {
      return $can('read_all', 'support') || $can('read', 'support');
    }
    if (path.startsWith('/admin/storage')) {
      return $can('read', 'storage') || $can('upload', 'storage') || $can('delete', 'storage');
    }
    if (path.startsWith('/admin/email-outbox')) {
      return (
        $can('read', 'email_outbox') ||
        $can('retry', 'email_outbox') ||
        $can('delete', 'email_outbox')
      );
    }

    // Unknown admin sub-route -> deny by default.
    return false;
  }

  // Reactive Auth Guard & Tenant Scoping
  $effect(() => {
    if (!$isAuthenticated || !$user) return;

    const currentHost = $page.url.hostname;
    const pathname = $page.url.pathname || '/';
    const userCustomDomain = ($user as any)?.tenant_custom_domain || ($user as any)?.custom_domain;
    const currentSlug = $page.params.tenant;
    const userSlug = $user?.tenant_slug;
    const onPlatformDomain = isPlatformDomain(currentHost);
    const currentSlugLooksLikeAppRoot = !!currentSlug && RESERVED_APP_SEGMENTS.has(currentSlug);
    const hasTenantPrefixInPath =
      !!currentSlug &&
      !currentSlugLooksLikeAppRoot &&
      (pathname === `/${currentSlug}` || pathname.startsWith(`/${currentSlug}/`));
    const canonicalPath = hasTenantPrefixInPath
      ? pathname.replace(new RegExp(`^/${currentSlug}`), '') || '/'
      : pathname;

    debugLog('guard-check', {
      host: currentHost,
      path: pathname,
      currentSlug,
      userSlug,
      onPlatformDomain,
      tenantPrefix: tenantCtx.tenantPrefix,
    });

    // Keep main domain URL clean: never expose /:tenant/... in browser URL.
    if (
      onPlatformDomain &&
      hasTenantPrefixInPath
    ) {
      debugLog('canonicalize-main-domain-path', {
        from: pathname,
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
      const restPath = hasTenantPrefixInPath
        ? pathname.replace(new RegExp(`^/${currentSlug}`), '') || '/'
        : pathname;
      if (onPlatformDomain) {
        debugLog('tenant-mismatch-normalize-platform', {
          from: pathname,
          to: restPath,
        });
        goto(restPath);
      } else {
        debugLog('tenant-mismatch-normalize-tenant', {
          from: pathname,
          to: `/${userSlug}${restPath}`,
        });
        goto(`/${userSlug}${restPath}`);
      }
    }

    // Global admin route guard:
    // deny rendering /admin pages when user has no matching permission.
    if (canonicalPath.startsWith('/admin') && !canAccessAdminPath(canonicalPath)) {
      debugLog('redirect-unauthorized-admin-route', {
        canonicalPath,
        role: $user?.role,
      });
      if (!$page.url.pathname.startsWith('/unauthorized')) {
        goto('/unauthorized');
      }
      return;
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
