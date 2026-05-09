<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import type { Invoice } from '$lib/api/client';

  let {
    t,
    billingStatus = $bindable(),
    billingDateFrom = $bindable(),
    billingDateTo = $bindable(),
    billingQuickRange = $bindable(),
    onApplyQuickRange,
    onBillingDateChange,
    onClearFilters,
    onRefresh,
    loadingBilling,
    billingStats,
    billingColumns,
    billingRows,
    getSubscriptionIdFromInvoice,
    subscriptionById,
    billingStatusLabel,
    formatMoney,
    onOpenInvoiceDetail,
  } = $props();
</script>

<div class="card section">
  <div class="section-head">
    <div>
      <h3>{$t('admin.customers.billing.title') || 'Billing'}</h3>
      <p class="subtitle">Riwayat invoice pelanggan.</p>
    </div>
    <div class="header-actions">
      <label class="inline-filter">
        <span>{$t('admin.customers.billing.filters.status') || 'Status'}</span>
        <select class="input" bind:value={billingStatus}>
          <option value="all">{$t('admin.customers.billing.filters.all') || 'All'}</option>
          <option value="pending">{$t('admin.package_invoices.statuses.pending') || 'Pending'}</option>
          <option value="verification_pending">{$t('admin.package_invoices.statuses.verification_pending') || 'Verification pending'}</option>
          <option value="paid">{$t('admin.package_invoices.statuses.paid') || 'Paid'}</option>
          <option value="failed">{$t('admin.package_invoices.statuses.failed') || 'Failed'}</option>
        </select>
      </label>
      <div class="quick-ranges">
        <button class="btn btn-secondary btn-quick" class:active={billingQuickRange === 'today'} onclick={() => onApplyQuickRange('today')}>
          {$t('admin.customers.billing.filters.today') || 'Today'}
        </button>
        <button class="btn btn-secondary btn-quick" class:active={billingQuickRange === '7d'} onclick={() => onApplyQuickRange('7d')}>
          {$t('admin.customers.billing.filters.last_7d') || '7D'}
        </button>
        <button class="btn btn-secondary btn-quick" class:active={billingQuickRange === '30d'} onclick={() => onApplyQuickRange('30d')}>
          {$t('admin.customers.billing.filters.last_30d') || '30D'}
        </button>
        <button class="btn btn-secondary btn-quick" class:active={billingQuickRange === 'month'} onclick={() => onApplyQuickRange('month')}>
          {$t('admin.customers.billing.filters.this_month') || 'This Month'}
        </button>
      </div>
      <label class="inline-filter">
        <span>{$t('admin.customers.billing.filters.from') || 'From'}</span>
        <input class="input" type="date" bind:value={billingDateFrom} oninput={onBillingDateChange} />
      </label>
      <label class="inline-filter">
        <span>{$t('admin.customers.billing.filters.to') || 'To'}</span>
        <input class="input" type="date" bind:value={billingDateTo} oninput={onBillingDateChange} />
      </label>
      <button class="btn btn-secondary" onclick={onClearFilters} disabled={billingStatus === 'all' && !billingDateFrom && !billingDateTo}>
        <Icon name="eraser" size={16} />
        {$t('admin.customers.billing.filters.clear') || 'Clear'}
      </button>
      <button class="btn btn-secondary" onclick={onRefresh} disabled={loadingBilling}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
    </div>
  </div>

  <div class="billing-stats">
    <div class="billing-stat">
      <div class="billing-stat-label">{$t('admin.customers.billing.stats.total') || 'Total invoices'}</div>
      <div class="billing-stat-value">{billingStats.total}</div>
    </div>
    <div class="billing-stat">
      <div class="billing-stat-label">{$t('admin.customers.billing.stats.unpaid') || 'Unpaid'}</div>
      <div class="billing-stat-value">{billingStats.unpaid}</div>
    </div>
    <div class="billing-stat">
      <div class="billing-stat-label">{$t('admin.customers.billing.stats.paid') || 'Paid'}</div>
      <div class="billing-stat-value">{billingStats.paid}</div>
    </div>
    <div class="billing-stat">
      <div class="billing-stat-label">{$t('admin.customers.billing.stats.overdue') || 'Overdue'}</div>
      <div class="billing-stat-value">{billingStats.overdue}</div>
    </div>
  </div>

  <Table
    columns={billingColumns}
    data={billingRows}
    loading={loadingBilling}
    emptyText={$t('admin.customers.billing.empty') || 'No invoices for this customer yet.'}
    pagination
  >
    {#snippet cell({ item, key })}
      {@const row = item as Invoice}
      {@const subscriptionId = getSubscriptionIdFromInvoice(row)}
      {@const subscription = subscriptionId ? subscriptionById.get(subscriptionId) : null}
      {#if key === 'invoice_number'}
        <div class="name">#{row.invoice_number}</div>
        <div class="sub mono">{row.created_at ? new Date(row.created_at).toLocaleString() : '-'}</div>
      {:else if key === 'subscription'}
        <div class="name">{subscription?.package_name || subscription?.package_id || '-'}</div>
        <div class="sub">{subscription?.billing_cycle || '-'}</div>
      {:else if key === 'amount'}
        <div class="name">{formatMoney(row.amount, { currency: row.currency_code || undefined })}</div>
      {:else if key === 'status'}
        <span class={`badge ${row.status === 'paid' ? 'ok' : row.status === 'failed' ? 'danger' : 'warn'}`}>{billingStatusLabel(row.status)}</span>
      {:else if key === 'due_date'}
        <div class="name">{new Date(row.due_date).toLocaleDateString()}</div>
        <div class="sub mono">{new Date(row.due_date).toLocaleTimeString()}</div>
      {:else if key === 'actions'}
        <div class="row-actions">
          <button class="btn-icon" title={$t('admin.package_invoices.list.actions.view_details') || 'View details'} onclick={() => onOpenInvoiceDetail(row.id)}>
            <Icon name="eye" size={16} />
          </button>
        </div>
      {:else}
        {item[key] ?? ''}
      {/if}
    {/snippet}
  </Table>
</div>

<style>
  .section {
    padding: 1.1rem;
    background: var(--bg-surface);
  }
  .section-head,
  .header-actions {
    display: flex;
    gap: 1rem;
  }
  .section-head {
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1rem;
  }
  .header-actions {
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  .subtitle,
  .sub,
  .billing-stat-label {
    color: var(--text-secondary);
  }
  .subtitle {
    margin-top: 0.25rem;
  }
  .inline-filter {
    display: grid;
    gap: 0.3rem;
    min-width: 180px;
  }
  .inline-filter span {
    font-size: 0.8rem;
    color: var(--text-secondary);
    margin: 0;
  }
  .input {
    width: 100%;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    padding: 0.65rem 0.75rem;
  }
  .quick-ranges {
    display: flex;
    align-items: flex-end;
    gap: 0.45rem;
  }
  .btn,
  .btn-icon {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    cursor: pointer;
  }
  .btn {
    border-radius: 12px;
    padding: 0.55rem 0.9rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    font-weight: 650;
    font-size: 0.9rem;
  }
  .btn-quick {
    min-height: 40px;
    padding-inline: 0.7rem;
    border-radius: 10px;
  }
  .btn-quick.active {
    border-color: rgba(99, 102, 241, 0.5);
    background: rgba(99, 102, 241, 0.14);
    color: #e0e7ff;
  }
  .billing-stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.65rem;
    margin-bottom: 0.85rem;
  }
  .billing-stat {
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 20%);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-surface), transparent 9%);
    padding: 0.65rem 0.75rem;
  }
  .billing-stat-label {
    font-size: 0.8rem;
    margin-bottom: 0.2rem;
  }
  .billing-stat-value {
    font-weight: 800;
    font-size: 1.1rem;
    letter-spacing: -0.01em;
    color: var(--text-primary);
  }
  .name {
    font-weight: 650;
  }
  .sub {
    font-size: 0.85rem;
    margin-top: 0.15rem;
  }
  .mono {
    font-variant-numeric: tabular-nums;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }
  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
  .btn-icon {
    border-radius: 10px;
    padding: 0.4rem 0.45rem;
  }
  .badge.danger {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgb(239, 68, 68);
    background: rgba(239, 68, 68, 0.1);
  }
  @media (max-width: 900px) {
    .section-head {
      flex-direction: column;
      align-items: stretch;
    }
    .header-actions {
      justify-content: stretch;
    }
    .billing-stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .quick-ranges {
      width: 100%;
      justify-content: flex-start;
      flex-wrap: wrap;
    }
  }
</style>
