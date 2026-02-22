<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';

  let {
    globalSummary,
    topIssues = [],
    openIncidentItems = [],
    incidentEvents = [],
    canManage = false,
    selectedMuteMinutes,
    getMaintenanceRemaining,
    onSetTopIssueMuteMinutes,
    onGotoTopIssue,
    onMuteTopIssue,
    onUnmuteTopIssue,
    onOpenIncident,
    onAckIncident,
    onResolveIncident,
    routerLabel,
    formatMetricTs,
    formatIncidentTs,
    kindClass,
    kindLabel,
    formatLatency,
  }: {
    globalSummary: {
      availability: number;
      online: number;
      total: number;
      critical: number;
      warning: number;
      avgLatencyMs: number | null;
    };
    topIssues?: any[];
    openIncidentItems?: any[];
    incidentEvents?: any[];
    canManage?: boolean;
    selectedMuteMinutes: (routerId: string) => number;
    getMaintenanceRemaining: (routerId: string) => string | null;
    onSetTopIssueMuteMinutes: (routerId: string, mins: number) => void;
    onGotoTopIssue: (routerId: string, title: string) => void;
    onMuteTopIssue: (routerId: string, mins: number) => void | Promise<void>;
    onUnmuteTopIssue: (routerId: string) => void | Promise<void>;
    onOpenIncident: (incidentId: string) => void;
    onAckIncident: (incidentId: string) => void | Promise<void>;
    onResolveIncident: (incidentId: string) => void | Promise<void>;
    routerLabel: (routerId: string) => string;
    formatMetricTs: (ts?: string | null) => string;
    formatIncidentTs: (ts: number) => string;
    kindClass: (kind: string) => string;
    kindLabel: (kind: string) => string;
    formatLatency: (ms?: number | null) => string;
  } = $props();
</script>

<div class="slo-strip">
  <div class="slo-card">
    <span class="k">{$t('admin.network.wallboard.slo.availability') || 'Availability'}</span>
    <span class="v mono">{globalSummary.availability.toFixed(1)}%</span>
  </div>
  <div class="slo-card">
    <span class="k">{$t('admin.network.wallboard.slo.routers_online') || 'Routers Online'}</span>
    <span class="v mono">{globalSummary.online}/{globalSummary.total}</span>
  </div>
  <div class="slo-card">
    <span class="k">{$t('admin.network.wallboard.slo.critical') || 'Critical Alerts'}</span>
    <span class="v mono">{globalSummary.critical}</span>
  </div>
  <div class="slo-card">
    <span class="k">{$t('admin.network.wallboard.slo.warning') || 'Warning Alerts'}</span>
    <span class="v mono">{globalSummary.warning}</span>
  </div>
  <div class="slo-card">
    <span class="k">{$t('admin.network.wallboard.slo.avg_latency') || 'Avg Latency'}</span>
    <span class="v mono">{formatLatency(globalSummary.avgLatencyMs)}</span>
  </div>
</div>

<div class="top-issues-strip">
  <div class="top-issues-head">
    <span class="title">{$t('admin.network.wallboard.top_issues.title') || 'Top Issues (1h)'}</span>
    <span class="muted">
      {$t('admin.network.wallboard.top_issues.subtitle') || 'Most frequent unresolved issues'}
    </span>
  </div>
  <div class="top-issues-list">
    {#if topIssues.length === 0}
      <span class="top-issue-empty">
        {$t('admin.network.wallboard.top_issues.empty') || 'No repeated issues in the last hour.'}
      </span>
    {:else}
      {#each topIssues as it (it.key)}
        {@const muteMins = selectedMuteMinutes(it.router_id)}
        {@const maintLeft = getMaintenanceRemaining(it.router_id)}
        <div class="top-issue-item">
          <button
            type="button"
            class="top-issue-main"
            onclick={() => onGotoTopIssue(it.router_id, it.title)}
            title={it.title}
          >
            <span class="mono router">{it.router_name}</span>
            <span class="issue-title">{it.title}</span>
            <span class="issue-count mono">x{it.count}</span>
          </button>
          {#if maintLeft}
            <span class="top-issue-maint" title={$t('admin.network.wallboard.maintenance') || 'Maintenance'}>
              <Icon name="clock" size={13} />
              {$t('admin.network.wallboard.maintenance') || 'Maintenance'} {maintLeft}
            </span>
          {/if}
          {#if canManage}
            <div class="top-issue-actions">
              {#if maintLeft}
                <button
                  type="button"
                  class="btn-mini ghost"
                  onclick={() => void onUnmuteTopIssue(it.router_id)}
                  title={$t('admin.network.wallboard.unmute') || 'Unmute'}
                >
                  <Icon name="x-circle" size={14} />
                  {$t('admin.network.wallboard.unmute') || 'Unmute'}
                </button>
              {/if}
              <select
                value={String(muteMins)}
                onchange={(e) => {
                  const v = Number((e.currentTarget as HTMLSelectElement).value);
                  onSetTopIssueMuteMinutes(it.router_id, v);
                }}
                aria-label={$t('admin.network.wallboard.top_issues.mute_for') || 'Mute duration'}
              >
                <option value="30">30m</option>
                <option value="60">1h</option>
                <option value="240">4h</option>
              </select>
              <button
                type="button"
                class="btn-mini ghost"
                onclick={() => void onMuteTopIssue(it.router_id, muteMins)}
                title={$t('admin.network.wallboard.top_issues.apply_mute') || 'Apply mute'}
              >
                <Icon name="clock" size={14} />
                {$t('admin.network.wallboard.top_issues.apply_mute') || 'Mute'}
              </button>
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>

<div class="top-issues-strip">
  <div class="top-issues-head">
    <span class="title">{$t('admin.network.wallboard.incidents.title') || 'Open Incidents'}</span>
    <span class="muted">
      {$t('admin.network.wallboard.incidents.subtitle') || 'Latest active incidents from monitoring engine'}
    </span>
  </div>
  <div class="top-issues-list">
    {#if openIncidentItems.length === 0}
      <span class="top-issue-empty">
        {$t('admin.network.wallboard.incidents.empty') || 'No active incidents.'}
      </span>
    {:else}
      {#each openIncidentItems as it (it.id)}
        <div class="top-issue-item">
          <button
            type="button"
            class="top-issue-main"
            onclick={() => onOpenIncident(it.id)}
            title={it.message || it.title}
          >
            <span class="mono router">{routerLabel(it.router_id)}</span>
            <span class="issue-title">{it.title}</span>
            <span class="issue-count mono">{String(it.severity || 'info').toUpperCase()}</span>
          </button>
          <span class="muted mono">{formatMetricTs(it.last_seen_at || it.updated_at)}</span>
          {#if canManage}
            <div class="top-issue-actions">
              <button
                type="button"
                class="btn-mini ghost"
                onclick={() => void onAckIncident(it.id)}
                title={$t('admin.network.alerts.actions.ack') || 'Acknowledge'}
              >
                <Icon name="check" size={14} />
                {$t('admin.network.alerts.actions.ack') || 'Acknowledge'}
              </button>
              <button
                type="button"
                class="btn-mini ghost danger"
                onclick={() => void onResolveIncident(it.id)}
                title={$t('admin.network.alerts.actions.resolve') || 'Resolve'}
              >
                <Icon name="check-circle" size={14} />
                {$t('admin.network.alerts.actions.resolve') || 'Resolve'}
              </button>
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>

<div class="timeline-strip">
  <div class="top-issues-head">
    <span class="title">{$t('admin.network.wallboard.timeline.title') || 'Incident Timeline'}</span>
    <span class="muted">
      {$t('admin.network.wallboard.timeline.subtitle') || 'Latest 20 wallboard events'}
    </span>
  </div>
  <div class="timeline-list">
    {#if incidentEvents.length === 0}
      <span class="top-issue-empty">
        {$t('admin.network.wallboard.timeline.empty') || 'No recent events yet.'}
      </span>
    {:else}
      {#each incidentEvents as ev (ev.id)}
        <div class="timeline-item">
          <span class={`timeline-kind ${kindClass(ev.kind)}`}>{kindLabel(ev.kind)}</span>
          <div class="timeline-content">
            <div class="timeline-msg">
              {#if ev.router_id}
                <span class="mono">{routerLabel(ev.router_id)}</span>
                <span class="muted">·</span>
              {/if}
              <span>{ev.message}</span>
            </div>
            <span class="muted mono">{formatIncidentTs(ev.ts)}</span>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .slo-strip {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }
  .slo-card {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 8px 10px;
    background: color-mix(in srgb, var(--bg-surface) 70%, transparent);
    display: grid;
    gap: 3px;
    min-height: 54px;
  }
  .slo-card .k {
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-weight: 900;
    color: var(--text-muted);
  }
  .slo-card .v {
    font-size: 15px;
    font-weight: 900;
    color: var(--text-primary);
    line-height: 1.1;
  }
  .top-issues-strip {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 9px 10px;
    background: color-mix(in srgb, var(--bg-surface) 74%, transparent);
    display: grid;
    gap: 8px;
    min-height: 0;
  }
  .timeline-strip {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 9px 10px;
    background: color-mix(in srgb, var(--bg-surface) 74%, transparent);
    display: grid;
    gap: 8px;
  }
  .top-issues-head {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .top-issues-head .title {
    font-size: 12px;
    font-weight: 900;
    color: var(--text-primary);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .top-issues-list {
    display: grid;
    gap: 7px;
    align-content: start;
  }
  .top-issue-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 7px 8px;
    background: color-mix(in srgb, var(--bg-surface) 68%, transparent);
  }
  .top-issue-main {
    border: none;
    background: transparent;
    color: var(--text-primary);
    padding: 0;
    margin: 0;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    min-width: 0;
    flex: 1;
    text-align: left;
  }
  .top-issue-main .router {
    max-width: 220px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 800;
  }
  .top-issue-main .issue-title {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 12px;
    color: var(--text-muted);
  }
  .top-issue-main .issue-count {
    font-weight: 900;
    color: var(--text-primary);
  }
  .top-issue-empty {
    border: 1px dashed var(--border-color);
    border-radius: 10px;
    padding: 8px 10px;
    color: var(--text-muted);
    font-size: 12px;
  }
  .top-issue-maint {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    border: 1px solid color-mix(in srgb, #f59e0b 45%, var(--border-color));
    border-radius: 999px;
    padding: 4px 8px;
    font-size: 11px;
    font-weight: 800;
    color: color-mix(in srgb, #f59e0b 86%, var(--text-primary));
    background: color-mix(in srgb, #f59e0b 14%, transparent);
    white-space: nowrap;
  }
  .top-issue-actions {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .btn-mini {
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
  .btn-mini:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }
  .btn-mini.ghost {
    background: transparent;
  }
  .btn-mini.danger {
    border-color: color-mix(in srgb, var(--color-danger) 40%, var(--border-color));
    color: color-mix(in srgb, var(--color-danger) 86%, var(--text-primary));
  }
  .btn-mini.danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
  }
  .top-issue-actions select {
    height: 32px;
    min-width: 72px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 72%, transparent);
    color: var(--text-primary);
    padding: 0 8px;
    font-size: 12px;
    font-weight: 700;
    outline: none;
  }
  .timeline-list {
    display: grid;
    gap: 7px;
  }
  .timeline-item {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 8px;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 7px 8px;
    background: color-mix(in srgb, var(--bg-surface) 68%, transparent);
  }
  .timeline-kind {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    padding: 3px 7px;
    font-size: 10px;
    font-weight: 900;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    white-space: nowrap;
    align-self: start;
  }
  .timeline-kind.critical {
    border-color: color-mix(in srgb, #ef4444 55%, var(--border-color));
    color: color-mix(in srgb, #ef4444 90%, var(--text-primary));
    background: color-mix(in srgb, #ef4444 15%, transparent);
  }
  .timeline-kind.warning {
    border-color: color-mix(in srgb, #f59e0b 55%, var(--border-color));
    color: color-mix(in srgb, #f59e0b 90%, var(--text-primary));
    background: color-mix(in srgb, #f59e0b 14%, transparent);
  }
  .timeline-kind.ok {
    border-color: color-mix(in srgb, #22c55e 40%, var(--border-color));
    color: color-mix(in srgb, #22c55e 85%, var(--text-primary));
    background: color-mix(in srgb, #22c55e 12%, transparent);
  }
  .timeline-content {
    min-width: 0;
    display: grid;
    gap: 4px;
  }
  .timeline-msg {
    min-width: 0;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 12px;
    color: var(--text-primary);
  }
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
  }
  .muted {
    color: var(--text-muted);
  }
</style>
