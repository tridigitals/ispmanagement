<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onDestroy, onMount } from 'svelte';
  import { can, isSuperAdmin, user } from '$lib/stores/auth';
  import { searchGlobalTopbar } from '$lib/search/globalSearchService';
  import type {
    GlobalSearchProviderContext,
    GlobalSearchResult,
  } from '$lib/search/globalSearchModel';
  import { globalSearch } from '$lib/stores/globalSearch';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { isSidebarCollapsed } from '$lib/stores/ui';
  import { t } from 'svelte-i18n';
  import Icon from '../ui/Icon.svelte';
  import NotificationDropdown from './NotificationDropdown.svelte';
  import TopbarGlobalSearchPanel from './TopbarGlobalSearchPanel.svelte';
  import UserMenuDropdown from './UserMenuDropdown.svelte';

  let { onMobileMenuClick }: { onMobileMenuClick: () => void } = $props();
  const DESKTOP_BP = 900; // Keep in sync with --bp-lg
  const SEARCH_DEBOUNCE_MS = 220;
  let inputEl = $state<HTMLInputElement | null>(null);
  let highlightedIndex = $state(-1);
  let closePanelTimer: ReturnType<typeof setTimeout> | null = null;
  let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;
  let latestSearchRequestId = 0;

  // Helper to get page title based on path (ordered by specificity)
  function getPageTitle(path: string) {
    const map: [string, string][] = [
      ['/notifications', $t('topbar.titles.notifications') || 'Notifications'],
      ['/superadmin/tenants', $t('topbar.titles.tenants') || 'Tenants'],
      ['/superadmin/users', $t('topbar.titles.users') || 'Users'],
      ['/superadmin/radius', $t('topbar.titles.radius') || 'RADIUS'],
      ['/superadmin/plans', $t('topbar.titles.plans') || 'Plans'],
      ['/superadmin/invoices', $t('topbar.titles.invoices') || 'Invoices'],
      ['/superadmin/storage', $t('topbar.titles.storage') || 'Storage'],
      ['/superadmin/audit-logs', $t('topbar.titles.audit_logs') || 'Audit Logs'],
      ['/superadmin/settings', $t('topbar.titles.settings') || 'Settings'],
      ['/superadmin/system', $t('topbar.titles.system') || 'System'],
      ['/superadmin', $t('topbar.titles.superadmin_dashboard') || 'Super Admin'],
      ['/admin/support', $t('topbar.titles.support') || 'Support'],
      ['/admin/audit-logs', $t('topbar.titles.audit_logs') || 'Audit Logs'],
      ['/admin/team', $t('topbar.titles.team') || 'Team'],
      ['/admin/roles', $t('topbar.titles.roles') || 'Roles'],
      ['/admin/message-templates', $t('topbar.titles.message_templates') || 'Message Templates'],
      ['/admin/settings', $t('topbar.titles.global_settings') || 'Settings'],
      ['/admin/storage', $t('topbar.titles.storage') || 'Storage'],
      ['/admin/subscription', $t('topbar.titles.subscription') || 'Subscription'],
      ['/admin/invoices', $t('topbar.titles.invoices') || 'Invoices'],
      ['/admin', $t('topbar.titles.admin_overview') || 'Admin'],
      ['/profile', $t('topbar.titles.profile') || 'Profile'],
      ['/support', $t('topbar.titles.support') || 'Support'],
      ['/dashboard', $t('topbar.titles.dashboard') || 'Dashboard'],
    ];

    for (const [route, label] of map) {
      if (path.includes(route)) return label;
    }
    return $t('topbar.titles.default') || 'SaaS App';
  }

  let title = $derived(getPageTitle($page.url.pathname));

  function handleSidebarToggle() {
    const isDesktop = typeof window !== 'undefined' && window.innerWidth >= DESKTOP_BP;
    if (!isDesktop) {
      onMobileMenuClick();
      // Always keep desktop state expanded when coming from mobile
      $isSidebarCollapsed = false;
      return;
    }
    $isSidebarCollapsed = !$isSidebarCollapsed;
  }

  let toggleLabel = $derived(
    $isSidebarCollapsed
      ? $t('sidebar.expand') || 'Expand sidebar'
      : $t('sidebar.collapse') || 'Collapse sidebar',
  );
  const searchContext = $derived.by<GlobalSearchProviderContext>(() => {
    const tenantCtx = resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      routeTenantSlug: $page.params.tenant,
    });
    const pathname = $page.url.pathname || '/';
    return {
      can: $can,
      isSuperAdmin: $isSuperAdmin,
      shellScope: pathname.startsWith('/superadmin')
        ? 'superadmin'
        : pathname.startsWith('/admin')
          ? 'admin'
          : 'workspace',
      tenantPrefix: tenantCtx.tenantPrefix,
    };
  });
  const flatItems = $derived.by(() =>
    $globalSearch.groups.flatMap((group) =>
      group.items.map((item) => ({
        groupKey: group.key,
        item,
      })),
    ),
  );
  const showSearchPanel = $derived($globalSearch.open && $globalSearch.query.trim().length > 0);

  async function runTopbarSearch(query: string) {
    const requestId = ++latestSearchRequestId;
    const trimmedQuery = query.trim();
    globalSearch.setQuery(query);

    if (!trimmedQuery) {
      globalSearch.setLoading(false);
      globalSearch.setResults([]);
      highlightedIndex = -1;
      return;
    }

    globalSearch.setLoading(true);
    const result = await searchGlobalTopbar(trimmedQuery, searchContext);
    if (requestId !== latestSearchRequestId) return;
    globalSearch.setResults(result.groups);
    globalSearch.setLoading(false);
    highlightedIndex = result.groups.length ? 0 : -1;
  }

  function scheduleTopbarSearch(query: string) {
    if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
    searchDebounceTimer = setTimeout(() => {
      void runTopbarSearch(query);
    }, SEARCH_DEBOUNCE_MS);
  }

  function handleTopbarSearchInput(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value;
    globalSearch.setQuery(value);
    if (!value.trim()) {
      latestSearchRequestId += 1;
      globalSearch.close();
      highlightedIndex = -1;
      if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
      return;
    }
    globalSearch.open();
    scheduleTopbarSearch(value);
  }

  function handleTopbarSearchFocus() {
    if (closePanelTimer) clearTimeout(closePanelTimer);
  }

  function handleTopbarSearchBlur() {
    closePanelTimer = setTimeout(() => {
      latestSearchRequestId += 1;
      globalSearch.close();
      highlightedIndex = -1;
    }, 120);
  }

  async function handleResultSelect(item: GlobalSearchResult) {
    globalSearch.close();
    latestSearchRequestId += 1;
    highlightedIndex = -1;
    await goto(item.href);
  }

  function focusSearchInput() {
    inputEl?.focus();
  }

  function handleTopbarSearchKeydown(event: KeyboardEvent) {
    if (!showSearchPanel && event.key === 'Escape') {
      event.preventDefault();
      globalSearch.close();
      highlightedIndex = -1;
      return;
    }
    if (event.key === 'ArrowDown' && flatItems.length) {
      event.preventDefault();
      highlightedIndex = (highlightedIndex + 1 + flatItems.length) % flatItems.length;
      return;
    }
    if (event.key === 'ArrowUp' && flatItems.length) {
      event.preventDefault();
      highlightedIndex = (highlightedIndex - 1 + flatItems.length) % flatItems.length;
      return;
    }
    if (event.key === 'Enter' && highlightedIndex >= 0) {
      event.preventDefault();
      const item = flatItems[highlightedIndex]?.item;
      if (item) void handleResultSelect(item);
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      globalSearch.close();
      highlightedIndex = -1;
    }
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      focusSearchInput();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleWindowKeydown);
    return () => window.removeEventListener('keydown', handleWindowKeydown);
  });

  onDestroy(() => {
    if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
    if (closePanelTimer) clearTimeout(closePanelTimer);
    latestSearchRequestId += 1;
    globalSearch.close();
  });

  $effect(() => {
    $page.url.pathname;
    latestSearchRequestId += 1;
    globalSearch.close();
    highlightedIndex = -1;
    if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
  });
</script>

<header class="topbar">
  <div class="left-section">
    <button
      class="icon-btn toggle-btn"
      onclick={handleSidebarToggle}
      title={toggleLabel}
      aria-label={toggleLabel}
      data-tooltip={toggleLabel}
    >
      <Icon name="menu" size={20} />
    </button>
    <h2 class="page-title">{title}</h2>
  </div>

  <div class="center-section">
    <div class="search-slot hide-mobile">
      <div class="search-bar">
        <span class="search-leading">
          <Icon name="search" size={16} />
        </span>
        <input
          bind:this={inputEl}
          type="text"
          value={$globalSearch.query}
          placeholder={$t('topbar.search_placeholder') || 'Search customers, routers, invoices, or tickets'}
          oninput={handleTopbarSearchInput}
          onfocus={handleTopbarSearchFocus}
          onblur={handleTopbarSearchBlur}
          onkeydown={handleTopbarSearchKeydown}
        />
        <span class="search-shortcut">Ctrl K</span>
      </div>
      {#if showSearchPanel}
        <TopbarGlobalSearchPanel
          groups={$globalSearch.groups}
          loading={$globalSearch.loading}
          query={$globalSearch.query}
          {highlightedIndex}
          onSelect={handleResultSelect}
        />
      {/if}
    </div>
  </div>

  <div class="right-section">
    <NotificationDropdown />
    <div class="topbar-user-menu">
      <UserMenuDropdown variant="topbar" />
    </div>
  </div>
</header>

<style>
  .topbar {
    height: var(--header-height);
    width: 100%;
    max-width: 100%;
    min-width: 0;
    box-sizing: border-box;
    background: var(--bg-primary);
    border-bottom: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.2rem clamp(12px, 2.4vw, 24px) 0.45rem;
    flex-shrink: 0;
    z-index: 40;
    overflow: visible;
  }

  .left-section {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex: 1 1 0;
    min-width: 0;
  }

  .center-section {
    display: flex;
    justify-content: center;
    flex: 0 1 min(40vw, 500px);
    min-width: 0;
  }

  .page-title {
    font-size: 1rem;
    font-weight: 750;
    color: var(--text-primary);
    margin: 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .right-section {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.75rem;
    flex: 1 1 0;
    min-width: 0;
  }

  .topbar-user-menu {
    min-width: 0;
    flex: 0 1 auto;
  }

  /* Search Bar */
  .search-slot {
    position: relative;
    width: 100%;
  }

  .search-bar {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    min-height: 46px;
    background: color-mix(in srgb, var(--bg-tertiary) 86%, var(--bg-surface));
    border: 1px solid color-mix(in srgb, var(--border-color) 84%, transparent);
    padding: 0.34rem 0.55rem 0.34rem 0.62rem;
    border-radius: 14px;
    transition:
      border-color 0.2s ease,
      background 0.2s ease,
      box-shadow 0.2s ease,
      transform 0.2s ease;
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.03),
      0 10px 24px rgba(2, 6, 23, 0.18);
  }

  .search-bar:focus-within {
    border-color: color-mix(in srgb, var(--color-primary) 54%, var(--border-color));
    background: color-mix(in srgb, var(--bg-secondary) 90%, var(--bg-surface));
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.04),
      0 14px 30px rgba(2, 6, 23, 0.24);
    transform: translateY(-1px);
  }

  .search-leading {
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 10px;
    background: color-mix(in srgb, var(--bg-primary) 72%, transparent);
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .search-bar input {
    background: transparent;
    border: 1px solid transparent;
    outline: none;
    color: var(--text-primary);
    font-size: 0.9rem;
    width: 100%;
    min-width: 0;
  }

  .search-bar input::placeholder {
    color: color-mix(in srgb, var(--text-secondary) 88%, transparent);
  }

  .search-shortcut {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 52px;
    padding: 0.22rem 0.46rem;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--border-color) 78%, transparent);
    background: color-mix(in srgb, var(--bg-primary) 68%, transparent);
    color: var(--text-secondary);
    font-size: 0.68rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    flex-shrink: 0;
  }

  /* Icon Buttons */
  .icon-btn {
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-secondary);
    width: 34px;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
    cursor: pointer;
    position: relative;
    transition: all 0.2s;
  }

  .icon-btn:hover {
    background: var(--bg-tertiary);
    border-color: var(--border-subtle);
    color: var(--text-primary);
  }

  /* Lightweight tooltip for desktop hover */
  @media (min-width: 900px) {
    .icon-btn[data-tooltip] {
      position: relative;
    }

    .icon-btn[data-tooltip]:hover::after {
      content: attr(data-tooltip);
      position: absolute;
      top: calc(100% + 8px);
      left: 50%;
      transform: translateX(-50%);
      padding: 6px 10px;
      background: var(--bg-surface);
      color: var(--text-primary);
      border: 1px solid var(--border-color);
      border-radius: 6px;
      white-space: nowrap;
      box-shadow: var(--shadow-md);
      font-size: 0.85rem;
      z-index: 10;
    }
  }

  @media (max-width: 900px) {
    .topbar {
      gap: 0.5rem;
      padding: 0.16rem clamp(10px, 3.5vw, 16px) 0.38rem;
      overflow: hidden;
    }

    .left-section {
      gap: 0.5rem;
      flex: 1 1 auto;
      min-width: 0;
    }

    .center-section {
      display: flex;
      flex: 0 1 auto;
      min-width: 0;
    }

    .center-section :global(.hide-mobile) {
      display: block !important;
    }

    .search-slot {
      width: auto;
    }

    .search-bar {
      min-height: 34px;
      padding: 0.15rem 0.4rem 0.15rem 0.5rem;
      border-radius: 10px;
      gap: 0.4rem;
    }

    .search-leading {
      width: 24px;
      height: 24px;
      border-radius: 8px;
    }

    .search-bar input {
      width: 80px;
      font-size: 0.82rem;
    }

    .search-shortcut {
      display: none;
    }

    .page-title {
      font-size: 0.95rem;
      max-width: 100%;
    }

    .right-section {
      gap: 0.45rem;
      flex: 0 0 auto;
    }

    .topbar-user-menu {
      display: flex;
    }
  }
</style>
