<script lang="ts">
  import { page } from '$app/stores';
  import { user, isAdmin, isSuperAdmin, logout, can, authVersion } from '$lib/stores/auth';
  import { appName } from '$lib/stores/settings';
  import { appLogo } from '$lib/stores/logo';
  import { isSidebarCollapsed } from '$lib/stores/ui';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import { canAccessNetworkMap } from '$lib/utils/adminNetworkAccess';
  import { api, type Invoice } from '$lib/api/client';
  import Icon from '../ui/Icon.svelte';

  let { isMobileOpen = $bindable(false) } = $props();

  const tenantPrefix = '';

  type NavItem = {
    label: string;
    icon: string;
    href: string;
    show?: boolean;
  };

  type NavSection = {
    id: string;
    title?: string;
    items: NavItem[];
    default_open?: boolean;
  };

  function visibleItems(items: NavItem[]) {
    return items.filter((i) => i.show !== false);
  }

  let packageBillingAlert = $state<'none' | 'pending' | 'overdue'>('none');
  let packageAlertTicking = false;

  function isPastInvoiceDue(inv: Invoice): boolean {
    const raw = inv.due_date || inv.created_at;
    if (!raw) return false;
    const ms = new Date(raw).getTime();
    if (!Number.isFinite(ms)) return false;
    return ms < Date.now();
  }

  function canShowPackageBillingAlert() {
    return (
      !$isAdmin && !$isSuperAdmin && (canAccessCustomerPackages() || $can('read_own', 'customers'))
    );
  }

  function canAccessCustomerPackages() {
    const role = String(($user as any)?.tenant_role || $user?.role || '')
      .trim()
      .toLowerCase();
    return role === 'customer';
  }

  async function refreshPackageOverdueAlert() {
    if (packageAlertTicking) return;
    if (!canShowPackageBillingAlert()) {
      packageBillingAlert = 'none';
      return;
    }

    packageAlertTicking = true;
    try {
      const invoices = await api.payment.listInvoices();
      const pendingInvoices = (invoices || []).filter((inv) => inv.status === 'pending');
      if (pendingInvoices.some((inv) => isPastInvoiceDue(inv))) {
        packageBillingAlert = 'overdue';
      } else if (pendingInvoices.length > 0) {
        packageBillingAlert = 'pending';
      } else {
        packageBillingAlert = 'none';
      }
    } catch {
      // Non-blocking: keep sidebar responsive even if check fails.
      packageBillingAlert = 'none';
    } finally {
      packageAlertTicking = false;
    }
  }

  function shouldShowPackageAlert(item: NavItem) {
    if (packageBillingAlert === 'none') return false;
    return item.href === `${tenantPrefix}/dashboard/services`;
  }

  function packageAlertLabel() {
    if (packageBillingAlert === 'overdue') {
      return (
        $t('sidebar.packages_overdue_alert') ||
        'There is an overdue invoice. Please review package billing.'
      );
    }
    return (
      $t('sidebar.packages_pending_alert') ||
      'There is a pending invoice. Please review package billing.'
    );
  }

  let appMenuSections = $derived.by(() => {
    // Access $user to create dependency for permissions changes
    const _ = $user?.permissions;

    const sections: NavSection[] = [
      {
        id: 'workspace',
        title: $t('sidebar.sections.workspace') || 'Workspace',
        items: visibleItems([
          {
            label: $t('sidebar.dashboard'),
            icon: 'dashboard',
            href: `${tenantPrefix}/dashboard`,
            show: true,
          },
          {
            label: $t('sidebar.locations') || 'Locations',
            icon: 'map-pin',
            href: `${tenantPrefix}/dashboard/locations`,
            show: true,
          },
          {
            label: $t('sidebar.services') || $t('sidebar.packages') || 'Services',
            icon: 'package',
            href: `${tenantPrefix}/dashboard/services`,
            show: $can('read_own', 'customers') || canAccessCustomerPackages(),
          },
          {
            label: $t('sidebar.invoices') || 'Invoices',
            icon: 'file-text',
            href: `${tenantPrefix}/dashboard/invoices`,
            show: true,
          },
          {
            label: $t('sidebar.announcements') || 'Announcements',
            icon: 'megaphone',
            href: `${tenantPrefix}/announcements`,
            show: true,
          },
        ]),
      },
      {
        id: 'help',
        title: $t('sidebar.sections.help') || 'Help',
        items: visibleItems([
          {
            label: $t('sidebar.support') || 'Support',
            icon: 'life-buoy',
            href: `${tenantPrefix}/support`,
            show: $can('read', 'support') || $can('create', 'support'),
          },
        ]),
      },
    ];

    return sections.filter((s) => s.items.length > 0);
  });

  let superAdminMenuSections = $derived.by(() => {
    const sections: NavSection[] = [
      {
        id: 'platform',
        title: $t('sidebar.sections.platform') || 'Platform',
        items: [
          { label: $t('sidebar.dashboard') || 'Dashboard', icon: 'grid', href: '/superadmin' },
          {
            label: $t('sidebar.tenants') || 'Tenants',
            icon: 'database',
            href: '/superadmin/tenants',
          },
          { label: $t('sidebar.users') || 'Users', icon: 'users', href: '/superadmin/users' },
          {
            label: $t('sidebar.radius') || 'RADIUS',
            icon: 'server',
            href: '/superadmin/radius',
          },
        ],
      },
      {
        id: 'billing',
        title: $t('sidebar.sections.billing') || 'Billing',
        items: [
          { label: $t('sidebar.plans') || 'Plans', icon: 'credit-card', href: '/superadmin/plans' },
          {
            label: $t('sidebar.invoices') || 'Invoices',
            icon: 'file-text',
            href: '/superadmin/invoices',
          },
        ],
      },
      {
        id: 'operations',
        title: $t('sidebar.sections.operations') || 'Operations',
        items: [
          {
            label: $t('sidebar.storage') || 'Storage',
            icon: 'folder',
            href: '/superadmin/storage',
          },
          {
            label: $t('sidebar.backups') || 'Backups',
            icon: 'archive',
            href: '/superadmin/backups',
          },
          { label: $t('sidebar.system') || 'System', icon: 'server', href: '/superadmin/system' },
        ],
      },
      {
        id: 'security',
        title: $t('sidebar.sections.security') || 'Security',
        items: [
          {
            label: $t('sidebar.audit_logs') || 'Audit Logs',
            icon: 'activity',
            href: '/superadmin/audit-logs',
          },
          {
            label: $t('sidebar.settings') || 'Settings',
            icon: 'settings',
            href: '/superadmin/settings',
          },
        ],
      },
    ];

    return sections.filter((s) => s.items.length > 0);
  });

  // Add $user as explicit dependency to force reactivity when user permissions change
  let adminMenuSections = $derived.by(() => {
    // Access $user to create dependency
    const _ = $user?.permissions;

    const sections: NavSection[] = [
      {
        id: 'workspace',
        title: $t('sidebar.sections.workspace') || 'Workspace',
        items: visibleItems([
          {
            label: $t('sidebar.overview'),
            icon: 'shield',
            href: `${tenantPrefix}/admin`,
            show:
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
              $can('read_all', 'support') ||
              $can('read', 'email_outbox'),
          },
          {
            label: $t('sidebar.customers') || 'Customers',
            icon: 'user-check',
            href: `${tenantPrefix}/admin/customers`,
            show: $can('read', 'customers') || $can('manage', 'customers'),
          },
        ]),
      },
      {
        id: 'network',
        title: $t('sidebar.sections.network') || 'Network',
        items: visibleItems([
          {
            label: $t('sidebar.noc') || 'NOC',
            icon: 'activity',
            href: `${tenantPrefix}/admin/network/noc`,
            show: $can('read', 'network_noc') || $can('manage', 'network_noc'),
          },
          {
            label: $t('sidebar.topology_map') || 'Topology Map',
            icon: 'map-pin',
            href: `${tenantPrefix}/admin/network/map`,
            show: canAccessNetworkMap($can),
          },
          {
            label: $t('sidebar.wallboard') || 'Wallboard',
            icon: 'monitor',
            href: `${tenantPrefix}/admin/network/noc/wallboard`,
            show: $can('read', 'network_noc') || $can('manage', 'network_noc'),
          },
          {
            label: $t('sidebar.alerts') || 'Alerts',
            icon: 'alert-triangle',
            href: `${tenantPrefix}/admin/network/alerts`,
            show: $can('read', 'network_alerts') || $can('manage', 'network_alerts'),
          },
          {
            label: $t('sidebar.incidents') || 'Incidents',
            icon: 'alert-triangle',
            href: `${tenantPrefix}/admin/network/incidents`,
            show: $can('read', 'network_incidents') || $can('manage', 'network_incidents'),
          },
          {
            label: $t('sidebar.installations') || 'Installations',
            icon: 'clipboard-list',
            href: `${tenantPrefix}/admin/network/installations`,
            show: $can('read', 'work_orders') || $can('manage', 'work_orders'),
          },
          {
            label: $t('sidebar.logs') || 'Logs',
            icon: 'file-text',
            href: `${tenantPrefix}/admin/network/logs`,
            show: $can('read', 'network_logs') || $can('manage', 'network_logs'),
          },
          {
            label: $t('sidebar.routers') || 'Routers',
            icon: 'router',
            href: `${tenantPrefix}/admin/network/routers`,
            show: $can('read', 'router_inventory') || $can('manage', 'router_inventory'),
          },
          {
            label: $t('sidebar.ppp_profiles') || 'PPP Profiles',
            icon: 'key',
            href: `${tenantPrefix}/admin/network/ppp-profiles`,
            show: $can('read', 'ppp_profiles') || $can('manage', 'ppp_profiles'),
          },
          {
            label: $t('sidebar.ip_pools') || 'IP Pools',
            icon: 'database',
            href: `${tenantPrefix}/admin/network/ip-pools`,
            show: $can('read', 'ip_pools') || $can('manage', 'ip_pools'),
          },
          {
            label: $t('sidebar.services') || $t('sidebar.packages') || 'Services',
            icon: 'package',
            href: `${tenantPrefix}/admin/services`,
            show: $can('read', 'isp_packages') || $can('manage', 'isp_packages'),
          },
          {
            label: $t('sidebar.pppoe') || 'PPPoE',
            icon: 'key',
            href: `${tenantPrefix}/admin/network/pppoe`,
            show: $can('read', 'pppoe') || $can('manage', 'pppoe'),
          },
          {
            label: $t('sidebar.imports') || 'Imports',
            icon: 'download',
            href: `${tenantPrefix}/admin/network/import`,
            show: $can('manage', 'pppoe'),
          },
        ]),
      },
      {
        id: 'access',
        title: $t('sidebar.sections.access') || 'Access',
        items: visibleItems([
          {
            label: $t('sidebar.team'),
            icon: 'users',
            href: `${tenantPrefix}/admin/team`,
            show: $can('read', 'team'),
          },
          {
            label: $t('sidebar.roles'),
            icon: 'lock',
            href: `${tenantPrefix}/admin/roles`,
            show: $can('read', 'roles'),
          },
        ]),
      },
      {
        id: 'compliance',
        title: $t('sidebar.sections.compliance') || 'Compliance',
        items: visibleItems([
          {
            label: $t('sidebar.audit_logs') || 'Audit Logs',
            icon: 'activity',
            href: `${tenantPrefix}/admin/audit-logs`,
            show: $can('read', 'audit_logs'),
          },
        ]),
      },
      {
        id: 'operations',
        title: $t('sidebar.sections.operations') || 'Operations',
        items: visibleItems([
          {
            label: $t('sidebar.storage') || 'Storage',
            icon: 'folder',
            href: `${tenantPrefix}/admin/storage`,
            show: $can('read', 'storage_console'),
          },
          {
            label: $t('sidebar.email_outbox') || 'Email Outbox',
            icon: 'mail',
            href: `${tenantPrefix}/admin/email-outbox`,
            show: $can('read', 'email_outbox'),
          },
          {
            label: $t('sidebar.support') || 'Support',
            icon: 'life-buoy',
            href: `${tenantPrefix}/admin/support`,
            show: $can('read_all', 'support'),
          },
        ]),
      },
      {
        id: 'billing',
        title: $t('sidebar.sections.billing') || 'Billing',
        items: visibleItems([
          {
            label: $t('sidebar.billing') || 'Billing',
            icon: 'file-text',
            href: `${tenantPrefix}/admin/invoices`,
            show: $can('read', 'billing') || $can('manage', 'billing'),
          },
          {
            label: $t('sidebar.collections') || 'Collections',
            icon: 'receipt',
            href: `${tenantPrefix}/admin/invoices/collection`,
            show: $can('read', 'billing') || $can('manage', 'billing'),
          },
        ]),
      },
      {
        id: 'configuration',
        title: $t('sidebar.sections.configuration') || 'Configuration',
        items: visibleItems([
          {
            label: $t('sidebar.settings'),
            icon: 'settings',
            href: `${tenantPrefix}/admin/settings`,
            show: $can('read', 'settings'),
          },
          {
            label: $t('sidebar.announcements') || 'Announcements',
            icon: 'megaphone',
            href: `${tenantPrefix}/admin/announcements`,
            show: $can('manage', 'announcements'),
          },
        ]),
      },
    ];

    return sections.filter((s) => s.items.length > 0);
  });

  let isDropdownOpen = $state(false);

  let isUrlSuperadmin = $derived($page.url.pathname.startsWith('/superadmin'));
  let isUrlAdmin = $derived($page.url.pathname.includes('/admin'));
  let menuScope = $derived(isUrlSuperadmin ? 'superadmin' : isUrlAdmin ? 'admin' : 'app');
  let currentMenuSections = $derived(
    isUrlSuperadmin ? superAdminMenuSections : isUrlAdmin ? adminMenuSections : appMenuSections,
  );

  let openSectionId = $state<string | null>(null);

  onMount(() => {
    let timer: ReturnType<typeof setInterval> | null = null;
    const onVisible = () => {
      if (!document.hidden) void refreshPackageOverdueAlert();
    };

    void refreshPackageOverdueAlert();
    timer = setInterval(() => {
      void refreshPackageOverdueAlert();
    }, 60_000);
    document.addEventListener('visibilitychange', onVisible);

    return () => {
      if (timer) clearInterval(timer);
      document.removeEventListener('visibilitychange', onVisible);
    };
  });

  $effect(() => {
    const _version = $authVersion;
    const _path = $page.url.pathname;
    void refreshPackageOverdueAlert();
  });

  function sectionStorageKey(scope: string) {
    return `sidebar.section_state.${scope}`;
  }

  function isSectionOpen(id: string) {
    if ($isSidebarCollapsed) return true;
    return openSectionId === id;
  }

  function toggleSection(id: string, e?: MouseEvent) {
    e?.stopPropagation();
    // Accordion behavior: keep only one section open at a time.
    // We intentionally do not "close" the active section to avoid an empty sidebar state.
    openSectionId = id;
  }

  $effect(() => {
    if (typeof window === 'undefined') return;

    const key = sectionStorageKey(menuScope);
    let saved: string | null = null;
    try {
      const raw = localStorage.getItem(key);
      if (raw) saved = raw;
    } catch {
      saved = null;
    }

    const first = currentMenuSections[0]?.id || null;

    // Only accept saved section ids that still exist in the current menu.
    const validSaved = saved && currentMenuSections.some((s) => s.id === saved) ? saved : null;

    openSectionId = validSaved || first;
  });

  $effect(() => {
    if (typeof window === 'undefined') return;

    // If there's an active link, keep only that section open.
    // This runs on navigation and doesn't interfere with manual toggles (no nav).
    const _path = $page.url.pathname;
    const activeSection = currentMenuSections.find((s) => s.items.some((it) => isActive(it)))?.id;
    // Important: don't depend on `openSectionId` here, otherwise clicking a section header would
    // immediately be overridden back to the active section.
    if (activeSection) openSectionId = activeSection;
  });

  $effect(() => {
    if (typeof window === 'undefined') return;
    const key = sectionStorageKey(menuScope);
    try {
      if (openSectionId) localStorage.setItem(key, openSectionId);
      else localStorage.removeItem(key);
    } catch {
      // ignore
    }
  });

  // When inside `/superadmin`, we still want a quick jump back to the tenant admin panel.
  const adminPanelHref = '/admin';

  function handleLogout() {
    logout();
    goto('/');
  }

  function navigate(href: string) {
    goto(href);
    isDropdownOpen = false;
    isMobileOpen = false; // Close mobile menu on navigate
  }

  function toggleDropdown(event: MouseEvent) {
    event.stopPropagation();
    isDropdownOpen = !isDropdownOpen;
  }

  function handleWindowClick() {
    if (isDropdownOpen) isDropdownOpen = false;
  }

  function handleEscape(e: KeyboardEvent) {
    if (e.key === 'Escape' && isDropdownOpen) {
      isDropdownOpen = false;
    }
  }

  function isActive(item: { href: string }) {
    const path = $page.url.pathname;

    if (isUrlSuperadmin) {
      if (item.href === '/superadmin') return path === '/superadmin';
      return path.startsWith(item.href);
    }

    if (item.href === `${tenantPrefix}/admin`) return path === item.href;
    if (item.href === `${tenantPrefix}/dashboard`) return path === item.href;

    return path === item.href || path.startsWith(`${item.href}/`);
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleEscape} />

<!-- Mobile Overlay Backdrop -->
{#if isMobileOpen}
  <button
    type="button"
    class="sidebar-overlay"
    onclick={() => (isMobileOpen = false)}
    aria-label={$t('sidebar.close_menu') || 'Close menu'}
    title={$t('sidebar.close_menu') || 'Close menu'}
  ></button>
{/if}

<aside class="sidebar" class:collapsed={$isSidebarCollapsed} class:mobile-open={isMobileOpen}>
  <!-- Header -->
  <div class="sidebar-header">
    <div class="logo-wrapper">
      {#if $appLogo}
        <img src={$appLogo} alt="Logo" class="app-logo" />
      {:else}
        <div class="logo-placeholder">
          <Icon name="app" size={20} />
        </div>
      {/if}
      <span class="app-name">{$appName}</span>
    </div>
  </div>

  <!-- Navigation - use {#key} to force re-render when authVersion changes -->
  {#key $authVersion}
    <nav class="sidebar-nav">
      {#each currentMenuSections as section, idx}
        {#if section.title && !$isSidebarCollapsed}
          <button
            type="button"
            class="nav-section-btn"
            aria-expanded={isSectionOpen(section.id)}
            onclick={(e) => toggleSection(section.id, e)}
          >
            <span class="nav-section-title">{section.title}</span>
            <span class="nav-section-chev" class:open={isSectionOpen(section.id)}>
              <Icon name="chevron-down" size={14} />
            </span>
          </button>
        {/if}

        {#if isSectionOpen(section.id)}
          {#each section.items as item}
            <button
              class="nav-item"
              class:active={isActive(item)}
              aria-label={item.label}
              data-tooltip={item.label}
              onclick={() => navigate(item.href)}
            >
              <Icon name={item.icon} size={18} />
              <span class="label">{item.label}</span>
              {#if shouldShowPackageAlert(item)}
                <span
                  class="item-alert"
                  class:pending={packageBillingAlert === 'pending'}
                  class:overdue={packageBillingAlert === 'overdue'}
                  title={packageAlertLabel()}
                  aria-label={packageAlertLabel()}
                >
                  <Icon
                    name={packageBillingAlert === 'pending' ? 'alert-circle' : 'alert-triangle'}
                    size={12}
                  />
                </span>
              {/if}
            </button>
          {/each}
        {/if}

        {#if idx < currentMenuSections.length - 1}
          <div class="nav-divider" aria-hidden="true"></div>
        {/if}
      {/each}
    </nav>
  {/key}

  <!-- Footer / Profile -->
  <div class="sidebar-footer">
    {#if $isSuperAdmin && !isUrlSuperadmin}
      <button
        class="context-btn"
        onclick={() => goto('/superadmin')}
        aria-label={$t('sidebar.super_admin') || 'Super Admin'}
        data-tooltip={$t('sidebar.super_admin') || 'Super Admin'}
      >
        <Icon name="server" size={16} />
        <span class="label">{$t('sidebar.super_admin') || 'Super Admin'}</span>
      </button>
    {/if}

    {#if isUrlSuperadmin}
      {#if $isAdmin}
        <button
          class="context-btn"
          onclick={() => goto(adminPanelHref)}
          aria-label={$t('sidebar.admin_panel') || 'Admin Panel'}
          data-tooltip={$t('sidebar.admin_panel') || 'Admin Panel'}
        >
          <Icon name="shield" size={16} />
          <span class="label">{$t('sidebar.admin_panel') || 'Admin Panel'}</span>
        </button>
      {/if}

      <button
        class="context-btn"
        onclick={() => goto(tenantPrefix ? `${tenantPrefix}/dashboard` : '/dashboard')}
        aria-label={$t('sidebar.exit') || 'Exit'}
        data-tooltip={$t('sidebar.exit') || 'Exit'}
      >
        <Icon name="arrow-left" size={16} />
        <span class="label">{$t('sidebar.exit') || 'Exit'}</span>
      </button>
    {/if}

    {#if $isAdmin && !isUrlSuperadmin}
      <button
        class="context-btn"
        onclick={() => goto(isUrlAdmin ? `${tenantPrefix}/dashboard` : `${tenantPrefix}/admin`)}
        aria-label={isUrlAdmin
          ? $t('sidebar.user_view') || 'User View'
          : $t('sidebar.admin_panel') || 'Admin Panel'}
        data-tooltip={isUrlAdmin
          ? $t('sidebar.user_view') || 'User View'
          : $t('sidebar.admin_panel') || 'Admin Panel'}
      >
        <Icon name={isUrlAdmin ? 'arrow-left' : 'shield'} size={16} />
        <span class="label">{isUrlAdmin ? $t('sidebar.user_view') : $t('sidebar.admin_panel')}</span
        >
      </button>
    {/if}

    <div class="profile-section">
      {#if isDropdownOpen}
        <div
          class="dropdown-menu"
          onclick={(e) => e.stopPropagation()}
          onkeydown={(e) => e.stopPropagation()}
          role="menu"
          tabindex="-1"
        >
          <div class="dropdown-header" aria-hidden="true">
            <div class="dropdown-avatar">
              {$user?.name?.charAt(0).toUpperCase() || '?'}
            </div>
            <div class="dropdown-meta">
              <div class="dropdown-name">
                {$user?.name || $t('profile.fallback.user') || 'User'}
              </div>
              <div class="dropdown-sub">
                {$user?.email || ''}
                {#if $user?.email && $user?.role}
                  <span class="dot">·</span>
                {/if}
                {$user?.role || ''}
              </div>
            </div>
          </div>

          <div class="divider"></div>

          <button
            class="menu-item"
            role="menuitem"
            onclick={() => navigate(`${tenantPrefix}/profile`)}
          >
            <Icon name="profile" size={16} />
            {$t('sidebar.profile')}
          </button>

          <button class="menu-item danger" role="menuitem" onclick={handleLogout}>
            <Icon name="logout" size={16} />
            {$t('sidebar.logout')}
          </button>
        </div>
      {/if}

      <button
        class="profile-btn"
        onclick={toggleDropdown}
        aria-haspopup="menu"
        aria-expanded={isDropdownOpen}
        onkeydown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            toggleDropdown(e as any);
          } else if (e.key === 'Escape') {
            isDropdownOpen = false;
          }
        }}
      >
        <div class="avatar">
          {$user?.name?.charAt(0).toUpperCase() || '?'}
        </div>
        <div class="user-meta">
          <span class="name">{$user?.name}</span>
          <span class="role">{$user?.role}</span>
        </div>
        <Icon name="chevron-up" size={14} class="chevron" />
      </button>
    </div>
  </div>
</aside>

<style>
  .sidebar {
    width: var(--sidebar-width);
    display: flex;
    flex-direction: column;
    padding: 12px 10px;
    color: var(--text-secondary);
    transition:
      transform 0.3s ease,
      width 0.3s ease;
    background: var(--bg-app);
    border-right: 1px solid color-mix(in srgb, var(--border-color) 86%, transparent);
    height: calc(100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom));
    overflow: hidden;

    /* Mobile defaults */
    position: fixed;
    left: 0;
    top: env(safe-area-inset-top);
    z-index: 50;
    transform: translateX(-100%);
  }

  /* Desktop styles */
  @media (min-width: 900px) {
    .sidebar {
      position: sticky;
      transform: none;
      height: calc(100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom));
    }
  }

  .sidebar.mobile-open {
    transform: translateX(0);
    box-shadow: var(--shadow-md);
  }

  .sidebar.collapsed {
    width: 76px;
  }

  /* Mobile Overlay */
  .sidebar-overlay {
    position: fixed;
    inset: 0;
    background: color-mix(in srgb, var(--bg-app) 64%, transparent);
    z-index: 40;
        animation: fadeIn 0.2s ease-out;
    border: 0;
    padding: 0;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  /* Header */
  .sidebar-header {
    padding: 8px 8px 12px;
    margin-bottom: 8px;
    white-space: nowrap;
    overflow: hidden;
  }

  .logo-wrapper {
    display: flex;
    align-items: center;
    gap: 11px;
    min-height: 42px;
    padding: 0 4px;
    font-weight: 800;
    color: var(--text-primary);
    transition: justify-content 0.3s;
  }

  .sidebar.collapsed .logo-wrapper {
    justify-content: center;
  }

  .app-logo {
    width: 30px;
    height: 30px;
    object-fit: contain;
    flex: 0 0 auto;
  }

  .logo-placeholder {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--color-primary) 16%, var(--bg-tertiary));
    color: var(--color-primary);
    border-radius: 8px;
    flex-shrink: 0;
    border: 1px solid color-mix(in srgb, var(--color-primary) 26%, transparent);
  }

  .app-name {
    min-width: 0;
    font-size: 0.96rem;
    font-weight: 850;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    letter-spacing: 0;
    transition:
      opacity 0.2s,
      width 0.2s;
  }

  .sidebar.collapsed .app-name {
    display: none;
  }

  /* Nav */
  .sidebar-nav {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 2px 2px 10px;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: color-mix(in srgb, var(--text-secondary) 28%, transparent) transparent;
  }

  .sidebar-nav::-webkit-scrollbar {
    width: 6px;
  }

  .sidebar-nav::-webkit-scrollbar-thumb {
    border-radius: 999px;
    background: color-mix(in srgb, var(--text-secondary) 22%, transparent);
  }

  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 11px;
    min-height: 40px;
    width: 100%;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid transparent;
    background: transparent;
    color: color-mix(in srgb, var(--text-secondary) 92%, var(--text-primary) 8%);
    font-size: 0.88rem;
    font-weight: 650;
    cursor: pointer;
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      color 0.15s ease,
      box-shadow 0.15s ease;
    text-align: left;
    white-space: nowrap;
  }

  .nav-item::before {
    content: '';
    position: absolute;
    left: 5px;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 999px;
    background: transparent;
    transition: background 0.15s ease;
  }

  .nav-item :global(svg) {
    flex: 0 0 auto;
    color: color-mix(in srgb, currentColor 82%, var(--text-primary) 18%);
  }

  .nav-item .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sidebar.collapsed .nav-item {
    justify-content: center;
    min-height: 42px;
    padding: 8px;
  }

  .sidebar.collapsed .nav-item .label {
    display: none;
  }

  .nav-item:hover {
    background: color-mix(in srgb, var(--bg-hover) 78%, transparent);
    border-color: color-mix(in srgb, var(--border-color) 78%, transparent);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: color-mix(in srgb, var(--color-primary) 9%, var(--bg-active));
    border-color: color-mix(in srgb, var(--color-primary) 20%, var(--border-color));
    color: var(--text-primary);
    box-shadow: inset 0 1px 0 var(--border-subtle);
  }

  .nav-item.active::before {
    background: color-mix(in srgb, var(--color-primary) 80%, var(--text-primary) 20%);
  }

  .nav-item:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--color-primary) 42%, transparent);
    outline-offset: 2px;
  }

  .item-alert {
    margin-left: auto;
    width: 18px;
    height: 18px;
    border-radius: 999px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--color-warning);
    background: color-mix(in srgb, var(--color-warning) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-warning) 30%, var(--border-color));
    flex-shrink: 0;
  }

  .item-alert.overdue {
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 14%, transparent);
    border-color: color-mix(in srgb, var(--color-danger) 30%, var(--border-color));
  }

  .sidebar.collapsed .item-alert {
    position: absolute;
    top: 3px;
    right: 5px;
    width: 14px;
    height: 14px;
  }

  .nav-section-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    width: 100%;
    min-height: 32px;
    padding: 8px 9px 4px;
    background: transparent;
    border: 1px solid transparent;
    cursor: pointer;
    text-align: left;
    border-radius: 10px;
    transition:
      background 0.15s ease,
      color 0.15s ease;
  }

  .nav-section-btn:hover {
    background: color-mix(in srgb, var(--bg-hover) 62%, transparent);
  }

  .nav-section-btn:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--color-primary) 45%, transparent);
    outline-offset: 2px;
  }

  .nav-section-title {
    font-size: 0.66rem;
    font-weight: 900;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--text-secondary) 82%, var(--text-primary) 18%);
    opacity: 0.9;
  }

  .nav-section-chev {
    color: var(--text-secondary);
    transition: transform 0.16s ease;
    transform: rotate(-90deg);
    display: grid;
    place-items: center;
  }

  .nav-section-chev.open {
    transform: rotate(0deg);
  }

  .nav-divider {
    height: 1px;
    margin: 6px 8px 5px;
    background: color-mix(in srgb, var(--border-color) 66%, transparent);
    opacity: 0.7;
  }

  /* Footer */
  .sidebar-footer {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 10px;
    margin-top: auto;
    border-top: 1px solid color-mix(in srgb, var(--border-color) 70%, transparent);
  }

  .context-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 38px;
    width: 100%;
    padding: 8px 10px;
    font-size: 0.85rem;
    font-weight: 780;
    color: var(--color-primary);
    background: color-mix(in srgb, var(--color-primary) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-primary) 18%, transparent);
    border-radius: 10px;
    cursor: pointer;
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      color 0.15s ease;
    white-space: nowrap;
  }

  .sidebar.collapsed .context-btn {
    justify-content: center;
    padding: 8px;
  }

  .sidebar.collapsed .context-btn .label {
    display: none;
  }

  .context-btn:hover {
    border-color: color-mix(in srgb, var(--color-primary) 44%, transparent);
    background: color-mix(in srgb, var(--color-primary) 14%, transparent);
  }

  .profile-section {
    position: relative;
    min-width: 0;
  }

  .profile-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 46px;
    padding: 8px;
    background: color-mix(in srgb, var(--bg-surface) 46%, transparent);
    border: 1px solid color-mix(in srgb, var(--border-color) 70%, transparent);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition:
      background 0.15s ease,
      border-color 0.15s ease;
  }

  .sidebar.collapsed .profile-btn {
    justify-content: center;
  }

  .sidebar.collapsed .user-meta {
    display: none;
  }

  .profile-btn:hover {
    background: color-mix(in srgb, var(--bg-hover) 78%, transparent);
    border-color: color-mix(in srgb, var(--border-color) 92%, transparent);
  }

  .profile-btn:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--color-primary), white 10%);
    outline-offset: 2px;
  }

  .avatar {
    width: 30px;
    height: 30px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--color-primary) 16%, var(--bg-active));
    color: color-mix(in srgb, var(--color-primary) 72%, var(--text-primary));
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.8rem;
    font-weight: 900;
    flex-shrink: 0;
  }

  .user-meta {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    overflow: hidden;
  }

  .name {
    font-size: 0.85rem;
    font-weight: 820;
    color: var(--text-primary);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .role {
    font-size: 0.7rem;
    color: var(--text-secondary);
    text-transform: capitalize;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Dropdown */
  .dropdown-menu {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    margin-bottom: 8px;
    background: color-mix(in srgb, var(--bg-surface), transparent 6%);
    border: 1px solid color-mix(in srgb, var(--border-color), white 8%);
    border-radius: var(--radius-md);
    padding: 6px;
    box-shadow: var(--shadow-md);
        display: flex;
    flex-direction: column;
    z-index: 100;
    animation: dropdownPop 0.14s ease-out;
    width: 100%;
    min-width: 0;
    max-width: 100%;
    box-sizing: border-box;
  }

  .dropdown-menu::after {
    content: '';
    position: absolute;
    left: 18px;
    bottom: -6px;
    width: 12px;
    height: 12px;
    background: color-mix(in srgb, var(--bg-surface), transparent 6%);
    border-right: 1px solid color-mix(in srgb, var(--border-color), white 8%);
    border-bottom: 1px solid color-mix(in srgb, var(--border-color), white 8%);
    transform: rotate(45deg);
  }

  .sidebar.collapsed .dropdown-menu {
    left: 100%;
    bottom: 0;
    margin-left: 8px;
    margin-bottom: 0;
    width: clamp(210px, 28vw, 280px);
    max-width: min(280px, calc(100vw - 24px));
  }

  .sidebar.collapsed .dropdown-menu::after {
    left: -6px;
    bottom: 16px;
    transform: rotate(135deg);
  }

  .dropdown-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 10px 8px;
    min-width: 0;
  }

  .dropdown-avatar {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 800;
    font-size: 0.9rem;
    flex: 0 0 auto;
    box-shadow: 0 0 0 1px var(--border-subtle) inset;
  }

  .dropdown-meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: hidden;
  }

  .dropdown-name {
    color: var(--text-primary);
    font-weight: 800;
    font-size: 0.9rem;
    line-height: 1.1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .dropdown-sub {
    color: var(--text-secondary);
    font-size: 0.78rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .dot {
    margin: 0 6px;
    opacity: 0.7;
  }

  /* Tooltips shown only when sidebar is collapsed (desktop) */
  @media (min-width: 900px) {
    .sidebar.collapsed [data-tooltip] {
      position: relative;
    }

    .sidebar.collapsed [data-tooltip]:hover::after {
      content: attr(data-tooltip);
      position: absolute;
      left: calc(100% + 8px);
      top: 50%;
      transform: translateY(-50%);
      padding: 6px 10px;
      background: var(--bg-surface);
      color: var(--text-primary);
      border: 1px solid var(--border-color);
      border-radius: 6px;
      white-space: nowrap;
      box-shadow: var(--shadow-md);
      font-size: 0.85rem;
      z-index: 200;
    }
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.9rem;
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
  }

  .menu-item:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--color-primary) 55%, transparent);
    outline-offset: 2px;
  }

  .menu-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .menu-item.danger:hover {
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
  }
  .divider {
    height: 1px;
    background: color-mix(in srgb, var(--border-color), transparent 35%);
    margin: 6px 6px;
  }

  @keyframes dropdownPop {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
</style>
