<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ResponsiveTabs from '$lib/components/ui/ResponsiveTabs.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import { user, tenant } from '$lib/stores/auth';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { timeAgo } from '$lib/utils/date';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { hasInternalAppAccess } from '$lib/utils/appLanding';
  import { openProfileModal } from '$lib/stores/profileModal';
  import { resolveAnnouncementActionUrl } from '$lib/utils/announcementRouting';
  import { getVisiblePortalNotifications } from '$lib/utils/dashboardNotifications';
  import { api } from '$lib/api/client';
  import {
    notifications,
    loading,
    pagination,
    loadNotifications,
    refreshUnreadCount,
    markAsRead,
    markAllAsRead,
    deleteNotification,
    deleteAllNotifications,
  } from '$lib/stores/notifications';
  import { notificationModal, closeNotificationModal } from '$lib/stores/notificationModal';

  let filter = $state<'all' | 'unread'>('all');
  let searchQuery = $state('');
  let portalInvoiceIds = $state<string[]>([]);
  let isMobile = $state(false);

  let tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  let tenantPrefix = $derived(tenantCtx.tenantPrefix);

  // Load data when modal opens
  $effect(() => {
    if ($notificationModal.open) {
      void (async () => {
        await loadNotifications(1);
        if (!hasInternalAppAccess($user)) {
          try {
            const invoiceRows = await api.payment.listInvoices();
            portalInvoiceIds = (invoiceRows || []).map((inv: any) => inv.id).filter(Boolean);
          } catch (e) {
            portalInvoiceIds = [];
          }
        }
        await refreshUnreadCount(true);
      })();
    }
  });

  let visibleNotifications = $derived(
    getVisiblePortalNotifications($notifications, hasInternalAppAccess($user), portalInvoiceIds),
  );
  let visibleUnreadCount = $derived(visibleNotifications.filter((n) => !n.is_read).length);
  let totalLoaded = $derived(visibleNotifications.length);

  let filteredNotifications = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    const base =
      filter === 'unread' ? visibleNotifications.filter((n) => !n.is_read) : visibleNotifications;
    if (!q) return base;
    return base.filter((n) => {
      const title = String(n.title || '').toLowerCase();
      const message = String(n.message || '').toLowerCase();
      return title.includes(q) || message.includes(q);
    });
  });

  let hasMore = $derived(!!$pagination.hasMore);
  let canMarkAllRead = $derived(visibleUnreadCount > 0);
  let filterTabs = $derived.by(() => [
    { id: 'all', label: $t('notifications_page.filters.all') || 'All' },
    {
      id: 'unread',
      label: $t('notifications_page.filters.unread') || 'Unread',
      count: visibleUnreadCount > 0 ? (visibleUnreadCount > 99 ? '99+' : visibleUnreadCount) : null,
    },
  ]);

  let viewportEl: HTMLDivElement | undefined;

  async function loadMore() {
    if ($loading || !hasMore) return;
    await loadNotifications($pagination.page + 1, true);
    await refreshUnreadCount();
  }

  function handleScroll() {
    if (!viewportEl || $loading || !hasMore) return;
    const { scrollTop, scrollHeight, clientHeight } = viewportEl;
    if (scrollHeight - scrollTop - clientHeight < 200) {
      void loadMore();
    }
  }

  function openPreferences() {
    closeNotificationModal();
    openProfileModal({ tab: 'notifications' });
  }

  function handleClick(n: any) {
    if (!n.is_read) markAsRead(n.id);
    if (n.action_url) {
      closeNotificationModal();
      goto(resolveActionUrl(n.action_url));
    }
  }

  function resolveActionUrl(actionUrl: string) {
    return resolveAnnouncementActionUrl(actionUrl, {
      tenantPrefix,
      internal: hasInternalAppAccess($user),
    });
  }

  function getIconForType(type: string) {
    switch (type) {
      case 'success':
        return 'check-circle';
      case 'warning':
        return 'alert-circle';
      case 'error':
        return 'alert-circle';
      default:
        return 'info';
    }
  }

  function getColorForType(type: string) {
    switch (type) {
      case 'success':
        return 'var(--color-success)';
      case 'warning':
        return 'var(--color-warning)';
      case 'error':
        return 'var(--color-danger)';
      default:
        return 'var(--color-info)';
    }
  }

  // Confirm dialogs
  let showDeleteModal = $state(false);
  let deleting = $state(false);
  let deleteTarget = $state<any | null>(null);

  let showMarkAllModal = $state(false);
  let markingAll = $state(false);

  function requestDelete(n: any) {
    deleteTarget = n;
    showDeleteModal = true;
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    deleting = true;
    try {
      await deleteNotification(deleteTarget.id);
      await refreshUnreadCount();
    } finally {
      deleting = false;
      showDeleteModal = false;
      deleteTarget = null;
    }
  }

  function requestMarkAll() {
    showMarkAllModal = true;
  }

  async function confirmMarkAll() {
    markingAll = true;
    try {
      await markAllAsRead();
      await refreshUnreadCount(true);
    } finally {
      markingAll = false;
      showMarkAllModal = false;
    }
  }

  let showClearAllModal = $state(false);
  let clearingAll = $state(false);

  function requestClearAll() {
    showClearAllModal = true;
  }

  async function confirmClearAll() {
    clearingAll = true;
    try {
      await deleteAllNotifications();
    } finally {
      clearingAll = false;
      showClearAllModal = false;
    }
  }

  function requestClose() {
    closeNotificationModal();
  }

  function handleBackdropClick() {
    requestClose();
  }

  function handleDialogKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      requestClose();
    }
  }
</script>

{#if $notificationModal.open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="notif-modal-backdrop"
    onclick={handleBackdropClick}
    onkeydown={handleDialogKeydown}
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="notif-modal-shell"
      role="dialog"
      aria-modal="true"
      aria-labelledby="notif-modal-title"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleDialogKeydown}
    >
      <div class="notif-modal-topbar">
        <div class="notif-modal-header-left">
          <h2 id="notif-modal-title">
            {$t('notifications_page.title') || $t('topbar.notifications') || 'Notifications'}
          </h2>
          {#if totalLoaded > 0}
            <span class="count-badge">{totalLoaded}</span>
          {/if}
        </div>
        <div class="notif-modal-header-actions">
          {#if canMarkAllRead}
            <button
              class="btn btn-glass btn-sm"
              onclick={requestMarkAll}
              title={$t('topbar.notifications_menu.mark_all_read')}
            >
              <Icon name="check-circle" size={14} />
              <span class="hide-xs"
                >{$t('topbar.notifications_menu.mark_all_read')}</span
              >
            </button>
          {/if}
          {#if totalLoaded > 0}
            <button
              class="btn btn-glass btn-sm"
              onclick={requestClearAll}
              title={$t('topbar.notifications_menu.clear_all')}
            >
              <Icon name="trash" size={14} />
              <span class="hide-xs"
                >{$t('topbar.notifications_menu.clear_all')}</span
              >
            </button>
          {/if}
          <button
            class="btn btn-glass btn-sm"
            onclick={openPreferences}
            title={$t('topbar.notifications_menu.settings')}
          >
            <Icon name="settings" size={14} />
          </button>
          <button
            class="notif-modal-close"
            type="button"
            onclick={requestClose}
            aria-label={$t('common.close')}
            title={$t('common.close')}
          >
            <Icon name="x" size={18} />
          </button>
        </div>
      </div>

      <div class="notif-modal-toolbar">
        <div class="notif-search">
          <Icon name="search" size={16} />
          <input
            type="text"
            placeholder={$t('notifications_page.search_placeholder')}
            bind:value={searchQuery}
          />
          {#if searchQuery}
            <button
              class="clear"
              onclick={() => (searchQuery = '')}
              aria-label={$t('common.clear')}
            >
              <Icon name="x" size={12} />
            </button>
          {/if}
        </div>
        <div class="notif-filters">
          <ResponsiveTabs
            items={filterTabs}
            bind:activeId={filter}
            {isMobile}
            priorityCount={2}
            ariaLabel="Filters"
          />
        </div>
      </div>

      <div class="notif-modal-viewport" bind:this={viewportEl} onscroll={handleScroll}>
        {#if $loading && $notifications.length === 0}
          <div class="notif-center">
            <div class="spinner"></div>
            <p class="muted">{$t('notifications_page.loading')}</p>
          </div>
        {:else if filteredNotifications.length === 0}
          <div class="notif-empty">
            <div class="icon-bg">
              <Icon name="bell" size={28} />
            </div>
            <h3>{$t('notifications_page.empty.title') || "You're all caught up"}</h3>
            <p class="muted">
              {$t('notifications_page.empty.subtitle')}
            </p>
          </div>
        {:else}
          <div class="notif-items">
            {#each filteredNotifications as n (n.id)}
              <article class="notif-item" class:unread={!n.is_read}>
                <button type="button" class="notif-item-main" onclick={() => handleClick(n)}>
                  <div class="notif-item-left">
                    <div class="notif-type-icon {n.notification_type}">
                      <Icon name={getIconForType(n.notification_type)} size={16} color={getColorForType(n.notification_type)} />
                    </div>
                    <div class="notif-item-text">
                      <div class="notif-item-row">
                        <span class="notif-item-title">{n.title}</span>
                        <span class="notif-item-time">{timeAgo(n.created_at)}</span>
                      </div>
                      {#if n.message}
                        <div class="notif-item-msg">{n.message}</div>
                      {/if}
                    </div>
                  </div>
                </button>
                <div class="notif-item-actions">
                  {#if !n.is_read}
                    <button
                      class="icon-btn"
                      title={$t('notifications_page.mark_as_read')}
                      onclick={() => markAsRead(n.id)}
                    >
                      <Icon name="check" size={14} />
                    </button>
                  {/if}
                  <button
                    class="icon-btn danger"
                    title={$t('common.delete')}
                    onclick={() => requestDelete(n)}
                  >
                    <Icon name="trash" size={14} />
                  </button>
                </div>
              </article>
            {/each}
          </div>
        {/if}

        {#if $loading && $notifications.length > 0}
          <div class="notif-load-more">
            <div class="spinner small"></div>
          </div>
        {/if}

        {#if !$loading && hasMore}
          <div class="notif-load-more">
            <button class="btn btn-glass btn-sm" onclick={loadMore}>
              {$t('common.load_more')}
            </button>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<ConfirmDialog
  bind:show={showDeleteModal}
  title={$t('notifications_page.delete_confirm.title')}
  message={$t('notifications_page.delete_confirm.message')}
  confirmText={$t('common.delete')}
  type="danger"
  loading={deleting}
  onconfirm={confirmDelete}
  oncancel={() => {
    showDeleteModal = false;
    deleteTarget = null;
  }}
/>

<ConfirmDialog
  bind:show={showMarkAllModal}
  title={$t('notifications_page.mark_all_confirm.title')}
  message={$t('notifications_page.mark_all_confirm.message')}
  confirmText={$t('topbar.notifications_menu.mark_all_read')}
  loading={markingAll}
  onconfirm={confirmMarkAll}
  oncancel={() => (showMarkAllModal = false)}
/>

<ConfirmDialog
  bind:show={showClearAllModal}
  title={$t('notifications_page.confirm_clear_all.title')}
  message={$t('notifications_page.confirm_clear_all.message')}
  confirmText={$t('topbar.notifications_menu.clear_all')}
  type="danger"
  loading={clearingAll}
  onconfirm={confirmClearAll}
  oncancel={() => (showClearAllModal = false)}
/>

<style>
  .notif-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1200;
    background: rgba(8, 12, 19, 0.66);
    display: flex;
    align-items: stretch;
    justify-content: center;
    padding: clamp(12px, 2vw, 24px);
  }

  .notif-modal-shell {
    width: 100%;
    max-width: 720px;
    max-height: 100%;
    margin: auto;
    background: var(--bg-primary);
    border-radius: 16px;
    border: 1px solid var(--border-color);
    box-shadow: 0 25px 60px rgba(0, 0, 0, 0.35);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .notif-modal-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-surface);
    flex-shrink: 0;
  }

  .notif-modal-header-left {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .notif-modal-header-left h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .count-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    height: 22px;
    padding: 0 6px;
    background: var(--color-primary);
    color: white;
    border-radius: 999px;
    font-size: 0.7rem;
    font-weight: 700;
  }

  .notif-modal-header-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .notif-modal-close {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .notif-modal-close:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .notif-modal-toolbar {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0.75rem 1.25rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-surface);
    flex-shrink: 0;
  }

  .notif-search {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 0.45rem 0.75rem;
  }

  .notif-search input {
    border: none;
    background: transparent;
    outline: none;
    flex: 1;
    font-size: 0.85rem;
    color: var(--text-primary);
    min-width: 0;
  }

  .notif-search input::placeholder {
    color: var(--text-tertiary);
  }

  .notif-search .clear {
    background: none;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
  }

  .notif-filters {
    display: flex;
    align-items: center;
  }

  .notif-modal-viewport {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
    background: var(--bg-primary);
  }

  .notif-center {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    gap: 0.75rem;
  }

  .notif-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    gap: 0.5rem;
  }

  .icon-bg {
    width: 56px;
    height: 56px;
    background: var(--bg-hover);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 0.5rem;
  }

  .notif-empty h3 {
    margin: 0;
    font-size: 1rem;
    color: var(--text-primary);
  }

  .notif-items {
    display: flex;
    flex-direction: column;
  }

  .notif-item {
    display: flex;
    align-items: flex-start;
    border-bottom: 1px solid var(--border-color);
    transition: background 0.15s;
  }

  .notif-item:hover {
    background: var(--bg-hover);
  }

  .notif-item.unread {
    background: var(--bg-active);
  }

  .notif-item.unread:hover {
    background: var(--bg-tertiary);
  }

  .notif-item-main {
    flex: 1;
    display: flex;
    align-items: flex-start;
    padding: 0.85rem 1rem;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    min-width: 0;
  }

  .notif-item-left {
    display: flex;
    gap: 0.7rem;
    min-width: 0;
    flex: 1;
  }

  .notif-type-icon {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-hover);
    margin-top: 2px;
  }

  .notif-type-icon.success {
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
  }

  .notif-type-icon.warning {
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
  }

  .notif-type-icon.error {
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
  }

  .notif-item-text {
    flex: 1;
    min-width: 0;
  }

  .notif-item-row {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
  }

  .notif-item-title {
    font-size: 0.88rem;
    font-weight: 500;
    color: var(--text-primary);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .notif-item.unread .notif-item-title {
    font-weight: 600;
  }

  .notif-item-time {
    font-size: 0.75rem;
    color: var(--text-tertiary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .notif-item-msg {
    font-size: 0.82rem;
    color: var(--text-secondary);
    margin-top: 2px;
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .notif-item-actions {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.85rem 0.75rem 0.85rem 0;
    flex-shrink: 0;
  }

  .icon-btn {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .icon-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .icon-btn.danger:hover {
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    color: var(--color-danger);
  }

  .notif-load-more {
    display: flex;
    justify-content: center;
    padding: 1rem;
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .spinner.small {
    width: 18px;
    height: 18px;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .muted {
    color: var(--text-tertiary);
    font-size: 0.85rem;
    margin: 0;
  }

  .hide-xs {
    display: inline;
  }

  @media (max-width: 640px) {
    .notif-modal-shell {
      max-width: 100%;
      border-radius: 12px;
    }

    .hide-xs {
      display: none;
    }
  }
</style>
