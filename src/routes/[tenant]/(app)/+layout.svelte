<script lang="ts">
  import Sidebar from '$lib/components/layout/Sidebar.svelte';
  import Topbar from '$lib/components/layout/Topbar.svelte';
  import { isAuthenticated, isSuperAdmin, is2FARequiredButDisabled } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { page } from '$app/stores';
  import { user } from '$lib/stores/auth';
  import { getSlugFromDomain } from '$lib/utils/domain';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  let { children } = $props();

  // Determine if we are on a custom domain that matches the current tenant
  let domainSlug = $derived(getSlugFromDomain($page.url.hostname));
  let isCustomDomain = $derived(domainSlug && domainSlug === $user?.tenant_slug);

  // If on custom domain, prefix is empty. Otherwise, use slug.
  let tenantPrefix = $derived($user?.tenant_slug && !isCustomDomain ? `/${$user.tenant_slug}` : '');

  // Reactive Auth Guard & Tenant Scoping
  $effect(() => {
    const currentHost = $page.url.hostname;
    const userCustomDomain = ($user as any)?.custom_domain;
    const currentSlug = $page.params.tenant;
    const userSlug = $user?.tenant_slug;

    if (userCustomDomain && currentHost !== userCustomDomain && !$isSuperAdmin) {
      console.warn(`[Layout] Domain Mismatch! User belongs to ${userCustomDomain}. Logging out.`);
      // Domain Mismatch -> Logout and redirect to login
      import('$lib/stores/auth').then((m) => m.logout());
      goto('/login');
      return;
    }

    // 2FA Enforcement
    if ($is2FARequiredButDisabled && !$page.url.pathname.includes('/profile')) {
      goto(`${tenantPrefix}/profile?2fa_required=true`);
      return;
    }

    if (currentSlug && userSlug && currentSlug.toLowerCase() !== userSlug.toLowerCase()) {
      console.warn(`[Layout] Tenant Mismatch! User ${userSlug} tried to access ${currentSlug}`);
      // Strict Isolation: Logout user if they try to access a different tenant's area
      import('$lib/stores/auth').then((m) => m.logout());
      goto(`/${currentSlug}`);
    }
  });

  onMount(() => {
    if (!$isAuthenticated) {
      goto('/');
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
