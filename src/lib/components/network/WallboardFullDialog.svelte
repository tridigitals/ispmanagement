<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import type { MetricsRange, MetricsBucket } from './wallboardMetrics';

  type HistRow = { ts: string; rx_bps: number; tx_bps: number };

  let {
    iface = '',
    routerLabel = '',
    routerOnline = false,
    fullTab = 'live',
    metricsRange = '24h',
    metricsFromLocal = '',
    metricsToLocal = '',
    metricsPointIdx = null,
    metricsTooltipX = 0,
    metricsTooltipY = 0,
    metricsSelecting = false,
    metricsSelStart = 0,
    metricsSelCurrent = 0,
    fullMetricsLoading = false,
    fullMetricsError = null,
    rx = [],
    tx = [],
    rxNow = null,
    txNow = null,
    warnRx = false,
    warnTx = false,
    rxPeak = null,
    txPeak = null,
    rxAvg = null,
    txAvg = null,
    metricsBucket = 'raw',
    hasMetricsZoom = false,
    zoomedHistRows = [],
    chartRows = [],
    chartRx = [],
    chartTx = [],
    chartMax = 1,
    histRxPeak = null,
    histTxPeak = null,
    histRxAvg = null,
    histTxAvg = null,
    peakRxIdx = -1,
    peakTxIdx = -1,
    pointIdx = null,
    pointRow = null,
    formatBps,
    formatMetricTs,
    bucketLabel,
    bucketHint,
    onClose,
    onOpenThreshold,
    onOpenFullTab,
    onSetMetricsRange,
    onExportMetricsCsv,
    onClearMetricsZoom,
    onMetricsFromChange,
    onMetricsToChange,
    onBeginMetricsSelection,
    onMoveMetricsSelection,
    onEndMetricsSelection,
    onSetMetricsHoverFromMouse,
    onSetMetricsHoverFromFocus,
    onClearMetricsPoint,
    onSetMetricsPoint,
  }: {
    iface?: string;
    routerLabel?: string;
    routerOnline?: boolean;
    fullTab?: 'live' | 'metrics';
    metricsRange?: MetricsRange;
    metricsFromLocal?: string;
    metricsToLocal?: string;
    metricsPointIdx?: number | null;
    metricsTooltipX?: number;
    metricsTooltipY?: number;
    metricsSelecting?: boolean;
    metricsSelStart?: number;
    metricsSelCurrent?: number;
    fullMetricsLoading?: boolean;
    fullMetricsError?: string | null;
    rx?: number[];
    tx?: number[];
    rxNow?: number | null;
    txNow?: number | null;
    warnRx?: boolean;
    warnTx?: boolean;
    rxPeak?: number | null;
    txPeak?: number | null;
    rxAvg?: number | null;
    txAvg?: number | null;
    metricsBucket?: MetricsBucket;
    hasMetricsZoom?: boolean;
    zoomedHistRows?: HistRow[];
    chartRows?: HistRow[];
    chartRx?: number[];
    chartTx?: number[];
    chartMax?: number;
    histRxPeak?: number | null;
    histTxPeak?: number | null;
    histRxAvg?: number | null;
    histTxAvg?: number | null;
    peakRxIdx?: number;
    peakTxIdx?: number;
    pointIdx?: number | null;
    pointRow?: HistRow | null;
    formatBps: (v?: number | null) => string;
    formatMetricTs: (ts: string | null | undefined) => string;
    bucketLabel: (bucket: MetricsBucket) => string;
    bucketHint: (bucket: MetricsBucket) => string;
    onClose: () => void;
    onOpenThreshold: () => void;
    onOpenFullTab: (tab: 'live' | 'metrics') => void;
    onSetMetricsRange: (range: MetricsRange) => void;
    onExportMetricsCsv: () => void;
    onClearMetricsZoom: () => void;
    onMetricsFromChange: (value: string) => void;
    onMetricsToChange: (value: string) => void;
    onBeginMetricsSelection: (e: PointerEvent) => void;
    onMoveMetricsSelection: (e: PointerEvent) => void;
    onEndMetricsSelection: (e: PointerEvent, rows: HistRow[]) => void;
    onSetMetricsHoverFromMouse: (i: number, e: MouseEvent) => void;
    onSetMetricsHoverFromFocus: (i: number, e: FocusEvent) => void;
    onClearMetricsPoint: () => void;
    onSetMetricsPoint: (i: number) => void;
  } = $props();

  const max = $derived(Math.max(1, ...rx, ...tx));
</script>

<div class="full-overlay" role="dialog" aria-modal="true">
  <button class="full-backdrop" type="button" onclick={onClose} aria-label={$t('common.close') || 'Close'}></button>
  <div class="full">
    <div class="full-head">
      <div class="full-titles">
        <div class="full-kicker">
          {$t('admin.network.wallboard.full_kicker') || 'INTERFACE VIEW'}
        </div>
        <div class="full-title">
          <span class="mono">{iface}</span>
          <span class="muted">·</span>
          <span>{routerLabel}</span>
        </div>
      </div>
      <div class="full-actions">
        <button class="btn-mini" type="button" onclick={(e) => {
          e.stopPropagation();
          onOpenThreshold();
        }}>
          <Icon name="edit" size={16} />
          {$t('common.edit') || 'Edit'}
        </button>
        <button class="icon-x" type="button" onclick={onClose} title={$t('common.close') || 'Close'}>
          <Icon name="x" size={18} />
        </button>
      </div>
    </div>

    <div class="full-body">
      <div class="full-summary-sticky">
        <div class="full-summary-grid">
          <div class="full-summary-item">
            <span class="k">{$t('admin.network.wallboard.summary.status') || 'Status'}</span>
            <span class="v mono">{routerOnline
              ? $t('admin.network.wallboard.summary.online') || 'ONLINE'
              : $t('admin.network.wallboard.summary.offline') || 'OFFLINE'}</span>
          </div>
          <div class="full-summary-item">
            <span class="k">{$t('admin.network.wallboard.summary.rx_now') || 'RX Now'}</span>
            <span class="v mono" class:warn={warnRx}>{formatBps(rxNow)}</span>
          </div>
          <div class="full-summary-item">
            <span class="k">{$t('admin.network.wallboard.summary.tx_now') || 'TX Now'}</span>
            <span class="v mono" class:warn={warnTx}>{formatBps(txNow)}</span>
          </div>
          <div class="full-summary-item">
            <span class="k">{$t('admin.network.wallboard.chart.peak') || 'RX Peak'}</span>
            <span class="v mono">{formatBps(fullTab === 'metrics' ? histRxPeak : rxPeak)}</span>
          </div>
          <div class="full-summary-item">
            <span class="k">{$t('admin.network.wallboard.chart.peak_tx') || 'TX Peak'}</span>
            <span class="v mono">{formatBps(fullTab === 'metrics' ? histTxPeak : txPeak)}</span>
          </div>
          <div class="full-summary-item">
            <span class="k">{fullTab === 'metrics'
              ? $t('admin.network.wallboard.metrics_points') || 'Points'
              : $t('admin.network.wallboard.summary.samples') || 'Samples'}</span>
            <span class="v mono">{fullTab === 'metrics' ? zoomedHistRows.length : rx.length}</span>
          </div>
        </div>
      </div>

      <div class="full-tabs">
        <button class="full-tab {fullTab === 'live' ? 'active' : ''}" type="button" onclick={() => onOpenFullTab('live')}>
          {$t('admin.network.wallboard.tabs.live') || 'Live'}
        </button>
        <button class="full-tab {fullTab === 'metrics' ? 'active' : ''}" type="button" onclick={() => onOpenFullTab('metrics')}>
          {$t('admin.network.wallboard.tabs.metrics') || 'Metrics'}
        </button>
      </div>

      {#if fullTab === 'live'}
        <div class="full-stats">
          <div class="stat-big">
            <div class="k">RX</div>
            <div class="v mono" class:warn={warnRx}>{formatBps(rxNow)}</div>
          </div>
          <div class="stat-big">
            <div class="k">TX</div>
            <div class="v mono" class:warn={warnTx}>{formatBps(txNow)}</div>
          </div>
        </div>

        <div class="spark huge">
          <div class="bars" class:warn={warnRx}>
            {#each rx as v, i (i)}
              <div class="bar rx" style={`height:${Math.round((v / max) * 100)}%;`}></div>
            {/each}
          </div>
          <div class="bars" class:warn={warnTx}>
            {#each tx as v, i (i)}
              <div class="bar tx" style={`height:${Math.round((v / max) * 100)}%;`}></div>
            {/each}
          </div>
        </div>
        <div class="chart-meta chart-meta-big muted">
          <span>{($t('admin.network.wallboard.chart.peak') || 'Peak') + ': ' + formatBps(rxPeak)}</span>
          <span>{($t('admin.network.wallboard.chart.avg') || 'Avg') + ': ' + formatBps(rxAvg)}</span>
          <span>{($t('admin.network.wallboard.chart.peak_tx') || 'TX Peak') + ': ' + formatBps(txPeak)}</span>
          <span>{($t('admin.network.wallboard.chart.avg_tx') || 'TX Avg') + ': ' + formatBps(txAvg)}</span>
        </div>
      {:else}
        <div class="metrics-filters">
          <div class="metrics-toolbar">
            <div class="metrics-range-select">
              <label for="metrics-range" class="muted">{$t('admin.network.wallboard.metrics.range') || 'Range'}</label>
              <select
                id="metrics-range"
                value={metricsRange}
                onchange={(e) => onSetMetricsRange((e.currentTarget as HTMLSelectElement).value as MetricsRange)}
              >
                <option value="24h">{$t('admin.network.wallboard.metrics.range_24h') || 'Last 24 Hours'}</option>
                <option value="7d">{$t('admin.network.wallboard.metrics.range_7d') || 'Last 7 Days'}</option>
                <option value="30d">{$t('admin.network.wallboard.metrics.range_30d') || 'Last 30 Days'}</option>
                <option value="month">{$t('admin.network.wallboard.metrics.range_month') || 'This Month'}</option>
                <option value="custom">{$t('admin.network.wallboard.metrics.range_custom') || 'Custom'}</option>
              </select>
            </div>
            <div
              class="metrics-bucket-chip"
              title={$t('admin.network.wallboard.metrics_agg_title') || 'Aggregation level used for this range'}
            >
              <span class="k">{$t('admin.network.wallboard.metrics_agg_label') || 'Aggregation'}</span>
              <span class="v mono">{bucketLabel(metricsBucket)} ({bucketHint(metricsBucket)})</span>
            </div>
            <button class="btn-mini" type="button" onclick={onExportMetricsCsv}>
              <Icon name="download" size={16} />
              {$t('admin.network.wallboard.metrics.export_csv') || 'Export CSV'}
            </button>
            {#if hasMetricsZoom}
              <button class="btn-mini" type="button" onclick={onClearMetricsZoom}>
                <Icon name="refresh-cw" size={16} />
                {$t('admin.network.wallboard.metrics.reset_zoom') || 'Reset Zoom'}
              </button>
            {/if}
          </div>
          {#if metricsRange === 'custom'}
            <div class="metrics-dates">
              <label>
                <span class="muted">{$t('common.from') || 'From'}</span>
                <input
                  type="datetime-local"
                  value={metricsFromLocal}
                  oninput={(e) => onMetricsFromChange((e.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <label>
                <span class="muted">{$t('common.to') || 'To'}</span>
                <input
                  type="datetime-local"
                  value={metricsToLocal}
                  oninput={(e) => onMetricsToChange((e.currentTarget as HTMLInputElement).value)}
                />
              </label>
            </div>
          {/if}
        </div>

        <div class="full-historical">
          <div class="full-historical-head">
            <div class="full-kicker">{$t('admin.network.wallboard.metrics.historical') || 'Historical Metrics'}</div>
            <span class="muted mono">
              {zoomedHistRows.length} {$t('admin.network.wallboard.metrics_points') || 'points'} ({bucketLabel(metricsBucket)})
              {#if hasMetricsZoom}
                · {$t('admin.network.wallboard.metrics.zoomed') || 'Zoomed'}
              {/if}
            </span>
          </div>

          {#if fullMetricsLoading}
            <div class="muted">{$t('common.loading') || 'Loading...'}</div>
          {:else if fullMetricsError}
            <div class="muted">{fullMetricsError}</div>
          {:else if chartRx.length === 0 && chartTx.length === 0}
            <div class="muted">{$t('admin.network.wallboard.metrics.empty_range') || 'No historical metrics yet for selected date range.'}</div>
          {:else}
            <div
              class="spark huge historical"
              role="application"
              aria-label={$t('admin.network.wallboard.metrics.zoom_area') || 'Metrics chart zoom area'}
              onpointerdown={onBeginMetricsSelection}
              onpointermove={onMoveMetricsSelection}
              onpointerup={(e) => onEndMetricsSelection(e, chartRows)}
              onpointercancel={(e) => onEndMetricsSelection(e, chartRows)}
            >
              {#if metricsSelecting}
                {@const left = Math.min(metricsSelStart, metricsSelCurrent)}
                {@const width = Math.max(0, Math.abs(metricsSelCurrent - metricsSelStart))}
                <div class="metrics-selection" style={`left:${left}px; width:${width}px;`}></div>
              {/if}
              <div class="bars">
                {#if pointIdx != null}
                  <div class="spark-crosshair" style={`--x:${((pointIdx + 0.5) / Math.max(1, chartRx.length)) * 100}%`}></div>
                {/if}
                {#each chartRx as v, i (i)}
                  <div
                    class="bar rx"
                    class:active={pointIdx === i}
                    class:peak={peakRxIdx === i}
                    title={peakRxIdx === i ? (($t('admin.network.wallboard.metrics.peak_marker') || 'Peak') + ' RX') : ''}
                    style={`height:${Math.round((v / chartMax) * 100)}%;`}
                    role="button"
                    tabindex="0"
                    onmouseenter={(e) => onSetMetricsHoverFromMouse(i, e)}
                    onmousemove={(e) => onSetMetricsHoverFromMouse(i, e)}
                    onmouseleave={onClearMetricsPoint}
                    onfocus={(e) => onSetMetricsHoverFromFocus(i, e)}
                    onblur={onClearMetricsPoint}
                    onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && onSetMetricsPoint(i)}
                  ></div>
                {/each}
              </div>
              <div class="bars">
                {#if pointIdx != null}
                  <div class="spark-crosshair" style={`--x:${((pointIdx + 0.5) / Math.max(1, chartTx.length)) * 100}%`}></div>
                {/if}
                {#each chartTx as v, i (i)}
                  <div
                    class="bar tx"
                    class:active={pointIdx === i}
                    class:peak={peakTxIdx === i}
                    title={peakTxIdx === i ? (($t('admin.network.wallboard.metrics.peak_marker') || 'Peak') + ' TX') : ''}
                    style={`height:${Math.round((v / chartMax) * 100)}%;`}
                    role="button"
                    tabindex="0"
                    onmouseenter={(e) => onSetMetricsHoverFromMouse(i, e)}
                    onmousemove={(e) => onSetMetricsHoverFromMouse(i, e)}
                    onmouseleave={onClearMetricsPoint}
                    onfocus={(e) => onSetMetricsHoverFromFocus(i, e)}
                    onblur={onClearMetricsPoint}
                    onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && onSetMetricsPoint(i)}
                  ></div>
                {/each}
              </div>
            </div>
            {#if pointRow}
              <div class="metrics-tooltip floating" style={`left:${metricsTooltipX}px; top:${metricsTooltipY}px;`}>
                <span class="mono">{formatMetricTs(pointRow.ts)}</span>
                <span class="spark-sep">·</span>
                <span>RX: <strong class="mono">{formatBps(pointRow.rx_bps)}</strong></span>
                <span class="spark-sep">·</span>
                <span>TX: <strong class="mono">{formatBps(pointRow.tx_bps)}</strong></span>
              </div>
            {/if}
            <div class="chart-meta chart-meta-big muted">
              <span>{($t('admin.network.wallboard.chart.peak') || 'Peak') + ': ' + formatBps(histRxPeak)}</span>
              <span>{($t('admin.network.wallboard.chart.avg') || 'Avg') + ': ' + formatBps(histRxAvg)}</span>
              <span>{($t('admin.network.wallboard.chart.peak_tx') || 'TX Peak') + ': ' + formatBps(histTxPeak)}</span>
              <span>{($t('admin.network.wallboard.chart.avg_tx') || 'TX Avg') + ': ' + formatBps(histTxAvg)}</span>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .full-overlay {
    position: fixed;
    inset: 0;
    z-index: 70;
    display: grid;
    place-items: center;
  }
  .full-backdrop {
    position: absolute;
    inset: 0;
    border: none;
    background: rgba(0, 0, 0, 0.7);
  }
  .full {
    position: relative;
    width: min(1100px, calc(100vw - 24px));
    max-height: min(860px, calc(100vh - 24px));
    overflow: auto;
    border-radius: 18px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    box-shadow: var(--shadow-lg);
    padding: 14px;
  }
  .full-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }
  .full-kicker {
    color: var(--text-muted);
    letter-spacing: 0.14em;
    font-weight: 900;
    font-size: 11px;
  }
  .full-title {
    margin-top: 6px;
    font-size: 22px;
    font-weight: 950;
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .full-actions {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }
  .full-body {
    display: grid;
    gap: 12px;
  }
  .full-summary-sticky {
    position: sticky;
    top: -2px;
    z-index: 6;
    padding: 2px 0 8px;
    background: linear-gradient(
      to bottom,
      color-mix(in srgb, var(--bg-surface) 96%, transparent),
      color-mix(in srgb, var(--bg-surface) 88%, transparent)
    );
    backdrop-filter: blur(4px);
  }
  .full-summary-grid {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 8px;
  }
  .full-summary-item {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 9px 10px;
    background: color-mix(in srgb, var(--bg-surface) 74%, transparent);
    display: grid;
    gap: 4px;
  }
  .full-summary-item .k {
    font-size: 10px;
    font-weight: 900;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .full-summary-item .v {
    font-size: 14px;
    font-weight: 900;
    color: var(--text-primary);
  }
  .full-summary-item .v.warn {
    color: var(--color-danger);
  }
  .full-tabs {
    display: inline-flex;
    gap: 6px;
    padding: 4px;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    width: fit-content;
    background: color-mix(in srgb, var(--bg-surface) 80%, transparent);
  }
  .full-tab {
    position: relative;
    border: none;
    background: transparent;
    color: color-mix(in srgb, var(--text-primary) 78%, var(--text-muted));
    padding: 8px 12px;
    border-radius: 8px;
    font-weight: 800;
    cursor: pointer;
    transition:
      background 120ms ease,
      color 120ms ease,
      box-shadow 120ms ease,
      transform 120ms ease;
  }
  .full-tab:hover {
    background: color-mix(in srgb, var(--bg-surface) 45%, var(--accent) 10%);
    color: var(--text-primary);
  }
  .full-tab.active {
    background: color-mix(in srgb, var(--accent) 65%, var(--bg-surface));
    color: var(--text-primary);
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--accent) 85%, transparent),
      0 0 0 1px color-mix(in srgb, var(--accent) 32%, transparent);
    transform: translateY(-1px);
  }
  .full-tab.active::after {
    content: '';
    position: absolute;
    left: 10px;
    right: 10px;
    bottom: 3px;
    height: 3px;
    border-radius: 999px;
    background: color-mix(in srgb, #ffffff 75%, var(--accent));
  }
  .metrics-filters {
    display: grid;
    gap: 10px;
  }
  .metrics-toolbar {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }
  .metrics-range-select {
    display: grid;
    gap: 6px;
    max-width: 280px;
  }
  .metrics-bucket-chip {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 8px 10px;
    min-width: 250px;
    background: color-mix(in srgb, var(--bg-surface) 74%, transparent);
    display: grid;
    gap: 2px;
  }
  .metrics-bucket-chip .k {
    font-size: 10px;
    font-weight: 900;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .metrics-bucket-chip .v {
    font-size: 12px;
    font-weight: 800;
    color: var(--text-primary);
  }
  .metrics-range-select select {
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 70%, transparent);
    color: var(--text-primary);
    padding: 9px 10px;
    border-radius: 10px;
    font-size: 13px;
    font-weight: 800;
    outline: none;
  }
  .metrics-dates {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }
  .metrics-dates label {
    display: grid;
    gap: 6px;
    font-size: 12px;
    font-weight: 700;
  }
  .metrics-dates input {
    width: 100%;
    padding: 9px 10px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 72%, transparent);
    color: var(--text-primary);
    outline: none;
  }
  .metrics-tooltip {
    margin-top: 8px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 82%, transparent);
    font-size: 12px;
    color: var(--text-primary);
  }
  .metrics-tooltip.floating {
    position: fixed;
    margin-top: 0;
    z-index: 120;
    pointer-events: none;
    transform: translate(0, 0);
    box-shadow: var(--shadow-md);
  }
  .full-historical {
    border-top: 1px solid var(--border-color);
    padding-top: 12px;
  }
  .full-historical-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 8px;
  }
  .full-stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }
  .stat-big {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 12px;
    background: color-mix(in srgb, var(--bg-surface) 70%, transparent);
  }
  .stat-big .k {
    font-size: 11px;
    letter-spacing: 0.14em;
    font-weight: 900;
    color: var(--text-muted);
    text-transform: uppercase;
  }
  .stat-big .v {
    margin-top: 8px;
    font-weight: 950;
    color: var(--text-primary);
    font-size: 18px;
  }
  .stat-big .v.warn {
    color: var(--color-danger);
  }
  .spark {
    margin-top: 10px;
    position: relative;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    height: 46px;
  }
  .spark.huge {
    height: min(44dvh, 420px);
  }
  .spark.huge.historical {
    height: min(34dvh, 300px);
  }
  .bars {
    position: relative;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    align-items: end;
    gap: 2px;
    height: 100%;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 26px 6px 6px;
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--bg-surface) 85%, transparent),
        color-mix(in srgb, var(--bg-surface) 92%, transparent)
      );
    overflow: hidden;
  }
  .bars.warn {
    border-color: color-mix(in srgb, var(--color-danger) 55%, var(--border-color));
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--color-danger) 12%, transparent),
        color-mix(in srgb, var(--bg-surface) 92%, transparent)
      );
  }
  .bar {
    border-radius: 2px 2px 0 0;
    min-height: 2px;
  }
  .bar.rx {
    background: linear-gradient(180deg, #4fc3f7, #2196f3);
  }
  .bar.tx {
    background: linear-gradient(180deg, #ff8a80, #ef5350);
  }
  .bar.active {
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 55%, transparent);
  }
  .bar.peak {
    outline: 2px solid color-mix(in srgb, var(--accent) 65%, transparent);
    outline-offset: -1px;
  }
  .spark-crosshair {
    position: absolute;
    top: 0;
    bottom: 0;
    left: var(--x);
    width: 1px;
    background: color-mix(in srgb, var(--text-primary) 45%, transparent);
    pointer-events: none;
  }
  .metrics-selection {
    position: absolute;
    top: 0;
    bottom: 0;
    border: 1px solid color-mix(in srgb, var(--accent) 75%, transparent);
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    z-index: 3;
    pointer-events: none;
  }
  .chart-meta {
    margin-top: 9px;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px 10px;
    font-size: 12px;
  }
  .chart-meta span {
    border: 1px solid var(--border-color);
    border-radius: 9px;
    padding: 5px 8px;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
  }
  .chart-meta-big {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
  .btn-mini {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 55%, transparent);
    color: var(--text-primary);
    cursor: pointer;
    font-weight: 850;
    font-size: 13px;
    white-space: nowrap;
    transition:
      border-color 120ms ease,
      background 120ms ease,
      transform 120ms ease;
  }
  .icon-x {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 55%, transparent);
    color: var(--text-primary);
    cursor: pointer;
  }
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
  }
  .muted {
    color: var(--text-muted);
  }
  .spark-sep {
    color: var(--text-muted);
  }

  @media (max-width: 920px) {
    .full-summary-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .metrics-toolbar {
      align-items: stretch;
    }
    .metrics-bucket-chip {
      min-width: 0;
      width: 100%;
    }
    .metrics-dates {
      grid-template-columns: 1fr;
    }
    .full-stats {
      grid-template-columns: 1fr;
    }
    .chart-meta,
    .chart-meta-big {
      grid-template-columns: 1fr;
    }
  }
</style>
