<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import {
    api,
    type CustomerSubscriptionView,
    type CustomerPortalInstallationTrackerResponse,
    type Invoice,
    type InstallationWorkOrderView,
    type WorkOrderRescheduleRequestView,
  } from '$lib/api/client';
  import { appSettings } from '$lib/stores/settings';
  import { toast } from '$lib/stores/toast';

  type StatusFilter = 'all' | 'active' | 'pending_installation' | 'needs_attention';
  type SortBy = 'updated_at' | 'price' | 'status' | 'package_name' | 'location_label';

  let loading = $state(true);
  let reopeningId = $state<string | null>(null);
  let subscriptions = $state<CustomerSubscriptionView[]>([]);
  let totalCount = $state(0);
  let page = $state(0);
  let pageSize = $state(10);
  let loadError = $state('');
  let statusFilter = $state<StatusFilter>('all');
  let sortBy = $state<SortBy>('updated_at');
  let sortDirection = $state<'asc' | 'desc'>('desc');
  let stats = $state({
    total: 0,
    active: 0,
    pendingInstallation: 0,
    needsAttention: 0,
  });
  let trackerOpen = $state(false);
  let trackerLoading = $state(false);
  let trackerSub = $state<CustomerSubscriptionView | null>(null);
  let trackerWo = $state<InstallationWorkOrderView | null>(null);
  let trackerReschedule = $state<WorkOrderRescheduleRequestView | null>(null);
  let trackerInvoice = $state<Invoice | null>(null);
  let trackerError = $state('');
  let rescheduleAt = $state('');
  let rescheduleReason = $state('');
  let rescheduleBusy = $state(false);

  onMount(() => {
    void refreshAll();
  });

  function tt(key: string, fallback: string) {
    const value = get(t)(key);
    return value && value !== key ? value : fallback;
  }

  function mapStatusFilter(filter: StatusFilter):
    | 'active'
    | 'pending_installation'
    | 'suspended'
    | 'cancelled'
    | 'needs_attention'
    | undefined {
    if (filter === 'all') return undefined;
    if (filter === 'active') return 'active';
    if (filter === 'pending_installation') return 'pending_installation';
    if (filter === 'needs_attention') return 'needs_attention';
    return undefined;
  }

  async function loadData() {
    loading = true;
    loadError = '';
    try {
      const mySubscriptions = await api.customers.portal.mySubscriptions({
        page: page + 1,
        per_page: pageSize,
        status: mapStatusFilter(statusFilter),
        sort_by: sortBy,
        sort_dir: sortDirection,
      });
      subscriptions = mySubscriptions?.data || [];
      totalCount = Number(mySubscriptions?.total || 0);
      if (subscriptions.length === 0 && totalCount > 0 && page > 0) {
        page = 0;
        await loadData();
        return;
      }
    } catch (e: any) {
      loadError = e?.message || String(e);
      toast.error(
        get(t)('dashboard.services_portal.toasts.load_failed') || 'Failed to load services',
      );
    } finally {
      loading = false;
    }
  }

  async function loadStats() {
    try {
      const statsRes = await api.customers.portal.mySubscriptionStats();
      stats = {
        total: Number(statsRes?.total || 0),
        active: Number(statsRes?.active || 0),
        pendingInstallation: Number(statsRes?.pending_installation || 0),
        needsAttention: Number(statsRes?.needs_attention || 0),
      };
    } catch {
      // Keep existing stats if count endpoint fails
    }
  }

  const filteredSubscriptions = $derived.by(() => subscriptions);

  function formatCurrency(amount: number, currencyCode?: string | null) {
    const currency = currencyCode || (($appSettings as any)?.currency_code || 'IDR');
    const locale = ($appSettings as any)?.default_locale || 'id-ID';
    try {
      return new Intl.NumberFormat(locale, { style: 'currency', currency }).format(amount || 0);
    } catch {
      return `${currency} ${Number(amount || 0).toLocaleString(locale)}`;
    }
  }

  function formatDate(value?: string | null) {
    if (!value) return '-';
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return '-';
    const locale = ($appSettings as any)?.default_locale || 'id-ID';
    return new Intl.DateTimeFormat(locale, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }).format(date);
  }

  function billingCycleLabel(cycle?: string | null) {
    const normalized = String(cycle || '').toLowerCase();
    if (normalized === 'yearly') return get(t)('dashboard.packages.cycles.yearly') || 'Yearly';
    return get(t)('dashboard.packages.cycles.monthly') || 'Monthly';
  }

  function serviceStatusMeta(
    status?: string | null,
    startsAt?: string | null,
    canRequestReopen = false,
  ) {
    if (canRequestReopen) {
      return {
        label: tt('dashboard.services_portal.status.cancelled', 'Cancelled'),
        tone: 'cancelled',
        hint: tt(
          'dashboard.services_portal.status_hint.cancelled_reopen',
          'Request was cancelled. You can submit reopen request.',
        ),
      };
    }

    const s = String(status || '').toLowerCase();
    if (s === 'active') {
      return {
        label: get(t)('common.active') || 'Active',
        tone: 'active',
        hint: tt('dashboard.services_portal.status_hint.active', 'Service is active and running.'),
      };
    }
    if (s === 'pending_installation') {
      return {
        label: tt(
          'dashboard.services_portal.status.pending_installation',
          'Pending Installation',
        ),
        tone: 'pending',
        hint: tt(
          'dashboard.services_portal.status_hint.pending_installation',
          'Waiting assignment and installation scheduling.',
        ),
      };
    }
    if (s === 'suspended') {
      const awaitingPayment = !startsAt;
      return {
        label: awaitingPayment
          ? tt('dashboard.services_portal.status.awaiting_payment', 'Awaiting Payment')
          : tt('dashboard.services_portal.status.suspended', 'Suspended'),
        tone: 'suspended',
        hint: awaitingPayment
          ? tt(
              'dashboard.services_portal.status_hint.awaiting_payment',
              'Installation is complete. Waiting for first invoice/payment confirmation.',
            )
          : tt(
              'dashboard.services_portal.status_hint.suspended',
              'Service is suspended. Please check billing or contact support.',
            ),
      };
    }
    if (s === 'cancelled') {
      return {
        label: tt('dashboard.services_portal.status.cancelled', 'Cancelled'),
        tone: 'cancelled',
        hint: tt('dashboard.services_portal.status_hint.cancelled', 'Request was cancelled.'),
      };
    }

    return {
      label: status || '-',
      tone: 'default',
      hint: '',
    };
  }

  function invoiceActionLabel(status?: string | null, startsAt?: string | null) {
    const normalized = String(status || '').toLowerCase();
    if (normalized === 'suspended' && !startsAt) {
      return tt('dashboard.services_portal.actions.pay_invoice', 'Pay Invoice');
    }
    return tt('dashboard.services_portal.actions.view_invoices', 'View Invoices');
  }

  function setStatusFilter(filter: StatusFilter) {
    statusFilter = filter;
    page = 0;
    void loadData();
  }

  function clearFilter() {
    statusFilter = 'all';
    page = 0;
    void loadData();
  }

  function handleTableSort(key: string) {
    const allowedSortKeys: SortBy[] = ['updated_at', 'price', 'status', 'package_name', 'location_label'];
    if (!allowedSortKeys.includes(key as SortBy)) return;
    const typedKey = key as SortBy;

    if (sortBy === typedKey) {
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
      page = 0;
      void loadData();
      return;
    }

    sortBy = typedKey;
    sortDirection = typedKey === 'updated_at' || typedKey === 'price' ? 'desc' : 'asc';
    page = 0;
    void loadData();
  }

  async function refreshAll() {
    await Promise.all([loadStats(), loadData()]);
  }

  async function requestReopen(sub: CustomerSubscriptionView) {
    if (!sub?.id || reopeningId) return;
    reopeningId = sub.id;
    try {
      const res = await api.customers.portal.reopenOrderRequest(sub.id);
      const woId = res?.work_order?.id;
      toast.success(
        woId
          ? tt(
              'dashboard.services_portal.toasts.reopen_success_with_id',
              `Installation request reopened (WO ${woId})`,
            ).replace('{id}', woId)
          : tt(
              'dashboard.services_portal.toasts.reopen_success',
              'Installation request reopened',
            ),
      );
      await refreshAll();
    } catch (e: any) {
      toast.error(
        e?.message ||
          tt(
            'dashboard.services_portal.toasts.reopen_failed',
            'Failed to reopen installation request',
          ),
      );
    } finally {
      reopeningId = null;
    }
  }

  function activeFilterLabel(filter: StatusFilter) {
    if (filter === 'all') return tt('dashboard.services_portal.filters.all', 'All Services');
    if (filter === 'active') return tt('dashboard.services_portal.filters.active', 'Active');
    if (filter === 'pending_installation') {
      return tt('dashboard.services_portal.filters.pending_installation', 'Pending Installation');
    }
    return tt('dashboard.services_portal.filters.needs_attention', 'Needs Attention');
  }

  function activeSortLabel(key: SortBy) {
    if (key === 'updated_at') return tt('dashboard.services_portal.sort.updated_at', 'Updated');
    if (key === 'price') return tt('dashboard.services_portal.sort.price', 'Price');
    if (key === 'status') return tt('dashboard.services_portal.sort.status', 'Status');
    if (key === 'location_label') return tt('dashboard.services_portal.sort.location', 'Location');
    return tt('dashboard.services_portal.sort.service', 'Service');
  }

  function canTrackInstallation(sub: CustomerSubscriptionView) {
    return (
      sub.status === 'pending_installation' ||
      sub.latest_work_order_status === 'pending' ||
      sub.latest_work_order_status === 'in_progress' ||
      sub.latest_work_order_status === 'completed'
    );
  }

  function toLocalInputValue(raw?: string | null) {
    if (!raw) return '';
    const d = new Date(raw);
    if (!Number.isFinite(d.getTime())) return '';
    const pad = (n: number) => `${n}`.padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(
      d.getHours(),
    )}:${pad(d.getMinutes())}`;
  }

  function stepState(step: 'requested' | 'assigned' | 'scheduled' | 'onsite' | 'active') {
    const wo = trackerWo;
    if (!wo) return 'todo';
    if (step === 'requested') return 'done';
    if (step === 'assigned') {
      return wo.assigned_to ? 'done' : 'todo';
    }
    if (step === 'scheduled') {
      return wo.scheduled_at ? 'done' : 'todo';
    }
    if (step === 'onsite') {
      return wo.status === 'in_progress' || wo.status === 'completed' ? 'done' : 'todo';
    }
    if (step === 'active') {
      return wo.status === 'completed' && trackerSub?.status === 'active' ? 'done' : 'todo';
    }
    return 'todo';
  }

  function canRequestReschedule() {
    const wo = trackerWo;
    return !!wo && wo.status === 'pending' && !!wo.scheduled_at;
  }

  function rescheduleStatusMeta(status?: string | null) {
    const s = String(status || '').toLowerCase();
    if (s === 'approved')
      return {
        tone: 'approved',
        label: tt('dashboard.services_portal.reschedule.badge.approved', 'Reschedule Approved'),
      };
    if (s === 'rejected')
      return {
        tone: 'rejected',
        label: tt('dashboard.services_portal.reschedule.badge.rejected', 'Reschedule Rejected'),
      };
    if (s === 'pending')
      return {
        tone: 'pending',
        label: tt('dashboard.services_portal.reschedule.badge.pending', 'Reschedule Pending'),
      };
    return null;
  }

  function showRescheduleInfo(status?: string | null) {
    return String(status || '').toLowerCase() === 'pending_installation';
  }

  function installationInvoiceForSubscription(invoices: Invoice[], subscriptionId: string) {
    const prefix = `pkgsub:${subscriptionId}`;
    const related = invoices
      .filter((invoice) => {
        const externalId = String(invoice.external_id || '');
        return externalId === prefix || externalId.startsWith(`${prefix}:`);
      })
      .sort((a, b) => {
        const aTime = new Date(a.created_at || a.due_date || 0).getTime();
        const bTime = new Date(b.created_at || b.due_date || 0).getTime();
        return bTime - aTime;
      });
    if (related.length === 0) return null;
    return (
      related.find((invoice) =>
        ['pending', 'verification_pending', 'failed', 'expired'].includes(
          String(invoice.status || '').toLowerCase(),
        ),
      ) || related[0]
    );
  }

  function invoiceStatusTone(status?: string | null) {
    const s = String(status || '').toLowerCase();
    if (s === 'paid') return 'approved';
    if (s === 'pending' || s === 'verification_pending') return 'pending';
    if (s === 'failed' || s === 'expired' || s === 'cancelled') return 'rejected';
    return 'pending';
  }

  async function openSubscriptionInvoice(subscriptionId: string) {
    try {
      const invoices = await api.payment.listCustomerPackageInvoices();
      const invoice = installationInvoiceForSubscription(invoices, subscriptionId);
      if (invoice?.id) {
        await goto(`/pay/${invoice.id}`);
        return;
      }
      await goto('/dashboard/invoices');
    } catch {
      await goto('/dashboard/invoices');
    }
  }

  async function openTracker(sub: CustomerSubscriptionView) {
    trackerOpen = true;
    trackerLoading = true;
    trackerError = '';
    trackerSub = sub;
    trackerWo = null;
    trackerReschedule = null;
    trackerInvoice = null;
    rescheduleReason = '';
    rescheduleAt = '';
    try {
      const [res, invoices] = await Promise.all([
        api.customers.portal.installationTracker(sub.id),
        api.payment.listCustomerPackageInvoices().catch(() => [] as Invoice[]),
      ]);
      const trackerRes = res as CustomerPortalInstallationTrackerResponse;
      trackerInvoice = installationInvoiceForSubscription(invoices, sub.id);
      trackerSub = trackerRes.subscription;
      trackerWo = trackerRes.work_order;
      trackerReschedule = trackerRes.reschedule_request;
      rescheduleAt = toLocalInputValue(trackerRes.work_order?.scheduled_at || null);
    } catch (e: any) {
      trackerError = e?.message || 'Failed to load installation tracker';
    } finally {
      trackerLoading = false;
    }
  }

  function closeTracker() {
    trackerOpen = false;
    trackerLoading = false;
    trackerError = '';
    trackerSub = null;
    trackerWo = null;
    trackerReschedule = null;
    trackerInvoice = null;
    rescheduleAt = '';
    rescheduleReason = '';
    rescheduleBusy = false;
  }

  async function submitReschedule() {
    if (!trackerSub || !canRequestReschedule() || rescheduleBusy) return;
    const value = rescheduleAt.trim();
    if (!value) {
      toast.error(
        tt(
          'dashboard.services_portal.reschedule.toasts.select_schedule',
          'Please select new installation schedule',
        ),
      );
      return;
    }
    const iso = new Date(value).toISOString();
    if (!Number.isFinite(new Date(iso).getTime())) {
      toast.error(
        tt('dashboard.services_portal.reschedule.toasts.invalid_datetime', 'Invalid date time'),
      );
      return;
    }
    rescheduleBusy = true;
    try {
      const res = await api.customers.portal.requestReschedule(trackerSub.id, {
        scheduled_at: iso,
        reason: rescheduleReason.trim() || undefined,
      });
      trackerWo = {
        ...(trackerWo as InstallationWorkOrderView),
        ...(res.work_order as unknown as InstallationWorkOrderView),
      };
      trackerReschedule = {
        ...(trackerReschedule || ({} as WorkOrderRescheduleRequestView)),
        status: 'pending',
        requested_schedule_at: iso,
        reason: rescheduleReason.trim() || null,
      } as WorkOrderRescheduleRequestView;
      rescheduleAt = toLocalInputValue((res.work_order as any)?.scheduled_at || null);
      toast.success(
        tt(
          'dashboard.services_portal.reschedule.toasts.submit_success',
          'Reschedule request submitted',
        ),
      );
      await refreshAll();
    } catch (e: any) {
      toast.error(
        e?.message ||
          tt(
            'dashboard.services_portal.reschedule.toasts.submit_failed',
            'Failed to submit reschedule request',
          ),
      );
    } finally {
      rescheduleBusy = false;
    }
  }

  const tableColumns = $derived.by(() => [
    { key: 'package_name', label: tt('dashboard.services_portal.table.service', 'Service'), sortable: true },
    { key: 'status', label: tt('dashboard.services_portal.table.status', 'Status'), sortable: true },
    { key: 'location_label', label: tt('dashboard.services_portal.table.location', 'Location'), sortable: true },
    { key: 'billing_cycle', label: tt('dashboard.services_portal.table.billing_cycle', 'Billing Cycle') },
    { key: 'price', label: tt('dashboard.services_portal.table.price', 'Price'), align: 'right', sortable: true },
    { key: 'updated_at', label: tt('dashboard.services_portal.table.updated_at', 'Updated'), sortable: true },
    { key: 'actions', label: tt('dashboard.services_portal.table.actions', 'Actions'), align: 'right' },
  ]);
</script>

<div class="services-content fade-in">
  <section class="page-head">
    <div>
      <h1>{tt('dashboard.services_portal.page.title', 'Customer Services')}</h1>
      <p>{tt('dashboard.services_portal.page.subtitle', 'List of all services you have ordered.')}</p>
    </div>
    <div class="head-actions">
      <button class="btn ghost" type="button" onclick={() => void refreshAll()} disabled={loading}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
      <button class="btn ghost" type="button" onclick={() => goto('/dashboard/invoices')}>
        <Icon name="file-text" size={16} />
        {tt('dashboard.services_portal.actions.billing_invoices', 'Billing & Invoices')}
      </button>
      <button class="btn primary" type="button" onclick={() => goto('/dashboard/services/order')}>
        <Icon name="plus" size={16} />
        {tt('dashboard.services_portal.actions.order_new_service', 'Order New Service')}
      </button>
    </div>
  </section>

  {#if loadError}
    <section class="alert">{loadError}</section>
  {/if}

  <section class="stats-grid">
    <button
      type="button"
      class={`stat-card indigo ${statusFilter === 'all' ? 'is-active' : ''}`}
      onclick={() => setStatusFilter('all')}
    >
      <div class="stat-icon"><Icon name="layers" size={20} /></div>
      <div class="stat-content">
        <span class="stat-value">{stats.total}</span>
        <span class="stat-label">{tt('dashboard.services_portal.stats.total', 'Total Services')}</span>
      </div>
    </button>

    <button
      type="button"
      class={`stat-card emerald ${statusFilter === 'active' ? 'is-active' : ''}`}
      onclick={() => setStatusFilter('active')}
    >
      <div class="stat-icon"><Icon name="check-circle" size={20} /></div>
      <div class="stat-content">
        <span class="stat-value">{stats.active}</span>
        <span class="stat-label">{tt('dashboard.services_portal.stats.active', 'Active')}</span>
      </div>
    </button>

    <button
      type="button"
      class={`stat-card amber ${statusFilter === 'pending_installation' ? 'is-active' : ''}`}
      onclick={() => setStatusFilter('pending_installation')}
    >
      <div class="stat-icon"><Icon name="clock" size={20} /></div>
      <div class="stat-content">
        <span class="stat-value">{stats.pendingInstallation}</span>
        <span class="stat-label">{tt('dashboard.services_portal.stats.pending_installation', 'Pending Installation')}</span>
      </div>
    </button>

    <button
      type="button"
      class={`stat-card rose ${statusFilter === 'needs_attention' ? 'is-active' : ''}`}
      onclick={() => setStatusFilter('needs_attention')}
    >
      <div class="stat-icon"><Icon name="alert-triangle" size={20} /></div>
      <div class="stat-content">
        <span class="stat-value">{stats.needsAttention}</span>
        <span class="stat-label">{tt('dashboard.services_portal.stats.needs_attention', 'Need Attention')}</span>
      </div>
    </button>
  </section>

  <section class="list-panel">
    <div class="panel-head">
      <div>
        <h2>{tt('dashboard.services_portal.list.title', 'Ordered Services')}</h2>
        <p>{tt('dashboard.services_portal.list.subtitle', 'Services requested by your account.')}</p>
      </div>
      <span class="count-pill">{totalCount} {tt('dashboard.services_portal.list.services_suffix', 'services')}</span>
    </div>

    <div class="list-tools">
      <div class="active-filter-chip">
        <span>{tt('dashboard.services_portal.list.filter', 'Filter')}: {activeFilterLabel(statusFilter)}</span>
        {#if statusFilter !== 'all'}
          <button type="button" class="chip-clear" onclick={clearFilter}>
            <Icon name="x" size={12} />
            {tt('common.clear', 'Clear')}
          </button>
        {/if}
      </div>
      <div class="active-filter-chip">
        <span>{tt('dashboard.services_portal.list.sort', 'Sort')}: {activeSortLabel(sortBy)} ({sortDirection.toUpperCase()})</span>
      </div>
    </div>

    {#if loading}
      <div class="state-block">
        <div class="spinner"></div>
        <p>{$t('common.loading') || 'Loading...'}</p>
      </div>
    {:else if filteredSubscriptions.length === 0}
      <div class="state-block empty">
        <Icon name="package" size={18} />
        {#if totalCount === 0}
          <p>{tt('dashboard.services_portal.empty.no_services', 'No ordered services yet.')}</p>
          <button class="btn primary" type="button" onclick={() => goto('/dashboard/services/order')}>
            <Icon name="plus" size={15} />
            {tt('dashboard.services_portal.actions.order_new_service', 'Order New Service')}
          </button>
        {:else}
          <p>{tt('dashboard.services_portal.empty.no_match', 'No service found for selected filter.')}</p>
          <button class="btn ghost" type="button" onclick={clearFilter}>{tt('dashboard.services_portal.actions.show_all_services', 'Show All Services')}</button>
        {/if}
      </div>
    {:else}
      <div class="desktop-table">
        <Table
          data={filteredSubscriptions}
          columns={tableColumns}
          loading={loading}
          pagination={true}
          serverSide={true}
          count={totalCount}
          pageSize={pageSize}
          mobileView="scroll"
          searchable={false}
          sortKey={sortBy}
          sortDirection={sortDirection}
          onsort={handleTableSort}
          onchange={(nextPage) => {
            page = nextPage;
            void loadData();
          }}
          onpageSizeChange={(nextSize) => {
            pageSize = nextSize;
            page = 0;
            void loadData();
          }}
        >
          {#snippet cell({ item, column })}
            {#if column.key === 'package_name'}
              <div class="service-name-cell">
                <strong>{item.package_name || item.package_id}</strong>
                {#if showRescheduleInfo(item.status) && rescheduleStatusMeta(item.latest_reschedule_status)}
                  <span class={`reschedule-chip ${rescheduleStatusMeta(item.latest_reschedule_status)?.tone}`}>
                    {rescheduleStatusMeta(item.latest_reschedule_status)?.label}
                  </span>
                {/if}
              </div>
            {:else if column.key === 'status'}
              {@const status = serviceStatusMeta(item.status, item.starts_at, item.can_request_reopen)}
              <span class={`table-status-pill ${status.tone}`}>{status.label}</span>
            {:else if column.key === 'location_label'}
              {item.location_label || item.location_id}
            {:else if column.key === 'billing_cycle'}
              {billingCycleLabel(item.billing_cycle)}
            {:else if column.key === 'price'}
              <strong>{formatCurrency(Number(item.price || 0), item.currency_code)}</strong>
            {:else if column.key === 'updated_at'}
              {formatDate(item.updated_at)}
            {:else if column.key === 'actions'}
              <div class="table-actions">
                {#if item.can_request_reopen}
                  <button
                    class="table-action-btn"
                    type="button"
                    onclick={() => requestReopen(item)}
                    disabled={reopeningId === item.id}
                  >
                    <Icon name="rotate-ccw" size={14} />
                    {reopeningId === item.id
                      ? tt('dashboard.services_portal.actions.reopening', 'Reopening...')
                      : tt('dashboard.services_portal.actions.request_reopen', 'Request Reopen')}
                  </button>
                {/if}
                <button class="table-action-btn" type="button" onclick={() => openSubscriptionInvoice(item.id)}>
                  <Icon name="file-text" size={14} />
                  {invoiceActionLabel(item.status, item.starts_at)}
                </button>
                {#if canTrackInstallation(item)}
                  <button class="table-action-btn" type="button" onclick={() => openTracker(item)}>
                    <Icon name="map-pin" size={14} />
                    {tt('dashboard.services_portal.actions.track_installation', 'Track Installation')}
                  </button>
                {/if}
              </div>
            {:else}
              {item[column.key] ?? '-'}
            {/if}
          {/snippet}
        </Table>
      </div>

      <div class="mobile-cards">
        {#each filteredSubscriptions as sub (sub.id)}
          {@const status = serviceStatusMeta(sub.status, sub.starts_at, sub.can_request_reopen)}
          <article class="service-card">
            <div class="service-top">
              <div class="service-top-left">
                <h3>{sub.package_name || sub.package_id}</h3>
                {#if showRescheduleInfo(sub.status) && rescheduleStatusMeta(sub.latest_reschedule_status)}
                  <span class={`reschedule-chip ${rescheduleStatusMeta(sub.latest_reschedule_status)?.tone}`}>
                    {rescheduleStatusMeta(sub.latest_reschedule_status)?.label}
                  </span>
                {/if}
              </div>
              <span class={`status-pill ${status.tone}`}>{status.label}</span>
            </div>

            <p class="service-hint">{status.hint}</p>

            <div class="service-meta">
              <div>
                <small>{tt('dashboard.services_portal.table.location', 'Location')}</small>
                <strong>{sub.location_label || sub.location_id}</strong>
              </div>
              <div>
                <small>{tt('dashboard.services_portal.table.billing_cycle', 'Billing Cycle')}</small>
                <strong>{billingCycleLabel(sub.billing_cycle)}</strong>
              </div>
              <div>
                <small>{tt('dashboard.services_portal.table.price', 'Price')}</small>
                <strong>{formatCurrency(Number(sub.price || 0), sub.currency_code)}</strong>
              </div>
              <div>
                <small>{tt('dashboard.services_portal.table.updated_at', 'Updated')}</small>
                <strong>{formatDate(sub.updated_at)}</strong>
              </div>
              {#if sub.latest_reschedule_requested_at}
                <div>
                  <small>
                    {tt('dashboard.services_portal.reschedule.labels.requested_at', 'Reschedule At')}
                  </small>
                  <strong>{formatDate(sub.latest_reschedule_requested_at)}</strong>
                </div>
              {/if}
            </div>

            <div class="service-actions">
              {#if sub.can_request_reopen}
                <button
                  class="text-btn"
                  type="button"
                  onclick={() => requestReopen(sub)}
                  disabled={reopeningId === sub.id}
                >
                  <Icon name="rotate-ccw" size={14} />
                  {reopeningId === sub.id
                    ? tt('dashboard.services_portal.actions.reopening', 'Reopening...')
                    : tt('dashboard.services_portal.actions.request_reopen', 'Request Reopen')}
                </button>
              {/if}
              <button class="text-btn" type="button" onclick={() => openSubscriptionInvoice(sub.id)}>
                <Icon name="file-text" size={14} />
                {invoiceActionLabel(sub.status, sub.starts_at)}
              </button>
              {#if canTrackInstallation(sub)}
                <button class="text-btn" type="button" onclick={() => openTracker(sub)}>
                  <Icon name="map-pin" size={14} />
                  {tt('dashboard.services_portal.actions.track_installation', 'Track Installation')}
                </button>
              {/if}
            </div>
          </article>
        {/each}
      </div>
      {#if totalCount > pageSize}
        <div class="mobile-pager">
          <button
            class="text-btn"
            type="button"
            onclick={() => {
              if (page <= 0 || loading) return;
              page -= 1;
              void loadData();
            }}
            disabled={loading || page <= 0}
          >
            <Icon name="chevron-left" size={14} />
            {tt('common.previous', 'Previous')}
          </button>
          <span class="mobile-page-indicator">
            {tt('common.page', 'Page')} {page + 1} / {Math.max(1, Math.ceil(totalCount / pageSize))}
          </span>
          <button
            class="text-btn"
            type="button"
            onclick={() => {
              const maxPage = Math.max(0, Math.ceil(totalCount / pageSize) - 1);
              if (loading || page >= maxPage) return;
              page += 1;
              void loadData();
            }}
            disabled={loading || page >= Math.max(0, Math.ceil(totalCount / pageSize) - 1)}
          >
            {tt('common.next', 'Next')}
            <Icon name="chevron-right" size={14} />
          </button>
        </div>
      {/if}
    {/if}
  </section>
</div>

{#if trackerOpen}
  <div
    class="tracker-backdrop"
    role="button"
    tabindex="0"
    onclick={(e) => {
      if (e.target === e.currentTarget) closeTracker();
    }}
    onkeydown={(e) => {
      if (e.key === 'Escape') closeTracker();
    }}
  >
    <section class="tracker-modal">
      <header class="tracker-head">
        <div>
          <h3>{tt('dashboard.services_portal.tracker.title', 'Installation Tracker')}</h3>
          <p>{trackerSub?.package_name || trackerSub?.package_id || '-'}</p>
        </div>
        <button class="btn ghost" type="button" onclick={closeTracker}>
          <Icon name="x" size={14} />
          {tt('common.close', 'Close')}
        </button>
      </header>

      {#if trackerLoading}
        <div class="state-block">
          <div class="spinner"></div>
          <p>{tt('dashboard.services_portal.tracker.loading', 'Loading tracker...')}</p>
        </div>
      {:else if trackerError}
        <div class="alert">{trackerError}</div>
      {:else}
        <div class="tracker-steps">
          <div class={`step-pill ${stepState('requested')}`}>1. {tt('dashboard.services_portal.tracker.steps.requested', 'Requested')}</div>
          <div class={`step-pill ${stepState('assigned')}`}>2. {tt('dashboard.services_portal.tracker.steps.assigned', 'Assigned')}</div>
          <div class={`step-pill ${stepState('scheduled')}`}>3. {tt('dashboard.services_portal.tracker.steps.scheduled', 'Scheduled')}</div>
          <div class={`step-pill ${stepState('onsite')}`}>4. {tt('dashboard.services_portal.tracker.steps.onsite', 'On-site')}</div>
          <div class={`step-pill ${stepState('active')}`}>5. {tt('dashboard.services_portal.tracker.steps.active', 'Active')}</div>
        </div>

        <div class="tracker-grid">
          <div>
            <small>{tt('dashboard.services_portal.table.status', 'Status')}</small>
            <strong>{trackerWo?.status || '-'}</strong>
          </div>
          <div>
            <small>{tt('common.assignee', 'Assignee')}</small>
            <strong>{trackerWo?.assigned_to_name || trackerWo?.assigned_to_email || '-'}</strong>
          </div>
          <div>
            <small>{tt('dashboard.services_portal.tracker.scheduled_at', 'Scheduled At')}</small>
            <strong>{formatDate(trackerWo?.scheduled_at)}</strong>
          </div>
          <div>
            <small>{tt('dashboard.services_portal.tracker.last_update', 'Last Update')}</small>
            <strong>{formatDate(trackerWo?.updated_at || trackerSub?.updated_at)}</strong>
          </div>
        </div>

        {#if trackerInvoice}
          <section class="reschedule-status">
            <div class="reschedule-status-head">
              <h4>Invoice</h4>
              <span class={`request-status ${invoiceStatusTone(trackerInvoice.status)}`}>
                {String(trackerInvoice.status || 'pending').toUpperCase()}
              </span>
            </div>
            <div class="reschedule-status-grid">
              <div>
                <small>Invoice Number</small>
                <strong>{trackerInvoice.invoice_number || '-'}</strong>
              </div>
              <div>
                <small>Amount</small>
                <strong>{formatCurrency(Number(trackerInvoice.amount || 0), trackerInvoice.currency_code)}</strong>
              </div>
              <div>
                <small>Due Date</small>
                <strong>{formatDate(trackerInvoice.due_date)}</strong>
              </div>
              <div>
                <small>Paid At</small>
                <strong>{formatDate(trackerInvoice.paid_at)}</strong>
              </div>
            </div>
            <div class="reschedule-actions">
              <button
                class="btn ghost"
                type="button"
                onclick={() => (trackerInvoice?.id ? goto(`/pay/${trackerInvoice.id}`) : openSubscriptionInvoice(trackerSub?.id || ''))}
              >
                <Icon name="file-text" size={14} />
                {invoiceActionLabel(trackerSub?.status, trackerSub?.starts_at)}
              </button>
            </div>
          </section>
        {/if}

        {#if trackerReschedule && showRescheduleInfo(trackerSub?.status)}
          <section class="reschedule-status">
            <div class="reschedule-status-head">
              <h4>
                {tt(
                  'dashboard.services_portal.reschedule.latest_title',
                  'Latest Reschedule Request',
                )}
              </h4>
              <span class={`request-status ${trackerReschedule.status || 'pending'}`}>
                {String(trackerReschedule.status || 'pending').toUpperCase()}
              </span>
            </div>
            <div class="reschedule-status-grid">
              <div>
                <small>
                  {tt(
                    'dashboard.services_portal.reschedule.labels.requested_schedule',
                    'Requested Schedule',
                  )}
                </small>
                <strong>{formatDate(trackerReschedule.requested_schedule_at)}</strong>
              </div>
              <div>
                <small>
                  {tt('dashboard.services_portal.reschedule.labels.requested_by', 'Requested By')}
                </small>
                <strong>{trackerReschedule.requested_by_name || trackerReschedule.requested_by_email || '-'}</strong>
              </div>
            </div>
            {#if trackerReschedule.reason}
              <p>
                <strong>{tt('dashboard.services_portal.reschedule.labels.reason', 'Reason')}:</strong>
                {trackerReschedule.reason}
              </p>
            {/if}
            {#if trackerReschedule.review_notes}
              <p>
                <strong>{tt('dashboard.services_portal.reschedule.labels.admin_notes', 'Admin Notes')}:</strong>
                {trackerReschedule.review_notes}
              </p>
            {/if}
          </section>
        {/if}

        {#if canRequestReschedule()}
          <section class="reschedule-box">
            <h4>{tt('dashboard.services_portal.reschedule.form.title', 'Request Reschedule')}</h4>
            <p>
              {tt(
                'dashboard.services_portal.reschedule.form.subtitle',
                'You can request new installation time before onsite work starts.',
              )}
            </p>
            <div class="reschedule-form">
              <label>
                {tt('dashboard.services_portal.reschedule.form.new_schedule', 'New Schedule')}
                <input type="datetime-local" bind:value={rescheduleAt} />
              </label>
              <label>
                {tt('dashboard.services_portal.reschedule.form.reason_optional', 'Reason (optional)')}
                <textarea
                  rows="3"
                  bind:value={rescheduleReason}
                  placeholder={tt(
                    'dashboard.services_portal.reschedule.form.reason_placeholder',
                    'Need different time slot',
                  )}
                ></textarea>
              </label>
            </div>
            <div class="reschedule-actions">
              <button class="btn primary" type="button" onclick={submitReschedule} disabled={rescheduleBusy}>
                <Icon name="calendar" size={14} />
                {rescheduleBusy
                  ? tt('dashboard.services_portal.reschedule.form.submitting', 'Submitting...')
                  : tt('dashboard.services_portal.reschedule.form.submit', 'Submit Reschedule')}
              </button>
            </div>
          </section>
        {:else if trackerWo?.status === 'pending' && !trackerWo?.scheduled_at}
          <section class="reschedule-box">
            <h4>{tt('dashboard.services_portal.reschedule.form.title', 'Request Reschedule')}</h4>
            <p>
              {tt(
                'dashboard.services_portal.reschedule.form.wait_schedule',
                'Reschedule will be available after admin/technician sets your installation schedule.',
              )}
            </p>
          </section>
        {/if}

      {/if}
    </section>
  </div>
{/if}

<style>
  .services-content {
    max-width: 1400px;
    margin: 0 auto;
    padding: 2rem;
    display: grid;
    gap: 1.25rem;
  }

  .page-head {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    background: linear-gradient(145deg, var(--bg-surface), #0b0c10);
    padding: 1.25rem;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 1rem;
    align-items: center;
  }

  .page-head h1 {
    margin: 0;
    font-size: 1.9rem;
    color: var(--text-primary);
  }

  .page-head p {
    margin: 0.35rem 0 0;
    color: var(--text-secondary);
  }

  .head-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 0.65rem;
  }

  .btn {
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    border-radius: 12px;
    padding: 0.62rem 0.9rem;
    font-weight: 800;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
  }

  .btn:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .btn.primary {
    background: var(--color-primary);
    border-color: color-mix(in srgb, var(--color-primary) 55%, var(--border-color));
    color: white;
  }

  .btn.ghost {
    background: color-mix(in srgb, var(--bg-surface) 75%, transparent);
  }

  .btn:disabled {
    opacity: 0.66;
    cursor: not-allowed;
  }

  .alert {
    border: 1px solid rgba(239, 68, 68, 0.35);
    background: rgba(239, 68, 68, 0.08);
    color: #fecaca;
    padding: 0.85rem 1rem;
    border-radius: 12px;
    font-weight: 600;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.85rem;
  }

  .stat-card {
    appearance: none;
    text-align: left;
    width: 100%;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 1rem;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
    cursor: pointer;
    transition: border-color 0.16s ease, transform 0.16s ease, box-shadow 0.16s ease;
  }

  .stat-card:hover {
    transform: translateY(-1px);
    border-color: color-mix(in srgb, var(--color-primary) 35%, var(--border-color));
  }

  .stat-card.is-active {
    border-color: color-mix(in srgb, var(--color-primary) 55%, var(--border-color));
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-primary) 26%, transparent),
      0 12px 28px rgba(0, 0, 0, 0.28);
  }

  .stat-icon {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(255, 255, 255, 0.2);
  }

  .stat-content {
    display: grid;
    gap: 0.15rem;
  }

  .stat-value {
    font-size: 1.35rem;
    font-weight: 900;
    line-height: 1;
    color: var(--text-primary);
  }

  .stat-label {
    font-size: 0.8rem;
    color: var(--text-secondary);
    font-weight: 700;
  }

  .stat-card.indigo {
    background: radial-gradient(circle at 15% 20%, rgba(99, 102, 241, 0.2), transparent 55%), #0c1020;
    border-color: rgba(99, 102, 241, 0.32);
  }

  .stat-card.emerald {
    background: radial-gradient(circle at 15% 20%, rgba(16, 185, 129, 0.18), transparent 55%), #0c1411;
    border-color: rgba(16, 185, 129, 0.3);
  }

  .stat-card.amber {
    background: radial-gradient(circle at 15% 20%, rgba(245, 158, 11, 0.18), transparent 55%), #1a1308;
    border-color: rgba(245, 158, 11, 0.3);
  }

  .stat-card.rose {
    background: radial-gradient(circle at 15% 20%, rgba(244, 63, 94, 0.18), transparent 55%), #1b0c11;
    border-color: rgba(244, 63, 94, 0.3);
  }

  .list-panel {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    background: linear-gradient(145deg, var(--bg-surface), #0b0c10);
    padding: 1rem;
    display: grid;
    gap: 0.95rem;
  }

  .panel-head {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.75rem;
    align-items: start;
  }

  .panel-head h2 {
    margin: 0;
    color: var(--text-primary);
    font-size: 1.12rem;
  }

  .panel-head p {
    margin: 0.3rem 0 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .count-pill {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 0.36rem 0.65rem;
    font-size: 0.78rem;
    color: var(--text-secondary);
    font-weight: 800;
    align-self: center;
  }

  .list-tools {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.8rem;
    align-items: center;
  }

  .active-filter-chip {
    display: inline-flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.45rem;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 78%, transparent);
    border-radius: 999px;
    padding: 0.34rem 0.66rem;
    font-size: 0.76rem;
    color: var(--text-secondary);
    font-weight: 700;
    width: fit-content;
  }

  .chip-clear {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    background: transparent;
    color: var(--text-primary);
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.15rem 0.48rem;
    font-size: 0.72rem;
    cursor: pointer;
    font-weight: 700;
  }

  .chip-clear:hover {
    background: var(--bg-hover);
  }

  .state-block {
    border: 1px dashed var(--border-color);
    border-radius: 12px;
    min-height: 120px;
    display: grid;
    place-content: center;
    text-align: center;
    gap: 0.7rem;
    color: var(--text-secondary);
    padding: 1.2rem;
  }

  .state-block.empty {
    gap: 0.85rem;
  }

  .spinner {
    width: 28px;
    height: 28px;
    border-radius: 999px;
    border: 2px solid color-mix(in srgb, var(--text-secondary) 35%, transparent);
    border-top-color: var(--color-primary);
    animation: spin 0.8s linear infinite;
    margin: 0 auto;
  }

  .desktop-table {
    display: block;
  }

  .mobile-cards {
    display: none;
  }

  :global(.desktop-table .table-search) {
    padding: 0.4rem 0.1rem 0.9rem;
  }

  :global(.desktop-table .search-input-wrapper) {
    max-width: 360px;
  }

  .table-status-pill {
    border-radius: 999px;
    border: 1px solid var(--border-color);
    padding: 0.24rem 0.6rem;
    font-weight: 800;
    font-size: 0.72rem;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .table-status-pill.active {
    background: rgba(34, 197, 94, 0.12);
    border-color: rgba(34, 197, 94, 0.32);
    color: rgba(34, 197, 94, 0.95);
  }

  .table-status-pill.pending {
    background: rgba(245, 158, 11, 0.12);
    border-color: rgba(245, 158, 11, 0.32);
    color: rgba(245, 158, 11, 0.95);
  }

  .table-status-pill.suspended {
    background: rgba(59, 130, 246, 0.12);
    border-color: rgba(59, 130, 246, 0.32);
    color: rgba(147, 197, 253, 0.98);
  }

  .table-status-pill.cancelled {
    background: rgba(239, 68, 68, 0.12);
    border-color: rgba(239, 68, 68, 0.32);
    color: rgba(252, 165, 165, 0.98);
  }

  .table-status-pill.default {
    color: var(--text-secondary);
  }

  .table-actions {
    display: flex;
    justify-content: flex-end;
  }

  .service-name-cell {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .reschedule-chip {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 0.15rem 0.5rem;
    font-size: 0.68rem;
    font-weight: 800;
    border: 1px solid var(--border-color);
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }

  .reschedule-chip.pending {
    border-color: rgba(245, 158, 11, 0.48);
    color: #facc15;
    background: rgba(161, 98, 7, 0.2);
  }

  .reschedule-chip.approved {
    border-color: rgba(34, 197, 94, 0.48);
    color: #86efac;
    background: rgba(21, 128, 61, 0.2);
  }

  .reschedule-chip.rejected {
    border-color: rgba(239, 68, 68, 0.48);
    color: #fca5a5;
    background: rgba(185, 28, 28, 0.2);
  }

  .table-action-btn {
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    border-radius: 9px;
    padding: 0.35rem 0.58rem;
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.76rem;
    font-weight: 700;
    cursor: pointer;
    white-space: nowrap;
  }

  .table-action-btn:hover {
    background: var(--bg-hover);
  }

  .mobile-cards {
    gap: 0.8rem;
  }

  .mobile-pager {
    display: none;
    margin-top: 0.7rem;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
  }

  .mobile-page-indicator {
    color: var(--text-secondary);
    font-size: 0.8rem;
    font-weight: 700;
  }

  .service-card {
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 0.85rem;
    display: grid;
    gap: 0.65rem;
    background: color-mix(in srgb, var(--bg-surface) 82%, transparent);
  }

  .service-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .service-top-left {
    display: grid;
    gap: 0.3rem;
    min-width: 0;
  }

  .tracker-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(2, 6, 18, 0.72);
    display: grid;
    place-items: center;
    z-index: 1200;
    padding: 18px;
  }

  .tracker-modal {
    width: min(860px, 100%);
    max-height: calc(100vh - 36px);
    overflow: auto;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: #0d1323;
    padding: 14px;
    display: grid;
    gap: 12px;
  }

  .tracker-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
  }

  .tracker-head h3 {
    margin: 0;
    color: var(--text-primary);
  }

  .tracker-head p {
    margin: 4px 0 0;
    color: var(--text-secondary);
    font-size: 0.88rem;
  }

  .tracker-steps {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 8px;
  }

  .step-pill {
    border-radius: 999px;
    border: 1px solid #334155;
    color: #9ca9c2;
    font-size: 0.78rem;
    padding: 7px 10px;
    text-align: center;
    font-weight: 700;
  }

  .step-pill.done {
    border-color: rgba(34, 197, 94, 0.4);
    background: rgba(34, 197, 94, 0.14);
    color: #86efac;
  }

  .tracker-grid {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: #11192d;
    padding: 10px;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .tracker-grid small {
    display: block;
    color: var(--text-secondary);
    margin-bottom: 4px;
    font-size: 0.75rem;
  }

  .tracker-grid strong {
    color: var(--text-primary);
    font-size: 0.92rem;
  }

  .reschedule-status {
    border: 1px solid rgba(234, 179, 8, 0.36);
    background: rgba(120, 53, 15, 0.12);
    border-radius: 12px;
    padding: 10px;
    display: grid;
    gap: 8px;
  }

  .reschedule-status-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .reschedule-status-head h4 {
    margin: 0;
    color: var(--text-primary);
  }

  .request-status {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 0.2rem 0.55rem;
    font-size: 0.72rem;
    font-weight: 800;
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
    background: rgba(148, 163, 184, 0.12);
  }

  .request-status.pending {
    border-color: rgba(245, 158, 11, 0.5);
    color: #facc15;
    background: rgba(161, 98, 7, 0.18);
  }

  .request-status.approved {
    border-color: rgba(34, 197, 94, 0.5);
    color: #86efac;
    background: rgba(21, 128, 61, 0.18);
  }

  .request-status.rejected {
    border-color: rgba(239, 68, 68, 0.5);
    color: #fca5a5;
    background: rgba(185, 28, 28, 0.2);
  }

  .reschedule-status-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .reschedule-status-grid small {
    display: block;
    color: var(--text-secondary);
    margin-bottom: 4px;
    font-size: 0.75rem;
  }

  .reschedule-status-grid strong {
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .reschedule-status p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.84rem;
  }

  .reschedule-box {
    border: 1px solid rgba(99, 102, 241, 0.32);
    background: rgba(99, 102, 241, 0.09);
    border-radius: 12px;
    padding: 10px;
    display: grid;
    gap: 8px;
  }

  .reschedule-box h4 {
    margin: 0;
    color: var(--text-primary);
  }

  .reschedule-box p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.85rem;
  }

  .reschedule-form {
    display: grid;
    gap: 8px;
  }

  .reschedule-form label {
    display: grid;
    gap: 6px;
    font-size: 0.82rem;
    color: var(--text-secondary);
  }

  .reschedule-form input,
  .reschedule-form textarea {
    border: 1px solid var(--border-color);
    background: #0b1220;
    color: var(--text-primary);
    border-radius: 10px;
    padding: 8px 10px;
  }

  .reschedule-actions {
    display: flex;
    justify-content: flex-end;
  }

  .tracker-notes {
    border-top: 1px dashed var(--border-color);
    padding-top: 10px;
  }

  .service-top h3 {
    margin: 0;
    color: var(--text-primary);
    font-size: 1rem;
  }

  .service-hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.84rem;
    line-height: 1.45;
  }

  .status-pill {
    border-radius: 999px;
    border: 1px solid var(--border-color);
    padding: 0.22rem 0.58rem;
    font-weight: 850;
    font-size: 0.72rem;
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .status-pill.active {
    background: rgba(34, 197, 94, 0.12);
    border-color: rgba(34, 197, 94, 0.32);
    color: rgba(34, 197, 94, 0.95);
  }

  .status-pill.pending {
    background: rgba(245, 158, 11, 0.12);
    border-color: rgba(245, 158, 11, 0.32);
    color: rgba(245, 158, 11, 0.95);
  }

  .status-pill.suspended {
    background: rgba(59, 130, 246, 0.12);
    border-color: rgba(59, 130, 246, 0.32);
    color: rgba(147, 197, 253, 0.98);
  }

  .status-pill.cancelled {
    background: rgba(239, 68, 68, 0.12);
    border-color: rgba(239, 68, 68, 0.32);
    color: rgba(252, 165, 165, 0.98);
  }

  .status-pill.default {
    color: var(--text-secondary);
  }

  .service-meta {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.6rem;
  }

  .service-meta div {
    display: grid;
    gap: 0.18rem;
  }

  .service-meta small {
    color: var(--text-secondary);
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 800;
  }

  .service-meta strong {
    color: var(--text-primary);
    font-size: 0.88rem;
    line-height: 1.3;
    word-break: break-word;
  }

  .service-actions {
    padding-top: 0.25rem;
    border-top: 1px dashed var(--border-color);
  }

  .text-btn {
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    border-radius: 10px;
    padding: 0.4rem 0.62rem;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.8rem;
    font-weight: 700;
    cursor: pointer;
  }

  .text-btn:hover {
    background: var(--bg-hover);
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 1100px) {
    .stats-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 760px) {
    .services-content {
      padding: 1rem;
    }

    .page-head,
    .panel-head,
    .list-tools {
      grid-template-columns: 1fr;
    }

    .head-actions {
      justify-content: flex-start;
    }

    .stats-grid,
    .service-meta,
    .tracker-grid,
    .reschedule-status-grid {
      grid-template-columns: 1fr;
    }

    .desktop-table {
      display: none;
    }

    .mobile-cards {
      display: grid;
      grid-template-columns: 1fr;
    }

    .mobile-pager {
      display: flex;
    }
  }
</style>
