<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Invoice } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import CompactFilterToolbar from '$lib/components/superadmin/shared/CompactFilterToolbar.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import Pagination from '$lib/components/ui/Pagination.svelte';
  import { toast } from '$lib/stores/toast';
  import { appSettings } from '$lib/stores/settings';
  import { formatMoney } from '$lib/utils/money';
  import { formatDate } from '$lib/utils/date';
  import { goto } from '$app/navigation';
  import { getTenantsCached } from '$lib/stores/superadminTenantsCache';
  import { t } from 'svelte-i18n';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { get } from 'svelte/store';

  type InvoiceStatus = 'all' | 'pending' | 'paid' | 'failed' | 'verification_pending' | 'expired';

  let invoices = $state<Invoice[]>([]);
  let totalInvoices = $state(0);
  let pendingInvoices = $state(0);
  let paidInvoices = $state(0);
  let failedInvoices = $state(0);
  let page = $state(1);
  let pageSize = $state(25);
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let loading = $state(true);
  let error = $state('');

  let tenantNameById = $state<Record<string, { name: string; slug?: string }>>({});

  let search = $state('');
  let statusFilter = $state<InvoiceStatus>('all');
  let viewMode = $state<'cards' | 'table'>('cards');
  let isMobile = $state(false);
  let filtersOpen = $state(false);

  const columns = $derived.by(() => [
    {
      key: 'invoice_number',
      label: $t('superadmin.invoices.list.columns.invoice_number') || 'Invoice #',
      sortable: true,
    },
    {
      key: 'tenant',
      label: $t('superadmin.invoices.list.columns.tenant') || 'Tenant',
      sortable: true,
    },
    {
      key: 'amount',
      label: $t('superadmin.invoices.list.columns.amount') || 'Amount',
      sortable: true,
    },
    {
      key: 'status',
      label: $t('superadmin.invoices.list.columns.status') || 'Status',
      sortable: true,
    },
    {
      key: 'due_date',
      label: $t('superadmin.invoices.list.columns.due_date') || 'Due Date',
      sortable: true,
    },
    {
      key: 'created_at',
      label: $t('superadmin.invoices.list.columns.created_at') || 'Created At',
      sortable: true,
    },
    {
      key: 'actions',
      label: $t('superadmin.invoices.list.columns.actions') || 'Actions',
      align: 'right',
    },
  ]);

  const stats = $derived({
    total: totalInvoices,
    pending: pendingInvoices,
    paid: paidInvoices,
    failed: failedInvoices,
  });

  const filteredInvoices = $derived(invoices);

  /* Server-side search/status filtering is applied in loadInvoices. */
  const legacyFilteredInvoices = $derived(
    invoices.filter((inv) => {
      const q = search.trim().toLowerCase();
      const tenant =
        inv.tenant_id && tenantNameById[inv.tenant_id] ? tenantNameById[inv.tenant_id].name : '';
      const matchesSearch =
        !q ||
        inv.invoice_number.toLowerCase().includes(q) ||
        (inv.tenant_id ?? '').toLowerCase().includes(q) ||
        tenant.toLowerCase().includes(q) ||
        (inv.status ?? '').toLowerCase().includes(q);

      const matchesStatus = statusFilter === 'all' || inv.status === statusFilter;

      return matchesSearch && matchesStatus;
    }),
  );

  onMount(() => {
    let cleanup: (() => void) | undefined;

    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(max-width: 899px)');
      const sync = () => {
        isMobile = mq.matches;
      };
      sync();

      try {
        mq.addEventListener('change', sync);
        cleanup = () => mq.removeEventListener('change', sync);
      } catch {
        // Safari/older WebView fallback
        // @ts-ignore
        mq.addListener?.(sync);
        // @ts-ignore
        cleanup = () => mq.removeListener?.(sync);
      }
    }

    void loadInvoices();
    return cleanup;
  });

  $effect(() => {
    if (isMobile && viewMode === 'table') viewMode = 'cards';
  });

  async function loadInvoices(targetPage = page) {
    loading = true;
    try {
      error = '';
      const [invoicesRes, tenantsRes] = await Promise.all([
        api.payment.listAllInvoicesPage({
          page: targetPage,
          per_page: pageSize,
          search: search.trim() || undefined,
          status: statusFilter === 'all' ? undefined : statusFilter,
        }),
        getTenantsCached()
          .then((data) => ({ data, total: data.length }))
          .catch(() => ({ data: [], total: 0 })),
      ]);

      page = targetPage;
      invoices = invoicesRes?.data || [];
      totalInvoices = invoicesRes?.total ?? 0;
      pendingInvoices = invoicesRes?.pending_total ?? 0;
      paidInvoices = invoicesRes?.paid_total ?? 0;
      failedInvoices = invoicesRes?.failed_total ?? 0;
      tenantNameById = Object.fromEntries(
        (tenantsRes.data || []).map((t: any) => [t.id, { name: t.name, slug: t.slug }]),
      );
    } catch (e: any) {
      error = extractApiErrorMessage(e, String(e || 'unknown error'));
      toast.error(
        get(t)('superadmin.invoices.list.errors.load_failed') || 'Failed to load invoices',
      );
    } finally {
      loading = false;
    }
  }

  function formatCurrency(amount: number, currency?: string) {
    return formatMoney(amount, { currency });
  }

  function tenantLabel(tenantId?: string) {
    if (!tenantId) return { name: '—', slug: '' };
    return tenantNameById[tenantId] || { name: tenantId, slug: '' };
  }

  function resetFilters() {
    search = '';
    statusFilter = 'all';
    page = 1;
    void loadInvoices(1);
  }

  async function checkStatus(id: string) {
    try {
      const status = await api.payment.checkStatus(id);
      toast.success(
        (get(t)('superadmin.invoices.list.toasts.status') || 'Invoice status: ') + status,
      );
      void loadInvoices();
    } catch (e: any) {
      toast.error(
        (get(t)('superadmin.invoices.list.errors.check_failed') || 'Failed to check status: ') +
          e.message,
      );
    }
  }
</script>

<div class="sa-invoices fade-in">
  <!-- ── Page header ── -->
  <div class="page-head">
    <div>
      <div class="crumbs">
        {$t('superadmin.invoices.crumbs.root')}
        <span class="crumb-sep">›</span>
        <b>{$t('superadmin.invoices.crumbs.invoices')}</b>
      </div>
      <h1>{$t('superadmin.invoices.list.title')}</h1>
      <p class="subtitle">{$t('superadmin.invoices.list.subtitle')}</p>
    </div>
    <div class="head-actions">
      <button class="btn ghost" type="button" onclick={() => loadInvoices(1)}><Icon name="refresh-cw" size={14} /> {$t('common.refresh')}</button>
    </div>
  </div>

  <div class="stats-row" aria-label={$t('superadmin.invoices.aria.stats')}>
    <button
      class="stat-btn"
      class:active={statusFilter === 'all'}
      type="button"
      onclick={() => (statusFilter = 'all')}
    >
      <div class="stat-title">
        {$t('superadmin.invoices.list.filters.all')}
      </div>
      <div class="stat-value">{stats.total}</div>
    </button>
    <button
      class="stat-btn"
      class:active={statusFilter === 'pending'}
      type="button"
      onclick={() => (statusFilter = 'pending')}
    >
      <div class="stat-title">
        {$t('superadmin.invoices.list.filters.pending')}
      </div>
      <div class="stat-value">{stats.pending}</div>
    </button>
    <button
      class="stat-btn"
      class:active={statusFilter === 'paid'}
      type="button"
      onclick={() => (statusFilter = 'paid')}
    >
      <div class="stat-title">
        {$t('superadmin.invoices.list.filters.paid')}
      </div>
      <div class="stat-value">{stats.paid}</div>
    </button>
    <button
      class="stat-btn"
      class:active={statusFilter === 'failed'}
      type="button"
      onclick={() => (statusFilter = 'failed')}
    >
      <div class="stat-title">
        {$t('superadmin.invoices.list.filters.failed')}
      </div>
      <div class="stat-value">{stats.failed}</div>
    </button>
  </div>

  <div class="card content-card">
    {#if error}
      <div class="alert alert-error">{error}</div>
    {/if}

    <div class="toolbar-wrapper">
      <CompactFilterToolbar
        bind:searchQuery={search}
        placeholder={$t('superadmin.invoices.list.search')}
        bind:filterPanelOpen={filtersOpen}
        activeFilterCount={statusFilter === 'all' ? 0 : 1}
        onReset={resetFilters}
        {isMobile}
        bind:viewMode
        onSearchChange={() => {
          if (searchTimer) clearTimeout(searchTimer);
          searchTimer = setTimeout(() => { page = 1; void loadInvoices(1); }, 300);
        }}
      >
        {#snippet advancedFilters()}
          <div class="toolbar-field">
            <label for="invoice-status-filter">
              {$t('superadmin.invoices.list.filters.status')}
            </label>
            <select id="invoice-status-filter" bind:value={statusFilter} onchange={() => { page = 1; void loadInvoices(1); }}>
              <option value="all">{$t('superadmin.invoices.list.filters.all') || $t('common.all') || 'All'}</option>
              <option value="pending">{$t('superadmin.invoices.list.filters.pending')}</option>
              <option value="paid">{$t('superadmin.invoices.list.filters.paid')}</option>
              <option value="failed">{$t('superadmin.invoices.list.filters.failed')}</option>
            </select>
          </div>
        {/snippet}
      </CompactFilterToolbar>
    </div>

    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>
          {$t('superadmin.invoices.list.loading')}
        </p>
      </div>
    {:else if filteredInvoices.length === 0}
      <div class="empty-grid">
        <div class="empty-icon">
          <Icon name="file-text" size={56} />
        </div>
        <h4>
          {$t('superadmin.invoices.list.empty')}
        </h4>
        <p>
          {$t('superadmin.invoices.list.empty_hint')}
        </p>
      </div>
    {:else}
      {#if viewMode === 'cards' || isMobile}
        <div
          class="invoices-grid"
          aria-label={$t('superadmin.invoices.aria.cards')}
        >
          {#each filteredInvoices as inv (inv.id)}
            <div class="invoice-card">
              <div class="invoice-top">
                <div class="invoice-title">
                  <div class="invoice-number">
                    #{inv.invoice_number}
                  </div>
                  <div class="invoice-tenant">
                    {#if inv.tenant_id}
                      <span class="tenant-name">
                        {tenantLabel(inv.tenant_id).name}
                      </span>
                      {#if tenantLabel(inv.tenant_id).slug}
                        <span class="tenant-slug muted-text">
                          {tenantLabel(inv.tenant_id).slug}
                        </span>
                      {/if}
                    {:else}
                      <span class="tenant-name">—</span>
                    {/if}
                  </div>
                </div>
                <span class="status-pill {inv.status}">
                  {inv.status}
                </span>
              </div>

              <div class="invoice-meta">
                <div class="meta-item">
                  <span class="meta-label">
                    {$t('superadmin.invoices.cards.amount')}
                  </span>
                  <span class="meta-value">
                    {formatCurrency(inv.amount, inv.currency_code)}
                  </span>
                </div>
                <div class="meta-item">
                  <span class="meta-label">
                    {$t('superadmin.invoices.cards.due')}
                  </span>
                  <span class="meta-value">
                    {formatDate(inv.due_date, { timeZone: $appSettings.app_timezone })}
                  </span>
                </div>
              </div>

              <div class="invoice-actions">
                <button
                  class="btn-icon"
                  type="button"
                  title={$t('superadmin.invoices.actions.check_status')}
                  onclick={() => checkStatus(inv.id)}
                >
                  <Icon name="refresh-cw" size={18} />
                </button>
                <button
                  class="btn-icon"
                  type="button"
                  title={$t('superadmin.invoices.actions.view_details')}
                  onclick={() => goto(`/superadmin/invoices/${inv.id}`)}
                >
                  <Icon name="eye" size={18} />
                </button>
              </div>
            </div>
          {/each}
        </div>
        <Pagination
          count={totalInvoices}
          page={page - 1}
          pageSize={pageSize}
          onchange={(p: number) => void loadInvoices(p + 1)}
          onpageSizeChange={(s: number) => { pageSize = s; void loadInvoices(1); }}
        />
      {/if}

      {#if viewMode === 'table' && !isMobile}
        <div
          class="table-wrapper"
          aria-label={$t('superadmin.invoices.aria.table')}
        >
          <Table
            data={filteredInvoices}
            {columns}
            loading={false}
            keyField="id"
            pagination={true}
            serverSide={true}
            count={totalInvoices}
            pageSize={pageSize}
            onchange={(p: number) => void loadInvoices(p + 1)}
            onpageSizeChange={(s: number) => { pageSize = s; void loadInvoices(1); }}
            mobileView="scroll"
          >
            {#snippet cell({ item, column, key })}
              {#if key === 'tenant'}
                <div class="table-tenant">
                  {#if item.tenant_id}
                    <div class="table-tenant-name">
                      {tenantLabel(item.tenant_id).name}
                    </div>
                    {#if tenantLabel(item.tenant_id).slug}
                      <div class="table-tenant-sub">
                        {tenantLabel(item.tenant_id).slug}
                      </div>
                    {/if}
                  {:else}
                    —
                  {/if}
                </div>
              {:else if key === 'amount'}
                {formatCurrency(item.amount, item.currency_code)}
              {:else if key === 'status'}
                <span class="status-pill {item.status}">
                  {item.status}
                </span>
              {:else if key === 'due_date' || key === 'created_at'}
                {item[key] ? formatDate(item[key], { timeZone: $appSettings.app_timezone }) : '—'}
              {:else if key === 'actions'}
                <div class="actions">
                  <button
                    class="action-btn"
                    title={$t('superadmin.invoices.actions.check_status')}
                    type="button"
                    onclick={() => checkStatus(item.id)}
                  >
                    <Icon name="refresh-cw" size={18} />
                  </button>
                  <button
                    type="button"
                    class="action-btn"
                    title={$t('superadmin.invoices.actions.view_details')}
                    onclick={() => goto(`/superadmin/invoices/${item.id}`)}
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
      {/if}
    {/if}
  </div>
</div>

<style>
  .sa-invoices {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1280px;
    margin: 0 auto;
    color: var(--text-primary);
  }

  /* ── Page header (shared across all superadmin pages) ── */
  .page-head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 22px;
    flex-wrap: wrap;
  }

  .crumbs {
    font-size: 0.75rem;
    color: var(--text-secondary);
    opacity: 0.75;
    margin-bottom: 6px;
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .crumbs b { font-weight: 500; opacity: 1; }

  .page-head h1 {
    font-size: 1.45rem;
    font-weight: 750;
    letter-spacing: -0.02em;
    margin: 0;
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 0.875rem;
    margin: 2px 0 0;
  }

  .head-actions {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    transition: all 0.15s;
  }

  .btn:hover { border-color: var(--color-primary); }

  .btn.ghost { background: transparent; }
  .content-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    overflow: hidden;
  }

  .stats-row {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
    margin-bottom: 1rem;
  }
  .stat-btn {
    text-align: left;
    padding: 0.9rem 1rem;
    border-radius: 14px;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    cursor: pointer;
    transition: 0.2s;
  }
  .stat-btn:hover {
    transform: translateY(-1px);
    border-color: rgba(99, 102, 241, 0.35);
  }
  .stat-btn.active {
    border-color: rgba(99, 102, 241, 0.6);
    box-shadow:
      0 18px 40px rgba(0, 0, 0, 0.2),
      0 0 0 1px rgba(99, 102, 241, 0.16);
  }
  .stat-title {
    color: var(--text-secondary);
    font-size: 0.85rem;
  }
  .stat-value {
    margin-top: 0.25rem;
    font-size: 1.5rem;
    font-weight: 800;
    letter-spacing: -0.02em;
  }

  .toolbar-wrapper {
    padding: 1rem 1rem 0.5rem 1rem;
  }
  .toolbar-field {
    display: grid;
    gap: 0.32rem;
    max-width: 240px;
  }
  .toolbar-field label {
    font-size: 0.74rem;
    font-weight: 800;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }
  .toolbar-field select {
    min-height: 40px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    padding: 0 0.75rem;
    outline: none;
  }
  .toolbar-field select:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .loading-state {
    padding: 2.5rem 1rem;
    display: grid;
    place-items: center;
    gap: 0.75rem;
    color: var(--text-secondary);
  }
  .spinner {
    width: 22px;
    height: 22px;
    border-radius: 999px;
    border: 3px solid rgba(255, 255, 255, 0.14);
    border-top-color: rgba(99, 102, 241, 0.95);
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .empty-grid {
    padding: 2.5rem 1rem 3rem 1rem;
    display: grid;
    place-items: center;
    text-align: center;
    gap: 0.35rem;
    color: var(--text-secondary);
  }
  .empty-icon {
    width: 92px;
    height: 92px;
    border-radius: var(--radius-lg);
    display: grid;
    place-items: center;
    background: rgba(99, 102, 241, 0.1);
    color: var(--color-primary);
    border: 1px solid rgba(99, 102, 241, 0.18);
    margin-bottom: 0.5rem;
  }
  .empty-grid h4 {
    margin: 0.3rem 0 0 0;
    color: var(--text-primary);
  }

  .status-pill {
    padding: 0.25rem 0.6rem;
    border-radius: 12px;
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }
  .status-pill.pending,
  .status-pill.verification_pending {
    background: rgba(245, 158, 11, 0.12);
    color: var(--color-warning, #f59e0b);
    border-color: rgba(245, 158, 11, 0.22);
  }
  .status-pill.paid {
    background: rgba(16, 185, 129, 0.12);
    color: var(--color-success, #10b981);
    border-color: rgba(16, 185, 129, 0.22);
  }
  .status-pill.failed {
    background: rgba(239, 68, 68, 0.12);
    color: var(--color-danger, #ef4444);
    border-color: rgba(239, 68, 68, 0.22);
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
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

  .invoices-grid {
    padding: 1rem;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 1rem;
  }
  .invoice-card {
    background: var(--bg-surface);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--radius-lg);
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }
  .invoice-top {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.75rem;
  }
  .invoice-number {
    font-weight: 900;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }
  .invoice-tenant {
    margin-top: 0.2rem;
    display: grid;
    gap: 0.1rem;
  }
  .tenant-name {
    color: var(--text-secondary);
    font-weight: 700;
    font-size: 0.9rem;
  }
  .tenant-slug {
    font-size: 0.8rem;
  }
  .invoice-meta {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }
  .meta-item {
    padding: 0.7rem 0.8rem;
    border-radius: 14px;
    background: rgba(0, 0, 0, 0.12);
    border: 1px solid rgba(255, 255, 255, 0.06);
  }
  .meta-label {
    display: block;
    font-size: 0.75rem;
    color: var(--text-tertiary, rgba(255, 255, 255, 0.6));
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .meta-value {
    display: block;
    margin-top: 0.25rem;
    color: var(--text-primary);
    font-weight: 800;
  }
  .invoice-actions {
    display: flex;
    gap: 0.6rem;
    justify-content: flex-end;
  }

  .table-wrapper {
    padding: 0 1rem 1rem 1rem;
  }
  .table-tenant-name {
    font-weight: 800;
    color: var(--text-primary);
  }
  .table-tenant-sub {
    margin-top: 0.1rem;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .muted-text {
    color: var(--text-secondary);
  }

  @media (max-width: 720px) {
    .page-head {
      align-items: flex-start;
      flex-direction: column;
    }
    .stats-row {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .table-wrapper {
      padding: 0 0.75rem 0.85rem 0.75rem;
    }
    .invoices-grid {
      padding: 0.75rem;
      grid-template-columns: 1fr;
    }
    .invoice-meta {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 480px) {
    .stats-row {
      grid-template-columns: 1fr;
    }
  }

  :global([data-theme='light']) .stat-btn {
    background: var(--bg-surface);
    border-color: rgba(0, 0, 0, 0.06);
    box-shadow: var(--shadow-sm);
  }
  :global([data-theme='light']) .invoice-card {
    background: var(--bg-surface);
    border-color: rgba(0, 0, 0, 0.06);
    box-shadow:
      0 10px 28px rgba(0, 0, 0, 0.06),
      0 0 0 1px rgba(255, 255, 255, 0.8);
  }
  :global([data-theme='light']) .meta-item {
    background: rgba(0, 0, 0, 0.03);
    border-color: rgba(0, 0, 0, 0.06);
  }
</style>
