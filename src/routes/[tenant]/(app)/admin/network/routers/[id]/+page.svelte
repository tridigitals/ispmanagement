<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';

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
  let ifaceHistoryLoading = $state(false);
  let ifaceHistory = $state<any[]>([]);

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
  let pppLoadedFor = $state<string | null>(null);
  let poolsLoadedFor = $state<string | null>(null);

  let cpuSeries = $derived.by(() => {
    const pts = metrics
      .slice()
      .reverse()
      .map((m) => (m.cpu_load == null ? null : Math.max(0, Math.min(100, m.cpu_load))));
    return pts.filter((v) => v != null) as number[];
  });

  let activeTab = $state<'overview' | 'interfaces' | 'ip' | 'metrics'>('overview');
  let ifFilter = $state<'all' | 'running' | 'down' | 'disabled'>('all');

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
    if (!$can('read', 'network_routers') && !$can('manage', 'network_routers')) {
      goto('/unauthorized');
      return;
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
              if (rx != null && rx >= prev.rx) rx_bps = Math.round(((rx - prev.rx) * 8 * 1000) / dt);
              if (tx != null && tx >= prev.tx) tx_bps = Math.round(((tx - prev.tx) * 8 * 1000) / dt);
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
      toast.success($t('admin.network.routers.ppp_profiles.toasts.synced') || 'Synced PPP profiles');
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
    selectedInterface = name;
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
    { key: 'name', label: 'Name' },
    { key: 'type', label: 'Type' },
    { key: 'status', label: 'Status' },
    { key: 'mtu', label: 'MTU', align: 'right' as const, width: '90px' },
    { key: 'mac', label: 'MAC', class: 'mono' },
    { key: 'rx_rate', label: 'RX Rate', class: 'mono', align: 'right' as const, width: '130px' },
    { key: 'tx_rate', label: 'TX Rate', class: 'mono', align: 'right' as const, width: '130px' },
    { key: 'rx', label: 'RX', class: 'mono', align: 'right' as const, width: '120px' },
    { key: 'tx', label: 'TX', class: 'mono', align: 'right' as const, width: '120px' },
    { key: 'downs', label: 'Downs', class: 'mono', align: 'right' as const, width: '90px' },
    { key: 'actions', label: '', align: 'right' as const, width: '60px' },
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
    { key: 'address', label: 'Address', class: 'mono' },
    { key: 'interface', label: 'Interface' },
    { key: 'network', label: 'Network', class: 'mono' },
    { key: 'flags', label: 'Flags' },
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
    { key: 'ts', label: 'Time', class: 'mono' },
    { key: 'cpu', label: 'CPU', class: 'mono', align: 'right' as const, width: '90px' },
    { key: 'mem', label: 'Memory', class: 'mono', align: 'right' as const, width: '220px' },
    { key: 'disk', label: 'Disk', class: 'mono', align: 'right' as const, width: '220px' },
    { key: 'uptime', label: 'Uptime', class: 'mono', align: 'right' as const, width: '120px' },
  ]);
</script>

<div class="page-content fade-in">
  <div class="head">
    <button
      class="back"
      type="button"
      onclick={() => goto($page.url.pathname.replace(/\/[^/]+\/?$/, ''))}
    >
      <Icon name="arrow-left" size={16} />
      {$t('common.back') || 'Back'}
    </button>

    <div class="head-actions">
      <button
        class="btn ghost"
        type="button"
        onclick={() => refresh()}
        title={$t('common.refresh') || 'Refresh'}
      >
        <Icon name="refresh-cw" size={16} />
        {$t('admin.network.routers.actions.refresh') || $t('common.refresh') || 'Refresh'}
      </button>
      <button class="btn" type="button" onclick={test} disabled={!router}>
        <Icon name="zap" size={16} />
        {$t('admin.network.routers.actions.test') || 'Test Connection'}
      </button>
    </div>
  </div>

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
          <span class="k">MikroTik</span>
          {#if router.maintenance_until && new Date(router.maintenance_until).getTime() > Date.now()}
            <span class="chip warn" title={router.maintenance_reason || ''}>
              {$t('admin.network.routers.badges.maintenance') || 'Maintenance'}
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
        <div class="badge" class:online={router.is_online} class:offline={!router.is_online}>
          {statusLabel()}
        </div>
        <div class="hint">
          {#if refreshing}
            <span class="spin"><Icon name="refresh-cw" size={14} /></span>
            <span class="muted">{$t('common.loading') || 'Loading...'}</span>
          {:else}
            <span class="muted">{$t('common.updated') || 'Updated'}</span>
          {/if}
        </div>
        <div class="kv">
          <div class="kv-item">
            <span class="kv-label">Latency</span>
            <span class="kv-value mono">{router.latency_ms ?? '—'} ms</span>
          </div>
          <div class="kv-item">
            <span class="kv-label">Last seen</span>
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

    <div class="tabs">
      <button
        type="button"
        class="tab {activeTab === 'overview' ? 'active' : ''}"
        onclick={() => (activeTab = 'overview')}
      >
        <Icon name="activity" size={16} />
        Overview
      </button>
      <button
        type="button"
        class="tab {activeTab === 'interfaces' ? 'active' : ''}"
        onclick={() => (activeTab = 'interfaces')}
      >
        <Icon name="router" size={16} />
        Interfaces
        {#if snapshot?.interfaces?.length}
          <span class="tab-count">{snapshot.interfaces.length}</span>
        {/if}
      </button>
      <button
        type="button"
        class="tab {activeTab === 'ip' ? 'active' : ''}"
        onclick={() => (activeTab = 'ip')}
      >
        <Icon name="map-pin" size={16} />
        IP Addresses
        {#if snapshot?.ip_addresses?.length}
          <span class="tab-count">{snapshot.ip_addresses.length}</span>
        {/if}
      </button>
      <button
        type="button"
        class="tab {activeTab === 'metrics' ? 'active' : ''}"
        onclick={() => (activeTab = 'metrics')}
      >
        <Icon name="trending-up" size={16} />
        Metrics
      </button>
    </div>

    {#if activeTab === 'overview'}
      <div class="grid">
        <div class="card">
          <div class="card-head">
            <h2>CPU</h2>
            <span class="muted">Last 120 samples</span>
          </div>
          <div class="spark">
            {#if cpuSeries.length === 0}
              <div class="muted">No metrics yet.</div>
            {:else}
              {#each cpuSeries as v}
                <div class="bar" style={`height:${v}%;`} title={`${v}%`}></div>
              {/each}
            {/if}
          </div>
        </div>

        <div class="card">
          <div class="card-head">
            <h2>Resources</h2>
            <span class="muted">Live snapshot</span>
          </div>

          {#if snapshot}
            {@const memUsed = pctUsed(snapshot.total_memory_bytes, snapshot.free_memory_bytes)}
            {@const diskUsed = pctUsed(snapshot.total_hdd_bytes, snapshot.free_hdd_bytes)}

            <div class="rows">
              <div class="row">
                <span class="muted">CPU load</span>
                <span class="mono">{snapshot.cpu_load ?? '—'}%</span>
              </div>
              <div class="row">
                <span class="muted">Memory used</span>
                <span class="mono">{memUsed == null ? '—' : `${memUsed}%`}</span>
              </div>
              <div class="row">
                <span class="muted">Disk used</span>
                <span class="mono">{diskUsed == null ? '—' : `${diskUsed}%`}</span>
              </div>
              <div class="row">
                <span class="muted">Uptime</span>
                <span class="mono">{formatUptime(snapshot.uptime_seconds)}</span>
              </div>
              <div class="row">
                <span class="muted">Memory</span>
                <span class="mono"
                  >{formatBytes(snapshot.free_memory_bytes)} / {formatBytes(
                    snapshot.total_memory_bytes,
                  )}</span
                >
              </div>
              <div class="row">
                <span class="muted">Disk</span>
                <span class="mono"
                  >{formatBytes(snapshot.free_hdd_bytes)} / {formatBytes(snapshot.total_hdd_bytes)}</span
                >
              </div>
            </div>
          {:else}
            <div class="muted">No snapshot yet.</div>
          {/if}
        </div>
      </div>

      {#if snapshot}
        <div class="grid2">
          <div class="card">
            <div class="card-head">
              <h2>Hardware</h2>
              <span class="muted">Live</span>
            </div>
            <div class="rows">
              <div class="row">
                <span class="muted">Board</span>
                <span class="mono">{snapshot.board_name || '—'}</span>
              </div>
              <div class="row">
                <span class="muted">Architecture</span>
                <span class="mono">{snapshot.architecture || '—'}</span>
              </div>
              <div class="row">
                <span class="muted">CPU</span>
                <span class="mono">{snapshot.cpu || '—'}</span>
              </div>
            </div>
          </div>

          <div class="card">
            <div class="card-head">
              <h2>Health</h2>
              <span class="muted">Optional</span>
            </div>
            {#if snapshot.health}
              <div class="rows">
                <div class="row">
                  <span class="muted">Temperature</span>
                  <span class="mono">{snapshot.health.temperature_c ?? '—'} °C</span>
                </div>
                <div class="row">
                  <span class="muted">CPU temperature</span>
                  <span class="mono">{snapshot.health.cpu_temperature_c ?? '—'} °C</span>
                </div>
                <div class="row">
                  <span class="muted">Voltage</span>
                  <span class="mono">{snapshot.health.voltage_v ?? '—'} V</span>
                </div>
              </div>
            {:else}
              <div class="muted">Not supported on this device.</div>
            {/if}
          </div>
        </div>
      {/if}
    {:else if activeTab === 'interfaces'}
      <div class="card full">
        <div class="card-head">
          <h2>Interfaces</h2>
          <span class="muted">{interfaceRows.length} shown</span>
        </div>

        <div class="seg">
          <button
            type="button"
            class="seg-btn {ifFilter === 'all' ? 'active' : ''}"
            onclick={() => (ifFilter = 'all')}
          >
            All
          </button>
          <button
            type="button"
            class="seg-btn {ifFilter === 'running' ? 'active' : ''}"
            onclick={() => (ifFilter = 'running')}
          >
            Running
          </button>
          <button
            type="button"
            class="seg-btn {ifFilter === 'down' ? 'active' : ''}"
            onclick={() => (ifFilter = 'down')}
          >
            Down
          </button>
          <button
            type="button"
            class="seg-btn {ifFilter === 'disabled' ? 'active' : ''}"
            onclick={() => (ifFilter = 'disabled')}
          >
            Disabled
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
            searchPlaceholder="Search interfaces..."
            mobileView={isMobile ? 'card' : 'scroll'}
          >
            {#snippet cell({ item, key }: any)}
              {#if key === 'name'}
                <button class="link" type="button" onclick={() => openInterface(item.name)}>
                  <span class="mono">{item.name}</span>
                </button>
              {:else if key === 'status'}
                {#if item.status === 'disabled'}
                  <span class="pill off">Disabled</span>
                {:else if item.status === 'running'}
                  <span class="pill ok">Running</span>
                {:else}
                  <span class="pill warn">Down</span>
                {/if}
              {:else if key === 'actions'}
                <button
                  class="icon-btn"
                  type="button"
                  onclick={() => openInterface(item.name)}
                  title="Open"
                >
                  <Icon name="arrow-right" size={16} />
                </button>
              {:else}
                {item[key] ?? ''}
              {/if}
            {/snippet}
          </Table>
        </div>
      </div>

      {#if selectedInterface}
        <div class="card full">
          <div class="card-head">
            <h2>Traffic History</h2>
            <span class="muted mono">{selectedInterface}</span>
          </div>
          {#if ifaceHistoryLoading}
            <div class="muted">{$t('common.loading') || 'Loading...'}</div>
          {:else if ifaceHistory.length === 0}
            <div class="muted">No history yet (wait for poller).</div>
          {:else}
            {@const rxSeries = ifaceHistory
              .slice()
              .reverse()
              .map((x) => (typeof x.rx_bps === 'number' ? x.rx_bps : null))
              .filter((v) => v != null) as number[]}
            {@const txSeries = ifaceHistory
              .slice()
              .reverse()
              .map((x) => (typeof x.tx_bps === 'number' ? x.tx_bps : null))
              .filter((v) => v != null) as number[]}

            <div class="traffic-grid">
              <div class="traffic-card">
                <div class="traffic-top">
                  <span class="muted">RX</span>
                  <span class="mono">{formatBps(ifaceRates[selectedInterface]?.rx_bps ?? null)}</span>
                </div>
                <div class="spark small">
                  {#if rxSeries.length === 0}
                    <div class="muted">No RX samples.</div>
                  {:else}
                    {@const max = Math.max(...rxSeries, 1)}
                    {#each rxSeries as v}
                      <div class="bar rx" style={`height:${Math.round((v / max) * 100)}%;`} title={formatBps(v)}></div>
                    {/each}
                  {/if}
                </div>
              </div>

              <div class="traffic-card">
                <div class="traffic-top">
                  <span class="muted">TX</span>
                  <span class="mono">{formatBps(ifaceRates[selectedInterface]?.tx_bps ?? null)}</span>
                </div>
                <div class="spark small">
                  {#if txSeries.length === 0}
                    <div class="muted">No TX samples.</div>
                  {:else}
                    {@const max = Math.max(...txSeries, 1)}
                    {#each txSeries as v}
                      <div class="bar tx" style={`height:${Math.round((v / max) * 100)}%;`} title={formatBps(v)}></div>
                    {/each}
                  {/if}
                </div>
              </div>
            </div>
          {/if}
        </div>
      {/if}
    {:else if activeTab === 'ip'}
      <div class="card full">
        <div class="card-head">
          <h2>IP Addresses</h2>
          <span class="muted">{ipRows.length} total</span>
        </div>
        <div class="table-wrap">
          <Table
            columns={ipColumns}
            data={ipTableData}
            keyField="id"
            pagination={true}
            pageSize={10}
            searchable={true}
            searchPlaceholder="Search IPs..."
            mobileView={isMobile ? 'card' : 'scroll'}
          >
            {#snippet cell({ item, key }: any)}
              {#if key === 'flags'}
                <div class="flag-row">
                  {#if item.dynamic}
                    <span class="pill info">Dynamic</span>
                  {/if}
                  {#if item.disabled}
                    <span class="pill off">Disabled</span>
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
          <h2>Metrics</h2>
          <span class="muted">{metrics.length} samples</span>
        </div>
        <div class="table-wrap">
          <Table
            columns={metricColumns}
            data={metricRows}
            keyField="id"
            pagination={true}
            pageSize={25}
            searchable={true}
            searchPlaceholder="Search metrics..."
            mobileView={isMobile ? 'card' : 'scroll'}
          />
        </div>
      </div>
    {/if}
  {:else}
    <div class="empty">
      <Icon name="alert-circle" size={18} />
      Router not found.
    </div>
  {/if}
</div>

<style>
  .page-content {
    padding: 28px;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    margin-bottom: 16px;
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

  .head-actions {
    display: flex;
    gap: 10px;
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

  .hero {
    background: radial-gradient(1200px 700px at 0% 0%, rgba(99, 102, 241, 0.18), transparent 55%),
      radial-gradient(1000px 600px at 100% 0%, rgba(34, 197, 94, 0.12), transparent 55%),
      var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 22px;
    padding: 18px 18px 16px;
    display: grid;
    grid-template-columns: 1.4fr 0.6fr;
    gap: 16px;
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
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
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
    gap: 10px;
  }

  .hint {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card), transparent 10%);
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
    background: color-mix(in srgb, var(--bg-card), transparent 8%);
    border: 1px solid var(--border-color);
    border-radius: 16px;
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

  .tabs {
    margin-top: 12px;
    margin-bottom: 12px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: var(--bg-card);
    color: var(--text-secondary);
    font-weight: 900;
    cursor: pointer;
    transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease;
  }

  .tab:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .tab.active {
    border-color: rgba(99, 102, 241, 0.35);
    background: rgba(99, 102, 241, 0.12);
    color: var(--text-primary);
  }

  .tab-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 28px;
    height: 22px;
    padding: 0 8px;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card), transparent 8%);
    color: var(--text-secondary);
    font-size: 0.78rem;
    font-weight: 900;
    margin-left: 2px;
  }

  .seg {
    margin: 10px 0 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .live-grid {
    grid-template-columns: 0.9fr 1.1fr;
    align-items: start;
  }

  .watch-toolbar {
    display: flex;
    gap: 10px;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
    flex-wrap: wrap;
  }

  .search.small {
    padding: 8px 10px;
    min-width: min(380px, 100%);
  }

  .watch-list {
    display: grid;
    gap: 8px;
    max-height: 520px;
    overflow: auto;
    padding-right: 4px;
  }

  .watch-item {
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    border-radius: 14px;
    padding: 10px 12px;
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }

  .watch-item:hover {
    background: var(--bg-hover);
  }

  .watch-item.on {
    border-color: rgba(99, 102, 241, 0.35);
    background: rgba(99, 102, 241, 0.12);
  }

  .spacer {
    flex: 1;
  }

  .live-cards {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .live-card {
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card), transparent 6%);
    border-radius: 18px;
    padding: 12px;
  }

  .live-top {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 10px;
  }

  .chip.small {
    padding: 2px 8px;
    font-weight: 900;
  }

  .bars {
    height: 48px;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    gap: 2px;
    align-items: end;
    opacity: 0.95;
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
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 18px;
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
    background: color-mix(in srgb, var(--bg-card), transparent 8%);
    overflow: hidden;
  }

  .bar {
    width: 100%;
    background: linear-gradient(180deg, rgba(99, 102, 241, 0.8), rgba(34, 197, 94, 0.45));
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
    background: color-mix(in srgb, var(--bg-card), transparent 8%);
  }

  .skeleton {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 18px;
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
    border-radius: 16px;
    border: 1px solid var(--border-color);
    background: var(--bg-card);
    color: var(--text-secondary);
  }

  .table-wrap {
    overflow: auto;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card), transparent 8%);
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

  .traffic-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .traffic-card {
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card), transparent 8%);
    border-radius: 18px;
    padding: 12px;
  }

  .traffic-top {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    margin-bottom: 10px;
    font-weight: 900;
  }

  .spark.small {
    height: 120px;
  }

  .bar.rx {
    background: linear-gradient(180deg, rgba(34, 197, 94, 0.9), rgba(34, 197, 94, 0.25));
  }

  .bar.tx {
    background: linear-gradient(180deg, rgba(99, 102, 241, 0.9), rgba(99, 102, 241, 0.25));
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

  .icon-btn {
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    border-radius: 12px;
    padding: 8px;
    cursor: pointer;
    display: grid;
    place-items: center;
  }

  .icon-btn:hover {
    background: var(--bg-hover);
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

    .traffic-grid {
      grid-template-columns: 1fr;
    }

    .live-grid {
      grid-template-columns: 1fr;
    }

    .live-cards {
      grid-template-columns: 1fr;
    }
  }
</style>
