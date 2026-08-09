<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import type { Component } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { api } from '$lib/api/client';
  import { extractApiErrorCode } from '$lib/api/core';
  import type { ManagedRadiusRouterSetup as ManagedRadiusRouterSetupResponse } from '$lib/api/types';
  import { toast } from '$lib/stores/toast';
  import { resolveBackTarget } from '$lib/utils/backNavigation';
  import {
    canCopyManagedRadiusSecret,
    getManagedRadiusSummary,
    getManagedRadiusDisplayedSecret,
    shouldShowAssignDefaultManagedRadius,
    shouldShowCreateManagedRadiusMapping,
    shouldShowManagedRadiusUpgrade,
  } from '$lib/utils/managedRadiusSetup';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ResponsiveTabs from '$lib/components/ui/ResponsiveTabs.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import { loadRouterDetailDialogs } from './routerDetailPageModules';

  type RouterRow = {
    id: string;
    name: string;
    host: string;
    port: number;
    username: string;
    enabled: boolean;
    identity?: string | null;
    ros_version?: string | null;
    is_online: boolean;
    last_seen_at?: string | null;
    latency_ms?: number | null;
    last_error?: string | null;
    maintenance_until?: string | null;
    maintenance_reason?: string | null;
    updated_at?: string | null;
  };

  type InterfaceSnap = {
    name: string;
    interface_type?: string | null;
    running?: boolean | null;
    disabled?: boolean | null;
    mtu?: number | null;
    mac_address?: string | null;
    rx_byte?: number | null;
    tx_byte?: number | null;
    rx_packet?: number | null;
    tx_packet?: number | null;
    link_downs?: number | null;
  };

  type IpSnap = {
    address: string;
    network?: string | null;
    interface?: string | null;
    disabled?: boolean | null;
    dynamic?: boolean | null;
  };

  type HealthSnap = {
    temperature_c?: number | null;
    voltage_v?: number | null;
    cpu_temperature_c?: number | null;
  };

  type RouterSnapshot = {
    router: RouterRow;
    cpu_load?: number | null;
    total_memory_bytes?: number | null;
    free_memory_bytes?: number | null;
    total_hdd_bytes?: number | null;
    free_hdd_bytes?: number | null;
    uptime_seconds?: number | null;
    board_name?: string | null;
    architecture?: string | null;
    cpu?: string | null;
    interfaces: InterfaceSnap[];
    ip_addresses: IpSnap[];
    health?: HealthSnap | null;
  };

  type MetricRow = {
    ts: string;
    cpu_load?: number | null;
    total_memory_bytes?: number | null;
    free_memory_bytes?: number | null;
    total_hdd_bytes?: number | null;
    free_hdd_bytes?: number | null;
    uptime_seconds?: number | null;
  };

  let initialLoading = $state(true);
  let refreshing = $state(false);
  let router = $state<RouterRow | null>(null);
  let snapshot = $state<RouterSnapshot | null>(null);
  let metrics = $state<MetricRow[]>([]);
  let ifacePrev = $state<Record<string, { rx: number; tx: number; ts: number }>>({});
  let ifaceRates = $state<Record<string, { rx_bps: number | null; tx_bps: number | null }>>({});

  let isMobile = $state(false);
  let mqCleanup: (() => void) | null = null;

  let selectedInterface = $state<string | null>(null);
  let showInterfaceTrafficModal = $state(false);
  let ifaceHistoryLoading = $state(false);
  let ifaceHistory = $state<any[]>([]);
  let RouterDetailDialogsComponent = $state<Component<any> | null>(null);

  type PppProfileRow = {
    id: string;
    name: string;
    local_address?: string | null;
    remote_address?: string | null;
    rate_limit?: string | null;
    dns_server?: string | null;
    comment?: string | null;
    router_present: boolean;
    last_sync_at?: string | null;
  };

  type IpPoolRow = {
    id: string;
    name: string;
    ranges?: string | null;
    next_pool?: string | null;
    comment?: string | null;
    router_present: boolean;
    last_sync_at?: string | null;
  };

  let pppProfilesLoading = $state(false);
  let ipPoolsLoading = $state(false);
  let pppProfiles = $state<PppProfileRow[]>([]);
  let ipPools = $state<IpPoolRow[]>([]);
  const routerListPath = $derived($page.url.pathname.replace(/\/[^/]+\/?$/, ''));
  const routerBackTarget = $derived(resolveBackTarget($page.url, routerListPath));
  let pppLoadedFor = $state<string | null>(null);
  let poolsLoadedFor = $state<string | null>(null);
  let managedRadiusSetupLoading = $state(false);
  let managedRadiusSetup = $state<ManagedRadiusRouterSetupResponse | null>(null);
  let managedRadiusLoadedFor = $state<string | null>(null);
  let showManagedRadiusSecret = $state(false);
  let showManagedRadiusModal = $state(false);
  let assigningManagedRadiusDefault = $state(false);
  let creatingManagedRadiusMapping = $state(false);
  let applyingManagedRadius = $state(false);
  let canRevealManagedRadiusSecret = $derived($can('manage_radius_secret', 'router_inventory'));

  let cpuSeries = $derived.by(() => {
    const pts = metrics
      .slice()
      .reverse()
      .map((m) => (m.cpu_load == null ? null : Math.max(0, Math.min(100, m.cpu_load))));
    return pts.filter((v) => v != null) as number[];
  });

  let activeTab = $state<'overview' | 'interfaces' | 'ip' | 'metrics'>('overview');
  let ifFilter = $state<'all' | 'running' | 'down' | 'disabled'>('all');
  const routerTabItems = $derived.by(() => [
    { id: 'overview', label: $t('network.router.tab_overview') || 'Overview' },
    { id: 'interfaces', label: $t('network.router.tab_interfaces') || 'Interfaces', count: snapshot?.interfaces?.length || 0 },
    { id: 'ip', label: $t('network.router.tab_ip_addresses') || 'IP Addresses', count: snapshot?.ip_addresses?.length || 0 },
    { id: 'metrics', label: $t('network.router.tab_metrics') || 'Metrics' },
  ]);

  let watchSearch = $state('');
  let watched = $state<string[]>([]);
  let liveLoading = $state(false);
  let livePrev = $state<Record<string, { rx: number; tx: number; ts: number }>>({});
  let liveSeries = $state<Record<string, { rx: number[]; tx: number[] }>>({});
  let liveRates = $state<Record<string, { rx_bps: number | null; tx_bps: number | null }>>({});
  let liveHandle: any = null;

  let refreshHandle: any = null;
  let refreshInFlight = false;

  onMount(() => {
    if (!$can('read', 'router_inventory') && !$can('manage', 'router_inventory')) {
      goto('/unauthorized');
      return;
    }

    const tab = ($page.url.searchParams.get('tab') || '').toLowerCase();
    if (tab === 'overview' || tab === 'interfaces' || tab === 'ip' || tab === 'metrics') {
      activeTab = tab;
    }

    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(max-width: 1024px)');
      const sync = () => (isMobile = mq.matches);
      sync();

      try {
        mq.addEventListener('change', sync);
        mqCleanup = () => mq.removeEventListener('change', sync);
      } catch {
        // @ts-ignore
        mq.addListener?.(sync);
        // @ts-ignore
        mqCleanup = () => mq.removeListener?.(sync);
      }
    }

    void refresh({ silent: true });

    // Re-check status/metrics periodically.
    refreshHandle = setInterval(() => {
      void refresh({ silent: true });
    }, 5000);
  });

  onDestroy(() => {
    if (refreshHandle) clearInterval(refreshHandle);
    mqCleanup?.();
    if (liveHandle) clearInterval(liveHandle);
  });

  $effect(() => {
    if (liveHandle) {
      clearInterval(liveHandle);
      liveHandle = null;
    }
  });

  function toggleWatched(name: string) {
    const set = new Set(watched);
    if (set.has(name)) set.delete(name);
    else {
      if (set.size >= 6) {
        toast.error($t('admin.network.routers.traffic.max_watch') || 'Max 6 interfaces.');
        return;
      }
      set.add(name);
    }
    watched = Array.from(set);
  }

  function clearWatched() {
    watched = [];
    livePrev = {};
    liveSeries = {};
    liveRates = {};
  }

  async function pollLive() {
    if (!router?.id) return;
    if (!watched.length) return;
    if (liveLoading) return;

    liveLoading = true;
    try {
      const rows = (await api.mikrotik.routers.interfaceLive(router.id, watched)) as any[];
      const nowMs = Date.now();

      const nextPrev = { ...livePrev };
      const nextRates: Record<string, { rx_bps: number | null; tx_bps: number | null }> = {
        ...liveRates,
      };
      const nextSeries: Record<string, { rx: number[]; tx: number[] }> = { ...liveSeries };

      for (const r of rows) {
        const name = String(r.name || '');
        if (!name) continue;

        const rx = typeof r.rx_byte === 'number' ? r.rx_byte : null;
        const tx = typeof r.tx_byte === 'number' ? r.tx_byte : null;

        const prev = nextPrev[name];
        let rx_bps: number | null = null;
        let tx_bps: number | null = null;

        if (prev && prev.ts > 0) {
          const dt = nowMs - prev.ts;
          if (dt > 0) {
            if (rx != null && rx >= prev.rx) rx_bps = Math.round(((rx - prev.rx) * 8 * 1000) / dt);
            if (tx != null && tx >= prev.tx) tx_bps = Math.round(((tx - prev.tx) * 8 * 1000) / dt);
          }
        }

        nextRates[name] = { rx_bps, tx_bps };

        const series = nextSeries[name] || { rx: [], tx: [] };
        const rxPoint = rx_bps == null ? 0 : Math.max(0, rx_bps);
        const txPoint = tx_bps == null ? 0 : Math.max(0, tx_bps);
        series.rx = [...series.rx, rxPoint].slice(-60);
        series.tx = [...series.tx, txPoint].slice(-60);
        nextSeries[name] = series;

        if (rx != null || tx != null) {
          nextPrev[name] = { rx: rx ?? prev?.rx ?? 0, tx: tx ?? prev?.tx ?? 0, ts: nowMs };
        }
      }

      livePrev = nextPrev;
      liveRates = nextRates;
      liveSeries = nextSeries;
    } catch (e: any) {
      // Avoid spamming toasts; show once in a while via console
      console.warn('[Traffic] live poll failed', e);
    } finally {
      liveLoading = false;
    }
  }

  async function refresh(opts?: { silent?: boolean }) {
    if (refreshInFlight) return;
    refreshInFlight = true;

    if (!router) initialLoading = true;
    else refreshing = true;

    const id = $page.params.id || '';
    if (!id) {
      initialLoading = false;
      refreshing = false;
      refreshInFlight = false;
      return;
    }
    try {
      const [snap, m] = await Promise.all([
        api.mikrotik.routers.snapshot(id) as any,
        api.mikrotik.routers.metrics(id, 120) as any,
      ]);
      snapshot = snap as RouterSnapshot;
      router = (snapshot?.router || null) as any;
      metrics = (m || []) as any;
      if ($can('manage', 'router_inventory') && (managedRadiusLoadedFor !== id || !opts?.silent)) {
        await loadManagedRadiusSetup(id, { silent: Boolean(opts?.silent) });
      }

      // Live per-interface bps (computed from UI refresh deltas).
      if (snapshot?.interfaces?.length) {
        const nowMs = Date.now();
        const nextPrev = { ...ifacePrev };
        const nextRates: Record<string, { rx_bps: number | null; tx_bps: number | null }> = {};

        for (const it of snapshot.interfaces) {
          const rx = typeof it.rx_byte === 'number' ? it.rx_byte : null;
          const tx = typeof it.tx_byte === 'number' ? it.tx_byte : null;
          const prev = nextPrev[it.name];

          let rx_bps: number | null = null;
          let tx_bps: number | null = null;

          if (prev && prev.ts > 0) {
            const dt = nowMs - prev.ts;
            if (dt > 0) {
              if (rx != null && rx >= prev.rx)
                rx_bps = Math.round(((rx - prev.rx) * 8 * 1000) / dt);
              if (tx != null && tx >= prev.tx)
                tx_bps = Math.round(((tx - prev.tx) * 8 * 1000) / dt);
            }
          }

          nextRates[it.name] = { rx_bps, tx_bps };

          if (rx != null || tx != null) {
            nextPrev[it.name] = { rx: rx ?? prev?.rx ?? 0, tx: tx ?? prev?.tx ?? 0, ts: nowMs };
          }
        }

        ifacePrev = nextPrev;
        ifaceRates = nextRates;
      }
    } catch (e: any) {
      if (!opts?.silent) toast.error(e?.message || e);
    } finally {
      initialLoading = false;
      refreshing = false;
      refreshInFlight = false;
    }
  }

  async function loadManagedRadiusSetup(routerId: string, opts?: { silent?: boolean }) {
    if (!$can('manage', 'router_inventory')) {
      managedRadiusSetup = null;
      return;
    }

    managedRadiusSetupLoading = true;
    try {
      managedRadiusSetup = (await api.mikrotik.routers.managedRadiusSetup(
        routerId,
      )) as ManagedRadiusRouterSetupResponse;
      managedRadiusLoadedFor = routerId;
      showManagedRadiusSecret = false;
    } catch (e: any) {
      managedRadiusSetup = null;
      if (!opts?.silent) toast.error(e?.message || e);
    } finally {
      managedRadiusSetupLoading = false;
    }
  }

  async function copyManagedRadiusScript() {
    const script = managedRadiusSetup?.cli_script;
    if (!script) return;

    try {
      await navigator.clipboard.writeText(script);
      toast.success(
        $t('admin.network.routers.managed_radius.toasts.cli_copied') || 'RADIUS CLI copied',
      );
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function copyManagedRadiusSecret() {
    const secret = managedRadiusSetup?.shared_secret;
    if (!secret) return;

    try {
      await navigator.clipboard.writeText(secret);
      toast.success(
        $t('admin.network.routers.managed_radius.toasts.secret_copied') ||
          'Shared secret copied',
      );
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function openManagedRadiusModal() {
    const id = $page.params.id || router?.id || '';
    await ensureRouterDetailDialogsComponent();
    showManagedRadiusModal = true;
    if (!id) return;

    if (managedRadiusLoadedFor !== id || !managedRadiusSetup) {
      await loadManagedRadiusSetup(id);
    }
  }

  function closeManagedRadiusModal() {
    showManagedRadiusModal = false;
    showManagedRadiusSecret = false;
  }

  function showManagedRadiusError(error: any) {
    const errorCode = extractApiErrorCode(error);
    if (errorCode === 'PLAN_FEATURE_REQUIRED:managed_radius') {
      toast.warning('Plan upgrade required: ' + (error?.message || 'Upgrade required'));
    } else {
      toast.error(error?.message || error);
    }
  }

  async function assignManagedRadiusDefault() {
    const id = $page.params.id || router?.id || '';
    if (!id || assigningManagedRadiusDefault) return;

    assigningManagedRadiusDefault = true;
    try {
      managedRadiusSetup = (await api.mikrotik.routers.assignManagedRadiusDefault(
        id,
      )) as ManagedRadiusRouterSetupResponse;
      managedRadiusLoadedFor = id;
      showManagedRadiusSecret = false;
      toast.success(
        $t('admin.network.routers.managed_radius.toasts.default_assigned') ||
          'Default Managed RADIUS assigned',
      );
    } catch (e: any) {
      showManagedRadiusError(e);
    } finally {
      assigningManagedRadiusDefault = false;
    }
  }

  async function createManagedRadiusMapping() {
    const id = $page.params.id || router?.id || '';
    if (!id || creatingManagedRadiusMapping) return;

    creatingManagedRadiusMapping = true;
    try {
      managedRadiusSetup = (await api.mikrotik.routers.createManagedRadiusMapping(
        id,
      )) as ManagedRadiusRouterSetupResponse;
      managedRadiusLoadedFor = id;
      showManagedRadiusSecret = false;
      toast.success(
        $t('admin.network.routers.managed_radius.toasts.mapping_created') ||
          'Managed RADIUS NAS mapping created',
      );
    } catch (e: any) {
      showManagedRadiusError(e);
    } finally {
      creatingManagedRadiusMapping = false;
    }
  }

  async function applyManagedRadius() {
    const id = $page.params.id || router?.id || '';
    if (!id || applyingManagedRadius) return;

    applyingManagedRadius = true;
    try {
      managedRadiusSetup = (await api.mikrotik.routers.applyManagedRadius(
        id,
      )) as ManagedRadiusRouterSetupResponse;
      managedRadiusLoadedFor = id;
      showManagedRadiusSecret = false;
      toast.success(
        $t('admin.network.routers.managed_radius.toasts.applied') ||
          'Managed RADIUS applied to router',
      );
    } catch (e: any) {
      showManagedRadiusError(e);
    } finally {
      applyingManagedRadius = false;
    }
  }

  async function loadPppProfiles(opts?: { silent?: boolean }) {
    const id = $page.params.id || '';
    if (!id) return;
    if (pppProfilesLoading) return;

    pppProfilesLoading = true;
    try {
      const rows = (await api.mikrotik.routers.pppProfiles(id)) as any[];
      pppProfiles = (rows || []) as any;
      pppLoadedFor = id;
    } catch (e: any) {
      if (!opts?.silent) toast.error(e?.message || e);
    } finally {
      pppProfilesLoading = false;
    }
  }

  async function syncPppProfiles() {
    const id = $page.params.id || '';
    if (!id) return;
    if (pppProfilesLoading) return;

    pppProfilesLoading = true;
    try {
      const rows = (await api.mikrotik.routers.syncPppProfiles(id)) as any[];
      pppProfiles = (rows || []) as any;
      pppLoadedFor = id;
      toast.success(
        $t('admin.network.routers.ppp_profiles.toasts.synced') || 'Synced PPP profiles',
      );
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      pppProfilesLoading = false;
    }
  }

  async function loadIpPools(opts?: { silent?: boolean }) {
    const id = $page.params.id || '';
    if (!id) return;
    if (ipPoolsLoading) return;

    ipPoolsLoading = true;
    try {
      const rows = (await api.mikrotik.routers.ipPools(id)) as any[];
      ipPools = (rows || []) as any;
      poolsLoadedFor = id;
    } catch (e: any) {
      if (!opts?.silent) toast.error(e?.message || e);
    } finally {
      ipPoolsLoading = false;
    }
  }

  async function syncIpPools() {
    const id = $page.params.id || '';
    if (!id) return;
    if (ipPoolsLoading) return;

    ipPoolsLoading = true;
    try {
      const rows = (await api.mikrotik.routers.syncIpPools(id)) as any[];
      ipPools = (rows || []) as any;
      poolsLoadedFor = id;
      toast.success($t('admin.network.routers.ip_pools.toasts.synced') || 'Synced IP pools');
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      ipPoolsLoading = false;
    }
  }

  async function test() {
    if (!router) return;
    try {
      const res = await api.mikrotik.routers.test(router.id);
      if (res?.ok) {
        toast.success(
          `${res.identity || router.name} • RouterOS ${res.ros_version || ''} • ${res.latency_ms ?? ''}ms`,
        );
      } else {
        toast.error(res?.error || 'Failed to connect');
      }
      await refresh({ silent: true });
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  function statusLabel() {
    if (!router) return '';
    if (router.is_online) return $t('admin.network.routers.badges.online') || 'Online';
    return $t('admin.network.routers.badges.offline') || 'Offline';
  }

  function pctUsed(total?: number | null, free?: number | null) {
    if (!total || total <= 0 || free == null) return null;
    const used = total - free;
    return Math.max(0, Math.min(100, Math.round((used / total) * 100)));
  }

  function formatBytes(n?: number | null) {
    if (n == null) return $t('common.na') || '—';
    const abs = Math.abs(n);
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let u = 0;
    let v = abs;
    while (v >= 1024 && u < units.length - 1) {
      v /= 1024;
      u++;
    }
    const s = `${v >= 10 || u === 0 ? v.toFixed(0) : v.toFixed(1)} ${units[u]}`;
    return n < 0 ? `-${s}` : s;
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

  function formatUptime(secs?: number | null) {
    if (secs == null) return $t('common.na') || '—';
    const s = Math.max(0, Math.floor(secs));
    const d = Math.floor(s / 86400);
    const h = Math.floor((s % 86400) / 3600);
    const m = Math.floor((s % 3600) / 60);
    if (d > 0) return `${d}d ${h}h`;
    if (h > 0) return `${h}h ${m}m`;
    return `${m}m`;
  }

  async function openInterface(name: string) {
    if (!router) return;
    await ensureRouterDetailDialogsComponent();
    selectedInterface = name;
    showInterfaceTrafficModal = true;
    ifaceHistoryLoading = true;
    try {
      ifaceHistory = (await api.mikrotik.routers.interfaceMetrics(router.id, {
        interface: name,
        limit: 120,
      })) as any[];
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      ifaceHistoryLoading = false;
    }
  }

  function closeInterfaceTrafficModal() {
    showInterfaceTrafficModal = false;
    selectedInterface = null;
    ifaceHistory = [];
  }

  async function ensureRouterDetailDialogsComponent() {
    if (RouterDetailDialogsComponent) return;

    const modules = await loadRouterDetailDialogs();
    RouterDetailDialogsComponent = modules.RouterDetailDialogsComponent;
  }

  type InterfaceRow = InterfaceSnap & {
    status: 'running' | 'down' | 'disabled';
  };

  const interfaceRows = $derived.by(() => {
    const list = snapshot?.interfaces || [];
    const rows: InterfaceRow[] = list.map((it) => {
      const status: InterfaceRow['status'] = it.disabled
        ? 'disabled'
        : it.running
          ? 'running'
          : 'down';
      return { ...it, status };
    });

    switch (ifFilter) {
      case 'running':
        return rows.filter((r) => r.status === 'running');
      case 'down':
        return rows.filter((r) => r.status === 'down');
      case 'disabled':
        return rows.filter((r) => r.status === 'disabled');
      default:
        return rows;
    }
  });

  const interfaceTableData = $derived.by(() =>
    interfaceRows.map((r) => ({
      id: r.name,
      name: r.name,
      type: r.interface_type || '—',
      status: r.status,
      mtu: r.mtu ?? '—',
      mac: r.mac_address || '—',
      rx_rate: formatBps(ifaceRates[r.name]?.rx_bps ?? null),
      tx_rate: formatBps(ifaceRates[r.name]?.tx_bps ?? null),
      rx: formatBytes(r.rx_byte),
      tx: formatBytes(r.tx_byte),
      downs: r.link_downs ?? '—',
      disabled: Boolean(r.disabled),
    })),
  );

  const interfaceColumns = $derived([
    { key: 'name', label: $t('common.name') },
    { key: 'type', label: $t('common.type') },
    { key: 'status', label: $t('common.status') },
    { key: 'mtu', label: $t('network.router.mtu'), align: 'right' as const, width: '90px' },
    { key: 'mac', label: $t('network.router.mac'), class: 'mono' },
    { key: 'rx_rate', label: $t('network.router.rx_rate'), class: 'mono', align: 'right' as const, width: '130px' },
    { key: 'tx_rate', label: $t('network.router.tx_rate'), class: 'mono', align: 'right' as const, width: '130px' },
    { key: 'rx', label: $t('network.router.rx'), class: 'mono', align: 'right' as const, width: '120px' },
    { key: 'tx', label: $t('network.router.tx'), class: 'mono', align: 'right' as const, width: '120px' },
    { key: 'downs', label: $t('network.router.downs'), class: 'mono', align: 'right' as const, width: '90px' },
  ]);

  const ipRows = $derived.by(() => snapshot?.ip_addresses || []);

  const ipTableData = $derived.by(() =>
    ipRows.map((ip, idx) => ({
      id: `${ip.address}:${ip.interface || ''}:${idx}`,
      address: ip.address,
      interface: ip.interface || '—',
      network: ip.network || '—',
      dynamic: Boolean(ip.dynamic),
      disabled: Boolean(ip.disabled),
    })),
  );

  const ipColumns = $derived([
    { key: 'address', label: $t('network.router.address'), class: 'mono' },
    { key: 'interface', label: $t('network.router.interface') },
    { key: 'network', label: $t('network.router.network'), class: 'mono' },
    { key: 'flags', label: $t('network.router.flags') },
  ]);

  const pppProfileTableData = $derived.by(() =>
    pppProfiles.map((p, idx) => ({
      id: p.id || `${p.name}:${idx}`,
      name: p.name,
      local_address: p.local_address || 'â€”',
      remote_address: p.remote_address || 'â€”',
      rate_limit: p.rate_limit || 'â€”',
      dns_server: p.dns_server || 'â€”',
      present: Boolean(p.router_present),
      last_sync_at: p.last_sync_at,
    })),
  );

  const pppProfileColumns = $derived([
    { key: 'name', label: $t('admin.network.routers.ppp_profiles.columns.name') || 'Name' },
    {
      key: 'local_address',
      label: $t('admin.network.routers.ppp_profiles.columns.local') || 'Local',
      class: 'mono',
      width: '170px',
    },
    {
      key: 'remote_address',
      label: $t('admin.network.routers.ppp_profiles.columns.remote') || 'Remote',
      class: 'mono',
      width: '170px',
    },
    {
      key: 'rate_limit',
      label: $t('admin.network.routers.ppp_profiles.columns.rate') || 'Rate',
      class: 'mono',
      width: '160px',
    },
    {
      key: 'dns_server',
      label: $t('admin.network.routers.ppp_profiles.columns.dns') || 'DNS',
      class: 'mono',
      width: '170px',
    },
    {
      key: 'present',
      label: $t('admin.network.routers.ppp_profiles.columns.state') || 'State',
      width: '120px',
    },
    {
      key: 'last_sync_at',
      label: $t('admin.network.routers.ppp_profiles.columns.synced') || 'Synced',
      class: 'mono',
      width: '120px',
    },
  ]);

  const ipPoolTableData = $derived.by(() =>
    ipPools.map((p, idx) => ({
      id: p.id || `${p.name}:${idx}`,
      name: p.name,
      ranges: p.ranges || 'â€”',
      next_pool: p.next_pool || 'â€”',
      present: Boolean(p.router_present),
      last_sync_at: p.last_sync_at,
    })),
  );

  const ipPoolColumns = $derived([
    { key: 'name', label: $t('admin.network.routers.ip_pools.columns.name') || 'Name' },
    {
      key: 'ranges',
      label: $t('admin.network.routers.ip_pools.columns.ranges') || 'Ranges',
      class: 'mono',
    },
    {
      key: 'next_pool',
      label: $t('admin.network.routers.ip_pools.columns.next') || 'Next pool',
      class: 'mono',
      width: '160px',
    },
    {
      key: 'present',
      label: $t('admin.network.routers.ip_pools.columns.state') || 'State',
      width: '120px',
    },
    {
      key: 'last_sync_at',
      label: $t('admin.network.routers.ip_pools.columns.synced') || 'Synced',
      class: 'mono',
      width: '120px',
    },
  ]);

  const metricRows = $derived.by(() =>
    metrics.map((m) => ({
      ...m,
      id: m.ts,
      cpu: m.cpu_load == null ? '—' : `${m.cpu_load}%`,
      mem:
        m.total_memory_bytes == null || m.free_memory_bytes == null
          ? '—'
          : `${formatBytes(m.free_memory_bytes)} / ${formatBytes(m.total_memory_bytes)}`,
      disk:
        m.total_hdd_bytes == null || m.free_hdd_bytes == null
          ? '—'
          : `${formatBytes(m.free_hdd_bytes)} / ${formatBytes(m.total_hdd_bytes)}`,
      uptime: formatUptime(m.uptime_seconds),
    })),
  );

  const metricColumns = $derived([
    { key: 'ts', label: $t('network.router.time'), class: 'mono' },
    { key: 'cpu', label: $t('network.router.cpu'), class: 'mono', align: 'right' as const, width: '90px' },
    { key: 'mem', label: $t('network.router.memory'), class: 'mono', align: 'right' as const, width: '220px' },
    { key: 'disk', label: $t('network.router.disk'), class: 'mono', align: 'right' as const, width: '220px' },
    { key: 'uptime', label: $t('network.router.uptime'), class: 'mono', align: 'right' as const, width: '120px' },
  ]);

  const selectedInterfaceRxRate = $derived.by(() =>
    selectedInterface ? formatBps(ifaceRates[selectedInterface]?.rx_bps ?? null) : '—',
  );
  const selectedInterfaceTxRate = $derived.by(() =>
    selectedInterface ? formatBps(ifaceRates[selectedInterface]?.tx_bps ?? null) : '—',
  );
  const selectedInterfaceRxSeries = $derived.by(() =>
    ifaceHistory
      .slice()
      .reverse()
      .map((x) => (typeof x.rx_bps === 'number' ? x.rx_bps : null))
      .filter((v) => v != null) as number[],
  );
  const selectedInterfaceTxSeries = $derived.by(() =>
    ifaceHistory
      .slice()
      .reverse()
      .map((x) => (typeof x.tx_bps === 'number' ? x.tx_bps : null))
      .filter((v) => v != null) as number[],
  );
</script>

<div class="page-content fade-in">
  <NetworkPageHeader
    title={$t('admin.network.routers.title')}
    subtitle={router
      ? `${router.name} • ${router.host}:${router.port}`
      : $t('admin.network.routers.subtitle') || 'Manage MikroTik routers and monitoring'}
  >
    {#snippet actions()}
      <button
        class="back"
        type="button"
        onclick={() => goto(routerBackTarget)}
      >
        <Icon name="arrow-left" size={16} />
        {$t('common.back')}
      </button>

      <button
        class="btn ghost"
        type="button"
        onclick={() => refresh()}
        title={$t('common.refresh')}
      >
        <Icon name="refresh-cw" size={16} />
        {$t('admin.network.routers.actions.refresh') || $t('common.refresh') || 'Refresh'}
      </button>
      <button class="btn" type="button" onclick={test} disabled={!router}>
        <Icon name="zap" size={16} />
        {$t('admin.network.routers.actions.test')}
      </button>
    {/snippet}
  </NetworkPageHeader>

  {#if initialLoading}
    <div class="skeleton">
      <div class="line"></div>
      <div class="line"></div>
      <div class="line"></div>
    </div>
  {:else if router}
    <div class="hero">
      <div class="hero-left">
        <div class="kicker">
          <span class="dot" class:online={router.is_online}></span>
          <span class="k">{$t('network.router.mikrotik')}</span>
          {#if router.maintenance_until && new Date(router.maintenance_until).getTime() > Date.now()}
            <span class="chip warn" title={router.maintenance_reason || ''}>
              {$t('admin.network.routers.badges.maintenance')}
            </span>
          {/if}
        </div>
        <h1 class="title">{router.name}</h1>
        <div class="meta">
          <span class="mono">{router.username}@{router.host}:{router.port}</span>
          {#if router.identity}
            <span class="sep">·</span>
            <span class="chip">{router.identity}</span>
          {/if}
          {#if router.ros_version}
            <span class="sep">·</span>
            <span class="muted">RouterOS {router.ros_version}</span>
          {/if}
        </div>
        {#if router.last_error}
          <div class="alert">
            <Icon name="alert-triangle" size={16} />
            <span>{router.last_error}</span>
          </div>
        {/if}
      </div>

      <div class="hero-right">
        {#if $can('manage', 'router_inventory')}
          <button class="btn ghost btn-sm hero-action" type="button" onclick={openManagedRadiusModal}>
            <Icon name="shield" size={14} />
            {$t('admin.network.routers.managed_radius.trigger.label')}
          </button>
        {/if}

        <div class="badge" class:online={router.is_online} class:offline={!router.is_online}>
          {statusLabel()}
        </div>
        <div class="hint">
          {#if refreshing}
            <span class="spin"><Icon name="refresh-cw" size={14} /></span>
            <span class="muted">{$t('common.loading')}</span>
          {:else}
            <span class="muted">{$t('common.updated')}</span>
          {/if}
        </div>
        <div class="kv">
          <div class="kv-item">
            <span class="kv-label">{$t('network.router.latency')}</span>
            <span class="kv-value mono">{router.latency_ms ?? '—'} ms</span>
          </div>
          <div class="kv-item">
            <span class="kv-label">{$t('network.router.last_seen')}</span>
            {#if router.last_seen_at}
              <span
                class="kv-value"
                title={formatDateTime(router.last_seen_at, { timeZone: $appSettings.app_timezone })}
              >
                {timeAgo(router.last_seen_at)}
              </span>
            {:else}
              <span class="kv-value">—</span>
            {/if}
          </div>
        </div>
      </div>
    </div>

    <ResponsiveTabs
      items={routerTabItems}
      bind:activeId={activeTab}
      {isMobile}
      priorityCount={3}
      ariaLabel="Router detail tabs"
    />

    {#if activeTab === 'overview'}
      <div class="grid">
        <div class="card">
          <div class="card-head">
            <h2>CPU</h2>
            <span class="muted">{$t('network.router.last_120_samples')}</span>
          </div>
          <div class="spark">
            {#if cpuSeries.length === 0}
              <div class="muted">{$t('network.router.no_metrics_yet')}</div>
            {:else}
              {#each cpuSeries as v}
                <div class="bar" style={`height:${v}%;`} title={`${v}%`}></div>
              {/each}
            {/if}
          </div>
        </div>

        <div class="card">
          <div class="card-head">
            <h2>{$t('network.router.resources')}</h2>
            <span class="muted">{$t('network.router.live_snapshot')}</span>
          </div>

          {#if snapshot}
            {@const memUsed = pctUsed(snapshot.total_memory_bytes, snapshot.free_memory_bytes)}
            {@const diskUsed = pctUsed(snapshot.total_hdd_bytes, snapshot.free_hdd_bytes)}

            <div class="rows">
              <div class="row">
                <span class="muted">{$t('network.router.cpu_load')}</span>
                <span class="mono">{snapshot.cpu_load ?? '—'}%</span>
              </div>
              <div class="row">
                <span class="muted">{$t('network.router.memory_used')}</span>
                <span class="mono">{memUsed == null ? '—' : `${memUsed}%`}</span>
              </div>
              <div class="row">
                <span class="muted">{$t('network.router.disk_used')}</span>
                <span class="mono">{diskUsed == null ? '—' : `${diskUsed}%`}</span>
              </div>
              <div class="row">
                <span class="muted">{$t('network.router.uptime')}</span>
                <span class="mono">{formatUptime(snapshot.uptime_seconds)}</span>
              </div>
              <div class="row">
                <span class="muted">{$t('network.router.memory')}</span>
                <span class="mono"
                  >{formatBytes(snapshot.free_memory_bytes)} / {formatBytes(
                    snapshot.total_memory_bytes,
                  )}</span
                >
              </div>
              <div class="row">
                <span class="muted">{$t('network.router.disk')}</span>
                <span class="mono"
                  >{formatBytes(snapshot.free_hdd_bytes)} / {formatBytes(
                    snapshot.total_hdd_bytes,
                  )}</span
                >
              </div>
            </div>
          {:else}
            <div class="muted">{$t('network.router.no_snapshot_yet')}</div>
          {/if}
        </div>
      </div>

      {#if snapshot}
        <div class="grid2">
          <div class="card">
            <div class="card-head">
              <h2>{$t('network.router.hardware')}</h2>
              <span class="muted">{$t('network.router.live_snapshot')}</span>
            </div>
            <div class="rows">
              <div class="row">
                <span class="muted">{$t('network.router.board')}</span>
                <span class="mono">{snapshot.board_name || '—'}</span>
              </div>
              <div class="row">
                <span class="muted">{$t('network.router.architecture')}</span>
                <span class="mono">{snapshot.architecture || '—'}</span>
              </div>
              <div class="row">
                <span class="muted">{$t('network.router.cpu')}</span>
                <span class="mono">{snapshot.cpu || '—'}</span>
              </div>
            </div>
          </div>

          <div class="card">
            <div class="card-head">
              <h2>{$t('network.router.health')}</h2>
              <span class="muted">{$t('network.router.optional')}</span>
            </div>
            {#if snapshot.health}
              <div class="rows">
                <div class="row">
                  <span class="muted">{$t('network.router.temperature')}</span>
                  <span class="mono">{snapshot.health.temperature_c ?? '—'} °C</span>
                </div>
                <div class="row">
                  <span class="muted">{$t('network.router.cpu_temperature')}</span>
                  <span class="mono">{snapshot.health.cpu_temperature_c ?? '—'} °C</span>
                </div>
                <div class="row">
                  <span class="muted">{$t('network.router.voltage')}</span>
                  <span class="mono">{snapshot.health.voltage_v ?? '—'} V</span>
                </div>
              </div>
            {:else}
              <div class="muted">{$t('network.router.health_not_supported')}</div>
            {/if}
          </div>
        </div>
      {/if}

    {:else if activeTab === 'interfaces'}
      <div class="card full">
        <div class="card-head">
          <h2>{$t('network.router.tab_interfaces')}</h2>
          <span class="muted">{interfaceRows.length} {$t('common.shown')}</span>
        </div>

        <div class="seg">
          <button
            type="button"
            class="seg-btn {ifFilter === 'all' ? 'active' : ''}"
            onclick={() => (ifFilter = 'all')}
          >
            {$t('common.all')}
          </button>
          <button
            type="button"
            class="seg-btn {ifFilter === 'running' ? 'active' : ''}"
            onclick={() => (ifFilter = 'running')}
          >
            {$t('common.running')}
          </button>
          <button
            type="button"
            class="seg-btn {ifFilter === 'down' ? 'active' : ''}"
            onclick={() => (ifFilter = 'down')}
          >
            {$t('common.down')}
          </button>
          <button
            type="button"
            class="seg-btn {ifFilter === 'disabled' ? 'active' : ''}"
            onclick={() => (ifFilter = 'disabled')}
          >
            {$t('common.disabled')}
          </button>
        </div>

        <div class="table-wrap">
          <Table
            columns={interfaceColumns}
            data={interfaceTableData}
            keyField="id"
            pagination={true}
            pageSize={10}
            searchable={true}
            searchPlaceholder={$t('network.router.search_interfaces')}
            mobileView={isMobile ? 'card' : 'scroll'}
          >
            {#snippet cell({ item, key }: any)}
              {#if key === 'name'}
                <button
                  class="link"
                  type="button"
                  ondblclick={() => openInterface(item.name)}
                  title={$t('network.router.double_click_open')}
                >
                  <span class="mono">{item.name}</span>
                </button>
              {:else if key === 'status'}
                {#if item.status === 'disabled'}
                  <span class="pill off">{$t('common.disabled')}</span>
                {:else if item.status === 'running'}
                  <span class="pill ok">{$t('common.running')}</span>
                {:else}
                  <span class="pill warn">{$t('common.down')}</span>
                {/if}
              {:else}
                {item[key] ?? ''}
              {/if}
            {/snippet}
          </Table>
        </div>
      </div>
    {:else if activeTab === 'ip'}
      <div class="card full">
        <div class="card-head">
          <h2>{$t('network.router.tab_ip_addresses')}</h2>
          <span class="muted">{ipRows.length} {$t('common.total')}</span>
        </div>
        <div class="table-wrap">
          <Table
            columns={ipColumns}
            data={ipTableData}
            keyField="id"
            pagination={true}
            pageSize={10}
            searchable={true}
            searchPlaceholder={$t('network.router.search_ips')}
            mobileView={isMobile ? 'card' : 'scroll'}
          >
            {#snippet cell({ item, key }: any)}
              {#if key === 'flags'}
                <div class="flag-row">
                  {#if item.dynamic}
                    <span class="pill info">{$t('common.dynamic')}</span>
                  {/if}
                  {#if item.disabled}
                    <span class="pill off">{$t('common.disabled')}</span>
                  {/if}
                  {#if !item.dynamic && !item.disabled}
                    <span class="muted">—</span>
                  {/if}
                </div>
              {:else}
                {item[key] ?? ''}
              {/if}
            {/snippet}
          </Table>
        </div>
      </div>
    {:else if activeTab === 'metrics'}
      <div class="card full">
        <div class="card-head">
          <h2>{$t('network.router.tab_metrics')}</h2>
          <span class="muted">{metrics.length} {$t('network.router.samples')}</span>
        </div>
        <div class="table-wrap">
          <Table
            columns={metricColumns}
            data={metricRows}
            keyField="id"
            pagination={true}
            pageSize={25}
            searchable={true}
            searchPlaceholder={$t('network.router.search_metrics')}
            mobileView={isMobile ? 'card' : 'scroll'}
          />
        </div>
      </div>
    {/if}
  {:else}
    <div class="empty">
      <Icon name="alert-circle" size={18} />
      {$t('network.router.not_found')}
    </div>
  {/if}
</div>

{#if RouterDetailDialogsComponent}
  <RouterDetailDialogsComponent
    bind:showManagedRadiusModal
    {managedRadiusSetupLoading}
    {managedRadiusSetup}
    {canRevealManagedRadiusSecret}
    bind:showManagedRadiusSecret
    {assigningManagedRadiusDefault}
    {creatingManagedRadiusMapping}
    {applyingManagedRadius}
    {copyManagedRadiusSecret}
    {copyManagedRadiusScript}
    {assignManagedRadiusDefault}
    {createManagedRadiusMapping}
    {applyManagedRadius}
    bind:showInterfaceTrafficModal
    {selectedInterface}
    {ifaceHistoryLoading}
    ifaceHistoryLength={ifaceHistory.length}
    rxSeries={selectedInterfaceRxSeries}
    txSeries={selectedInterfaceTxSeries}
    {selectedInterfaceRxRate}
    {selectedInterfaceTxRate}
    {formatBps}
    {closeManagedRadiusModal}
    {closeInterfaceTrafficModal}
  />
{/if}

<style>
  .page-content {
    padding: 28px;
  }

  .back {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--color-primary);
    color: white;
    font-weight: 800;
    cursor: pointer;
  }

  .btn.ghost {
    background: transparent;
    color: var(--text-primary);
  }

  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .btn-sm {
    padding: 8px 12px;
    border-radius: 10px;
    font-size: 0.85rem;
  }

  .hero {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 14px 14px 13px;
    display: grid;
    grid-template-columns: 1.4fr 0.6fr;
    gap: 12px;
    margin-bottom: 14px;
  }

  .kicker {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
    font-weight: 900;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-size: 0.72rem;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: rgba(239, 68, 68, 0.9);
    box-shadow: 0 0 0 6px rgba(239, 68, 68, 0.12);
  }

  .dot.online {
    background: rgba(34, 197, 94, 0.9);
    box-shadow: 0 0 0 6px rgba(34, 197, 94, 0.12);
  }

  .title {
    margin: 8px 0 6px;
    font-size: 1.7rem;
    color: var(--text-primary);
  }

  .meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    color: var(--text-secondary);
  }

  .mono {
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
    color: var(--text-primary);
  }

  .sep {
    opacity: 0.6;
  }

  .chip {
    font-size: 0.72rem;
    font-weight: 800;
    padding: 3px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg-hover), transparent 20%);
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
  }

  .chip.warn {
    border-color: rgba(245, 158, 11, 0.28);
    background: rgba(245, 158, 11, 0.12);
    color: rgba(245, 158, 11, 0.95);
  }

  .alert {
    margin-top: 10px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 14px;
    border: 1px solid rgba(239, 68, 68, 0.28);
    background: rgba(239, 68, 68, 0.1);
    color: rgba(239, 68, 68, 0.95);
    font-weight: 700;
  }

  .hero-right {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
  }

  .hero-action {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .hint {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 10%);
  }

  .spin {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    animation: spin 1.1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-radius: 999px;
    font-weight: 900;
    font-size: 0.78rem;
    border: 1px solid var(--border-color);
    background: rgba(239, 68, 68, 0.12);
    color: rgba(239, 68, 68, 0.95);
    border-color: rgba(239, 68, 68, 0.28);
  }

  .badge.online {
    background: rgba(34, 197, 94, 0.12);
    color: rgba(34, 197, 94, 0.95);
    border-color: rgba(34, 197, 94, 0.28);
  }

  .kv {
    width: 100%;
    display: grid;
    gap: 10px;
    padding-top: 6px;
  }

  .kv-item {
    background: color-mix(in srgb, var(--bg-surface), transparent 8%);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 10px 12px;
  }

  .kv-label {
    display: block;
    color: var(--text-secondary);
    font-size: 0.8rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .kv-value {
    display: block;
    margin-top: 6px;
    color: var(--text-primary);
    font-weight: 900;
  }

  .grid {
    display: grid;
    grid-template-columns: 1.1fr 0.9fr;
    gap: 12px;
  }

  .grid2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    margin-top: 12px;
  }

  .seg {
    margin: 10px 0 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .seg-btn {
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-secondary);
    padding: 8px 10px;
    border-radius: 999px;
    font-weight: 900;
    cursor: pointer;
  }

  .seg-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .seg-btn.active {
    border-color: rgba(99, 102, 241, 0.35);
    background: rgba(99, 102, 241, 0.12);
    color: var(--text-primary);
  }

  .card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 14px;
  }

  .card.full {
    margin-top: 12px;
  }

  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    margin-bottom: 10px;
  }

  h2 {
    margin: 0;
    color: var(--text-primary);
    font-size: 1rem;
  }

  .muted {
    color: var(--text-secondary);
  }

  .spark {
    height: 140px;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    gap: 2px;
    align-items: end;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    padding: 10px;
    background: color-mix(in srgb, var(--bg-surface), transparent 8%);
    overflow: hidden;
  }

  .bar {
    width: 100%;
    background: rgba(99, 102, 241, 0.72);
    border-radius: 6px 6px 2px 2px;
    opacity: 0.95;
  }

  .rows {
    display: grid;
    gap: 10px;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 12px;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 8%);
  }

  .skeleton {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 14px;
  }

  .line {
    height: 14px;
    background: var(--bg-hover);
    border-radius: 10px;
    margin-bottom: 10px;
  }

  .empty {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-secondary);
  }

  .table-wrap {
    overflow: auto;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 8%);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    padding: 4px 8px;
    border-radius: 999px;
    font-weight: 900;
    font-size: 0.72rem;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-hover), transparent 15%);
    color: var(--text-secondary);
  }

  .pill.ok {
    border-color: rgba(34, 197, 94, 0.28);
    background: rgba(34, 197, 94, 0.12);
    color: rgba(34, 197, 94, 0.95);
  }

  .pill.warn {
    border-color: rgba(245, 158, 11, 0.35);
    background: rgba(245, 158, 11, 0.12);
    color: rgba(245, 158, 11, 0.95);
  }

  .pill.off {
    border-color: rgba(148, 163, 184, 0.28);
    background: rgba(148, 163, 184, 0.12);
    color: rgba(148, 163, 184, 0.95);
  }

  .pill.info {
    border-color: rgba(99, 102, 241, 0.28);
    background: rgba(99, 102, 241, 0.12);
    color: rgba(99, 102, 241, 0.95);
  }

  .flag-row {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .link {
    border: none;
    background: transparent;
    padding: 0;
    cursor: pointer;
    text-align: left;
  }

  .link:hover {
    text-decoration: underline;
  }

  @media (max-width: 900px) {
    .page-content {
      padding: 18px;
    }

    .hero {
      grid-template-columns: 1fr;
    }

    .hero-right {
      align-items: flex-start;
    }

    .grid {
      grid-template-columns: 1fr;
    }

    .grid2 {
      grid-template-columns: 1fr;
    }
  }
</style>
