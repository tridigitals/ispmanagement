<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import WallboardInsightsControls from '$lib/components/network/WallboardInsightsControls.svelte';
  import WallboardInsightsSummary from '$lib/components/network/WallboardInsightsSummary.svelte';
  import WallboardAlertsPanel from '$lib/components/network/WallboardAlertsPanel.svelte';
  import type { LayoutPreset } from '$lib/constants/wallboard';

  type AlertSeverityFilter = 'all' | 'critical' | 'warning';

  let {
    insightsOpen = $bindable(false),
    alertsOpen = $bindable(false),
    alertSeverityFilter = $bindable<AlertSeverityFilter>('all'),
    insightsBadge,
    sortedAlerts = [],
    refreshing,
    paused,
    isFullscreen,
    criticalSoundEnabled,
    pollMs = $bindable(1000),
    layout = $bindable<LayoutPreset>('3x3'),
    onRefresh,
    onTogglePaused,
    onToggleFullscreen,
    onToggleCriticalSound,
    onExit,
    globalSummary,
    topIssues,
    openIncidentItems,
    incidentEvents,
    canManage,
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
    visibleAlerts,
    alertStats,
    onAckVisible,
    onOpenAlerts,
    onToggleAlertsPanel,
  }: {
    insightsOpen?: boolean;
    alertsOpen?: boolean;
    alertSeverityFilter?: AlertSeverityFilter;
    pollMs?: number;
    layout?: LayoutPreset;
    [key: string]: any;
  } = $props();
</script>

{#if insightsOpen}
  <button
    class="insights-backdrop"
    type="button"
    onclick={() => (insightsOpen = false)}
    aria-label={$t('common.close') || 'Close'}
  ></button>
  <aside class="wall-insights" aria-label={$t('admin.network.wallboard.settings') || 'Settings'}>
    <div class="insights-head">
      <span class="title">{$t('admin.network.wallboard.settings') || 'Settings'}</span>
      <button class="icon-x" type="button" onclick={() => (insightsOpen = false)} title={$t('common.close') || 'Close'}>
        <Icon name="x" size={16} />
      </button>
    </div>

    <WallboardInsightsControls
      bind:pollMs
      bind:layout
      {refreshing}
      {paused}
      {isFullscreen}
      {criticalSoundEnabled}
      onRefresh={onRefresh}
      onTogglePaused={onTogglePaused}
      onToggleFullscreen={onToggleFullscreen}
      onToggleCriticalSound={onToggleCriticalSound}
      onExit={onExit}
    />

    <WallboardInsightsSummary
      {globalSummary}
      {topIssues}
      {openIncidentItems}
      {incidentEvents}
      {canManage}
      {selectedMuteMinutes}
      {getMaintenanceRemaining}
      {onSetTopIssueMuteMinutes}
      {onGotoTopIssue}
      {onMuteTopIssue}
      {onUnmuteTopIssue}
      {onOpenIncident}
      {onAckIncident}
      {onResolveIncident}
      {routerLabel}
      {formatMetricTs}
      {formatIncidentTs}
      {kindClass}
      {kindLabel}
      {formatLatency}
    />
  </aside>
{/if}

{#if sortedAlerts.length > 0 && alertsOpen}
  <div id="wallboard-alert-panel" class="alert-strip floating-alert-panel">
    <WallboardAlertsPanel
      bind:alertSeverityFilter
      {visibleAlerts}
      {alertStats}
      {canManage}
      onAckVisible={onAckVisible}
      onOpenAlerts={onOpenAlerts}
      {routerLabel}
    />
  </div>
{/if}

{#if sortedAlerts.length > 0}
  <button
    class="floating-alert-btn"
    class:open={alertsOpen}
    type="button"
    onclick={onToggleAlertsPanel}
    aria-expanded={alertsOpen}
    aria-controls="wallboard-alert-panel"
    aria-label={$t('admin.network.wallboard.alerts_open') || 'Open alerts'}
    title={$t('admin.network.wallboard.alerts_open') || 'Open alerts'}
  >
    <Icon name="alert-triangle" size={17} />
    <span class="floating-alert-count">{sortedAlerts.length > 99 ? '99+' : sortedAlerts.length}</span>
  </button>
{/if}

<style>
  .insights-backdrop {
    position: fixed;
    inset: 0;
    z-index: 68;
    border: none;
    background: rgba(0, 0, 0, 0.35);
  }
  .wall-insights {
    position: fixed;
    top: 82px;
    right: 18px;
    bottom: 18px;
    width: min(440px, calc(100vw - 36px));
    z-index: 69;
    display: grid;
    grid-template-rows: auto auto auto 1fr;
    gap: 10px;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 10px;
    background: color-mix(in srgb, var(--bg-surface) 90%, transparent);
    box-shadow: var(--shadow-lg);
    overflow: auto;
  }
  .insights-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .insights-head .title {
    font-size: 12px;
    font-weight: 900;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-primary);
  }
  .alert-strip {
    display: grid;
    gap: 8px;
    margin-bottom: 12px;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 10px 12px;
    background: color-mix(in srgb, var(--bg-surface) 70%, transparent);
  }
  .alert-strip.floating-alert-panel {
    position: fixed;
    right: 18px;
    bottom: 68px;
    z-index: 74;
    width: min(460px, calc(100vw - 36px));
    margin-bottom: 0;
    border-color: color-mix(in srgb, var(--color-warning) 45%, var(--border-color));
    background: color-mix(in srgb, var(--color-warning) 10%, var(--bg-surface));
    box-shadow: var(--shadow-lg);
    max-height: min(38vh, 340px);
    overflow: auto;
  }
  .floating-alert-btn {
    position: fixed;
    right: 18px;
    bottom: 18px;
    z-index: 75;
    width: 42px;
    height: 42px;
    border-radius: 13px;
    border: 1px solid color-mix(in srgb, var(--color-warning) 45%, var(--border-color));
    background: color-mix(in srgb, var(--color-warning) 12%, var(--bg-surface));
    color: color-mix(in srgb, var(--color-warning) 88%, var(--text-primary));
    display: grid;
    place-items: center;
    padding: 0;
    cursor: pointer;
    box-shadow: var(--shadow-md);
  }
  .floating-alert-btn.open {
    border-color: color-mix(in srgb, var(--color-warning) 65%, var(--border-color));
    background: color-mix(in srgb, var(--color-warning) 18%, var(--bg-surface));
  }
  .floating-alert-count {
    position: absolute;
    top: -6px;
    right: -6px;
    min-width: 18px;
    height: 18px;
    border-radius: 999px;
    padding: 0 5px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid color-mix(in srgb, var(--color-warning) 35%, var(--border-color));
    background: color-mix(in srgb, var(--color-warning) 20%, var(--bg-surface));
    color: color-mix(in srgb, var(--color-warning) 95%, var(--text-primary));
    font-size: 10px;
    font-weight: 900;
    line-height: 1;
  }
  @media (max-width: 900px) {
    .wall-insights {
      top: 70px;
      right: 12px;
      left: 12px;
      bottom: 72px;
      width: auto;
    }
    .alert-strip.floating-alert-panel {
      right: 12px;
      left: 12px;
      bottom: 64px;
      width: auto;
      max-height: 42vh;
    }
    .floating-alert-btn {
      right: 12px;
      bottom: 12px;
    }
  }
</style>
