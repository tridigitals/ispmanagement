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
  .tile {
    position: relative;
    border: 1px solid var(--border-color);
    border-radius: 18px;
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-surface) 72%, transparent), color-mix(in srgb, var(--bg-surface) 92%, transparent));
    overflow: hidden;
    min-height: 0;
  }
  .tile.iface-tile { cursor: pointer; }
  .tile.iface-tile.warn {
    border-color: color-mix(in srgb, var(--color-danger) 55%, var(--border-color));
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-danger) 20%, transparent);
  }
  .tile.drag-over { outline: 2px dashed color-mix(in srgb, var(--accent) 65%, transparent); outline-offset: 4px; }
  .tile-head {
    padding: 14px 14px 10px;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
    border-bottom: 1px solid var(--border-color);
  }
  .tile-body { padding: 14px; }
  .name {
    font-weight: 800;
    font-size: 16px;
    line-height: 1.2;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }
  .name .mono { display: block; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .meta { margin-top: 4px; color: var(--text-muted); font-size: 12px; }
  .right { display: inline-flex; align-items: center; gap: 8px; }
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
  .icon-x.attn {
    border-color: color-mix(in srgb, var(--color-warning) 45%, var(--border-color));
    color: color-mix(in srgb, var(--color-warning) 80%, var(--text-primary));
    gap: 6px;
    padding-inline: 8px;
    min-width: 42px;
  }
  .attn-count { font-size: 11px; font-weight: 900; line-height: 1; }
  .icon-x:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border-color));
    background: color-mix(in srgb, var(--accent) 10%, var(--bg-surface));
  }
  .tile-actions { position: relative; }
  .tile-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    min-width: 140px;
    padding: 6px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 92%, transparent);
    box-shadow: var(--shadow-md);
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
  }
  .tile-menu button:hover { background: color-mix(in srgb, var(--bg-surface) 60%, var(--accent) 10%); }
  .tile-menu button.danger { color: color-mix(in srgb, var(--color-danger) 82%, var(--text-primary)); }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    font-weight: 800;
    font-size: 12px;
    letter-spacing: 0.02em;
  }
  .badge .dot { width: 8px; height: 8px; border-radius: 999px; box-shadow: none; }
  .badge.ok .dot { background: #2ecc71; }
  .badge.bad .dot { background: #ff6b6b; }
  .badge.status-dot { padding: 6px; min-width: 0; gap: 0; }
  .badge.warn {
    border-color: color-mix(in srgb, var(--color-warning) 55%, var(--border-color));
    color: color-mix(in srgb, var(--color-warning) 85%, var(--text-primary));
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
  }
  .badge.maintenance {
    border-color: color-mix(in srgb, #f59e0b 50%, var(--border-color));
    color: color-mix(in srgb, #f59e0b 88%, var(--text-primary));
    background: color-mix(in srgb, #f59e0b 14%, transparent);
    gap: 6px;
  }
  .badge.poll-err {
    border-color: color-mix(in srgb, #ef4444 50%, var(--border-color));
    color: color-mix(in srgb, #ef4444 90%, var(--text-primary));
    background: color-mix(in srgb, #ef4444 14%, transparent);
    gap: 6px;
  }
  .spark {
    margin-top: 10px;
    position: relative;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    height: 46px;
  }
  .spark.wide { height: 112px; }
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
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-surface) 72%, transparent), color-mix(in srgb, var(--bg-surface) 92%, transparent));
    overflow: hidden;
  }
  .bars.warn {
    border-color: color-mix(in srgb, var(--color-danger) 45%, var(--border-color));
    background: linear-gradient(180deg, color-mix(in srgb, var(--color-danger) 8%, transparent), color-mix(in srgb, var(--bg-surface) 90%, transparent));
  }
  .bar { border-radius: 6px 6px 0 0; min-height: 2px; transition: height 160ms linear; }
  .bar.rx { background: color-mix(in srgb, #22d3ee 65%, #2563eb 35%); }
  .bar.tx { background: color-mix(in srgb, #fb7185 65%, #f97316 35%); }
  .bar.active { filter: brightness(1.16); box-shadow: 0 0 0 1px color-mix(in srgb, var(--text-primary) 32%, transparent); }
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
    padding: 2px 6px;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    color: var(--text-muted);
    font-weight: 800;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    background: color-mix(in srgb, var(--bg-surface) 72%, transparent);
  }
  .spark-rate { display: inline-flex; align-items: center; gap: 6px; }
  .trend-chip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    min-height: 20px;
    padding: 2px 7px;
    font-size: 10px;
    font-weight: 900;
    letter-spacing: 0.02em;
    color: var(--text-muted);
    background: color-mix(in srgb, var(--bg-surface) 72%, transparent);
    white-space: nowrap;
  }
  .trend-chip.up {
    border-color: color-mix(in srgb, #22c55e 48%, var(--border-color));
    color: #22c55e;
    background: color-mix(in srgb, #22c55e 14%, transparent);
  }
  .trend-chip.down {
    border-color: color-mix(in srgb, #f97316 48%, var(--border-color));
    color: #f97316;
    background: color-mix(in srgb, #f97316 14%, transparent);
  }
  .rate.warn { color: var(--color-danger); font-weight: 950; }
  .spark-crosshair {
    position: absolute;
    top: 22px;
    bottom: 4px;
    width: 1px;
    left: var(--x);
    transform: translateX(-0.5px);
    background: color-mix(in srgb, var(--text-primary) 35%, transparent);
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
    padding: 6px 10px;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 90%, transparent);
    color: var(--text-primary);
    font-size: 11px;
    font-weight: 750;
    pointer-events: none;
    box-shadow: var(--shadow-sm);
  }
  .spark-sep { color: var(--text-muted); }
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
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
  }
  .muted { color: var(--text-muted); }
</style>
