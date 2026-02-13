<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page as pageStore } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import MiniSelect from '$lib/components/ui/MiniSelect.svelte';
  import { toast } from '$lib/stores/toast';
  import { isSidebarCollapsed } from '$lib/stores/ui';

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

  let loading = $state(true);
  let refreshing = $state(false);
  let rows = $state<NocRow[]>([]);
  let alerts = $state<AlertRow[]>([]);

  let statusFilter = $state<'all' | 'offline' | 'online'>('all');

  let kiosk = $state(true);
  let pollMs = $state(1000);
  let keepAwake = $state(false);

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

  type LayoutPreset = '2x2' | '3x2' | '3x3' | '4x3';
  type RotateMode = 'manual' | 'auto';
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
  let thresholdIndex = $state<number | null>(null);
  let thWarnRxKbps = $state<string>('');
  let thWarnTxKbps = $state<string>('');
  let thWarnRxUnit = $state<'Kbps' | 'Mbps' | 'Gbps'>('Kbps');
  let thWarnTxUnit = $state<'Kbps' | 'Mbps' | 'Gbps'>('Kbps');
  const thresholdUnitOptions: Array<{ value: 'Kbps' | 'Mbps' | 'Gbps'; label: string }> = [
    { value: 'Kbps', label: 'Kbps' },
    { value: 'Mbps', label: 'Mbps' },
    { value: 'Gbps', label: 'Gbps' },
  ];

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
  let focusMode = $state(false);
  let alertsOpen = $state(false);
  let renderNow = $state(Date.now());
  let uninstallAutoHide: (() => void) | null = null;

  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);
  let dragging = $state(false);

  const tenantPrefix = $derived.by(() => {
    const tid = String($pageStore.params.tenant || '');
    return tid ? `/${tid}` : '';
  });

  // Fit-to-screen (no scroll): scale and center wallboard content inside a fixed viewport.
  let fit = $state(false);
  let fitViewport: HTMLDivElement | null = null;
  let fitContent: HTMLDivElement | null = null;
  let fitScale = $state(1);
  let fitX = $state(0);
  let fitY = $state(0);
  let fitObs: ResizeObserver | null = null;

  // Auto-hide toolbar when Fit is enabled (NOC display friendly).
  let controlsHidden = $state(false);
  let lastActivityAt = $state(Date.now());
  let hideHandle: any = null;

  const SETTINGS_LAYOUT_KEY = 'mikrotik_wallboard_layout';
  const SETTINGS_SLOTS_KEY = 'mikrotik_wallboard_slots_json';
  const ROTATE_MODE_KEY = 'mikrotik_wallboard_rotate_mode';
  const ROTATE_MS_KEY = 'mikrotik_wallboard_rotate_ms';
  const FOCUS_MODE_KEY = 'mikrotik_wallboard_focus_mode';
  const STATUS_FILTER_KEY = 'mikrotik_wallboard_status_filter';
  const POLL_MS_KEY = 'mikrotik_wallboard_poll_ms';
  const KEEP_AWAKE_KEY = 'mikrotik_wallboard_keep_awake';
  const KIOSK_KEY = 'mikrotik_wallboard_kiosk';
  const FIT_TARGET_ASPECT = 16 / 9;

  function isLayoutPreset(v: string | null): v is LayoutPreset {
    return v === '2x2' || v === '3x2' || v === '3x3' || v === '4x3';
  }

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

  async function loadAlerts(silent = true) {
    try {
      alerts = (await api.mikrotik.alerts.list({ activeOnly: true, limit: 300 })) as any;
    } catch (e: any) {
      if (!silent) toast.error(e?.message || e);
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
      await loadAlerts(false);
    } catch (e: any) {
      toast.error(e?.message || e);
    }
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
  }

  function closeFull() {
    fullIndex = null;
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
      localStorage.setItem(KEEP_AWAKE_KEY, keepAwake ? '1' : '0');
      localStorage.setItem(KIOSK_KEY, kiosk ? '1' : '0');
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
      if ([5000, 10000, 15000, 30000, 60000].includes(rms)) rotateMs = rms;
      const fm = localStorage.getItem(FOCUS_MODE_KEY);
      if (fm != null) focusMode = fm === '1' || fm === 'true';
      const sf = localStorage.getItem(STATUS_FILTER_KEY);
      if (sf === 'all' || sf === 'online' || sf === 'offline') statusFilter = sf;
      const pm = Number(localStorage.getItem(POLL_MS_KEY) || 1000);
      if ([1000, 2000, 5000].includes(pm)) pollMs = pm;
      const ka = localStorage.getItem(KEEP_AWAKE_KEY);
      if (ka != null) keepAwake = ka === '1' || ka === 'true';
      const kz = localStorage.getItem(KIOSK_KEY);
      if (kz != null) kiosk = kz === '1' || kz === 'true';
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
    await loadAlerts(true);
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
      } catch {
        // quiet: wallboard should not spam toasts every second
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

  function recomputeFit() {
    if (!fit || !fitViewport || !fitContent) {
      fitScale = 1;
      fitX = 0;
      fitY = 0;
      return;
    }

    const viewportW = fitViewport.clientWidth;
    const viewportH = fitViewport.clientHeight;
    const contentW = fitContent.offsetWidth;
    const contentH = fitContent.offsetHeight;
    if (!viewportW || !viewportH || !contentW || !contentH) return;

    // Fit into a fixed 16:9 stage (letterbox on non-16:9 screens), then center content inside it.
    let stageW = viewportW;
    let stageH = Math.round(stageW / FIT_TARGET_ASPECT);
    if (stageH > viewportH) {
      stageH = viewportH;
      stageW = Math.round(stageH * FIT_TARGET_ASPECT);
    }

    const stageX = Math.floor((viewportW - stageW) / 2);
    const stageY = Math.floor((viewportH - stageH) / 2);

    const pad = 12; // breathing room to avoid edge clipping
    const s = Math.min(1, (stageW - pad * 2) / contentW, (stageH - pad * 2) / contentH);
    fitScale = Math.max(0.25, s);
    fitX = stageX + Math.max(0, Math.floor((stageW - contentW * fitScale) / 2));
    fitY = stageY + Math.max(0, Math.floor((stageH - contentH * fitScale) / 2));
  }

  function showControls() {
    controlsHidden = false;
    lastActivityAt = Date.now();
    if (hideHandle) clearTimeout(hideHandle);
    hideHandle = null;
  }

  function toggleAlertsPanel() {
    alertsOpen = !alertsOpen;
  }

  function installAutoHideListeners() {
    if (typeof window === 'undefined') return;

    const onAny = () => showControls();
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
    window.addEventListener('wheel', onAny, { passive: true });
    window.addEventListener('touchstart', onAny, { passive: true });
    window.addEventListener('keydown', onKey);

    return () => {
      window.removeEventListener('mousemove', onAny as any);
      window.removeEventListener('pointermove', onAny as any);
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

  async function navigateWithTransition(path: string) {
    try {
      const start = (document as any).startViewTransition;
      if (typeof start === 'function') {
        await start(() => goto(path)).finished;
        return;
      }
    } catch {
      // fallback
    }
    await goto(path);
  }

  onMount(() => {
    if (!$can('read', 'network_routers') && !$can('manage', 'network_routers')) {
      goto('/unauthorized');
      return;
    }

    loadConfig();
    ensureSlots();
    applyKiosk(kiosk);
    void loadRemoteConfig();

    // Fit-to-screen setup (avoid scrollbars).
    if (typeof window !== 'undefined') window.addEventListener('resize', recomputeFit);
    if (typeof ResizeObserver !== 'undefined') {
      fitObs = new ResizeObserver(() => recomputeFit());
      if (fitViewport) fitObs.observe(fitViewport);
      if (fitContent) fitObs.observe(fitContent);
    }
    setTimeout(recomputeFit, 0);

    uninstallAutoHide = installAutoHideListeners() ?? null;
    showControls();

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
    alertTick = setInterval(() => void loadAlerts(true), 10000);
  });

  onDestroy(() => {
    if (tick) clearInterval(tick);
    if (alertTick) clearInterval(alertTick);
    if (persistTimer) clearTimeout(persistTimer);
    // Best-effort flush so layout/slots don't get lost on fast logout/navigation.
    void persistRemoteNow();
    void applyWakeLock(false);
    if (typeof document !== 'undefined') document.body.classList.remove('kiosk-wallboard');
    if (typeof window !== 'undefined') window.removeEventListener('resize', recomputeFit);
    try {
      fitObs?.disconnect?.();
    } catch {
      // ignore
    }
    if (hideHandle) clearTimeout(hideHandle);
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
    setTimeout(recomputeFit, 0);
  });

  $effect(() => {
    void applyWakeLock(keepAwake);
  });

  $effect(() => {
    if (typeof document === 'undefined') return;
    document.body.classList.toggle('wallboard-fit', fit);
    controlsHidden = false;
    if (hideHandle) clearTimeout(hideHandle);
    hideHandle = null;
    setTimeout(recomputeFit, 0);
  });

  $effect(() => {
    focusMode;
    persistConfig();
    setTimeout(recomputeFit, 0);
  });
</script>

<div class="wallboard-viewport" class:fit bind:this={fitViewport}>
  <div
    class="wallboard"
    class:focus={focusMode}
    bind:this={fitContent}
    style={fit
      ? `transform: translate(${fitX}px, ${fitY}px) scale(${fitScale}); transform-origin: top left;`
      : ''}
  >
  <div class="wb-top">
    <div class="controls wall-actions">
      {#if pageCount > 1}
        <div class="pager" aria-label="Pages">
          <button
            class="pager-btn"
            type="button"
            onclick={() => (page = Math.max(0, page - 1))}
            disabled={page === 0}
            aria-label="Previous page"
          >
            <Icon name="chevron-left" size={16} />
          </button>
          <span class="pager-label">
            {($t('common.page') || 'Page') + ' ' + (page + 1) + '/' + pageCount}
          </span>
          <button
            class="pager-btn"
            type="button"
            onclick={() => (page = Math.min(pageCount - 1, page + 1))}
            disabled={page >= pageCount - 1}
            aria-label="Next page"
          >
            <Icon name="chevron-right" size={16} />
          </button>
        </div>
      {/if}

      <div class="seg">
        <button onclick={() => void refresh()} disabled={refreshing}>
          <Icon name="refresh-cw" size={16} />
          {$t('common.refresh') || 'Refresh'}
        </button>
        <button
          onclick={() => setPaused(!paused)}
          title={paused ? $t('admin.network.wallboard.resume') || 'Resume' : $t('admin.network.wallboard.pause') || 'Pause'}
        >
          <Icon name={paused ? 'play' : 'pause'} size={16} />
          {paused ? $t('admin.network.wallboard.resume') || 'Resume' : $t('admin.network.wallboard.pause') || 'Pause'}
        </button>
        <button onclick={() => void toggleFullscreen()}>
          <Icon name="monitor" size={16} />
          {$t('admin.network.wallboard.fullscreen') || 'Fullscreen'}
        </button>
        <button
          class:active={focusMode}
          onclick={() => (focusMode = !focusMode)}
          title={$t('admin.network.wallboard.focus_mode_hint') || 'Focus mode (F)'}
        >
          <Icon name="target" size={16} />
          {$t('admin.network.wallboard.focus_mode') || 'Focus'}
        </button>
      </div>

      <div class="seg">
        <button onclick={() => void navigateWithTransition(`${tenantPrefix}/admin/network/noc/wallboard/settings`)}>
          <Icon name="settings" size={16} />
          {$t('admin.network.wallboard.controls.open') || 'Settings'}
        </button>
        <button onclick={exitWallboard} title={$t('admin.network.wallboard.exit') || $t('sidebar.exit') || 'Exit'}>
          <Icon name="arrow-left" size={16} />
          {$t('admin.network.wallboard.exit') || $t('sidebar.exit') || 'Exit'}
        </button>
      </div>

      <div class="poll">
        <span class="muted">{$t('admin.network.wallboard.poll') || 'Poll'}</span>
        <strong class="mono">{Math.round(pollMs / 1000)}s</strong>
      </div>
      {#if rotateMode === 'auto'}
        <div class="poll">
          <span class="muted">{$t('admin.network.wallboard.controls.auto_rotate') || 'Auto rotate'}</span>
          <strong class="mono">{Math.round(rotateMs / 1000)}s</strong>
        </div>
      {/if}
    </div>
  </div>

  {#if sortedAlerts.length > 0 && alertsOpen}
    <div id="wallboard-alert-panel" class="alert-strip floating-alert-panel">
      <div class="alert-strip-head">
        <Icon name="alert-triangle" size={15} />
        <span class="alert-strip-title">
          {$t('admin.network.wallboard.alerts_open') || 'Open alerts'}
        </span>
        <span class="alert-strip-count">{sortedAlerts.length}</span>
      </div>
      <div class="alert-strip-list">
        {#each sortedAlerts.slice(0, 8) as a (a.id)}
          <button
            type="button"
            class="alert-chip"
            class:crit={String(a.severity || '').toLowerCase() === 'critical'}
            onclick={() => goto(`${tenantPrefix}/admin/network/alerts`)}
            title={a.message}
          >
            <span class="mono">{routerById(a.router_id)?.identity || routerById(a.router_id)?.name || a.router_id}</span>
            <span class="muted">·</span>
            <span>{a.title}</span>
          </button>
        {/each}
      </div>
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
          {@const ra = routerAlertMap[slot.routerId]}
          <div
            class="tile iface-tile"
            class:warn={warnRx || warnTx}
            class:drag-over={dragOver === gidx}
            data-wall-slot={gidx}
            role="button"
            tabindex="0"
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
        <button
          class="icon-x drag"
          type="button"
          onclick={(e) => e.stopPropagation()}
          onpointerdown={(e) => startDrag(e, gidx)}
          title={$t('admin.network.wallboard.drag') || 'Drag to reorder'}
        >
          <Icon name="grip-vertical" size={16} />
        </button>
        <button
          class="icon-x"
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            openThreshold(gidx);
          }}
          title={$t('common.edit') || 'Edit'}
        >
          <Icon name="edit" size={16} />
        </button>
                <button
                  class="icon-x"
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    clearSlot(gidx);
                  }}
                  title={$t('common.remove') || 'Remove'}
                >
                  <Icon name="x" size={16} />
                </button>
                {#if stale}
                  <span class="badge warn" title={$t('admin.network.wallboard.stale') || 'Data stale'}>
                    <Icon name="alert-triangle" size={14} />
                    {$t('admin.network.wallboard.stale') || 'Stale'}
                  </span>
                {/if}
                <span class="badge" class:ok={r?.is_online} class:bad={!r?.is_online}>
                  <span class="dot"></span>
                  {r?.is_online
                    ? $t('admin.network.routers.badges.online') || 'Online'
                    : $t('admin.network.routers.badges.offline') || 'Offline'}
                </span>
              </div>
            </div>

            <div class="tile-body">
              <div class="spark wide">
                <div class="bars" class:warn={warnRx}>
                  <div class="spark-panel-title">
                    <span class="spark-chip">RX</span>
                    <span class="mono rate" class:warn={warnRx}>{formatBps(rxNow)}</span>
                  </div>
                  {#each rx as v, i (i)}
                    <div
                      class="bar rx"
                      style={`height:${Math.round((v / max) * 100)}%;`}
                      data-value={formatBps(v)}
                      title={`RX • ${formatBps(v)}`}
                    ></div>
                  {/each}
                </div>
                <div class="bars" class:warn={warnTx}>
                  <div class="spark-panel-title">
                    <span class="spark-chip">TX</span>
                    <span class="mono rate" class:warn={warnTx}>{formatBps(txNow)}</span>
                  </div>
                  {#each tx as v, i (i)}
                    <div
                      class="bar tx"
                      style={`height:${Math.round((v / max) * 100)}%;`}
                      data-value={formatBps(v)}
                      title={`TX • ${formatBps(v)}`}
                    ></div>
                  {/each}
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
      <div class="picker-tools">
        <div class="pill small">
          <Icon name="search" size={16} />
          <input
            value={pickerRouterSearch}
            oninput={(e) => (pickerRouterSearch = (e.currentTarget as HTMLInputElement).value)}
            placeholder={$t('admin.network.wallboard.pick_search') || 'Search routers...'}
          />
        </div>
        <div class="seg">
          <button class:active={statusFilter === 'all'} onclick={() => (statusFilter = 'all')}>
            {$t('admin.network.wallboard.filters.all') || 'All'}
          </button>
          <button class:active={statusFilter === 'online'} onclick={() => (statusFilter = 'online')}>
            {$t('admin.network.wallboard.filters.online') || 'Online'}
          </button>
          <button class:active={statusFilter === 'offline'} onclick={() => (statusFilter = 'offline')}>
            {$t('admin.network.wallboard.filters.offline') || 'Offline'}
          </button>
        </div>
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
                          <span class="tag">disabled</span>
                        {:else if it.running}
                          <span class="tag ok">up</span>
                        {:else}
                          <span class="tag">down</span>
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
              <MiniSelect
                minWidth={88}
                ariaLabel="Unit"
                bind:value={thWarnRxUnit}
                options={thresholdUnitOptions}
              />
            </div>
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
              <MiniSelect
                minWidth={88}
                ariaLabel="Unit"
                bind:value={thWarnTxUnit}
                options={thresholdUnitOptions}
              />
            </div>
          </label>
        </div>

        <div class="settings-actions">
          <button class="btn-mini primary" type="button" onclick={saveThreshold}>
            <Icon name="save" size={16} />
            {$t('common.save') || 'Save'}
          </button>
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
  :global(body.wallboard-fit) {
    overflow: hidden;
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

  .wallboard-viewport.fit {
    position: fixed;
    inset: 0;
    overflow: hidden;
    z-index: 60;
    background: color-mix(in srgb, var(--bg-base, #000) 85%, transparent);
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
  .wallboard.focus .controls {
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

  .seg {
    display: inline-flex;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    overflow: hidden;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
  }
  .seg button {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border: none;
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
    font-weight: 650;
    font-size: 13px;
  }
  .seg button:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .seg button.active {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .pager {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
  }

  .pager-label {
    font-weight: 850;
    font-size: 13px;
    color: var(--text-muted);
    min-width: 88px;
    text-align: center;
    white-space: nowrap;
  }

  .pager-btn {
    width: 34px;
    height: 34px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 80%, transparent);
    color: var(--text-primary);
    display: grid;
    place-items: center;
    cursor: pointer;
  }

  .pager-btn:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .poll {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
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

  .alert-strip-head {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-weight: 800;
    color: var(--text-primary);
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
  .icon-x.drag {
    cursor: grab;
  }
  .icon-x.drag:active {
    cursor: grabbing;
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
  .badge.warn {
    border-color: color-mix(in srgb, var(--color-warning) 55%, var(--border-color));
    color: color-mix(in srgb, var(--color-warning) 85%, var(--text-primary));
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
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
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
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
  .bar::after {
    content: attr(data-value);
    position: absolute;
    left: 50%;
    bottom: calc(100% + 6px);
    transform: translateX(-50%);
    white-space: nowrap;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 2px 6px;
    font-size: 10px;
    font-weight: 700;
    color: var(--text-primary);
    background: color-mix(in srgb, var(--bg-surface) 90%, transparent);
    opacity: 0;
    pointer-events: none;
    transition: opacity 100ms ease;
    z-index: 3;
  }
  .bar:hover::after {
    opacity: 1;
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
  }
  .picker-tools {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
    margin-bottom: 10px;
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
    min-height: 360px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .col-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .col-title {
    font-weight: 900;
  }
  .picker-list {
    display: grid;
    gap: 8px;
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
    margin-top: 10px;
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 10px;
    background: color-mix(in srgb, var(--bg-surface) 70%, transparent);
  }
  .settings-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
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
  }
  .settings-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 12px;
  }

  @media (max-width: 920px) {
    .settings-grid {
      grid-template-columns: 1fr;
    }
  }
  .pick:hover {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border-color));
  }

  @media (max-width: 920px) {
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
    margin-bottom: 12px;
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
    .full-stats {
      grid-template-columns: 1fr;
    }
  }
</style>
