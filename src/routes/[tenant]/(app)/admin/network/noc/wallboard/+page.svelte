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
    aggregateHistPoints as aggregateHistPointsValue,
    applyMetricsZoom as applyMetricsZoomValue,
    buildHistPoints as buildHistPointsValue,
    downsampleHistPoints as downsampleHistPointsValue,
    filterMetricsRowsByRange,
    parseLocalDate as parseLocalDateValue,
    requiredMetricLimit as requiredMetricLimitValue,
    resolveMetricsBucket as resolveMetricsBucketValue,
    type HistPoint,
    type MetricsRange,
  } from '$lib/components/network/wallboardMetrics';
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
  let metricsAutoRefreshSig = $state('');
  let thresholdIndex = $state<number | null>(null);
  let thWarnRxKbps = $state<string>('');
  let thWarnTxKbps = $state<string>('');
  let thWarnRxUnit = $state<'Kbps' | 'Mbps' | 'Gbps'>('Kbps');
  let thWarnTxUnit = $state<'Kbps' | 'Mbps' | 'Gbps'>('Kbps');

  // Rate computation
  let liveRates = $state<Record<string, Record<string, LiveRate>>>({});
  let series = $state<Record<string, Record<string, { rx: number[]; tx: number[] }>>>({});
  const lastBytes = new Map<string, { rx: number; tx: number; at: number }>();

  let tick: any = null;
  let alertTick: any = null;
  let wakeLock: any = null;
  let livePollInFlight = $state(false);
  let alertSyncInFlight = $state(false);
  let debugEnabled = $state(false);
  let debugStats = $state({
    livePollRuns: 0,
    liveRouterTargets: 0,
    livePollSkipped: 0,
    alertsLoads: 0,
    incidentsLoads: 0,
    metricsLoads: 0,
    metricsFallbackLoads: 0,
    metricsEmptyResponses: 0,
    lastMetricsKey: '',
    lastMetricsAt: 0,
  });
  let remoteLoaded = $state(false);
  let paused = $state(false);
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
  let audioCtx: AudioContext | null = null;

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
    debugStats = { ...debugStats, alertsLoads: debugStats.alertsLoads + 1 };
    alerts = await loadWallboardAlerts(silent);
  }

  async function loadIncidents(silent = true) {
    debugStats = { ...debugStats, incidentsLoads: debugStats.incidentsLoads + 1 };
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

  function normalizeSlotsAllForAutoSpare(input: (Slot | null)[], size: number): (Slot | null)[] {
    if (size <= 0) return input;
    let lastUsed = -1;
    for (let i = input.length - 1; i >= 0; i--) {
      if (input[i]) {
        lastUsed = i;
        break;
      }
    }

    const usedLen = Math.max(0, lastUsed + 1);
    const basePages = Math.max(1, Math.ceil(usedLen / size));
    const baseLen = basePages * size;
    const allFilled = usedLen > 0 && usedLen === baseLen;
    const targetLen = allFilled ? baseLen + size : baseLen;

    const out = input.slice(0, targetLen);
    if (out.length < targetLen) {
      out.push(...Array.from({ length: targetLen - out.length }, () => null));
    }
    return out;
  }

  function routerById(id: string) {
    return rows.find((r) => r.id === id) || null;
  }

  function routerLabel(routerId: string) {
    const rr = routerById(routerId);
    return rr?.identity || rr?.name || routerId;
  }

  function incidentHrefById(id?: string | null) {
    const rid = String(id || '').trim();
    if (!rid) return `${tenantPrefix}/admin/network/incidents`;
    return `${tenantPrefix}/admin/network/incidents?incident=${encodeURIComponent(rid)}`;
  }

  function incidentHrefForTopIssue(routerId: string, title: string) {
    const match = incidents.find((x) => x.router_id === routerId && x.title === title);
    return incidentHrefById(match?.id);
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
    const q = pickerRouterSearch.trim().toLowerCase();
    return filterRows(rows).filter((r) => {
      if (!q) return true;
      const hay = `${r.name} ${r.identity || ''} ${r.host}`.toLowerCase();
      return hay.includes(q);
    });
  }

  function pickerInterfaces() {
    if (!pickerRouterId) return [];
    const iq = pickerIfaceSearch.trim().toLowerCase();
    return (ifaceCatalog[pickerRouterId] || []).filter((i) => {
      if (!iq) return true;
      return i.name.toLowerCase().includes(iq) || (i.interface_type || '').toLowerCase().includes(iq);
    });
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
    void loadFullMetrics(
      idx,
      requiredMetricLimit(next.metricsRange, next.metricsFromLocal, next.metricsToLocal),
    );
  }

  function closeFull() {
    const next = closeFullPanelState();
    fullIndex = next.fullIndex;
    fullMetricsLoading = next.fullMetricsLoading;
    fullMetricsError = next.fullMetricsError;
    fullMetricsRows = next.fullMetricsRows;
    fullMetricsKey = next.fullMetricsKey;
    fullMetricsLimit = next.fullMetricsLimit;
    metricsFromLocal = next.metricsFromLocal;
    metricsToLocal = next.metricsToLocal;
    metricsPointIdx = next.metricsPointIdx;
    metricsZoomFrom = next.metricsZoomFrom;
    metricsZoomTo = next.metricsZoomTo;
    metricsSelecting = next.metricsSelecting;
    metricsAutoRefreshSig = '';
  }

  function requiredMetricLimit(range: MetricsRange, fromLocal: string, toLocal: string) {
    return requiredMetricLimitValue(range, fromLocal, toLocal);
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
    if (state.shouldRefresh) void refreshMetricsForCurrentRange();
  }

  function parseLocalDate(v: string): number | null {
    return parseLocalDateValue(v);
  }

  function filteredFullMetricsRows() {
    return filterMetricsRowsByRange(fullMetricsRows, metricsFromLocal, metricsToLocal, parseMetricTs);
  }

  function buildHistPoints(rows: any[]) {
    return buildHistPointsValue(rows, parseMetricTs);
  }

  function downsampleHistPoints(
    rows: HistPoint[],
    maxPoints: number = 120,
  ) {
    return downsampleHistPointsValue(rows, maxPoints);
  }

  function applyMetricsZoom(
    rows: HistPoint[],
    fromMs: number | null,
    toMs: number | null,
  ) {
    return applyMetricsZoomValue(rows, fromMs, toMs, parseMetricTs);
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

  function resolveMetricsBucket(range: MetricsRange, fromLocal: string, toLocal: string): 'raw' | 'hour' | 'day' {
    return resolveMetricsBucketValue(range, fromLocal, toLocal);
  }

  function aggregateHistPoints(
    rows: HistPoint[],
    bucket: 'raw' | 'hour' | 'day',
  ) {
    return aggregateHistPointsValue(rows, bucket, parseMetricTs);
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
    if (tab === 'metrics') void refreshMetricsForCurrentRange();
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

  function extractMetricsRows(payload: any): any[] {
    return Array.isArray(payload)
      ? payload
      : Array.isArray(payload?.items)
        ? payload.items
        : Array.isArray(payload?.data)
          ? payload.data
          : Array.isArray(payload?.rows)
            ? payload.rows
            : Array.isArray(payload?.history)
              ? payload.history
              : Array.isArray(payload?.metrics)
                ? payload.metrics
                : Array.isArray(payload?.result?.items)
                  ? payload.result.items
                  : Array.isArray(payload?.result?.data)
                    ? payload.result.data
                    : [];
  }

  function rowInterfaceName(row: any): string {
    return normalizeIfaceName(
      String(
      row?.interface ?? row?.interface_name ?? row?.iface ?? row?.name ?? '',
      ),
    );
  }

  function normalizeIfaceName(v: string): string {
    return String(v || '')
      .replace(/[\u200B-\u200D\uFEFF]/g, '')
      .trim()
      .toLowerCase()
      .replace(/\s+/g, ' ');
  }

  function matchesIfaceName(rowName: string, target: string): boolean {
    if (!rowName || !target) return false;
    if (rowName === target) return true;
    return rowName.includes(target) || target.includes(rowName);
  }

  async function loadFullMetrics(idx: number, minLimit: number = 240) {
    const s = slotsAll[idx];
    if (!s) return;

    const key = `${s.routerId}:${s.iface}`;
    if (
      fullMetricsKey === key &&
      fullMetricsRows.length > 0 &&
      fullMetricsLimit >= minLimit
    )
      return;

    fullMetricsKey = key;
    fullMetricsLoading = true;
    fullMetricsError = null;
    debugStats = {
      ...debugStats,
      metricsLoads: debugStats.metricsLoads + 1,
      lastMetricsKey: key,
      lastMetricsAt: Date.now(),
    };

    try {
      const payload = await api.mikrotik.routers.interfaceMetrics(s.routerId, {
        interface: s.iface,
        limit: minLimit,
      });
      let rows = extractMetricsRows(payload);

      if (rows.length === 0 && s.iface) {
        debugStats = {
          ...debugStats,
          metricsFallbackLoads: debugStats.metricsFallbackLoads + 1,
        };
        const fallbackPayload = await api.mikrotik.routers.interfaceMetrics(s.routerId, {
          limit: minLimit,
        });
        const allRows = extractMetricsRows(fallbackPayload);
        const target = normalizeIfaceName(String(s.iface || ''));
        const hasIfaceKey = allRows.some((row) => rowInterfaceName(row));
        if (hasIfaceKey) {
          const strict = allRows.filter((row) => rowInterfaceName(row) === target);
          const fuzzy = strict.length
            ? strict
            : allRows.filter((row) => matchesIfaceName(rowInterfaceName(row), target));
          rows = fuzzy.length ? fuzzy : allRows;
        } else {
          rows = allRows;
        }
        if (debugEnabled) {
          const first = allRows[0];
          console.debug('[wallboard.metrics.fallback]', {
            routerId: s.routerId,
            iface: s.iface,
            requestedLimit: minLimit,
            allRows: allRows.length,
            filteredRows: rows.length,
            firstRowKeys: first ? Object.keys(first) : [],
          });
        }
      }

      if (fullMetricsKey !== key) return;
      fullMetricsRows = rows;
      fullMetricsLimit = minLimit;
      if (rows.length === 0) {
        debugStats = {
          ...debugStats,
          metricsEmptyResponses: debugStats.metricsEmptyResponses + 1,
        };
        if (debugEnabled) {
          console.debug('[wallboard.metrics.empty]', {
            routerId: s.routerId,
            iface: s.iface,
            limit: minLimit,
          });
        }
      }
    } catch (e: any) {
      if (fullMetricsKey !== key) return;
      fullMetricsRows = [];
      fullMetricsError = e?.message || String(e);
    } finally {
      if (fullMetricsKey === key) fullMetricsLoading = false;
    }
  }

  function openThreshold(idx: number) {
    const s = slotsAll[idx];
    if (!s) return;
    thresholdIndex = idx;
    const rx = s.warn_below_rx_bps;
    const tx = s.warn_below_tx_bps;

    if (rx != null && Number.isFinite(rx) && rx > 0) {
      if (rx >= 1_000_000_000) {
        thWarnRxUnit = 'Gbps';
        thWarnRxKbps = String((rx / 1_000_000_000).toFixed(3).replace(/\.?0+$/, ''));
      } else if (rx >= 1_000_000) {
        thWarnRxUnit = 'Mbps';
        thWarnRxKbps = String((rx / 1_000_000).toFixed(3).replace(/\.?0+$/, ''));
      } else {
        thWarnRxUnit = 'Kbps';
        thWarnRxKbps = String((rx / 1_000).toFixed(3).replace(/\.?0+$/, ''));
      }
    } else {
      thWarnRxUnit = 'Kbps';
      thWarnRxKbps = '';
    }

    if (tx != null && Number.isFinite(tx) && tx > 0) {
      if (tx >= 1_000_000_000) {
        thWarnTxUnit = 'Gbps';
        thWarnTxKbps = String((tx / 1_000_000_000).toFixed(3).replace(/\.?0+$/, ''));
      } else if (tx >= 1_000_000) {
        thWarnTxUnit = 'Mbps';
        thWarnTxKbps = String((tx / 1_000_000).toFixed(3).replace(/\.?0+$/, ''));
      } else {
        thWarnTxUnit = 'Kbps';
        thWarnTxKbps = String((tx / 1_000).toFixed(3).replace(/\.?0+$/, ''));
      }
    } else {
      thWarnTxUnit = 'Kbps';
      thWarnTxKbps = '';
    }
  }

  function closeThreshold() {
    thresholdIndex = null;
  }

  function updateSlotThreshold(idx: number, rxBps: number | null, txBps: number | null) {
    const s = slotsAll[idx];
    if (!s) return;
    slotsAll[idx] = {
      ...s,
      warn_below_rx_bps: rxBps,
      warn_below_tx_bps: txBps,
    };
    persistConfig();
  }

  function saveThreshold() {
    if (thresholdIndex == null) return;
    const rxK = Number.parseFloat(thWarnRxKbps || '');
    const txK = Number.parseFloat(thWarnTxKbps || '');
    const unitMul = (u: 'Kbps' | 'Mbps' | 'Gbps') =>
      u === 'Gbps' ? 1_000_000_000 : u === 'Mbps' ? 1_000_000 : 1_000;
    const rxBps = Number.isFinite(rxK) && rxK > 0 ? Math.round(rxK * unitMul(thWarnRxUnit)) : null;
    const txBps = Number.isFinite(txK) && txK > 0 ? Math.round(txK * unitMul(thWarnTxUnit)) : null;
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
    ensureSlotIndex(idx);
    slotsAll[idx] = {
      routerId,
      iface,
      warn_below_rx_bps: warnBelowRxBps ?? null,
      warn_below_tx_bps: warnBelowTxBps ?? null,
    };
    pickerIndex = null;
    pickerRouterId = null;
    persistConfig();
  }

  function clearSlot(idx: number) {
    ensureSlotIndex(idx);
    slotsAll[idx] = null;
    persistConfig();
  }

  async function loadInterfaces(routerId: string) {
    if (ifaceCatalog[routerId]?.length) return;
    ifaceLoading[routerId] = true;
    try {
      const snap = await api.mikrotik.routers.snapshot(routerId);
      const list = ((snap?.interfaces || []) as any[]).map((i) => ({
        name: String(i?.name || ''),
        interface_type: (i?.interface_type ?? null) as string | null,
        running: !!i?.running,
        disabled: !!i?.disabled,
      }));
      ifaceCatalog[routerId] = list.filter((i) => i.name);
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      ifaceLoading[routerId] = false;
    }
  }

  function persistConfig() {
    persistWallboardLocalConfigValue({
      layout,
      slotsAll,
      rotateMode,
      rotateMs,
      focusMode,
      statusFilter,
      pollMs,
      criticalSoundEnabled,
    });
  }

  function loadConfig() {
    const conf = loadWallboardLocalConfigValue();
    if (conf.layout) layout = conf.layout;
    if (conf.rotateMode) rotateMode = conf.rotateMode;
    if (conf.rotateMs != null) rotateMs = conf.rotateMs;
    if (conf.statusFilter) statusFilter = conf.statusFilter;
    if (conf.pollMs != null) pollMs = conf.pollMs;
    if (conf.criticalSoundEnabled != null) criticalSoundEnabled = conf.criticalSoundEnabled;
    if (conf.slotsAll) slotsAll = conf.slotsAll;
  }

  async function loadRemoteConfig() {
    const conf = await loadWallboardRemoteConfigValue({
      canUseTenantSettings,
      getValue: (key) => api.settings.getValue(key),
    });
    if (conf.layout) layout = conf.layout;
    if (conf.slotsAll) slotsAll = conf.slotsAll;
    remoteLoaded = conf.remoteLoaded;
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
      // Clear slots that reference removed routers.
      const ids = new Set(rows.map((r) => r.id));
      slotsAll = slotsAll.map((s) => (s && ids.has(s.routerId) ? s : null));
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      refreshing = false;
    }
    await syncAlertsIncidents(true);
  }

  function filterRows(list: NocRow[]) {
    return list.filter((r) => {
      if (statusFilter === 'online' && !r.is_online) return false;
      if (statusFilter === 'offline' && r.is_online) return false;
      return true;
    });
  }

  function activeRouterCount() {
    const ids = new Set<string>();
    for (const s of slotsAll) {
      if (s?.routerId) ids.add(s.routerId);
    }
    return ids.size;
  }

  function effectivePollMs() {
    const active = activeRouterCount();
    if (active >= 10) return Math.max(pollMs, 5000);
    if (active >= 6) return Math.max(pollMs, 3000);
    return pollMs;
  }

  async function pollLiveOnce() {
    if (livePollInFlight) {
      debugStats = { ...debugStats, livePollSkipped: debugStats.livePollSkipped + 1 };
      return;
    }
    livePollInFlight = true;
    try {
      debugStats = {
        ...debugStats,
        livePollRuns: debugStats.livePollRuns + 1,
        liveRouterTargets: debugStats.liveRouterTargets + activeRouterCount(),
      };
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
      renderNow = next.renderNow;
    } finally {
      livePollInFlight = false;
    }
  }

  function setPaused(on: boolean) {
    paused = on;
    if (paused) {
      if (tick) clearInterval(tick);
      tick = null;
    } else {
      if (!tick) tick = setInterval(() => void pollLiveOnce(), effectivePollMs());
    }
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

  async function debugProbeMetrics() {
    const firstIdx = slotsAll.findIndex((s) => !!s?.routerId && !!s?.iface);
    if (firstIdx < 0) {
      console.debug('[wallboard.metrics.probe] no configured slot');
      return;
    }
    const s = slotsAll[firstIdx]!;
    const key = `${s.routerId}:${s.iface}`;
    debugStats = {
      ...debugStats,
      metricsLoads: debugStats.metricsLoads + 1,
      lastMetricsKey: key,
      lastMetricsAt: Date.now(),
    };
    try {
      const payload = await api.mikrotik.routers.interfaceMetrics(s.routerId, {
        interface: s.iface,
        limit: 200,
      });
      const rows = extractMetricsRows(payload);
      if (rows.length === 0) {
        debugStats = {
          ...debugStats,
          metricsEmptyResponses: debugStats.metricsEmptyResponses + 1,
        };
      }
      const first = rows[0];
      console.debug('[wallboard.metrics.probe]', {
        slotIndex: firstIdx,
        routerId: s.routerId,
        iface: s.iface,
        rows: rows.length,
        firstRowKeys: first ? Object.keys(first) : [],
        firstRow: first || null,
      });
    } catch (e: any) {
      console.debug('[wallboard.metrics.probe.error]', e?.message || String(e));
    }
  }

  function installAutoHideListeners() {
    if (typeof window === 'undefined') return;

    const onAny = () => showControls();
    const onPointerDown = (e: PointerEvent) => {
      const target = e.target as HTMLElement | null;
      if (!target?.closest?.('.tile-actions')) tileMenuIndex = null;
      showControls();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (alertsOpen) {
          alertsOpen = false;
          return;
        }
      }
      if (e.key.toLowerCase() === 'f' && !e.metaKey && !e.ctrlKey && !e.altKey) {
        const tag = (e.target as HTMLElement | null)?.tagName?.toLowerCase() || '';
        const editing = tag === 'input' || tag === 'textarea' || tag === 'select';
        if (!editing) {
          focusMode = !focusMode;
          e.preventDefault();
        }
      }
      if (e.key === 'Escape') return;
      showControls();
    };

    window.addEventListener('mousemove', onAny, { passive: true });
    window.addEventListener('pointermove', onAny, { passive: true });
    window.addEventListener('pointerdown', onPointerDown, { passive: true });
    window.addEventListener('wheel', onAny, { passive: true });
    window.addEventListener('touchstart', onAny, { passive: true });
    window.addEventListener('keydown', onKey);

    return () => {
      window.removeEventListener('mousemove', onAny as any);
      window.removeEventListener('pointermove', onAny as any);
      window.removeEventListener('pointerdown', onPointerDown as any);
      window.removeEventListener('wheel', onAny as any);
      window.removeEventListener('touchstart', onAny as any);
      window.removeEventListener('keydown', onKey as any);
    };
  }

  async function toggleFullscreen() {
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await document.documentElement.requestFullscreen();
    } catch {
      // ignore
    }
  }

  async function playCriticalBeep() {
    if (!criticalSoundEnabled || typeof window === 'undefined') return;
    const AC = (window as any).AudioContext || (window as any).webkitAudioContext;
    if (!AC) return;
    if (!audioCtx) {
      audioCtx = new AC();
    }
    const ctx = audioCtx;
    if (!ctx) return;
    if (ctx.state === 'suspended') {
      try {
        await ctx.resume();
      } catch {
        return;
      }
    }

    const base = ctx.currentTime;
    const pulse = (start: number, freq: number, gainValue: number, dur = 0.12) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'sine';
      osc.frequency.value = freq;
      gain.gain.setValueAtTime(0.0001, start);
      gain.gain.exponentialRampToValueAtTime(gainValue, start + 0.01);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + dur);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(start);
      osc.stop(start + dur + 0.02);
    };

    pulse(base, 740, 0.028, 0.14);
    pulse(base + 0.18, 660, 0.03, 0.16);
  }

  async function applyWakeLock(on: boolean) {
    if (typeof navigator === 'undefined') return;
    // @ts-ignore
    const wl = navigator.wakeLock;
    if (!wl) return;
    try {
      if (on) {
        // @ts-ignore
        wakeLock = await wl.request('screen');
      } else {
        await wakeLock?.release?.();
        wakeLock = null;
      }
    } catch {
      // ignore
    }
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
    if (typeof window !== 'undefined') {
      const qs = new URLSearchParams(window.location.search);
      const byQuery = qs.get('wallDebug') === '1';
      const byStorage = localStorage.getItem('wallboard_debug') === '1';
      debugEnabled = byQuery || byStorage;
      if (byQuery) localStorage.setItem('wallboard_debug', '1');
    }
    ensureSlots();
    // Wallboard is meant for NOC/full window display. Keep kiosk enabled while this route is active.
    kiosk = true;
    applyKiosk(true);
    void loadRemoteConfig();

    uninstallAutoHide = installAutoHideListeners() ?? null;
    showControls();
    isFullscreen = typeof document !== 'undefined' && !!document.fullscreenElement;

    const onFullscreenChange = () => {
      isFullscreen = !!document.fullscreenElement;
    };
    if (typeof document !== 'undefined') {
      document.addEventListener('fullscreenchange', onFullscreenChange);
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

    tick = setInterval(() => void pollLiveOnce(), effectivePollMs());
    alertTick = setInterval(() => {
      void syncAlertsIncidents(true);
    }, 10000);

    return () => {
      if (typeof document !== 'undefined') {
        document.removeEventListener('fullscreenchange', onFullscreenChange);
      }
    };
  });

  onDestroy(() => {
    if (tick) clearInterval(tick);
    if (alertTick) clearInterval(alertTick);
    remotePersister.clearScheduledPersist();
    dndController.dispose();
    // Best-effort flush so layout/slots don't get lost on fast logout/navigation.
    void persistRemoteNow();
    void applyWakeLock(false);
    if (typeof document !== 'undefined') document.body.classList.remove('kiosk-wallboard');
    if (hideHandle) clearTimeout(hideHandle);
    try {
      audioCtx?.close?.();
    } catch {}
    try {
      uninstallAutoHide?.();
    } catch {}
  });

  $effect(() => {
    if (fullIndex == null || fullTab !== 'metrics') return;
    const sig = `${fullIndex}|${metricsRange}|${metricsFromLocal}|${metricsToLocal}`;
    if (sig === metricsAutoRefreshSig) return;
    metricsAutoRefreshSig = sig;
    void refreshMetricsForCurrentRange();
  });

  $effect(() => {
    // restart polling when interval changes or pause changes
    const nextPollMs = effectivePollMs();
    if (paused) {
      if (tick) clearInterval(tick);
      tick = null;
      return;
    }
    if (tick) clearInterval(tick);
    tick = setInterval(() => void pollLiveOnce(), nextPollMs);
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
    const normalized = normalizeSlotsAllForAutoSpare(slotsAll, size);
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
    const criticalIds = sortedAlerts
      .filter((a) => String(a.severity || '').toLowerCase() === 'critical')
      .map((a) => a.id)
      .sort();
    const sig = criticalIds.join(',');

    if (!sig) {
      lastCriticalSignature = '';
      return;
    }
    if (!criticalSoundEnabled || paused) {
      lastCriticalSignature = sig;
      return;
    }
    if (!lastCriticalSignature) {
      lastCriticalSignature = sig;
      return;
    }
    if (sig !== lastCriticalSignature) {
      const now = Date.now();
      if (now - lastCriticalBeepAt >= 8000) {
        lastCriticalBeepAt = now;
        void playCriticalBeep();
      }
    }
    lastCriticalSignature = sig;
  });

  $effect(() => {
    const current: Record<string, string> = {};
    for (const a of sortedAlerts) {
      const sev = String(a.severity || '').toLowerCase();
      current[a.id] = sev;
    }

    const prevKeys = Object.keys(alertSnapshot);
    if (prevKeys.length === 0) {
      alertSnapshot = current;
      return;
    }

    for (const a of sortedAlerts) {
      if (alertSnapshot[a.id]) continue;
      const sev = String(a.severity || '').toLowerCase();
      if (sev === 'critical' || sev === 'warning') {
        pushIncident(
          sev as 'critical' | 'warning',
          `${a.title || 'Alert'} · ${a.message || ''}`.trim(),
          a.router_id,
        );
      }
    }
    alertSnapshot = current;
  });
</script>

<div class="wallboard-viewport">
  <div class="wallboard" class:focus={focusMode}>
  <div class="wb-top" class:hidden={controlsHidden}>
    <div class="controls wall-actions">
      <div class="toolbar-left">
        {#if pageCount > 1}
          <div class="top-pager" aria-label={$t('admin.network.wallboard.pager.aria') || 'Pages'}>
            <button
              class="top-pager-btn"
              type="button"
              onclick={() => (page = Math.max(0, page - 1))}
              disabled={page === 0}
              aria-label={$t('admin.network.wallboard.pager.prev') || 'Previous page'}
            >
              <Icon name="chevron-left" size={15} />
            </button>
            <span class="top-pager-label">
              {($t('common.page') || 'Page') + ' ' + (page + 1) + '/' + pageCount}
            </span>
            <button
              class="top-pager-btn"
              type="button"
              onclick={() => (page = Math.min(pageCount - 1, page + 1))}
              disabled={page >= pageCount - 1}
              aria-label={$t('admin.network.wallboard.pager.next') || 'Next page'}
            >
              <Icon name="chevron-right" size={15} />
            </button>
          </div>
        {/if}
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
        onGotoTopIssue={(routerId, title) => goto(incidentHrefForTopIssue(routerId, title))}
        onMuteTopIssue={(routerId, mins) => void muteRouterAlerts(routerId, mins)}
        onUnmuteTopIssue={(routerId) => void unmuteRouter(routerId)}
        onOpenIncident={(incidentId) => goto(incidentHrefById(incidentId))}
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

  {#if debugEnabled}
    <div class="wall-debug">
      <div>poll: {debugStats.livePollRuns} run / {debugStats.liveRouterTargets} router-target</div>
      <div>poll-skip(overlap): {debugStats.livePollSkipped}</div>
      <div>alerts: {debugStats.alertsLoads} | incidents: {debugStats.incidentsLoads}</div>
      <div>
        metrics: {debugStats.metricsLoads} | fallback: {debugStats.metricsFallbackLoads} | empty:{' '}
        {debugStats.metricsEmptyResponses}
      </div>
      <div>last metrics key: {debugStats.lastMetricsKey || '-'}</div>
      <div>effective poll: {effectivePollMs()}ms (base {pollMs}ms)</div>
      <button class="wall-debug-btn" type="button" onclick={() => void debugProbeMetrics()}>
        probe metrics
      </button>
    </div>
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
  {@const filteredMetricsRows = filteredFullMetricsRows()}
  {@const histRawRows = buildHistPoints(filteredMetricsRows)}
  {@const metricsBucket = resolveMetricsBucket(metricsRange, metricsFromLocal, metricsToLocal)}
  {@const histRows = aggregateHistPoints(histRawRows, metricsBucket)}
  {@const zoomedHistRows = applyMetricsZoom(histRows, metricsZoomFrom, metricsZoomTo)}
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
      void refreshMetricsForCurrentRange();
    }}
    onMetricsToChange={(value) => {
      metricsToLocal = value;
      metricsPointIdx = null;
      clearMetricsZoom();
      void refreshMetricsForCurrentRange();
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
  .top-pager {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
  }
  .top-pager-label {
    font-weight: 850;
    font-size: 12px;
    color: var(--text-muted);
    min-width: 76px;
    text-align: center;
    white-space: nowrap;
  }
  .top-pager-btn {
    width: 30px;
    height: 30px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 80%, transparent);
    color: var(--text-primary);
    display: grid;
    place-items: center;
    cursor: pointer;
    padding: 0;
  }
  .top-pager-btn:disabled {
    opacity: 0.55;
    cursor: default;
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
  .wall-debug {
    position: fixed;
    left: 18px;
    top: 86px;
    z-index: 76;
    min-width: 320px;
    max-width: min(620px, calc(100vw - 36px));
    border-radius: 12px;
    border: 1px solid color-mix(in srgb, var(--accent) 35%, var(--border-color));
    background: color-mix(in srgb, var(--bg-surface) 88%, #000 12%);
    box-shadow: var(--shadow-md);
    color: var(--text-primary);
    font: 12px/1.35 ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
    padding: 8px 10px;
    display: grid;
    gap: 2px;
    pointer-events: auto;
  }
  .wall-debug-btn {
    margin-top: 4px;
    justify-self: start;
    border-radius: 8px;
    border: 1px solid color-mix(in srgb, var(--accent) 35%, var(--border-color));
    background: color-mix(in srgb, var(--accent) 10%, var(--bg-surface));
    color: var(--text-primary);
    padding: 4px 8px;
    font: inherit;
    cursor: pointer;
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
  .tile.iface-tile {
    cursor: pointer;
  }
  .tile.iface-tile.warn {
    border-color: color-mix(in srgb, var(--color-danger) 55%, var(--border-color));
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-danger) 20%, transparent);
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
    .top-pager {
      padding: 5px 7px;
    }
    .top-pager-label {
      min-width: 68px;
      font-size: 11px;
    }
    .pause-indicator {
      left: 10px;
      bottom: 10px;
    }
    .wall-debug {
      left: 10px;
      top: 74px;
      max-width: calc(100vw - 20px);
    }
    .grid {
      grid-template-columns: 1fr;
      grid-template-rows: none;
      height: auto;
    }
  }

</style>
