<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { api } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import { toast } from '$lib/stores/toast';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { user, tenant } from '$lib/stores/auth';
  import AlertsIncidentsSwitch from '$lib/components/network/AlertsIncidentsSwitch.svelte';

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

  let loading = $state(true);
  let refreshing = $state(false);
  let rows = $state<NocRow[]>([]);
  let isMobile = $state(false);
  let mqCleanup: (() => void) | null = null;

  let statusFilter = $state<'all' | 'offline' | 'online'>('all');
  let riskFilter = $state<'all' | 'hot' | 'latency' | 'cpu'>('all');

  // Thresholds for NOC risk filters.
  // "CPU" and "Latency" are meant to be practical daily filters (lower thresholds),
  // while "Hot" is reserved for critical attention (higher thresholds or offline).
  let CPU_RISK = $state(70);
  let LATENCY_RISK = $state(200);
  let CPU_HOT = $state(85);
  let LATENCY_HOT = $state(400);

  let refreshHandle: any = null;

  let tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  let tenantPrefix = $derived(tenantCtx.tenantPrefix);

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

    // Load thresholds from tenant settings (best-effort).
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

        const cpuRisk = Number.parseInt(map['mikrotik_alert_cpu_risk'] || '', 10);
        const cpuHot = Number.parseInt(map['mikrotik_alert_cpu_hot'] || '', 10);
        const latRisk = Number.parseInt(map['mikrotik_alert_latency_risk_ms'] || '', 10);
        const latHot = Number.parseInt(map['mikrotik_alert_latency_hot_ms'] || '', 10);

        if (Number.isFinite(cpuRisk) && cpuRisk > 0) CPU_RISK = cpuRisk;
        if (Number.isFinite(cpuHot) && cpuHot > 0) CPU_HOT = Math.max(cpuHot, CPU_RISK);
        if (Number.isFinite(latRisk) && latRisk > 0) LATENCY_RISK = latRisk;
        if (Number.isFinite(latHot) && latHot > 0) LATENCY_HOT = Math.max(latHot, LATENCY_RISK);
      } catch {
        // ignore
      }
    })();

    void load();
    refreshHandle = setInterval(() => void refreshSilent(), 5000);
  });

  onDestroy(() => {
    if (refreshHandle) clearInterval(refreshHandle);
    mqCleanup?.();
  });

  async function load() {
    loading = true;
    try {
      rows = (await api.mikrotik.routers.noc()) as any;
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  async function refreshSilent() {
    if (refreshing) return;
    refreshing = true;
    try {
      rows = (await api.mikrotik.routers.noc()) as any;
    } catch {
      // ignore
    } finally {
      refreshing = false;
    }
  }

  function pctUsed(total?: number | null, free?: number | null) {
    if (!total || total <= 0 || free == null) return null;
    const used = total - free;
    return Math.max(0, Math.min(100, Math.round((used / total) * 100)));
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

  function statusLabel(r: NocRow) {
    if (r.is_online) return $t('admin.network.routers.badges.online') || 'Online';
    return $t('admin.network.routers.badges.offline') || 'Offline';
  }

  function statusTooltip(k: 'all' | 'online' | 'offline') {
    const key =
      k === 'all'
        ? 'admin.network.noc.tooltips.status_all'
        : k === 'online'
          ? 'admin.network.noc.tooltips.status_online'
          : 'admin.network.noc.tooltips.status_offline';

    const fallback =
      k === 'all'
        ? 'Show all routers.'
        : k === 'online'
          ? 'Show routers that are currently reachable.'
          : 'Show routers that are currently unreachable.';

    const s = $t(key) as any;
    return !s || s === key ? fallback : s;
  }

  function riskTooltip(k: 'all' | 'hot' | 'latency' | 'cpu') {
    if (k === 'all') {
      const key = 'admin.network.noc.tooltips.risk_any';
      const s = $t(key) as any;
      return !s || s === key ? 'No additional risk filtering.' : s;
    }
    if (k === 'hot') {
      const key = 'admin.network.noc.tooltips.risk_hot';
      const s = $t(key) as any;
      const base =
        !s || s === key ? 'Critical attention: offline, very high CPU, or very high latency.' : s;
      return `${base} (offline • CPU ≥ ${CPU_HOT}% • latency ≥ ${LATENCY_HOT}ms)`;
    }
    if (k === 'latency') {
      const key = 'admin.network.noc.tooltips.risk_latency';
      const s = $t(key) as any;
      const base = !s || s === key ? 'Show routers with high latency.' : s;
      return `${base} (≥ ${LATENCY_RISK}ms)`;
    }
    const key = 'admin.network.noc.tooltips.risk_cpu';
    const s = $t(key) as any;
    const base = !s || s === key ? 'Show routers with high CPU load.' : s;
    return `${base} (≥ ${CPU_RISK}%)`;
  }

  const filtered = $derived.by(() => {
    let out = rows.slice();
    if (statusFilter === 'offline') out = out.filter((r) => !r.is_online);
    if (statusFilter === 'online') out = out.filter((r) => r.is_online);

    // "Hot" means: offline OR very high cpu OR high latency.
    if (riskFilter !== 'all') {
      out = out.filter((r) => {
        const maintenanceUntil = r.maintenance_until ? new Date(r.maintenance_until).getTime() : NaN;
        const inMaintenance = Number.isFinite(maintenanceUntil) ? maintenanceUntil > Date.now() : false;
        if (inMaintenance) return false; // muted

        const cpu = r.cpu_load ?? 0;
        const lat = r.latency_ms ?? 0;
        const isHot = !r.is_online || cpu >= CPU_HOT || lat >= LATENCY_HOT;
        if (riskFilter === 'hot') return isHot;
        if (riskFilter === 'latency') return lat >= LATENCY_RISK;
        if (riskFilter === 'cpu') return cpu >= CPU_RISK;
        return true;
      });
    }
    return out;
  });

  const stats = $derived.by(() => {
    const total = rows.length;
    const online = rows.filter((r) => r.is_online).length;
    const offline = total - online;
    const hot = rows.filter(
      (r) =>
        // Exclude maintenance from hot count; it's muted.
        !(r.maintenance_until && new Date(r.maintenance_until).getTime() > Date.now()) &&
        (!r.is_online || (r.cpu_load ?? 0) >= CPU_HOT || (r.latency_ms ?? 0) >= LATENCY_HOT),
    )
      .length;
    return { total, online, offline, hot };
  });

  const columns = $derived([
    { key: 'router', label: $t('admin.network.noc.columns.router') || 'Router' },
    { key: 'status', label: $t('admin.network.noc.columns.status') || 'Status' },
    { key: 'health', label: $t('admin.network.noc.columns.health') || 'Health' },
    { key: 'traffic', label: $t('admin.network.noc.columns.traffic') || 'Traffic' },
    { key: 'latency', label: $t('admin.network.noc.columns.latency') || 'Latency', align: 'right' as const, width: '110px' },
    { key: 'seen', label: $t('admin.network.noc.columns.seen') || 'Last Seen' },
    { key: 'actions', label: '', align: 'right' as const, width: '80px' },
  ]);

  function openRouter(id: string) {
    goto($page.url.pathname.replace(/\/admin\/network\/noc\/?$/, `/admin/network/routers/${id}`));
  }

  function networkRoute(to: 'noc' | 'alerts' | 'incidents') {
    return `${tenantPrefix}/admin/network/${to}`;
  }
</script>

<div class="page-content fade-in">
  <div class="head">
    <div>
      <h1 class="title">{$t('admin.network.noc.title') || 'Network NOC'}</h1>
      <p class="sub">
        {$t('admin.network.noc.subtitle') ||
          'A quick operational view of all routers: status, health, and traffic.'}
      </p>
    </div>

    <div class="head-actions">
      <AlertsIncidentsSwitch
        current="noc"
        nocHref={networkRoute('noc')}
        alertsHref={networkRoute('alerts')}
        incidentsHref={networkRoute('incidents')}
      />
      <button
        class="btn ghost"
        type="button"
        onclick={() => goto(`${tenantPrefix}/admin/network/noc/wallboard`)}
        title={$t('sidebar.wallboard') || 'Wallboard'}
      >
        <Icon name="monitor" size={16} />
        {$t('sidebar.wallboard') || 'Wallboard'}
      </button>
    </div>
  </div>

  <div class="stats">
    <div class="stat-card">
      <div class="stat-top">
        <span>{$t('admin.network.noc.stats.total') || 'Total'}</span>
        <Icon name="router" size={16} />
      </div>
      <div class="stat-value">{stats.total}</div>
    </div>
    <div class="stat-card tone-ok">
      <div class="stat-top">
        <span>{$t('admin.network.noc.stats.online') || 'Online'}</span>
        <Icon name="check-circle" size={16} />
      </div>
      <div class="stat-value">{stats.online}</div>
    </div>
    <div class="stat-card tone-bad">
      <div class="stat-top">
        <span>{$t('admin.network.noc.stats.offline') || 'Offline'}</span>
        <Icon name="alert-triangle" size={16} />
      </div>
      <div class="stat-value">{stats.offline}</div>
    </div>
    <div class="stat-card tone-warn">
      <div class="stat-top">
        <span>{$t('admin.network.noc.stats.hot') || 'Hot'}</span>
        <Icon name="activity" size={16} />
      </div>
      <div class="stat-value">{stats.hot}</div>
    </div>
  </div>

  <div class="filters">
    <div class="seg">
      <button
        type="button"
        class="seg-btn {statusFilter === 'all' ? 'active' : ''}"
        title={statusTooltip('all')}
        onclick={() => (statusFilter = 'all')}
      >
        {$t('common.all') || 'All'}
      </button>
      <button
        type="button"
        class="seg-btn {statusFilter === 'online' ? 'active' : ''}"
        title={statusTooltip('online')}
        onclick={() => (statusFilter = 'online')}
      >
        {$t('admin.network.noc.filters.online') || 'Online'}
      </button>
      <button
        type="button"
        class="seg-btn {statusFilter === 'offline' ? 'active' : ''}"
        title={statusTooltip('offline')}
        onclick={() => (statusFilter = 'offline')}
      >
        {$t('admin.network.noc.filters.offline') || 'Offline'}
      </button>
    </div>

    <div class="seg">
      <button
        type="button"
        class="seg-btn {riskFilter === 'all' ? 'active' : ''}"
        title={riskTooltip('all')}
        onclick={() => (riskFilter = 'all')}
      >
        {$t('admin.network.noc.filters.any') || 'Any'}
      </button>
      <button
        type="button"
        class="seg-btn {riskFilter === 'hot' ? 'active' : ''}"
        title={riskTooltip('hot')}
        onclick={() => (riskFilter = 'hot')}
      >
        {$t('admin.network.noc.filters.hot') || 'Hot'}
      </button>
      <button
        type="button"
        class="seg-btn {riskFilter === 'latency' ? 'active' : ''}"
        title={riskTooltip('latency')}
        onclick={() => (riskFilter = 'latency')}
      >
        {$t('admin.network.noc.filters.latency') || 'Latency'}
      </button>
      <button
        type="button"
        class="seg-btn {riskFilter === 'cpu' ? 'active' : ''}"
        title={riskTooltip('cpu')}
        onclick={() => (riskFilter = 'cpu')}
      >
        {$t('admin.network.noc.filters.cpu') || 'CPU'}
      </button>
    </div>
  </div>

  <div class="table-wrap">
    <Table
      {columns}
      data={filtered}
      keyField="id"
      {loading}
      pagination={true}
      pageSize={10}
      searchable={true}
      searchPlaceholder={$t('admin.network.noc.search') || 'Search routers...'}
      mobileView={isMobile ? 'card' : 'scroll'}
      emptyText={$t('admin.network.noc.empty') || 'No routers'}
    >
      {#snippet cell({ item, key }: any)}
        {#if key === 'router'}
          <button class="link" type="button" onclick={() => openRouter(item.id)}>
            <div class="r-top">
              <span class="name">{item.name}</span>
              {#if item.identity}
                <span class="chip mono">{item.identity}</span>
              {/if}
              {#if item.ros_version}
                <span class="muted">ROS {item.ros_version}</span>
              {/if}
              {#if item.maintenance_until && new Date(item.maintenance_until).getTime() > Date.now()}
                <span class="chip warn" title={item.maintenance_reason || ''}>
                  {$t('admin.network.routers.badges.maintenance') || 'Maintenance'}
                </span>
              {/if}
            </div>
            <div class="muted mono">{item.username}@{item.host}:{item.port}</div>
            {#if item.last_error}
              <div class="error">{item.last_error}</div>
            {/if}
          </button>
        {:else if key === 'status'}
          <span class="badge" class:online={item.is_online} class:offline={!item.is_online}>
            {statusLabel(item)}
          </span>
        {:else if key === 'health'}
          {@const cpu = item.cpu_load ?? null}
          {@const mem = pctUsed(item.total_memory_bytes, item.free_memory_bytes)}
          {@const disk = pctUsed(item.total_hdd_bytes, item.free_hdd_bytes)}
          <div class="mini">
            <span class:bad={cpu != null && cpu >= CPU_RISK} class="mono">{cpu == null ? '—' : `${cpu}%`}</span>
            <span class="muted">CPU</span>
            <span class="sep">·</span>
            <span class:bad={mem != null && mem >= 90} class="mono">{mem == null ? '—' : `${mem}%`}</span>
            <span class="muted">MEM</span>
            <span class="sep">·</span>
            <span class:bad={disk != null && disk >= 90} class="mono">{disk == null ? '—' : `${disk}%`}</span>
            <span class="muted">DISK</span>
          </div>
        {:else if key === 'traffic'}
          <div class="mini">
            <span class="mono">{formatBps(item.rx_bps)}</span>
            <span class="muted">RX</span>
            <span class="sep">·</span>
            <span class="mono">{formatBps(item.tx_bps)}</span>
            <span class="muted">TX</span>
          </div>
        {:else if key === 'latency'}
          {#if item.latency_ms != null}
            <span class="mono {item.latency_ms >= LATENCY_RISK ? 'bad' : ''}">{item.latency_ms} ms</span>
          {:else}
            <span class="muted">—</span>
          {/if}
        {:else if key === 'seen'}
          {#if item.last_seen_at}
            <span
              class="muted"
              title={formatDateTime(item.last_seen_at, { timeZone: $appSettings.app_timezone })}
              >{timeAgo(item.last_seen_at)}</span
            >
          {:else}
            <span class="muted">—</span>
          {/if}
        {:else if key === 'actions'}
          <button class="icon-btn" type="button" onclick={() => openRouter(item.id)} title={$t('common.open') || 'Open'}>
            <Icon name="arrow-right" size={16} />
          </button>
        {:else}
          {item[key] ?? ''}
        {/if}
      {/snippet}
    </Table>
  </div>
</div>

<style>
  .page-content {
    padding: 28px;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 14px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }

  .title {
    margin: 0;
    font-size: 1.6rem;
    font-weight: 950;
    color: var(--text-primary);
  }

  .sub {
    margin: 0.35rem 0 0 0;
    color: var(--text-secondary);
  }

  .head-actions {
    display: flex;
    align-items: center;
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

  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 12px;
    margin-bottom: 14px;
  }

  .stat-card {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 14px 14px 12px;
  }

  .stat-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--text-secondary);
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-size: 0.72rem;
  }

  .stat-value {
    margin-top: 10px;
    font-size: 1.6rem;
    font-weight: 950;
    color: var(--text-primary);
  }

  .tone-ok {
    box-shadow: 0 0 0 1px rgba(34, 197, 94, 0.15) inset;
  }

  .tone-bad {
    box-shadow: 0 0 0 1px rgba(239, 68, 68, 0.16) inset;
  }

  .tone-warn {
    box-shadow: 0 0 0 1px rgba(245, 158, 11, 0.16) inset;
  }

  .filters {
    display: flex;
    justify-content: flex-start;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }

  .seg {
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

  .table-wrap {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 18px;
    overflow: hidden;
  }

  .link {
    width: 100%;
    border: none;
    background: transparent;
    padding: 0;
    cursor: pointer;
    text-align: left;
  }

  .link:hover .name {
    text-decoration: underline;
  }

  .r-top {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .name {
    font-weight: 950;
    color: var(--text-primary);
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

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
  }

  .muted {
    color: var(--text-secondary);
  }

  .sep {
    opacity: 0.6;
  }

  .error {
    margin-top: 6px;
    color: color-mix(in srgb, #ef4444, var(--text-primary) 15%);
    font-size: 0.85rem;
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
  }

  .badge.online {
    background: rgba(34, 197, 94, 0.12);
    color: rgba(34, 197, 94, 0.95);
    border-color: rgba(34, 197, 94, 0.28);
  }

  .badge.offline {
    background: rgba(239, 68, 68, 0.12);
    color: rgba(239, 68, 68, 0.95);
    border-color: rgba(239, 68, 68, 0.28);
  }

  .mini {
    display: inline-flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    color: var(--text-secondary);
  }

  .bad {
    color: rgba(239, 68, 68, 0.95);
    font-weight: 900;
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

    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
