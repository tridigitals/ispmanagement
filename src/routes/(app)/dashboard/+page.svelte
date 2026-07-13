<script lang="ts">
  import { user, isAdmin } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { hasInternalAppAccess } from '$lib/utils/appLanding';
  import { openProfileModal } from '$lib/stores/profileModal';
  import { openNotificationModal } from '$lib/stores/notificationModal';
  import {
    getAnnouncementDetailPath,
    resolveAnnouncementActionUrl,
  } from '$lib/utils/announcementRouting';
  import { formatDate, timeAgo } from '$lib/utils/date';
  import { getDashboardRecentNotifications } from '$lib/utils/dashboardNotifications';
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
  let portalInvoiceIds = $state<string[]>([]);

  let recent = $derived(
    getDashboardRecentNotifications(
      $notifications,
      hasInternalAppAccess($user),
      6,
      portalInvoiceIds,
    ),
  );

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

      portalInvoiceIds = (invoiceRows || []).map((inv) => inv.id).filter(Boolean);
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
      portalInvoiceIds = [];
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
    goto(getAnnouncementDetailPath(id, { tenantPrefix, internal: hasInternalAppAccess($user) }));
  }

  function openNotification(n: any) {
    if (n?.action_url) goto(resolveActionUrl(n.action_url));
    else openNotificationModal();
  }

  function resolveActionUrl(actionUrl: string) {
    return resolveAnnouncementActionUrl(actionUrl, {
      tenantPrefix,
      internal: hasInternalAppAccess($user),
    });
  }

  function iconForType(type: string) {
    if (type === 'success') return 'check-circle';
    if (type === 'warning') return 'alert-triangle';
    if (type === 'error') return 'alert-circle';
    return 'info';
  }
</script>

<div class="dashboard-content fade-in">
  <!-- Hero Welcome -->
  <section class="hero-card welcome-hero">
    <div class="welcome-body">
      <h1 class="welcome-greeting">{greeting()}, <span class="welcome-name">{$user?.name}</span></h1>
      <p class="welcome-sub">{$t('dashboard.greeting.welcome_message')}</p>
      <div class="welcome-meta">
        <span class="welcome-badge"><Icon name="calendar" size={13} /> {$t('dashboard.stats.member_since')}: {formatDate($user?.created_at || Date.now(), { timeZone: $appSettings.app_timezone })}</span>
        <span class="welcome-badge"><Icon name="check-circle" size={13} /> {$t('dashboard.stats.active')}</span>
      </div>
    </div>
  </section>

  {#if $isAdmin}
    <div class="admin-banner" onclick={() => goto(`${tenantPrefix}/admin`)} onkeydown={(e) => e.key === 'Enter' && goto(`${tenantPrefix}/admin`)} role="button" tabindex="0">
      <div class="banner-content">
        <div class="banner-icon"><Icon name="shield" size={22} /></div>
        <div>
          <h3>{$t('dashboard.admin_mode.title')}</h3>
          <p>{$t('dashboard.admin_mode.description')}</p>
        </div>
      </div>
      <Icon name="arrow-right" size={18} />
    </div>
  {/if}

  <!-- Bento Stats -->
  <div class="bento-grid">
    <div class="bento-card">
      <div class="bento-icon" style="background:color-mix(in srgb, var(--color-primary) 18%, transparent);color:var(--color-primary)">
        <Icon name="package" size={18} />
      </div>
      <span class="bento-value">
        {#if activePortalSubscription}
          {activePortalSubscription.package_name || 'Aktif'}
        {:else}
          —
        {/if}
      </span>
      <span class="bento-label">{$t('dashboard.portal_summary.active_package') || 'Paket Aktif'}</span>
    </div>

    <div class="bento-card">
      <div class="bento-icon" style="background:color-mix(in srgb, var(--color-warning) 18%, transparent);color:var(--color-warning)">
        <Icon name="credit-card" size={18} />
      </div>
      <span class="bento-value" style="background:linear-gradient(135deg,#fbbf24,#f59e0b);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text">
        {#if nextPendingInvoice}
          {formatInvoiceAmount(nextPendingInvoice)}
        {:else}
          Rp 0
        {/if}
      </span>
      <span class="bento-label">
        {#if nextPendingInvoice}
          {$t('dashboard.portal_summary.due') || 'Jatuh Tempo'}: {formatDate(invoiceDateForDisplay(nextPendingInvoice), { timeZone: $appSettings.app_timezone })}
        {:else}
          {$t('dashboard.portal_summary.no_pending_invoice') || 'Tidak ada tagihan'}
        {/if}
      </span>
    </div>

    <div class="bento-card">
      <div class="bento-icon" style="background:color-mix(in srgb, var(--color-success) 18%, transparent);color:var(--color-success)">
        <Icon name="shield" size={18} />
      </div>
      <span class="bento-value" style="background:linear-gradient(135deg,#7dd3ae,#34d399);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text">
        {portalStatusLabel(portalHealthStatus)}
      </span>
      <span class="bento-label">Status Akun</span>
    </div>

    <div class="bento-card">
      <div class="bento-icon" style="background:color-mix(in srgb, var(--color-info, #3b82f6) 18%, transparent);color:var(--color-info, #3b82f6)">
        <Icon name="message-circle" size={18} />
      </div>
      <span class="bento-value">{$notifications.length || 0}</span>
      <span class="bento-label">Notifikasi</span>
    </div>
  </div>

  <!-- Main Grid: Activity + Sidebar -->
  <div class="main-grid">
    <section class="activity-section">
      <!-- Activity Feed -->
      <div class="section-header">
        <h2>{$t('dashboard.recent_activity.title')}</h2>
        <button class="text-btn" onclick={() => openNotificationModal()}>{$t('dashboard.recent_activity.view_all')}</button>
      </div>
      <div class="glass-card activity-feed">
        {#if $notificationsLoading && recent.length === 0}
          <div class="loading-state"><div class="spinner"></div><p class="muted">{$t('common.loading') || 'Loading...'}</p></div>
        {:else if recent.length === 0}
          <div class="empty-state">
            <div class="empty-icon-circle"><Icon name="bell" size={32} /></div>
            <h3>{$t('dashboard.recent_activity.empty.title')}</h3>
            <p>{$t('dashboard.recent_activity.empty.description')}</p>
            <button class="btn btn-secondary mt-4" onclick={() => openNotificationModal()}>{$t('dashboard.recent_activity.empty.learn_more')}</button>
          </div>
        {:else}
          <ul class="activity-list">
            {#each recent as n (n.id)}
              <li class="activity-li">
                <button type="button" class="activity-item" onclick={() => openNotification(n)}>
                  <div class="activity-icon {n.notification_type}"><Icon name={iconForType(n.notification_type)} size={16} /></div>
                  <div class="activity-text">
                    <div class="activity-row"><span class="activity-title">{n.title}</span><span class="activity-time">{timeAgo(n.created_at)}</span></div>
                    {#if n.message}<div class="activity-msg">{n.message}</div>{/if}
                  </div>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <!-- Announcements -->
      <div class="section-header" style="margin-top:1.25rem">
        <h2>{$t('dashboard.announcements.title') || $t('announcements.title') || 'Announcements'}</h2>
        <button class="text-btn" onclick={() => goto(`${tenantPrefix}/announcements`)}>{$t('dashboard.announcements.view_all') || 'View all'}</button>
      </div>
      <div class="glass-card ann-feed">
        {#if annLoading && annPosts.length === 0}
          <div class="loading-state ann-state"><div class="spinner"></div><p class="muted">{$t('common.loading')}</p></div>
        {:else if annPosts.length === 0}
          <div class="empty-state ann-state">
            <div class="empty-icon-circle"><Icon name="megaphone" size={32} /></div>
            <h3>{$t('dashboard.announcements.empty.title') || 'No announcements yet.'}</h3>
            <p>{$t('dashboard.announcements.empty.description')}</p>
            <button class="btn btn-secondary mt-4" onclick={() => goto(`${tenantPrefix}/announcements`)}>{$t('dashboard.announcements.empty.open')}</button>
          </div>
        {:else}
          <ul class="ann-list">
            {#each annPosts as a (a.id)}
              <li class="ann-li">
                <button class="ann-item" type="button" onclick={() => openAnnouncement(a.id)}>
                  <div class="ann-dot {a.severity}"></div>
                  <div class="ann-text">
                    <div class="ann-row"><div class="ann-title">{a.title}</div><div class="ann-time">{timeAgo(a.updated_at || a.created_at)}</div></div>
                    <div class="ann-body">{stripHtmlToText(a.body || '')}</div>
                  </div>
                  <div class="ann-go"><Icon name="arrow-right" size={16} /></div>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </section>

    <!-- Sidebar: Billing + Quick Actions -->
    <aside class="quick-actions">
      {#if !$isAdmin}
        <div class="section-header"><h2>{$t('dashboard.portal_summary.title')}</h2></div>
        <div class="billing-card">
          {#if portalSummaryLoading}
            <div class="summary-loading"><div class="spinner"></div><span>{$t('common.loading')}</span></div>
          {:else}
            <div class="billing-header">
              <span class="billing-label">{$t('dashboard.portal_summary.health')}</span>
              <span class="summary-health {portalHealthStatus}"><span class="summary-health-dot"></span>{portalStatusLabel(portalHealthStatus)}</span>
            </div>
            <div class="billing-body">
              <span class="billing-amount">{nextPendingInvoice ? formatInvoiceAmount(nextPendingInvoice) : 'Rp 0'}</span>
              <span class="billing-due">
                <Icon name="clock" size={13} />
                {#if nextPendingInvoice}
                  {$t('dashboard.portal_summary.due') || 'Due'}: {formatDate(invoiceDateForDisplay(nextPendingInvoice), { timeZone: $appSettings.app_timezone })}
                {:else}
                  {$t('dashboard.portal_summary.no_pending_invoice') || 'No pending invoice'}
                {/if}
              </span>
            </div>
            <div class="billing-actions">
              <button class="btn btn-secondary btn-sm" onclick={() => goto(`${tenantPrefix}/dashboard/services`)}>
                <Icon name="package" size={13} /> {$t('dashboard.portal_summary.manage_package')}
              </button>
              {#if nextPendingInvoice}
                <button class="btn btn-primary btn-sm" onclick={() => goto(`/pay/${nextPendingInvoice!.id}`)}>
                  <Icon name="credit-card" size={13} /> {$t('dashboard.portal_summary.pay_now')}
                </button>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <div class="section-header" style="margin-top:1rem"><h2>{$t('dashboard.quick_actions.title')}</h2></div>
      <div class="actions-list">
        <button class="action-item" onclick={() => openProfileModal({ tab: 'general' })}><Icon name="profile" size={16} />{$t('dashboard.quick_actions.update_profile')}</button>
        <button class="action-item" onclick={() => openNotificationModal()}><Icon name="mail" size={16} />{$t('dashboard.quick_actions.check_messages')}</button>
        <button class="action-item" onclick={() => openProfileModal({ tab: 'security' })}><Icon name="lock" size={16} />{$t('dashboard.quick_actions.security_settings')}</button>
        <button class="action-item" onclick={() => openProfileModal({ tab: 'notifications' })}><Icon name="message-circle" size={16} />{$t('dashboard.quick_actions.contact_support')}</button>
      </div>
    </aside>
  </div>
</div>

<style>
  .dashboard-content {
    padding: clamp(1rem, 2.2vw, 2rem);
    max-width: 1260px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  @media (max-width: 640px) {
    .dashboard-content { padding: .75rem; gap: 1rem; }
  }

  /* Hero Welcome */
  .welcome-hero {
    padding: 1.5rem 1.75rem;
  }
  .welcome-body { position: relative; z-index: 1; }
  .welcome-greeting {
    font-size: clamp(1.4rem, 2.2vw, 1.85rem);
    font-weight: 750; color: var(--text-primary); margin: 0 0 .35rem;
  }
  .welcome-name {
    background: linear-gradient(135deg, var(--color-primary), #c4b5fd);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
    background-clip: text;
  }
  .welcome-sub { color: var(--text-secondary); margin: 0 0 .75rem; font-size: .92rem; }
  .welcome-meta { display: flex; gap: 1rem; flex-wrap: wrap; }
  .welcome-badge {
    display: inline-flex; align-items: center; gap: .35rem;
    font-size: .78rem; color: var(--text-tertiary);
    padding: .25rem .6rem; border-radius: 6px;
    background: rgba(255,255,255,.04); border: 1px solid rgba(255,255,255,.05);
  }

  /* Admin Banner */
  .admin-banner {
    background: linear-gradient(160deg, rgba(255,255,255,.035), rgba(255,255,255,.005));
    border: 1px solid rgba(255,255,255,.07);
    border-radius: var(--radius-lg);
    padding: 1rem 1.25rem;
    display: flex; justify-content: space-between; align-items: center;
    cursor: pointer; transition: all .2s;
  }
  .admin-banner:hover {
    border-color: color-mix(in srgb, var(--color-primary) 30%, rgba(255,255,255,.07));
    transform: translateY(-1px);
  }
  .banner-content { display: flex; align-items: center; gap: 1rem; }
  .banner-icon {
    width: 38px; height: 38px; border-radius: 10px;
    background: color-mix(in srgb, var(--color-primary) 18%, transparent);
    color: var(--color-primary);
    display: flex; align-items: center; justify-content: center;
  }
  .banner-content h3 { margin: 0 0 .1rem; font-size: 1rem; font-weight: 650; }
  .banner-content p { margin: 0; font-size: .82rem; color: var(--text-secondary); }

  /* Main Grid */
  .main-grid {
    display: grid; grid-template-columns: 2fr 1fr; gap: 1.25rem;
  }
  @media (max-width: 900px) {
    .main-grid { grid-template-columns: 1fr; }
  }

  .section-header h2 { font-size: .95rem; font-weight: 650; color: var(--text-primary); margin: 0; }
  .section-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: .75rem; }

  .text-btn { background: transparent; border: none; color: var(--color-primary); font-size: .82rem; font-weight: 600; cursor: pointer; }

  /* Glass Feed */
  .activity-feed, .ann-feed { min-height: 200px; }
  .ann-state { min-height: 160px; }

  .empty-state { text-align: center; padding: 2rem; max-width: 320px; margin: 0 auto; }
  .empty-state h3 { font-size: 1rem; font-weight: 650; margin: .75rem 0 .4rem; color: var(--text-primary); }
  .empty-state p { color: var(--text-secondary); font-size: .85rem; line-height: 1.5; margin: 0 0 1rem; }
  .empty-icon-circle {
    width: 56px; height: 56px; border-radius: 12px;
    background: rgba(255,255,255,.04); display: flex; align-items: center; justify-content: center;
    margin: 0 auto 1rem; color: var(--text-secondary); opacity: .5;
  }

  .loading-state { display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 200px; gap: .6rem; padding: 2rem; }
  .muted { color: var(--text-secondary); margin: 0; }

  .spinner { width: 18px; height: 18px; border: 2px solid rgba(255,255,255,.08); border-top-color: var(--color-primary); border-radius: 50%; animation: spin .7s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes fadeIn { from { opacity:0; transform:translateY(10px); } }

  /* Activity */
  .activity-list { display: flex; flex-direction: column; list-style: none; padding: 0; margin: 0; }
  .activity-li { border-bottom: 1px solid rgba(255,255,255,.04); }
  .activity-li:last-child { border-bottom: 0; }
  .activity-item {
    width: 100%; background: transparent; border: 0;
    padding: .75rem; display: flex; gap: .8rem; align-items: flex-start;
    text-align: left; cursor: pointer; transition: background .12s; color: inherit;
  }
  .activity-item:hover { background: rgba(255,255,255,.02); }
  .activity-icon {
    width: 32px; height: 32px; border-radius: 8px;
    display: flex; align-items: center; justify-content: center;
    border: 1px solid rgba(255,255,255,.08); background: rgba(255,255,255,.02);
    color: var(--text-secondary); flex-shrink: 0; margin-top: 1px;
  }
  .activity-icon.success { color: var(--color-success); background: color-mix(in srgb, var(--color-success) 14%, transparent); border-color: color-mix(in srgb, var(--color-success) 24%, rgba(255,255,255,.08)); }
  .activity-icon.warning { color: var(--color-warning); background: color-mix(in srgb, var(--color-warning) 14%, transparent); border-color: color-mix(in srgb, var(--color-warning) 24%, rgba(255,255,255,.08)); }
  .activity-icon.error { color: var(--color-danger); background: color-mix(in srgb, var(--color-danger) 14%, transparent); border-color: color-mix(in srgb, var(--color-danger) 24%, rgba(255,255,255,.08)); }
  .activity-text { flex: 1; min-width: 0; }
  .activity-row { display: flex; align-items: baseline; justify-content: space-between; gap: .75rem; }
  .activity-title { font-weight: 650; color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: .88rem; }
  .activity-time { font-size: .75rem; color: var(--text-tertiary); flex-shrink: 0; }
  .activity-msg { margin-top: .2rem; font-size: .82rem; color: var(--text-secondary); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }

  /* Announcements */
  .ann-list { display: flex; flex-direction: column; list-style: none; padding: 0; margin: 0; }
  .ann-li { border-bottom: 1px solid rgba(255,255,255,.04); }
  .ann-li:last-child { border-bottom: 0; }
  .ann-item {
    width: 100%; background: transparent; border: 0;
    padding: .75rem; display: grid; grid-template-columns: auto 1fr auto;
    gap: .7rem; align-items: start; text-align: left; cursor: pointer;
    transition: background .12s; color: inherit;
  }
  .ann-item:hover { background: rgba(255,255,255,.02); }
  .ann-dot { width: 8px; height: 8px; border-radius: 50%; margin-top: .35rem; background: var(--text-tertiary); }
  .ann-dot.info { background: var(--color-primary); }
  .ann-dot.success { background: var(--color-success); }
  .ann-dot.warning { background: var(--color-warning); }
  .ann-dot.error { background: var(--color-danger); }
  .ann-text { min-width: 0; }
  .ann-row { display: flex; align-items: baseline; justify-content: space-between; gap: .5rem; }
  .ann-title { font-weight: 650; color: var(--text-primary); font-size: .88rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .ann-time { font-size: .75rem; color: var(--text-tertiary); flex-shrink: 0; }
  .ann-body { margin-top: .2rem; font-size: .82rem; color: var(--text-secondary); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .ann-go {
    width: 30px; height: 30px; border-radius: 8px;
    display: flex; align-items: center; justify-content: center;
    border: 1px solid rgba(255,255,255,.08); background: rgba(255,255,255,.02);
    color: var(--text-secondary); flex-shrink: 0; margin-top: 2px;
  }

  @media (max-width: 560px) {
    .ann-item { grid-template-columns: auto minmax(0, 1fr); padding: .65rem; }
    .ann-go { display: none; }
    .ann-row, .activity-row { flex-direction: column; gap: .2rem; }
    .ann-title, .activity-title { white-space: normal; }
  }

  /* Billing Card */
  .billing-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: .75rem; }
  .billing-label { font-size: .78rem; color: var(--text-tertiary); text-transform: uppercase; letter-spacing: .04em; font-weight: 650; }
  .billing-body { display: flex; flex-direction: column; gap: .4rem; margin-bottom: 1rem; }
  .billing-actions { display: flex; gap: .5rem; flex-wrap: wrap; }

  .summary-health {
    display: inline-flex; align-items: center; gap: .35rem;
    font-size: .72rem; font-weight: 700; text-transform: uppercase; letter-spacing: .03em;
    border-radius: 999px; padding: .18rem .5rem; border: 1px solid transparent;
  }
  .summary-health.normal { color: var(--color-success); background: color-mix(in srgb, var(--color-success) 14%, transparent); border-color: color-mix(in srgb, var(--color-success) 30%, rgba(255,255,255,.08)); }
  .summary-health.pending { color: var(--color-warning); background: color-mix(in srgb, var(--color-warning) 14%, transparent); border-color: color-mix(in srgb, var(--color-warning) 30%, rgba(255,255,255,.08)); }
  .summary-health.overdue { color: var(--color-danger); background: color-mix(in srgb, var(--color-danger) 14%, transparent); border-color: color-mix(in srgb, var(--color-danger) 30%, rgba(255,255,255,.08)); }
  .summary-health-dot { width: 6px; height: 6px; border-radius: 50%; background: currentColor; }
  .summary-loading { display: inline-flex; align-items: center; gap: .5rem; color: var(--text-secondary); font-size: .84rem; }

  /* Quick Actions */
  .actions-list { display: flex; flex-direction: column; gap: .5rem; }
  .action-item {
    background: rgba(255,255,255,.025); border: 1px solid rgba(255,255,255,.05);
    border-radius: 10px; padding: .75rem .85rem;
    display: flex; align-items: center; gap: .75rem;
    font-size: .84rem; font-weight: 550; color: var(--text-primary);
    cursor: pointer; transition: all .15s; text-align: left;
  }
  .action-item:hover {
    border-color: color-mix(in srgb, var(--color-primary) 30%, rgba(255,255,255,.05));
    background: rgba(255,255,255,.04); transform: translateX(2px);
  }

  /* Buttons */
  .btn { display: flex; align-items: center; gap: .4rem; padding: .55rem 1rem; border-radius: 8px; font-size: .85rem; font-weight: 600; cursor: pointer; transition: all .15s; border: none; }
  .btn-primary { background: var(--color-primary); color: #fff; }
  .btn-primary:hover { filter: brightness(1.15); }
  .btn-secondary { background: rgba(255,255,255,.05); color: var(--text-primary); border: 1px solid rgba(255,255,255,.08); }
  .btn-secondary:hover { background: rgba(255,255,255,.08); }
  .btn-sm { padding: .4rem .7rem; font-size: .78rem; gap: .3rem; }
  .mt-4 { margin-top: 1rem; }
  .fade-in { animation: fadeIn .35s ease-out; }
</style>
