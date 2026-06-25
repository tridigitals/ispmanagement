<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { avgBps, calcTrend, maintenanceRemaining, peakBps, type TrendInfo } from './wallboardUtils';

  type Slot = {
    routerId: string;
    iface: string;
    warn_below_rx_bps?: number | null;
    warn_below_tx_bps?: number | null;
  };

  type RouterRow = {
    id: string;
    name: string;
    identity?: string | null;
    is_online: boolean;
    maintenance_until?: string | null;
  };

  type HoverBar = {
    tileKey: string;
    idx: number;
  } | null;

  let {
    gidx,
    slot,
    router = null,
    rx = [],
    tx = [],
    rxNow = null,
    txNow = null,
    lastSeenAt = null,
    pollFails = 0,
    pollRetrySec = 0,
    routerAlertTotal = 0,
    canManage = false,
    dragOver = null,
    tileMenuIndex = null,
    hoverBar = null,
    paused = false,
    pollMs = 1000,
    renderNow = Date.now(),
    formatBps,
    trendBadgeText,
    trendLabel,
    onStartDragFromTile,
    onOpenFull,
    onOpenThreshold,
    onClearSlot,
    onAckRouterAlerts,
    onOpenAlerts,
    onToggleTileMenu,
    onSetHover,
    onClearHover,
  }: {
    gidx: number;
    slot: Slot;
    router?: RouterRow | null;
    rx?: number[];
    tx?: number[];
    rxNow?: number | null;
    txNow?: number | null;
    lastSeenAt?: number | null;
    pollFails?: number;
    pollRetrySec?: number;
    routerAlertTotal?: number;
    canManage?: boolean;
    dragOver?: number | null;
    tileMenuIndex?: number | null;
    hoverBar?: HoverBar;
    paused?: boolean;
    pollMs?: number;
    renderNow?: number;
    formatBps: (bps?: number | null) => string;
    trendBadgeText: (ti: TrendInfo) => string;
    trendLabel: (ti: TrendInfo) => string;
    onStartDragFromTile: (e: PointerEvent, idx: number) => void;
    onOpenFull: (idx: number) => void;
    onOpenThreshold: (idx: number) => void;
    onClearSlot: (idx: number) => void;
    onAckRouterAlerts: (routerId: string) => void | Promise<void>;
    onOpenAlerts: () => void;
    onToggleTileMenu: (idx: number) => void;
    onSetHover: (tileKey: string, e: PointerEvent) => void;
    onClearHover: (tileKey: string) => void;
  } = $props();

  const iface = $derived(slot.iface);
  const max = $derived(Math.max(1, ...rx, ...tx));
  const rxPeak = $derived(peakBps(rx));
  const txPeak = $derived(peakBps(tx));
  const rxAvg = $derived(avgBps(rx));
  const txAvg = $derived(avgBps(tx));
  const stale = $derived(
    !paused &&
      lastSeenAt != null &&
      Number.isFinite(lastSeenAt) &&
      renderNow - (lastSeenAt as number) > Math.max(10_000, pollMs * 3),
  );
  const warnRx = $derived(
    slot.warn_below_rx_bps != null && rxNow != null && rxNow >= 0 && rxNow < slot.warn_below_rx_bps,
  );
  const warnTx = $derived(
    slot.warn_below_tx_bps != null && txNow != null && txNow >= 0 && txNow < slot.warn_below_tx_bps,
  );
  const maintLeft = $derived(maintenanceRemaining(router?.maintenance_until));
  const pollDegraded = $derived(pollFails >= 3);
  const rxTrend = $derived(calcTrend(rx));
  const txTrend = $derived(calcTrend(tx));
  const tileKey = $derived(`${slot.routerId}:${iface}:${gidx}`);
  const hoverIdx = $derived(
    hoverBar && hoverBar.tileKey === tileKey
      ? Math.min(rx.length ? rx.length - 1 : 0, Math.max(0, hoverBar.idx))
      : null,
  );
  const hoverRx = $derived(hoverIdx != null ? rx[hoverIdx] ?? null : null);
  const hoverTx = $derived(hoverIdx != null ? tx[hoverIdx] ?? null : null);
</script>

<div
  class="tile iface-tile"
  class:warn={warnRx || warnTx}
  class:drag-over={dragOver === gidx}
  data-wall-slot={gidx}
  role="button"
  tabindex="0"
  onpointerdown={(e) => onStartDragFromTile(e, gidx)}
  ondblclick={() => onOpenFull(gidx)}
  onkeydown={(e) => e.key === 'Enter' && onOpenFull(gidx)}
>
  <div class="tile-head">
    <div class="left">
      <div class="name">
        <span class="mono">{iface}</span>
      </div>
      <div class="meta">
        <span class="mono">{router ? (router.identity || router.name) : slot.routerId}</span>
      </div>
    </div>

    <div class="right">
      {#if routerAlertTotal}
        <button
          class="icon-x attn"
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            onOpenAlerts();
          }}
          title={`${routerAlertTotal} ${$t('admin.network.wallboard.alerts_open') || 'open alerts'}`}
        >
          <Icon name="alert-triangle" size={16} />
          <span class="attn-count">{routerAlertTotal}</span>
        </button>
        {#if canManage}
          <button
            class="icon-x"
            type="button"
            onclick={(e) => {
              e.stopPropagation();
              void onAckRouterAlerts(slot.routerId);
            }}
            title={$t('admin.network.wallboard.ack_router_alerts') || 'Acknowledge router alerts'}
          >
            <Icon name="check-circle" size={16} />
          </button>
        {/if}
      {/if}

      <div class="tile-actions">
        <button
          class="icon-x"
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            onToggleTileMenu(gidx);
          }}
          title={$t('common.actions') || 'Actions'}
        >
          <Icon name="list" size={16} />
        </button>
        {#if tileMenuIndex === gidx}
          <div class="tile-menu" role="menu" tabindex="-1">
            <button
              type="button"
              role="menuitem"
              onclick={(e) => {
                e.stopPropagation();
                onToggleTileMenu(-1);
                onOpenFull(gidx);
              }}
            >
              <Icon name="monitor" size={15} />
              {$t('admin.network.wallboard.view') || 'View'}
            </button>
            <button
              type="button"
              role="menuitem"
              onclick={(e) => {
                e.stopPropagation();
                onToggleTileMenu(-1);
                onOpenThreshold(gidx);
              }}
            >
              <Icon name="edit" size={15} />
              {$t('common.edit') || 'Edit'}
            </button>
            <button
              type="button"
              role="menuitem"
              class="danger"
              onclick={(e) => {
                e.stopPropagation();
                onToggleTileMenu(-1);
                onClearSlot(gidx);
              }}
            >
              <Icon name="x" size={15} />
              {$t('common.remove') || 'Remove'}
            </button>
          </div>
        {/if}
      </div>

      {#if stale}
        <span class="badge warn" title={$t('admin.network.wallboard.stale') || 'Data stale'}>
          <Icon name="alert-triangle" size={14} />
          {$t('admin.network.wallboard.stale') || 'Stale'}
        </span>
      {/if}
      {#if maintLeft}
        <span class="badge maintenance" title={($t('admin.network.wallboard.maintenance') || 'Maintenance') + ` ${maintLeft}`}>
          <Icon name="clock" size={13} />
          {$t('admin.network.wallboard.maintenance') || 'Maintenance'} {maintLeft}
        </span>
      {/if}
      {#if pollDegraded}
        <span class="badge poll-err" title={`${$t('admin.network.wallboard.poll_error') || 'Poll error'} (${pollFails}x)`}>
          <Icon name="wifi-off" size={13} />
          {($t('admin.network.wallboard.poll_error') || 'Poll error') + ` ${pollFails}x`}
          {#if pollRetrySec > 0}
            <span class="mono">({pollRetrySec}s)</span>
          {/if}
        </span>
      {/if}
      <span
        class="badge status-dot"
        class:ok={router?.is_online}
        class:bad={!router?.is_online}
        title={router?.is_online
          ? $t('admin.network.routers.badges.online') || 'Online'
          : $t('admin.network.routers.badges.offline') || 'Offline'}
        aria-label={router?.is_online
          ? $t('admin.network.routers.badges.online') || 'Online'
          : $t('admin.network.routers.badges.offline') || 'Offline'}
      >
        <span class="dot"></span>
      </span>
    </div>
  </div>

  <div class="tile-body">
    <div class="spark wide">
      <div class="bars" class:warn={warnRx}>
        <div class="spark-panel-title">
          <span class="spark-chip">RX</span>
          <div class="spark-rate">
            <span class="mono rate" class:warn={warnRx}>{formatBps(rxNow)}</span>
            <span
              class="trend-chip"
              class:up={rxTrend.dir === 'up'}
              class:down={rxTrend.dir === 'down'}
              class:flat={rxTrend.dir === 'flat'}
              title={trendLabel(rxTrend)}
            >
              {trendBadgeText(rxTrend)}
            </span>
          </div>
        </div>
        {#if hoverIdx != null}
          <div class="spark-crosshair" style={`--x:${((hoverIdx + 0.5) / Math.max(1, rx.length)) * 100}%`}></div>
        {/if}
        {#each rx as v, i (i)}
          <div class="bar rx" class:active={hoverIdx === i} style={`height:${Math.round((v / max) * 100)}%;`} data-idx={i}></div>
        {/each}
      </div>
      <div class="bars" class:warn={warnTx}>
        <div class="spark-panel-title">
          <span class="spark-chip">TX</span>
          <div class="spark-rate">
            <span class="mono rate" class:warn={warnTx}>{formatBps(txNow)}</span>
            <span
              class="trend-chip"
              class:up={txTrend.dir === 'up'}
              class:down={txTrend.dir === 'down'}
              class:flat={txTrend.dir === 'flat'}
              title={trendLabel(txTrend)}
            >
              {trendBadgeText(txTrend)}
            </span>
          </div>
        </div>
        {#if hoverIdx != null}
          <div class="spark-crosshair" style={`--x:${((hoverIdx + 0.5) / Math.max(1, tx.length)) * 100}%`}></div>
        {/if}
        {#each tx as v, i (i)}
          <div class="bar tx" class:active={hoverIdx === i} style={`height:${Math.round((v / max) * 100)}%;`} data-idx={i}></div>
        {/each}
      </div>

      <div
        class="spark-hover"
        role="presentation"
        aria-hidden="true"
        onpointermove={(e) => onSetHover(tileKey, e)}
        onpointerleave={() => onClearHover(tileKey)}
      >
        {#if hoverIdx != null}
          <div class="spark-tooltip" role="status" aria-live="polite">
            <span class="spark-chip">RX</span>
            <span class="mono">{formatBps(hoverRx)}</span>
            <span class="spark-sep">·</span>
            <span class="spark-chip">TX</span>
            <span class="mono">{formatBps(hoverTx)}</span>
          </div>
        {/if}
      </div>
    </div>

    <div class="chart-meta muted">
      <span>{($t('admin.network.wallboard.chart.peak') || 'Peak') + ': ' + formatBps(rxPeak)}</span>
      <span>{($t('admin.network.wallboard.chart.avg') || 'Avg') + ': ' + formatBps(rxAvg)}</span>
      <span>{($t('admin.network.wallboard.chart.peak_tx') || 'TX Peak') + ': ' + formatBps(txPeak)}</span>
      <span>{($t('admin.network.wallboard.chart.avg_tx') || 'TX Avg') + ': ' + formatBps(txAvg)}</span>
    </div>
  </div>
</div>

<style>
  /* ── NOC Glassmorphism Dark Theme ── */

  @keyframes pulse-glow {
    0%, 100% { box-shadow: 0 0 4px 1px rgba(46, 204, 113, 0.5); }
    50% { box-shadow: 0 0 8px 3px rgba(46, 204, 113, 0.8); }
  }

  .tile {
    position: relative;
    border: 1px solid color-mix(in srgb, var(--border-color) 60%, transparent);
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--bg-surface) 55%, transparent);
    backdrop-filter: blur(18px) saturate(1.3);
    -webkit-backdrop-filter: blur(18px) saturate(1.3);
    overflow: hidden;
    min-height: 0;
    box-shadow:
      0 0 0 1px color-mix(in srgb, var(--accent) 6%, transparent),
      0 4px 24px -4px rgba(0, 0, 0, 0.5),
      inset 0 1px 0 color-mix(in srgb, var(--text-primary) 6%, transparent);
    transition:
      border-color 0.3s ease,
      box-shadow 0.3s ease,
      transform 0.2s ease;
  }
  .tile.iface-tile { cursor: pointer; }
  .tile.iface-tile:hover {
    border-color: color-mix(in srgb, var(--accent) 40%, var(--border-color));
    box-shadow:
      0 0 0 1px color-mix(in srgb, var(--accent) 15%, transparent),
      0 0 16px -2px color-mix(in srgb, var(--accent) 20%, transparent),
      0 8px 32px -6px rgba(0, 0, 0, 0.6),
      inset 0 1px 0 color-mix(in srgb, var(--accent) 12%, transparent);
    transform: translateY(-1px);
  }
  .tile.iface-tile.warn {
    border-color: color-mix(in srgb, var(--color-danger) 50%, var(--border-color));
    box-shadow:
      0 0 0 1px color-mix(in srgb, var(--color-danger) 15%, transparent),
      0 0 20px -4px color-mix(in srgb, var(--color-danger) 25%, transparent),
      inset 0 1px 0 color-mix(in srgb, var(--color-danger) 10%, transparent);
  }
  .tile.drag-over {
    outline: 2px dashed color-mix(in srgb, var(--accent) 65%, transparent);
    outline-offset: 4px;
  }

  /* ── Header ── */
  .tile-head {
    padding: 14px 14px 10px;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
    border-bottom: 1px solid color-mix(in srgb, var(--border-color) 40%, transparent);
    background: color-mix(in srgb, var(--text-primary) 2%, transparent);
  }
  .tile-body { padding: 14px; }
  .name {
    font-weight: 800;
    font-size: 15px;
    line-height: 1.25;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
    letter-spacing: 0.01em;
  }
  .name .mono {
    display: block;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    margin-top: 3px;
    color: var(--text-muted);
    font-size: 11px;
    letter-spacing: 0.02em;
    opacity: 0.75;
  }
  .right { display: inline-flex; align-items: center; gap: 6px; }

  /* ── Icon Buttons ── */
  .icon-x {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 10px;
    border: 1px solid color-mix(in srgb, var(--border-color) 50%, transparent);
    background: color-mix(in srgb, var(--bg-surface) 40%, transparent);
    backdrop-filter: blur(8px);
    color: var(--text-primary);
    cursor: pointer;
    transition: all 0.2s ease;
  }
  .icon-x.attn {
    border-color: color-mix(in srgb, var(--color-warning) 40%, var(--border-color));
    color: color-mix(in srgb, var(--color-warning) 85%, var(--text-primary));
    gap: 6px;
    padding-inline: 8px;
    min-width: 42px;
  }
  .attn-count { font-size: 11px; font-weight: 900; line-height: 1; }
  .icon-x:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border-color));
    background: color-mix(in srgb, var(--accent) 15%, var(--bg-surface));
    box-shadow: 0 0 8px -2px color-mix(in srgb, var(--accent) 30%, transparent);
  }

  /* ── Tile Menu ── */
  .tile-actions { position: relative; }
  .tile-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    min-width: 140px;
    padding: 6px;
    border-radius: 12px;
    border: 1px solid color-mix(in srgb, var(--border-color) 50%, transparent);
    background: color-mix(in srgb, var(--bg-surface) 70%, rgba(0, 0, 0, 0.6));
    backdrop-filter: blur(20px) saturate(1.4);
    -webkit-backdrop-filter: blur(20px) saturate(1.4);
    box-shadow:
      0 8px 32px -4px rgba(0, 0, 0, 0.6),
      inset 0 1px 0 color-mix(in srgb, var(--text-primary) 6%, transparent);
    display: grid;
    gap: 4px;
    z-index: 20;
  }
  .tile-menu button {
    border: none;
    background: transparent;
    color: var(--text-primary);
    border-radius: 9px;
    padding: 8px 9px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s ease;
  }
  .tile-menu button:hover {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .tile-menu button.danger {
    color: color-mix(in srgb, var(--color-danger) 85%, var(--text-primary));
  }
  .tile-menu button.danger:hover {
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
  }

  /* ── Badges ── */
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 9px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--border-color) 50%, transparent);
    font-weight: 800;
    font-size: 10px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    background: color-mix(in srgb, var(--bg-surface) 40%, transparent);
    backdrop-filter: blur(6px);
    transition: all 0.2s ease;
  }
  .badge .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    transition: all 0.3s ease;
  }
  .badge.ok .dot {
    background: #2ecc71;
    box-shadow: 0 0 6px 1px rgba(46, 204, 113, 0.5);
    animation: pulse-glow 2s ease-in-out infinite;
  }
  .badge.bad .dot {
    background: #ff6b6b;
    opacity: 0.5;
    box-shadow: none;
    animation: none;
  }
  .badge.status-dot { padding: 6px; min-width: 0; gap: 0; }
  .badge.warn {
    border-color: color-mix(in srgb, var(--color-warning) 45%, var(--border-color));
    color: color-mix(in srgb, var(--color-warning) 85%, var(--text-primary));
    background: color-mix(in srgb, var(--color-warning) 10%, transparent);
  }
  .badge.maintenance {
    border-color: color-mix(in srgb, #f59e0b 40%, var(--border-color));
    color: color-mix(in srgb, #f59e0b 88%, var(--text-primary));
    background: color-mix(in srgb, #f59e0b 10%, transparent);
    gap: 5px;
  }
  .badge.poll-err {
    border-color: color-mix(in srgb, #ef4444 40%, var(--border-color));
    color: color-mix(in srgb, #ef4444 90%, var(--text-primary));
    background: color-mix(in srgb, #ef4444 10%, transparent);
    gap: 5px;
  }

  /* ── Sparkline Area ── */
  .spark {
    margin-top: 10px;
    position: relative;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    height: 46px;
  }
  .spark.wide { height: 120px; }
  .bars {
    position: relative;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    align-items: end;
    gap: 2px;
    height: 100%;
    border: 1px solid color-mix(in srgb, var(--border-color) 40%, transparent);
    border-radius: 12px;
    padding: 28px 6px 6px;
    background: color-mix(in srgb, var(--bg-surface) 35%, rgba(0, 0, 0, 0.3));
    backdrop-filter: blur(12px);
    overflow: hidden;
    transition: border-color 0.3s ease, box-shadow 0.3s ease;
  }
  .bars:hover {
    border-color: color-mix(in srgb, var(--accent) 25%, var(--border-color));
  }
  .bars.warn {
    border-color: color-mix(in srgb, var(--color-danger) 40%, var(--border-color));
    box-shadow: inset 0 0 20px -8px color-mix(in srgb, var(--color-danger) 15%, transparent);
  }

  /* ── Individual Bar ── */
  .bar {
    border-radius: 4px 4px 1px 1px;
    min-height: 2px;
    transition: height 160ms linear, filter 0.2s ease, box-shadow 0.2s ease;
  }
  .bar.rx {
    background: linear-gradient(to top, #22d3ee, #2563eb);
    box-shadow: 0 0 6px -1px color-mix(in srgb, #22d3ee 35%, transparent);
  }
  .bar.tx {
    background: linear-gradient(to top, #fb7185, #f97316);
    box-shadow: 0 0 6px -1px color-mix(in srgb, #fb7185 35%, transparent);
  }
  .bar.active {
    filter: brightness(1.25) saturate(1.2);
    box-shadow: 0 0 10px -1px color-mix(in srgb, var(--text-primary) 20%, transparent);
  }

  /* ── Spark Panel Title ── */
  .spark-panel-title {
    position: absolute;
    left: 8px;
    top: 6px;
    right: 8px;
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    z-index: 2;
  }
  .spark-chip {
    display: inline-flex;
    align-items: center;
    padding: 2px 7px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--border-color) 45%, transparent);
    color: var(--text-muted);
    font-weight: 800;
    font-size: 9px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    background: color-mix(in srgb, var(--bg-surface) 50%, transparent);
    backdrop-filter: blur(4px);
  }
  .spark-rate { display: inline-flex; align-items: center; gap: 6px; }
  .rate {
    font-size: 14px;
    font-weight: 800;
    letter-spacing: -0.01em;
  }
  .rate.warn {
    color: var(--color-danger);
    font-weight: 950;
    text-shadow: 0 0 8px color-mix(in srgb, var(--color-danger) 40%, transparent);
  }

  /* ── Trend Chips ── */
  .trend-chip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--border-color) 45%, transparent);
    min-height: 18px;
    padding: 1px 6px;
    font-size: 9px;
    font-weight: 900;
    letter-spacing: 0.03em;
    color: var(--text-muted);
    background: color-mix(in srgb, var(--bg-surface) 50%, transparent);
    white-space: nowrap;
    transition: all 0.2s ease;
  }
  .trend-chip.up {
    border-color: color-mix(in srgb, #22c55e 40%, var(--border-color));
    color: #22c55e;
    background: color-mix(in srgb, #22c55e 10%, transparent);
    text-shadow: 0 0 6px color-mix(in srgb, #22c55e 30%, transparent);
  }
  .trend-chip.down {
    border-color: color-mix(in srgb, #f97316 40%, var(--border-color));
    color: #f97316;
    background: color-mix(in srgb, #f97316 10%, transparent);
    text-shadow: 0 0 6px color-mix(in srgb, #f97316 30%, transparent);
  }

  /* ── Spark Hover / Tooltip / Crosshair ── */
  .spark-crosshair {
    position: absolute;
    top: 24px;
    bottom: 4px;
    width: 1px;
    left: var(--x);
    transform: translateX(-0.5px);
    background: color-mix(in srgb, var(--text-primary) 30%, transparent);
    pointer-events: none;
    z-index: 3;
  }
  .spark-hover { position: absolute; inset: 0; z-index: 4; }
  .spark-tooltip {
    position: absolute;
    left: 10px;
    bottom: 10px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: 10px;
    border: 1px solid color-mix(in srgb, var(--border-color) 50%, transparent);
    background: color-mix(in srgb, var(--bg-surface) 70%, rgba(0, 0, 0, 0.5));
    backdrop-filter: blur(16px) saturate(1.3);
    -webkit-backdrop-filter: blur(16px) saturate(1.3);
    color: var(--text-primary);
    font-size: 11px;
    font-weight: 750;
    pointer-events: none;
    box-shadow:
      0 4px 16px -2px rgba(0, 0, 0, 0.5),
      inset 0 1px 0 color-mix(in srgb, var(--text-primary) 5%, transparent);
  }
  .spark-sep { color: var(--text-muted); opacity: 0.5; }

  /* ── Chart Meta ── */
  .chart-meta {
    margin-top: 10px;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 5px 8px;
    font-size: 11px;
    letter-spacing: 0.01em;
  }
  .chart-meta span {
    border: 1px solid color-mix(in srgb, var(--border-color) 35%, transparent);
    border-radius: 8px;
    padding: 5px 8px;
    background: color-mix(in srgb, var(--bg-surface) 35%, transparent);
    backdrop-filter: blur(6px);
    transition: border-color 0.2s ease;
  }
  .chart-meta span:hover {
    border-color: color-mix(in srgb, var(--accent) 20%, var(--border-color));
  }

  /* ── Utility ── */
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
  }
  .muted { color: var(--text-muted); }
</style>
