<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';

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

  let cpuSeries = $derived.by(() => {
    const pts = metrics
      .slice()
      .reverse()
      .map((m) => (m.cpu_load == null ? null : Math.max(0, Math.min(100, m.cpu_load))));
    return pts.filter((v) => v != null) as number[];
  });

  let refreshHandle: any = null;
  let refreshInFlight = false;

  onMount(() => {
    if (!$can('read', 'network_routers') && !$can('manage', 'network_routers')) {
      goto('/unauthorized');
      return;
    }
    void refresh({ silent: true });

    // Re-check status/metrics periodically.
    refreshHandle = setInterval(() => {
      void refresh({ silent: true });
    }, 5000);
  });

  onDestroy(() => {
    if (refreshHandle) clearInterval(refreshHandle);
  });

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
    } catch (e: any) {
      if (!opts?.silent) toast.error(e?.message || e);
    } finally {
      initialLoading = false;
      refreshing = false;
      refreshInFlight = false;
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
            <span class="kv-value">{router.last_seen_at || '—'}</span>
          </div>
        </div>
      </div>
    </div>

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
                >{formatBytes(snapshot.free_memory_bytes)} / {formatBytes(snapshot.total_memory_bytes)}</span
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

      <div class="card full">
        <div class="card-head">
          <h2>Interfaces</h2>
          <span class="muted">{snapshot.interfaces.length} total</span>
        </div>
        <div class="table-wrap">
          <table class="table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Type</th>
                <th>Status</th>
                <th class="right">MTU</th>
                <th class="right">RX</th>
                <th class="right">TX</th>
                <th class="right">Downs</th>
              </tr>
            </thead>
            <tbody>
              {#each snapshot.interfaces as it (it.name)}
                <tr class:dim={it.disabled}>
                  <td class="mono">{it.name}</td>
                  <td class="muted">{it.interface_type || '—'}</td>
                  <td>
                    {#if it.disabled}
                      <span class="pill off">Disabled</span>
                    {:else if it.running}
                      <span class="pill ok">Running</span>
                    {:else}
                      <span class="pill warn">Down</span>
                    {/if}
                  </td>
                  <td class="mono right">{it.mtu ?? '—'}</td>
                  <td class="mono right">{formatBytes(it.rx_byte)}</td>
                  <td class="mono right">{formatBytes(it.tx_byte)}</td>
                  <td class="mono right">{it.link_downs ?? '—'}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>

      <div class="card full">
        <div class="card-head">
          <h2>IP Addresses</h2>
          <span class="muted">{snapshot.ip_addresses.length} total</span>
        </div>
        {#if snapshot.ip_addresses.length === 0}
          <div class="muted">No IP addresses.</div>
        {:else}
          <div class="ip-grid">
            {#each snapshot.ip_addresses as ip, idx (ip.address + ':' + (ip.interface || '') + ':' + idx)}
              <div class="ip-item">
                <div class="ip-top">
                  <span class="mono">{ip.address}</span>
                  {#if ip.dynamic}
                    <span class="pill info">Dynamic</span>
                  {/if}
                  {#if ip.disabled}
                    <span class="pill off">Disabled</span>
                  {/if}
                </div>
                <div class="ip-meta">
                  <span class="muted">{ip.interface || '—'}</span>
                  {#if ip.network}
                    <span class="muted">· {ip.network}</span>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
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

  .table {
    width: 100%;
    border-collapse: collapse;
    min-width: 840px;
  }

  .table th,
  .table td {
    padding: 10px 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--border-color), transparent 30%);
    text-align: left;
    white-space: nowrap;
  }

  .table th {
    position: sticky;
    top: 0;
    z-index: 2;
    background: color-mix(in srgb, var(--bg-card), transparent 2%);
    color: var(--text-secondary);
    font-weight: 900;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    font-size: 0.72rem;
  }

  .right {
    text-align: right !important;
  }

  tr.dim td {
    opacity: 0.7;
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

  .ip-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .ip-item {
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card), transparent 8%);
    border-radius: 16px;
    padding: 12px;
  }

  .ip-top {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .ip-meta {
    margin-top: 6px;
    color: var(--text-secondary);
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

    .ip-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
