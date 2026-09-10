<script lang="ts">
  /*
    Dashboard portal v2 — gelombang 25b.
    Versi lama: (app)/dashboard/+page.svelte (1.505 baris).
    Perilaku identik: alert overdue + greeting + KPI + langganan +
    aktivitas + tagihan + pengumuman.
    Pola DS: PortalShell + PageHeader + StatTile + Card + Badge + Button.
  */
  import { onMount } from 'svelte';
  import { user, isAdmin } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { getDashboardRecentNotifications } from '$lib/utils/dashboardNotifications';
  import { hasInternalAppAccess } from '$lib/utils/appLanding';
  import { openNotificationModal } from '$lib/stores/notificationModal';
  import {
    getAnnouncementDetailPath,
    resolveAnnouncementActionUrl,
  } from '$lib/utils/announcementRouting';
  import { formatDate, timeAgo } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import {
    api,
    type Announcement,
    type CustomerSubscriptionView,
    type Invoice,
    type PaginatedResponse,
  } from '$lib/api/client';
  import {
    notifications,
    loading as notificationsLoading,
    loadNotifications,
  } from '$lib/stores/notifications';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Icon from '$lib/components/ds/Icon.svelte';
  import type { StatusTone } from '$lib/components/ds/tokens';

  onMount(() => {
    void loadNotifications(1);
    void loadDashboardAnnouncements();
    if (!$isAdmin) void loadPortalSummary();
  });

  function greeting() {
    const h = new Date().getHours();
    if (h < 12) return 'Selamat pagi';
    if (h < 17) return 'Selamat siang';
    return 'Selamat malam';
  }

  let portalInvoiceIds = $state<string[]>([]);
  let recent = $derived(
    getDashboardRecentNotifications($notifications, hasInternalAppAccess($user), 6, portalInvoiceIds),
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
      if (st === 'verification_pending') return false;
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
      (s) => (inv as any).subscription_id === s.id || (inv.description || '').includes(s.package_name || ''),
    );
    return match?.package_name || inv.description || inv.invoice_number || '';
  });
  let locationCount = $derived.by(() => {
    const labels = new Set(activeSubscriptions.map((s) => s.location_label || s.location_id).filter(Boolean));
    return labels.size;
  });

  async function loadDashboardAnnouncements() {
    annLoading = true;
    try {
      const res: PaginatedResponse<Announcement> = await api.announcements.listRecent({ page: 1, per_page: 3 });
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
      activeSubscriptions = (subRes?.data || []).filter((s) => {
        const st = String(s.status || '').toLowerCase();
        return st !== 'cancelled' && st !== 'ended' && st !== 'terminated';
      });
      portalInvoiceIds = (invoiceRows || []).map((inv) => inv.id).filter(Boolean);
      pendingInvoices = (invoiceRows || []).filter((inv) => {
        const st = String(inv.status || '').toLowerCase();
        return st === 'pending' || st === 'verification_pending' || st === 'failed';
      });
      totalPending = pendingInvoices.reduce((sum, inv) => sum + (inv.amount || 0), 0);
      const tickets = ticketRes?.data || [];
      openTicketCount = tickets.filter((t: any) => t.status === 'open' || t.status === 'pending').length;
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
    if (amount >= 1_000) return `Rp ${(amount / 1_000).toFixed(0)}rb`;
    return formatAmount(amount);
  }
  function subDueDate(sub: CustomerSubscriptionView): string | null {
    const inv = pendingInvoices.find(
      (i) => (i as any).subscription_id === sub.id || (i.description || '').includes(sub.package_name || ''),
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
  function subStatusLabel(sub: CustomerSubscriptionView): 'active' | 'grace' | 'overdue' | 'suspended' {
    const st = String(sub.status || '').toLowerCase();
    if (st === 'suspended') return 'suspended';
    const due = invoiceDueMs(sub);
    if (due && due < Date.now()) return 'overdue';
    if (st === 'grace_active') return 'grace';
    const hasPending = pendingInvoices.some(
      (i) => (i as any).subscription_id === sub.id || (i.description || '').includes(sub.package_name || ''),
    );
    if (hasPending) return 'grace';
    return 'active';
  }
  function subStatusTone(status: 'active' | 'grace' | 'overdue' | 'suspended'): StatusTone {
    if (status === 'overdue' || status === 'suspended') return 'negative';
    if (status === 'grace') return 'warning';
    return 'positive';
  }
  function subStatusText(status: 'active' | 'grace' | 'overdue' | 'suspended') {
    if (status === 'overdue') return 'Terlambat';
    if (status === 'suspended') return 'Ditangguhkan';
    if (status === 'grace') return 'Menunggu bayar';
    return 'Aktif';
  }
  function openAnnouncement(id: string) {
    goto(`/v2/announcements/${id}`);
  }
  function openNotification(n: any) {
    if (n?.action_url) goto(n.action_url);
    else openNotificationModal();
  }
  function iconForType(type: string) {
    if (type === 'success') return 'check';
    if (type === 'warning') return 'alert';
    if (type === 'error') return 'alert';
    return 'bell';
  }
  function payFirstPending() {
    if (overdueInvoices[0]) return void goto(`/pay/${overdueInvoices[0].id}`);
    if (payableInvoices[0]) return void goto(`/pay/${payableInvoices[0].id}`);
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
    goto('/v2/dashboard/services');
  }
  function goInvoices() {
    goto('/v2/dashboard/invoices');
  }
  function goTickets() {
    goto('/v2/dashboard/tickets');
  }
  function goNewTicket() {
    goto('/v2/support');
  }
</script>
<PortalShell title={ $t('dashboard.v2.home') }>
  {#if $isAdmin}
    <button
      type="button"
      class="admin-banner"
      onclick={() => goto('/v2/admin')}
      onkeydown={(e) => e.key === 'Enter' && goto('/v2/admin')}
    >
      <Icon name="shield" size={20} />
      <span class="flex-1 text-left">
        <span class="block text-sm font-semibold">{ $t('dashboard.v2.admin_mode1') }</span>
        <span class="block text-xs text-ink-500">{ $t('dashboard.v2.admin_mode2') }</span>
      </span>
      <Icon name="chevronRight" size={16} />
    </button>
  {/if}

  {#if !$isAdmin}
    {#if overdueInvoices.length > 0}
      {@const first = overdueInvoices[0]}
      <div class="alert-overdue" role="alert">
        <div class="flex items-center gap-3">
          <Icon name="alert" size={20} />
          <div>
            <p class="text-sm font-semibold">{ $t('dashboard.v2.overdue_services', { values: { n: overdueInvoices.length } }) }</p>
            <p class="text-xs text-ink-300">
              {first.description || first.invoice_number} · {formatAmount(first.amount, first.currency_code)}
            </p>
          </div>
        </div>
        <Button variant="danger" size="sm" onclick={() => goto(`/pay/${first.id}`)}>
          Bayar sekarang
        </Button>
      </div>
    {/if}
  {/if}

  <PageHeader
    title={greeting() + ', ' + ($user?.name || '')}
    desc={portalSummaryLoading
      ? $t('dashboard.v2.sum_loading')
      : $t('dashboard.v2.sum_active', { values: { n: activeSubscriptions.length } }) +
        (pendingInvoices.length > 0 ? ' · ' + $t('dashboard.v2.sum_pending', { values: { n: pendingInvoices.length } }) : '') +
        (overdueInvoices.length > 0 ? ' · ' + $t('dashboard.v2.sum_overdue', { values: { n: overdueInvoices.length } }) : '')}
  >
    {#snippet actions()}
      <Button variant="ghost" icon="plus" onclick={goNewTicket}>{ $t('dashboard.v2.new_ticket') }</Button>
      {#if pendingInvoices.length > 0}
        <Button icon="card" onclick={payFirstPending}>
          {#if payableInvoices.length > 0}
            Bayar semua · {formatShortAmount(payableInvoices.reduce((s, i) => s + (i.amount || 0), 0))}
          {:else}
            Lihat tagihan
          {/if}
        </Button>
      {/if}
    {/snippet}
  </PageHeader>

  {#if !$isAdmin}
    <div class="mb-6 grid grid-cols-2 gap-3 lg:grid-cols-4">
      <StatTile
        label={ $t('dashboard.v2.st_services') }
        value={portalSummaryLoading ? '—' : String(activeSubscriptions.length)}
        hint={locationCount > 0 ? $t('dashboard.v2.h_locations', { values: { n: locationCount } }) : $t('dashboard.v2.all_pkgs')}
      />
      <StatTile
        label={ $t('dashboard.v2.st_bills') }
        value={portalSummaryLoading ? '—' : totalPending > 0 ? formatShortAmount(totalPending) : 'Rp 0'}
        hint={pendingInvoices.length > 0 ? $t('dashboard.v2.h_inv_wait', { values: { n: pendingInvoices.length } }) : $t('dashboard.v2.h_no_bills')}
        tone={totalPending > 0 ? 'warning' : 'neutral'}
      />
      <StatTile
        label={ $t('dashboard.v2.st_due') }
        value={portalSummaryLoading
          ? '—'
          : nearestDueInvoice
            ? formatDate(nearestDueInvoice.due_date || nearestDueInvoice.created_at || Date.now(), {
                timeZone: $appSettings.app_timezone,
              })
            : '—'}
        hint={nearestDueSubName || $t('dashboard.v2.h_none')}
        tone={overdueInvoices.length > 0 ? 'negative' : 'neutral'}
      />
      <StatTile
        label={ $t('dashboard.v2.st_tickets') }
        value={portalSummaryLoading ? '—' : String(openTicketCount)}
        hint={ $t('dashboard.v2.h_help') }
      />
    </div>
  {/if}

  <div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_360px]">
    <div class="flex flex-col gap-4">
      {#if !$isAdmin}
        <Card title={ $t('dashboard.v2.subscriptions') }
          padded={false}
        >
          {#snippet aside()}
            <Button variant="ghost" size="sm" icon="chevronRight" onclick={goServices}>
              Kelola layanan
            </Button>
          {/snippet}
          <div class="p-4">
            {#if portalSummaryLoading}
              <p class="text-sm text-ink-500">Memuat…</p>
            {:else if sortedSubscriptions.length === 0}
              <div class="flex flex-col items-start gap-3 py-4">
                <div>
                  <p class="text-sm font-medium">{ $t('dashboard.v2.no_subs') }</p>
                  <p class="text-sm text-ink-500">{ $t('dashboard.v2.no_subs_hint') }</p>
                </div>
                <Button icon="box" onclick={goServices}>{ $t('dashboard.v2.view_services') }</Button>
              </div>
            {:else}
              <div class="flex flex-col gap-2">
                {#each sortedSubscriptions as sub (sub.id)}
                  {@const status = subStatusLabel(sub)}
                  {@const due = subDueDate(sub)}
                  <button type="button" class="sub-card" onclick={goServices}>
                    <div class="min-w-0 flex-1">
                      <div class="flex flex-wrap items-center gap-2">
                        <span class="truncate text-sm font-medium">{sub.package_name || sub.id}</span>
                        <Badge tone={subStatusTone(status)} label={subStatusText(status)} />
                      </div>
                      {#if sub.location_label}
                        <span class="mt-0.5 block truncate text-xs text-ink-400">{sub.location_label}</span>
                      {/if}
                    </div>
                    <div class="shrink-0 text-right">
                      <span class="block text-sm font-semibold tabular-nums">
                        {formatAmount(sub.price, sub.currency_code)}
                      </span>
                      {#if due}
                        <span class="mt-0.5 flex items-center gap-1 text-xs text-ink-400 justify-end">
                          <Icon name="clock" size={12} />
                          {formatDate(due, { timeZone: $appSettings.app_timezone })}
                        </span>
                      {/if}
                    </div>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </Card>
      {/if}

      <Card title={ $t('dashboard.v2.activity') } padded={false}>
        {#snippet aside()}
          <Button variant="ghost" size="sm" icon="chevronRight" onclick={() => openNotificationModal()}>
            Semua
          </Button>
        {/snippet}
        <div class="p-2">
          {#if $notificationsLoading && recent.length === 0}
            <p class="p-3 text-sm text-ink-500">Memuat…</p>
          {:else if recent.length === 0}
            <div class="flex flex-col items-start gap-2 p-4">
              <Icon name="bell" size={24} />
              <p class="text-sm font-medium">{ $t('dashboard.v2.no_activity') }</p>
              <p class="text-sm text-ink-500">{ $t('dashboard.v2.no_activity_hint') }</p>
            </div>
          {:else}
            <ul class="flex flex-col">
              {#each recent as n (n.id)}
                <li>
                  <button type="button" class="activity-item" onclick={() => openNotification(n)}>
                    <span class="activity-ico" class:type={n.notification_type}>
                      <Icon name={iconForType(n.notification_type)} size={15} />
                    </span>
                    <span class="min-w-0 flex-1">
                      <span class="flex items-baseline justify-between gap-3">
                        <span class="truncate text-sm">{n.title}</span>
                        <span class="shrink-0 text-xs text-ink-400">{timeAgo(n.created_at, (k) => $t(k))}</span>
                      </span>
                      {#if n.message}
                        <span class="block truncate text-xs text-ink-500">{n.message}</span>
                      {/if}
                    </span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </Card>
    </div>
    <aside class="flex flex-col gap-4">
      {#if !$isAdmin}
        <Card title={ $t('dashboard.v2.billing') }
          padded={false}
        >
          {#snippet aside()}
            <Button variant="ghost" size="sm" icon="chevronRight" onclick={goInvoices}>Detail</Button>
          {/snippet}
          <div class="p-4">
            {#if portalSummaryLoading}
              <p class="text-sm text-ink-500">Memuat…</p>
            {:else if pendingInvoices.length > 0}
              <div class="flex flex-col gap-2.5">
                {#each pendingInvoices as inv (inv.id)}
                  <div class="flex items-baseline justify-between gap-3">
                    <span class="min-w-0">
                      <span class="block truncate text-sm">{inv.description || inv.invoice_number}</span>
                      <span class="text-xs text-ink-400">{invStatusText(inv)}</span>
                    </span>
                    <span class="shrink-0 text-sm font-medium tabular-nums">
                      {formatAmount(inv.amount, inv.currency_code)}
                    </span>
                  </div>
                {/each}
                <div class="mt-1 flex items-baseline justify-between border-t border-ink-200 pt-2.5">
                  <span class="text-sm font-medium">Total</span>
                  <span class="text-sm font-semibold tabular-nums text-amber-600">{formatAmount(totalPending)}</span>
                </div>
              </div>
              {#if nearestDueInvoice}
                <p class="mt-3 flex items-center gap-1.5 text-xs text-ink-500">
                  <Icon name="clock" size={13} />
                  Terdekat:
                  {formatDate(nearestDueInvoice.due_date || nearestDueInvoice.created_at || Date.now(), {
                    timeZone: $appSettings.app_timezone,
                  })}
                  {#if nearestDueSubName} · {nearestDueSubName}{/if}
                </p>
              {/if}
              <div class="mt-4 flex gap-2">
                <Button variant="ghost" size="sm" onclick={goInvoices}>{ $t('dashboard.v2.inv_detail') }</Button>
                {#if payableInvoices.length > 0}
                  <Button size="sm" icon="card" onclick={payFirstPending}>{ $t('dashboard.v2.pay_all') }</Button>
                {/if}
              </div>
            {:else}
              <div class="flex items-center gap-2 py-2">
                <span class="text-lg font-semibold">Rp 0</span>
                <span class="flex items-center gap-1 text-xs text-emerald-600">
                  <Icon name="check" size={13} />
                  { $t('dashboard.v2.h_no_bills') }
                </span>
              </div>
              <Button variant="ghost" size="sm" icon="box" class="mt-3" onclick={goServices}>
                Kelola layanan
              </Button>
            {/if}
          </div>
        </Card>
      {/if}

      <Card title={ $t('dashboard.v2.announcements') }
        padded={false}
      >
        {#snippet aside()}
          <Button variant="ghost" size="sm" icon="chevronRight" onclick={() => goto('/v2/announcements')}>
            Semua
          </Button>
        {/snippet}
        <div class="p-2">
          {#if annLoading && annPosts.length === 0}
            <p class="p-3 text-sm text-ink-500">Memuat…</p>
          {:else if annPosts.length === 0}
            <p class="p-4 text-sm text-ink-500">{ $t('dashboard.v2.no_ann') }</p>
          {:else}
            <ul class="flex flex-col">
              {#each annPosts as a (a.id)}
                <li>
                  <button type="button" class="activity-item" onclick={() => openAnnouncement(a.id)}>
                    <span class="activity-ico">
                      <Icon name="megaphone" size={15} />
                    </span>
                    <span class="min-w-0 flex-1">
                      <span class="flex items-baseline justify-between gap-3">
                        <span class="truncate text-sm">{a.title}</span>
                        <span class="shrink-0 text-xs text-ink-400">
                          {formatDate(a.starts_at, { timeZone: $appSettings.app_timezone })}
                        </span>
                      </span>
                    </span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </Card>
    </aside>
  </div>
</PortalShell>

<style>
  .admin-banner {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    border-radius: 0.75rem;
    border: 1px solid #1c1917;
    background: #1c1917;
    color: #fff;
    padding: 0.9rem 1.1rem;
    margin-bottom: 1.25rem;
    cursor: pointer;
  }
  .admin-banner:hover {
    background: #292524;
  }
  .alert-overdue {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
    border-radius: 0.75rem;
    border: 1px solid #fecaca;
    background: #fef2f2;
    color: #991b1b;
    padding: 0.85rem 1.1rem;
    margin-bottom: 1.25rem;
  }
  .sub-card {
    display: flex;
    align-items: center;
    gap: 1rem;
    width: 100%;
    border-radius: 0.65rem;
    border: 1px solid var(--ink-200, #e7e5e4);
    background: #fff;
    padding: 0.7rem 0.9rem;
    cursor: pointer;
    text-align: left;
  }
  .sub-card:hover {
    border-color: #d6d3d1;
    background: #fafaf9;
  }
  .activity-item {
    display: flex;
    align-items: flex-start;
    gap: 0.7rem;
    width: 100%;
    border: none;
    background: transparent;
    padding: 0.6rem 0.75rem;
    border-radius: 0.6rem;
    cursor: pointer;
    text-align: left;
  }
  .activity-item:hover {
    background: var(--ink-50, #fafaf9);
  }
  .activity-ico {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 9999px;
    background: var(--ink-100, #f5f5f4);
    color: var(--ink-500, #78716c);
    flex-shrink: 0;
  }
  .activity-ico.success {
    background: #dcfce7;
    color: #15803d;
  }
  .activity-ico.warning {
    background: #fef3c7;
    color: #b45309;
  }
  .activity-ico.error {
    background: #fee2e2;
    color: #b91c1c;
  }
</style>
