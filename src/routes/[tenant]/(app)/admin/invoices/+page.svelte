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
  import { user, tenant } from '$lib/stores/auth';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';

  let invoices = $state<Invoice[]>([]);
  let loading = $state(true);
  let creating = $state(false);
  let bulkGenerating = $state(false);
  let error = $state('');
  let statusFilter = $state<'all' | 'pending' | 'verification_pending' | 'paid' | 'failed'>('all');
  let dateFrom = $state('');
  let dateTo = $state('');
  let selectedCustomerId = $state('');
  let selectedSubscriptionId = $state('');
  let subscriptionOptions = $state<
    Array<{ id: string; customerId: string; label: string; status: string }>
  >([]);
  let customers = $state<Array<{ id: string; name: string }>>([]);
  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);
  const adminHomePath = $derived(`${tenantPrefix}/admin`);

  const columns = $derived.by(() => [
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

  onMount(() => {
    Promise.all([loadInvoices(), loadSubscriptionOptions()]);
  });

  async function loadInvoices() {
    loading = true;
    try {
      invoices = await api.payment.listCustomerPackageInvoices();
    } catch (e: any) {
      error = e.toString();
      toast.error(
        get(t)('admin.package_invoices.list.toasts.load_failed') ||
          'Failed to load customer package invoices',
      );
    } finally {
      loading = false;
    }
  }

  async function loadSubscriptionOptions() {
    try {
      const customerRes = await api.customers.list({ page: 1, perPage: 200 });
      customers = (customerRes.data || []).map((c) => ({ id: c.id, name: c.name }));

      const subResults = await Promise.all(
        customers.map(async (customer) => {
          const subRes = await api.customers.subscriptions.list(customer.id, { page: 1, per_page: 200 });
          return (subRes.data || []).map((sub) => ({
            id: sub.id,
            customerId: customer.id,
            status: sub.status,
            label: `${customer.name} - ${sub.package_name || 'Package'} (${sub.billing_cycle})`,
          }));
        }),
      );

      subscriptionOptions = subResults.flat();
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
        get(t)('admin.package_invoices.list.toasts.created') || 'Customer package invoice created',
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

  function openInvoiceDetail(id: string) {
    const basePath =
      typeof window !== 'undefined'
        ? window.location.pathname.replace(/\/$/, '')
        : '/admin/invoices';
    void goto(`${basePath}/${id}`);
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

  function goToBillingLogs() {
    const basePath =
      typeof window !== 'undefined'
        ? window.location.pathname.replace(/\/$/, '')
        : '/admin/invoices';
    void goto(`${basePath}/collection`);
  }
</script>

<div class="page-container fade-in">
  <nav class="breadcrumb" aria-label="Breadcrumb">
    <button class="crumb-link" type="button" onclick={() => goto(adminHomePath)}>
      {$t('sidebar.overview') || 'Overview'}
    </button>
    <span class="crumb-sep">/</span>
    <span class="crumb-current">{$t('sidebar.invoices') || 'Invoices'}</span>
  </nav>

  <div class="page-header">
    <div class="header-content">
      <h1>{$t('admin.package_invoices.list.title') || 'Customer Package Invoices'}</h1>
      <p class="subtitle">
        {$t('admin.package_invoices.list.subtitle') ||
          'Generate and manage invoices for customer internet packages.'}
      </p>
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" onclick={goToBillingLogs}>
        <Icon name="activity" size={16} />
        <span>{$t('admin.package_invoices.list.actions.billing_logs') || 'Billing Logs'}</span>
      </button>
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

  <div class="create-row">
    <select bind:value={selectedCustomerId} class="select-input">
      <option value="">
        {$t('admin.package_invoices.list.fields.select_customer') || 'Select customer'}
      </option>
      {#each customers as customer}
        <option value={customer.id}>{customer.name}</option>
      {/each}
    </select>

    <select bind:value={selectedSubscriptionId} class="select-input" disabled={!selectedCustomerId}>
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
          : $t('admin.package_invoices.list.actions.generate_invoice') || 'Generate Invoice'}</span
      >
    </button>
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
        <option value="pending">{$t('admin.package_invoices.list.filters.pending') || 'Pending'}</option>
        <option value="verification_pending">
          {$t('admin.package_invoices.list.filters.verification_pending') || 'Verification pending'}
        </option>
        <option value="paid">{$t('admin.package_invoices.list.filters.paid') || 'Paid'}</option>
        <option value="failed">{$t('admin.package_invoices.list.filters.failed') || 'Failed'}</option>
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
        'Search customer package invoices...'}
    >
      {#snippet cell({ item, column })}
        {#if column.key === 'amount'}
          {formatCurrency(item.amount, item.currency_code)}
        {:else if column.key === 'status'}
          <span class="status-pill {item.status}">{statusLabel(item.status)}</span>
        {:else if column.key === 'due_date'}
          {formatDate(item[column.key], { timeZone: $appSettings.app_timezone })}
        {:else if column.key === 'actions'}
          <div class="actions">
            <button
              type="button"
              class="action-btn"
              title={$t('admin.package_invoices.list.actions.view_details') || 'View Details'}
              aria-label={$t('admin.package_invoices.list.actions.view_details') || 'View Details'}
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
</div>

<style>
  .page-container {
    padding: clamp(1rem, 3vw, 2rem);
    max-width: 1200px;
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
    margin-bottom: 2rem;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .header-actions {
    display: flex;
    gap: 0.75rem;
    align-items: center;
  }
  .create-row {
    display: grid;
    grid-template-columns: 1fr 1.6fr auto;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }
  .select-input {
    min-height: 40px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
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
    grid-template-columns: 1fr 160px 160px auto;
    gap: 0.75rem;
    padding: 0.85rem;
    border-bottom: 1px solid var(--border-color);
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
    background: rgba(245, 158, 11, 0.14);
    color: var(--color-warning, #f59e0b);
    border-color: rgba(245, 158, 11, 0.22);
  }
  .status-pill.verification_pending {
    background: rgba(245, 158, 11, 0.14);
    color: var(--color-warning, #f59e0b);
    border-color: rgba(245, 158, 11, 0.22);
  }
  .status-pill.paid {
    background: rgba(16, 185, 129, 0.14);
    color: var(--color-success, #10b981);
    border-color: rgba(16, 185, 129, 0.22);
  }
  .status-pill.failed {
    background: rgba(239, 68, 68, 0.14);
    color: var(--color-danger, #ef4444);
    border-color: rgba(239, 68, 68, 0.22);
  }
  .status-pill.expired {
    background: rgba(148, 163, 184, 0.12);
    color: var(--text-secondary);
    border-color: rgba(148, 163, 184, 0.18);
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
    .create-row {
      grid-template-columns: 1fr;
    }
    .filter-row {
      grid-template-columns: 1fr;
    }

    .btn.btn-secondary {
      width: 100%;
      justify-content: center;
    }

    .header-content h1 {
      font-size: 1.35rem;
    }

    .content-card {
      border-radius: 16px;
    }
  }
</style>
