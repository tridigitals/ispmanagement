<script lang="ts">
  import { onMount } from 'svelte';
  import { fly, fade } from 'svelte/transition';
  import { clickOutside } from '$lib/actions/clickOutside';
  import { t } from 'svelte-i18n';
  import Icon from '../ui/Icon.svelte';
  import { page } from '$app/stores';
  import {
    notifications,
    unreadCount,
    loading,
    loadNotifications,
    markAsRead,
    markAllAsRead,
    deleteNotification,
    deleteAllNotifications,
  } from '$lib/stores/notifications';
  import { timeAgo } from '$lib/utils/date';
  import { goto } from '$app/navigation';
  import { openProfileModal } from '$lib/stores/profileModal';
  import { openNotificationModal } from '$lib/stores/notificationModal';
  import { user } from '$lib/stores/auth';
  import { hasInternalAppAccess } from '$lib/utils/appLanding';
  import { resolveAnnouncementActionUrl } from '$lib/utils/announcementRouting';
  import { getVisiblePortalNotifications } from '$lib/utils/dashboardNotifications';
  import { api } from '$lib/api/client';

  let isOpen = $state(false);
  const OPEN_REFRESH_MIN_INTERVAL_MS = 10_000;
  let lastOpenRefreshAt = 0;
  let portalInvoiceIds = $state<string[]>([]);

  const tenantPrefix = '';
  let isSuperadminUrl = $derived($page.url.pathname.startsWith('/superadmin'));
  let visibleNotifications = $derived(
    getVisiblePortalNotifications($notifications, hasInternalAppAccess($user), portalInvoiceIds),
  );
  let visibleUnreadCount = $derived(visibleNotifications.filter((n) => !n.is_read).length);

  onMount(() => {
    if (hasInternalAppAccess($user)) return;
    void loadPortalInvoiceIds();
  });

  async function loadPortalInvoiceIds() {
    try {
      const invoiceRows = await api.payment.listInvoices();
      portalInvoiceIds = (invoiceRows || []).map((inv) => inv.id).filter(Boolean);
    } catch (e) {
      console.warn('Failed to load portal invoice ids for dropdown:', e);
      portalInvoiceIds = [];
    }
  }

  async function open() {
    isOpen = true;
    const now = Date.now();
    const shouldRefresh =
      $notifications.length === 0 ||
      ($unreadCount > 0 && now - lastOpenRefreshAt > OPEN_REFRESH_MIN_INTERVAL_MS);

    if (shouldRefresh) {
      lastOpenRefreshAt = now;
      await loadNotifications(1);
    }

    if (!hasInternalAppAccess($user)) {
      await loadPortalInvoiceIds();
    }
  }

  function toggle() {
    if (isOpen) close();
    else open();
  }

  function close() {
    isOpen = false;
  }

  function handleNotificationClick(n: any) {
    if (!n.is_read) {
      markAsRead(n.id);
    }
    if (n.action_url) {
      goto(resolveActionUrl(n.action_url));
      close();
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
        return 'alert-circle'; // or x-circle if available
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
</script>

<svelte:window
  onkeydown={(e) => {
    if (!isOpen) return;
    if (e.key === 'Escape') close();
  }}
/>

<div class="notification-dropdown" use:clickOutside={{ callback: close }}>
  <button
    class="icon-btn"
    onclick={toggle}
    title={$t('topbar.notifications')}
    aria-label={$t('topbar.notifications')}
    aria-haspopup="dialog"
    aria-expanded={isOpen}
  >
    <Icon name="bell" size={18} />
    {#if visibleUnreadCount > 0}
      <span class="badge-count" transition:fade>
        {visibleUnreadCount > 99 ? '99+' : visibleUnreadCount}
      </span>
    {/if}
  </button>

  {#if isOpen}
    <button
      type="button"
      class="backdrop"
      onclick={close}
      aria-label={$t('common.cancel') || 'Close'}
    ></button>
    <div
      class="dropdown-panel"
      transition:fly={{ y: 10, duration: 200 }}
      role="dialog"
      aria-label={$t('topbar.notifications')}
    >
      <div class="header">
        <h3>{$t('topbar.notifications')}</h3>
        <div class="actions">
          {#if visibleUnreadCount > 0}
            <button class="text-btn" onclick={() => markAllAsRead()}>
              {$t('topbar.notifications_menu.mark_all_read') || 'Mark all read'}
            </button>
          {/if}
          <button
            class="icon-btn-sm"
            onclick={() => {
              openProfileModal({ tab: 'notifications' });
              close();
            }}
            title={$t('topbar.notifications_menu.settings') || 'Settings'}
            aria-label={$t('topbar.notifications_menu.settings') || 'Settings'}
          >
            <Icon name="settings" size={14} />
          </button>
        </div>
      </div>

      <div class="content">
        {#if $loading && visibleNotifications.length === 0}
          <div class="loading">
            <div class="spinner"></div>
          </div>
        {:else if visibleNotifications.length === 0}
          <div class="empty-state">
            <div class="icon-bg">
              <Icon name="bell" size={24} color="var(--text-tertiary)" />
            </div>
            <p>
              {$t('topbar.notifications_menu.empty') || 'No notifications yet'}
            </p>
          </div>
        {:else}
          <div class="list">
            {#each visibleNotifications as n (n.id)}
              <div
                class="notification-item"
                class:unread={!n.is_read}
                onclick={() => handleNotificationClick(n)}
                role="button"
                tabindex="0"
                onkeydown={(e) => e.key === 'Enter' && handleNotificationClick(n)}
              >
                <div class="icon-col">
                  <Icon
                    name={getIconForType(n.notification_type)}
                    size={18}
                    color={getColorForType(n.notification_type)}
                  />
                </div>
                <div class="text-col">
                  <p class="title">{n.title}</p>
                  <p class="message">{n.message}</p>
                  <span class="time">{timeAgo(n.created_at)}</span>
                </div>
                <div class="actions-col">
                  {#if !n.is_read}
                    <button
                      class="action-btn"
                      onclick={(e) => {
                        e.stopPropagation();
                        markAsRead(n.id);
                      }}
                      title={$t('topbar.notifications_menu.mark_read') || 'Mark as read'}
                      aria-label={$t('topbar.notifications_menu.mark_read') || 'Mark as read'}
                    >
                      <div class="dot"></div>
                    </button>
                  {/if}
                  <button
                    class="delete-btn"
                    onclick={(e) => {
                      e.stopPropagation();
                      deleteNotification(n.id);
                    }}
                    title={$t('topbar.notifications_menu.delete') || 'Delete'}
                    aria-label={$t('topbar.notifications_menu.delete') || 'Delete'}
                  >
                    <Icon name="x" size={12} />
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <div class="footer">
        <button
          class="footer-link"
          onclick={() => {
            if (isSuperadminUrl) openProfileModal({ tab: 'notifications' });
            else openNotificationModal();
            close();
          }}
        >
          {$t('topbar.notifications_menu.view_all') || 'View all'}
        </button>
        {#if $notifications.length > 0}
          <button
            class="footer-link danger"
            onclick={async () => {
              await deleteAllNotifications();
              close();
            }}
          >
            <Icon name="trash" size={12} />
            {$t('topbar.notifications_menu.clear_all') || 'Clear all'}
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .notification-dropdown {
    position: relative;
    flex: 0 0 auto;
  }

  .icon-btn {
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
    position: relative;
    transition: all 0.2s;
  }

  .icon-btn:hover,
  :global(.icon-btn.active) {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .badge-count {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--color-danger);
    color: white;
    border-radius: 999px;
    font-size: 0.7rem;
    font-weight: 800;
    border: 2px solid var(--bg-primary);
    line-height: 1;
  }

  .backdrop {
    position: fixed;
    inset: 0;
    background: transparent;
    z-index: 999;
    border: 0;
    padding: 0;
  }

  .dropdown-panel {
    position: absolute;
    top: 100%;
    right: -10px; /* Align slightly specific to topbar layout */
    margin-top: 8px;
    width: 360px;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    box-shadow: var(--shadow-md);
    z-index: 1000;
    display: flex;
    flex-direction: column;
    max-height: 80vh;
    overflow: hidden;
  }

  .header {
    padding: 1rem;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--bg-hover);
  }

  .header h3 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .text-btn {
    background: none;
    border: none;
    color: var(--color-primary);
    font-size: 0.8rem;
    cursor: pointer;
    padding: 0.2rem 0.35rem;
    border-radius: 8px;
    white-space: nowrap;
  }

  .text-btn:hover {
    background: var(--bg-tertiary);
    text-decoration: none;
  }

  .icon-btn-sm {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    padding: 4px;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
  }

  .icon-btn-sm:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    min-height: 100px;
    max-height: 420px;
    background: var(--bg-surface);
  }

  .loading {
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 2rem;
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    color: var(--text-tertiary);
    gap: 0.5rem;
  }

  .icon-bg {
    width: 48px;
    height: 48px;
    background: var(--bg-hover);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 0.5rem;
  }

  .list {
    display: flex;
    flex-direction: column;
  }

  .notification-item {
    display: flex;
    padding: 1rem;
    gap: 0.8rem;
    border-bottom: 1px solid var(--border-color);
    cursor: pointer;
    transition: background 0.2s;
    position: relative;
  }

  .notification-item:hover {
    background: var(--bg-hover);
  }

  .notification-item.unread {
    background: var(--bg-active); /* Or slightly tinted */
  }

  .notification-item.unread:hover {
    background: var(--bg-tertiary);
  }

  .icon-col {
    padding-top: 2px;
    flex-shrink: 0;
  }

  .text-col {
    flex: 1;
    min-width: 0; /* Text truncation */
  }

  .title {
    font-size: 0.9rem;
    font-weight: 500;
    color: var(--text-primary);
    margin: 0 0 2px 0;
    line-height: 1.3;
  }

  .unread .title {
    font-weight: 600;
  }

  .message {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin: 0 0 4px 0;
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .time {
    font-size: 0.75rem;
    color: var(--text-tertiary);
  }

  .actions-col {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.5rem;
    opacity: 0; /* Hide by default, show on hover */
    transition: opacity 0.2s;
  }

  .notification-item:hover .actions-col {
    opacity: 1;
  }

  /* Always show if unread? Maybe just the dot */
  .notification-item.unread .actions-col {
    opacity: 1;
  }

  .action-btn,
  .delete-btn {
    width: 28px;
    height: 28px;
    border-radius: 8px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 0;
    transition: all 0.15s;
    color: var(--text-secondary);
  }

  .dot {
    width: 8px;
    height: 8px;
    background: var(--color-primary);
    border-radius: 50%;
  }

  .delete-btn:hover {
    background: var(--bg-hover);
    color: var(--color-danger);
  }

  .action-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .footer {
    padding: 0.6rem 1rem;
    border-top: 1px solid var(--border-color);
    background: var(--bg-hover);
    display: flex;
    justify-content: flex-end;
  }

  .footer-link {
    background: transparent;
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
    padding: 0.4rem 0.65rem;
    border-radius: 8px;
    font-size: 0.85rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .footer-link:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .footer-link.danger {
    color: var(--color-danger);
    border-color: color-mix(in srgb, var(--color-danger) 20%, var(--border-color));
  }

  .footer-link.danger:hover {
    background: color-mix(in srgb, var(--color-danger) 8%, transparent);
    color: var(--color-danger);
  }

  @media (max-width: 520px) {
    .backdrop {
      background: color-mix(in srgb, var(--bg-app) 58%, transparent);
    }

    .dropdown-panel {
      position: fixed;
      top: 72px;
      left: 12px;
      right: 12px;
      width: auto;
      max-width: calc(100dvw - 24px);
      box-sizing: border-box;
      margin-top: 0;
      max-height: calc(100vh - 88px);
    }

    .header {
      align-items: flex-start;
      gap: 0.75rem;
      padding: 0.9rem;
    }

    .header h3 {
      min-width: 0;
    }

    .actions {
      display: grid;
      grid-template-columns: 1fr auto;
      align-items: center;
      gap: 0.4rem;
      width: 100%;
    }

    .content {
      max-height: none;
    }

    .actions-col {
      opacity: 1;
      flex-direction: row;
      align-items: center;
    }

    .text-btn {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-height: 34px;
      padding: 0.35rem 0.55rem;
      font-size: 0.78rem;
    }

    .footer {
      padding: 0.7rem 0.9rem;
    }
  }
</style>
