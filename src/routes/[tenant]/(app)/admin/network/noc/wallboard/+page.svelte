<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page as pageStore } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can, user, tenant } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import WallboardAlertsPanel from '$lib/components/network/WallboardAlertsPanel.svelte';
  import WallboardInsightsControls from '$lib/components/network/WallboardInsightsControls.svelte';
  import WallboardInsightsSummary from '$lib/components/network/WallboardInsightsSummary.svelte';
  import {
    ALERT_SOUND_KEY,
    FOCUS_MODE_KEY,
    KEEP_AWAKE_KEY,
    POLL_MS_KEY,
    ROTATE_MODE_KEY,
    ROTATE_MS_KEY,
    SETTINGS_LAYOUT_KEY,
    SETTINGS_SLOTS_KEY,
    STATUS_FILTER_KEY,
    WALLBOARD_ROTATE_MS_OPTIONS,
    WALLBOARD_POLL_MS_OPTIONS,
    isLayoutPreset,
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

  type LiveCounter = {
    name: string;
    running: boolean;
    disabled: boolean;
    rx_byte: number;
    tx_byte: number;
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
  type TrendInfo = {
    dir: 'up' | 'down' | 'flat';
    deltaPct: number;
  };
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
  type IncidentKind =
    | 'critical'
    | 'warning'
    | 'ack'
    | 'mute'
    | 'unmute'
    | 'poll_error'
    | 'recovered';
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
  let metricsRange = $state<'24h' | '7d' | '30d' | 'month' | 'custom'>('24h');
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
  let persistTimer: any = null;
  let lastRemotePayload: string | null = null;
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
    if (bps == null) return $t('common.na') || '—';
    const abs = Math.abs(bps);
    const units = ['bps', 'Kbps', 'Mbps', 'Gbps'];
    let u = 0;
    let v = abs;
    while (v >= 1000 && u < units.length - 1) {
      v /= 1000;
      u++;
    }
    const s = `${v >= 10 || u === 0 ? v.toFixed(0) : v.toFixed(1)} ${units[u]}`;
    return bps < 0 ? `-${s}` : s;
  }

  function formatLatency(ms?: number | null) {
    if (ms == null || !Number.isFinite(ms)) return $t('common.na') || '—';
    const v = Number(ms);
    if (v < 1000) return `${Math.round(v)} ms`;
    return `${(v / 1000).toFixed(2)} s`;
  }

  function maintenanceRemaining(until?: string | null) {
    const raw = String(until || '').trim();
    if (!raw) return null;
    const end = Date.parse(raw);
    if (!Number.isFinite(end)) return null;
    const diffMs = (end as number) - Date.now();
    if (diffMs <= 0) return null;
    const totalMin = Math.ceil(diffMs / 60_000);
    if (totalMin < 60) return `${totalMin}m`;
    const hh = Math.floor(totalMin / 60);
    const mm = totalMin % 60;
    return mm > 0 ? `${hh}h ${mm}m` : `${hh}h`;
  }

  function parseMetricTs(ts?: string | null): number | null {
    const raw = String(ts || '').trim();
    if (!raw) return null;

    let ms = Date.parse(raw);
    if (Number.isFinite(ms)) return ms;

    // Fallback parser for timestamps with long fractional seconds or space separator.
    let normalized = raw.includes(' ') && !raw.includes('T') ? raw.replace(' ', 'T') : raw;
    normalized = normalized.replace(
      /\.(\d{3})\d+(Z|[+-]\d{2}:?\d{2})$/,
      '.$1$2',
    );
    normalized = normalized.replace(/\.(\d{3})\d+$/, '.$1');
    ms = Date.parse(normalized);
    if (Number.isFinite(ms)) return ms;

    // Last fallback: if no timezone suffix, treat as UTC.
    if (!/[zZ]|[+-]\d{2}:?\d{2}$/.test(normalized)) {
      ms = Date.parse(`${normalized}Z`);
      if (Number.isFinite(ms)) return ms;
    }
    return null;
  }

  function formatMetricTs(ts?: string | null) {
    const ms = parseMetricTs(ts);
    if (!Number.isFinite(ms)) return '—';
    return new Date(ms as number).toLocaleString();
  }

  function peakBps(list: number[]) {
    if (!list.length) return null;
    return Math.max(...list);
  }

  function avgBps(list: number[]) {
    if (!list.length) return null;
    return Math.round(list.reduce((a, b) => a + b, 0) / list.length);
  }

  function routerTitle(r: NocRow) {
    const name = r.identity || r.name;
    const ros = r.ros_version ? ` • ROS ${r.ros_version}` : '';
    return `${name}${ros}`;
  }

  const sortedAlerts = $derived.by(() => {
    const rank = (s?: string) => {
      const x = String(s || '').toLowerCase();
      if (x === 'critical') return 3;
      if (x === 'warning') return 2;
      return 1;
    };
    return [...alerts]
      .filter((a) => String(a.status || '').toLowerCase() !== 'resolved')
      .sort((a, b) => {
        const bySeverity = rank(b.severity) - rank(a.severity);
        if (bySeverity !== 0) return bySeverity;
        const ta = Date.parse(a.last_seen_at || a.updated_at || '');
        const tb = Date.parse(b.last_seen_at || b.updated_at || '');
        return (Number.isFinite(tb) ? tb : 0) - (Number.isFinite(ta) ? ta : 0);
      });
  });

  const routerAlertMap = $derived.by(() => {
    const map: Record<string, { total: number; critical: number; warning: number; ids: string[] }> = {};
    for (const a of sortedAlerts) {
      const rid = String(a.router_id || '');
      if (!rid) continue;
      map[rid] ||= { total: 0, critical: 0, warning: 0, ids: [] };
      map[rid].total += 1;
      const sev = String(a.severity || '').toLowerCase();
      if (sev === 'critical') map[rid].critical += 1;
      else if (sev === 'warning') map[rid].warning += 1;
      map[rid].ids.push(a.id);
    }
    return map;
  });

  const alertStats = $derived.by(() => {
    let critical = 0;
    let warning = 0;
    for (const a of sortedAlerts) {
      const sev = String(a.severity || '').toLowerCase();
      if (sev === 'critical') critical += 1;
      else if (sev === 'warning') warning += 1;
    }
    return { total: sortedAlerts.length, critical, warning };
  });

  const activeIncidents = $derived.by(() =>
    incidents.filter((i) => {
      const status = String(i.status || '').toLowerCase();
      const resolvedAt = String(i.resolved_at || '').trim();
      return status !== 'resolved' && !resolvedAt;
    }),
  );

  function severityScore(s?: string) {
    const sev = String(s || '').toLowerCase();
    if (sev === 'critical') return 3;
    if (sev === 'warning') return 2;
    return 1;
  }

  const sortedActiveIncidents = $derived.by(() =>
    [...activeIncidents].sort((a, b) => {
      const sev = severityScore(b.severity) - severityScore(a.severity);
      if (sev !== 0) return sev;
      const ta = Date.parse(a.last_seen_at || a.updated_at || '');
      const tb = Date.parse(b.last_seen_at || b.updated_at || '');
      return (Number.isFinite(tb) ? tb : 0) - (Number.isFinite(ta) ? ta : 0);
    }),
  );

  const openIncidentItems = $derived.by(() => sortedActiveIncidents.slice(0, 8));

  const incidentStats = $derived.by(() => {
    let critical = 0;
    let warning = 0;
    for (const i of activeIncidents) {
      const sev = String(i.severity || '').toLowerCase();
      if (sev === 'critical') critical += 1;
      else if (sev === 'warning') warning += 1;
    }
    return { total: activeIncidents.length, critical, warning };
  });

  const visibleAlerts = $derived.by(() => {
    if (alertSeverityFilter === 'all') return sortedAlerts;
    return sortedAlerts.filter((a) => String(a.severity || '').toLowerCase() === alertSeverityFilter);
  });

  const globalSummary = $derived.by(() => {
    const total = rows.length;
    const online = rows.filter((r) => !!r.is_online).length;
    const offline = Math.max(0, total - online);
    const availability = total > 0 ? (online / total) * 100 : 0;
    const latencies = rows
      .filter((r) => !!r.is_online && Number.isFinite(r.latency_ms))
      .map((r) => Number(r.latency_ms));
    const avgLatencyMs = latencies.length
      ? latencies.reduce((a, b) => a + b, 0) / latencies.length
      : null;

    return {
      total,
      online,
      offline,
      availability,
      critical: incidentStats.critical,
      warning: incidentStats.warning,
      avgLatencyMs,
    };
  });

  const topIssues = $derived.by(() => {
    const oneHourAgo = Date.now() - 60 * 60 * 1000;
    const map = new Map<string, TopIssue>();
    for (const i of activeIncidents) {
      const tsRaw = i.last_seen_at || i.updated_at || '';
      const ts = Date.parse(tsRaw);
      if (!Number.isFinite(ts) || (ts as number) < oneHourAgo) continue;
      const routerId = String(i.router_id || '');
      if (!routerId) continue;
      const title = String(i.title || '').trim() || 'Incident';
      const key = `${routerId}::${title.toLowerCase()}`;
      const sev = String(i.severity || '').toLowerCase();

      if (!map.has(key)) {
        const rr = routerById(routerId);
        map.set(key, {
          key,
          router_id: routerId,
          router_name: rr?.identity || rr?.name || routerId,
          title,
          count: 0,
          critical: 0,
          warning: 0,
          lastSeenMs: ts as number,
        });
      }
      const cur = map.get(key)!;
      cur.count += 1;
      if (sev === 'critical') cur.critical += 1;
      else if (sev === 'warning') cur.warning += 1;
      cur.lastSeenMs = Math.max(cur.lastSeenMs, ts as number);
    }

    return Array.from(map.values())
      .sort((a, b) => {
        if (b.critical !== a.critical) return b.critical - a.critical;
        if (b.count !== a.count) return b.count - a.count;
        return b.lastSeenMs - a.lastSeenMs;
      })
      .slice(0, 5);
  });

  const insightsBadge = $derived.by(() => {
    const critical = incidentStats.critical;
    const warning = incidentStats.warning;
    const total = critical + warning;
    return {
      total,
      level: critical > 0 ? 'critical' : warning > 0 ? 'warning' : 'ok',
    } as const;
  });

  function calcTrend(list: number[]): TrendInfo {
    const points = list.filter((v) => Number.isFinite(v));
    if (points.length < 10) return { dir: 'flat', deltaPct: 0 };

    const win = Math.max(3, Math.min(6, Math.floor(points.length / 2)));
    const cur = points.slice(-win);
    const prev = points.slice(-(win * 2), -win);
    if (!cur.length || !prev.length) return { dir: 'flat', deltaPct: 0 };

    const avg = (arr: number[]) => arr.reduce((a, b) => a + b, 0) / Math.max(1, arr.length);
    const curAvg = avg(cur);
    const prevAvg = avg(prev);
    const base = Math.max(1, prevAvg);
    const deltaPct = ((curAvg - prevAvg) / base) * 100;

    if (Math.abs(deltaPct) < 5) return { dir: 'flat', deltaPct: 0 };
    return { dir: deltaPct > 0 ? 'up' : 'down', deltaPct };
  }

  function trendBadgeText(ti: TrendInfo) {
    if (ti.dir === 'flat') return $t('admin.network.wallboard.trend.stable') || 'Stable';
    const pct = Math.abs(ti.deltaPct);
    const num = pct >= 10 ? Math.round(pct).toString() : pct.toFixed(1);
    return `${ti.dir === 'up' ? '↑' : '↓'} ${num}%`;
  }

  function trendLabel(ti: TrendInfo) {
    if (ti.dir === 'up') return $t('admin.network.wallboard.trend.up') || 'Rising';
    if (ti.dir === 'down') return $t('admin.network.wallboard.trend.down') || 'Falling';
    return $t('admin.network.wallboard.trend.stable') || 'Stable';
  }

  async function loadAlerts(silent = true) {
    try {
      alerts = (await api.mikrotik.alerts.list({ activeOnly: true, limit: 300 })) as any;
    } catch (e: any) {
      if (!silent) toast.error(e?.message || e);
    }
  }

  async function loadIncidents(silent = true) {
    try {
      incidents = (await api.mikrotik.incidents.list({ activeOnly: false, limit: 500 })) as any;
    } catch (e: any) {
      if (!silent) toast.error(e?.message || e);
    }
  }

  async function ackIncident(id: string) {
    if (!$can('manage', 'network_routers')) return;
    try {
      await api.mikrotik.incidents.ack(id);
      toast.success($t('admin.network.alerts.toasts.acked') || 'Alert acknowledged');
      pushIncident('ack', 'Incident acknowledged');
      await Promise.all([loadAlerts(true), loadIncidents(true)]);
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function resolveIncident(id: string) {
    if (!$can('manage', 'network_routers')) return;
    try {
      await api.mikrotik.incidents.resolve(id);
      toast.success($t('admin.network.alerts.toasts.resolved') || 'Alert resolved');
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
      for (const id of ids.slice(0, 50)) {
        await api.mikrotik.alerts.ack(id);
      }
      toast.success($t('admin.network.alerts.toasts.acked') || 'Alert acknowledged');
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
      for (const id of ids.slice(0, 80)) {
        await api.mikrotik.alerts.ack(id);
      }
      toast.success(
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
      const until = new Date(Date.now() + minutes * 60 * 1000).toISOString();
      await api.mikrotik.routers.update(routerId, {
        maintenance_until: until,
        maintenance_reason: `Snoozed from wallboard for ${minutes}m`,
      });
      toast.success($t('admin.network.alerts.toasts.snoozed') || 'Router snoozed');
      pushIncident('mute', `Mute ${minutes}m`, routerId);
      await refresh();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function unmuteRouter(routerId: string) {
    if (!$can('manage', 'network_routers')) return;
    try {
      await api.mikrotik.routers.update(routerId, {
        maintenance_until: null,
        maintenance_reason: null,
      });
      pushIncident('unmute', 'Maintenance cleared', routerId);
      toast.success($t('admin.network.wallboard.unmuted') || 'Maintenance cleared');
      await refresh();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  function selectedMuteMinutes(routerId: string) {
    const v = topIssueMuteMinutes[routerId];
    return v === 60 || v === 240 ? v : 30;
  }

  function slotCountForLayout(p: LayoutPreset) {
    switch (p) {
      case '2x2':
        return 4;
      case '3x2':
        return 6;
      case '3x3':
        return 9;
      case '4x3':
        return 12;
    }
  }

  function colsForLayout(p: LayoutPreset) {
    switch (p) {
      case '2x2':
        return 2;
      case '3x2':
        return 3;
      case '3x3':
        return 3;
      case '4x3':
        return 4;
    }
  }

  function rowsForLayout(p: LayoutPreset) {
    switch (p) {
      case '2x2':
        return 2;
      case '3x2':
        return 2;
      case '3x3':
        return 3;
      case '4x3':
        return 3;
    }
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

  function formatIncidentTs(ms: number) {
    return new Date(ms).toLocaleString();
  }

  function kindClass(kind: IncidentKind) {
    if (kind === 'critical' || kind === 'poll_error') return 'critical';
    if (kind === 'warning' || kind === 'mute') return 'warning';
    return 'ok';
  }

  function kindLabel(kind: IncidentKind) {
    if (kind === 'critical') return 'Critical';
    if (kind === 'warning') return 'Warning';
    if (kind === 'ack') return 'Ack';
    if (kind === 'mute') return 'Mute';
    if (kind === 'unmute') return 'Unmute';
    if (kind === 'poll_error') return 'Poll Error';
    return 'Recovered';
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

  function openFull(idx: number) {
    fullIndex = idx;
    fullTab = 'live';
    clearMetricsZoom();
    setMetricsRange('24h');
    void loadFullMetrics(idx, requiredMetricLimit(metricsRange, metricsFromLocal, metricsToLocal));
  }

  function closeFull() {
    fullIndex = null;
    fullMetricsLoading = false;
    fullMetricsError = null;
    fullMetricsRows = [];
    fullMetricsKey = '';
    fullMetricsLimit = 0;
    metricsFromLocal = '';
    metricsToLocal = '';
    metricsPointIdx = null;
    clearMetricsZoom();
    metricsSelecting = false;
  }

  function toLocalInput(date: Date) {
    const y = date.getFullYear();
    const m = String(date.getMonth() + 1).padStart(2, '0');
    const d = String(date.getDate()).padStart(2, '0');
    const hh = String(date.getHours()).padStart(2, '0');
    const mm = String(date.getMinutes()).padStart(2, '0');
    return `${y}-${m}-${d}T${hh}:${mm}`;
  }

  function requiredMetricLimit(
    range: '24h' | '7d' | '30d' | 'month' | 'custom',
    fromLocal: string,
    toLocal: string,
  ) {
    if (range === '24h') return 400;
    if (range === '7d') return 2500;
    if (range === '30d') return 10000;
    if (range === 'month') return 10000;
    const fromMs = parseLocalDate(fromLocal);
    const toMs = parseLocalDate(toLocal);
    if (fromMs != null && toMs != null && toMs > fromMs) {
      const days = Math.ceil((toMs - fromMs) / (24 * 60 * 60 * 1000));
      if (days <= 2) return 800;
      if (days <= 8) return 3000;
      return 10000;
    }
    return 10000;
  }

  async function refreshMetricsForCurrentRange() {
    if (fullIndex == null) return;
    const limit = requiredMetricLimit(metricsRange, metricsFromLocal, metricsToLocal);
    await loadFullMetrics(fullIndex, limit);
  }

  function setMetricsRange(next: '24h' | '7d' | '30d' | 'month' | 'custom') {
    metricsRange = next;
    metricsPointIdx = null;
    clearMetricsZoom();
    if (next === 'custom') return;

    const now = new Date();
    let from = new Date(now);
    if (next === '24h') from.setHours(from.getHours() - 24);
    else if (next === '7d') from.setDate(from.getDate() - 7);
    else if (next === '30d') from.setDate(from.getDate() - 30);
    else if (next === 'month') from = new Date(now.getFullYear(), now.getMonth(), 1, 0, 0, 0, 0);

    metricsFromLocal = toLocalInput(from);
    metricsToLocal = toLocalInput(now);
    void refreshMetricsForCurrentRange();
  }

  function parseLocalDate(v: string): number | null {
    if (!v) return null;
    const ms = Date.parse(v);
    return Number.isFinite(ms) ? ms : null;
  }

  function filteredFullMetricsRows() {
    const fromMs = parseLocalDate(metricsFromLocal);
    const toMs = parseLocalDate(metricsToLocal);
    if (fromMs == null && toMs == null) return fullMetricsRows;

    const filtered = fullMetricsRows.filter((row) => {
      const ts = parseMetricTs(row?.ts);
      if (ts == null) return false;
      if (fromMs != null && ts < fromMs) return false;
      if (toMs != null && ts > toMs) return false;
      return true;
    });

    // If filtering unexpectedly yields nothing while data exists, show raw rows
    // so chart is still visible and user can adjust date range.
    if (filtered.length === 0 && fullMetricsRows.length > 0) return fullMetricsRows;
    return filtered;
  }

  function buildHistPoints(rows: any[]) {
    const asc = rows.slice().reverse();
    const out: { ts: string; rx_bps: number; tx_bps: number }[] = [];

    let prevTs: number | null = null;
    let prevRxByte: number | null = null;
    let prevTxByte: number | null = null;

    for (const row of asc) {
      const ts = String(row?.ts || '');
      const tsMs = parseMetricTs(ts);
      if (tsMs == null) continue;

      const directRx = typeof row?.rx_bps === 'number' ? Math.max(0, row.rx_bps) : null;
      const directTx = typeof row?.tx_bps === 'number' ? Math.max(0, row.tx_bps) : null;

      let rx = directRx;
      let tx = directTx;

      const curRxByte = typeof row?.rx_byte === 'number' ? row.rx_byte : null;
      const curTxByte = typeof row?.tx_byte === 'number' ? row.tx_byte : null;

      if (rx == null && curRxByte != null && prevRxByte != null && prevTs != null && tsMs > prevTs) {
        const delta = curRxByte - prevRxByte;
        if (delta >= 0) rx = Math.round((delta * 8 * 1000) / (tsMs - prevTs));
      }
      if (tx == null && curTxByte != null && prevTxByte != null && prevTs != null && tsMs > prevTs) {
        const delta = curTxByte - prevTxByte;
        if (delta >= 0) tx = Math.round((delta * 8 * 1000) / (tsMs - prevTs));
      }

      if (rx != null || tx != null) {
        out.push({ ts, rx_bps: rx ?? 0, tx_bps: tx ?? 0 });
      }

      if (curRxByte != null) prevRxByte = curRxByte;
      if (curTxByte != null) prevTxByte = curTxByte;
      prevTs = tsMs;
    }

    return out;
  }

  function downsampleHistPoints(
    rows: { ts: string; rx_bps: number; tx_bps: number }[],
    maxPoints: number = 120,
  ) {
    if (rows.length <= maxPoints) return rows;
    const step = rows.length / maxPoints;
    const out: { ts: string; rx_bps: number; tx_bps: number }[] = [];

    for (let i = 0; i < maxPoints; i++) {
      const start = Math.floor(i * step);
      const end = Math.max(start + 1, Math.floor((i + 1) * step));
      const chunk = rows.slice(start, end);
      if (!chunk.length) continue;

      const rx = Math.round(chunk.reduce((acc, r) => acc + (r.rx_bps || 0), 0) / chunk.length);
      const tx = Math.round(chunk.reduce((acc, r) => acc + (r.tx_bps || 0), 0) / chunk.length);
      const ts = chunk[chunk.length - 1]?.ts || chunk[0]?.ts || '';
      out.push({ ts, rx_bps: rx, tx_bps: tx });
    }

    return out;
  }

  function applyMetricsZoom(
    rows: { ts: string; rx_bps: number; tx_bps: number }[],
    fromMs: number | null,
    toMs: number | null,
  ) {
    if (fromMs == null || toMs == null) return rows;
    const min = Math.min(fromMs, toMs);
    const max = Math.max(fromMs, toMs);
    const filtered = rows.filter((r) => {
      const ts = parseMetricTs(r.ts);
      return ts != null && ts >= min && ts <= max;
    });
    return filtered.length ? filtered : rows;
  }

  function clearMetricsZoom() {
    metricsZoomFrom = null;
    metricsZoomTo = null;
    metricsPointIdx = null;
  }

  function beginMetricsSelection(e: PointerEvent) {
    if (e.button !== 0) return;
    const el = e.currentTarget as HTMLElement | null;
    const rect = el?.getBoundingClientRect();
    if (!rect || rect.width <= 0) return;
    metricsSelecting = true;
    metricsSelWidth = rect.width;
    metricsSelStart = Math.max(0, Math.min(rect.width, e.clientX - rect.left));
    metricsSelCurrent = metricsSelStart;
    metricsPointIdx = null;
    try {
      el?.setPointerCapture?.(e.pointerId);
    } catch {
      // no-op
    }
  }

  function moveMetricsSelection(e: PointerEvent) {
    if (!metricsSelecting) return;
    const el = e.currentTarget as HTMLElement | null;
    const rect = el?.getBoundingClientRect();
    if (!rect || rect.width <= 0) return;
    metricsSelWidth = rect.width;
    metricsSelCurrent = Math.max(0, Math.min(rect.width, e.clientX - rect.left));
  }

  function endMetricsSelection(
    e: PointerEvent,
    rows: { ts: string; rx_bps: number; tx_bps: number }[],
  ) {
    if (!metricsSelecting) return;
    const el = e.currentTarget as HTMLElement | null;
    try {
      el?.releasePointerCapture?.(e.pointerId);
    } catch {
      // no-op
    }
    metricsSelecting = false;
    const from = Math.min(metricsSelStart, metricsSelCurrent);
    const to = Math.max(metricsSelStart, metricsSelCurrent);
    if (!rows.length || metricsSelWidth <= 0 || to - from < 8) return;

    const len = rows.length;
    const fromIdx = Math.max(0, Math.min(len - 1, Math.floor((from / metricsSelWidth) * (len - 1))));
    const toIdx = Math.max(0, Math.min(len - 1, Math.ceil((to / metricsSelWidth) * (len - 1))));
    const fromTs = parseMetricTs(rows[fromIdx]?.ts);
    const toTs = parseMetricTs(rows[toIdx]?.ts);
    if (fromTs == null || toTs == null) return;

    metricsZoomFrom = Math.min(fromTs, toTs);
    metricsZoomTo = Math.max(fromTs, toTs);
    metricsPointIdx = null;
  }

  function resolveMetricsBucket(
    range: '24h' | '7d' | '30d' | 'month' | 'custom',
    fromLocal: string,
    toLocal: string,
  ): 'raw' | 'hour' | 'day' {
    if (range === '24h') return 'raw';
    if (range === '7d') return 'hour';
    if (range === '30d' || range === 'month') return 'day';

    const fromMs = parseLocalDate(fromLocal);
    const toMs = parseLocalDate(toLocal);
    if (fromMs != null && toMs != null && toMs > fromMs) {
      const days = (toMs - fromMs) / (24 * 60 * 60 * 1000);
      if (days <= 2) return 'raw';
      if (days <= 14) return 'hour';
      return 'day';
    }
    return 'hour';
  }

  function aggregateHistPoints(
    rows: { ts: string; rx_bps: number; tx_bps: number }[],
    bucket: 'raw' | 'hour' | 'day',
  ) {
    if (bucket === 'raw') return rows;

    const byKey = new Map<string, { ts: string; rxSum: number; txSum: number; count: number }>();
    for (const row of rows) {
      const ms = parseMetricTs(row.ts);
      if (ms == null) continue;
      const d = new Date(ms);
      let key = '';
      if (bucket === 'hour') {
        key = `${d.getUTCFullYear()}-${d.getUTCMonth()}-${d.getUTCDate()}-${d.getUTCHours()}`;
      } else {
        key = `${d.getUTCFullYear()}-${d.getUTCMonth()}-${d.getUTCDate()}`;
      }

      const cur = byKey.get(key);
      if (!cur) {
        byKey.set(key, {
          ts: row.ts,
          rxSum: row.rx_bps,
          txSum: row.tx_bps,
          count: 1,
        });
      } else {
        cur.rxSum += row.rx_bps;
        cur.txSum += row.tx_bps;
        cur.count += 1;
        cur.ts = row.ts;
      }
    }

    return Array.from(byKey.values()).map((x) => ({
      ts: x.ts,
      rx_bps: Math.round(x.rxSum / Math.max(1, x.count)),
      tx_bps: Math.round(x.txSum / Math.max(1, x.count)),
    }));
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
    fullTab = tab;
    metricsPointIdx = null;
    metricsSelecting = false;
    if (tab === 'metrics') void refreshMetricsForCurrentRange();
  }

  function setMetricsHoverFromMouse(i: number, e: MouseEvent) {
    metricsPointIdx = i;
    metricsTooltipX = e.clientX + 14;
    metricsTooltipY = e.clientY + 14;
  }

  function setMetricsHoverFromFocus(i: number, e: FocusEvent) {
    metricsPointIdx = i;
    const el = e.currentTarget as HTMLElement | null;
    const r = el?.getBoundingClientRect();
    if (!r) return;
    metricsTooltipX = r.left + r.width / 2 + 10;
    metricsTooltipY = r.top + 10;
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

    try {
      const rows = (await api.mikrotik.routers.interfaceMetrics(s.routerId, {
        interface: s.iface,
        limit: minLimit,
      })) as any[];
      if (fullMetricsKey !== key) return;
      fullMetricsRows = Array.isArray(rows) ? rows : [];
      fullMetricsLimit = minLimit;
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
    try {
      localStorage.setItem('mikrotik_wallboard_layout', layout);
      localStorage.setItem('mikrotik_wallboard_slots', JSON.stringify(slotsAll));
      localStorage.setItem(ROTATE_MODE_KEY, rotateMode);
      localStorage.setItem(ROTATE_MS_KEY, String(rotateMs));
      localStorage.setItem(FOCUS_MODE_KEY, focusMode ? '1' : '0');
      localStorage.setItem(STATUS_FILTER_KEY, statusFilter);
      localStorage.setItem(POLL_MS_KEY, String(pollMs));
      localStorage.setItem(ALERT_SOUND_KEY, criticalSoundEnabled ? '1' : '0');
      localStorage.setItem(KEEP_AWAKE_KEY, '1');
    } catch {
      // ignore
    }
  }

  function loadConfig() {
    try {
      const l = localStorage.getItem('mikrotik_wallboard_layout') as LayoutPreset | null;
      if (isLayoutPreset(l)) layout = l;
      const rm = localStorage.getItem(ROTATE_MODE_KEY);
      if (rm === 'manual' || rm === 'auto') rotateMode = rm;
      const rms = Number(localStorage.getItem(ROTATE_MS_KEY) || 10000);
      if ((WALLBOARD_ROTATE_MS_OPTIONS as readonly number[]).includes(rms)) rotateMs = rms;
      const sf = localStorage.getItem(STATUS_FILTER_KEY);
      if (sf === 'all' || sf === 'online' || sf === 'offline') statusFilter = sf;
      const pm = Number(localStorage.getItem(POLL_MS_KEY) || 1000);
      if ((WALLBOARD_POLL_MS_OPTIONS as readonly number[]).includes(pm)) pollMs = pm;
      const sd = localStorage.getItem(ALERT_SOUND_KEY);
      if (sd === '0' || sd === '1') criticalSoundEnabled = sd === '1';
      const s = localStorage.getItem('mikrotik_wallboard_slots');
      if (s) {
        const parsed = JSON.parse(s);
        if (Array.isArray(parsed)) {
          slotsAll = parsed.map((it) => {
            if (!it) return null;
            // Back-compat: old format was just routerId strings.
            if (typeof it === 'string')
              return { routerId: it, iface: 'ether1', warn_below_rx_bps: null, warn_below_tx_bps: null };
            if (typeof it === 'object' && typeof it.routerId === 'string' && typeof it.iface === 'string') {
              return {
                routerId: it.routerId,
                iface: it.iface,
                warn_below_rx_bps: typeof it.warn_below_rx_bps === 'number' ? it.warn_below_rx_bps : null,
                warn_below_tx_bps: typeof it.warn_below_tx_bps === 'number' ? it.warn_below_tx_bps : null,
              };
            }
            return null;
          });
        }
      }
    } catch {
      // ignore
    }
  }

  async function loadRemoteConfig() {
    try {
      const [remoteLayout, remoteSlots] = await Promise.all([
        api.settings.getValue(SETTINGS_LAYOUT_KEY),
        api.settings.getValue(SETTINGS_SLOTS_KEY),
      ]);

      if (isLayoutPreset(remoteLayout)) {
        layout = remoteLayout;
      }

      if (remoteSlots) {
        const parsed = JSON.parse(remoteSlots);
        if (Array.isArray(parsed)) {
          slotsAll = parsed.map((it) => {
            if (!it) return null;
            // Back-compat: old format was just routerId strings.
            if (typeof it === 'string') return { routerId: it, iface: 'ether1', warn_below_rx_bps: null, warn_below_tx_bps: null };
            if (typeof it === 'object' && typeof it.routerId === 'string' && typeof it.iface === 'string') {
              return {
                routerId: it.routerId,
                iface: it.iface,
                warn_below_rx_bps: typeof it.warn_below_rx_bps === 'number' ? it.warn_below_rx_bps : null,
                warn_below_tx_bps: typeof it.warn_below_tx_bps === 'number' ? it.warn_below_tx_bps : null,
              };
            }
            return null;
          });
        }
      }
      remoteLoaded = true;
    } catch {
      // ignore (wallboard should always load)
      remoteLoaded = true;
    }
  }

  function schedulePersistRemote() {
    if (!remoteLoaded) return;
    if (persistTimer) clearTimeout(persistTimer);
    persistTimer = setTimeout(() => void persistRemoteNow(), 700);
  }

  async function persistRemoteNow() {
    if (!remoteLoaded) return;
    const payload = JSON.stringify({ layout, slots: slotsAll });
    if (payload === lastRemotePayload) return;
    lastRemotePayload = payload;

    try {
      await Promise.all([
        api.settings.upsert(SETTINGS_LAYOUT_KEY, layout, 'Wallboard layout preset (tenant scoped)'),
        api.settings.upsert(SETTINGS_SLOTS_KEY, JSON.stringify(slotsAll), 'Wallboard interface tiles (tenant scoped)'),
      ]);
    } catch {
      // ignore: avoid spamming toasts on background saves
    }
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
    await Promise.all([loadAlerts(true), loadIncidents(true)]);
  }

  function filterRows(list: NocRow[]) {
    return list.filter((r) => {
      if (statusFilter === 'online' && !r.is_online) return false;
      if (statusFilter === 'offline' && r.is_online) return false;
      return true;
    });
  }

  async function pollLiveOnce() {
    // Avoid burning resources if user isn't looking at the tab.
    if (typeof document !== 'undefined' && document.hidden) return;
    if (paused) return;

    const wanted = slotsAll.filter(Boolean) as Slot[];
    if (wanted.length === 0) return;

    const byRouter = new Map<string, Set<string>>();
    for (const s of wanted) {
      if (!s.routerId || !s.iface) continue;
      let set = byRouter.get(s.routerId);
      if (!set) {
        set = new Set<string>();
        byRouter.set(s.routerId, set);
      }
      set.add(s.iface);
    }

    // Guardrail: keep router API load predictable.
    const routerIds = Array.from(byRouter.keys()).slice(0, 12);

    // sequential loop = keeps router API load predictable
    for (const routerId of routerIds) {
      const names = Array.from(byRouter.get(routerId) || []).filter(Boolean).slice(0, 12);
      if (!names.length) continue;
      const now = Date.now();
      const ps = routerPollState[routerId];
      if (ps && ps.nextRetryAt > now) continue;

      try {
        const counters = (await api.mikrotik.routers.interfaceLive(routerId, names)) as any as LiveCounter[];
        const now = Date.now();
        liveRates[routerId] ||= {};
        series[routerId] ||= {};

        for (const c of counters) {
          const key = `${routerId}:${c.name}`;
          const prev = lastBytes.get(key);
          const rx = c.rx_byte ?? 0;
          const tx = c.tx_byte ?? 0;

          let rxBps: number | null = null;
          let txBps: number | null = null;
          if (prev && now > prev.at) {
            const dt = (now - prev.at) / 1000;
            rxBps = Math.max(0, Math.round((rx - prev.rx) / dt) * 8);
            txBps = Math.max(0, Math.round((tx - prev.tx) / dt) * 8);
          }

          lastBytes.set(key, { rx, tx, at: now });
          liveRates[routerId][c.name] = { rx_bps: rxBps, tx_bps: txBps, last_seen_at: now };

          if (!series[routerId][c.name]) series[routerId][c.name] = { rx: [], tx: [] };
          const buf = series[routerId][c.name];
          buf.rx.push(rxBps ?? 0);
          buf.tx.push(txBps ?? 0);
          if (buf.rx.length > 60) buf.rx.splice(0, buf.rx.length - 60);
          if (buf.tx.length > 60) buf.tx.splice(0, buf.tx.length - 60);
        }
        if (ps && ps.fails >= 3) {
          pushIncident('recovered', 'Polling recovered', routerId);
        }
        if (ps && ps.fails > 0) {
          routerPollState = {
            ...routerPollState,
            [routerId]: {
              fails: 0,
              nextRetryAt: 0,
              lastErrorAt: ps.lastErrorAt,
              lastSuccessAt: now,
            },
          };
        }
      } catch {
        const prev = routerPollState[routerId] || {
          fails: 0,
          nextRetryAt: 0,
          lastErrorAt: null,
          lastSuccessAt: null,
        };
        const fails = prev.fails + 1;
        const backoffMs = Math.min(30_000, 1000 * 2 ** Math.min(fails, 5));
        const nextRetryAt = Date.now() + backoffMs;
        routerPollState = {
          ...routerPollState,
          [routerId]: {
            fails,
            nextRetryAt,
            lastErrorAt: Date.now(),
            lastSuccessAt: prev.lastSuccessAt,
          },
        };
        if (fails === 3 || fails === 5 || fails % 10 === 0) {
          pushIncident('poll_error', `Polling failed (${fails}x)`, routerId);
        }
      }
    }

    renderNow = Date.now();
  }

  function setPaused(on: boolean) {
    paused = on;
    if (paused) {
      if (tick) clearInterval(tick);
      tick = null;
    } else {
      if (!tick) tick = setInterval(() => void pollLiveOnce(), pollMs);
    }
  }

  function swapSlots(from: number, to: number) {
    if (from === to) return;
    const next = [...slotsAll];
    const need = Math.max(from, to);
    if (next.length <= need) next.push(...Array.from({ length: need + 1 - next.length }, () => null));
    const a = next[from] ?? null;
    const b = next[to] ?? null;
    next[from] = b;
    next[to] = a;
    slotsAll = next;
  }

  function getHoverIndexFromPoint(x: number, y: number) {
    const target = document.elementFromPoint(x, y) as HTMLElement | null;
    const bar = target?.closest?.('.bar') as HTMLElement | null;
    if (!bar) return null;
    const raw = bar.dataset?.idx;
    if (!raw) return null;
    const idx = Number.parseInt(raw, 10);
    return Number.isFinite(idx) && idx >= 0 ? idx : null;
  }

  function getSlotIndexFromPoint(x: number, y: number) {
    const el = document.elementFromPoint(x, y) as HTMLElement | null;
    const host = el?.closest?.('[data-wall-slot]') as HTMLElement | null;
    const raw = host?.dataset?.wallSlot;
    if (!raw) return null;
    const idx = Number.parseInt(raw, 10);
    return Number.isFinite(idx) && idx >= 0 ? idx : null;
  }

  function endDrag(apply: boolean) {
    if (apply && dragFrom != null && dragOver != null) swapSlots(dragFrom, dragOver);
    dragFrom = null;
    dragOver = null;
    dragging = false;
    if (typeof document !== 'undefined') document.body.classList.remove('wall-dragging');
    window.removeEventListener('pointermove', onDragMove as any);
    window.removeEventListener('pointerup', onDragUp as any);
    window.removeEventListener('pointercancel', onDragCancel as any);
  }

  function onDragMove(e: PointerEvent) {
    if (!dragging) return;
    const idx = getSlotIndexFromPoint(e.clientX, e.clientY);
    if (idx != null) dragOver = idx;
  }

  function onDragUp() {
    endDrag(true);
  }

  function onDragCancel() {
    endDrag(false);
  }

  function startDrag(e: PointerEvent, idx: number) {
    e.preventDefault();
    e.stopPropagation();
    dragging = true;
    dragFrom = idx;
    dragOver = idx;
    if (typeof document !== 'undefined') document.body.classList.add('wall-dragging');
    window.addEventListener('pointermove', onDragMove as any);
    window.addEventListener('pointerup', onDragUp as any);
    window.addEventListener('pointercancel', onDragCancel as any);
  }

  function startDragFromTile(e: PointerEvent, idx: number) {
    const target = e.target as HTMLElement | null;
    if (!target) return;
    if (target.closest('button, a, input, select, textarea, [role="menu"], .tile-menu')) return;
    startDrag(e, idx);
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

    tick = setInterval(() => void pollLiveOnce(), pollMs);
    alertTick = setInterval(() => {
      void Promise.all([loadAlerts(true), loadIncidents(true)]);
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
    if (persistTimer) clearTimeout(persistTimer);
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
    // restart polling when interval changes or pause changes
    if (paused) {
      if (tick) clearInterval(tick);
      tick = null;
      return;
    }
    if (tick) clearInterval(tick);
    tick = setInterval(() => void pollLiveOnce(), pollMs);
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

    ensureSlots();

    const size = slotCountForLayout(layout);
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
        bind:page
        {pageCount}
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
          {@const iface = slot.iface}
          {@const rx = series[slot.routerId]?.[iface]?.rx ?? []}
          {@const tx = series[slot.routerId]?.[iface]?.tx ?? []}
          {@const max = Math.max(1, ...rx, ...tx)}
          {@const rxPeak = peakBps(rx)}
          {@const txPeak = peakBps(tx)}
          {@const rxAvg = avgBps(rx)}
          {@const txAvg = avgBps(tx)}
          {@const rxNow = liveRates[slot.routerId]?.[iface]?.rx_bps ?? null}
          {@const txNow = liveRates[slot.routerId]?.[iface]?.tx_bps ?? null}
          {@const lastSeenAt = liveRates[slot.routerId]?.[iface]?.last_seen_at ?? null}
          {@const stale =
            !paused &&
            lastSeenAt != null &&
            Number.isFinite(lastSeenAt) &&
            renderNow - (lastSeenAt as number) > Math.max(10_000, pollMs * 3)}
          {@const warnRx =
            slot.warn_below_rx_bps != null &&
            rxNow != null &&
            rxNow >= 0 &&
            rxNow < slot.warn_below_rx_bps}
          {@const warnTx =
            slot.warn_below_tx_bps != null &&
            txNow != null &&
            txNow >= 0 &&
            txNow < slot.warn_below_tx_bps}
          {@const maintLeft = maintenanceRemaining(r?.maintenance_until)}
          {@const pollFails = routerPollState[slot.routerId]?.fails ?? 0}
          {@const pollRetryAt = routerPollState[slot.routerId]?.nextRetryAt ?? 0}
          {@const pollRetrySec = pollRetryAt > Date.now() ? Math.ceil((pollRetryAt - Date.now()) / 1000) : 0}
          {@const pollDegraded = pollFails >= 3}
          {@const rxTrend = calcTrend(rx)}
          {@const txTrend = calcTrend(tx)}
          {@const ra = routerAlertMap[slot.routerId]}
          {@const tileKey = `${slot.routerId}:${iface}:${gidx}`}
          {@const hoverIdx =
            hoverBar && hoverBar.tileKey === tileKey
              ? Math.min(rx.length ? rx.length - 1 : 0, Math.max(0, hoverBar.idx))
              : null}
          {@const hoverRx = hoverIdx != null ? rx[hoverIdx] ?? null : null}
          {@const hoverTx = hoverIdx != null ? tx[hoverIdx] ?? null : null}
          <div
            class="tile iface-tile"
            class:warn={warnRx || warnTx}
            class:drag-over={dragOver === gidx}
            data-wall-slot={gidx}
            role="button"
            tabindex="0"
            onpointerdown={(e) => startDragFromTile(e, gidx)}
            ondblclick={() => openFull(gidx)}
            onkeydown={(e) => e.key === 'Enter' && openFull(gidx)}
          >
            <div class="tile-head">
              <div class="left">
                <div class="name">
                  <span class="mono">{iface}</span>
                </div>
                <div class="meta">
                  <span class="mono">{r ? (r.identity || r.name) : slot.routerId}</span>
                </div>
              </div>

      <div class="right">
        {#if ra?.total}
          <button
            class="icon-x attn"
            type="button"
            onclick={(e) => {
              e.stopPropagation();
              goto(`${tenantPrefix}/admin/network/alerts`);
            }}
            title={`${ra.total} ${$t('admin.network.wallboard.alerts_open') || 'open alerts'}`}
          >
            <Icon name="alert-triangle" size={16} />
            <span class="attn-count">{ra.total}</span>
          </button>
          {#if $can('manage', 'network_routers')}
            <button
              class="icon-x"
              type="button"
              onclick={(e) => {
                e.stopPropagation();
                void ackRouterAlerts(slot.routerId);
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
              tileMenuIndex = tileMenuIndex === gidx ? null : gidx;
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
                  tileMenuIndex = null;
                  openFull(gidx);
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
                  tileMenuIndex = null;
                  openThreshold(gidx);
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
                  tileMenuIndex = null;
                  clearSlot(gidx);
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
                  <span
                    class="badge maintenance"
                    title={($t('admin.network.wallboard.maintenance') || 'Maintenance') + ` ${maintLeft}`}
                  >
                    <Icon name="clock" size={13} />
                    {$t('admin.network.wallboard.maintenance') || 'Maintenance'} {maintLeft}
                  </span>
                {/if}
                {#if pollDegraded}
                  <span
                    class="badge poll-err"
                    title={`${$t('admin.network.wallboard.poll_error') || 'Poll error'} (${pollFails}x)`}
                  >
                    <Icon name="wifi-off" size={13} />
                    {($t('admin.network.wallboard.poll_error') || 'Poll error') + ` ${pollFails}x`}
                    {#if pollRetrySec > 0}
                      <span class="mono">({pollRetrySec}s)</span>
                    {/if}
                  </span>
                {/if}
                <span
                  class="badge status-dot"
                  class:ok={r?.is_online}
                  class:bad={!r?.is_online}
                  title={r?.is_online
                    ? $t('admin.network.routers.badges.online') || 'Online'
                    : $t('admin.network.routers.badges.offline') || 'Offline'}
                  aria-label={r?.is_online
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
                    <div
                      class="bar rx"
                      class:active={hoverIdx === i}
                      style={`height:${Math.round((v / max) * 100)}%;`}
                      data-idx={i}
                    ></div>
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
                    <div
                      class="bar tx"
                      class:active={hoverIdx === i}
                      style={`height:${Math.round((v / max) * 100)}%;`}
                      data-idx={i}
                    ></div>
                  {/each}
                </div>

                <div
                  class="spark-hover"
                  role="presentation"
                  aria-hidden="true"
                  onpointermove={(e) => {
                    const idx = getHoverIndexFromPoint(e.clientX, e.clientY);
                    if (idx == null) return;
                    hoverBar = { tileKey, idx };
                  }}
                  onpointerleave={() => {
                    if (hoverBar?.tileKey === tileKey) hoverBar = null;
                  }}
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
  {@const isEditing = !!slotsAll[pickerIndex]}
  {@const curSlot = slotsAll[pickerIndex]}
  <div class="picker-overlay" role="dialog" aria-modal="true">
    <button class="picker-backdrop" type="button" onclick={closePicker} aria-label={$t('common.close') || 'Close'}></button>
    <div class="picker">
      <div class="picker-head">
        <h3>
          {isEditing
            ? ($t('admin.network.wallboard.edit_tile') || 'Edit interface tile')
            : ($t('admin.network.wallboard.pick_tile') || 'Add interface tile')}
        </h3>
        <button class="icon-x" type="button" onclick={closePicker} title={$t('common.close') || 'Close'}>
          <Icon name="x" size={18} />
        </button>
      </div>
      <div class="picker-summary">
        <span class="picker-chip">
          <span class="k">Router</span>
          <span class="v mono">{pickerRouterId ? (routerById(pickerRouterId)?.identity || routerById(pickerRouterId)?.name || pickerRouterId) : '—'}</span>
        </span>
        <span class="picker-chip">
          <span class="k">Interface</span>
          <span class="v mono">{curSlot?.iface || '—'}</span>
        </span>
      </div>
      {#if true}
        {@const q = pickerRouterSearch.trim().toLowerCase()}
        {@const routerList = filterRows(rows).filter((r) => {
          if (!q) return true;
          const hay = `${r.name} ${r.identity || ''} ${r.host}`.toLowerCase();
          return hay.includes(q);
        })}
        <div class="picker-body">
          <div class="picker-col">
            <div class="col-head">
              <span class="col-title">{$t('admin.network.wallboard.pick_router') || 'Router'}</span>
              <span class="muted">{routerList.length}</span>
            </div>
            <div class="pill small">
              <Icon name="search" size={16} />
              <input
                value={pickerRouterSearch}
                oninput={(e) => (pickerRouterSearch = (e.currentTarget as HTMLInputElement).value)}
                placeholder={$t('admin.network.wallboard.pick_search') || 'Search routers...'}
              />
            </div>
            <div class="picker-list">
              {#if routerList.length === 0}
                <div class="panel-empty muted">
                  <Icon name="search" size={16} />
                  {$t('admin.network.wallboard.empty') || 'No routers match your filters.'}
                </div>
              {:else}
                {#each routerList as r (r.id)}
                  <button
                    class="pick"
                    class:active={pickerRouterId === r.id}
                    type="button"
                    onclick={() => {
                      pickerRouterId = r.id;
                      void loadInterfaces(r.id);
                      pickerIfaceSearch = '';
                    }}
                  >
                    <span class="name">{routerTitle(r)}</span>
                    <span class="spacer"></span>
                    <span class="badge" class:ok={r.is_online} class:bad={!r.is_online}>
                      <span class="dot"></span>
                      {r.is_online
                        ? $t('admin.network.routers.badges.online') || 'Online'
                        : $t('admin.network.routers.badges.offline') || 'Offline'}
                    </span>
                  </button>
                {/each}
              {/if}
            </div>
          </div>

          <div class="picker-col">
            <div class="col-head">
              <span class="col-title">{$t('admin.network.wallboard.pick_interface') || 'Interface'}</span>
              {#if pickerRouterId}
                <span class="muted">{ifaceCatalog[pickerRouterId]?.length || 0}</span>
              {/if}
            </div>

            {#if !pickerRouterId}
              <div class="panel-empty muted">
                <Icon name="info" size={16} />
                {$t('admin.network.wallboard.pick_interface_hint') || 'Select a router first.'}
              </div>
            {:else}
              <div class="pill small">
                <Icon name="search" size={16} />
                <input
                  value={pickerIfaceSearch}
                  oninput={(e) => (pickerIfaceSearch = (e.currentTarget as HTMLInputElement).value)}
                  placeholder={$t('admin.network.wallboard.pick_interface_search') || 'Search interfaces...'}
                />
              </div>

              {#if ifaceLoading[pickerRouterId]}
                <div class="panel-empty muted">
                  <Icon name="loader" size={16} />
                  {$t('admin.network.wallboard.watch_loading') || 'Loading interfaces...'}
                </div>
              {:else}
                {@const iq = pickerIfaceSearch.trim().toLowerCase()}
                {@const ifaces = (ifaceCatalog[pickerRouterId] || []).filter((i) => {
                  if (!iq) return true;
                  return (
                    i.name.toLowerCase().includes(iq) ||
                    (i.interface_type || '').toLowerCase().includes(iq)
                  );
                })}
                <div class="picker-list">
                  {#if ifaces.length === 0}
                    <div class="panel-empty muted">
                      <Icon name="search" size={16} />
                      {$t('admin.network.wallboard.watch_none') || 'No interfaces.'}
                    </div>
                  {:else}
                    {#each ifaces as it (it.name)}
                      <button
                        class="pick"
                        type="button"
                        onclick={() => {
                          const cur = slotsAll[pickerIndex as number];
                          const rx = cur?.warn_below_rx_bps ?? null;
                          const tx = cur?.warn_below_tx_bps ?? null;
                          setSlot(
                            pickerIndex as number,
                            pickerRouterId as string,
                            it.name,
                            rx,
                            tx,
                          );
                        }}
                      >
                        <span class="name mono">{it.name}</span>
                        <span class="muted">{it.interface_type || ''}</span>
                        <span class="spacer"></span>
                        {#if it.disabled}
                          <span class="tag">{$t('admin.network.wallboard.interface_state.disabled') || 'disabled'}</span>
                        {:else if it.running}
                          <span class="tag ok">{$t('admin.network.wallboard.interface_state.up') || 'up'}</span>
                        {:else}
                          <span class="tag">{$t('admin.network.wallboard.interface_state.down') || 'down'}</span>
                        {/if}
                      </button>
                    {/each}
                  {/if}
                </div>
              {/if}
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>
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
  <div class="full-overlay" role="dialog" aria-modal="true">
    <button class="full-backdrop" type="button" onclick={closeFull} aria-label={$t('common.close') || 'Close'}></button>
    <div class="full">
      <div class="full-head">
        <div class="full-titles">
          <div class="full-kicker">
            {$t('admin.network.wallboard.full_kicker') || 'INTERFACE VIEW'}
          </div>
          <div class="full-title">
            <span class="mono">{iface}</span>
            <span class="muted">·</span>
            <span>{r ? (r.identity || r.name) : s?.routerId}</span>
          </div>
        </div>
        <div class="full-actions">
          <button
            class="btn-mini"
            type="button"
            onclick={(e) => {
              e.stopPropagation();
              openThreshold(fullIndex as number);
            }}
          >
            <Icon name="edit" size={16} />
            {$t('common.edit') || 'Edit'}
          </button>
          <button class="icon-x" type="button" onclick={closeFull} title={$t('common.close') || 'Close'}>
            <Icon name="x" size={18} />
          </button>
        </div>
      </div>

      <div class="full-body">
        <div class="full-summary-sticky">
          <div class="full-summary-grid">
            <div class="full-summary-item">
              <span class="k">{$t('admin.network.wallboard.summary.status') || 'Status'}</span>
              <span class="v mono">{r?.is_online
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
          <button
            class="full-tab {fullTab === 'live' ? 'active' : ''}"
            type="button"
            onclick={() => openFullTab('live')}
          >
            {$t('admin.network.wallboard.tabs.live') || 'Live'}
          </button>
          <button
            class="full-tab {fullTab === 'metrics' ? 'active' : ''}"
            type="button"
            onclick={() => openFullTab('metrics')}
          >
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
                  onchange={(e) =>
                    setMetricsRange((e.currentTarget as HTMLSelectElement).value as typeof metricsRange)}
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
              <button
                class="btn-mini"
                type="button"
                onclick={() =>
                  exportMetricsCsv(
                    zoomedHistRows,
                    iface,
                    r ? (r.identity || r.name) : s?.routerId || '',
                    metricsBucket,
                  )}
              >
                <Icon name="download" size={16} />
                {$t('admin.network.wallboard.metrics.export_csv') || 'Export CSV'}
              </button>
              {#if hasMetricsZoom}
                <button class="btn-mini" type="button" onclick={() => clearMetricsZoom()}>
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
                    oninput={(e) => {
                      metricsFromLocal = (e.currentTarget as HTMLInputElement).value;
                      metricsPointIdx = null;
                      clearMetricsZoom();
                      void refreshMetricsForCurrentRange();
                    }}
                  />
                </label>
                <label>
                  <span class="muted">{$t('common.to') || 'To'}</span>
                  <input
                    type="datetime-local"
                    value={metricsToLocal}
                    oninput={(e) => {
                      metricsToLocal = (e.currentTarget as HTMLInputElement).value;
                      metricsPointIdx = null;
                      clearMetricsZoom();
                      void refreshMetricsForCurrentRange();
                    }}
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
                onpointerdown={beginMetricsSelection}
                onpointermove={moveMetricsSelection}
                onpointerup={(e) => endMetricsSelection(e, chartRows)}
                onpointercancel={(e) => endMetricsSelection(e, chartRows)}
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
                      onmouseenter={(e) => setMetricsHoverFromMouse(i, e)}
                      onmousemove={(e) => setMetricsHoverFromMouse(i, e)}
                      onmouseleave={() => (metricsPointIdx = null)}
                      onfocus={(e) => setMetricsHoverFromFocus(i, e)}
                      onblur={() => (metricsPointIdx = null)}
                      onkeydown={(e) =>
                        (e.key === 'Enter' || e.key === ' ') && (metricsPointIdx = i)}
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
                      onmouseenter={(e) => setMetricsHoverFromMouse(i, e)}
                      onmousemove={(e) => setMetricsHoverFromMouse(i, e)}
                      onmouseleave={() => (metricsPointIdx = null)}
                      onfocus={(e) => setMetricsHoverFromFocus(i, e)}
                      onblur={() => (metricsPointIdx = null)}
                      onkeydown={(e) =>
                        (e.key === 'Enter' || e.key === ' ') && (metricsPointIdx = i)}
                    ></div>
                  {/each}
                </div>
              </div>
              {#if pointRow}
                <div
                  class="metrics-tooltip floating"
                  style={`left:${metricsTooltipX}px; top:${metricsTooltipY}px;`}
                >
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
{/if}

{#if thresholdIndex !== null}
  {@const s = slotsAll[thresholdIndex]}
  {@const r = s ? routerById(s.routerId) : null}
  <div class="threshold-overlay" role="dialog" aria-modal="true">
    <button
      class="threshold-backdrop"
      type="button"
      onclick={closeThreshold}
      aria-label={$t('common.close') || 'Close'}
    ></button>
    <div class="threshold">
      <div class="threshold-head">
        <div>
          <div class="full-kicker">{$t('admin.network.wallboard.thresholds.title') || 'Thresholds'}</div>
          <div class="full-title">
            <span class="mono">{s?.iface || ''}</span>
            <span class="muted">·</span>
            <span>{r ? (r.identity || r.name) : s?.routerId}</span>
          </div>
        </div>
        <div class="full-actions">
          <button
            class="btn-mini"
            type="button"
            onclick={() => {
              // Optional: change router/interface (uses picker)
              const idx = thresholdIndex;
              closeThreshold();
              if (idx != null) setTimeout(() => openPicker(idx), 0);
            }}
          >
            <Icon name="settings" size={16} />
            {$t('admin.network.wallboard.thresholds.change_interface') || 'Change interface'}
          </button>
          <button class="icon-x" type="button" onclick={closeThreshold} title={$t('common.close') || 'Close'}>
            <Icon name="x" size={18} />
          </button>
        </div>
      </div>

      <div class="threshold-summary">
        <div class="threshold-chip">
          <span class="k">{$t('admin.network.wallboard.thresholds.current_rx') || 'Current RX threshold'}</span>
          <span class="v mono">{s?.warn_below_rx_bps != null
            ? formatBps(s.warn_below_rx_bps)
            : $t('admin.network.wallboard.thresholds.not_set') || 'Not set'}</span>
        </div>
        <div class="threshold-chip">
          <span class="k">{$t('admin.network.wallboard.thresholds.current_tx') || 'Current TX threshold'}</span>
          <span class="v mono">{s?.warn_below_tx_bps != null
            ? formatBps(s.warn_below_tx_bps)
            : $t('admin.network.wallboard.thresholds.not_set') || 'Not set'}</span>
        </div>
      </div>

      <div class="tile-settings">
        <div class="settings-grid">
          <label class="field">
            <span class="k">{$t('admin.network.wallboard.warn_below_rx') || 'Warn if RX below'}</span>
            <div class="row">
              <input
                inputmode="numeric"
                value={thWarnRxKbps}
                oninput={(e) => (thWarnRxKbps = (e.currentTarget as HTMLInputElement).value)}
                placeholder="0"
              />
              <select class="unit-select" value={thWarnRxUnit} onchange={(e) => (thWarnRxUnit = (e.currentTarget as HTMLSelectElement).value as typeof thWarnRxUnit)}>
                <option value="Kbps">Kbps</option>
                <option value="Mbps">Mbps</option>
                <option value="Gbps">Gbps</option>
              </select>
            </div>
            <span class="hint">{$t('admin.network.wallboard.thresholds.hint') || 'Leave empty to disable warning.'}</span>
          </label>
          <label class="field">
            <span class="k">{$t('admin.network.wallboard.warn_below_tx') || 'Warn if TX below'}</span>
            <div class="row">
              <input
                inputmode="numeric"
                value={thWarnTxKbps}
                oninput={(e) => (thWarnTxKbps = (e.currentTarget as HTMLInputElement).value)}
                placeholder="0"
              />
              <select class="unit-select" value={thWarnTxUnit} onchange={(e) => (thWarnTxUnit = (e.currentTarget as HTMLSelectElement).value as typeof thWarnTxUnit)}>
                <option value="Kbps">Kbps</option>
                <option value="Mbps">Mbps</option>
                <option value="Gbps">Gbps</option>
              </select>
            </div>
            <span class="hint">{$t('admin.network.wallboard.thresholds.hint') || 'Leave empty to disable warning.'}</span>
          </label>
        </div>

        <div class="settings-actions">
          <button
            class="btn-mini ghost"
            type="button"
            onclick={() => {
              thWarnRxKbps = '';
              thWarnTxKbps = '';
              thWarnRxUnit = 'Kbps';
              thWarnTxUnit = 'Kbps';
            }}
          >
            {$t('common.clear') || 'Clear'}
          </button>
          <button class="btn-mini primary" type="button" onclick={saveThreshold}>
            <Icon name="save" size={16} />
            {$t('common.save') || 'Save'}
          </button>
        </div>
      </div>
    </div>
  </div>
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
  .wallboard.focus .spark.wide {
    height: 108px;
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
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: radial-gradient(circle at 30% 30%, #7cdbff, #6b6bff);
    box-shadow: 0 0 0 3px color-mix(in srgb, #6b6bff 25%, transparent);
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

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
    min-width: 260px;
  }
  .pill.small {
    min-width: 0;
    padding: 8px 10px;
    border-radius: 12px;
  }
  .pill input {
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    width: 100%;
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
  .muted {
    color: var(--text-muted);
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
  .icon-x.attn {
    border-color: color-mix(in srgb, var(--color-warning) 45%, var(--border-color));
    color: color-mix(in srgb, var(--color-warning) 80%, var(--text-primary));
    gap: 6px;
    padding-inline: 8px;
    min-width: 42px;
  }
  .attn-count {
    font-size: 11px;
    font-weight: 900;
    line-height: 1;
  }
  .icon-x:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border-color));
    background: color-mix(in srgb, var(--accent) 10%, var(--bg-surface));
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
  .btn-mini:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border-color));
    background: color-mix(in srgb, var(--accent) 10%, var(--bg-surface));
  }
  .btn-mini:active:not(:disabled) {
    transform: translateY(1px);
  }
  .btn-mini:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .btn-mini.primary {
    border-color: color-mix(in srgb, var(--accent) 65%, var(--border-color));
    background: color-mix(in srgb, var(--accent) 22%, var(--bg-surface));
  }
  .btn-mini.primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 28%, var(--bg-surface));
  }
  .btn-mini.ghost {
    background: transparent;
  }
  .right {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .tile-actions {
    position: relative;
  }
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
  .tile-menu button:hover {
    background: color-mix(in srgb, var(--bg-surface) 60%, var(--accent) 10%);
  }
  .tile-menu button.danger {
    color: color-mix(in srgb, var(--color-danger) 82%, var(--text-primary));
  }

  .tile-head {
    padding: 14px 14px 10px;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
    border-bottom: 1px solid var(--border-color);
  }

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
  .name .mono {
    display: block;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    margin-top: 4px;
    color: var(--text-muted);
    font-size: 12px;
  }

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
  .badge .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    box-shadow: none;
  }
  .badge.ok .dot {
    background: #2ecc71;
  }
  .badge.bad .dot {
    background: #ff6b6b;
  }
  .badge.status-dot {
    padding: 6px;
    min-width: 0;
    gap: 0;
  }
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

  .tile-body {
    padding: 14px;
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
  .spark-sep {
    color: var(--text-muted);
  }
  .spark-hover {
    position: absolute;
    inset: 0;
    z-index: 4;
  }
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
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
  }
  .spark-rate {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
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
  .trend-chip.flat {
    color: var(--text-muted);
  }

  .rate.warn {
    color: var(--color-danger);
    font-weight: 950;
  }

  .panel-empty {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px;
    border-radius: 14px;
    border: 1px dashed var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 45%, transparent);
  }
  .spacer {
    flex: 1;
  }
  .tag {
    padding: 3px 8px;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .tag.ok {
    border-color: color-mix(in srgb, #22c55e 45%, var(--border-color));
    color: #22c55e;
  }

  .spark {
    margin-top: 10px;
    position: relative;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    height: 46px;
  }
  .spark.wide {
    height: 112px;
  }
  .spark.huge {
    height: min(44dvh, 420px);
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
        to top,
        color-mix(in srgb, var(--border-color) 45%, transparent) 1px,
        transparent 1px
      ) 0 0 / 100% 25%,
      color-mix(in srgb, var(--bg-surface) 45%, transparent);
    overflow: hidden;
  }
  .spark-panel-title {
    position: absolute;
    top: 6px;
    left: 6px;
    right: 6px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    z-index: 2;
    pointer-events: none;
  }
  .spark-crosshair {
    position: absolute;
    top: 26px;
    bottom: 6px;
    left: var(--x, 50%);
    width: 1px;
    transform: translateX(-0.5px);
    background: color-mix(in srgb, var(--text-muted) 50%, transparent);
    opacity: 0.75;
    pointer-events: none;
    z-index: 1;
  }
  .bars.warn {
    border-color: color-mix(in srgb, var(--color-danger) 55%, var(--border-color));
    background: color-mix(in srgb, var(--color-danger) 8%, var(--bg-surface));
  }
  .bar {
    position: relative;
    border-radius: 6px;
    opacity: 0.95;
    transition: filter 120ms ease;
  }
  .bar:hover {
    filter: brightness(1.08);
  }
  .bar.active {
    filter: brightness(1.12);
    outline: 1px solid color-mix(in srgb, var(--accent) 55%, transparent);
    outline-offset: 1px;
  }
  .spark.huge.historical .bar {
    cursor: pointer;
  }
  .spark.huge.historical {
    cursor: crosshair;
  }
  .metrics-selection {
    position: absolute;
    top: 0;
    bottom: 0;
    border-left: 1px solid color-mix(in srgb, var(--accent) 85%, transparent);
    border-right: 1px solid color-mix(in srgb, var(--accent) 85%, transparent);
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    pointer-events: none;
    z-index: 3;
  }
  .bars.warn .bar {
    background: linear-gradient(180deg, #ff8a8a, var(--color-danger));
  }
  .bar.rx {
    background: linear-gradient(180deg, #4fd1ff, #3f6bff);
  }
  .bar.tx {
    background: linear-gradient(180deg, #7bffb2, #22c55e);
  }
  .bar.peak {
    outline: 1px solid color-mix(in srgb, #ffd166 75%, transparent);
    outline-offset: 1px;
    filter: brightness(1.16);
  }
  .bar.peak::after {
    content: '';
    position: absolute;
    top: -3px;
    left: 50%;
    width: 6px;
    height: 6px;
    border-radius: 999px;
    transform: translateX(-50%);
    background: #ffd166;
    box-shadow: 0 0 0 2px color-mix(in srgb, #ffd166 30%, transparent);
  }

  .chart-meta {
    margin-top: 8px;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px 10px;
    font-size: 11px;
    font-weight: 700;
  }
  .chart-meta span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .chart-meta-big {
    margin-top: 10px;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    font-size: 12px;
  }

  .grid.compact {
    gap: 10px;
  }
  .grid.compact .tile-head {
    padding: 10px 10px 8px;
  }
  .grid.compact .tile-body {
    padding: 10px;
  }
  .grid.compact .spark.wide {
    height: 86px;
  }
  .grid.compact .chart-meta {
    margin-top: 6px;
    gap: 4px 8px;
    font-size: 10px;
  }
  .grid.compact .trend-chip {
    min-height: 18px;
    padding: 1px 6px;
    font-size: 9px;
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
    .chart-meta,
    .chart-meta-big {
      grid-template-columns: 1fr;
    }
  }

  .picker-overlay {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: grid;
    place-items: center;
  }
  .picker-backdrop {
    position: absolute;
    inset: 0;
    border: none;
    background: rgba(0, 0, 0, 0.55);
  }
  .picker {
    position: relative;
    width: min(860px, calc(100vw - 24px));
    max-height: min(740px, calc(100vh - 24px));
    overflow: auto;
    border-radius: 18px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    box-shadow: var(--shadow-lg);
    padding: 14px;
  }
  .picker-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 10px;
  }
  .picker-head h3 {
    margin: 0;
    font-size: 20px;
    font-weight: 900;
  }
  .picker-summary {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-bottom: 10px;
  }
  .picker-chip {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 8px 10px;
    display: grid;
    gap: 2px;
    background: color-mix(in srgb, var(--bg-surface) 74%, transparent);
  }
  .picker-chip .k {
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-weight: 800;
    color: var(--text-muted);
  }
  .picker-chip .v {
    font-size: 12px;
    font-weight: 800;
    color: var(--text-primary);
  }
  .picker-body {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .picker-col {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 10px;
    background: color-mix(in srgb, var(--bg-surface) 80%, transparent);
    min-height: 380px;
    max-height: min(56vh, 520px);
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow: hidden;
  }
  .col-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    position: sticky;
    top: 0;
    z-index: 2;
    background: color-mix(in srgb, var(--bg-surface) 88%, transparent);
    padding-bottom: 6px;
  }
  .col-title {
    font-weight: 900;
  }
  .picker-list {
    display: grid;
    gap: 8px;
    overflow: auto;
    padding-right: 2px;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px;
    border-radius: 16px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 70%, transparent);
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
  }
  .pick.active {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border-color));
    background: color-mix(in srgb, var(--accent) 14%, var(--bg-surface));
  }

  .tile-settings {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 12px;
    background:
      linear-gradient(
        to bottom,
        color-mix(in srgb, var(--bg-surface) 82%, transparent),
        color-mix(in srgb, var(--bg-surface) 68%, transparent)
      );
  }
  .settings-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .field {
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 10px;
    background: color-mix(in srgb, var(--bg-surface) 74%, transparent);
  }
  .field .k {
    display: block;
    font-size: 11px;
    letter-spacing: 0.12em;
    font-weight: 900;
    color: var(--text-muted);
    text-transform: uppercase;
    margin-bottom: 8px;
  }
  .field .hint {
    display: block;
    margin-top: 8px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .field .row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .field input {
    width: 100%;
    padding: 10px 12px;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
    color: var(--text-primary);
    outline: none;
    transition:
      border-color 120ms ease,
      box-shadow 120ms ease;
  }
  .field input:focus {
    border-color: color-mix(in srgb, var(--accent) 52%, var(--border-color));
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 20%, transparent);
  }
  .unit-select {
    width: 92px;
    min-width: 92px;
    max-width: 92px;
    padding: 10px 8px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 700;
    outline: none;
  }
  .unit-select:focus {
    border-color: color-mix(in srgb, var(--accent) 52%, var(--border-color));
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 20%, transparent);
  }
  .settings-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid color-mix(in srgb, var(--border-color) 85%, transparent);
  }

  @media (max-width: 920px) {
    .threshold-summary {
      grid-template-columns: 1fr;
    }
    .settings-grid {
      grid-template-columns: 1fr;
    }
  }
  .pick:hover {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border-color));
  }

  @media (max-width: 920px) {
    .picker-summary {
      grid-template-columns: 1fr;
    }
    .picker-body {
      grid-template-columns: 1fr;
    }
    .picker-col {
      min-height: auto;
    }
  }

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

  .threshold-overlay {
    position: fixed;
    inset: 0;
    z-index: 95;
    display: grid;
    place-items: center;
  }
  .threshold-backdrop {
    position: absolute;
    inset: 0;
    border: none;
    background: rgba(0, 0, 0, 0.7);
  }
  .threshold {
    position: relative;
    width: min(860px, calc(100vw - 24px));
    max-height: min(740px, calc(100vh - 24px));
    overflow: auto;
    border-radius: 18px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    box-shadow: var(--shadow-lg);
    padding: 14px;
  }
  .threshold-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }
  .threshold-summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
    margin-bottom: 10px;
  }
  .threshold-chip {
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-surface) 74%, transparent);
    padding: 10px 12px;
    display: grid;
    gap: 4px;
  }
  .threshold-chip .k {
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-weight: 900;
    color: var(--text-muted);
  }
  .threshold-chip .v {
    font-size: 14px;
    font-weight: 900;
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
  .spark.huge.historical {
    height: min(34dvh, 300px);
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
  }
</style>
