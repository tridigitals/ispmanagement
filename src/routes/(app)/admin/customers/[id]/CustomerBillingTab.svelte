<script lang="ts">
  import Table from '$lib/components/ui/Table.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import type { Invoice } from '$lib/api/client';
  import type { CustomerBillingFilter } from './customerBillingState';

  let {
    t,
    billingFilter = $bindable<CustomerBillingFilter>(),
    onSelectBillingFilter,
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
      <h3>{$t('admin.customers.billing.title')}</h3>
      <p class="subtitle">{$t('admin.customers.billing.subtitle')}</p>
    </div>
  </div>

  <div class="billing-stats">
    <button
      type="button"
      class={`billing-stat-button ${billingFilter === 'all' ? 'active' : ''}`}
      onclick={() => onSelectBillingFilter('all')}
      aria-pressed={billingFilter === 'all'}
    >
      <div class="billing-stat">
        <div class="billing-stat-label">{$t('admin.customers.billing.stats.total')}</div>
        <div class="billing-stat-value">{billingStats.total}</div>
      </div>
    </button>
    <button
      type="button"
      class={`billing-stat-button ${billingFilter === 'unpaid' ? 'active' : ''}`}
      onclick={() => onSelectBillingFilter('unpaid')}
      aria-pressed={billingFilter === 'unpaid'}
    >
      <div class="billing-stat">
        <div class="billing-stat-label">{$t('admin.customers.billing.stats.unpaid')}</div>
        <div class="billing-stat-value">{billingStats.unpaid}</div>
      </div>
    </button>
    <button
      type="button"
      class={`billing-stat-button ${billingFilter === 'paid' ? 'active' : ''}`}
      onclick={() => onSelectBillingFilter('paid')}
      aria-pressed={billingFilter === 'paid'}
    >
      <div class="billing-stat">
        <div class="billing-stat-label">{$t('admin.customers.billing.stats.paid')}</div>
        <div class="billing-stat-value">{billingStats.paid}</div>
      </div>
    </button>
    <button
      type="button"
      class={`billing-stat-button ${billingFilter === 'overdue' ? 'active' : ''}`}
      onclick={() => onSelectBillingFilter('overdue')}
      aria-pressed={billingFilter === 'overdue'}
    >
      <div class="billing-stat">
        <div class="billing-stat-label">{$t('admin.customers.billing.stats.overdue')}</div>
        <div class="billing-stat-value">{billingStats.overdue}</div>
      </div>
    </button>
  </div>

  <Table
    columns={billingColumns}
    data={billingRows}
    loading={loadingBilling}
    emptyText={$t('admin.customers.billing.empty')}
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
          <button class="btn-icon" title={$t('admin.package_invoices.list.actions.view_details')} onclick={() => onOpenInvoiceDetail(row.id)}>
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

  .section-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 0.75rem;
  }

  .subtitle,
  .sub,
  .billing-stat-label {
    color: var(--text-secondary);
  }

  .subtitle {
    margin-top: 0.25rem;
  }

  .billing-stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.65rem;
    margin-bottom: 0.85rem;
  }

  .billing-stat-button {
    border: 0;
    background: transparent;
    padding: 0;
    text-align: left;
    cursor: pointer;
  }

  .billing-stat {
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 20%);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-surface), transparent 9%);
    padding: 0.65rem 0.75rem;
    min-height: 100%;
    transition:
      border-color 140ms ease,
      transform 140ms ease,
      background 140ms ease,
      box-shadow 140ms ease;
  }

  .billing-stat-button:hover .billing-stat,
  .billing-stat-button:focus-visible .billing-stat {
    border-color: color-mix(in srgb, var(--accent), var(--border-color) 42%);
    background: color-mix(in srgb, var(--bg-surface), var(--accent) 10%);
    transform: translateY(-1px);
  }

  .billing-stat-button.active .billing-stat {
    border-color: color-mix(in srgb, var(--accent), white 8%);
    background: color-mix(in srgb, var(--bg-surface), var(--accent) 14%);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent), transparent 35%);
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
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    cursor: pointer;
    border-radius: 10px;
    width: 38px;
    height: 38px;
    padding: 0;
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

    .billing-stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 560px) {
    .billing-stats {
      grid-template-columns: 1fr;
    }
  }
</style>
