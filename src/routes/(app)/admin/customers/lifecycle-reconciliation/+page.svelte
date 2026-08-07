<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page as pageStore } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { toast } from 'svelte-sonner';
  import { can } from '$lib/stores/auth';
  import {
    api,
    type CustomerServiceLifecycleIssue,
    type CustomerServiceLifecycleRepairResult,
    type CustomerServiceLifecycleReport,
  } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import StatsCard from '$lib/components/dashboard/StatsCard.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import TableToolbar from '$lib/components/ui/TableToolbar.svelte';

  const columns = $derived.by(() => [
    {
      key: 'customer',
      label: $t('admin.customers.reconciliation.columns.customer') || 'Customer',
    },
    {
      key: 'service',
      label: $t('admin.customers.reconciliation.columns.service') || 'Service',
    },
    {
      key: 'issue',
      label: $t('admin.customers.reconciliation.columns.issue') || 'Issue',
    },
    {
      key: 'action',
      label: $t('admin.customers.reconciliation.columns.action') || 'Recommended action',
    },
  ]);

  let report = $state<CustomerServiceLifecycleReport>({
    generated_at: '',
    total_issues: 0,
    missing_bootstrap_invoice: 0,
    invalid_active_lifecycle: 0,
    page: 1,
    per_page: 25,
    data: [],
  });
  let repairResult = $state<CustomerServiceLifecycleRepairResult | null>(null);
  let repairConfirmOpen = $state(false);
  let pendingRepairType = $state<'missing_bootstrap_invoice' | 'invalid_active_lifecycle' | null>(null);
  let loading = $state(true);
  let repairing = $state(false);
  let error = $state('');
  let q = $state('');
  let issueFilter = $state<'all' | 'missing_bootstrap_invoice' | 'invalid_active_lifecycle'>('all');
  let page = $state(0);
  let perPage = $state(25);

  const canReadCustomers = $derived($can('read', 'customers') || $can('create', 'orders'));
  const canRepairLifecycle = $derived($can('manage', 'billing'));
  const visibleRows = $derived(report.data.length);

  onMount(async () => {
    if (!canReadCustomers) {
      goto('/unauthorized');
      return;
    }
    await loadReport();
  });

  function adminBasePath() {
    return $pageStore.url.pathname.replace(
      /\/admin\/customers\/lifecycle-reconciliation\/?$/,
      '/admin',
    );
  }

  function openCustomer(issue: CustomerServiceLifecycleIssue) {
    goto(`${adminBasePath()}/customers/${issue.customer_id}?tab=subscriptions`);
  }

  async function loadReport() {
    loading = true;
    error = '';
    try {
      report = await api.customers.reconciliation.report({
        q: q.trim() || undefined,
        issueType: issueFilter,
        page: page + 1,
        perPage,
      });
      if (!repairing) repairResult = null;
    } catch (e: any) {
      error =
        e?.message ||
        $t('admin.customers.reconciliation.toasts.load_failed') ||
        'Failed to load lifecycle reconciliation report';
      toast.error(error);
    } finally {
      loading = false;
    }
  }

  function requestRepair(issueType: 'missing_bootstrap_invoice' | 'invalid_active_lifecycle') {
    if (repairing) return;
    pendingRepairType = issueType;
    repairConfirmOpen = true;
  }

  async function repairIssues(issueType: 'missing_bootstrap_invoice' | 'invalid_active_lifecycle') {
    if (repairing) return;
    repairing = true;
    try {
      const result = await api.customers.reconciliation.repair(issueType);
      repairResult = result;
      toast.success(
        ($t('admin.customers.reconciliation.toasts.repair_success', {
          values: {
            matched: result.matched_count,
            repaired: result.repaired_count,
            skipped: result.skipped_count,
            failed: result.failed_count,
          },
        }) as string) || `Berhasil membuat invoice awal untuk ${result.repaired_count} layanan.`,
      );
      await loadReport();
    } catch (e: any) {
      toast.error(
        e?.message ||
          $t('admin.customers.reconciliation.toasts.repair_failed') ||
          'Failed to repair lifecycle issues',
      );
    } finally {
      repairing = false;
    }
  }

  async function setIssueFilter(
    next: 'all' | 'missing_bootstrap_invoice' | 'invalid_active_lifecycle',
  ) {
    if (issueFilter === next) return;
    issueFilter = next;
    page = 0;
    await loadReport();
  }

  async function handlePageChange(nextPage: number) {
    if (page === nextPage) return;
    page = nextPage;
    await loadReport();
  }

  async function handlePageSizeChange(nextSize: number) {
    if (perPage === nextSize) return;
    perPage = nextSize;
    page = 0;
    await loadReport();
  }

  function issueLabel(issueType: string) {
    if (issueType === 'missing_bootstrap_invoice') {
      return (
        $t('admin.customers.reconciliation.issue_types.missing_bootstrap_invoice') ||
        'Belum ada invoice awal'
      );
    }
    if (issueType === 'invalid_active_lifecycle') {
      return (
        $t('admin.customers.reconciliation.issue_types.invalid_active_lifecycle') ||
        'Lifecycle aktif tidak valid'
      );
    }
    return issueType;
  }

  function recommendedActionLabel(action: string) {
    if (action === 'bootstrap_invoice') {
      return (
        $t('admin.customers.reconciliation.actions.bootstrap_missing_invoices') ||
        'Buat invoice awal'
      );
    }
    if (action === 'review_lifecycle_data') {
      return (
        $t('admin.customers.reconciliation.actions.review_lifecycle_data') ||
        'Tinjau data lifecycle'
      );
    }
    if (action === 'suspend_invalid_active_lifecycle') {
      return (
        $t('admin.customers.reconciliation.actions.suspend_invalid_active_lifecycle') ||
        'Suspend service'
      );
    }
    return action;
  }

  function serviceLabel(issue: CustomerServiceLifecycleIssue) {
    const packageName =
      issue.package_name || $t('admin.customers.reconciliation.fallback.package') || 'Package';
    const location =
      issue.location_label ||
      $t('admin.customers.reconciliation.fallback.unknown_location') ||
      'Unknown location';
    return `${packageName} • ${location}`;
  }

  function formatPeriod(issue: CustomerServiceLifecycleIssue) {
    const start = issue.starts_at ? issue.starts_at.slice(0, 10) : '—';
    const end = issue.ends_at ? issue.ends_at.slice(0, 10) : '—';
    return `${start} → ${end}`;
  }
</script>

<svelte:head>
  <title>{$t('admin.customers.reconciliation.title')}</title>
</svelte:head>

<div class="page-content fade-in lifecycle-page">
  <div class="page-header">
    <div>
      <button class="back-link" onclick={() => goto(`${adminBasePath()}/customers`)}>
        <Icon name="arrow-left" size={16} />
        <span>{$t('common.back')}</span>
      </button>
      <h1>{$t('admin.customers.reconciliation.title')}</h1>
      <p class="subtitle muted">
        {$t('admin.customers.reconciliation.subtitle')}
      </p>
    </div>
    <div class="header-actions">
      <button class="btn ghost" onclick={loadReport} disabled={loading || repairing}>
        <Icon name="refresh-cw" size={16} />
        <span>{$t('common.refresh')}</span>
      </button>
      {#if canRepairLifecycle}
        <button
          class="btn primary"
          onclick={() => requestRepair('missing_bootstrap_invoice')}
          disabled={repairing || loading || report.missing_bootstrap_invoice === 0}
        >
          <Icon name="receipt" size={16} />
          <span>
            {$t('admin.customers.reconciliation.actions.bootstrap_missing_invoices')}
          </span>
        </button>
        <button
          class="btn ghost danger"
          onclick={() => requestRepair('invalid_active_lifecycle')}
          disabled={repairing || loading || report.invalid_active_lifecycle === 0}
        >
          <Icon name="alert-triangle" size={16} />
          <span>
            {$t('admin.customers.reconciliation.actions.suspend_invalid_active_lifecycle')}
          </span>
        </button>
      {/if}
    </div>
  </div>

  <div class="stats-grid reconciliation-stats-grid">
    <StatsCard
      title={$t('admin.customers.reconciliation.stats.total_issues')}
      value={report.total_issues}
      icon="shield-alert"
      color="orange"
    />
    <StatsCard
      title={$t('admin.customers.reconciliation.stats.missing_bootstrap_invoice')}
      value={report.missing_bootstrap_invoice}
      icon="alert-triangle"
      color="orange"
    />
    <StatsCard
      title={$t('admin.customers.reconciliation.stats.invalid_active_lifecycle')}
      value={report.invalid_active_lifecycle}
      icon="alert-triangle"
      color="orange"
    />
    <StatsCard
      title={$t('admin.customers.reconciliation.stats.visible_rows')}
      value={visibleRows}
      icon="list"
      color="blue"
    />
  </div>

  <div class="card table-card">
    <TableToolbar
      bind:searchQuery={q}
      placeholder={$t('admin.customers.reconciliation.search')}
      onsearch={async () => {
        page = 0;
        await loadReport();
      }}
      onclear={async () => {
        page = 0;
        await loadReport();
      }}
    >
      {#snippet filters()}
        <label class="reconciliation-filter-field">
          <span>{$t('admin.customers.reconciliation.filters.issue_type')}</span>
          <select
            class="reconciliation-filter-select"
            aria-label={$t('admin.customers.reconciliation.filters.issue_type')}
            value={issueFilter}
            onchange={(event) =>
              setIssueFilter(
                (event.currentTarget as HTMLSelectElement).value as
                  | 'all'
                  | 'missing_bootstrap_invoice'
                  | 'invalid_active_lifecycle',
              )}
          >
            <option value="all">
              {$t('admin.customers.reconciliation.filters.all_issues')}
            </option>
            <option value="missing_bootstrap_invoice">
              {$t('admin.customers.reconciliation.issue_types.missing_bootstrap_invoice')}
            </option>
            <option value="invalid_active_lifecycle">
              {$t('admin.customers.reconciliation.issue_types.invalid_active_lifecycle')}
            </option>
          </select>
        </label>
      {/snippet}
      {#snippet actions()}
        <span class="muted">
          {$t('admin.customers.reconciliation.generated_at')}
          {' '}
          {report.generated_at ? report.generated_at.replace('T', ' ').slice(0, 16) : '-'}
        </span>
      {/snippet}
    </TableToolbar>

    {#if error}
      <div class="error-banner">
        <Icon name="alert-triangle" size={18} />
        <span>{error}</span>
      </div>
    {/if}

    {#if repairResult}
      <div class="repair-result-card">
        <div class="repair-result-header">
          <div>
            <div class="customer-name">
              {$t('admin.customers.reconciliation.repair_result.title')}
            </div>
            <div class="muted small">
              {$t('admin.customers.reconciliation.repair_result.subtitle')}
            </div>
          </div>
        </div>
        <div class="repair-result-grid">
          <div class="repair-stat">
            <span class="muted small">
              {$t('admin.customers.reconciliation.repair_result.matched')}
            </span>
            <strong>{repairResult.matched_count}</strong>
          </div>
          <div class="repair-stat">
            <span class="muted small">
              {$t('admin.customers.reconciliation.repair_result.repaired')}
            </span>
            <strong>{repairResult.repaired_count}</strong>
          </div>
          <div class="repair-stat">
            <span class="muted small">
              {$t('admin.customers.reconciliation.repair_result.skipped')}
            </span>
            <strong>{repairResult.skipped_count}</strong>
          </div>
          <div class="repair-stat">
            <span class="muted small">
              {$t('admin.customers.reconciliation.repair_result.failed')}
            </span>
            <strong>{repairResult.failed_count}</strong>
          </div>
        </div>
        {#if repairResult.errors.length > 0}
          <div class="repair-errors">
            <div class="muted small">
              {$t('admin.customers.reconciliation.repair_result.errors')}
            </div>
            <ul class="repair-error-list">
              {#each repairResult.errors as repairError}
                <li>{repairError}</li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>
    {/if}

    <Table
      {columns}
      data={report.data}
      {loading}
      pagination={true}
      serverSide={true}
      count={report.total_issues}
      pageSize={perPage}
      onchange={handlePageChange}
      onpageSizeChange={handlePageSizeChange}
      emptyText={$t('admin.customers.reconciliation.empty')}
    >
      {#snippet cell({ item, column })}
        {#if column.key === 'customer'}
          <button class="linkish" onclick={() => openCustomer(item)}>
            <div class="customer-name">{item.customer_name}</div>
            <div class="muted small">{item.subscription_id}</div>
          </button>
        {:else if column.key === 'service'}
          <div class="service-cell">
            <div>{serviceLabel(item)}</div>
            <div class="muted small">{formatPeriod(item)}</div>
          </div>
        {:else if column.key === 'issue'}
          <div class="issue-cell">
            <span class="issue-badge warning">{issueLabel(item.issue_type)}</span>
            <div class="muted small">{item.subscription_status}</div>
          </div>
        {:else if column.key === 'action'}
          <div class="action-cell">
            <span class="action-chip">{recommendedActionLabel(item.recommended_action)}</span>
            <button class="btn btn-secondary btn-xs" onclick={() => openCustomer(item)}>
              <Icon name="arrow-right" size={14} />
              <span
                >{$t('admin.customers.reconciliation.actions.open_service')}</span
              >
            </button>
          </div>
        {/if}
      {/snippet}
    </Table>
  </div>
</div>

<ConfirmDialog
  bind:show={repairConfirmOpen}
  title={$t('admin.customers.reconciliation.confirm.title')}
  message={pendingRepairType === 'invalid_active_lifecycle'
    ? $t('admin.customers.reconciliation.confirm.invalid_lifecycle')
    : $t('admin.customers.reconciliation.confirm.bootstrap_invoice')}
  confirmText={$t('common.confirm') || 'Confirm'}
  cancelText={$t('common.cancel') || 'Cancel'}
  type={pendingRepairType === 'invalid_active_lifecycle' ? 'danger' : 'warning'}
  loading={repairing}
  onconfirm={() => {
    repairConfirmOpen = false;
    if (pendingRepairType) void repairIssues(pendingRepairType);
  }}
  oncancel={() => (pendingRepairType = null)}
/>

<style>
  .lifecycle-page {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    padding-block: 0.5rem 1.5rem;
    padding-inline: 1rem;
  }

  @media (min-width: 768px) {
    .lifecycle-page {
      padding-inline: 1.5rem;
    }
  }

  @media (min-width: 1280px) {
    .lifecycle-page {
      padding-inline: 2rem;
    }
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    gap: 1.25rem;
    align-items: flex-start;
  }

  .header-actions {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .header-actions :global(.danger) {
    border-color: rgba(239, 68, 68, 0.35);
    color: #fca5a5;
  }

  .back-link {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    border: none;
    background: transparent;
    color: var(--color-text-muted, #94a3b8);
    padding: 0;
    margin-bottom: 0.5rem;
    cursor: pointer;
  }

  .reconciliation-stats-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 1rem;
  }

  .table-card {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem;
    overflow: hidden;
  }

  .reconciliation-filter-field {
    display: grid;
    gap: 0.25rem;
    min-width: 220px;
  }

  .reconciliation-filter-field span {
    color: var(--text-secondary);
    font-size: 0.72rem;
  }

  .reconciliation-filter-select {
    min-height: 38px;
    padding: 0.55rem 0.75rem;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.85rem 1rem;
    border-radius: 0.85rem;
    background: rgba(185, 28, 28, 0.1);
    color: #fecaca;
  }

  .repair-result-card {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem;
    border-radius: 1rem;
    border: 1px solid rgba(148, 163, 184, 0.18);
    background: color-mix(in srgb, var(--bg-surface, #111827) 88%, rgba(30, 41, 59, 0.4));
  }

  .repair-result-header {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: flex-start;
  }

  .repair-result-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .repair-stat {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.85rem;
    border-radius: 0.85rem;
    border: 1px solid rgba(148, 163, 184, 0.14);
    background: rgba(15, 23, 42, 0.45);
  }

  .repair-stat strong {
    font-size: 1.2rem;
  }

  .repair-errors {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .repair-error-list {
    margin: 0;
    padding-left: 1.1rem;
    display: grid;
    gap: 0.35rem;
    color: var(--color-text-muted, #cbd5e1);
    font-size: 0.86rem;
  }

  .linkish {
    border: none;
    background: transparent;
    padding: 0;
    text-align: left;
    cursor: pointer;
    color: inherit;
  }

  .customer-name {
    font-weight: 600;
  }

  .service-cell,
  .issue-cell {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .issue-badge,
  .action-chip {
    display: inline-flex;
    align-items: center;
    width: fit-content;
    padding: 0.25rem 0.6rem;
    border-radius: 999px;
    font-size: 0.8rem;
    border: 1px solid rgba(148, 163, 184, 0.2);
  }

  .issue-badge.warning {
    background: rgba(245, 158, 11, 0.12);
    color: #fbbf24;
    border-color: rgba(245, 158, 11, 0.3);
  }

  .small {
    font-size: 0.82rem;
  }

  .action-cell {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .btn-xs {
    min-height: 32px;
    padding: 0.45rem 0.7rem;
    border-radius: 10px;
  }

  @media (max-width: 640px) {
    .page-header {
      flex-direction: column;
    }

    .header-actions {
      width: 100%;
      justify-content: stretch;
    }

    .reconciliation-stats-grid {
      grid-template-columns: 1fr;
    }

    .repair-result-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
