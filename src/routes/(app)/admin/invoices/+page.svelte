<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { api, type Invoice } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import { toast } from '$lib/stores/toast';
  import { appSettings } from '$lib/stores/settings';
  import { formatDate } from '$lib/utils/date';
  import { formatMoney } from '$lib/utils/money';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { can, user, tenant } from '$lib/stores/auth';
  import { getAdminBillingNavigation } from '$lib/utils/adminBillingNavigation';
  import { findCustomerPackageInvoiceRelation } from '$lib/utils/customerPackageInvoice';

  let invoices = $state<Invoice[]>([]);
  let loading = $state(true);
  let creating = $state(false);
  let bulkGenerating = $state(false);
  let selectedIds = $state<Set<string>>(new Set());
  let bulkSending = $state(false);
  let error = $state('');
  let statusFilter = $state<'all' | 'pending' | 'verification_pending' | 'paid' | 'failed'>('all');
  let dateFrom = $state('');
  let dateTo = $state('');
  let invoiceSortBy = $state<'invoice_number' | 'description' | 'amount' | 'status' | 'due_date'>(
    'due_date',
  );
  let invoiceSortDirection = $state<'asc' | 'desc'>('desc');
  let selectedCustomerId = $state('');
  let selectedSubscriptionId = $state('');
  let subscriptionOptions = $state<
    Array<{ id: string; customerId: string; label: string; status: string }>
  >([]);
  let customers = $state<Array<{ id: string; name: string }>>([]);
  const billingNav = $derived.by(() =>
    getAdminBillingNavigation({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const adminHomePath = $derived(`${billingNav.tenantPrefix}/admin`);
  const collectionsPath = $derived(billingNav.collectionsPath);
  const customerBasePath = $derived(`${billingNav.tenantPrefix}/admin/customers`);

  const columns = $derived.by(() => [
    {
      key: 'select',
      label: '',
      width: '36px',
      align: 'center' as const,
    },
    {
      key: 'invoice_number',
      label: $t('admin.package_invoices.list.columns.invoice_number') || 'Invoice #',
      sortable: true,
    },
    {
      key: 'description',
      label: $t('admin.package_invoices.list.columns.description') || 'Description',
      sortable: true,
    },
    {
      key: 'amount',
      label: $t('admin.package_invoices.list.columns.amount') || 'Amount',
      sortable: true,
    },
    {
      key: 'status',
      label: $t('admin.package_invoices.list.columns.status') || 'Status',
      sortable: true,
    },
    {
      key: 'due_date',
      label: $t('admin.package_invoices.list.columns.due_date') || 'Due Date',
      sortable: true,
    },
    { key: 'actions', label: '', align: 'right' as const },
  ]);

  const filteredSubscriptions = $derived(
    subscriptionOptions.filter((s) => s.customerId === selectedCustomerId),
  );

  const filteredInvoices = $derived.by(() => {
    return invoices.filter((inv) => {
      if (statusFilter !== 'all' && inv.status !== statusFilter) return false;
      const refDateRaw = inv.created_at || inv.due_date;
      const refDate = new Date(refDateRaw);
      if (Number.isNaN(refDate.getTime())) return false;

      if (dateFrom) {
        const from = new Date(`${dateFrom}T00:00:00`);
        if (refDate < from) return false;
      }
      if (dateTo) {
        const to = new Date(`${dateTo}T23:59:59`);
        if (refDate > to) return false;
      }
      return true;
    });
  });

  const invoiceStats = $derived.by(() => {
    const source = filteredInvoices;
    const now = Date.now();
    const overdue = source.filter((inv) => {
      if (inv.status === 'paid') return false;
      const due = new Date(inv.due_date).getTime();
      return Number.isFinite(due) && due < now;
    }).length;
    const pending = source.filter((inv) => inv.status === 'pending').length;
    const verificationPending = source.filter(
      (inv) => inv.status === 'verification_pending',
    ).length;
    const paid = source.filter((inv) => inv.status === 'paid').length;
    return {
      total: source.length,
      paid,
      pending,
      verificationPending,
      overdue,
    };
  });
  const actionableInvoices = $derived(invoiceStats.pending + invoiceStats.verificationPending);

  onMount(() => {
    if (!$can('read', 'billing') && !$can('manage', 'billing')) {
      goto('/unauthorized');
      return;
    }
    Promise.all([loadInvoices(), loadSubscriptionOptions()]);
  });

  async function loadInvoices() {
    loading = true;
    try {
      invoices = await api.payment.listCustomerPackageInvoices({
        sort_by: invoiceSortBy,
        sort_dir: invoiceSortDirection,
      });
    } catch (e: any) {
      error = e.toString();
      toast.error(
        get(t)('admin.package_invoices.list.toasts.load_failed') ||
          'Failed to load customer service invoices',
      );
    } finally {
      loading = false;
    }
  }

  async function loadSubscriptionOptions() {
    try {
      const customerRes = await api.customers.list({ page: 1, perPage: 200 });
      customers = (customerRes.data || []).map((c) => ({ id: c.id, name: c.name }));

      const options = (await api.customers.subscriptions.listOptions({ limit: 2000 })) || [];
      subscriptionOptions = options.map((sub) => ({
        id: sub.id,
        customerId: sub.customer_id,
        status: sub.status,
        label: `${sub.customer_name || 'Customer'} - ${sub.package_name || 'Package'} (${sub.billing_cycle})`,
      }));
    } catch (e: any) {
      toast.error(
        e?.message ||
          get(t)('admin.package_invoices.list.toasts.load_subscriptions_failed') ||
          'Failed to load customer subscriptions',
      );
    }
  }

  async function createInvoiceFromSubscription() {
    if (!selectedSubscriptionId || creating) return;
    creating = true;
    try {
      const inv = await api.payment.createInvoiceForCustomerSubscription(selectedSubscriptionId);
      toast.success(
        get(t)('admin.package_invoices.list.toasts.created') || 'Customer service invoice created',
      );
      selectedSubscriptionId = '';
      await loadInvoices();
      await goto(`/pay/${inv.id}`);
    } catch (e: any) {
      toast.error(
        e?.message ||
          get(t)('admin.package_invoices.list.toasts.create_failed') ||
          'Failed to create invoice',
      );
    } finally {
      creating = false;
    }
  }

  async function generateDueInvoicesBulk() {
    if (bulkGenerating) return;
    bulkGenerating = true;
    try {
      const res = await api.payment.generateDueCustomerPackageInvoices();
      toast.success(
        (get(t)('admin.package_invoices.list.toasts.bulk_generated') || 'Bulk generated') +
          `: ${res.created_count} created, ${res.skipped_count} skipped, ${res.failed_count} failed`,
      );
      await loadInvoices();
    } catch (e: any) {
      toast.error(
        e?.message ||
          get(t)('admin.package_invoices.list.toasts.bulk_generate_failed') ||
          'Failed to generate due invoices',
      );
    } finally {
      bulkGenerating = false;
    }
  }

  // ─── Bulk-send invoice (Phase 4) ──────────────────────────────────────────
  function toggleSelected(id: string) {
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selectedIds = next;
  }

  function toggleSelectAllVisible() {
    const ids = filteredInvoices.map((i: Invoice) => i.id);
    const allSelected = ids.length > 0 && ids.every((id: string) => selectedIds.has(id));
    selectedIds = allSelected ? new Set() : new Set(ids);
  }

  async function bulkSendSelectedInvoices() {
    if (bulkSending) return;
    if (selectedIds.size === 0) {
      toast.error(
        get(t)('admin.package_invoices.list.toasts.bulk_send_no_selection') ||
          'Pilih minimal satu invoice',
      );
      return;
    }
    const ids = Array.from(selectedIds);
    const confirmMsg =
      (get(t)('admin.package_invoices.list.actions.bulk_send_confirm') ||
        'Kirim {count} invoice ke email pelanggan? (Email + Notifikasi in-app)').replace(
        '{count}',
        String(ids.length),
      );
    if (typeof window !== 'undefined' && !window.confirm(confirmMsg)) return;

    bulkSending = true;
    try {
      const res = await api.payment.bulkSendInvoices({
        invoice_ids: ids,
        attach_pdf: true,
      });
      const summaryHeader =
        get(t)('admin.package_invoices.list.toasts.bulk_sent') || 'Bulk send result';
      const summaryStats = (
        get(t)('admin.package_invoices.list.toasts.bulk_sent_stats') ||
        '{sent} terkirim, {skipped} dilewati, {failed} gagal'
      )
        .replace('{sent}', String(res.sent_count))
        .replace('{skipped}', String(res.skipped_count))
        .replace('{failed}', String(res.failed_count));
      const summary = `${summaryHeader}: ${summaryStats}`;
      if (res.failed_count > 0) {
        const firstFail = res.items.find((it) => it.status === 'failed');
        const detail = firstFail?.error || firstFail?.reason || '';
        toast.error(`${summary}${detail ? ` — ${detail}` : ''}`);
      } else {
        toast.success(summary);
      }
      selectedIds = new Set();
    } catch (e: any) {
      toast.error(
        e?.message ||
          get(t)('admin.package_invoices.list.toasts.bulk_send_failed') ||
          'Gagal mengirim faktur',
      );
    } finally {
      bulkSending = false;
    }
  }

  function openInvoiceDetail(id: string) {
    const basePath =
      typeof window !== 'undefined'
        ? window.location.pathname.replace(/\/$/, '')
        : '/admin/invoices';
    void goto(`${basePath}/${id}`);
  }

  function openCustomerDetail(customerId: string) {
    void goto(`${customerBasePath}/${customerId}`);
  }

  function formatCurrency(amount: number, currency?: string) {
    return formatMoney(amount, { currency });
  }

  function statusLabel(status: string) {
    const map: Record<string, string> = {
      pending: get(t)('admin.package_invoices.statuses.pending') || 'Pending',
      verification_pending:
        get(t)('admin.package_invoices.statuses.verification_pending') || 'Verification pending',
      paid: get(t)('admin.package_invoices.statuses.paid') || 'Paid',
      failed: get(t)('admin.package_invoices.statuses.failed') || 'Failed',
    };
    return map[status] || status;
  }

  function clearFilters() {
    statusFilter = 'all';
    dateFrom = '';
    dateTo = '';
  }

  function handleInvoiceSort(key: string) {
    const allowed: Array<'invoice_number' | 'description' | 'amount' | 'status' | 'due_date'> = [
      'invoice_number',
      'description',
      'amount',
      'status',
      'due_date',
    ];
    if (!allowed.includes(key as any)) return;
    const typed = key as (typeof allowed)[number];
    if (invoiceSortBy === typed) {
      invoiceSortDirection = invoiceSortDirection === 'asc' ? 'desc' : 'asc';
      void loadInvoices();
      return;
    }
    invoiceSortBy = typed;
    invoiceSortDirection = typed === 'amount' || typed === 'due_date' ? 'desc' : 'asc';
    void loadInvoices();
  }

  function goToBillingLogs() {
    void goto(collectionsPath);
  }
</script>

<div class="page-container fade-in">
  <nav class="breadcrumb" aria-label="Breadcrumb">
    <button class="crumb-link" type="button" onclick={() => goto(adminHomePath)}>
      {$t('sidebar.overview') || 'Overview'}
    </button>
    <span class="crumb-sep">/</span>
    <span class="crumb-current">{$t('sidebar.billing') || 'Billing'}</span>
  </nav>

  <div class="page-header">
    <div class="header-content">
      <span class="page-eyebrow">
        {$t('admin.package_invoices.list.eyebrow') || 'Customer billing'}
      </span>
      <h1>{$t('admin.package_invoices.list.title') || 'Billing'}</h1>
      <p class="subtitle">Tagihan, collection, dan invoice pelanggan.</p>
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" onclick={goToBillingLogs}>
        <Icon name="activity" size={16} />
        <span>{$t('sidebar.collections') || 'Collections'}</span>
      </button>
      {#if selectedIds.size === 0 && filteredInvoices.length > 0}
        <button
          class="btn btn-secondary"
          onclick={toggleSelectAllVisible}
          title={$t('admin.package_invoices.list.actions.select_all_visible_title') ||
            'Pilih semua invoice di halaman ini'}
        >
          <Icon name="check-square" size={16} />
          <span>{$t('admin.package_invoices.list.actions.select_all_visible') || 'Pilih Semua'}</span>
        </button>
      {/if}
      {#if selectedIds.size > 0}
        <button
          class="btn btn-secondary"
          onclick={() => (selectedIds = new Set())}
          disabled={bulkSending}
          title={$t('admin.package_invoices.list.actions.clear_selection') || 'Batal pilih'}
        >
          <Icon name="x" size={16} />
          <span>{$t('common.cancel') || 'Batal'}</span>
        </button>
        <button
          class="btn btn-primary"
          onclick={bulkSendSelectedInvoices}
          disabled={bulkSending}
          title={$t('admin.package_invoices.list.actions.bulk_send_title') ||
            'Kirim invoice terpilih via email + notifikasi'}
        >
          <Icon name="send" size={16} />
          <span>
            {bulkSending
              ? $t('admin.package_invoices.list.actions.bulk_sending') || 'Mengirim...'
              : (
                  $t('admin.package_invoices.list.actions.bulk_send_count') ||
                  'Kirim {count} Invoice'
                ).replace('{count}', String(selectedIds.size))}
          </span>
        </button>
      {/if}
      <button class="btn btn-primary" onclick={generateDueInvoicesBulk} disabled={bulkGenerating}>
        <Icon name="layers" size={16} />
        <span
          >{bulkGenerating
            ? $t('admin.package_invoices.list.actions.bulk_generating') || 'Generating...'
            : $t('admin.package_invoices.list.actions.generate_due_bulk') ||
              'Generate Due Invoices'}</span
        >
      </button>
      <button class="btn btn-secondary" onclick={loadInvoices}>
        <Icon name="refresh-cw" size={18} />
        <span>{$t('common.refresh') || 'Refresh'}</span>
      </button>
    </div>
  </div>

  <div class="workspace-grid">
    <article class="workspace-card">
      <span class="workspace-card__label">
        {$t('admin.package_invoices.list.workspace.action_needed') || 'Need action'}
      </span>
      <strong class="workspace-card__value tone-pending">{actionableInvoices}</strong>
      <p>
        {$t('admin.package_invoices.list.workspace.action_needed_desc') ||
          'Pending and verification-pending invoices still need follow-up.'}
      </p>
    </article>
    <article class="workspace-card">
      <span class="workspace-card__label">
        {$t('admin.package_invoices.list.workspace.overdue_now') || 'Overdue now'}
      </span>
      <strong class="workspace-card__value tone-overdue">{invoiceStats.overdue}</strong>
      <p>
        {$t('admin.package_invoices.list.workspace.overdue_now_desc') ||
          'Focus collection reminders and service actions on these accounts first.'}
      </p>
    </article>
    <article class="workspace-card">
      <span class="workspace-card__label">
        {$t('admin.package_invoices.list.workspace.queue_title') || 'Invoice queue'}
      </span>
      <strong class="workspace-card__value">{invoiceStats.total}</strong>
      <p>Pantau invoice aktif tanpa pindah halaman.</p>
    </article>
  </div>

  <section class="section-block">
    <div class="section-heading">
      <div>
        <h2>
          {$t('admin.package_invoices.list.sections.manual_title') || 'Manual billing action'}
        </h2>
        <p>Pilih subscription untuk membuat invoice manual.</p>
      </div>
    </div>

    <div class="create-row">
      <select bind:value={selectedCustomerId} class="select-input">
        <option value="">
          {$t('admin.package_invoices.list.fields.select_customer') || 'Select customer'}
        </option>
        {#each customers as customer}
          <option value={customer.id}>{customer.name}</option>
        {/each}
      </select>

      <select
        bind:value={selectedSubscriptionId}
        class="select-input"
        disabled={!selectedCustomerId}
      >
        <option value="">
          {$t('admin.package_invoices.list.fields.select_subscription') || 'Select subscription'}
        </option>
        {#each filteredSubscriptions as sub}
          <option value={sub.id}>{sub.label} - {sub.status}</option>
        {/each}
      </select>

      <button
        class="btn btn-primary"
        onclick={createInvoiceFromSubscription}
        disabled={!selectedSubscriptionId || creating}
      >
        <Icon name="plus" size={16} />
        <span
          >{creating
            ? $t('admin.package_invoices.list.actions.creating') || 'Creating...'
            : $t('admin.package_invoices.list.actions.generate_invoice') ||
              'Generate Invoice'}</span
        >
      </button>
    </div>
  </section>

  <section class="section-block">
    <div class="section-heading">
      <div>
        <h2>{$t('admin.package_invoices.list.sections.summary_title') || 'Billing overview'}</h2>
        <p>Ringkasan status pembayaran.</p>
      </div>
    </div>

    <div class="stats-grid">
      <article class="stat-card">
        <span class="stat-label">{$t('admin.package_invoices.list.stats.total') || 'Total'}</span>
        <strong class="stat-value">{invoiceStats.total}</strong>
      </article>
      <article class="stat-card">
        <span class="stat-label">{$t('admin.package_invoices.list.stats.paid') || 'Paid'}</span>
        <strong class="stat-value tone-paid">{invoiceStats.paid}</strong>
      </article>
      <article class="stat-card">
        <span class="stat-label"
          >{$t('admin.package_invoices.list.stats.pending') || 'Pending'}</span
        >
        <strong class="stat-value tone-pending"
          >{invoiceStats.pending + invoiceStats.verificationPending}</strong
        >
      </article>
      <article class="stat-card">
        <span class="stat-label"
          >{$t('admin.package_invoices.list.stats.overdue') || 'Overdue'}</span
        >
        <strong class="stat-value tone-overdue">{invoiceStats.overdue}</strong>
      </article>
    </div>
  </section>

  <section class="section-block">
    <div class="section-heading">
      <div>
        <h2>{$t('admin.package_invoices.list.sections.queue_title') || 'Invoice queue'}</h2>
        <p>Filter dan cek invoice pelanggan.</p>
      </div>
    </div>

    <div class="card content-card">
      {#if error}
        <div class="alert alert-error">{error}</div>
      {/if}

      <div class="filter-row">
        <select bind:value={statusFilter} class="select-input">
          <option value="all">
            {$t('admin.package_invoices.list.filters.all_status') || 'All status'}
          </option>
          <option value="pending"
            >{$t('admin.package_invoices.list.filters.pending') || 'Pending'}</option
          >
          <option value="verification_pending">
            {$t('admin.package_invoices.list.filters.verification_pending') ||
              'Verification pending'}
          </option>
          <option value="paid">{$t('admin.package_invoices.list.filters.paid') || 'Paid'}</option>
          <option value="failed"
            >{$t('admin.package_invoices.list.filters.failed') || 'Failed'}</option
          >
        </select>

        <input
          class="select-input"
          type="date"
          bind:value={dateFrom}
          title={$t('admin.package_invoices.list.filters.created_from') || 'Created from'}
        />
        <input
          class="select-input"
          type="date"
          bind:value={dateTo}
          title={$t('admin.package_invoices.list.filters.created_to') || 'Created to'}
        />

        <button class="btn btn-secondary btn-sm" onclick={clearFilters}>
          {$t('admin.package_invoices.list.filters.clear') || 'Clear'}
        </button>
      </div>

      <Table
        {loading}
        data={filteredInvoices}
        {columns}
        searchable={true}
        searchPlaceholder={$t('admin.package_invoices.list.search_placeholder') ||
          'Search customer service invoices...'}
        sortKey={invoiceSortBy}
        sortDirection={invoiceSortDirection}
        onsort={handleInvoiceSort}
      >
        {#snippet cell({ item, column })}
          {#if column.key === 'select'}
            <input
              type="checkbox"
              checked={selectedIds.has(item.id)}
              onchange={() => toggleSelected(item.id)}
              aria-label={$t('admin.package_invoices.list.actions.select_invoice') ||
                'Pilih invoice'}
            />
          {:else if column.key === 'amount'}
            {formatCurrency(item.amount, item.currency_code)}
          {:else if column.key === 'status'}
            <span class="status-pill {item.status}">{statusLabel(item.status)}</span>
          {:else if column.key === 'due_date'}
            {formatDate(item[column.key], { timeZone: $appSettings.app_timezone })}
          {:else if column.key === 'actions'}
            <div class="actions">
              {#if findCustomerPackageInvoiceRelation(item, subscriptionOptions)?.customerId}
                <button
                  type="button"
                  class="action-btn"
                  title={$t('admin.package_invoices.list.actions.open_customer') || 'Open customer'}
                  aria-label={$t('admin.package_invoices.list.actions.open_customer') ||
                    'Open customer'}
                  onclick={() =>
                    openCustomerDetail(
                      findCustomerPackageInvoiceRelation(item, subscriptionOptions)?.customerId ||
                        '',
                    )}
                >
                  <Icon name="users" size={18} />
                </button>
              {/if}
              <button
                type="button"
                class="action-btn"
                title={$t('admin.package_invoices.list.actions.view_details') || 'View Details'}
                aria-label={$t('admin.package_invoices.list.actions.view_details') ||
                  'View Details'}
                onclick={() => openInvoiceDetail(item.id)}
              >
                <Icon name="eye" size={18} />
              </button>
            </div>
          {:else}
            {item[column.key]}
          {/if}
        {/snippet}
      </Table>
    </div>
  </section>
</div>

<style>
  .page-container {
    padding: clamp(1rem, 3vw, 2rem);
    max-width: 1520px;
    margin: 0 auto;
  }
  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.75rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
    flex-wrap: wrap;
  }
  .crumb-link {
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    padding: 0;
    cursor: pointer;
  }
  .crumb-link:hover {
    color: var(--text-primary);
    text-decoration: underline;
  }
  .crumb-current {
    color: var(--text-primary);
    font-weight: 600;
  }
  .crumb-sep {
    color: var(--text-tertiary);
  }
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1rem;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .page-eyebrow {
    display: inline-flex;
    margin-bottom: 0.35rem;
    font-size: 0.78rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-primary);
  }
  .header-actions {
    display: flex;
    gap: 0.55rem;
    align-items: center;
    flex-wrap: wrap;
  }
  .workspace-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.75rem;
    margin-bottom: 0.9rem;
  }
  .workspace-card {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    padding: 0.85rem 0.95rem;
  }
  .workspace-card p {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.45;
    font-size: 0.86rem;
  }
  .workspace-card__label {
    display: inline-flex;
    margin-bottom: 0.45rem;
    color: var(--text-secondary);
    font-size: 0.78rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .workspace-card__value {
    display: block;
    margin-bottom: 0.35rem;
    font-size: 1.25rem;
    line-height: 1;
    color: var(--text-primary);
  }
  .section-block {
    margin-bottom: 0.9rem;
  }
  .section-heading {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 1rem;
    margin-bottom: 0.6rem;
  }
  .section-heading h2 {
    margin: 0 0 0.2rem;
    font-size: 1rem;
    color: var(--text-primary);
  }
  .section-heading p {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.4;
    font-size: 0.88rem;
  }
  .create-row {
    display: grid;
    grid-template-columns: minmax(220px, 1fr) minmax(320px, 1.6fr) auto;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
    margin-bottom: 1rem;
  }
  .stat-card {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    padding: 0.75rem 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .stat-label {
    font-size: 0.8rem;
    color: var(--text-secondary);
  }
  .stat-value {
    font-size: 1.25rem;
    line-height: 1.1;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }
  .tone-paid {
    color: var(--color-success);
  }
  .tone-pending {
    color: var(--color-warning);
  }
  .tone-overdue {
    color: var(--color-danger);
  }
  .select-input {
    min-height: 40px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border-radius: 8px;
    padding: 0 0.75rem;
  }
  .header-content h1 {
    font-size: 1.8rem;
    font-weight: 700;
    margin: 0 0 0.5rem;
  }
  .filter-row {
    display: grid;
    grid-template-columns: minmax(180px, 1fr) 170px 170px auto;
    gap: 0.75rem;
    padding: 0.85rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-surface);
  }
  .subtitle {
    color: var(--text-secondary);
  }
  .content-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    overflow: hidden;
  }

  .status-pill {
    padding: 0.25rem 0.6rem;
    border-radius: 12px;
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    border: 1px solid transparent;
  }
  .status-pill.pending {
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
    color: var(--color-warning);
    border-color: color-mix(in srgb, var(--color-warning) 24%, var(--border-color));
  }
  .status-pill.verification_pending {
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
    color: var(--color-warning);
    border-color: color-mix(in srgb, var(--color-warning) 24%, var(--border-color));
  }
  .status-pill.paid {
    background: var(--bg-success);
    color: var(--color-success);
    border-color: color-mix(in srgb, var(--color-success) 24%, var(--border-color));
  }
  .status-pill.failed {
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    color: var(--color-danger);
    border-color: color-mix(in srgb, var(--color-danger) 24%, var(--border-color));
  }
  .status-pill.expired {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border-color: var(--border-color);
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    align-items: center;
  }
  .action-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 6px;
  }
  .action-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 1rem;
    border-radius: 8px;
    font-weight: 600;
    cursor: pointer;
    border: none;
    text-decoration: none;
  }
  .btn-sm {
    padding: 0.4rem 0.8rem;
    font-size: 0.85rem;
  }
  .btn-primary {
    background: var(--color-primary);
    color: white;
  }
  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  @media (max-width: 768px) {
    .page-header {
      flex-direction: column;
      align-items: stretch;
    }
    .header-actions {
      width: 100%;
    }
    .header-actions .btn {
      flex: 1 1 calc(50% - 0.55rem);
      justify-content: center;
    }
    .workspace-grid {
      grid-template-columns: 1fr;
    }
    .create-row {
      grid-template-columns: 1fr;
    }
    .stats-grid {
      grid-template-columns: 1fr 1fr;
    }
    .filter-row {
      grid-template-columns: 1fr;
    }
    .actions {
      justify-content: flex-start;
    }
    .section-heading {
      align-items: stretch;
    }
    .header-content h1 {
      font-size: 1.35rem;
    }

    .content-card {
      border-radius: var(--radius-lg);
    }
  }

  @media (max-width: 640px) {
    .stats-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
