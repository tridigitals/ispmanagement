<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import type { CustomerSubscriptionView } from '$lib/api/client';

  let {
    t,
    loadingSubscriptions,
    loadingLifecycleObservability,
    lifecycleObservability,
    metricCount,
    agingBucketCount,
    timeAgo,
    subscriptionColumns,
    subscriptions,
    subscriptionStatusLabel,
    getSubscriptionPolicySummary,
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
  } = $props();
</script>

<div class="card section">
  <div class="section-head">
    <div>
      <h3>{$t('admin.customers.subscriptions.title') || 'Subscriptions'}</h3>
      <p class="subtitle">
        {$t('admin.customers.subscriptions.subtitle') ||
          'Customer service subscriptions for billing and service assignment.'}
      </p>
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" onclick={onRefresh} disabled={loadingSubscriptions}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
      {#if canManageCustomers}
        <button class="btn btn-primary" onclick={onAdd}>
          <Icon name="plus" size={16} />
          {$t('common.add') || 'Add'}
        </button>
      {/if}
    </div>
  </div>

  <div class="lifecycle-observability card">
    <div class="observability-head">
      <div>
        <h4>Lifecycle observability</h4>
        <p class="subtitle">Operational funnel and aging snapshot for this customer's activations.</p>
      </div>
      <span class="meta-pill">
        <Icon name="activity" size={14} />
        {#if loadingLifecycleObservability}
          Loading...
        {:else if lifecycleObservability?.generated_at}
          {`Updated ${timeAgo(lifecycleObservability.generated_at)}`}
        {:else}
          Waiting for data
        {/if}
      </span>
    </div>

    <div class="observability-grid">
      <div class="metric-tile">
        <span class="metric-label">Pending installation</span>
        <strong>{metricCount('pending_installation')}</strong>
      </div>
      <div class="metric-tile emphasis">
        <span class="metric-label">Grace active</span>
        <strong>{metricCount('grace_active') || metricCount('installation_done_awaiting_payment')}</strong>
      </div>
      <div class="metric-tile">
        <span class="metric-label">Active</span>
        <strong>{metricCount('active')}</strong>
      </div>
      <div class="metric-tile">
        <span class="metric-label">Cancelled</span>
        <strong>{metricCount('cancelled')}</strong>
      </div>
      <div class="metric-tile">
        <span class="metric-label">WO pending</span>
        <strong>{metricCount('pending', 'work_order')}</strong>
      </div>
      <div class="metric-tile">
        <span class="metric-label">WO in progress</span>
        <strong>{metricCount('in_progress', 'work_order')}</strong>
      </div>
      <div class="metric-tile">
        <span class="metric-label">WO completed</span>
        <strong>{metricCount('completed', 'work_order')}</strong>
      </div>
    </div>

    <div class="aging-row">
      <span class="aging-pill">0-1d: {agingBucketCount('0-1d')}</span>
      <span class="aging-pill">2-3d: {agingBucketCount('2-3d')}</span>
      <span class="aging-pill">4-7d: {agingBucketCount('4-7d')}</span>
      <span class="aging-pill">>7d: {agingBucketCount('>7d')}</span>
    </div>
  </div>

  <Table
    columns={subscriptionColumns}
    data={subscriptions}
    loading={loadingSubscriptions}
    emptyText={$t('admin.customers.subscriptions.empty') || 'No subscriptions yet.'}
    pagination
  >
    {#snippet cell({ item, key })}
      {@const row = item as CustomerSubscriptionView}
      {#if key === 'package'}
        <div class="name">{row.package_name || row.package_id}</div>
        <div class="sub">{subscriptionStatusLabel(row.status)}</div>
      {:else if key === 'billing'}
        <div class="name">{row.billing_cycle}</div>
        <div class="sub mono">
          {row.currency_code}
          {Number(row.price || 0).toLocaleString()}
        </div>
      {:else if key === 'location'}
        <div>{row.location_label || '-'}</div>
      {:else if key === 'router'}
        <div>{row.router_name || '-'}</div>
      {:else if key === 'lifecycle'}
        {@const summary = getSubscriptionPolicySummary(row)}
        <div class="policy-stack">
          <div>
            <span class="policy-label">Masa aktif</span>
            <div class="sub">
              {summary.activeUntilMissing
                ? 'Belum ada masa aktif akhir'
                : formatCustomerPolicyDate(summary.activeUntilIso)}
            </div>
          </div>
          <div>
            <span class="policy-label">Policy</span>
            <div class="sub">{summary.policyLabel}</div>
          </div>
          <div>
            <span class="policy-label">Perkiraan suspend</span>
            <div class="sub">
              {summary.estimatedSuspendIso
                ? formatCustomerPolicyDate(summary.estimatedSuspendIso)
                : summary.estimatedSuspendMissingReason || '-'}
            </div>
          </div>
        </div>
      {:else if key === 'actions'}
        <div class="row-actions">
          {#if canManageCustomers}
            <button
              class="btn-icon"
              title={$t('admin.customers.billing.actions.generate_from_subscription') || 'Generate invoice'}
              onclick={() => onGenerateInvoice(row.id)}
              disabled={generatingInvoiceFor === row.id || deletingSubscription === row.id}
            >
              <Icon name="file-text" size={16} />
            </button>
            {#if row.status === 'active'}
              <button
                class="btn-icon"
                title="Suspend"
                onclick={() => onSetSubscriptionStatus(row, 'suspended')}
                disabled={togglingSubscription === row.id || deletingSubscription === row.id}
              >
                <Icon name="pause" size={16} />
              </button>
            {:else if row.status === 'suspended'}
              <button
                class="btn-icon"
                title="Resume"
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
              title={$t('common.delete') || 'Delete'}
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
  .observability-head,
  .header-actions {
    display: flex;
    gap: 1rem;
  }

  .section-head,
  .observability-head {
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
  .metric-label,
  .aging-pill,
  .policy-label {
    color: var(--text-secondary);
  }

  .subtitle {
    margin-top: 0.25rem;
  }

  .meta-pill,
  .aging-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    border-radius: 999px;
    padding: 0.28rem 0.62rem;
    font-size: 0.8rem;
    font-weight: 700;
    background: color-mix(in srgb, var(--bg-surface), transparent 12%);
  }

  .lifecycle-observability {
    margin-bottom: 1rem;
    padding: 1rem;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 20%);
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--bg-surface), transparent 8%);
  }

  .observability-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 0.75rem;
  }

  .metric-tile {
    border-radius: 14px;
    padding: 0.85rem 0.9rem;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    background: color-mix(in srgb, var(--bg-surface), transparent 5%);
  }

  .metric-tile.emphasis {
    border-color: rgba(245, 158, 11, 0.35);
    background: rgba(245, 158, 11, 0.08);
  }

  .metric-label {
    display: block;
    font-size: 0.78rem;
    margin-bottom: 0.35rem;
  }

  .metric-tile strong {
    font-size: 1.4rem;
    line-height: 1;
  }

  .aging-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 0.9rem;
  }

  .name {
    font-weight: 650;
  }

  .sub {
    font-size: 0.85rem;
    margin-top: 0.15rem;
  }

  .policy-stack {
    display: grid;
    gap: 0.35rem;
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

  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
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

  .btn-primary {
    background: rgba(99, 102, 241, 0.95);
    border-color: rgba(99, 102, 241, 0.55);
    color: white;
  }

  .btn-icon {
    border-radius: 10px;
    padding: 0.4rem 0.45rem;
  }

  .btn-icon.danger {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgb(239, 68, 68);
  }

  @media (max-width: 900px) {
    .section-head,
    .observability-head {
      flex-direction: column;
      align-items: stretch;
    }

    .header-actions {
      justify-content: stretch;
    }
  }
</style>
