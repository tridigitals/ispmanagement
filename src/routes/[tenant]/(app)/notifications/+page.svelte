<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { user } from '$lib/stores/auth';
  import { timeAgo } from '$lib/utils/date';
  import { getSlugFromDomain } from '$lib/utils/domain';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import {
    notifications,
    unreadCount,
    loading,
    pagination,
    loadNotifications,
    refreshUnreadCount,
    markAsRead,
    markAllAsRead,
    deleteNotification,
  } from '$lib/stores/notifications';

  let filter = $state<'all' | 'unread'>('all');
  let searchQuery = $state('');

  let domainSlug = $derived(getSlugFromDomain($page.url.hostname));
  let isCustomDomain = $derived(domainSlug && domainSlug === $user?.tenant_slug);
  let tenantPrefix = $derived($user?.tenant_slug && !isCustomDomain ? `/${$user.tenant_slug}` : '');

  onMount(async () => {
    await loadNotifications(1);
    await refreshUnreadCount(true);
  });

  let totalLoaded = $derived($notifications.length);
  let totalAll = $derived($pagination.total || $notifications.length);

  let filteredNotifications = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    const base = filter === 'unread' ? $notifications.filter((n) => !n.is_read) : $notifications;
    if (!q) return base;
    return base.filter((n) => {
      const title = String(n.title || '').toLowerCase();
      const message = String(n.message || '').toLowerCase();
      return title.includes(q) || message.includes(q);
    });
  });

  let hasMore = $derived(!!$pagination.hasMore);
  let canMarkAllRead = $derived($unreadCount > 0);

  async function loadMore() {
    if ($loading || !hasMore) return;
    await loadNotifications($pagination.page + 1, true);
    await refreshUnreadCount();
  }

  function openPreferences() {
    goto(`${tenantPrefix}/profile?tab=notifications`);
  }

  function handleClick(n: any) {
    if (!n.is_read) markAsRead(n.id);
    if (n.action_url) goto(resolveActionUrl(n.action_url));
  }

  function resolveActionUrl(actionUrl: string) {
    if (!actionUrl || !tenantPrefix) return actionUrl;
    if (actionUrl.startsWith(tenantPrefix + '/')) return actionUrl;
    if (
      actionUrl.startsWith('/admin') ||
      actionUrl.startsWith('/dashboard') ||
      actionUrl.startsWith('/profile') ||
      actionUrl.startsWith('/notifications')
    ) {
      return `${tenantPrefix}${actionUrl}`;
    }
    return actionUrl;
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
</script>

<div class="page-content fade-in">
  <div class="sticky-header">
    <header class="page-header">
      <div class="title-col">
        <div class="title-row">
          <h1>
            {$t('notifications_page.title') || $t('topbar.notifications')}
          </h1>
          <span class="count-badge">{totalLoaded}/{totalAll}</span>
        </div>
        <p class="muted">
          {$t('notifications_page.subtitle') || 'Updates, alerts, and system messages.'}
        </p>
      </div>

      <div class="actions-col">
        {#if canMarkAllRead}
          <button
            class="btn btn-glass"
            onclick={requestMarkAll}
            title={$t('topbar.notifications_menu.mark_all_read') || 'Mark all read'}
          >
            <Icon name="check-circle" size={16} />
            <span class="hide-xs"
              >{$t('topbar.notifications_menu.mark_all_read') || 'Mark all read'}</span
            >
          </button>
        {/if}
        <button
          class="btn btn-glass"
          onclick={openPreferences}
          title={$t('topbar.notifications_menu.settings') || 'Settings'}
        >
          <Icon name="settings" size={16} />
          <span class="hide-xs">{$t('topbar.notifications_menu.settings') || 'Settings'}</span>
        </button>
      </div>
    </header>

    <div class="toolbar glass-card">
      <div class="search">
        <Icon name="search" size={18} />
        <input
          type="text"
          placeholder={$t('notifications_page.search_placeholder') || 'Search notifications...'}
          bind:value={searchQuery}
        />
        {#if searchQuery}
          <button
            class="clear"
            onclick={() => (searchQuery = '')}
            aria-label={$t('common.clear') || 'Clear'}
            title={$t('common.clear') || 'Clear'}
          >
            <Icon name="x" size={14} />
          </button>
        {/if}
      </div>

      <div
        class="filters"
        role="tablist"
        aria-label={$t('notifications_page.filters.aria') || 'Filters'}
      >
        <button
          class="chip"
          class:active={filter === 'all'}
          onclick={() => (filter = 'all')}
          role="tab"
          aria-selected={filter === 'all'}
        >
          {$t('notifications_page.filters.all') || 'All'}
        </button>
        <button
          class="chip"
          class:active={filter === 'unread'}
          onclick={() => (filter = 'unread')}
          role="tab"
          aria-selected={filter === 'unread'}
        >
          {$t('notifications_page.filters.unread') || 'Unread'}
          {#if $unreadCount > 0}
            <span class="chip-badge">{$unreadCount > 99 ? '99+' : $unreadCount}</span>
          {/if}
        </button>
      </div>
    </div>
  </div>

  <section class="list glass-card" aria-busy={$loading}>
    {#if $loading && $notifications.length === 0}
      <div class="center">
        <div class="spinner"></div>
        <p class="muted">
          {$t('notifications_page.loading') || 'Loading notifications...'}
        </p>
      </div>
    {:else if filteredNotifications.length === 0}
      <div class="empty">
        <div class="icon-bg">
          <Icon name="bell" size={24} />
        </div>
        <h3>{$t('notifications_page.empty.title') || "You're all caught up"}</h3>
        <p class="muted">
          {$t('notifications_page.empty.subtitle') || 'No notifications to show.'}
        </p>
      </div>
    {:else}
      <div class="items">
        {#each filteredNotifications as n (n.id)}
          <article class="item" class:unread={!n.is_read}>
            <button type="button" class="item-main" onclick={() => handleClick(n)}>
              <div class="left">
                <div class="type-dot {n.notification_type}"></div>
                <div class="text">
                  <div class="row">
                    <div class="title">{n.title}</div>
                    <div class="time">
                      {timeAgo(n.created_at)}
                    </div>
                  </div>
                  {#if n.message}
                    <div class="msg">{n.message}</div>
                  {/if}
                </div>
              </div>
            </button>

            <div class="right">
              {#if !n.is_read}
                <button
                  class="icon-btn"
                  title={$t('topbar.notifications_menu.mark_read') || 'Mark as read'}
                  aria-label={$t('topbar.notifications_menu.mark_read') || 'Mark as read'}
                  onclick={() => markAsRead(n.id)}
                >
                  <Icon name="check" size={16} />
                </button>
              {/if}
              <button
                class="icon-btn danger"
                title={$t('topbar.notifications_menu.delete') || 'Delete'}
                aria-label={$t('topbar.notifications_menu.delete') || 'Delete'}
                onclick={() => requestDelete(n)}
              >
                <Icon name="trash" size={16} />
              </button>
            </div>
          </article>
        {/each}
      </div>

      {#if hasMore}
        <div class="footer">
          <button class="btn btn-glass w-full" onclick={loadMore} disabled={$loading}>
            {#if $loading}
              <div class="spinner-sm"></div>
              {$t('common.loading') || 'Loading...'}
            {:else}
              {$t('notifications_page.load_more') || 'Load more'}
            {/if}
          </button>
        </div>
      {/if}
    {/if}
  </section>
</div>

<ConfirmDialog
  bind:show={showDeleteModal}
  title={$t('notifications_page.confirm_delete.title') || 'Delete notification'}
  message={$t('notifications_page.confirm_delete.message') ||
    'Delete this notification? This action cannot be undone.'}
  confirmText={$t('topbar.notifications_menu.delete') || 'Delete'}
  cancelText={$t('common.cancel') || 'Cancel'}
  type="danger"
  loading={deleting}
  onconfirm={confirmDelete}
  oncancel={() => {
    deleteTarget = null;
  }}
/>

<ConfirmDialog
  bind:show={showMarkAllModal}
  title={$t('notifications_page.confirm_mark_all.title') || 'Mark all as read'}
  message={$t('notifications_page.confirm_mark_all.message') || 'Mark all notifications as read?'}
  confirmText={$t('topbar.notifications_menu.mark_all_read') || 'Mark all read'}
  cancelText={$t('common.cancel') || 'Cancel'}
  type="info"
  loading={markingAll}
  onconfirm={confirmMarkAll}
/>

<style>
  .page-content {
    padding: clamp(16px, 2.5vw, 24px);
    max-width: 1100px;
    margin: 0 auto;
  }

  .sticky-header {
    position: sticky;
    top: 0;
    z-index: 2;
    background: var(--bg-surface);
    padding-top: 0.25rem;
  }

  .page-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  h1 {
    margin: 0;
    font-size: 1.2rem;
    font-weight: 800;
    color: var(--text-primary);
  }

  .muted {
    color: var(--text-secondary);
    margin: 0.25rem 0 0;
    font-size: 0.92rem;
    line-height: 1.4;
  }

  .actions-col {
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .glass-card {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.25);
  }

  .count-badge {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
    padding: 0.35rem 0.75rem;
    border-radius: 12px;
    font-size: 0.8rem;
    font-weight: 800;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .toolbar {
    padding: 0.9rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.8rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }

  .search {
    flex: 1;
    min-width: 220px;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 0.65rem 0.8rem;
  }

  .search input {
    width: 100%;
    background: transparent;
    border: 0;
    outline: none;
    color: var(--text-primary);
    font-size: 0.95rem;
  }

  .clear {
    width: 30px;
    height: 30px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .clear:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .hide-xs {
    display: inline;
  }

  .filters {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .chip {
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-secondary);
    border-radius: 12px;
    padding: 0.55rem 0.75rem;
    font-weight: 700;
    font-size: 0.9rem;
    cursor: pointer;
    display: inline-flex;
    gap: 0.5rem;
    align-items: center;
  }

  .chip.active {
    background: rgba(99, 102, 241, 0.16);
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--text-primary);
  }

  .chip-badge {
    background: rgba(239, 68, 68, 0.2);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #fecaca;
    border-radius: 999px;
    padding: 0.1rem 0.45rem;
    font-size: 0.78rem;
    font-weight: 800;
  }

  .list {
    padding: 0.25rem;
    overflow: hidden;
    margin-bottom: 1rem;
  }

  .items {
    display: flex;
    flex-direction: column;
  }

  .item {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.2rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    transition: background 0.15s;
  }

  .item:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .item.unread {
    background: rgba(99, 102, 241, 0.08);
  }

  .item-main {
    border: 0;
    background: transparent;
    text-align: left;
    padding: 0.75rem;
    flex: 1;
    min-width: 0;
    cursor: pointer;
    border-radius: 14px;
    color: inherit;
  }

  .item-main:focus-visible {
    outline: 2px solid rgba(99, 102, 241, 0.55);
    outline-offset: 2px;
  }

  .left {
    display: flex;
    gap: 0.75rem;
    flex: 1;
    min-width: 0;
  }

  .type-dot {
    width: 10px;
    height: 10px;
    border-radius: 999px;
    margin-top: 7px;
    flex-shrink: 0;
    background: var(--color-info);
  }

  .type-dot.success {
    background: var(--color-success);
  }
  .type-dot.warning {
    background: var(--color-warning);
  }
  .type-dot.error {
    background: var(--color-danger);
  }

  .text {
    flex: 1;
    min-width: 0;
  }

  .row {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: baseline;
  }

  .title {
    font-weight: 800;
    color: var(--text-primary);
    font-size: 0.98rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .time {
    color: var(--text-tertiary);
    font-size: 0.82rem;
    flex-shrink: 0;
  }

  .msg {
    margin-top: 0.25rem;
    color: var(--text-secondary);
    font-size: 0.92rem;
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .right {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
    align-items: center;
    padding: 0.75rem 0.75rem 0.75rem 0;
  }

  .icon-btn {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s;
  }

  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-primary);
  }

  .icon-btn.danger:hover {
    background: rgba(239, 68, 68, 0.16);
    border-color: rgba(239, 68, 68, 0.3);
    color: #fecaca;
  }

  .center,
  .empty {
    padding: 3rem 1rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 0.5rem;
  }

  .icon-bg {
    width: 48px;
    height: 48px;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.04);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    margin-bottom: 0.4rem;
  }

  .spinner {
    width: 22px;
    height: 22px;
    border: 2px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .spinner-sm {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-right: 0.5rem;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .footer {
    padding: 0.8rem;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  :global(.w-full) {
    width: 100%;
  }

  @media (max-width: 600px) {
    .page-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .actions-col {
      width: 100%;
      justify-content: flex-start;
    }

    .hide-xs {
      display: none;
    }

    .row {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.25rem;
    }

    .time {
      font-size: 0.78rem;
    }
  }

  /* Light theme tweaks */
  :global([data-theme='light']) .glass-card {
    background: #ffffff;
    border-color: rgba(0, 0, 0, 0.06);
    box-shadow:
      0 12px 32px rgba(0, 0, 0, 0.08),
      0 0 0 1px rgba(255, 255, 255, 0.9);
  }

  :global([data-theme='light']) .item {
    border-bottom-color: rgba(0, 0, 0, 0.06);
  }

  :global([data-theme='light']) .item:hover {
    background: rgba(0, 0, 0, 0.02);
  }

  :global([data-theme='light']) .item.unread {
    background: rgba(99, 102, 241, 0.08);
  }

  :global([data-theme='light']) .icon-btn {
    background: rgba(0, 0, 0, 0.02);
    border-color: rgba(0, 0, 0, 0.08);
    color: #475569;
  }

  :global([data-theme='light']) .icon-btn:hover {
    background: rgba(99, 102, 241, 0.12);
    border-color: rgba(99, 102, 241, 0.25);
    color: #111827;
  }

  :global([data-theme='light']) .count-badge {
    background: rgba(0, 0, 0, 0.04);
    border-color: rgba(0, 0, 0, 0.08);
    color: #475569;
  }
</style>
