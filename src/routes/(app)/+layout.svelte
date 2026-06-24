<script lang="ts">
  import Sidebar from '$lib/components/layout/Sidebar.svelte';
  import Topbar from '$lib/components/layout/Topbar.svelte';
  import AnnouncementBanner from '$lib/components/layout/AnnouncementBanner.svelte';
  import { isAuthenticated, isSuperAdmin, is2FARequiredButDisabled, can, checkAuth } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { page } from '$app/stores';
  import { user } from '$lib/stores/auth';
  import { resolveTenantContext, APP_ROOT_SEGMENTS } from '$lib/utils/tenantRouting';
  import { isPlatformDomain } from '$lib/utils/domain';
  import { canAccessNetworkMap } from '$lib/utils/adminNetworkAccess';
  import { canAccessServiceCatalog } from '$lib/utils/serviceCatalogAccess';
  import { canAccessCustomerDashboard } from '$lib/utils/appLanding';
  import ProfileModal from '$lib/components/profile/ProfileModal.svelte';
  import NotificationModal from '$lib/components/notifications/NotificationModal.svelte';
  import { openProfileModal, profileModal, setProfileModalLock } from '$lib/stores/profileModal';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { secureGetItem } from '$lib/utils/tauri-store';

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
      $can('read', 'network_noc') ||
      $can('read', 'network_alerts') ||
      $can('read', 'network_incidents') ||
      $can('read', 'network_logs') ||
      $can('read', 'router_inventory') ||
      $can('read', 'ppp_profiles') ||
      $can('read', 'ip_pools') ||
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
      $can('read', 'support') ||
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
        $can('read', 'network_noc') ||
        $can('read', 'network_alerts') ||
        $can('read', 'network_incidents') ||
        $can('read', 'network_logs') ||
        $can('read', 'router_inventory') ||
        $can('read', 'network_topology') ||
        $can('read', 'ppp_profiles') ||
        $can('read', 'ip_pools') ||
        $can('read', 'pppoe') ||
        $can('manage', 'pppoe') ||
        $can('read', 'dhcp_static') ||
        $can('manage', 'dhcp_static') ||
        $can('read', 'isp_packages') ||
        $can('manage', 'isp_packages') ||
        $can('read', 'work_orders') ||
        $can('manage', 'work_orders')
      );
    }
    if (path.startsWith('/admin/network/pppoe')) {
      return $can('read', 'pppoe') || $can('manage', 'pppoe');
    }
    if (path.startsWith('/admin/network/dhcp-static')) {
      return $can('read', 'dhcp_static') || $can('manage', 'dhcp_static');
    }
    if (path.startsWith('/admin/network/import')) {
      return $can('manage', 'pppoe');
    }
    if (path.startsWith('/admin/services') || path.startsWith('/admin/network/packages')) {
      return canAccessServiceCatalog(
        $user,
        $can('read', 'isp_packages'),
        $can('manage', 'isp_packages'),
      );
    }
    if (path.startsWith('/admin/network/installations')) {
      return $can('read', 'work_orders') || $can('manage', 'work_orders');
    }
    if (path.startsWith('/admin/network/noc')) {
      return $can('read', 'network_noc') || $can('manage', 'network_noc');
    }
    if (path.startsWith('/admin/network/map')) {
      return canAccessNetworkMap($can);
    }
    if (path.startsWith('/admin/network/alerts')) {
      return $can('read', 'network_alerts') || $can('manage', 'network_alerts');
    }
    if (path.startsWith('/admin/network/incidents')) {
      return $can('read', 'network_incidents') || $can('manage', 'network_incidents');
    }
    if (path.startsWith('/admin/network/logs')) {
      return $can('read', 'network_logs') || $can('manage', 'network_logs');
    }
    if (path.startsWith('/admin/network/routers')) {
      return $can('read', 'router_inventory') || $can('manage', 'router_inventory');
    }
    if (path.startsWith('/admin/network/ppp-profiles')) {
      return $can('read', 'ppp_profiles') || $can('manage', 'ppp_profiles');
    }
    if (path.startsWith('/admin/network/ip-pools')) {
      return $can('read', 'ip_pools') || $can('manage', 'ip_pools');
    }
    if (path.startsWith('/admin/customers')) {
      return $can('read', 'customers') || $can('manage', 'customers');
    }
    if (path.startsWith('/admin/message-templates')) {
      return (
        $can('read', 'communication_templates') || $can('manage', 'communication_templates')
      );
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
    if (path.startsWith('/admin/billing')) {
      return $can('read', 'billing') || $can('manage', 'billing');
    }
    if (path.startsWith('/admin/announcements')) {
      if (path === '/admin/announcements' || path === '/admin/announcements/') {
        return $can('manage', 'announcements');
      }
      return hasAnyAdminCapability();
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
      return $can('read', 'support') || $can('read_all', 'support');
    }
    if (path.startsWith('/admin/storage')) {
      return $can('read', 'storage_console');
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
    if (onPlatformDomain && hasTenantPrefixInPath) {
      debugLog('canonicalize-main-domain-path', {
        from: pathname,
        to: canonicalPath,
      });
      goto(canonicalPath);
      return;
    }

    if (
      userCustomDomain &&
      currentHost !== userCustomDomain &&
      !$isSuperAdmin &&
      !onPlatformDomain
    ) {
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

    if (canonicalPath.startsWith('/dashboard') && !canAccessCustomerDashboard($user)) {
      debugLog('redirect-internal-user-from-customer-dashboard', {
        canonicalPath,
        role: $user?.role,
        target: `${tenantPrefix}/admin`,
      });
      goto(`${tenantPrefix}/admin`);
      return;
    }

    if ($is2FARequiredButDisabled) {
      debugLog('open-profile-modal-2fa-required', {
        tab: 'security',
      });
      if (!$profileModal.open || $profileModal.tab !== 'security' || !$profileModal.locked) {
        openProfileModal({ tab: 'security', locked: true, reason: '2fa_required' });
      }
      return;
    }

    if ($profileModal.locked) {
      setProfileModalLock(false);
    }
  });

  // Force leaving protected app routes when session is gone/expired.
  $effect(() => {
    const hasStoredToken =
      (typeof window !== 'undefined' && !!secureGetItem('auth_token')) || false;

    if (!$isAuthenticated && !hasStoredToken) {
      debugLog('redirect-login-session-missing', {
        isAuthenticated: $isAuthenticated,
        hasStoredToken: !!hasStoredToken,
        path: $page.url.pathname,
      });
      goto('/login');
    }
  });

  onMount(() => {
    let cancelled = false;
    const runGuard = async () => {
      const hasStoredToken =
        (typeof window !== 'undefined' && !!secureGetItem('auth_token')) || false;

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

      if (hasStoredToken) {
        const valid = await checkAuth();
        if (cancelled) return;
        if (!valid) {
          goto('/login?reason=expired');
          return;
        }
      }

      // Check maintenance mode on mount
      const settings = $appSettings as any;
      const isMaintenanceMode =
        settings.maintenance_mode === true || settings.maintenance_mode === 'true';

      if (isMaintenanceMode && !$isSuperAdmin) {
        goto('/maintenance');
      }
    };

    void runGuard();
    return () => {
      cancelled = true;
    };
  });

  let mobileOpen = $state(false);
  const isCompactMapEmbed = $derived.by(() => {
    const path = $page.url.pathname || '';
    if (!path.includes('/admin/network/map')) return false;
    return $page.url.searchParams.get('compact') === '1';
  });
</script>

{#if isCompactMapEmbed}
  <div class="embed-shell">
    {@render children()}
  </div>
{:else}
  <div class="app-shell">
    <!-- Sidebar sits on the base layer -->
    <Sidebar bind:isMobileOpen={mobileOpen} />

    <!-- Main area -->
    <div class="main-viewport">
      <div class="content-surface">
        <Topbar onMobileMenuClick={() => (mobileOpen = !mobileOpen)} />
        <AnnouncementBanner />
        <div class="scroll-area">
          {@render children()}
        </div>
      </div>
    </div>
    <ProfileModal />
    <NotificationModal />
  </div>
{/if}

<style>
  .embed-shell {
    width: 100%;
    height: 100%;
    min-height: 100dvh;
    background: transparent;
    overflow: hidden;
  }
  .app-shell {
    display: flex;
    height: calc(100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom));
    min-height: calc(
      100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom)
    ); /* Prevent body scrolling caused by global safe-area padding */
    width: 100%;
    background: var(--bg-app);
    overflow: hidden;
  }

  .main-viewport {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: clamp(6px, 0.9vw, 12px);
    padding-left: 0; /* Sidebar occupies the left edge */
    width: 100%;
    min-width: 0;
    min-height: 0; /* allow .scroll-area to be the scroller */
  }

  .content-surface {
    flex: 1;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: var(--shadow-sm);
    position: relative;
    min-width: 0;
    min-height: 0; /* allow .scroll-area to be the scroller */
  }

  .scroll-area {
    flex: 1;
    overflow-y: auto;
    position: relative;
    padding-bottom: env(safe-area-inset-bottom);
    min-width: 0;
    min-height: 0;
    overscroll-behavior: contain;
  }

  @media (max-width: 900px) {
    .main-viewport {
      padding: 0;
    }

    .content-surface {
      border-radius: 0;
      border: 0;
      box-shadow: none;
    }
  }
</style>
