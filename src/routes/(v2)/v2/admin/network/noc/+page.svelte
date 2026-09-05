<script lang="ts">
  /*
    NOC v2 — gelombang 24b.

    Versi lama: (app)/admin/network/noc/+page.svelte (670 baris).
    Perilaku identik: pantau router 5 detik, filter status/risiko/sort,
    ambang dari settings tenant, tautan ke detail router + wallboard +
    alerts/incidents. Skor/filter/format kini dari helper murni
    nocInsights (4 tes).
  */
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { toast } from '$lib/stores/toast';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import {
    nocFormatBps,
    nocHealthScore,
    nocInMaintenance,
    nocMatchesRisk,
    nocMemoryPct,
    type NocThresholds,
  } from '$lib/utils/nocInsights';
  import {
    AppShell,
    Badge,
    Button,
    Card,
    DataTable,
    Field,
    PageHeader,
    RowActions,
    StatTile,
  } from '$lib/components/ds';
  import type { Column } from '$lib/components/ds/table-types';

  type NocRowFull = {
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

  const columns: Column[] = [
    { key: 'router', label: 'Router' },
    { key: 'status', label: 'Status' },
    { key: 'health', label: 'Kesehatan' },
    { key: 'traffic', label: 'Trafik' },
    { key: 'latency', label: 'Latensi' },
    { key: 'seen', label: 'Terakhir terlihat' },
    { key: 'actions', label: '' },
  ];

  let loading = $state(true);
  let refreshing = $state(false);
  let rows = $state<NocRowFull[]>([]);
  let statusFilter = $state<'all' | 'offline' | 'online'>('all');
  let riskFilter = $state<'all' | 'hot' | 'latency' | 'cpu'>('all');
  let sortFilter = $state<'health_desc' | 'last_seen_desc' | 'latency_desc' | 'cpu_desc'>('health_desc');
  let cpuRisk = $state(70);
  let latRisk = $state(200);
  let cpuHot = $state(85);
  let latHot = $state(400);
  let refreshHandle: ReturnType<typeof setInterval> | null = null;

  const th = $derived<NocThresholds>({ cpuRisk, cpuHot, latRisk, latHot });
  const canUseTenantSettings = $derived($can('read', 'settings') || $can('update', 'settings'));

  const statusOptions = [
    { value: 'all', label: 'Semua status' },
    { value: 'online', label: 'Online' },
    { value: 'offline', label: 'Offline' },
  ];
  const riskOptions = [
    { value: 'all', label: 'Semua risiko' },
    { value: 'hot', label: 'Panas (kritis)' },
    { value: 'latency', label: `Latensi ≥ ${latRisk} ms` },
    { value: 'cpu', label: `CPU ≥ ${cpuRisk}%` },
  ];
  const sortOptions = [
    { value: 'health_desc', label: 'Paling bermasalah' },
    { value: 'last_seen_desc', label: 'Terakhir terlihat' },
    { value: 'latency_desc', label: 'Latensi tertinggi' },
    { value: 'cpu_desc', label: 'CPU tertinggi' },
  ];

  const filtered = $derived.by(() => {
    let out = rows.slice();
    if (statusFilter === 'offline') out = out.filter((r) => !r.is_online);
    if (statusFilter === 'online') out = out.filter((r) => r.is_online);
    if (riskFilter !== 'all') out = out.filter((r) => nocMatchesRisk(r, riskFilter, th));
    out.sort((a, b) => {
      if (sortFilter === 'latency_desc') return (b.latency_ms ?? -1) - (a.latency_ms ?? -1);
      if (sortFilter === 'cpu_desc') return (b.cpu_load ?? -1) - (a.cpu_load ?? -1);
      if (sortFilter === 'last_seen_desc') {
        return ms(b.last_seen_at) - ms(a.last_seen_at);
      }
      const byHealth = nocHealthScore(b, th) - nocHealthScore(a, th);
      if (byHealth !== 0) return byHealth;
      return ms(b.last_seen_at) - ms(a.last_seen_at);
    });
    return out;
  });

  const stats = $derived.by(() => {
    const total = rows.length;
    const online = rows.filter((r) => r.is_online).length;
    return {
      total,
      online,
      offline: total - online,
      hot: rows.filter((r) => nocMatchesRisk(r, 'hot', th)).length,
    };
  });

  function ms(v?: string | null): number {
    if (!v) return 0;
    const t = new Date(v).getTime();
    return Number.isFinite(t) ? t : 0;
  }

  function openRouter(id: string) {
    goto(`/v2/admin/network/routers/${id}`);
  }

  function resetFilters() {
    statusFilter = 'all';
    riskFilter = 'all';
    sortFilter = 'health_desc';
  }

  onMount(() => {
    if (!$can('read', 'network_noc') && !$can('manage', 'network_noc')) {
      goto('/unauthorized');
      return;
    }
    if (canUseTenantSettings) {
      void (async () => {
        try {
          const keys = [
            'mikrotik_alert_cpu_risk',
            'mikrotik_alert_cpu_hot',
            'mikrotik_alert_latency_risk_ms',
            'mikrotik_alert_latency_hot_ms',
          ] as const;
          const vals = await Promise.all(keys.map((k) => api.settings.getValue(k)));
          const map = Object.fromEntries(keys.map((k, i) => [k, vals[i]])) as Record<string, string | null>;
          const cpuR = Number.parseInt(map['mikrotik_alert_cpu_risk'] || '', 10);
          const cpuH = Number.parseInt(map['mikrotik_alert_cpu_hot'] || '', 10);
          const latR = Number.parseInt(map['mikrotik_alert_latency_risk_ms'] || '', 10);
          const latH = Number.parseInt(map['mikrotik_alert_latency_hot_ms'] || '', 10);
          if (Number.isFinite(cpuR) && cpuR > 0) cpuRisk = cpuR;
          if (Number.isFinite(cpuH) && cpuH > 0) cpuHot = Math.max(cpuH, cpuRisk);
          if (Number.isFinite(latR) && latR > 0) latRisk = latR;
          if (Number.isFinite(latH) && latH > 0) latHot = Math.max(latH, latRisk);
        } catch {
          // abaikan — pakai default
        }
      })();
    }
    void load();
    refreshHandle = setInterval(() => void refreshSilent(), 5000);
  });

  onDestroy(() => {
    if (refreshHandle) clearInterval(refreshHandle);
  });

  async function load() {
    loading = true;
    try {
      rows = (await api.mikrotik.routers.noc()) as NocRowFull[];
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      loading = false;
    }
  }

  async function refreshSilent() {
    if (refreshing) return;
    refreshing = true;
    try {
      rows = (await api.mikrotik.routers.noc()) as NocRowFull[];
    } catch {
      // abaikan
    } finally {
      refreshing = false;
    }
  }
</script>
<AppShell title="NOC">
  <PageHeader
    title="NOC"
    eyebrow="Jaringan"
    desc="Pantauan kesehatan router — disegarkan tiap 5 detik."
  >
    {#snippet actions()}
      <Button variant="ghost" href="/v2/admin/network/alerts">Alert</Button>
      <Button variant="ghost" href="/v2/admin/network/incidents">Insiden</Button>
      <Button variant="ghost" href="/v2/admin/network/noc/wallboard">Wallboard</Button>
      <Button variant="ghost" icon="refresh" onclick={() => void load()} disabled={loading}>
        Segarkan
      </Button>
    {/snippet}
  </PageHeader>

  <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
    <StatTile label="Total router" value={String(stats.total)} hint={`${stats.online} online`} />
    <StatTile label="Online" value={String(stats.online)} hint={`dari ${stats.total} router`} tone="positive" />
    <StatTile label="Offline" value={String(stats.offline)} hint="perlu tindak lanjut" tone="negative" />
    <StatTile label="Panas" value={String(stats.hot)} hint={`CPU ≥ ${cpuHot}% atau latensi ≥ ${latHot} ms`} tone="warning" />
  </div>

  <Card title="Filter">
    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      <Field id="noc-status" label="Status" type="select" stacked value={statusFilter} options={statusOptions} onchange={(v) => (statusFilter = v as typeof statusFilter)} />
      <Field id="noc-risk" label="Risiko" type="select" stacked value={riskFilter} options={riskOptions} onchange={(v) => (riskFilter = v as typeof riskFilter)} />
      <Field id="noc-sort" label="Urut" type="select" stacked value={sortFilter} options={sortOptions} onchange={(v) => (sortFilter = v as typeof sortFilter)} />
      <div class="flex items-end">
        <Button variant="ghost" onclick={resetFilters}>Atur ulang</Button>
      </div>
    </div>
  </Card>

  <Card title={`Router (${filtered.length})`} padded={false}>
    <DataTable
      {columns}
      rows={filtered}
      {loading}
      emptyTitle="Tidak ada router cocok filter"
      emptyHint="Ubah filter atau atur ulang."
    >
      {#snippet cell(item, column)}
        {#if column.key === 'router'}
          <div>
            <button type="button" class="text-sm font-semibold text-ink-900 hover:underline" onclick={() => openRouter(item.id)}>
              {item.name}
            </button>
            <div class="mt-0.5 flex flex-wrap items-center gap-1.5 text-xs text-ink-400">
              {#if item.identity}<span class="font-mono">{item.identity}</span>{/if}
              {#if item.ros_version}<span>ROS {item.ros_version}</span>{/if}
              {#if nocInMaintenance(item)}<Badge tone="info" label="Maintenance" />{/if}
            </div>
            <div class="font-mono text-xs text-ink-400">{item.host}:{item.port}</div>
            {#if item.last_error}<div class="text-xs text-red-600">{item.last_error}</div>{/if}
          </div>
        {:else if column.key === 'status'}
          <Badge tone={item.is_online ? 'positive' : 'negative'} label={item.is_online ? 'Online' : 'Offline'} />
        {:else if column.key === 'health'}
          {@const cpu = item.cpu_load ?? null}
          {@const mem = nocMemoryPct(item.total_memory_bytes, item.free_memory_bytes)}
          {@const disk = nocMemoryPct(item.total_hdd_bytes, item.free_hdd_bytes)}
          <span class="font-mono text-xs {cpu != null && cpu >= cpuRisk ? 'text-red-600' : ''}">{cpu == null ? '—' : `${cpu}%`} CPU</span>
          <span class="text-xs text-ink-400"> · {mem == null ? '—' : `${mem}%`} MEM · {disk == null ? '—' : `${disk}%`} disk</span>
        {:else if column.key === 'traffic'}
          <span class="font-mono text-xs">{nocFormatBps(item.rx_bps)} RX · {nocFormatBps(item.tx_bps)} TX</span>
        {:else if column.key === 'latency'}
          {#if item.latency_ms != null}
            <span class="font-mono text-xs {item.latency_ms >= latRisk ? 'text-red-600' : ''}">{item.latency_ms} ms</span>
          {:else}
            <span class="text-xs text-ink-400">—</span>
          {/if}
        {:else if column.key === 'seen'}
          {#if item.last_seen_at}
            <span class="text-xs text-ink-500" title={formatDateTime(item.last_seen_at, { timeZone: $appSettings.app_timezone })}>{timeAgo(item.last_seen_at)}</span>
          {:else}
            <span class="text-xs text-ink-400">—</span>
          {/if}
        {:else if column.key === 'actions'}
          <RowActions
            primary={{ label: 'Buka', icon: 'chevronRight', onclick: () => openRouter(item.id) }}
            rest={[]}
          />
        {/if}
      {/snippet}
    </DataTable>
  </Card>
</AppShell>
