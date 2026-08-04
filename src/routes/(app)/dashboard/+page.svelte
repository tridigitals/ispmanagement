<script lang="ts">
  import { user, isAdmin } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { hasInternalAppAccess } from '$lib/utils/appLanding';
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

  let activeSubscriptions = $state<CustomerSubscriptionView[]>([]);
  let pendingInvoices = $state<Invoice[]>([]);
  let totalPending = $state(0);
  let openTicketCount = $state(0);

  let overdueInvoices = $derived.by(() => {
    const now = Date.now();
    return pendingInvoices.filter((inv) => {
      const st = String(inv.status || '').toLowerCase();
      if (st === 'verification_pending') return false; // already paid, waiting review
      const d = new Date(inv.due_date || inv.created_at || 0).getTime();
      return Number.isFinite(d) && d < now;
    });
  });

  let verificationInvoices = $derived.by(() =>
    pendingInvoices.filter((inv) => String(inv.status || '').toLowerCase() === 'verification_pending'),
  );

  let payableInvoices = $derived.by(() =>
    pendingInvoices.filter((inv) => {
      const st = String(inv.status || '').toLowerCase();
      return st === 'pending' || st === 'failed';
    }),
  );

  let sortedSubscriptions = $derived.by(() => {
    return [...activeSubscriptions].sort((a, b) => {
      const rank = (s: CustomerSubscriptionView) => {
        const st = subStatusLabel(s);
        if (st === 'overdue') return 0;
        if (st === 'suspended') return 1;
        if (st === 'grace') return 2;
        return 3;
      };
      const ra = rank(a);
      const rb = rank(b);
      if (ra !== rb) return ra - rb;
      const da = invoiceDueMs(a) ?? Number.MAX_SAFE_INTEGER;
      const db = invoiceDueMs(b) ?? Number.MAX_SAFE_INTEGER;
      return da - db;
    });
  });

  let nearestDueInvoice = $derived.by(() => {
    if (pendingInvoices.length === 0) return null;
    return [...pendingInvoices].sort(
      (a, b) =>
        new Date(a.due_date || a.created_at || 0).getTime() -
        new Date(b.due_date || b.created_at || 0).getTime(),
    )[0];
  });

  let nearestDueSubName = $derived.by(() => {
    const inv = nearestDueInvoice;
    if (!inv) return '';
    const match = activeSubscriptions.find(
      (s) =>
        (inv as any).subscription_id === s.id ||
        (inv.description || '').includes(s.package_name || ''),
    );
    return match?.package_name || inv.description || inv.invoice_number || '';
  });

  let locationCount = $derived.by(() => {
    const labels = new Set(
      activeSubscriptions.map((s) => s.location_label || s.location_id).filter(Boolean),
    );
    return labels.size;
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
      console.warn('Failed to load dashboard announcements:', e);
    } finally {
      annLoading = false;
    }
  }

  async function loadPortalSummary() {
    portalSummaryLoading = true;
    try {
      const [subRes, invoiceRows, ticketRes] = await Promise.all([
        api.customers.portal.mySubscriptions({ page: 1, per_page: 50 }),
        api.payment.listInvoices(),
        api.support.list({ perPage: 20 }).catch(() => ({ data: [] as any[] })),
      ]);

      // Show live packages (not cancelled/ended). Suspended stays visible — often unpaid.
      activeSubscriptions = (subRes?.data || []).filter((s) => {
        const st = String(s.status || '').toLowerCase();
        return st !== 'cancelled' && st !== 'ended' && st !== 'terminated';
      });

      portalInvoiceIds = (invoiceRows || []).map((inv) => inv.id).filter(Boolean);
      // pending + verification_pending = still open for customer attention
      pendingInvoices = (invoiceRows || []).filter((inv) => {
        const st = String(inv.status || '').toLowerCase();
        return st === 'pending' || st === 'verification_pending' || st === 'failed';
      });
      totalPending = pendingInvoices.reduce((sum, inv) => sum + (inv.amount || 0), 0);

      const tickets = ticketRes?.data || [];
      openTicketCount = tickets.filter(
        (t: any) => t.status === 'open' || t.status === 'pending',
      ).length;
    } catch (e) {
      console.warn('Failed to load portal summary:', e);
      portalInvoiceIds = [];
      activeSubscriptions = [];
      pendingInvoices = [];
      totalPending = 0;
      openTicketCount = 0;
    } finally {
      portalSummaryLoading = false;
    }
  }

  function formatAmount(amount: number, currency?: string) {
    const locale = ($appSettings as any)?.default_locale || 'id-ID';
    const curr = currency || ($appSettings as any)?.currency_code || 'IDR';
    try {
      return new Intl.NumberFormat(locale, { style: 'currency', currency: curr }).format(amount);
    } catch {
      return `${curr} ${amount.toLocaleString(locale)}`;
    }
  }

  function formatShortAmount(amount: number) {
    if (amount >= 1_000_000) {
      const m = amount / 1_000_000;
      return `Rp ${m % 1 === 0 ? m.toFixed(0) : m.toFixed(1).replace('.', ',')}jt`;
    }
    if (amount >= 1_000) {
      return `Rp ${(amount / 1_000).toFixed(0)}rb`;
    }
    return formatAmount(amount);
  }

  function subDueDate(sub: CustomerSubscriptionView): string | null {
    const inv = pendingInvoices.find(
      (i) =>
        (i as any).subscription_id === sub.id ||
        (i.description || '').includes(sub.package_name || ''),
    );
    if (inv?.due_date) return inv.due_date;
    return sub.grace_until || sub.ends_at;
  }

  function invoiceDueMs(sub: CustomerSubscriptionView): number | null {
    const raw = subDueDate(sub);
    if (!raw) return null;
    const d = new Date(raw).getTime();
    return Number.isFinite(d) ? d : null;
  }

  function subStatusLabel(
    sub: CustomerSubscriptionView,
  ): 'active' | 'grace' | 'overdue' | 'suspended' {
    const st = String(sub.status || '').toLowerCase();
    if (st === 'suspended') return 'suspended';
    const due = invoiceDueMs(sub);
    if (due && due < Date.now()) return 'overdue';
    if (st === 'grace_active') return 'grace';
    const hasPending = pendingInvoices.some(
      (i) =>
        (i as any).subscription_id === sub.id ||
        (i.description || '').includes(sub.package_name || ''),
    );
    if (hasPending) return 'grace';
    return 'active';
  }

  function subStatusText(status: 'active' | 'grace' | 'overdue' | 'suspended') {
    if (status === 'overdue') return 'Terlambat';
    if (status === 'suspended') return 'Suspended';
    if (status === 'grace') return 'Menunggu bayar';
    return 'Aktif';
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

  function subIcon(sub: CustomerSubscriptionView): string {
    const name = (sub.package_name || '').toLowerCase();
    if (name.includes('dedicated') || name.includes('1:1')) return 'lock';
    if (name.includes('business') || name.includes('corporate')) return 'landmark';
    return 'wifi';
  }

  function payFirstPending() {
    if (overdueInvoices[0]) {
      goto(`/pay/${overdueInvoices[0].id}`);
      return;
    }
    if (payableInvoices[0]) {
      goto(`/pay/${payableInvoices[0].id}`);
      return;
    }
    if (pendingInvoices[0]) goto(`/pay/${pendingInvoices[0].id}`);
  }

  function invStatusText(inv: Invoice) {
    const st = String(inv.status || '').toLowerCase();
    if (st === 'verification_pending') return 'Menunggu verifikasi';
    if (st === 'failed') return 'Gagal';
    const d = new Date(inv.due_date || inv.created_at || 0).getTime();
    if (Number.isFinite(d) && d < Date.now()) return 'Terlambat';
    return 'Pending';
  }

  function goServices() {
    goto(`${tenantPrefix}/dashboard/services`);
  }

  function goInvoices() {
    goto(`${tenantPrefix}/dashboard/invoices`);
  }

  function goTickets() {
    goto(`${tenantPrefix}/dashboard/tickets`);
  }

  function goNewTicket() {
    goto(`${tenantPrefix}/support`);
  }
</script>

<div class="dashboard-content fade-in">
  {#if $isAdmin}
    <div
      class="admin-banner"
      onclick={() => goto(`${tenantPrefix}/admin`)}
      onkeydown={(e) => e.key === 'Enter' && goto(`${tenantPrefix}/admin`)}
      role="button"
      tabindex="0"
    >
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

  {#if !$isAdmin}
    <!-- 1. Overdue alert (action-first) -->
    {#if overdueInvoices.length > 0}
      {@const first = overdueInvoices[0]}
      <div class="alert-overdue" role="alert">
        <div class="alert-left">
          <div class="alert-ico"><Icon name="alert-triangle" size={18} /></div>
          <div class="alert-text">
            <strong>
              {overdueInvoices.length} layanan jatuh tempo
            </strong>
            <p>
              {first.description || first.invoice_number}
              · {formatAmount(first.amount, first.currency_code)}
            </p>
          </div>
        </div>
        <button class="btn btn-danger" type="button" onclick={() => goto(`/pay/${first.id}`)}>
          Bayar sekarang
        </button>
      </div>
    {/if}

    <!-- 2. Greeting + primary CTA -->
    <div class="page-head">
      <div class="page-head-text">
        <h1 class="page-greeting">
          {greeting()}, <strong>{$user?.name || ''}</strong>
        </h1>
        <p class="page-sub">
          {#if portalSummaryLoading}
            Memuat ringkasan...
          {:else}
            {activeSubscriptions.length} layanan aktif
            {#if pendingInvoices.length > 0}
              · {pendingInvoices.length} tagihan menunggu
            {/if}
            {#if overdueInvoices.length > 0}
              · {overdueInvoices.length} overdue
            {/if}
          {/if}
        </p>
      </div>
      <div class="head-actions">
        <button class="btn btn-ghost" type="button" onclick={goNewTicket}>Buat tiket</button>
        {#if pendingInvoices.length > 0}
          <button class="btn btn-primary" type="button" onclick={payFirstPending}>
            {#if payableInvoices.length > 0}
              Bayar semua · {formatAmount(payableInvoices.reduce((s, i) => s + (i.amount || 0), 0))}
            {:else}
              Lihat tagihan
            {/if}
          </button>
        {/if}
      </div>
    </div>

    <!-- 3. KPI strip -->
    <div class="kpis">
      <div class="kpi">
        <div class="kpi-label">Layanan aktif</div>
        <div class="kpi-val ok">{portalSummaryLoading ? '—' : activeSubscriptions.length}</div>
        <div class="kpi-sub">
          {#if locationCount > 0}
            {locationCount} lokasi
          {:else}
            Semua paket aktif
          {/if}
        </div>
      </div>
      <div class="kpi">
        <div class="kpi-label">Total tagihan</div>
        <div class="kpi-val {totalPending > 0 ? 'warn' : ''}">
          {portalSummaryLoading ? '—' : totalPending > 0 ? formatShortAmount(totalPending) : 'Rp 0'}
        </div>
        <div class="kpi-sub">
          {pendingInvoices.length > 0
            ? `${pendingInvoices.length} invoice pending`
            : 'Tidak ada tagihan'}
        </div>
      </div>
      <div class="kpi">
        <div class="kpi-label">Jatuh tempo</div>
        <div class="kpi-val {overdueInvoices.length > 0 ? 'bad' : ''}">
          {#if portalSummaryLoading}
            —
          {:else if nearestDueInvoice}
            {formatDate(nearestDueInvoice.due_date || nearestDueInvoice.created_at || Date.now(), {
              timeZone: $appSettings.app_timezone,
            })}
          {:else}
            —
          {/if}
        </div>
        <div class="kpi-sub">{nearestDueSubName || 'Belum ada'}</div>
      </div>
      <div class="kpi">
        <div class="kpi-label">Tiket terbuka</div>
        <div class="kpi-val">{portalSummaryLoading ? '—' : openTicketCount}</div>
        <div class="kpi-sub">
          <button class="kpi-link" type="button" onclick={goTickets}>Lihat tiket</button>
        </div>
      </div>
    </div>
  {:else}
    <section class="hero-card welcome-hero">
      <div class="welcome-body">
        <h1 class="welcome-greeting">
          {greeting()}, <strong>{$user?.name}</strong>
        </h1>
        <p class="welcome-sub">{$t('dashboard.greeting.welcome_message')}</p>
      </div>
    </section>
  {/if}

  <div class="main-grid">
    <div class="col-main">
      <!-- 4. Langganan overdue-first -->
      {#if !$isAdmin}
        <section class="panel">
          <div class="panel-h">
            <h2>{$t('dashboard.portal_summary.active_package') || 'Langganan'}</h2>
            <button class="text-btn" type="button" onclick={goServices}>
              {$t('dashboard.portal_summary.manage_package') || 'Kelola layanan'}
            </button>
          </div>
          <div class="panel-b">
            {#if portalSummaryLoading}
              <div class="loading-state compact">
                <div class="spinner"></div>
                <p class="muted">{$t('common.loading') || 'Loading...'}</p>
              </div>
            {:else if sortedSubscriptions.length === 0}
              <div class="empty-state compact">
                <div class="empty-icon-circle"><Icon name="package" size={28} /></div>
                <h3>Belum ada layanan aktif</h3>
                <p>Pesan paket internet untuk mulai berlangganan.</p>
                <button class="btn btn-primary mt-4" type="button" onclick={goServices}>
                  Lihat layanan
                </button>
              </div>
            {:else}
              <div class="sub-stack">
                {#each sortedSubscriptions as sub (sub.id)}
                  {@const status = subStatusLabel(sub)}
                  {@const due = subDueDate(sub)}
                  <button
                    class="sub-card {status}"
                    type="button"
                    onclick={goServices}
                  >
                    <div class="sub-icon">
                      <Icon name={subIcon(sub)} size={18} />
                    </div>
                    <div class="sub-body">
                      <div class="sub-name-row">
                        <span class="sub-name">{sub.package_name || sub.id}</span>
                        <span class="sub-pill pill-{status}">{subStatusText(status)}</span>
                      </div>
                      {#if sub.location_label}
                        <span class="sub-meta">{sub.location_label}</span>
                      {/if}
                    </div>
                    <div class="sub-price-col">
                      <span class="sub-price">{formatAmount(sub.price, sub.currency_code)}</span>
                      {#if due}
                        <span class="sub-due {status === 'overdue' ? 'text-danger' : ''}">
                          <Icon name="clock" size={11} />
                          Jatuh tempo {formatDate(due, { timeZone: $appSettings.app_timezone })}
                        </span>
                      {/if}
                    </div>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </section>
      {/if}

      <!-- Activity -->
      <section class="panel">
        <div class="panel-h">
          <h2>{$t('dashboard.recent_activity.title')}</h2>
          <button class="text-btn" type="button" onclick={() => openNotificationModal()}>
            {$t('dashboard.recent_activity.view_all')}
          </button>
        </div>
        <div class="panel-b feed-wrap">
          {#if $notificationsLoading && recent.length === 0}
            <div class="loading-state compact">
              <div class="spinner"></div>
              <p class="muted">{$t('common.loading') || 'Loading...'}</p>
            </div>
          {:else if recent.length === 0}
            <div class="empty-state compact">
              <div class="empty-icon-circle"><Icon name="bell" size={28} /></div>
              <h3>{$t('dashboard.recent_activity.empty.title')}</h3>
              <p>{$t('dashboard.recent_activity.empty.description')}</p>
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
                        <div class="activity-msg">{n.message}</div>
                      {/if}
                    </div>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </section>
    </div>

    <aside class="col-side">
      <!-- 5. Billing breakdown -->
      {#if !$isAdmin}
        <section class="panel">
          <div class="panel-h">
            <h2>{$t('dashboard.portal_summary.title') || 'Tagihan'}</h2>
            <button class="text-btn" type="button" onclick={goInvoices}>Detail</button>
          </div>
          <div class="panel-b">
            {#if portalSummaryLoading}
              <div class="summary-loading">
                <div class="spinner"></div>
                <span>{$t('common.loading')}</span>
              </div>
            {:else if pendingInvoices.length > 0}
              <div class="billing-breakdown">
                {#each pendingInvoices as inv (inv.id)}
                  <div class="billing-line">
                    <span class="billing-line-label">
                      {inv.description || inv.invoice_number}
                      <span class="billing-status">{invStatusText(inv)}</span>
                    </span>
                    <span class="billing-line-amount">
                      {formatAmount(inv.amount, inv.currency_code)}
                    </span>
                  </div>
                {/each}
                <div class="billing-line billing-total">
                  <span class="billing-line-label">Total</span>
                  <span class="billing-line-amount text-warning">
                    {formatAmount(totalPending)}
                  </span>
                </div>
              </div>
              {#if nearestDueInvoice}
                <div class="billing-due">
                  <Icon name="clock" size={13} />
                  Terdekat: {formatDate(
                    nearestDueInvoice.due_date || nearestDueInvoice.created_at || Date.now(),
                    { timeZone: $appSettings.app_timezone },
                  )}
                  {#if nearestDueSubName}
                    · {nearestDueSubName}
                  {/if}
                </div>
              {/if}
              <div class="billing-actions">
                <button class="btn btn-ghost btn-sm" type="button" onclick={goInvoices}>
                  Detail invoice
                </button>
                {#if payableInvoices.length > 0}
                  <button class="btn btn-primary btn-sm" type="button" onclick={payFirstPending}>
                    Bayar semua
                  </button>
                {/if}
              </div>
            {:else}
              <div class="billing-body">
                <span class="billing-amount">Rp 0</span>
                <span class="billing-due">
                  <Icon name="check-circle" size={13} />
                  {$t('dashboard.portal_summary.no_pending_invoice') || 'Tidak ada tagihan'}
                </span>
              </div>
              <div class="billing-actions">
                <button class="btn btn-ghost btn-sm" type="button" onclick={goServices}>
                  <Icon name="package" size={13} />
                  Kelola layanan
                </button>
              </div>
            {/if}
          </div>
        </section>
      {/if}

      <!-- Pengumuman -->
      <section class="panel">
        <div class="panel-h">
          <h2>
            {$t('dashboard.announcements.title') || $t('announcements.title') || 'Pengumuman'}
          </h2>
          <button class="text-btn" type="button" onclick={() => goto(`${tenantPrefix}/announcements`)}>
            {$t('dashboard.announcements.view_all') || 'Lihat semua'}
          </button>
        </div>
        <div class="panel-b feed-wrap">
          {#if annLoading && annPosts.length === 0}
            <div class="loading-state compact">
              <div class="spinner"></div>
              <p class="muted">{$t('common.loading')}</p>
            </div>
          {:else if annPosts.length === 0}
            <div class="empty-state compact">
              <div class="empty-icon-circle"><Icon name="megaphone" size={28} /></div>
              <h3>{$t('dashboard.announcements.empty.title') || 'Belum ada pengumuman.'}</h3>
              <p>{$t('dashboard.announcements.empty.description')}</p>
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
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </section>
    </aside>
  </div>
</div>

<style>
  .dashboard-content {
    padding: clamp(1rem, 2.2vw, 1.5rem);
    max-width: 1180px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.15rem;
  }
  @media (max-width: 640px) {
    .dashboard-content {
      padding: 0.85rem;
      gap: 0.95rem;
    }
  }

  /* Overdue alert */
  .alert-overdue {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.85rem 1rem;
    border-radius: 10px;
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-danger) 28%, transparent);
  }
  .alert-left {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    min-width: 0;
  }
  .alert-ico {
    width: 34px;
    height: 34px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 14%, transparent);
    flex-shrink: 0;
  }
  .alert-text strong {
    font-size: 0.88rem;
    display: block;
    color: var(--text-primary);
  }
  .alert-text p {
    font-size: 0.78rem;
    color: var(--text-secondary);
    margin: 0.1rem 0 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  @media (max-width: 560px) {
    .alert-overdue {
      flex-direction: column;
      align-items: stretch;
    }
    .alert-overdue .btn {
      width: 100%;
    }
    .alert-text p {
      white-space: normal;
    }
  }

  /* Page head */
  .page-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .page-greeting {
    font-size: clamp(1.25rem, 2.2vw, 1.45rem);
    font-weight: 750;
    letter-spacing: -0.02em;
    line-height: 1.2;
    margin: 0;
    color: var(--text-primary);
  }
  .page-greeting strong {
    font-weight: 800;
  }
  .page-sub {
    color: var(--text-secondary);
    font-size: 0.88rem;
    margin: 0.25rem 0 0;
  }
  .head-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  @media (max-width: 560px) {
    .page-head {
      align-items: stretch;
    }
    .head-actions {
      width: 100%;
    }
    .head-actions .btn {
      flex: 1;
      min-height: 44px;
    }
  }

  /* KPI */
  .kpis {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.7rem;
  }
  .kpi {
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 10px;
    padding: 0.9rem 1rem;
  }
  .kpi-label {
    font-size: 0.7rem;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-tertiary);
    margin-bottom: 0.35rem;
  }
  .kpi-val {
    font-size: 1.25rem;
    font-weight: 750;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }
  .kpi-val.ok {
    color: var(--color-success);
  }
  .kpi-val.warn {
    color: var(--color-warning);
  }
  .kpi-val.bad {
    color: var(--color-danger);
  }
  .kpi-sub {
    font-size: 0.74rem;
    color: var(--text-secondary);
    margin-top: 0.2rem;
  }
  .kpi-link {
    background: none;
    border: 0;
    padding: 0;
    color: var(--color-primary);
    font-size: 0.74rem;
    font-weight: 650;
    cursor: pointer;
  }
  @media (max-width: 900px) {
    .kpis {
      grid-template-columns: 1fr 1fr;
    }
  }
  @media (max-width: 560px) {
    .kpis {
      grid-template-columns: 1fr 1fr;
      gap: 0.55rem;
    }
    .kpi {
      padding: 0.75rem 0.85rem;
    }
    .kpi-val {
      font-size: 1.1rem;
    }
  }

  /* Admin welcome (compact) */
  .welcome-hero {
    padding: 1.25rem 1.5rem;
  }
  .welcome-greeting {
    font-size: clamp(1.3rem, 2.2vw, 1.7rem);
    font-weight: 750;
    color: var(--text-primary);
    margin: 0 0 0.35rem;
  }
  .welcome-sub {
    color: var(--text-secondary);
    margin: 0;
    font-size: 0.92rem;
  }

  /* Admin banner */
  .admin-banner {
    background: var(--bg-surface);
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: var(--radius-lg);
    padding: 1rem 1.25rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    cursor: pointer;
    transition: all 0.2s;
  }
  .admin-banner:hover {
    border-color: color-mix(in srgb, var(--color-primary) 30%, rgba(255, 255, 255, 0.07));
    transform: translateY(-1px);
  }
  .banner-content {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .banner-icon {
    width: 38px;
    height: 38px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--color-primary) 18%, transparent);
    color: var(--color-primary);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .banner-content h3 {
    margin: 0 0 0.1rem;
    font-size: 1rem;
    font-weight: 650;
  }
  .banner-content p {
    margin: 0;
    font-size: 0.82rem;
    color: var(--text-secondary);
  }

  /* Grid */
  .main-grid {
    display: grid;
    grid-template-columns: 1.55fr 0.95fr;
    gap: 1rem;
    align-items: start;
  }
  .col-main,
  .col-side {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    min-width: 0;
  }
  @media (max-width: 900px) {
    .main-grid {
      grid-template-columns: 1fr;
    }
  }

  /* Panel */
  .panel {
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }
  .panel-h {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.85rem 1rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    gap: 0.5rem;
  }
  .panel-h h2 {
    font-size: 0.9rem;
    font-weight: 700;
    margin: 0;
    color: var(--text-primary);
  }
  .panel-b {
    padding: 0.75rem;
  }
  .feed-wrap {
    padding: 0;
  }
  .text-btn {
    background: transparent;
    border: none;
    color: var(--color-primary);
    font-size: 0.78rem;
    font-weight: 650;
    cursor: pointer;
    text-decoration: none;
    white-space: nowrap;
  }

  /* Subscriptions */
  .sub-stack {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  .sub-card {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 0.85rem;
    padding: 0.8rem 0.9rem;
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.12s;
    text-align: left;
    color: inherit;
    width: 100%;
  }
  .sub-card:hover {
    border-color: color-mix(in srgb, var(--color-primary) 28%, rgba(255, 255, 255, 0.06));
    background: rgba(255, 255, 255, 0.03);
  }
  .sub-card.overdue {
    border-color: color-mix(in srgb, var(--color-danger) 35%, rgba(255, 255, 255, 0.06));
  }
  .sub-icon {
    width: 40px;
    height: 40px;
    border-radius: 9px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--color-primary) 10%, transparent);
    color: var(--color-primary);
    flex-shrink: 0;
  }
  .sub-body {
    min-width: 0;
  }
  .sub-name-row {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
    margin-bottom: 0.15rem;
  }
  .sub-name {
    font-weight: 700;
    font-size: 0.9rem;
  }
  .sub-meta {
    font-size: 0.74rem;
    color: var(--text-tertiary);
  }
  .sub-price-col {
    text-align: right;
    flex-shrink: 0;
  }
  .sub-price {
    font-weight: 750;
    font-size: 0.9rem;
  }
  .sub-due {
    font-size: 0.72rem;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 0.25rem;
    justify-content: flex-end;
    margin-top: 0.15rem;
  }
  .sub-due.text-danger {
    color: var(--color-danger);
    font-weight: 650;
  }

  .sub-pill {
    display: inline-flex;
    align-items: center;
    font-size: 0.66rem;
    font-weight: 750;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    padding: 0.12rem 0.45rem;
    border-radius: 999px;
    border: 1px solid transparent;
  }
  .pill-active {
    color: var(--color-success);
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-success) 28%, transparent);
  }
  .pill-grace {
    color: var(--color-warning);
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-warning) 28%, transparent);
  }
  .pill-overdue {
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-danger) 28%, transparent);
  }
  .pill-suspended {
    color: var(--color-warning);
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-warning) 28%, transparent);
  }
  .sub-card.suspended {
    border-color: color-mix(in srgb, var(--color-warning) 35%, rgba(255, 255, 255, 0.06));
  }

  @media (max-width: 640px) {
    .sub-card {
      grid-template-columns: auto 1fr;
    }
    .sub-price-col {
      grid-column: 1 / -1;
      text-align: left;
      padding-left: 3.05rem;
    }
    .sub-due {
      justify-content: flex-start;
    }
  }

  /* Empty / loading */
  .empty-state {
    text-align: center;
    padding: 1.5rem 1rem;
    max-width: 320px;
    margin: 0 auto;
  }
  .empty-state.compact {
    padding: 1.25rem 0.75rem;
  }
  .empty-state h3 {
    font-size: 0.95rem;
    font-weight: 650;
    margin: 0.6rem 0 0.35rem;
    color: var(--text-primary);
  }
  .empty-state p {
    color: var(--text-secondary);
    font-size: 0.82rem;
    line-height: 1.5;
    margin: 0;
  }
  .empty-icon-circle {
    width: 48px;
    height: 48px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.04);
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0 auto;
    color: var(--text-secondary);
    opacity: 0.55;
  }
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 140px;
    gap: 0.6rem;
    padding: 1.5rem;
  }
  .loading-state.compact {
    min-height: 100px;
  }
  .muted {
    color: var(--text-secondary);
    margin: 0;
    font-size: 0.84rem;
  }
  .spinner {
    width: 18px;
    height: 18px;
    border: 2px solid rgba(255, 255, 255, 0.08);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* Activity */
  .activity-list {
    display: flex;
    flex-direction: column;
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .activity-li {
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }
  .activity-li:last-child {
    border-bottom: 0;
  }
  .activity-item {
    width: 100%;
    background: transparent;
    border: 0;
    padding: 0.75rem 0.85rem;
    display: flex;
    gap: 0.7rem;
    align-items: flex-start;
    text-align: left;
    cursor: pointer;
    transition: background 0.12s;
    color: inherit;
  }
  .activity-item:hover {
    background: rgba(255, 255, 255, 0.02);
  }
  .activity-icon {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.02);
    color: var(--text-secondary);
    flex-shrink: 0;
    margin-top: 1px;
  }
  .activity-icon.success {
    color: var(--color-success);
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-success) 24%, rgba(255, 255, 255, 0.08));
  }
  .activity-icon.warning {
    color: var(--color-warning);
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-warning) 24%, rgba(255, 255, 255, 0.08));
  }
  .activity-icon.error {
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-danger) 24%, rgba(255, 255, 255, 0.08));
  }
  .activity-text {
    flex: 1;
    min-width: 0;
  }
  .activity-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.6rem;
  }
  .activity-title {
    font-weight: 650;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 0.86rem;
  }
  .activity-time {
    font-size: 0.72rem;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }
  .activity-msg {
    margin-top: 0.15rem;
    font-size: 0.8rem;
    color: var(--text-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    line-height: 1.4;
  }

  /* Announcements */
  .ann-list {
    display: flex;
    flex-direction: column;
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .ann-li {
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }
  .ann-li:last-child {
    border-bottom: 0;
  }
  .ann-item {
    width: 100%;
    background: transparent;
    border: 0;
    padding: 0.75rem 0.85rem;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.7rem;
    align-items: start;
    text-align: left;
    cursor: pointer;
    transition: background 0.12s;
    color: inherit;
  }
  .ann-item:hover {
    background: rgba(255, 255, 255, 0.02);
  }
  .ann-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    margin-top: 0.4rem;
    background: var(--text-tertiary);
  }
  .ann-dot.info {
    background: var(--color-primary);
  }
  .ann-dot.success {
    background: var(--color-success);
  }
  .ann-dot.warning {
    background: var(--color-warning);
  }
  .ann-dot.error {
    background: var(--color-danger);
  }
  .ann-text {
    min-width: 0;
  }
  .ann-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .ann-title {
    font-weight: 650;
    color: var(--text-primary);
    font-size: 0.86rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ann-time {
    font-size: 0.72rem;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }
  .ann-body {
    margin-top: 0.15rem;
    font-size: 0.8rem;
    color: var(--text-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    line-height: 1.4;
  }

  @media (max-width: 560px) {
    .ann-row,
    .activity-row {
      flex-direction: column;
      gap: 0.15rem;
    }
    .ann-title,
    .activity-title {
      white-space: normal;
    }
  }

  /* Billing */
  .billing-breakdown {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    margin-bottom: 0.55rem;
  }
  .billing-line {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.82rem;
  }
  .billing-line-label {
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 65%;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .billing-status {
    font-size: 0.68rem;
    font-weight: 650;
    color: var(--color-warning);
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }
  .billing-line-amount {
    font-weight: 650;
    flex-shrink: 0;
  }
  .billing-line-amount.text-warning {
    color: var(--color-warning);
  }
  .billing-total {
    border-top: 1px solid rgba(255, 255, 255, 0.07);
    padding-top: 0.65rem;
    margin-top: 0.35rem;
    font-weight: 750;
    font-size: 0.88rem;
  }
  .billing-total .billing-line-amount {
    font-size: 1.1rem;
    font-weight: 750;
  }
  .billing-due {
    font-size: 0.74rem;
    color: var(--text-tertiary);
    display: flex;
    align-items: center;
    gap: 0.35rem;
    margin: 0.55rem 0 0.8rem;
    flex-wrap: wrap;
  }
  .billing-body {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin-bottom: 0.85rem;
  }
  .billing-amount {
    font-size: 1.5rem;
    font-weight: 750;
  }
  .billing-actions {
    display: flex;
    gap: 0.45rem;
    flex-wrap: wrap;
  }
  .summary-loading {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.84rem;
    padding: 1rem 0;
  }

  /* Buttons */
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.35rem;
    padding: 0.5rem 0.85rem;
    border-radius: 8px;
    font-size: 0.8rem;
    font-weight: 650;
    cursor: pointer;
    transition: all 0.15s;
    border: none;
    white-space: nowrap;
  }
  .btn-primary {
    background: var(--color-primary);
    color: #fff;
  }
  .btn-primary:hover {
    filter: brightness(1.12);
  }
  .btn-danger {
    background: var(--color-danger);
    color: #fff;
  }
  .btn-danger:hover {
    filter: brightness(1.1);
  }
  .btn-ghost {
    background: transparent;
    color: var(--text-primary);
    border: 1px solid rgba(255, 255, 255, 0.12);
  }
  .btn-ghost:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .btn-sm {
    padding: 0.4rem 0.7rem;
    font-size: 0.76rem;
  }
  .mt-4 {
    margin-top: 1rem;
  }

  @media (max-width: 560px) {
    .billing-actions {
      flex-direction: column;
    }
    .billing-actions .btn {
      width: 100%;
      min-height: 42px;
    }
  }
</style>
