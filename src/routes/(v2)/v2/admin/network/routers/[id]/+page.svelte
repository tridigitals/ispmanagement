<script lang="ts">
  /*
    Detail router v2 — gelombang 22.

    Versi lama: (app)/admin/network/routers/[id]/+page.svelte (1.752 baris)
    + RouterDetailDialogs.svelte (493). Backend /api/admin/mikrotik/routers
    sudah konsisten (AppResult + guard izin per request, snapshot membawa
    router utuh) — gelombang ini murni redesign FE:
    - format/agregasi dipindah ke src/lib/utils/routerDetailInsights.ts
      (19 tes) — keluar dari komponen raksasa.
    - markup ds (AppShell, DetailHeader, Tabs, Card, StatTile, DataTable)
      menggantikan hero custom + Table lama.
    - polling tetap 5 detik (snapshot + metrics paralel), live bps
      dihitung dari delta counter antar polling seperti legacy.
    - dialog managed-RADIUS & grafik lalu lintas interface di-IMPORT
      dari komponen legacy RouterDetailDialogs.svelte (props-based,
      tidak ditulis ulang).
  */
  import { onDestroy, onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { resolveBackTarget } from '$lib/utils/backNavigation';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import type { ManagedRadiusRouterSetup } from '$lib/api/types';
  import {
    formatBps,
    formatBytes,
    formatUptime,
    friendlyRouterError,
    pctUsed,
    snapshotHealthStats,
  } from '$lib/utils/routerDetailInsights';
  import RouterDetailDialogs from '../../../../../../(app)/admin/network/routers/[id]/RouterDetailDialogs.svelte';
  import {
    AppShell,
    Badge,
    Button,
    Card,
    DataTable,
    DetailHeader,
    Icon,
    StatTile,
    Tabs,
    type Column,
  } from '$lib/components/ds';

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
    health?: {
      temperature_c?: number | null;
      voltage_v?: number | null;
      cpu_temperature_c?: number | null;
    } | null;
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

  const id = $derived($page.params.id || '');
  const listPath = $derived($page.url.pathname.replace(/\/[^/]+\/?$/, ''));
  const backTarget = $derived(resolveBackTarget($page.url, listPath));

  const canRead = $derived($can('read', 'router_inventory') || $can('manage', 'router_inventory'));
  const canManage = $derived($can('manage', 'router_inventory'));

  let initialLoading = $state(true);
  let refreshing = $state(false);
  let refreshInFlight = $state(false);
  let router = $state<RouterRow | null>(null);
  let snapshot = $state<RouterSnapshot | null>(null);
  let metrics = $state<MetricRow[]>([]);
  let loadError = $state<string | null>(null);

  let activeTab = $state<'overview' | 'interfaces' | 'ip' | 'metrics'>('overview');
  let ifFilter = $state<'all' | 'running' | 'down' | 'disabled'>('all');

  let refreshHandle: ReturnType<typeof setInterval> | null = null;

  // rate live dihitung dari delta counter antar polling (pola legacy)
  let ifacePrev = $state<Record<string, { rx: number | null; tx: number | null; ts: number }>>({});
  let ifaceRates = $state<Record<string, { rx_bps: number | null; tx_bps: number | null }>>({});

  const health = $derived(
    snapshotHealthStats({
      isOnline: Boolean(router?.is_online),
      cpuLoad: snapshot?.cpu_load,
      totalMemoryBytes: snapshot?.total_memory_bytes,
      freeMemoryBytes: snapshot?.free_memory_bytes,
      totalHddBytes: snapshot?.total_hdd_bytes,
      freeHddBytes: snapshot?.free_hdd_bytes,
      uptimeSeconds: snapshot?.uptime_seconds,
    }),
  );

  const cpuSeries = $derived(
    metrics
      .slice()
      .reverse()
      .map((m) => (m.cpu_load == null ? null : Math.max(0, Math.min(100, m.cpu_load))))
      .filter((v) => v != null) as number[],
  );

  const tabItems = $derived([
    { id: 'overview', label: 'Ringkasan' },
    { id: 'interfaces', label: 'Interfaces', count: snapshot?.interfaces?.length || 0 },
    { id: 'ip', label: 'IP Address', count: snapshot?.ip_addresses?.length || 0 },
    { id: 'metrics', label: 'Metrik' },
  ]);

  const interfaceCols: Column[] = [
    { key: 'name', label: 'Nama' },
    { key: 'type', label: 'Tipe' },
    { key: 'status', label: 'Status' },
    { key: 'mtu', label: 'MTU', align: 'right', width: '90px' },
    { key: 'mac', label: 'MAC', hideSm: true },
    { key: 'rx_rate', label: 'RX rate', align: 'right', width: '130px', num: true },
    { key: 'tx_rate', label: 'TX rate', align: 'right', width: '130px', num: true },
    { key: 'rx', label: 'RX', align: 'right', width: '110px', num: true, hideSm: true },
    { key: 'tx', label: 'TX', align: 'right', width: '110px', num: true, hideSm: true },
    { key: 'downs', label: 'Link down', align: 'right', width: '90px', num: true, hideSm: true },
  ];

  const interfaceRows = $derived.by(() => {
    const list = snapshot?.interfaces || [];
    const rows = list.map((it) => {
      const status = it.disabled ? 'disabled' : it.running ? 'running' : 'down';
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

  const interfaceTableRows = $derived(
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
    })),
  );

  const ipCols: Column[] = [
    { key: 'address', label: 'Alamat', num: true },
    { key: 'interface', label: 'Interface' },
    { key: 'network', label: 'Network', num: true, hideSm: true },
    { key: 'flags', label: 'Flag' },
  ];

  const ipTableRows = $derived(
    (snapshot?.ip_addresses || []).map((ip, idx) => ({
      id: `${ip.address}:${ip.interface || ''}:${idx}`,
      address: ip.address,
      interface: ip.interface || '—',
      network: ip.network || '—',
      dynamic: Boolean(ip.dynamic),
      disabled: Boolean(ip.disabled),
    })),
  );

  const metricCols: Column[] = [
    { key: 'ts', label: 'Waktu' },
    { key: 'cpu', label: 'CPU', align: 'right', width: '90px', num: true },
    { key: 'mem', label: 'Memori', align: 'right', width: '200px', num: true },
    { key: 'disk', label: 'Disk', align: 'right', width: '200px', num: true },
    { key: 'uptime', label: 'Uptime', align: 'right', width: '110px', num: true },
  ];

  const metricRows = $derived(
    metrics.map((m) => ({
      ...m,
      id: m.ts,
      ts: formatDateTime(m.ts, { timeZone: undefined }),
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

  // ---- managed radius + traffic interface (dialog legacy) ----
  let managedRadiusSetup = $state<ManagedRadiusRouterSetup | null>(null);
  let managedRadiusSetupLoading = $state(false);
  let managedRadiusLoadedFor = $state('');
  let showManagedRadiusModal = $state(false);
  let showManagedRadiusSecret = $state(false);
  let assigningManagedRadiusDefault = $state(false);
  let creatingManagedRadiusMapping = $state(false);
  let applyingManagedRadius = $state(false);

  let showInterfaceTrafficModal = $state(false);
  let selectedInterface = $state<string | null>(null);
  let ifaceHistory = $state<any[]>([]);
  let ifaceHistoryLoading = $state(false);

  const canRevealManagedRadiusSecret = $derived($can('manage_radius_secret', 'router_inventory'));

  const selectedInterfaceRxRate = $derived(
    selectedInterface ? formatBps(ifaceRates[selectedInterface]?.rx_bps ?? null) : '—',
  );
  const selectedInterfaceTxRate = $derived(
    selectedInterface ? formatBps(ifaceRates[selectedInterface]?.tx_bps ?? null) : '—',
  );
  const selectedInterfaceRxSeries = $derived(
    ifaceHistory
      .slice()
      .reverse()
      .map((x) => (typeof x.rx_bps === 'number' ? x.rx_bps : null))
      .filter((v) => v != null) as number[],
  );
  const selectedInterfaceTxSeries = $derived(
    ifaceHistory
      .slice()
      .reverse()
      .map((x) => (typeof x.tx_bps === 'number' ? x.tx_bps : null))
      .filter((v) => v != null) as number[],
  );

  onMount(() => {
    if (!canRead) {
      goto('/unauthorized');
      return;
    }
    const tab = ($page.url.searchParams.get('tab') || '').toLowerCase();
    if (tab === 'overview' || tab === 'interfaces' || tab === 'ip' || tab === 'metrics') {
      activeTab = tab as typeof activeTab;
    }
    void refresh(true);
    refreshHandle = setInterval(() => void refresh(true), 5000);
  });

  onDestroy(() => {
    if (refreshHandle) clearInterval(refreshHandle);
  });

  async function refresh(silent = false) {
    if (refreshInFlight || !id) return;
    refreshInFlight = true;
    if (!silent) {
      if (!router) initialLoading = true;
      else refreshing = true;
    }
    try {
      const [snap, m] = await Promise.all([
        api.mikrotik.routers.snapshot(id) as Promise<RouterSnapshot>,
        api.mikrotik.routers.metrics(id, 120) as Promise<MetricRow[]>,
      ]);
      snapshot = snap;
      router = snap?.router || null;
      metrics = m || [];
      loadError = null;
      if (canManage && managedRadiusLoadedFor !== id) {
        await loadManagedRadiusSetup(id, true);
      }
      // rate live dari delta counter
      if (snap?.interfaces?.length) {
        const nowMs = Date.now();
        const nextPrev = { ...ifacePrev };
        const nextRates: Record<string, { rx_bps: number | null; tx_bps: number | null }> = {};
        for (const it of snap.interfaces) {
          const rx = typeof it.rx_byte === 'number' ? it.rx_byte : null;
          const tx = typeof it.tx_byte === 'number' ? it.tx_byte : null;
          const prev = nextPrev[it.name];
          let rx_bps: number | null = null;
          let tx_bps: number | null = null;
          if (prev && prev.ts > 0) {
            const dt = nowMs - prev.ts;
            if (dt > 0) {
              if (rx != null && prev.rx != null && rx >= prev.rx)
                rx_bps = Math.round(((rx - prev.rx) * 8 * 1000) / dt);
              if (tx != null && prev.tx != null && tx >= prev.tx)
                tx_bps = Math.round(((tx - prev.tx) * 8 * 1000) / dt);
            }
          }
          nextRates[it.name] = { rx_bps, tx_bps };
          if (rx != null || tx != null) {
            nextPrev[it.name] = { rx, tx, ts: nowMs };
          }
        }
        ifacePrev = nextPrev;
        ifaceRates = nextRates;
      }
    } catch (e) {
      const msg = friendlyRouterError(extractApiErrorMessage(e));
      if (!silent) toast.error(msg);
      else loadError = msg;
    } finally {
      initialLoading = false;
      refreshing = false;
      refreshInFlight = false;
    }
  }

  async function test() {
    try {
      const res = await api.mikrotik.routers.test(id);
      const ok = Boolean((res as any)?.ok);
      if (ok) toast.success('Koneksi router OK.');
      else toast.error(String((res as any)?.error || 'Tes koneksi gagal.'));
      await refresh(true);
    } catch (e) {
      toast.error(friendlyRouterError(extractApiErrorMessage(e)));
    }
  }

  async function loadManagedRadiusSetup(routerId: string, silent = false) {
    if (!canManage) {
      managedRadiusSetup = null;
      return;
    }
    managedRadiusSetupLoading = true;
    try {
      managedRadiusSetup = (await api.mikrotik.routers.managedRadiusSetup(
        routerId,
      )) as ManagedRadiusRouterSetup;
      managedRadiusLoadedFor = routerId;
      showManagedRadiusSecret = false;
    } catch (e) {
      managedRadiusSetup = null;
      if (!silent) toast.error(friendlyRouterError(extractApiErrorMessage(e)));
    } finally {
      managedRadiusSetupLoading = false;
    }
  }

  async function copyManagedRadiusSecret() {
    const secret = managedRadiusSetup?.shared_secret;
    if (!secret) return;
    try {
      await navigator.clipboard.writeText(secret);
      toast.success('Shared secret disalin.');
    } catch (e) {
      toast.error(friendlyRouterError(extractApiErrorMessage(e)));
    }
  }

  async function copyManagedRadiusScript() {
    const script = managedRadiusSetup?.cli_script;
    if (!script) return;
    try {
      await navigator.clipboard.writeText(script);
      toast.success('CLI disalin.');
    } catch (e) {
      toast.error(friendlyRouterError(extractApiErrorMessage(e)));
    }
  }

  async function openManagedRadiusModal() {
    if (!id) return;
    showManagedRadiusModal = true;
    if (managedRadiusLoadedFor !== id || !managedRadiusSetup) {
      await loadManagedRadiusSetup(id);
    }
  }
  function closeManagedRadiusModal() {
    showManagedRadiusModal = false;
    showManagedRadiusSecret = false;
  }

  async function assignManagedRadiusDefault() {
    assigningManagedRadiusDefault = true;
    try {
      managedRadiusSetup = (await api.mikrotik.routers.assignManagedRadiusDefault(
        id,
      )) as ManagedRadiusRouterSetup;
      toast.success('Mapping default berhasil.');
    } catch (e) {
      toast.error(friendlyRouterError(extractApiErrorMessage(e)));
    } finally {
      assigningManagedRadiusDefault = false;
    }
  }
  async function createManagedRadiusMapping() {
    creatingManagedRadiusMapping = true;
    try {
      managedRadiusSetup = (await api.mikrotik.routers.createManagedRadiusMapping(
        id,
      )) as ManagedRadiusRouterSetup;
      toast.success('Mapping dibuat.');
    } catch (e) {
      toast.error(friendlyRouterError(extractApiErrorMessage(e)));
    } finally {
      creatingManagedRadiusMapping = false;
    }
  }
  async function applyManagedRadius() {
    applyingManagedRadius = true;
    try {
      managedRadiusSetup = (await api.mikrotik.routers.applyManagedRadius(
        id,
      )) as ManagedRadiusRouterSetup;
      toast.success('Konfigurasi RADIUS diterapkan.');
    } catch (e) {
      toast.error(friendlyRouterError(extractApiErrorMessage(e)));
    } finally {
      applyingManagedRadius = false;
    }
  }

  function openInterface(name: string) {
    if (!router) return;
    selectedInterface = name;
    showInterfaceTrafficModal = true;
    ifaceHistoryLoading = true;
    void api.mikrotik.routers
      .interfaceMetrics(router.id, { interface: name, limit: 120 })
      .then((rows) => {
        ifaceHistory = rows || [];
      })
      .catch((e) => toast.error(friendlyRouterError(extractApiErrorMessage(e))))
      .finally(() => {
        ifaceHistoryLoading = false;
      });
  }
  function closeInterfaceTrafficModal() {
    showInterfaceTrafficModal = false;
    selectedInterface = null;
    ifaceHistory = [];
  }

  function statusLabel(): string {
    if (!router) return '—';
    if (router.maintenance_until && new Date(router.maintenance_until).getTime() > Date.now())
      return 'Pemeliharaan';
    return router.is_online ? 'Online' : 'Offline';
  }

  const maintenance = $derived(
    router?.maintenance_until && new Date(router.maintenance_until).getTime() > Date.now(),
  );
</script><AppShell>
  {#if initialLoading && !router}
    <div class="p-8 text-center text-ink-500">Memuat router…</div>
  {:else if !router}
    <div class="p-8 text-center text-ink-500">{loadError || 'Router tidak ditemukan.'}</div>
  {:else}
    <DetailHeader
      title={router.name}
      subtitle={`${router.username}@${router.host}:${router.port}`}
      status={statusLabel()}
      statusTone={maintenance ? 'warning' : router.is_online ? 'positive' : 'negative'}
      statusLabel={statusLabel()}
      backHref={backTarget}
      meta={[
        { label: 'Latency', value: router.latency_ms == null ? '—' : `${router.latency_ms} ms` },
        {
          label: 'Terakhir online',
          value: router.last_seen_at ? timeAgo(router.last_seen_at) : '—',
        },
        { label: 'RouterOS', value: router.ros_version || '—' },
        ...(router.identity ? [{ label: 'Identity', value: router.identity }] : []),
      ]}
    >
      {#snippet actions()}
        <Button variant="ghost" icon="refresh" onclick={() => void refresh(false)} disabled={refreshing}>
          {refreshing ? 'Menyegarkan…' : 'Segarkan'}
        </Button>
        <Button variant="ghost" icon="zap" onclick={() => void test()} disabled={!router || refreshing}>
          Tes koneksi
        </Button>
        {#if canManage}
          <Button variant="secondary" onclick={() => void openManagedRadiusModal()}>RADIUS</Button>
        {/if}
      {/snippet}
    </DetailHeader>

    {#if router.last_error}
      <div class="mt-3 flex items-center gap-2 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700 ring-1 ring-red-200">
        <Icon name="alert" size={15} />
        <span>{router.last_error}</span>
      </div>
    {/if}

    <div class="mt-4 grid grid-cols-2 gap-3 lg:grid-cols-4">
      <StatTile label="Status" value={router.is_online ? 'Online' : 'Offline'} tone={router.is_online ? 'positive' : 'negative'} hint="RouterOS reachable" />
      <StatTile label="CPU" value={health.cpu == null ? '—' : `${health.cpu}%`} tone={health.cpu == null ? 'neutral' : health.cpu > 80 ? 'negative' : 'positive'} hint="beban terkini" />
      <StatTile label="Memori" value={health.memPct == null ? '—' : `${health.memPct}%`} tone={health.memPct == null ? 'neutral' : health.memPct > 85 ? 'negative' : 'positive'} hint="terpakai" />
      <StatTile label="Uptime" value={health.uptime} hint="sejak boot" />
    </div>

    <Tabs items={tabItems} active={activeTab} onselect={(id) => (activeTab = id as typeof activeTab)} />

    {#if activeTab === 'overview'}
      <div class="mt-4 grid gap-4 lg:grid-cols-2">
        <Card title="CPU">
          <div class="text-xs text-ink-500">120 sampel terakhir</div>
          {#if cpuSeries.length === 0}
            <div class="mt-4 text-sm text-ink-500">Belum ada data metrik.</div>
          {:else}
            <div class="mt-3 flex h-20 items-end gap-[2px]">
              {#each cpuSeries as v}
                <div class="w-full rounded-sm bg-ink-900" style={`height:${Math.max(3, v)}%`} title={`${v}%`}></div>
              {/each}
            </div>
          {/if}
        </Card>

        <Card title="Sumber daya">
          <dl class="mt-2 grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
            <dt class="text-ink-500">CPU load</dt>
            <dd class="font-mono text-ink-900">{snapshot?.cpu_load ?? '—'}%</dd>
            <dt class="text-ink-500">Memori terpakai</dt>
            <dd class="font-mono text-ink-900">{snapshot && pctUsed(snapshot.total_memory_bytes, snapshot.free_memory_bytes) != null ? `${pctUsed(snapshot.total_memory_bytes, snapshot.free_memory_bytes)}%` : '—'}</dd>
            <dt class="text-ink-500">Disk terpakai</dt>
            <dd class="font-mono text-ink-900">{snapshot && pctUsed(snapshot.total_hdd_bytes, snapshot.free_hdd_bytes) != null ? `${pctUsed(snapshot.total_hdd_bytes, snapshot.free_hdd_bytes)}%` : '—'}</dd>
            <dt class="text-ink-500">Uptime</dt>
            <dd class="font-mono text-ink-900">{formatUptime(snapshot?.uptime_seconds)}</dd>
          </dl>
          <div class="mt-3 grid grid-cols-2 gap-x-4 gap-y-1 border-t border-ink-100 pt-3 text-sm">
            <span class="text-ink-500">Memori</span>
            <span class="text-right font-mono text-ink-700">{snapshot ? `${formatBytes(snapshot.free_memory_bytes)} / ${formatBytes(snapshot.total_memory_bytes)}` : '—'}</span>
            <span class="text-ink-500">Disk</span>
            <span class="text-right font-mono text-ink-700">{snapshot ? `${formatBytes(snapshot.free_hdd_bytes)} / ${formatBytes(snapshot.total_hdd_bytes)}` : '—'}</span>
          </div>
        </Card>

        <Card title="Perangkat keras">
          <dl class="mt-2 grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
            <dt class="text-ink-500">Board</dt>
            <dd class="text-ink-900">{snapshot?.board_name || '—'}</dd>
            <dt class="text-ink-500">Arsitektur</dt>
            <dd class="text-ink-900">{snapshot?.architecture || '—'}</dd>
            <dt class="text-ink-500">CPU model</dt>
            <dd class="text-ink-900">{snapshot?.cpu || '—'}</dd>
          </dl>
        </Card>

        <Card title="Kesehatan">
          {#if snapshot?.health}
            <dl class="mt-2 grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
              <dt class="text-ink-500">Suhu</dt>
              <dd class="font-mono text-ink-900">{snapshot.health.temperature_c ?? '—'} °C</dd>
              <dt class="text-ink-500">Suhu CPU</dt>
              <dd class="font-mono text-ink-900">{snapshot.health.cpu_temperature_c ?? '—'} °C</dd>
              <dt class="text-ink-500">Tegangan</dt>
              <dd class="font-mono text-ink-900">{snapshot.health.voltage_v ?? '—'} V</dd>
            </dl>
          {:else}
            <div class="mt-2 text-sm text-ink-500">Perangkat tidak mendukung pembacaan kesehatan.</div>
          {/if}
        </Card>
      </div>
    {:else if activeTab === 'interfaces'}
      <div class="mt-4">
        <div class="mb-3 flex flex-wrap gap-2">
          {#each [['all', 'Semua'], ['running', 'Jalan'], ['down', 'Down'], ['disabled', 'Nonaktif']] as [val, label] (val)}
            <button
              type="button"
              class="focus-ring rounded-lg px-3 py-1.5 text-sm {ifFilter === val ? 'bg-ink-900 text-white' : 'bg-white text-ink-700 ring-1 ring-ink-200 hover:bg-ink-50'}"
              aria-pressed={ifFilter === val}
              onclick={() => (ifFilter = val as typeof ifFilter)}
            >
              {label}
            </button>
          {/each}
        </div>
        <DataTable
          columns={interfaceCols}
          rows={interfaceTableRows}
          emptyTitle="Tidak ada interface"
          emptyHint="Snapshot router belum memuat interface."
        >
          {#snippet cell(row: any, col: Column)}
            {#if col.key === 'name'}
              <button type="button" class="focus-ring font-mono text-ink-900 underline-offset-2 hover:underline" onclick={() => openInterface(row.name)} title="Buka lalu lintas interface">
                {row.name}
              </button>
            {:else if col.key === 'status'}
              <Badge tone={row.status === 'running' ? 'positive' : row.status === 'disabled' ? 'neutral' : 'negative'} label={row.status === 'running' ? 'Jalan' : row.status === 'disabled' ? 'Nonaktif' : 'Down'} />
            {:else}
              <span class={col.key === 'mac' || col.key === 'rx' || col.key === 'tx' ? 'font-mono' : ''}>{row[col.key] ?? ''}</span>
            {/if}
          {/snippet}
        </DataTable>
      </div>
    {:else if activeTab === 'ip'}
      <div class="mt-4">
        <DataTable
          columns={ipCols}
          rows={ipTableRows}
          emptyTitle="Tidak ada alamat IP"
          emptyHint="Snapshot router belum memuat alamat IP."
        >
          {#snippet cell(row: any, col: Column)}
            {#if col.key === 'address'}
              <span class="font-mono text-ink-900">{row.address}</span>
            {:else if col.key === 'flags'}
              <span class="flex flex-wrap gap-1">
                {#if row.dynamic}<Badge tone="info" label="dynamic" />{/if}
                {#if row.disabled}<Badge tone="neutral" label="disabled" />{/if}
                {#if !row.dynamic && !row.disabled}<span class="text-ink-400">—</span>{/if}
              </span>
            {:else}
              <span>{row[col.key] ?? ''}</span>
            {/if}
          {/snippet}
        </DataTable>
      </div>
    {:else if activeTab === 'metrics'}
      <div class="mt-4">
        <DataTable columns={metricCols} rows={metricRows} emptyTitle="Belum ada metrik" emptyHint="Metrik akan terkumpul tiap interval pemantauan." />
      </div>
    {/if}
  {/if}

  <RouterDetailDialogs
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
</AppShell>
