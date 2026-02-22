<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';

  let {
    visibleAlerts = [],
    alertStats,
    alertSeverityFilter = $bindable<'all' | 'critical' | 'warning'>('all'),
    canManage = false,
    onAckVisible,
    onOpenAlerts,
    routerLabel,
  }: {
    visibleAlerts?: any[];
    alertStats: { total: number; critical: number; warning: number };
    alertSeverityFilter?: 'all' | 'critical' | 'warning';
    canManage?: boolean;
    onAckVisible?: () => void | Promise<void>;
    onOpenAlerts: () => void;
    routerLabel: (routerId: string) => string;
  } = $props();
</script>

<div class="alert-strip-head">
  <Icon name="alert-triangle" size={15} />
  <span class="alert-strip-title">
    {$t('admin.network.wallboard.alerts_open') || 'Open alerts'}
  </span>
  <span class="alert-strip-count">{visibleAlerts.length}</span>
</div>
{#if canManage}
  <div class="alert-strip-actions">
    <button class="awp-btn awp-btn-ghost" type="button" onclick={() => void onAckVisible?.()}>
      <Icon name="check" size={14} />
      {$t('admin.network.wallboard.alerts_ack_visible') || 'Ack visible'}
    </button>
  </div>
{/if}
<div class="alert-filter-seg">
  <button
    class:active={alertSeverityFilter === 'all'}
    type="button"
    onclick={() => (alertSeverityFilter = 'all')}
  >
    {$t('admin.network.wallboard.filters.all') || 'All'} ({alertStats.total})
  </button>
  <button
    class:active={alertSeverityFilter === 'critical'}
    type="button"
    onclick={() => (alertSeverityFilter = 'critical')}
  >
    {$t('admin.network.alerts.severity.critical') || 'Critical'} ({alertStats.critical})
  </button>
  <button
    class:active={alertSeverityFilter === 'warning'}
    type="button"
    onclick={() => (alertSeverityFilter = 'warning')}
  >
    {$t('admin.network.alerts.severity.warning') || 'Warning'} ({alertStats.warning})
  </button>
</div>
<div class="alert-strip-list">
  {#each visibleAlerts.slice(0, 8) as a (a.id)}
    <button
      type="button"
      class="alert-chip"
      class:crit={String(a.severity || '').toLowerCase() === 'critical'}
      onclick={onOpenAlerts}
      title={a.message}
    >
      <span class="mono">{routerLabel(a.router_id)}</span>
      <span class="muted">·</span>
      <span>{a.title}</span>
    </button>
  {/each}
</div>

<style>
  .alert-strip-head {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-weight: 800;
    color: var(--text-primary);
  }
  .alert-strip-actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .awp-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 6px 10px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 78%, transparent);
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
  }
  .awp-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }
  .awp-btn-ghost {
    background: transparent;
  }
  .alert-strip-title {
    font-size: 0.86rem;
  }
  .alert-strip-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 20px;
    border-radius: 999px;
    padding: 0 8px;
    font-size: 0.74rem;
    font-weight: 900;
    border: 1px solid color-mix(in srgb, var(--color-warning) 40%, var(--border-color));
    background: color-mix(in srgb, var(--color-warning) 16%, transparent);
  }
  .alert-strip-list {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .alert-filter-seg {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 3px;
    width: fit-content;
    background: color-mix(in srgb, var(--bg-surface) 78%, transparent);
  }
  .alert-filter-seg button {
    border: 1px solid transparent;
    border-radius: 8px;
    padding: 5px 8px;
    background: transparent;
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 800;
    cursor: pointer;
  }
  .alert-filter-seg button.active {
    color: var(--text-primary);
    border-color: color-mix(in srgb, var(--color-warning) 40%, var(--border-color));
    background: color-mix(in srgb, var(--color-warning) 14%, transparent);
  }
  .alert-chip {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 6px 10px;
    background: color-mix(in srgb, var(--bg-surface) 75%, transparent);
    color: var(--text-primary);
    font-size: 0.78rem;
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    max-width: min(100%, 420px);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .alert-chip.crit {
    border-color: color-mix(in srgb, var(--color-danger) 45%, var(--border-color));
    background: color-mix(in srgb, var(--color-danger) 14%, transparent);
  }
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
  }
  .muted {
    color: var(--text-muted);
  }
</style>
