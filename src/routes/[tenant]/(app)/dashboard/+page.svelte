<script lang="ts">
  import { user, isAdmin } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { formatDate, timeAgo } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import {
    api,
    type Announcement,
    type CustomerSubscriptionView,
    type Invoice,
    type PaginatedResponse,
  } from '$lib/api/client';
  import { stripHtmlToText } from '$lib/utils/sanitizeHtml';
  import {
    notifications,
    loading as notificationsLoading,
    loadNotifications,
  } from '$lib/stores/notifications';

  onMount(() => {
    // Auth handled by layout
    // Load a small slice of activity without blocking first paint.
    void loadNotifications(1);
    void loadDashboardAnnouncements();
    if (!$isAdmin) {
      void loadPortalSummary();
    }
  });

  const greeting = () => {
    const hour = new Date().getHours();
    if (hour < 12) return $t('dashboard.greeting.morning');
    if (hour < 17) return $t('dashboard.greeting.afternoon');
    return $t('dashboard.greeting.evening');
  };

  let tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  let tenantPrefix = $derived(tenantCtx.tenantPrefix);

  let recent = $derived($notifications.slice(0, 6));

  let annLoading = $state(false);
  let annPosts = $state<Announcement[]>([]);
  let portalSummaryLoading = $state(false);
  let activePortalSubscription = $state<CustomerSubscriptionView | null>(null);
  let nextPendingInvoice = $state<Invoice | null>(null);
  let portalHealthStatus = $derived.by(() => {
    const dueMs = invoiceDueMs(nextPendingInvoice);
    if (nextPendingInvoice && dueMs !== null && dueMs < Date.now()) return 'overdue' as const;
    if (nextPendingInvoice) return 'pending' as const;
    return 'normal' as const;
  });

  async function loadDashboardAnnouncements() {
    annLoading = true;
    try {
      const res: PaginatedResponse<Announcement> = await api.announcements.listRecent({
        page: 1,
        per_page: 3,
      });
      annPosts = (res.data || []).slice(0, 3);
    } catch (e) {
      // non-blocking
      console.warn('Failed to load dashboard announcements:', e);
    } finally {
      annLoading = false;
    }
  }

  async function loadPortalSummary() {
    portalSummaryLoading = true;
    try {
      const [subRes, invoiceRows] = await Promise.all([
        api.customers.portal.mySubscriptions({ page: 1, per_page: 50 }),
        api.payment.listInvoices(),
      ]);

      const subs = (subRes?.data || []).filter((s) => s.status === 'active');
      activePortalSubscription =
        subs.sort(
          (a, b) =>
            new Date(b.updated_at || b.created_at || 0).getTime() -
            new Date(a.updated_at || a.created_at || 0).getTime(),
        )[0] || null;

      const pending = (invoiceRows || []).filter((inv) => inv.status === 'pending');
      nextPendingInvoice =
        pending.sort(
          (a, b) =>
            new Date(a.due_date || a.created_at || 0).getTime() -
            new Date(b.due_date || b.created_at || 0).getTime(),
        )[0] || null;
    } catch (e) {
      // Keep dashboard resilient for customer portal.
      console.warn('Failed to load portal summary:', e);
    } finally {
      portalSummaryLoading = false;
    }
  }

  function formatInvoiceAmount(inv: Invoice | null) {
    if (!inv) return '-';
    const locale = ($appSettings as any)?.default_locale || 'id-ID';
    const currency = inv.currency_code || ($appSettings as any)?.currency_code || 'IDR';
    try {
      return new Intl.NumberFormat(locale, {
        style: 'currency',
        currency,
      }).format(inv.amount || 0);
    } catch {
      return `${currency} ${Number(inv.amount || 0).toLocaleString(locale)}`;
    }
  }

  function invoiceDateForDisplay(inv: Invoice | null): string | number | Date {
    if (!inv) return Date.now();
    return inv.due_date || inv.created_at || Date.now();
  }

  function invoiceDueMs(inv: Invoice | null): number | null {
    if (!inv) return null;
    const raw = inv.due_date || inv.created_at;
    if (!raw) return null;
    const parsed = new Date(raw).getTime();
    return Number.isFinite(parsed) ? parsed : null;
  }

  function portalStatusLabel(status: 'normal' | 'pending' | 'overdue') {
    if (status === 'overdue') {
      return $t('dashboard.portal_summary.status.overdue') || 'Overdue';
    }
    if (status === 'pending') {
      return $t('dashboard.portal_summary.status.pending') || 'Pending invoice';
    }
    return $t('dashboard.portal_summary.status.normal') || 'Normal';
  }

  function openAnnouncement(id: string) {
    goto(`${tenantPrefix}/announcements/${id}`);
  }

  function openNotification(n: any) {
    if (n?.action_url) goto(resolveActionUrl(n.action_url));
    else goto(`${tenantPrefix}/notifications`);
  }

  function resolveActionUrl(actionUrl: string) {
    if (!actionUrl || !tenantPrefix) return actionUrl;
    if (actionUrl.startsWith(tenantPrefix + '/')) return actionUrl;
    if (
      actionUrl.startsWith('/admin') ||
      actionUrl.startsWith('/support') ||
      actionUrl.startsWith('/dashboard') ||
      actionUrl.startsWith('/profile') ||
      actionUrl.startsWith('/notifications')
    ) {
      return `${tenantPrefix}${actionUrl}`;
    }
    return actionUrl;
  }

  function iconForType(type: string) {
    if (type === 'success') return 'check-circle';
    if (type === 'warning') return 'alert-triangle';
    if (type === 'error') return 'alert-circle';
    return 'info';
  }
</script>

<div class="dashboard-content fade-in">
  <header class="welcome-header">
    <div class="welcome-text">
      <h1>{greeting()}, {$user?.name}!</h1>
      <p>{$t('dashboard.greeting.welcome_message')}</p>
    </div>
  </header>

  {#if $isAdmin}
    <div
      class="admin-banner"
      onclick={() => goto(`${tenantPrefix}/admin`)}
      onkeydown={(e) => e.key === 'Enter' && goto(`${tenantPrefix}/admin`)}
      role="button"
      tabindex="0"
    >
      <div class="banner-content">
        <div class="banner-icon">
          <Icon name="shield" size={24} />
        </div>
        <div>
          <h3>{$t('dashboard.admin_mode.title')}</h3>
          <p>{$t('dashboard.admin_mode.description')}</p>
        </div>
      </div>
      <Icon name="arrow-right" size={20} />
    </div>
  {/if}

  <!-- Stats Row (User Focused) -->
  <div class="stats-grid">
    <div class="stat-card">
      <div class="stat-header">
        <div class="icon-wrapper primary">
          <Icon name="profile" size={20} />
        </div>
      </div>
      <div class="stat-body">
        <span class="stat-value">{$user?.role}</span>
        <span class="stat-label">{$t('dashboard.stats.account_role')}</span>
      </div>
    </div>

    <div class="stat-card">
      <div class="stat-header">
        <div class="icon-wrapper success">
          <Icon name="calendar" size={20} />
        </div>
      </div>
      <div class="stat-body">
        <span class="stat-value"
          >{formatDate($user?.created_at || Date.now(), {
            timeZone: $appSettings.app_timezone,
          })}</span
        >
        <span class="stat-label">{$t('dashboard.stats.member_since')}</span>
      </div>
    </div>

    <div class="stat-card">
      <div class="stat-header">
        <div class="icon-wrapper info">
          <Icon name="check" size={20} />
        </div>
      </div>
      <div class="stat-body">
        <span class="stat-value">{$t('dashboard.stats.active')}</span>
        <span class="stat-label">{$t('dashboard.stats.system_status')}</span>
      </div>
    </div>
  </div>

  <div class="main-grid">
    <section class="activity-section">
      <div class="section-header">
        <h2>{$t('dashboard.recent_activity.title')}</h2>
        <button class="text-btn" onclick={() => goto(`${tenantPrefix}/notifications`)}>
          {$t('dashboard.recent_activity.view_all')}
        </button>
      </div>

      <div class="card activity-card">
        {#if $notificationsLoading && recent.length === 0}
          <div class="loading-state">
            <div class="spinner"></div>
            <p class="muted">
              {$t('dashboard.recent_activity.loading') || $t('common.loading') || 'Loading...'}
            </p>
          </div>
        {:else if recent.length === 0}
          <div class="empty-state">
            <div class="empty-icon-circle">
              <Icon name="bell" size={32} />
            </div>
            <h3>{$t('dashboard.recent_activity.empty.title')}</h3>
            <p>{$t('dashboard.recent_activity.empty.description')}</p>
            <button
              class="btn btn-secondary mt-4"
              onclick={() => goto(`${tenantPrefix}/notifications`)}
            >
              {$t('dashboard.recent_activity.empty.learn_more')}
            </button>
          </div>
        {:else}
          <ul class="activity-list">
            {#each recent as n (n.id)}
              <li class="activity-li">
                <button type="button" class="activity-item" onclick={() => openNotification(n)}>
                  <div class="activity-icon {n.notification_type}">
                    <Icon name={iconForType(n.notification_type)} size={16} />
                  </div>
                  <div class="activity-text">
                    <div class="activity-row">
                      <span class="activity-title">{n.title}</span>
                      <span class="activity-time">{timeAgo(n.created_at)}</span>
                    </div>
                    {#if n.message}
                      <div class="activity-msg">
                        {n.message}
                      </div>
                    {/if}
                  </div>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <div class="section-header section-gap">
        <h2>
          {$t('dashboard.announcements.title') || $t('announcements.title') || 'Announcements'}
        </h2>
        <button class="text-btn" onclick={() => goto(`${tenantPrefix}/announcements`)}>
          {$t('dashboard.announcements.view_all') ||
            $t('dashboard.recent_activity.view_all') ||
            'View all'}
        </button>
      </div>

      <div class="card ann-card">
        {#if annLoading && annPosts.length === 0}
          <div class="loading-state ann-state">
            <div class="spinner"></div>
            <p class="muted">{$t('common.loading') || 'Loading...'}</p>
          </div>
        {:else if annPosts.length === 0}
          <div class="empty-state ann-state">
            <div class="empty-icon-circle">
              <Icon name="megaphone" size={32} />
            </div>
            <h3>
              {$t('dashboard.announcements.empty.title') ||
                $t('announcements.empty_feed') ||
                'No announcements yet.'}
            </h3>
            <p>
              {$t('dashboard.announcements.empty.description') ||
                'When the team publishes updates, they will show up here.'}
            </p>
            <button
              class="btn btn-secondary mt-4"
              onclick={() => goto(`${tenantPrefix}/announcements`)}
            >
              {$t('dashboard.announcements.empty.open') || 'Open announcements'}
            </button>
          </div>
        {:else}
          <ul class="ann-list">
            {#each annPosts as a (a.id)}
              <li class="ann-li">
                <button class="ann-item" type="button" onclick={() => openAnnouncement(a.id)}>
                  <div class="ann-dot {a.severity}"></div>
                  <div class="ann-text">
                    <div class="ann-row">
                      <div class="ann-title">{a.title}</div>
                      <div class="ann-time">{timeAgo(a.updated_at || a.created_at)}</div>
                    </div>
                    <div class="ann-body">{stripHtmlToText(a.body || '')}</div>
                  </div>
                  <div class="ann-go">
                    <Icon name="arrow-right" size={16} />
                  </div>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </section>

    <aside class="quick-actions">
      {#if !$isAdmin}
        <div class="section-header">
          <h2>{$t('dashboard.portal_summary.title') || 'Service Overview'}</h2>
        </div>
        <div class="portal-summary-card">
          {#if portalSummaryLoading}
            <div class="summary-loading">
              <div class="spinner"></div>
              <span>{$t('common.loading') || 'Loading...'}</span>
            </div>
          {:else}
            <div class="summary-health-row">
              <div class="summary-k">{$t('dashboard.portal_summary.health') || 'Status'}</div>
              <span class="summary-health {portalHealthStatus}">
                <span class="summary-health-dot"></span>
                {portalStatusLabel(portalHealthStatus)}
              </span>
            </div>

            <div class="summary-row">
              <div class="summary-k">
                {$t('dashboard.portal_summary.active_package') || 'Active package'}
              </div>
              <div class="summary-v">
                {#if activePortalSubscription}
                  {activePortalSubscription.package_name || activePortalSubscription.package_id}
                {:else}
                  {$t('dashboard.portal_summary.none') || '-'}
                {/if}
              </div>
              {#if activePortalSubscription}
                <div class="summary-sub">
                  {activePortalSubscription.billing_cycle} ·
                  {activePortalSubscription.location_label || '-'}
                </div>
              {/if}
            </div>

            <div class="summary-row">
              <div class="summary-k">
                {$t('dashboard.portal_summary.next_invoice') || 'Next invoice'}
              </div>
              <div class="summary-v">
                {#if nextPendingInvoice}
                  {nextPendingInvoice.invoice_number}
                {:else}
                  {$t('dashboard.portal_summary.no_pending_invoice') || 'No pending invoice'}
                {/if}
              </div>
              {#if nextPendingInvoice}
                <div class="summary-sub">
                  {formatInvoiceAmount(nextPendingInvoice)} ·
                  {$t('dashboard.portal_summary.due') || 'Due'}:
                  {formatDate(invoiceDateForDisplay(nextPendingInvoice), {
                    timeZone: $appSettings.app_timezone,
                  })}
                </div>
              {/if}
            </div>

            <div class="summary-actions">
              <button
                class="summary-btn"
                onclick={() => goto(`${tenantPrefix}/dashboard/packages`)}
              >
                <Icon name="package" size={15} />
                {$t('dashboard.portal_summary.manage_package') || 'Manage package'}
              </button>
              {#if nextPendingInvoice}
                <button
                  class="summary-btn primary"
                  onclick={() => goto(`/pay/${nextPendingInvoice!.id}`)}
                >
                  <Icon name="credit-card" size={15} />
                  {$t('dashboard.portal_summary.pay_now') || 'Pay now'}
                </button>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <div class="section-header">
        <h2>{$t('dashboard.quick_actions.title')}</h2>
      </div>
      <div class="actions-list">
        <button class="action-item" onclick={() => goto(`${tenantPrefix}/profile`)}>
          <Icon name="profile" size={18} />
          {$t('dashboard.quick_actions.update_profile')}
        </button>
        <button class="action-item" onclick={() => goto(`${tenantPrefix}/notifications`)}>
          <Icon name="mail" size={18} />
          {$t('dashboard.quick_actions.check_messages')}
        </button>
        <button class="action-item" onclick={() => goto(`${tenantPrefix}/profile?tab=security`)}>
          <Icon name="lock" size={18} />
          {$t('dashboard.quick_actions.security_settings')}
        </button>
        <button
          class="action-item"
          onclick={() => goto(`${tenantPrefix}/profile?tab=notifications`)}
        >
          <Icon name="help-circle" size={18} />
          {$t('dashboard.quick_actions.contact_support')}
        </button>
      </div>
    </aside>
  </div>
</div>

<style>
  .dashboard-content {
    padding: 2rem;
    max-width: 1200px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }

  @media (max-width: 640px) {
    .dashboard-content {
      padding: 1rem;
      gap: 1.5rem;
    }

    .welcome-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 1rem;
    }
  }

  /* Header */
  .welcome-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    margin-bottom: 0.5rem;
  }

  .welcome-text h1 {
    font-size: 1.85rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 0.5rem 0;
  }

  .welcome-text p {
    color: var(--text-secondary);
    font-size: 1rem;
    margin: 0;
  }

  /* Admin Banner */
  .admin-banner {
    background: var(--color-primary);
    color: white;
    border-radius: var(--radius-lg);
    padding: 1.25rem 1.5rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    cursor: pointer;
    transition:
      transform 0.2s,
      box-shadow 0.2s;
    box-shadow: 0 4px 12px rgba(99, 102, 241, 0.2);
  }

  .admin-banner:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 16px rgba(99, 102, 241, 0.3);
  }

  .banner-content {
    display: flex;
    align-items: center;
    gap: 1.25rem;
  }

  .banner-icon {
    background: rgba(255, 255, 255, 0.2);
    width: 48px;
    height: 48px;
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .banner-content h3 {
    margin: 0 0 0.15rem 0;
    font-size: 1.1rem;
    font-weight: 600;
  }

  .banner-content p {
    margin: 0;
    font-size: 0.9rem;
    opacity: 0.9;
  }

  /* Stats Grid */
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 1.5rem;
  }

  @media (max-width: 640px) {
    .stats-grid {
      grid-template-columns: 1fr;
      gap: 1rem;
    }
  }

  .stat-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    transition: border-color 0.2s;
  }

  .stat-card:hover {
    border-color: var(--color-primary);
  }

  .stat-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .icon-wrapper {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .icon-wrapper.primary {
    background: rgba(99, 102, 241, 0.1);
    color: var(--color-primary);
  }
  .icon-wrapper.success {
    background: rgba(34, 197, 94, 0.1);
    color: #22c55e;
  }
  .icon-wrapper.info {
    background: rgba(59, 130, 246, 0.1);
    color: #3b82f6;
  }

  .stat-body {
    display: flex;
    flex-direction: column;
  }

  .stat-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    text-transform: capitalize;
  }

  .stat-label {
    font-size: 0.875rem;
    color: var(--text-secondary);
    font-weight: 500;
  }

  /* Main Grid */
  .main-grid {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 2rem;
  }

  @media (max-width: 900px) {
    .main-grid {
      grid-template-columns: 1fr;
    }
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .section-header h2 {
    font-size: 1.15rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .text-btn {
    background: transparent;
    border: none;
    color: var(--color-primary);
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
  }

  /* Activity Card */
  .activity-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    min-height: 300px;
    overflow: hidden;
  }

  .section-gap {
    margin-top: 1.25rem;
  }

  .ann-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .ann-state {
    min-height: 220px;
  }

  .ann-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
  }

  .ann-li {
    border-bottom: 1px solid var(--border-color);
  }

  .ann-li:last-child {
    border-bottom: 0;
  }

  .ann-item {
    width: 100%;
    background: transparent;
    border: 0;
    padding: 1rem;
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 0.85rem;
    align-items: start;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s;
    color: inherit;
  }

  .ann-item:hover {
    background: var(--bg-hover);
  }

  .ann-dot {
    width: 10px;
    height: 10px;
    border-radius: 999px;
    margin-top: 0.35rem;
    background: rgba(148, 163, 184, 0.7);
    box-shadow: 0 0 0 6px rgba(148, 163, 184, 0.1);
  }

  .ann-dot.info {
    background: rgba(59, 130, 246, 0.95);
    box-shadow: 0 0 0 6px rgba(59, 130, 246, 0.12);
  }
  .ann-dot.success {
    background: rgba(34, 197, 94, 0.95);
    box-shadow: 0 0 0 6px rgba(34, 197, 94, 0.12);
  }
  .ann-dot.warning {
    background: rgba(245, 158, 11, 0.95);
    box-shadow: 0 0 0 6px rgba(245, 158, 11, 0.12);
  }
  .ann-dot.error {
    background: rgba(239, 68, 68, 0.95);
    box-shadow: 0 0 0 6px rgba(239, 68, 68, 0.12);
  }

  .ann-text {
    min-width: 0;
  }

  .ann-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .ann-title {
    font-weight: 750;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ann-time {
    font-size: 0.8rem;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .ann-body {
    margin-top: 0.25rem;
    font-size: 0.9rem;
    color: var(--text-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .ann-go {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-secondary);
    flex-shrink: 0;
    margin-top: 2px;
  }

  :global([data-theme='light']) .ann-go {
    border-color: rgba(0, 0, 0, 0.1);
    background: rgba(0, 0, 0, 0.03);
  }

  .empty-state {
    text-align: center;
    padding: 2rem;
    max-width: 320px;
    margin: 0 auto;
  }

  .empty-icon-circle {
    width: 64px;
    height: 64px;
    background: var(--bg-tertiary);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0 auto 1.5rem;
    color: var(--text-secondary);
    opacity: 0.5;
  }

  .empty-state h3 {
    font-size: 1.1rem;
    font-weight: 600;
    margin-bottom: 0.5rem;
  }

  .empty-state p {
    color: var(--text-secondary);
    font-size: 0.9rem;
    line-height: 1.5;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 300px;
    gap: 0.75rem;
    padding: 2rem;
  }

  .muted {
    color: var(--text-secondary);
    margin: 0;
  }

  .activity-list {
    display: flex;
    flex-direction: column;
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .activity-li {
    border-bottom: 1px solid var(--border-color);
  }

  .activity-li:last-child {
    border-bottom: 0;
  }

  .activity-item {
    width: 100%;
    background: transparent;
    border: 0;
    padding: 1rem;
    display: flex;
    gap: 0.9rem;
    align-items: flex-start;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s;
    color: inherit;
  }

  .activity-item:hover {
    background: var(--bg-hover);
  }

  .activity-icon {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-secondary);
    flex-shrink: 0;
    margin-top: 2px;
  }

  .activity-icon.success {
    color: var(--color-success);
    background: rgba(16, 185, 129, 0.12);
    border-color: rgba(16, 185, 129, 0.25);
  }

  .activity-icon.warning {
    color: var(--color-warning);
    background: rgba(245, 158, 11, 0.12);
    border-color: rgba(245, 158, 11, 0.25);
  }

  .activity-icon.error {
    color: var(--color-danger);
    background: rgba(239, 68, 68, 0.12);
    border-color: rgba(239, 68, 68, 0.25);
  }

  .activity-text {
    flex: 1;
    min-width: 0;
  }

  .activity-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .activity-title {
    font-weight: 700;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .activity-time {
    font-size: 0.8rem;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .activity-msg {
    margin-top: 0.25rem;
    font-size: 0.9rem;
    color: var(--text-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  /* Quick Actions */
  .actions-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .action-item {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    padding: 1rem;
    display: flex;
    align-items: center;
    gap: 1rem;
    font-size: 0.9rem;
    font-weight: 500;
    color: var(--text-primary);
    cursor: pointer;
    transition: all 0.2s;
    text-align: left;
  }

  .action-item:hover {
    border-color: var(--color-primary);
    background: var(--bg-hover);
    transform: translateX(4px);
  }

  .portal-summary-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 1rem;
    margin-bottom: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }

  .summary-row {
    display: flex;
    flex-direction: column;
    gap: 0.22rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px dashed var(--border-color);
  }

  .summary-row:last-of-type {
    border-bottom: 0;
    padding-bottom: 0;
  }

  .summary-health-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px dashed var(--border-color);
  }

  .summary-health {
    display: inline-flex;
    align-items: center;
    gap: 0.38rem;
    font-size: 0.76rem;
    border-radius: 999px;
    padding: 0.22rem 0.55rem;
    border: 1px solid transparent;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .summary-health-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: currentColor;
  }

  .summary-health.normal {
    color: #16a34a;
    background: rgba(22, 163, 74, 0.12);
    border-color: rgba(22, 163, 74, 0.3);
  }

  .summary-health.pending {
    color: #d97706;
    background: rgba(217, 119, 6, 0.12);
    border-color: rgba(217, 119, 6, 0.3);
  }

  .summary-health.overdue {
    color: #dc2626;
    background: rgba(220, 38, 38, 0.12);
    border-color: rgba(220, 38, 38, 0.3);
  }

  .summary-k {
    font-size: 0.78rem;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 700;
  }

  .summary-v {
    font-size: 0.98rem;
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1.3;
  }

  .summary-sub {
    font-size: 0.83rem;
    color: var(--text-secondary);
  }

  .summary-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .summary-btn {
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border-radius: 10px;
    padding: 0.5rem 0.75rem;
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
  }

  .summary-btn:hover {
    background: var(--bg-hover);
  }

  .summary-btn.primary {
    background: var(--color-primary);
    border-color: color-mix(in srgb, var(--color-primary) 70%, transparent);
    color: #fff;
  }

  .summary-loading {
    display: inline-flex;
    align-items: center;
    gap: 0.6rem;
    color: var(--text-secondary);
    font-size: 0.88rem;
  }

  /* Buttons */
  .btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 1.2rem;
    border-radius: 8px;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    border: none;
  }

  .btn-primary {
    background: var(--color-primary);
    color: white;
  }

  .btn-primary:hover {
    filter: brightness(1.1);
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .btn-secondary:hover {
    background: var(--bg-hover);
  }

  .mt-4 {
    margin-top: 1rem;
  }

  .fade-in {
    animation: fadeIn 0.4s ease-out;
  }

  .spinner {
    width: 18px;
    height: 18px;
    border: 2px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
