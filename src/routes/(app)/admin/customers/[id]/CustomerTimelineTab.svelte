<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import { t } from 'svelte-i18n';

  let {
    loadingTimeline,
    onRefresh,
    timelineType = $bindable(),
    timelineColumns,
    timelineRows,
    timeAgo,
  } = $props();
</script>

<div class="card section">
  <div class="section-head">
    <div>
      <h3>{$t('admin.customers.tabs.timeline')}</h3>
      <p class="subtitle">{$t('admin.customers.timeline.subtitle')}</p>
    </div>
    <div class="timeline-toolbar">
      <div class="timeline-filters">
        <button class:active={timelineType === 'all'} onclick={() => (timelineType = 'all')}>{$t('common.all')}</button>
        <button class:active={timelineType === 'customer'} onclick={() => (timelineType = 'customer')}>{$t('common.profile')}</button>
        <button class:active={timelineType === 'location'} onclick={() => (timelineType = 'location')}>{$t('common.location')}</button>
        <button class:active={timelineType === 'subscription'} onclick={() => (timelineType = 'subscription')}>{$t('common.subscription')}</button>
      </div>
      <button class="btn btn-secondary" onclick={onRefresh} disabled={loadingTimeline}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh')}
      </button>
    </div>
  </div>
  <Table
    columns={timelineColumns}
    data={timelineRows}
    loading={loadingTimeline}
    emptyText={$t('admin.customers.timeline.empty')}
    pagination
    searchable
    searchPlaceholder={$t('admin.customers.timeline.search')}
    mobileView="card"
  >
    {#snippet cell({ item, key })}
      {#if key === 'created_at'}
        <div class="timeline-table-time">
          <div>{new Date(item.created_at).toLocaleString()}</div>
          <div class="sub">{timeAgo(item.created_at)}</div>
        </div>
      {:else if key === 'action'}
        <div class="timeline-table-action">{item.action}</div>
      {:else if key === 'resource'}
        <span class="pill">{item.resource}</span>
      {:else if key === 'actor'}
        <div class="timeline-table-actor">{item.actor}</div>
      {:else if key === 'details'}
        <div class:subtle-empty={!item.details}>{item.details || ($t('common.no_detail') || 'No detail')}</div>
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
    margin-bottom: 1rem;
  }
  .subtitle,
  .sub,
  .subtle-empty {
    color: var(--text-secondary);
  }
  .subtitle {
    margin-top: 0.25rem;
  }
  .btn {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    min-height: 42px;
    padding: 0.55rem 0.9rem;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    font-weight: 650;
    font-size: 0.9rem;
  }
  .timeline-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem;
  }
  .timeline-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.85rem;
    margin-bottom: 0.75rem;
    padding: 0.85rem;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 16%);
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--bg-surface), transparent 5%);
  }
  .timeline-filters button {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-secondary);
    border-radius: 999px;
    padding: 0.28rem 0.65rem;
    font-size: 0.82rem;
    font-weight: 650;
    cursor: pointer;
  }
  .timeline-filters button.active {
    color: var(--text-primary);
    border-color: rgba(99, 102, 241, 0.45);
    box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.1);
  }
  .timeline-table-time,
  .timeline-table-action,
  .timeline-table-actor {
    display: grid;
    gap: 0.2rem;
  }
  .timeline-table-action,
  .timeline-table-actor {
    font-weight: 560;
  }
  .subtle-empty {
    font-style: italic;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 0.28rem 0.62rem;
    background: color-mix(in srgb, var(--bg-surface), transparent 12%);
  }
  @media (max-width: 900px) {
    .section-head {
      flex-direction: column;
      align-items: stretch;
    }
    .timeline-toolbar {
      flex-direction: column;
      align-items: stretch;
    }
    .timeline-toolbar .btn {
      width: 100%;
    }
  }
</style>
