<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page as pageStore } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can, user, tenant } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import WallboardFullDialog from '$lib/components/network/WallboardFullDialog.svelte';
  import WallboardInterfaceTile from '$lib/components/network/WallboardInterfaceTile.svelte';
  import WallboardSlotPicker from '$lib/components/network/WallboardSlotPicker.svelte';
  import WallboardTopPager from '$lib/components/network/WallboardTopPager.svelte';
  import WallboardThresholdDialog from '$lib/components/network/WallboardThresholdDialog.svelte';
  import WallboardAlertsPanel from '$lib/components/network/WallboardAlertsPanel.svelte';
  import {
    ackWallboardAlerts,
    ackWallboardIncident,
    loadWallboardAlerts,
    loadWallboardIncidents,
    muteWallboardRouter,
    resolveWallboardIncident,
    unmuteWallboardRouter,
  } from '$lib/components/network/wallboardAlertsActions';
  import WallboardInsightsControls from '$lib/components/network/WallboardInsightsControls.svelte';
  import WallboardInsightsSummary from '$lib/components/network/WallboardInsightsSummary.svelte';
import {
    aggregateHistPoints,
    applyMetricsZoom,
    buildHistPoints,
    downsampleHistPoints,
    filterMetricsRowsByRange,
    requiredMetricLimit,
    resolveMetricsBucket,
    type MetricsRange,
  } from '$lib/components/network/wallboardMetrics';
  import { fetchInterfaceMetricsRows } from '$lib/components/network/wallboardMetricsApi';
  import {
    closeFullPanelState,
    openFullPanelState,
    setMetricsRangeState,
    switchFullTabState,
  } from '$lib/components/network/wallboardFullPanel';
  import {
    beginMetricsSelection as beginMetricsSelectionValue,
    endMetricsSelection as endMetricsSelectionValue,
    metricsHoverFromFocus,
    metricsHoverFromMouse,
    moveMetricsSelection as moveMetricsSelectionValue,
  } from '$lib/components/network/wallboardMetricsSelection';
  import {
    createWallboardDnDController,
    swapWallboardSlots,
  } from '$lib/components/network/wallboardDnD';
  import { pollWallboardLiveOnce, type WallboardLiveCounter } from '$lib/components/network/wallboardPolling';
  import {
    createWallboardRemotePersister,
    loadWallboardLocalConfig as loadWallboardLocalConfigValue,
    loadWallboardRemoteConfig as loadWallboardRemoteConfigValue,
    persistWallboardLocalConfig as persistWallboardLocalConfigValue,
  } from '$lib/components/network/wallboardConfig';
  import {
    applyLocalWallboardConfigState,
    applyRemoteWallboardConfigState,
    buildLocalWallboardConfigPayload,
  } from '$lib/components/network/wallboardConfigState';
  import {
    thresholdBpsFromInput,
    thresholdInputFromBps,
    type ThresholdUnit,
  } from '$lib/components/network/wallboardThreshold';
  import {
    clearSlotAt,
    mapInterfaceCatalogFromSnapshot,
    setSlotAt,
    updateSlotThresholdAt,
  } from '$lib/components/network/wallboardSlots';
  import {
    countActiveRouters,
    pruneSlotsByRouterIds,
    resolveAdaptivePollMs,
  } from '$lib/components/network/wallboardRuntime';
  import { installWallboardAutoHideListeners } from '$lib/components/network/wallboardUiBehavior';
  import {
    createCriticalBeepPlayer,
    createWakeLockController,
    toggleFullscreen as toggleFullscreenValue,
  } from '$lib/components/network/wallboardMedia';
  import {
    createWallboardPollingScheduler,
    installVisibilityListener,
  } from '$lib/components/network/wallboardPollingScheduler';
  import {
    buildMetricsAutoRefreshSig,
    resolveCriticalBeepEffect,
    resolveNewAlertIncidents,
  } from '$lib/components/network/wallboardEffects';
  import {
    filterPickerInterfaces,
    filterPickerRouters,
    findRouterById,
  } from '$lib/components/network/wallboardPicker';
  import { incidentHrefById, incidentHrefForTopIssue } from '$lib/components/network/wallboardLinks';
  import {
    buildAlertStats,
    buildGlobalSummary,
    buildIncidentStats,
    buildInsightsBadge,
    buildRouterAlertMap,
    buildTopIssues,
    filterVisibleAlerts,
    getActiveIncidents,
    sortActiveIncidents,
    sortAlerts,
  } from '$lib/components/network/wallboardDerivations';
  import {
    avgBps,
    calcTrend,
    colsForLayout,
    formatBps as formatBpsValue,
    formatIncidentTs,
    formatLatency as formatLatencyValue,
    formatMetricTs,
    kindClass,
    kindLabel,
    maintenanceRemaining,
    parseMetricTs,
    peakBps,
    rowsForLayout,
    severityScore,
    slotCountForLayout,
    trendBadgeText as trendBadgeTextValue,
    trendLabel as trendLabelValue,
    type IncidentKind,
    type TrendInfo,
  } from '$lib/components/network/wallboardUtils';
  import { normalizeSlotsForAutoSpare } from '$lib/components/network/wallboardPager';
  import {
    type LayoutPreset,
    type RotateMode,
  } from '$lib/constants/wallboard';
  import { toast } from '$lib/stores/toast';
  import { isSidebarCollapsed } from '$lib/stores/ui';
  import { exportCsvRows } from '$lib/utils/tabularExport';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';

  type NocRow = {
    id: string;
    name: string;
    host: string;
    port: number;
    is_online: boolean;
    latency_ms?: number | null;
    last_seen_at?: string | null;
    last_error?: string | null;
    identity?: string | null;
    ros_version?: string | null;
    maintenance_until?: string | null;
    maintenance_reason?: string | null;

    cpu_load?: number | null;
    total_memory_bytes?: number | null;
    free_memory_bytes?: number | null;
    total_hdd_bytes?: number | null;
    free_hdd_bytes?: number | null;
    uptime_seconds?: number | null;
    rx_bps?: number | null;
    tx_bps?: number | null;
  };

  type LiveRate = {
    rx_bps: number | null;
    tx_bps: number | null;
    last_seen_at: number;
  };
  type AlertRow = {
    id: string;
    router_id: string;
    severity: string;
    status: string;
    title: string;
    message: string;
    last_seen_at?: string | null;
    updated_at?: string | null;
  };
  type IncidentRow = {
    id: string;
    router_id: string;
    interface_name?: string | null;
    incident_type: string;
    severity: string;
    status: string;
    title: string;
    message: string;
    last_seen_at?: string | null;
    updated_at?: string | null;
    resolved_at?: string | null;
  };

  type HoverBar = {
    tileKey: string;
    idx: number;
  } | null;
  type TopIssue = {
    key: string;
    router_id: string;
    router_name: string;
    title: string;
    count: number;
    critical: number;
    warning: number;
    lastSeenMs: number;
  };
  type IncidentEvent = {
    id: number;
    ts: number;
    kind: IncidentKind;
    message: string;
    router_id?: string;
  };
  type RouterPollState = {
    fails: number;
    nextRetryAt: number;
    lastErrorAt: number | null;
    lastSuccessAt: number | null;
  };

  let loading = $state(true);
  let refreshing = $state(false);
  let rows = $state<NocRow[]>([]);
  let alerts = $state<AlertRow[]>([]);
  let incidents = $state<IncidentRow[]>([]);
  let alertSeverityFilter = $state<'all' | 'critical' | 'warning'>('all');

  let statusFilter = $state<'all' | 'offline' | 'online'>('all');

  let kiosk = $state(true);
  let pollMs = $state(1000);

  let ifaceLoading = $state<Record<string, boolean>>({});
  let ifaceCatalog = $state<
    Record<
      string,
      { name: string; interface_type?: string | null; running: boolean; disabled: boolean }[]
    >
  >({});

  type Slot = {
    routerId: string;
    iface: string;
    warn_below_rx_bps?: number | null;
    warn_below_tx_bps?: number | null;
  };

  let layout = $state<LayoutPreset>('3x3');
  let lastLayout = $state<LayoutPreset>('3x3');
  let rotateMode = $state<RotateMode>('manual');
  let rotateMs = $state(10000);
  // All configured tiles (can be longer than current layout capacity).
  // When layout is smaller, we show tiles in "pages" instead of truncating.
  let slotsAll = $state<(Slot | null)[]>([]);
  let page = $state(0);
  let pageCount = $state(1);
  // Visible page slice (always padded to layout capacity).
  let slots = $state<(Slot | null)[]>([]);
  let pickerIndex = $state<number | null>(null);
  let pickerRouterSearch = $state('');
  let pickerRouterId = $state<string | null>(null);
  let pickerIfaceSearch = $state('');

  let fullIndex = $state<number | null>(null);
  let fullTab = $state<'live' | 'metrics'>('live');
  let metricsRange = $state<MetricsRange>('24h');
  let metricsFromLocal = $state('');
  let metricsToLocal = $state('');
  let metricsPointIdx = $state<number | null>(null);
  let metricsTooltipX = $state(0);
  let metricsTooltipY = $state(0);
  let metricsZoomFrom = $state<number | null>(null);
  let metricsZoomTo = $state<number | null>(null);
  let metricsSelecting = $state(false);
  let metricsSelStart = $state(0);
  let metricsSelCurrent = $state(0);
  let metricsSelWidth = $state(0);
  let fullMetricsLoading = $state(false);
  let fullMetricsError = $state<string | null>(null);
  let fullMetricsRows = $state<any[]>([]);
  let fullMetricsKey = $state('');
  let fullMetricsLimit = $state(0);
  let fullMetricsInFlightSig = $state('');
  let metricsAutoRefreshSig = $state('');
  let thresholdIndex = $state<number | null>(null);
  let thWarnRxKbps = $state<string>('');
  let thWarnTxKbps = $state<string>('');
  let thWarnRxUnit = $state<ThresholdUnit>('Kbps');
  let thWarnTxUnit = $state<ThresholdUnit>('Kbps');

  // Rate computation
  let liveRates = $state<Record<string, Record<string, LiveRate>>>({});
  let series = $state<Record<string, Record<string, { rx: number[]; tx: number[] }>>>({});
  const lastBytes = new Map<string, { rx: number; tx: number; at: number }>();

  let livePollInFlight = $state(false);
  let alertSyncInFlight = $state(false);
  let remoteLoaded = $state(false);
  let paused = $state(false);
  let documentVisible = $state(true);
  let focusMode = $state(true);
  let alertsOpen = $state(false);
  let insightsOpen = $state(false);
  let renderNow = $state(Date.now());
  let uninstallAutoHide: (() => void) | null = null;

  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);
  let dragging = $state(false);
  let tileMenuIndex = $state<number | null>(null);

  let hoverBar = $state<HoverBar>(null);
  let topIssueMuteMinutes = $state<Record<string, number>>({});
  let incidentEvents = $state<IncidentEvent[]>([]);
  let incidentSeq = $state(0);
  // Keep previous alert ids/severity in a non-reactive snapshot to avoid effect loops.
  let alertSnapshot: Record<string, string> = {};
  let routerPollState = $state<Record<string, RouterPollState>>({});
  let lastPollDurationMs = $state(0);
  let hasPollFailure = $state(false);

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $pageStore.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $pageStore.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);
  const canUseTenantSettings = $derived($can('read', 'settings') || $can('update', 'settings'));
  const remotePersister = createWallboardRemotePersister({
    canUseTenantSettings: () => canUseTenantSettings,
    remoteLoaded: () => remoteLoaded,
    getLayout: () => layout,
    getSlotsAll: () => slotsAll,
    upsert: (key, value, description) => api.settings.upsert(key, value, description),
  });
  const dndController = createWallboardDnDController({
    getDragState: () => ({ dragFrom, dragOver, dragging }),
    setDragState: (next) => {
      dragFrom = next.dragFrom;
      dragOver = next.dragOver;
      dragging = next.dragging;
    },
    onSwapSlots: (from, to) => {
      slotsAll = swapWallboardSlots(slotsAll, from, to);
    },
  });

  // Auto-hide toolbar for NOC display friendly behavior.
  let lastActivityAt = $state(Date.now());
  let hideHandle: any = null;
  let isFullscreen = $state(false);
  let controlsHidden = $state(false);
  let criticalSoundEnabled = $state(true);
  let lastCriticalSignature = $state('');
  let lastCriticalBeepAt = $state(0);
  const criticalBeepPlayer = createCriticalBeepPlayer();
  const wakeLockController = createWakeLockController();
  const pollingScheduler = createWallboardPollingScheduler({
    pollLiveOnce: () => void pollLiveOnce(),
    syncAlertsIncidents: () => void syncAlertsIncidents(true),
    getPollMs: () =>
      resolveAdaptivePollMs({
        basePollMs: pollMs,
        activeRouters: countActiveRouters(slotsAll),
        lastPollDurationMs,
        hasPollFailure,
      }),
    isPaused: () => paused,
    isVisible: () => documentVisible,
    alertsIntervalMs: 10_000,
  });

  const TOOLBAR_HIDE_MS = 10_000;

  function formatBps(bps?: number | null) {
    return formatBpsValue(bps, $t('common.na') || '—');
  }

  function formatLatency(ms?: number | null) {
    return formatLatencyValue(ms, $t('common.na') || '—');
  }

  function routerTitle(r: NocRow) {
    const name = r.identity || r.name;
    const ros = r.ros_version ? ` • ROS ${r.ros_version}` : '';
    return `${name}${ros}`;
  }

  const sortedAlerts = $derived.by(() => sortAlerts(alerts));

  const routerAlertMap = $derived.by(() => buildRouterAlertMap(sortedAlerts));

  const alertStats = $derived.by(() => buildAlertStats(sortedAlerts));

  const activeIncidents = $derived.by(() => getActiveIncidents(incidents));

  const sortedActiveIncidents = $derived.by(() => sortActiveIncidents(activeIncidents));

  const openIncidentItems = $derived.by(() => sortedActiveIncidents.slice(0, 8));

  const incidentStats = $derived.by(() => buildIncidentStats(activeIncidents));

  const visibleAlerts = $derived.by(() => filterVisibleAlerts(sortedAlerts, alertSeverityFilter));

  const globalSummary = $derived.by(() => buildGlobalSummary(rows, incidentStats));

  const topIssues = $derived.by(() => buildTopIssues({ activeIncidents, rows }));

  const insightsBadge = $derived.by(() => buildInsightsBadge(incidentStats));

  function trendBadgeText(ti: TrendInfo) {
    return trendBadgeTextValue(ti, $t('admin.network.wallboard.trend.stable') || 'Stable');
  }

  function trendLabel(ti: TrendInfo) {
    return trendLabelValue(ti, {
      up: $t('admin.network.wallboard.trend.up') || 'Rising',
      down: $t('admin.network.wallboard.trend.down') || 'Falling',
      stable: $t('admin.network.wallboard.trend.stable') || 'Stable',
    });
  }

  async function loadAlerts(silent = true) {
    alerts = await loadWallboardAlerts(silent);
  }

  async function loadIncidents(silent = true) {
    incidents = await loadWallboardIncidents(silent);
  }

  async function syncAlertsIncidents(silent = true) {
    if (alertSyncInFlight) return;
    alertSyncInFlight = true;
    try {
      await Promise.all([loadAlerts(silent), loadIncidents(silent)]);
    } finally {
      alertSyncInFlight = false;
    }
  }

  async function ackIncident(id: string) {
    if (!$can('manage', 'network_routers')) return;
    try {
      await ackWallboardIncident(id, $t('admin.network.alerts.toasts.acked') || 'Alert acknowledged');
      pushIncident('ack', 'Incident acknowledged');
      await Promise.all([loadAlerts(true), loadIncidents(true)]);
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function resolveIncident(id: string) {
    if (!$can('manage', 'network_routers')) return;
    try {
      await resolveWallboardIncident(id, $t('admin.network.alerts.toasts.resolved') || 'Alert resolved');
      pushIncident('recovered', 'Incident resolved');
      await Promise.all([loadAlerts(true), loadIncidents(true)]);
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function ackRouterAlerts(routerId: string) {
    if (!$can('manage', 'network_routers')) return;
    const ids = routerAlertMap[routerId]?.ids || [];
    if (!ids.length) return;
    try {
      await ackWallboardAlerts(ids.slice(0, 50), $t('admin.network.alerts.toasts.acked') || 'Alert acknowledged');
      pushIncident('ack', `Ack ${Math.min(ids.length, 50)} alert(s)`, routerId);
      await Promise.all([loadAlerts(false), loadIncidents(false)]);
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function ackVisibleAlerts() {
    if (!$can('manage', 'network_routers')) return;
    const ids = visibleAlerts
      .filter((a) => {
        const st = String(a.status || '').toLowerCase();
        return st !== 'ack' && st !== 'resolved';
      })
      .map((a) => a.id);
    if (!ids.length) return;

    try {
      await ackWallboardAlerts(
        ids.slice(0, 80),
        `${$t('admin.network.alerts.toasts.acked') || 'Alert acknowledged'} (${Math.min(ids.length, 80)})`,
      );
      pushIncident('ack', `Ack visible ${Math.min(ids.length, 80)} alert(s)`);
      await Promise.all([loadAlerts(false), loadIncidents(false)]);
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function muteRouterAlerts(routerId: string, minutes: number) {
    if (!$can('manage', 'network_routers')) return;
    try {
      await muteWallboardRouter(routerId, minutes, $t('admin.network.alerts.toasts.snoozed') || 'Router snoozed');
      pushIncident('mute', `Mute ${minutes}m`, routerId);
      await refresh();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function unmuteRouter(routerId: string) {
    if (!$can('manage', 'network_routers')) return;
    try {
      await unmuteWallboardRouter(routerId, $t('admin.network.wallboard.unmuted') || 'Maintenance cleared');
      pushIncident('unmute', 'Maintenance cleared', routerId);
      await refresh();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  function selectedMuteMinutes(routerId: string) {
    const v = topIssueMuteMinutes[routerId];
    return v === 60 || v === 240 ? v : 30;
  }

  function ensureSlots() {
    const want = slotCountForLayout(layout);
    // Never shrink `slotsAll` on layout change (we paginate instead).
    if (slotsAll.length < want) {
      slotsAll = [...slotsAll, ...Array.from({ length: want - slotsAll.length }, () => null)];
    }
  }

  function ensureSlotIndex(idx: number) {
    if (idx < slotsAll.length) return;
    slotsAll = [...slotsAll, ...Array.from({ length: idx + 1 - slotsAll.length }, () => null)];
  }

  function globalIndex(localIdx: number) {
    return page * slotCountForLayout(layout) + localIdx;
  }

  function routerById(id: string) {
    return findRouterById(rows, id);
  }

  function routerLabel(routerId: string) {
    const rr = routerById(routerId);
    return rr?.identity || rr?.name || routerId;
  }

  function pushIncident(kind: IncidentKind, message: string, routerId?: string) {
    const ev: IncidentEvent = {
      id: ++incidentSeq,
      ts: Date.now(),
      kind,
      message,
      router_id: routerId,
    };
    incidentEvents = [ev, ...incidentEvents].slice(0, 20);
  }

  function openPicker(idx: number) {
    pickerIndex = idx;
    pickerRouterSearch = '';
    pickerIfaceSearch = '';

    const cur = slotsAll[idx];
    pickerRouterId = cur?.routerId ?? null;
    if (pickerRouterId) void loadInterfaces(pickerRouterId);
  }

  function closePicker() {
    pickerIndex = null;
    pickerRouterId = null;
  }

  function pickerRouterList() {
    return filterPickerRouters(rows, statusFilter, pickerRouterSearch);
  }

  function pickerInterfaces() {
    return filterPickerInterfaces(ifaceCatalog, pickerRouterId, pickerIfaceSearch);
  }

  function openFull(idx: number) {
    const next = openFullPanelState(idx);
    fullIndex = next.fullIndex;
    fullTab = next.fullTab;
    metricsRange = next.metricsRange;
    metricsFromLocal = next.metricsFromLocal;
    metricsToLocal = next.metricsToLocal;
    metricsPointIdx = next.metricsPointIdx;
    metricsZoomFrom = next.metricsZoomFrom;
    metricsZoomTo = next.metricsZoomTo;
    metricsSelecting = next.metricsSelecting;
  }

  function closeFull() {
    const next = closeFullPanelState();
    fullIndex = next.fullIndex;
    fullMetricsLoading = next.fullMetricsLoading;
    fullMetricsError = next.fullMetricsError;
    fullMetricsRows = next.fullMetricsRows;
    fullMetricsKey = next.fullMetricsKey;
    fullMetricsLimit = next.fullMetricsLimit;
    fullMetricsInFlightSig = '';
    metricsFromLocal = next.metricsFromLocal;
    metricsToLocal = next.metricsToLocal;
    metricsPointIdx = next.metricsPointIdx;
    metricsZoomFrom = next.metricsZoomFrom;
    metricsZoomTo = next.metricsZoomTo;
    metricsSelecting = next.metricsSelecting;
    metricsAutoRefreshSig = '';
  }

  async function refreshMetricsForCurrentRange() {
    if (fullIndex == null) return;
    const limit = requiredMetricLimit(metricsRange, metricsFromLocal, metricsToLocal);
    await loadFullMetrics(fullIndex, limit);
  }

  function setMetricsRange(next: MetricsRange) {
    const state = setMetricsRangeState(next);
    metricsRange = state.metricsRange;
    metricsPointIdx = state.metricsPointIdx;
    metricsZoomFrom = state.metricsZoomFrom;
    metricsZoomTo = state.metricsZoomTo;
    metricsFromLocal = state.metricsFromLocal;
    metricsToLocal = state.metricsToLocal;
  }

  function clearMetricsZoom() {
    metricsZoomFrom = null;
    metricsZoomTo = null;
    metricsPointIdx = null;
  }

  function beginMetricsSelection(e: PointerEvent) {
    const selected = beginMetricsSelectionValue(e);
    if (!selected) return;
    metricsSelecting = true;
    metricsSelWidth = selected.selWidth;
    metricsSelStart = selected.selStart;
    metricsSelCurrent = selected.selCurrent;
    metricsPointIdx = null;
  }

  function moveMetricsSelection(e: PointerEvent) {
    if (!metricsSelecting) return;
    const moved = moveMetricsSelectionValue(e);
    if (!moved) return;
    metricsSelWidth = moved.selWidth;
    metricsSelCurrent = moved.selCurrent;
  }

  function endMetricsSelection(
    e: PointerEvent,
    rows: { ts: string; rx_bps: number; tx_bps: number }[],
  ) {
    if (!metricsSelecting) return;
    metricsSelecting = false;
    const zoom = endMetricsSelectionValue(
      e,
      rows,
      metricsSelStart,
      metricsSelCurrent,
      metricsSelWidth,
      parseMetricTs,
    );
    if (!zoom) return;
    metricsZoomFrom = zoom.zoomFrom;
    metricsZoomTo = zoom.zoomTo;
    metricsPointIdx = null;
  }

  function bucketLabel(bucket: 'raw' | 'hour' | 'day') {
    if (bucket === 'raw')
      return $t('admin.network.wallboard.metrics_agg.detail') || 'Detail';
    if (bucket === 'hour')
      return $t('admin.network.wallboard.metrics_agg.hourly') || 'Hourly Average';
    return $t('admin.network.wallboard.metrics_agg.daily') || 'Daily Average';
  }

  function bucketHint(bucket: 'raw' | 'hour' | 'day') {
    if (bucket === 'raw')
      return $t('admin.network.wallboard.metrics_agg_hint.detail') || 'without summarization';
    if (bucket === 'hour')
      return $t('admin.network.wallboard.metrics_agg_hint.hourly') || 'summarized per hour';
    return $t('admin.network.wallboard.metrics_agg_hint.daily') || 'summarized per day';
  }

  function exportMetricsCsv(
    rows: { ts: string; rx_bps: number; tx_bps: number }[],
    iface: string,
    routerName: string,
    bucket: 'raw' | 'hour' | 'day',
  ) {
    if (!rows.length) {
      toast.error($t('admin.network.wallboard.metrics.export_empty') || 'No metrics data to export.');
      return;
    }
    const csvRows = rows.map((r) => ({
      timestamp: r.ts,
      router: routerName,
      interface: iface,
      bucket,
      rx_bps: r.rx_bps ?? 0,
      tx_bps: r.tx_bps ?? 0,
    }));
    const filePrefix = `metrics-${iface || 'interface'}-${bucket}`;
    exportCsvRows(csvRows, filePrefix);
  }

  function openFullTab(tab: 'live' | 'metrics') {
    const next = switchFullTabState(tab);
    fullTab = next.fullTab;
    metricsPointIdx = next.metricsPointIdx;
    metricsSelecting = next.metricsSelecting;
  }

  function setMetricsHoverFromMouse(i: number, e: MouseEvent) {
    const next = metricsHoverFromMouse(i, e);
    metricsPointIdx = next.pointIdx;
    metricsTooltipX = next.tooltipX;
    metricsTooltipY = next.tooltipY;
  }

  function setMetricsHoverFromFocus(i: number, e: FocusEvent) {
    const next = metricsHoverFromFocus(i, e);
    metricsPointIdx = next.pointIdx;
    if (next.tooltipX == null || next.tooltipY == null) return;
    metricsTooltipX = next.tooltipX;
    metricsTooltipY = next.tooltipY;
  }

  async function loadFullMetrics(idx: number, minLimit: number = 240) {
    const s = slotsAll[idx];
    if (!s) return;

    const key = `${s.routerId}:${s.iface}`;
    const requestSig = `${key}:${minLimit}`;
    if (
      fullMetricsKey === key &&
      fullMetricsRows.length > 0 &&
      fullMetricsLimit >= minLimit
    )
      return;
    if (fullMetricsInFlightSig === requestSig) return;

    fullMetricsInFlightSig = requestSig;
    fullMetricsKey = key;
    fullMetricsLoading = true;
    fullMetricsError = null;

    try {
      const rows = await fetchInterfaceMetricsRows({
        slot: { routerId: s.routerId, iface: s.iface },
        minLimit,
        fetchMetrics: (routerId, params) => api.mikrotik.routers.interfaceMetrics(routerId, params),
      });

      if (fullMetricsKey !== key) return;
      fullMetricsRows = rows;
      fullMetricsLimit = minLimit;
    } catch (e: any) {
      if (fullMetricsKey !== key) return;
      fullMetricsRows = [];
      fullMetricsError = e?.message || String(e);
    } finally {
      if (fullMetricsInFlightSig === requestSig) fullMetricsInFlightSig = '';
      if (fullMetricsKey === key) fullMetricsLoading = false;
    }
  }

  function openThreshold(idx: number) {
    const s = slotsAll[idx];
    if (!s) return;
    thresholdIndex = idx;
    const rx = thresholdInputFromBps(s.warn_below_rx_bps ?? null);
    const tx = thresholdInputFromBps(s.warn_below_tx_bps ?? null);
    thWarnRxUnit = rx.unit;
    thWarnRxKbps = rx.value;
    thWarnTxUnit = tx.unit;
    thWarnTxKbps = tx.value;
  }

  function closeThreshold() {
    thresholdIndex = null;
  }

  function updateSlotThreshold(idx: number, rxBps: number | null, txBps: number | null) {
    slotsAll = updateSlotThresholdAt(slotsAll, idx, rxBps, txBps);
    persistConfig();
  }

  function saveThreshold() {
    if (thresholdIndex == null) return;
    const rxBps = thresholdBpsFromInput(thWarnRxKbps, thWarnRxUnit);
    const txBps = thresholdBpsFromInput(thWarnTxKbps, thWarnTxUnit);
    updateSlotThreshold(thresholdIndex, rxBps, txBps);
    closeThreshold();
  }

  function setSlot(
    idx: number,
    routerId: string,
    iface: string,
    warnBelowRxBps?: number | null,
    warnBelowTxBps?: number | null,
  ) {
    slotsAll = setSlotAt(slotsAll, idx, routerId, iface, warnBelowRxBps, warnBelowTxBps);
    pickerIndex = null;
    pickerRouterId = null;
    persistConfig();
  }

  function clearSlot(idx: number) {
    slotsAll = clearSlotAt(slotsAll, idx);
    persistConfig();
  }

  async function loadInterfaces(routerId: string) {
    if (ifaceCatalog[routerId]?.length) return;
    ifaceLoading[routerId] = true;
    try {
      const snap = await api.mikrotik.routers.snapshot(routerId);
      ifaceCatalog[routerId] = mapInterfaceCatalogFromSnapshot(snap);
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      ifaceLoading[routerId] = false;
    }
  }

  function persistConfig() {
    persistWallboardLocalConfigValue(buildLocalWallboardConfigPayload({
      layout,
      slotsAll,
      rotateMode,
      rotateMs,
      focusMode,
      statusFilter,
      pollMs,
      criticalSoundEnabled,
    }));
  }

  function loadConfig() {
    applyLocalWallboardConfigState(loadWallboardLocalConfigValue(), {
      setLayout: (v) => (layout = v),
      setRotateMode: (v) => (rotateMode = v),
      setRotateMs: (v) => (rotateMs = v),
      setStatusFilter: (v) => (statusFilter = v),
      setPollMs: (v) => (pollMs = v),
      setCriticalSoundEnabled: (v) => (criticalSoundEnabled = v),
      setSlotsAll: (v) => (slotsAll = v),
    });
  }

  async function loadRemoteConfig() {
    const conf = await loadWallboardRemoteConfigValue({
      canUseTenantSettings,
      getValue: (key) => api.settings.getValue(key),
    });
    applyRemoteWallboardConfigState(conf, {
      setLayout: (v) => (layout = v),
      setSlotsAll: (v) => (slotsAll = v),
      setRemoteLoaded: (v) => (remoteLoaded = v),
    });
  }

  function schedulePersistRemote() {
    remotePersister.schedulePersistRemote();
  }

  async function persistRemoteNow() {
    await remotePersister.persistRemoteNow();
  }

  async function refresh() {
    refreshing = true;
    try {
      const list = (await api.mikrotik.routers.noc()) as any as NocRow[];
      rows = list;
      slotsAll = pruneSlotsByRouterIds(slotsAll, rows.map((r) => r.id));
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      refreshing = false;
    }
    await syncAlertsIncidents(true);
  }

  async function pollLiveOnce() {
    if (livePollInFlight) {
      return;
    }
    livePollInFlight = true;
    const startedAt = Date.now();
    try {
      const next = await pollWallboardLiveOnce({
        paused,
        documentHidden: typeof document !== 'undefined' && document.hidden,
        slotsAll,
        routerPollState,
        liveRates,
        series,
        lastBytes,
        loadInterfaceLive: (routerId, ifaceNames) =>
          api.mikrotik.routers.interfaceLive(routerId, ifaceNames) as Promise<WallboardLiveCounter[]>,
        onRecovered: (routerId) => pushIncident('recovered', 'Polling recovered', routerId),
        onPollError: (routerId, fails) => pushIncident('poll_error', `Polling failed (${fails}x)`, routerId),
      });
      if (!next) return;
      liveRates = next.liveRates;
      series = next.series;
      routerPollState = next.routerPollState;
      hasPollFailure = Object.values(next.routerPollState).some((state) => state.fails > 0);
      renderNow = next.renderNow;
    } finally {
      lastPollDurationMs = Date.now() - startedAt;
      livePollInFlight = false;
    }
  }

  function setPaused(on: boolean) {
    paused = on;
    pollingScheduler.refresh();
  }

  function getHoverIndexFromPoint(x: number, y: number) {
    return dndController.getHoverIndexFromPoint(x, y);
  }

  function setHoverBarFromPointer(tileKey: string, e: PointerEvent) {
    const idx = getHoverIndexFromPoint(e.clientX, e.clientY);
    if (idx == null) return;
    hoverBar = { tileKey, idx };
  }

  function clearHoverBar(tileKey: string) {
    if (hoverBar?.tileKey === tileKey) hoverBar = null;
  }

  function startDragFromTile(e: PointerEvent, idx: number) {
    dndController.startDragFromTile(e, idx);
  }

  function showControls() {
    controlsHidden = false;
    lastActivityAt = Date.now();
    if (hideHandle) clearTimeout(hideHandle);
    hideHandle = setTimeout(() => {
      controlsHidden = true;
    }, TOOLBAR_HIDE_MS);
  }

  function toggleAlertsPanel() {
    alertsOpen = !alertsOpen;
  }

  async function toggleFullscreen() {
    await toggleFullscreenValue();
  }

  async function playCriticalBeep() {
    await criticalBeepPlayer.play(criticalSoundEnabled);
  }

  async function applyWakeLock(on: boolean) {
    await wakeLockController.apply(on);
  }

  function applyKiosk(on: boolean) {
    kiosk = on;
    if (typeof document === 'undefined') return;
    document.body.classList.toggle('kiosk-wallboard', kiosk);
    // Make sure we get maximum screen real estate.
    if (kiosk) $isSidebarCollapsed = true;
  }

  function exitWallboard() {
    applyKiosk(false);
    $isSidebarCollapsed = false;
    // Use absolute tenant-aware path to avoid relative-navigation mismatches in grouped routes.
    goto(`${tenantPrefix}/admin/network/noc`);
  }

  onMount(() => {
    if (!$can('read', 'network_routers') && !$can('manage', 'network_routers')) {
      goto('/unauthorized');
      return;
    }

    loadConfig();
    ensureSlots();
    // Wallboard is meant for NOC/full window display. Keep kiosk enabled while this route is active.
    kiosk = true;
    applyKiosk(true);
    void loadRemoteConfig();

    uninstallAutoHide =
      installWallboardAutoHideListeners({
        showControls,
        onPointerDown: (target) => {
          if (!target?.closest?.('.tile-actions')) tileMenuIndex = null;
        },
        onEscape: () => {
          if (alertsOpen) {
            alertsOpen = false;
            return true;
          }
          return false;
        },
        onToggleFocusMode: () => {
          focusMode = !focusMode;
        },
      }) ?? null;
    showControls();
    isFullscreen = typeof document !== 'undefined' && !!document.fullscreenElement;

    const onFullscreenChange = () => {
      isFullscreen = !!document.fullscreenElement;
    };
    const uninstallVisibility =
      installVisibilityListener((visible) => {
        documentVisible = visible;
        if (documentVisible && !paused) {
          void pollLiveOnce();
          void syncAlertsIncidents(true);
        }
        pollingScheduler.refresh();
      }) ?? null;
    if (typeof document !== 'undefined') {
      document.addEventListener('fullscreenchange', onFullscreenChange);
      documentVisible = !document.hidden;
    }

    void (async () => {
      loading = true;
      try {
        await refresh();
        ensureSlots();
      } finally {
        loading = false;
      }
    })();

    pollingScheduler.refresh();

    return () => {
      if (typeof document !== 'undefined') {
        document.removeEventListener('fullscreenchange', onFullscreenChange);
      }
      uninstallVisibility?.();
    };
  });

  onDestroy(() => {
    pollingScheduler.stopAll();
    remotePersister.clearScheduledPersist();
    dndController.dispose();
    // Best-effort flush so layout/slots don't get lost on fast logout/navigation.
    void persistRemoteNow();
    void applyWakeLock(false);
    if (typeof document !== 'undefined') document.body.classList.remove('kiosk-wallboard');
    if (hideHandle) clearTimeout(hideHandle);
    void criticalBeepPlayer.close();
    try {
      uninstallAutoHide?.();
    } catch {}
  });

  $effect(() => {
    if (fullIndex == null || fullTab !== 'metrics') return;
    const sig = buildMetricsAutoRefreshSig({
      fullIndex,
      metricsRange,
      metricsFromLocal,
      metricsToLocal,
    });
    if (sig === metricsAutoRefreshSig) return;
    metricsAutoRefreshSig = sig;
    void refreshMetricsForCurrentRange();
  });

  $effect(() => {
    // restart polling when interval changes or pause changes
    const nextPollMs = resolveAdaptivePollMs({
      basePollMs: pollMs,
      activeRouters: countActiveRouters(slotsAll),
      lastPollDurationMs,
      hasPollFailure,
    });
    pollingScheduler.refresh(nextPollMs);
  });

  $effect(() => {
    if (rotateMode !== 'auto' || pageCount <= 1) return;
    const h = setInterval(() => {
      page = page >= pageCount - 1 ? 0 : page + 1;
    }, rotateMs);
    return () => clearInterval(h);
  });

  $effect(() => {
    if (layout !== lastLayout) {
      lastLayout = layout;
      page = 0;
    }

    const size = slotCountForLayout(layout);
    ensureSlots();
    const normalized = normalizeSlotsForAutoSpare(slotsAll, size);
    if (
      normalized.length !== slotsAll.length ||
      normalized.some((it, idx) => it !== slotsAll[idx])
    ) {
      slotsAll = normalized;
      return;
    }
    const pages = Math.max(1, Math.ceil(slotsAll.length / size));
    pageCount = pages;
    if (page >= pages) page = pages - 1;

    const start = page * size;
    let view = slotsAll.slice(start, start + size);
    if (view.length < size) view = [...view, ...Array.from({ length: size - view.length }, () => null)];
    slots = view;

    persistConfig();
    schedulePersistRemote();
  });

  $effect(() => {
    void applyWakeLock(true);
  });

  $effect(() => {
    focusMode;
    persistConfig();
  });

  $effect(() => {
    const next = resolveCriticalBeepEffect({
      alerts: sortedAlerts,
      lastSignature: lastCriticalSignature,
      lastBeepAt: lastCriticalBeepAt,
      soundEnabled: criticalSoundEnabled,
      paused,
      now: Date.now(),
      minBeepGapMs: 8000,
    });
    lastCriticalSignature = next.signature;
    lastCriticalBeepAt = next.beepAt;
    if (next.shouldBeep) void playCriticalBeep();
  });

  $effect(() => {
    const next = resolveNewAlertIncidents({
      alerts: sortedAlerts,
      previousSnapshot: alertSnapshot,
    });
    for (const item of next.newIncidents) {
      pushIncident(item.severity, item.message, item.routerId);
    }
    alertSnapshot = next.nextSnapshot;
  });
</script>

<div class="wallboard-viewport">
  <div class="wallboard" class:focus={focusMode}>
  <div class="wb-top" class:hidden={controlsHidden}>
    <div class="controls wall-actions">
      <div class="toolbar-left">
        <WallboardTopPager bind:page {pageCount} />
        <button
          class="settings-btn"
          class:has-critical={insightsBadge.level === 'critical'}
          class:has-warning={insightsBadge.level === 'warning'}
          onclick={() => {
            insightsOpen = !insightsOpen;
          }}
          title={$t('admin.network.wallboard.controls.open') || 'Open settings'}
        >
          <Icon name="settings" size={16} />
          {$t('admin.network.wallboard.settings') || 'Settings'}
          {#if insightsBadge.total > 0}
            <span class="insights-badge">
              {insightsBadge.total > 99 ? '99+' : insightsBadge.total}
            </span>
          {/if}
        </button>
      </div>

    </div>
  </div>

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
        onRefresh={() => void refresh()}
        onTogglePaused={() => setPaused(!paused)}
        onToggleFullscreen={() => void toggleFullscreen()}
        onToggleCriticalSound={() => {
          criticalSoundEnabled = !criticalSoundEnabled;
        }}
        onExit={exitWallboard}
      />
      <WallboardInsightsSummary
        {globalSummary}
        {topIssues}
        {openIncidentItems}
        {incidentEvents}
        canManage={$can('manage', 'network_routers')}
        selectedMuteMinutes={selectedMuteMinutes}
        getMaintenanceRemaining={(routerId) => maintenanceRemaining(routerById(routerId)?.maintenance_until)}
        onSetTopIssueMuteMinutes={(routerId, mins) => {
          topIssueMuteMinutes = {
            ...topIssueMuteMinutes,
            [routerId]: mins === 60 || mins === 240 ? mins : 30,
          };
        }}
        onGotoTopIssue={(routerId, title) =>
          goto(incidentHrefForTopIssue(incidents, tenantPrefix, routerId, title))}
        onMuteTopIssue={(routerId, mins) => void muteRouterAlerts(routerId, mins)}
        onUnmuteTopIssue={(routerId) => void unmuteRouter(routerId)}
        onOpenIncident={(incidentId) => goto(incidentHrefById(tenantPrefix, incidentId))}
        onAckIncident={(incidentId) => void ackIncident(incidentId)}
        onResolveIncident={(incidentId) => void resolveIncident(incidentId)}
        {routerLabel}
        {formatMetricTs}
        {formatIncidentTs}
        kindClass={(kind) => kindClass(kind as IncidentKind)}
        kindLabel={(kind) => kindLabel(kind as IncidentKind)}
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
        canManage={$can('manage', 'network_routers')}
        onAckVisible={() => void ackVisibleAlerts()}
        onOpenAlerts={() => goto(`${tenantPrefix}/admin/network/alerts`)}
        routerLabel={(routerId) => routerById(routerId)?.identity || routerById(routerId)?.name || routerId}
      />
    </div>
  {/if}

  {#if paused}
    <div class="pause-indicator">
      <Icon name="pause" size={16} />
      <span>{$t('admin.network.wallboard.pause') || 'Pause'}</span>
    </div>
  {/if}

  {#if loading}
    <div class="empty">
      <Icon name="loader" size={18} />
      {$t('common.loading') || 'Loading...'}
    </div>
  {:else}
    <div class="grid" class:compact={layout === '4x3'} style={`--cols:${colsForLayout(layout)}; --rows:${rowsForLayout(layout)};`}>
      {#each slots as slot, idx (idx)}
        {@const gidx = globalIndex(idx)}
        {@const r = slot ? routerById(slot.routerId) : null}
        {#if !slot}
          <button
            class="tile add"
            class:drag-over={dragOver === gidx}
            data-wall-slot={gidx}
            type="button"
            onclick={() => openPicker(gidx)}
          >
            <div class="add-inner">
              <div class="plus">+</div>
              <div class="add-title">
                {$t('admin.network.wallboard.add_tile') || 'Add interface tile'}
              </div>
              <div class="add-sub">
                {$t('admin.network.wallboard.add_tile_sub') || 'Choose a router + interface'}
              </div>
            </div>
          </button>
        {:else}
          <WallboardInterfaceTile
            {gidx}
            {slot}
            router={r}
            rx={series[slot.routerId]?.[slot.iface]?.rx ?? []}
            tx={series[slot.routerId]?.[slot.iface]?.tx ?? []}
            rxNow={liveRates[slot.routerId]?.[slot.iface]?.rx_bps ?? null}
            txNow={liveRates[slot.routerId]?.[slot.iface]?.tx_bps ?? null}
            lastSeenAt={liveRates[slot.routerId]?.[slot.iface]?.last_seen_at ?? null}
            pollFails={routerPollState[slot.routerId]?.fails ?? 0}
            pollRetrySec={Math.max(
              0,
              Math.ceil(((routerPollState[slot.routerId]?.nextRetryAt ?? 0) - Date.now()) / 1000),
            )}
            routerAlertTotal={routerAlertMap[slot.routerId]?.total ?? 0}
            canManage={$can('manage', 'network_routers')}
            {dragOver}
            {tileMenuIndex}
            {hoverBar}
            {paused}
            {pollMs}
            {renderNow}
            {formatBps}
            {trendBadgeText}
            {trendLabel}
            onStartDragFromTile={startDragFromTile}
            onOpenFull={openFull}
            onOpenThreshold={openThreshold}
            onClearSlot={clearSlot}
            onAckRouterAlerts={(routerId) => void ackRouterAlerts(routerId)}
            onOpenAlerts={() => goto(`${tenantPrefix}/admin/network/alerts`)}
            onToggleTileMenu={(idx) => {
              tileMenuIndex = idx < 0 ? null : tileMenuIndex === idx ? null : idx;
            }}
            onSetHover={setHoverBarFromPointer}
            onClearHover={clearHoverBar}
          />
        {/if}
      {/each}
    </div>
  {/if}
  </div>

  {#if sortedAlerts.length > 0}
    <button
      class="floating-alert-btn"
      class:open={alertsOpen}
      type="button"
      onclick={toggleAlertsPanel}
      aria-expanded={alertsOpen}
      aria-controls="wallboard-alert-panel"
      aria-label={$t('admin.network.wallboard.alerts_open') || 'Open alerts'}
      title={$t('admin.network.wallboard.alerts_open') || 'Open alerts'}
    >
      <Icon name="alert-triangle" size={17} />
      <span class="floating-alert-count">{sortedAlerts.length > 99 ? '99+' : sortedAlerts.length}</span>
    </button>
  {/if}

</div>

{#if pickerIndex !== null}
  {@const curSlot = slotsAll[pickerIndex]}
  {@const routerList = pickerRouterList()}
  {@const ifaces = pickerInterfaces()}
  <WallboardSlotPicker
    isEditing={!!curSlot}
    currentIface={curSlot?.iface || '—'}
    selectedRouterLabel={pickerRouterId
      ? (routerById(pickerRouterId)?.identity || routerById(pickerRouterId)?.name || pickerRouterId)
      : '—'}
    bind:routerSearch={pickerRouterSearch}
    bind:ifaceSearch={pickerIfaceSearch}
    bind:selectedRouterId={pickerRouterId}
    {routerList}
    interfaces={ifaces}
    interfacesTotal={pickerRouterId ? (ifaceCatalog[pickerRouterId]?.length || 0) : 0}
    interfacesLoading={pickerRouterId ? !!ifaceLoading[pickerRouterId] : false}
    {routerTitle}
    onClose={closePicker}
    onSelectRouter={(routerId) => {
      pickerRouterId = routerId;
      void loadInterfaces(routerId);
      pickerIfaceSearch = '';
    }}
    onSelectInterface={(iface) => {
      const cur = slotsAll[pickerIndex as number];
      const rx = cur?.warn_below_rx_bps ?? null;
      const tx = cur?.warn_below_tx_bps ?? null;
      setSlot(pickerIndex as number, pickerRouterId as string, iface, rx, tx);
    }}
  />
{/if}

{#if fullIndex !== null}
  {@const s = slotsAll[fullIndex]}
  {@const r = s ? routerById(s.routerId) : null}
  {@const iface = s?.iface || ''}
  {@const rx = s ? series[s.routerId]?.[iface]?.rx ?? [] : []}
  {@const tx = s ? series[s.routerId]?.[iface]?.tx ?? [] : []}
  {@const max = Math.max(1, ...rx, ...tx)}
  {@const rxPeak = peakBps(rx)}
  {@const txPeak = peakBps(tx)}
  {@const rxAvg = avgBps(rx)}
  {@const txAvg = avgBps(tx)}
  {@const filteredMetricsRows = filterMetricsRowsByRange(fullMetricsRows, metricsFromLocal, metricsToLocal, parseMetricTs)}
  {@const histRawRows = buildHistPoints(filteredMetricsRows, parseMetricTs)}
  {@const metricsBucket = resolveMetricsBucket(metricsRange, metricsFromLocal, metricsToLocal)}
  {@const histRows = aggregateHistPoints(histRawRows, metricsBucket, parseMetricTs)}
  {@const zoomedHistRows = applyMetricsZoom(histRows, metricsZoomFrom, metricsZoomTo, parseMetricTs)}
  {@const chartRows = downsampleHistPoints(zoomedHistRows, 120)}
  {@const hasMetricsZoom = metricsZoomFrom != null && metricsZoomTo != null}
  {@const histRx = zoomedHistRows.map((x) => x.rx_bps)}
  {@const histTx = zoomedHistRows.map((x) => x.tx_bps)}
  {@const histMax = Math.max(1, ...histRx, ...histTx)}
  {@const histRxPeak = peakBps(histRx)}
  {@const histTxPeak = peakBps(histTx)}
  {@const histRxAvg = avgBps(histRx)}
  {@const histTxAvg = avgBps(histTx)}
  {@const chartRx = chartRows.map((x) => x.rx_bps)}
  {@const chartTx = chartRows.map((x) => x.tx_bps)}
  {@const chartMax = Math.max(1, ...chartRx, ...chartTx)}
  {@const peakRxIdx = chartRx.length ? chartRx.indexOf(Math.max(...chartRx)) : -1}
  {@const peakTxIdx = chartTx.length ? chartTx.indexOf(Math.max(...chartTx)) : -1}
  {@const pointIdx =
    metricsPointIdx != null
      ? Math.min(chartRows.length - 1, Math.max(0, metricsPointIdx))
      : null}
  {@const pointRow = pointIdx != null ? chartRows[pointIdx] : null}
  {@const rxNow = s ? liveRates[s.routerId]?.[iface]?.rx_bps ?? null : null}
  {@const txNow = s ? liveRates[s.routerId]?.[iface]?.tx_bps ?? null : null}
  {@const warnRx =
    s?.warn_below_rx_bps != null && rxNow != null && rxNow >= 0 && rxNow < s.warn_below_rx_bps}
  {@const warnTx =
    s?.warn_below_tx_bps != null && txNow != null && txNow >= 0 && txNow < s.warn_below_tx_bps}
  <WallboardFullDialog
    {iface}
    routerLabel={r ? (r.identity || r.name) : (s?.routerId || '')}
    routerOnline={!!r?.is_online}
    {fullTab}
    {metricsRange}
    {metricsFromLocal}
    {metricsToLocal}
    {metricsPointIdx}
    {metricsTooltipX}
    {metricsTooltipY}
    {metricsSelecting}
    {metricsSelStart}
    {metricsSelCurrent}
    {fullMetricsLoading}
    {fullMetricsError}
    {rx}
    {tx}
    {rxNow}
    {txNow}
    {warnRx}
    {warnTx}
    {rxPeak}
    {txPeak}
    {rxAvg}
    {txAvg}
    {metricsBucket}
    {hasMetricsZoom}
    {zoomedHistRows}
    {chartRows}
    {chartRx}
    {chartTx}
    {chartMax}
    {histRxPeak}
    {histTxPeak}
    {histRxAvg}
    {histTxAvg}
    {peakRxIdx}
    {peakTxIdx}
    {pointIdx}
    {pointRow}
    {formatBps}
    {formatMetricTs}
    {bucketLabel}
    {bucketHint}
    onClose={closeFull}
    onOpenThreshold={() => openThreshold(fullIndex as number)}
    onOpenFullTab={openFullTab}
    onSetMetricsRange={setMetricsRange}
    onExportMetricsCsv={() =>
      exportMetricsCsv(
        zoomedHistRows,
        iface,
        r ? (r.identity || r.name) : s?.routerId || '',
        metricsBucket,
      )}
    onClearMetricsZoom={clearMetricsZoom}
    onMetricsFromChange={(value) => {
      metricsFromLocal = value;
      metricsPointIdx = null;
      clearMetricsZoom();
    }}
    onMetricsToChange={(value) => {
      metricsToLocal = value;
      metricsPointIdx = null;
      clearMetricsZoom();
    }}
    onBeginMetricsSelection={beginMetricsSelection}
    onMoveMetricsSelection={moveMetricsSelection}
    onEndMetricsSelection={endMetricsSelection}
    onSetMetricsHoverFromMouse={setMetricsHoverFromMouse}
    onSetMetricsHoverFromFocus={setMetricsHoverFromFocus}
    onClearMetricsPoint={() => (metricsPointIdx = null)}
    onSetMetricsPoint={(i) => (metricsPointIdx = i)}
  />
{/if}

{#if thresholdIndex !== null}
  {@const s = slotsAll[thresholdIndex]}
  {@const r = s ? routerById(s.routerId) : null}
  <WallboardThresholdDialog
    iface={s?.iface || ''}
    routerLabel={r ? (r.identity || r.name) : (s?.routerId || '')}
    currentRxBps={s?.warn_below_rx_bps ?? null}
    currentTxBps={s?.warn_below_tx_bps ?? null}
    bind:thWarnRxKbps
    bind:thWarnTxKbps
    bind:thWarnRxUnit
    bind:thWarnTxUnit
    {formatBps}
    onClose={closeThreshold}
    onChangeInterface={() => {
      const idx = thresholdIndex;
      closeThreshold();
      if (idx != null) setTimeout(() => openPicker(idx), 0);
    }}
    onClear={() => {
      thWarnRxKbps = '';
      thWarnTxKbps = '';
      thWarnRxUnit = 'Kbps';
      thWarnTxUnit = 'Kbps';
    }}
    onSave={saveThreshold}
  />
{/if}

<style>
  .wallboard-viewport {
    min-height: 100dvh;
    overflow: hidden;
  }

  :global(body.kiosk-wallboard header.topbar) {
    display: none;
  }
  :global(body.wall-dragging),
  :global(body.wall-dragging *) {
    cursor: grabbing !important;
    user-select: none;
  }
  :global(body.kiosk-wallboard .wrap[role='region']),
  :global(body.kiosk-wallboard .wrap.loading) {
    display: none;
  }
  :global(body.kiosk-wallboard .sidebar) {
    display: none;
  }
  :global(body.kiosk-wallboard .main-viewport) {
    padding-left: clamp(6px, 1vw, 12px);
  }

.wallboard {
    height: 100dvh;
    box-sizing: border-box;
    padding: 22px;
    animation: wallboard-in 180ms ease-out;
  }
  .wallboard.focus .wb-top {
    margin-bottom: 10px;
  }
  .wallboard.focus .controls:not(.wall-actions) {
    width: 100%;
    justify-content: space-between;
  }
  .wallboard.focus .alert-strip {
    margin-bottom: 8px;
  }

  .wb-top {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 18px;
    margin-bottom: 8px;
    transition:
      opacity 180ms ease,
      transform 180ms ease;
  }
  .wb-top.hidden {
    opacity: 0;
    transform: translateY(-8px);
    pointer-events: none;
  }
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
  .controls {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  .controls.wall-actions {
    width: 100%;
    justify-content: flex-end;
    gap: 12px;
  }
  .toolbar-left {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    margin-left: auto;
  }

  .settings-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
    color: var(--text-primary);
    cursor: pointer;
    font-weight: 700;
    font-size: 13px;
    position: relative;
    padding-right: 30px;
  }
  .settings-btn.has-warning {
    color: color-mix(in srgb, #f59e0b 85%, var(--text-primary));
  }
  .settings-btn.has-critical {
    color: color-mix(in srgb, #ef4444 88%, var(--text-primary));
  }
  .insights-badge {
    position: absolute;
    top: 5px;
    right: 7px;
    min-width: 17px;
    height: 17px;
    border-radius: 999px;
    padding: 0 5px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 900;
    line-height: 1;
    color: #fff;
    background: #f59e0b;
    border: 1px solid color-mix(in srgb, #f59e0b 65%, var(--border-color));
  }
  .settings-btn.has-critical .insights-badge {
    background: #ef4444;
    border-color: color-mix(in srgb, #ef4444 65%, var(--border-color));
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

  .empty {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 20px;
    display: flex;
    align-items: center;
    gap: 10px;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
    color: var(--text-muted);
  }
  .pause-indicator {
    position: fixed;
    left: 18px;
    bottom: 18px;
    z-index: 75;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 8px 10px;
    border-radius: 11px;
    border: 1px solid color-mix(in srgb, #f59e0b 45%, var(--border-color));
    background: color-mix(in srgb, #f59e0b 16%, var(--bg-surface));
    color: color-mix(in srgb, #f59e0b 88%, var(--text-primary));
    box-shadow: var(--shadow-md);
    font-weight: 900;
    font-size: 12px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(var(--cols, 3), minmax(0, 1fr));
    grid-template-rows: repeat(var(--rows, 3), minmax(0, 1fr));
    gap: 14px;
    min-height: 0;
    height: calc(100dvh - 110px);
  }

  .tile {
    border: 1px solid var(--border-color);
    border-radius: 18px;
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--bg-surface) 72%, transparent),
      color-mix(in srgb, var(--bg-surface) 92%, transparent)
    );
    overflow: hidden;
    min-height: 0;
  }
  .tile.drag-over {
    outline: 2px dashed color-mix(in srgb, var(--accent) 65%, transparent);
    outline-offset: 4px;
  }
  .tile.add {
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
  }
  .add-inner {
    padding: 18px;
  }
  .plus {
    width: 64px;
    height: 64px;
    border-radius: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px dashed var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 55%, transparent);
    font-size: 34px;
    font-weight: 900;
    color: var(--text-primary);
    margin: 0 auto 12px;
  }
  .add-title {
    font-weight: 900;
    color: var(--text-primary);
  }
  .add-sub {
    margin-top: 4px;
    color: var(--text-muted);
    font-size: 12px;
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
  .icon-x:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border-color));
    background: color-mix(in srgb, var(--accent) 10%, var(--bg-surface));
  }

  .grid.compact {
    gap: 10px;
  }
  .grid.compact .add-inner {
    padding: 10px;
  }
  .grid.compact .plus {
    width: 48px;
    height: 48px;
    font-size: 28px;
    margin-bottom: 8px;
  }

  @keyframes wallboard-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @media (max-width: 1280px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      grid-template-rows: none;
      height: auto;
    }
  }
  @media (max-width: 920px) {
    .wall-insights {
      top: 62px;
      right: 10px;
      bottom: 10px;
      width: calc(100vw - 20px);
      padding: 8px;
    }
    .alert-strip.floating-alert-panel {
      right: 10px;
      bottom: 58px;
      width: calc(100vw - 20px);
    }
    .floating-alert-btn {
      right: 10px;
      bottom: 10px;
    }
    .wallboard.focus .controls {
      justify-content: flex-start;
    }
    .wb-top {
      flex-direction: column;
      align-items: flex-start;
    }
    .controls {
      justify-content: flex-start;
    }
    .controls.wall-actions {
      justify-content: flex-end;
    }
    .toolbar-left {
      width: auto;
      margin-left: auto;
    }
    .pause-indicator {
      left: 10px;
      bottom: 10px;
    }
    .grid {
      grid-template-columns: 1fr;
      grid-template-rows: none;
      height: auto;
    }
  }

</style>
