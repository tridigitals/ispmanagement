<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import type { CustomerSubscriptionView } from '$lib/api/client';

  let {
    t,
    loadingSubscriptions,
    metricCount,
    subscriptionColumns,
    subscriptions,
    subscriptionStatusLabel,
    getSubscriptionPolicySummary,
    getSubscriptionAccessState,
    formatCustomerPolicyDate,
    canManageCustomers,
    onRefresh,
    onAdd,
    onGenerateInvoice,
    generatingInvoiceFor,
    deletingSubscription,
    onSetSubscriptionStatus,
    togglingSubscription,
    onEditSubscription,
    onDeleteSubscription,
    onChangePackage,
  } = $props();
</script>

<div class="card section">
  <div class="section-head">
    <div>
      <h3>{$t('admin.customers.subscriptions.title') || 'Layanan'}</h3>
      <p class="subtitle">{$t('admin.customers.subscriptions.subtitle')}</p>
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" onclick={onRefresh} disabled={loadingSubscriptions}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Muat ulang'}
      </button>
      {#if canManageCustomers}
        <button class="btn btn-primary" onclick={onAdd}>
          <Icon name="plus" size={16} />
          {$t('common.add') || 'Tambah'}
        </button>
      {/if}
    </div>
  </div>

  <div class="summary-strip">
    <div class="summary-card">
      <div>
        <span class="summary-label">{$t('admin.customers.subscriptions.total_services')}</span>
        <strong>{subscriptions.length}</strong>
      </div>
    </div>
    <div class="summary-card emphasis">
      <div>
        <span class="summary-label">{$t('admin.customers.subscriptions.grace_active')}</span>
        <strong>{metricCount('grace_active') || metricCount('installation_done_awaiting_payment')}</strong>
      </div>
    </div>
    <div class="summary-card">
      <div>
        <span class="summary-label">{$t('common.active')}</span>
        <strong>{metricCount('active')}</strong>
      </div>
    </div>
    <div class="summary-card">
      <div>
        <span class="summary-label">{$t('admin.customers.subscriptions.wo_active')}</span>
        <strong>{metricCount('in_progress', 'work_order')}</strong>
      </div>
    </div>
  </div>

  <Table
    columns={subscriptionColumns}
    data={subscriptions}
    loading={loadingSubscriptions}
    emptyText={$t('admin.customers.subscriptions.empty') || 'Belum ada layanan.'}
    pagination
  >
    {#snippet cell({ item, key })}
      {@const row = item as CustomerSubscriptionView}
      {#if key === 'package'}
        <div class="package-stack">
          <div class="name">{row.package_name || row.package_id}</div>
          <div class="status-chip">{subscriptionStatusLabel(row.status)}</div>
        </div>
      {:else if key === 'billing'}
        <div class="billing-stack">
          <div class="name">{row.billing_cycle}</div>
          <div class="amount-pill mono">
            {row.currency_code}
            {Number(row.price || 0).toLocaleString()}
          </div>
        </div>
      {:else if key === 'location'}
        <div>{row.location_label || '-'}</div>
      {:else if key === 'router'}
        <div>{row.router_name || '-'}</div>
      {:else if key === 'lifecycle'}
        {@const summary = getSubscriptionPolicySummary(row)}
        {@const accessState = getSubscriptionAccessState(row)}
        <div class="policy-card">
          <div class="policy-row">
            <span class="policy-label">{$t('admin.customers.subscriptions.active_period')}</span>
            <div class="policy-value">
              {summary.activeUntilMissing
                : $t('admin.customers.subscriptions.active_period') + ' —'
                : formatCustomerPolicyDate(summary.activeUntilIso)}
            </div>
          </div>
          <div class="policy-row">
            <span class="policy-label">{$t('admin.customers.subscriptions.policy')}</span>
            <div class="policy-value">{summary.policyLabel}</div>
          </div>
          <div class="policy-row">
            <span class="policy-label">{$t('admin.customers.subscriptions.estimated_suspend')}</span>
            <div class="policy-value emphasis">
              {summary.estimatedSuspendIso
                ? formatCustomerPolicyDate(summary.estimatedSuspendIso)
                : summary.estimatedSuspendMissingReason || '-'}
            </div>
          </div>
          {#if accessState}
            <div class="policy-row">
              <span class="policy-label">{$t('admin.customers.subscriptions.access_on_suspend')}</span>
              <div class="access-state">
                <span class={`access-badge ${accessState.tone}`}>{accessState.label}</span>
                <div class="policy-value subtle">{accessState.detail}</div>
              </div>
            </div>
          {/if}
        </div>
      {:else if key === 'actions'}
        <div class="row-actions">
          {#if canManageCustomers}
            <button
              class="btn-icon"
              title={$t('admin.customers.billing.actions.generate_from_subscription') || 'Buat invoice'}
              onclick={() => onGenerateInvoice(row.id)}
              disabled={generatingInvoiceFor === row.id || deletingSubscription === row.id}
            >
              <Icon name="file-text" size={16} />
            </button>
            {#if row.status === 'active' && onChangePackage}
              <button
                class="btn-icon"
                title={$t('admin.customers.subscriptions.change_package')}
                onclick={() => onChangePackage(row)}
                disabled={togglingSubscription === row.id || deletingSubscription === row.id}
              >
                <Icon name="repeat" size={16} />
              </button>
            {/if}
            {#if row.status === 'active'}
              <button
                class="btn-icon"
                title={$t('common.suspend') || 'Suspend'}
                onclick={() => onSetSubscriptionStatus(row, 'suspended')}
                disabled={togglingSubscription === row.id || deletingSubscription === row.id}
              >
                <Icon name="pause" size={16} />
              </button>
            {:else if row.status === 'suspended'}
              <button
                class="btn-icon"
                title={$t('common.activate') || 'Activate'}
                onclick={() => onSetSubscriptionStatus(row, 'active')}
                disabled={togglingSubscription === row.id || deletingSubscription === row.id}
              >
                <Icon name="play" size={16} />
              </button>
            {/if}
            <button class="btn-icon" title={$t('common.edit') || 'Edit'} onclick={() => onEditSubscription(row)}>
              <Icon name="edit-3" size={16} />
            </button>
            <button
              class="btn-icon danger"
              title={$t('common.delete') || 'Hapus'}
              onclick={() => onDeleteSubscription(row.id)}
              disabled={deletingSubscription === row.id}
            >
              <Icon name="trash-2" size={16} />
            </button>
          {/if}
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
  .policy-label {
    color: var(--text-secondary);
  }

  .subtitle {
    margin-top: 0.25rem;
  }

  .summary-strip {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .summary-card {
    padding: 0.75rem 0.85rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 4%);
  }

  .summary-card.emphasis {
    border-color: rgba(245, 158, 11, 0.22);
    background: rgba(245, 158, 11, 0.05);
  }

  .summary-label {
    display: block;
    margin-bottom: 0.22rem;
    color: var(--text-secondary);
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .summary-card strong {
    font-size: 1rem;
  }

  .name {
    font-weight: 650;
  }

  .package-stack,
  .billing-stack {
    display: grid;
    gap: 0.35rem;
  }

  .policy-card {
    display: grid;
    gap: 0.55rem;
    min-width: 0;
    padding: 0.8rem 0.85rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 3%);
  }

  .policy-row {
    display: grid;
    gap: 0.12rem;
  }

  .policy-value {
    color: var(--text-primary);
    font-size: 0.9rem;
    font-weight: 600;
    line-height: 1.35;
  }

  .policy-value.emphasis {
    color: color-mix(in srgb, var(--text-primary), #9ec5ff 22%);
  }

  .policy-value.subtle {
    color: var(--text-secondary);
    font-size: 0.82rem;
    font-weight: 500;
  }

  .policy-label {
    display: block;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .mono {
    font-variant-numeric: tabular-nums;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .status-chip,
  .amount-pill {
    display: inline-flex;
    align-items: center;
    width: fit-content;
    border-radius: 999px;
    padding: 0.25rem 0.6rem;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    background: color-mix(in srgb, var(--bg-surface), transparent 6%);
  }

  .status-chip {
    color: var(--text-secondary);
    font-size: 0.78rem;
    font-weight: 700;
  }

  .amount-pill {
    color: var(--text-primary);
    font-size: 0.82rem;
  }

  .access-state {
    display: grid;
    gap: 0.35rem;
  }

  .access-badge {
    display: inline-flex;
    align-items: center;
    width: fit-content;
    border-radius: 999px;
    padding: 0.26rem 0.62rem;
    font-size: 0.74rem;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 10%);
  }

  .access-badge.warning {
    color: rgb(245, 158, 11);
    background: rgba(245, 158, 11, 0.1);
    border-color: rgba(245, 158, 11, 0.24);
  }

  .access-badge.danger {
    color: rgb(239, 68, 68);
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.24);
  }

  .access-badge.neutral {
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--bg-surface), transparent 5%);
  }

  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .btn,
  .btn-icon {
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 3%);
    color: var(--text-primary);
    cursor: pointer;
    transition:
      border-color 0.16s ease,
      transform 0.16s ease,
      background 0.16s ease;
  }

  .btn {
    border-radius: 12px;
    min-height: 42px;
    padding: 0.55rem 0.9rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    font-weight: 650;
    font-size: 0.9rem;
  }

  .btn:hover,
  .btn-icon:hover {
    transform: translateY(-1px);
    border-color: color-mix(in srgb, var(--accent-color, #3b82f6), var(--border-color) 45%);
  }

  .btn-primary {
    background: rgba(99, 102, 241, 0.95);
    border-color: rgba(99, 102, 241, 0.55);
    color: white;
  }

  .btn-icon {
    border-radius: 10px;
    width: 38px;
    height: 38px;
    padding: 0;
  }

  .btn-icon:disabled,
  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none;
  }

  .btn-icon.danger {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgb(239, 68, 68);
  }

  @media (max-width: 900px) {
    .section-head {
      flex-direction: column;
      align-items: stretch;
    }

    .header-actions {
      justify-content: stretch;
    }

    .header-actions .btn {
      flex: 1 1 0;
      min-width: 0;
    }

    .summary-strip,
    .row-actions {
      justify-content: flex-start;
    }
  }

  @media (max-width: 640px) {
    .summary-strip {
      grid-template-columns: 1fr;
    }
  }
</style>
