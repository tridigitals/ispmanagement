<script lang="ts">
  /*
    Layanan portal v2 — gelombang 25c.
    Versi lama: (app)/dashboard/services/+page.svelte (1.454 baris).
    Perilaku identik: KPI filter + tabel layanan (server-side, sort) +
    aksi (reopen, tagihan instalasi, track installation + reschedule).
    Pola DS: PortalShell + PageHeader + Card + DataTable + Badge + Button.
  */
  import { goto } from '$app/navigation';
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import { onMount } from 'svelte';
  import type { Component } from 'svelte';
  import { toast } from '$lib/stores/toast';
  import {
    api,
    type CustomerSubscriptionView,
    type CustomerPortalInstallationTrackerResponse,
    type Invoice,
    type InstallationWorkOrderView,
    type WorkOrderRescheduleRequestView,
  } from '$lib/api/client';
  import { appSettings } from '$lib/stores/settings';
  import { fetchAllRows } from '$lib/utils/fetchAllPages';
  import { loadDashboardServicesTrackerModal } from '../../../../(app)/dashboard/services/dashboardServicesPageModules';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import DataTable from '$lib/components/ds/DataTable.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Field from '$lib/components/ds/Field.svelte';
  import type { Column } from '$lib/components/ds/table-types';
  import type { StatusTone } from '$lib/components/ds/tokens';

  type StatusFilter = 'all' | 'active' | 'pending_installation' | 'needs_attention';
  type SortBy = 'updated_at' | 'price' | 'status' | 'package_name' | 'location_label';
  type DeferredComponent = Component<any>;

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
  let TrackerModalComponent = $state<DeferredComponent | null>(null);
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
      toast.error('Gagal memuat layanan.');
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
      // keep existing stats
    }
  }

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
    if (normalized === 'yearly') return 'Tahunan';
    return 'Bulanan';
  }

  function serviceStatusMeta(status?: string | null, startsAt?: string | null, canRequestReopen = false): { label: string; tone: StatusTone; hint: string } {
    if (canRequestReopen) {
      return {
        label: 'Dibatalkan',
        tone: 'neutral',
        hint: 'Permintaan dibatalkan. Anda bisa mengajukan buka ulang.',
      };
    }
    const s = String(status || '').toLowerCase();
    if (s === 'active') {
      return { label: 'Aktif', tone: 'positive', hint: 'Layanan aktif dan berjalan.' };
    }
    if (s === 'pending_installation') {
      return { label: 'Menunggu Instalasi', tone: 'warning', hint: 'Menunggu penugasan dan jadwal instalasi.' };
    }
    if (s === 'suspended') {
      const awaitingPayment = !startsAt;
      return {
        label: awaitingPayment ? 'Menunggu Pembayaran' : 'Ditangguhkan',
        tone: 'negative',
        hint: awaitingPayment
          ? 'Instalasi selesai. Menunggu konfirmasi invoice/pembayaran pertama.'
          : 'Layanan ditangguhkan. Cek tagihan atau hubungi support.',
      };
    }
    if (s === 'cancelled') {
      return { label: 'Dibatalkan', tone: 'neutral', hint: 'Permintaan dibatalkan.' };
    }
    return { label: status || '-', tone: 'neutral', hint: '' };
  }

  function invoiceActionLabel(status?: string | null, startsAt?: string | null) {
    const normalized = String(status || '').toLowerCase();
    if (normalized === 'suspended' && !startsAt) return 'Bayar Invoice';
    return 'Lihat Tagihan';
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
      toast.success(woId ? `Permintaan instalasi dibuka ulang (WO ${woId})` : 'Permintaan instalasi dibuka ulang.');
      await refreshAll();
    } catch (e: any) {
      toast.error(e?.message || 'Gagal membuka ulang permintaan instalasi.');
    } finally {
      reopeningId = null;
    }
  }

  function activeFilterLabel(filter: StatusFilter) {
    if (filter === 'all') return 'Semua Layanan';
    if (filter === 'active') return 'Aktif';
    if (filter === 'pending_installation') return 'Menunggu Instalasi';
    return 'Perlu Perhatian';
  }

  function activeSortLabel(key: SortBy) {
    if (key === 'updated_at') return 'Diperbarui';
    if (key === 'price') return 'Harga';
    if (key === 'status') return 'Status';
    if (key === 'location_label') return 'Lokasi';
    return 'Layanan';
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
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  function stepState(step: 'requested' | 'assigned' | 'scheduled' | 'onsite' | 'active') {
    const wo = trackerWo;
    if (!wo) return 'todo';
    if (step === 'requested') return 'done';
    if (step === 'assigned') return wo.assigned_to ? 'done' : 'todo';
    if (step === 'scheduled') return wo.scheduled_at ? 'done' : 'todo';
    if (step === 'onsite') return wo.status === 'in_progress' || wo.status === 'completed' ? 'done' : 'todo';
    if (step === 'active') return wo.status === 'completed' && trackerSub?.status === 'active' ? 'done' : 'todo';
    return 'todo';
  }

  function canRequestReschedule() {
    const wo = trackerWo;
    return !!wo && wo.status === 'pending' && !!wo.scheduled_at;
  }

  function rescheduleStatusMeta(status?: string | null) {
    const s = String(status || '').toLowerCase();
    if (s === 'approved') return { tone: 'approved', label: 'Reschedule Disetujui' };
    if (s === 'rejected') return { tone: 'rejected', label: 'Reschedule Ditolak' };
    if (s === 'pending') return { tone: 'pending', label: 'Reschedule Menunggu' };
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
      const invoices = await fetchAllRows<Invoice>((page, per_page) =>
        api.payment.listCustomerPackageInvoices({ page, per_page }),
      );
      const invoice = installationInvoiceForSubscription(invoices, subscriptionId);
      if (invoice?.id) {
        await goto(`/pay/${invoice.id}`);
        return;
      }
      await goto('/v2/dashboard/invoices');
    } catch {
      await goto('/v2/dashboard/invoices');
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
      if (!TrackerModalComponent) {
        const { TrackerModalComponent: DashboardServicesTrackerModalComponent } =
          await loadDashboardServicesTrackerModal();
        TrackerModalComponent = DashboardServicesTrackerModalComponent;
      }
      const [res, invoices] = await Promise.all([
        api.customers.portal.installationTracker(sub.id),
        fetchAllRows<Invoice>((page, per_page) =>
          api.payment.listCustomerPackageInvoices({ page, per_page }),
        ).catch(() => [] as Invoice[]),
      ]);
      const trackerRes = res as CustomerPortalInstallationTrackerResponse;
      trackerInvoice = installationInvoiceForSubscription(invoices, sub.id);
      trackerSub = trackerRes.subscription;
      trackerWo = trackerRes.work_order;
      trackerReschedule = trackerRes.reschedule_request;
      rescheduleAt = toLocalInputValue(trackerRes.work_order?.scheduled_at || null);
    } catch (e: any) {
      trackerError = e?.message || 'Gagal memuat pelacak instalasi';
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
      toast.error('Pilih jadwal instalasi baru.');
      return;
    }
    const iso = new Date(value).toISOString();
    if (!Number.isFinite(new Date(iso).getTime())) {
      toast.error('Tanggal dan waktu tidak valid.');
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
      toast.success('Permintaan reschedule dikirim.');
      await refreshAll();
    } catch (e: any) {
      toast.error(e?.message || 'Gagal mengirim permintaan reschedule.');
    } finally {
      rescheduleBusy = false;
    }
  }


  const totalPages = $derived(Math.max(1, Math.ceil(totalCount / pageSize)));
</script>
<PortalShell title="Layanan">
  <PageHeader
    title="Layanan Saya"
    eyebrow="Portal"
    desc="Semua layanan yang pernah Anda pesan."
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" disabled={loading} onclick={() => void refreshAll()}>
        Segarkan
      </Button>
      <Button variant="ghost" icon="receipt" onclick={() => goto('/v2/dashboard/invoices')}>
        Tagihan
      </Button>
      <Button icon="plus" onclick={() => goto('/v2/dashboard/services/order')}>
        Pesan Layanan
      </Button>
    {/snippet}
  </PageHeader>

  {#if loadError}
    <div class="banner-bad">
      <span>{loadError}</span>
      <Button variant="ghost" size="sm" onclick={() => void refreshAll()}>Coba lagi</Button>
    </div>
  {/if}

  <div class="mb-4 grid grid-cols-2 gap-3 lg:grid-cols-4">
    <button
      type="button"
      class="kpi-filter {statusFilter === 'all' ? 'kpi-filter-active' : ''}"
      onclick={() => setStatusFilter('all')}
    >
      <span class="kpi-label">Total Layanan</span>
      <span class="kpi-val">{stats.total}</span>
    </button>
    <button
      type="button"
      class="kpi-filter {statusFilter === 'active' ? 'kpi-filter-active' : ''}"
      onclick={() => setStatusFilter('active')}
    >
      <span class="kpi-label">Aktif</span>
      <span class="kpi-val ok">{stats.active}</span>
    </button>
    <button
      type="button"
      class="kpi-filter {statusFilter === 'pending_installation' ? 'kpi-filter-active' : ''}"
      onclick={() => setStatusFilter('pending_installation')}
    >
      <span class="kpi-label">Menunggu Instalasi</span>
      <span class="kpi-val warn">{stats.pendingInstallation}</span>
    </button>
    <button
      type="button"
      class="kpi-filter {statusFilter === 'needs_attention' ? 'kpi-filter-active' : ''}"
      onclick={() => setStatusFilter('needs_attention')}
    >
      <span class="kpi-label">Perlu Perhatian</span>
      <span class="kpi-val bad">{stats.needsAttention}</span>
    </button>
  </div>

  <Card title="Layanan Terpesan" padded={false}>
    {#snippet aside()}
      <div class="flex flex-wrap items-center gap-2">
        <div class="chip-filter">
          <span>Filter: {activeFilterLabel(statusFilter)}</span>
          {#if statusFilter !== 'all'}
            <Button variant="ghost" size="sm" icon="close" onclick={clearFilter}>Bersihkan</Button>
          {/if}
        </div>
        <div class="chip-filter">
          <span>Urut:</span>
          <select
            class="sort-select"
            value={sortBy}
            onchange={(e) => handleTableSort((e.target as HTMLSelectElement).value)}
          >
            <option value="updated_at">Diperbarui</option>
            <option value="price">Harga</option>
            <option value="status">Status</option>
            <option value="location_label">Lokasi</option>
            <option value="package_name">Layanan</option>
          </select>
          <select
            class="sort-select"
            value={sortDirection}
            onchange={(e) => {
              sortDirection = (e.target as HTMLSelectElement).value as 'asc' | 'desc';
              page = 0;
              void loadData();
            }}
          >
            <option value="desc">Terbaru</option>
            <option value="asc">Terlama</option>
          </select>
        </div>
      </div>
    {/snippet}

    {#if loading}
      <p class="px-4 py-6 text-sm text-ink-500">Memuat layanan…</p>
    {:else if subscriptions.length === 0}
      <div class="px-4 py-8 text-center">
        {#if totalCount === 0}
          <p class="mb-1 font-medium">Belum ada layanan terpesan.</p>
          <p class="mb-4 text-sm text-ink-500">Pesan layanan baru untuk mulai berlangganan.</p>
          <Button icon="plus" onclick={() => goto('/v2/dashboard/services/order')}>Pesan Layanan</Button>
        {:else}
          <p class="mb-1 font-medium">Tidak ada layanan untuk filter ini.</p>
          <p class="mb-4 text-sm text-ink-500">Ubah atau bersihkan filter.</p>
          <Button variant="ghost" onclick={clearFilter}>Tampilkan Semua</Button>
        {/if}
      </div>
    {:else}
      <DataTable
        rows={subscriptions}
        columns={[
          { key: 'package_name', label: 'Layanan',  },
          { key: 'status', label: 'Status',  },
          { key: 'location_label', label: 'Lokasi',  },
          { key: 'billing_cycle', label: 'Siklus' },
          { key: 'price', label: 'Harga', align: 'right',  },
          { key: 'updated_at', label: 'Diperbarui',  },
          { key: 'actions', label: 'Aksi', align: 'right' },
        ]}
        emptyTitle="Belum ada layanan"
      >
        {#snippet cell(row: CustomerSubscriptionView, col: Column)}
          {#if col.key === 'package_name'}
            <div>
              <span class="block font-medium">{row.package_name || row.package_id}</span>
              {#if showRescheduleInfo(row.status) && rescheduleStatusMeta(row.latest_reschedule_status)}
                <Badge
                  label={rescheduleStatusMeta(row.latest_reschedule_status)!.label}
                  tone={rescheduleStatusMeta(row.latest_reschedule_status)!.tone === 'approved'
                    ? 'positive'
                    : rescheduleStatusMeta(row.latest_reschedule_status)!.tone === 'rejected'
                      ? 'negative'
                      : 'warning'}
                />
              {/if}
            </div>
          {:else if col.key === 'status'}
            {@const status = serviceStatusMeta(row.status, row.starts_at, row.can_request_reopen)}
            <Badge tone={status.tone} label={status.label} />
          {:else if col.key === 'location_label'}
            {row.location_label || row.location_id}
          {:else if col.key === 'billing_cycle'}
            {billingCycleLabel(row.billing_cycle)}
          {:else if col.key === 'price'}
            <strong>{formatCurrency(Number(row.price || 0), row.currency_code)}</strong>
          {:else if col.key === 'updated_at'}
            {formatDate(row.updated_at)}
          {:else if col.key === 'actions'}
            <div class="flex flex-wrap justify-end gap-1">
              {#if row.can_request_reopen}
                <Button
                  variant="ghost"
                  size="sm"
                  loading={reopeningId === row.id}
                  onclick={() => requestReopen(row)}
                >
                  {reopeningId === row.id ? 'Membuka…' : 'Buka Ulang'}
                </Button>
              {/if}
              <Button variant="ghost" size="sm" onclick={() => openSubscriptionInvoice(row.id)}>
                {invoiceActionLabel(row.status, row.starts_at)}
              </Button>
              {#if canTrackInstallation(row)}
                <Button variant="secondary" size="sm" icon="pin" onclick={() => openTracker(row)}>
                  Lacak Instalasi
                </Button>
              {/if}
            </div>
          {/if}
        {/snippet}
      </DataTable>

      {#if totalCount > pageSize}
        <div class="pager">
          <span class="text-xs text-ink-500">Hal {page + 1} / {totalPages}</span>
          <div class="flex gap-1">
            <Button
              variant="ghost"
              size="sm"
              icon="chevronLeft"
              disabled={loading || page <= 0}
              onclick={() => {
                if (loading || page <= 0) return;
                page -= 1;
                void loadData();
              }}>Sebelumnya</Button
            >
            <Button
              variant="ghost"
              size="sm"
              icon="chevronRight"
              disabled={loading || page >= totalPages - 1}
              onclick={() => {
                if (loading || page >= totalPages - 1) return;
                page += 1;
                void loadData();
              }}>Berikutnya</Button
            >
          </div>
        </div>
      {/if}
    {/if}
  </Card>
</PortalShell>

{#if trackerOpen}
  {#if TrackerModalComponent}
    <TrackerModalComponent
      {trackerLoading}
      {trackerError}
      {trackerSub}
      {trackerWo}
      {trackerReschedule}
      {trackerInvoice}
      bind:rescheduleAt
      bind:rescheduleReason
      {rescheduleBusy}
      {tt}
      {formatDate}
      {formatCurrency}
      {invoiceActionLabel}
      {invoiceStatusTone}
      {stepState}
      {showRescheduleInfo}
      {rescheduleStatusMeta}
      {canRequestReschedule}
      onClose={closeTracker}
      onOpenTrackerInvoice={() =>
        trackerInvoice?.id
          ? goto(`/pay/${trackerInvoice.id}`)
          : openSubscriptionInvoice(trackerSub?.id || '')}
      onSubmitReschedule={submitReschedule}
    />
  {:else}
    <div class="tracker-loading">
      <p class="text-sm text-ink-500">Memuat pelacak instalasi…</p>
    </div>
  {/if}
{/if}

<style>
  .banner-bad {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    border-radius: 0.75rem;
    border: 1px solid #fecaca;
    background: #fef2f2;
    color: #991b1b;
    padding: 0.7rem 1rem;
    margin-bottom: 1rem;
    font-size: 0.875rem;
  }
  .kpi-filter {
    appearance: none;
    text-align: left;
    width: 100%;
    background: #fff;
    border: 1px solid var(--ink-200, #e7e5e4);
    border-radius: 0.75rem;
    padding: 0.8rem 1rem;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .kpi-filter-active {
    border-color: var(--brand, #2563eb);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--brand, #2563eb) 22%, transparent);
  }
  .kpi-label {
    font-size: 0.68rem;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink-500, #78716c);
  }
  .kpi-val {
    font-size: 1.3rem;
    font-weight: 750;
    color: var(--ink-900, #1c1917);
  }
  .kpi-val.ok { color: #16a34a; }
  .kpi-val.warn { color: #d97706; }
  .kpi-val.bad { color: #dc2626; }
  .chip-filter {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    border-radius: 9999px;
    border: 1px solid var(--ink-200, #e7e5e4);
    background: #fafaf9;
    padding: 0.25rem 0.75rem;
    font-size: 0.72rem;
    color: var(--ink-500, #78716c);
  }
  .pager {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 16px;
    border-top: 1px solid var(--ink-100, #f5f5f4);
    flex-wrap: wrap;
  }
  .sort-select {
    appearance: none;
    border: none;
    background: transparent;
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--ink-700, #44403c);
    cursor: pointer;
    padding: 0 0.2rem;
  }
  .tracker-loading {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.7);
  }
</style>
