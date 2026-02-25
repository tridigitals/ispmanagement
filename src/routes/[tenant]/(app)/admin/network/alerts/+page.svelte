<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import AlertsIncidentsSwitch from '$lib/components/network/AlertsIncidentsSwitch.svelte';
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import RowActionButtons from '$lib/components/network/RowActionButtons.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { user, tenant } from '$lib/stores/auth';

  type AlertRow = {
    id: string;
    tenant_id: string;
    router_id: string;
    alert_type: string;
    severity: string;
    status: string;
    title: string;
    message: string;
    value_num?: number | null;
    threshold_num?: number | null;
    triggered_at: string;
    last_seen_at: string;
    resolved_at?: string | null;
    acked_at?: string | null;
    acked_by?: string | null;
    created_at: string;
    updated_at: string;
  };

  let loading = $state(true);
  let refreshing = $state(false);
  let rows = $state<AlertRow[]>([]);
  let activeOnly = $state(true);
  let isMobile = $state(false);
  let filterStatus = $state('all');
  let filterSeverity = $state('all');
  let filterType = $state('all');
  let filterFrom = $state('');
  let filterTo = $state('');
  let filterSort = $state('last_seen_desc');

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

  const columns = $derived.by(() => [
    { key: 'title', label: $t('admin.network.alerts.columns.alert') || 'Alert' },
    { key: 'type', label: $t('admin.network.alerts.columns.type') || 'Type' },
    { key: 'severity', label: $t('admin.network.alerts.columns.severity') || 'Severity' },
    { key: 'status', label: $t('admin.network.alerts.columns.status') || 'Status' },
    { key: 'seen', label: $t('admin.network.alerts.columns.seen') || 'Last Seen' },
    { key: 'actions', label: '', align: 'right' as const, width: '140px' },
  ]);
  const alertTypeOptions = $derived.by(() =>
    Array.from(new Set(rows.map((row) => row.alert_type).filter(Boolean))).sort((a, b) => a.localeCompare(b)),
  );
  const filteredRows = $derived.by(() => {
    const severityWeight = (severity: string) => {
      if (severity === 'critical') return 3;
      if (severity === 'warning') return 2;
      if (severity === 'info') return 1;
      return 0;
    };

    const list = rows.filter((row) => {
      if (filterStatus !== 'all' && row.status !== filterStatus) return false;
      if (filterSeverity !== 'all' && row.severity !== filterSeverity) return false;
      if (filterType !== 'all' && row.alert_type !== filterType) return false;

      const seenTs = new Date(row.last_seen_at).getTime();
      if (Number.isNaN(seenTs)) return false;

      if (filterFrom) {
        const fromTs = new Date(`${filterFrom}T00:00:00`).getTime();
        if (!Number.isNaN(fromTs) && seenTs < fromTs) return false;
      }
      if (filterTo) {
        const toTs = new Date(`${filterTo}T23:59:59.999`).getTime();
        if (!Number.isNaN(toTs) && seenTs > toTs) return false;
      }

      return true;
    });

    list.sort((a, b) => {
      const aLastSeen = new Date(a.last_seen_at).getTime() || 0;
      const bLastSeen = new Date(b.last_seen_at).getTime() || 0;
      if (filterSort === 'last_seen_asc') return aLastSeen - bLastSeen;
      if (filterSort === 'severity_desc') {
        const bySeverity = severityWeight(b.severity) - severityWeight(a.severity);
        if (bySeverity !== 0) return bySeverity;
      }
      return bLastSeen - aLastSeen;
    });

    return list;
  });
  const stats = $derived.by(() => ({
    total: rows.length,
    open: rows.filter((r) => r.status === 'open').length,
    ack: rows.filter((r) => r.status === 'ack').length,
    critical: rows.filter((r) => r.severity === 'critical').length,
  }));

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
      } catch {
        // @ts-ignore
        mq.addListener?.(sync);
      }
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
      rows = (await api.mikrotik.alerts.list({ activeOnly })) as any;
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
      rows = (await api.mikrotik.alerts.list({ activeOnly })) as any;
    } catch {
      // ignore background refresh
    } finally {
      refreshing = false;
    }
  }

  function typeLabel(tpe: string) {
    if (tpe === 'offline') return $t('admin.network.alerts.types.offline') || 'Offline';
    if (tpe === 'cpu') return $t('admin.network.alerts.types.cpu') || 'CPU';
    if (tpe === 'latency') return $t('admin.network.alerts.types.latency') || 'Latency';
    return tpe;
  }

  function severityLabel(sev: string) {
    if (sev === 'critical') return $t('admin.network.alerts.severity.critical') || 'Critical';
    if (sev === 'warning') return $t('admin.network.alerts.severity.warning') || 'Warning';
    return $t('admin.network.alerts.severity.info') || 'Info';
  }

  async function ack(id: string) {
    try {
      await api.mikrotik.alerts.ack(id);
      toast.success($t('admin.network.alerts.toasts.acked') || 'Alert acknowledged');
      void load();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function resolve(id: string) {
    try {
      await api.mikrotik.alerts.resolve(id);
      toast.success($t('admin.network.alerts.toasts.resolved') || 'Alert resolved');
      void load();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function snooze(routerId: string, minutes: number) {
    try {
      const until = new Date(Date.now() + minutes * 60 * 1000).toISOString();
      await api.mikrotik.routers.update(routerId, {
        maintenance_until: until,
        maintenance_reason: `Snoozed from alert for ${minutes}m`,
      });
      toast.success($t('admin.network.alerts.toasts.snoozed') || 'Router snoozed');
      void load();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  function openRouter(routerId: string) {
    goto($page.url.pathname.replace(/\/admin\/network\/alerts\/?$/, `/admin/network/routers/${routerId}`));
  }

  function networkRoute(to: 'noc' | 'alerts' | 'incidents') {
    return `${tenantPrefix}/admin/network/${to}`;
  }

  function resetFilters() {
    filterStatus = 'all';
    filterSeverity = 'all';
    filterType = 'all';
    filterSort = 'last_seen_desc';
    filterFrom = '';
    filterTo = '';
  }
</script>

<div class="page-content fade-in">
  <NetworkPageHeader
    title={$t('admin.network.alerts.title') || 'Router Alerts'}
    subtitle={$t('admin.network.alerts.subtitle') || 'Incidents detected from router polling.'}
  >
    {#snippet actions()}
      <AlertsIncidentsSwitch
        current="alerts"
        nocHref={networkRoute('noc')}
        alertsHref={networkRoute('alerts')}
        incidentsHref={networkRoute('incidents')}
      />

      <button
        class="btn ghost"
        type="button"
        onclick={() => {
          activeOnly = !activeOnly;
          void load();
        }}
        title={$t('admin.network.alerts.actions.toggle') || 'Toggle active/resolved'}
      >
        <Icon name={activeOnly ? 'filter' : 'archive'} size={16} />
        {activeOnly ? $t('admin.network.alerts.actions.active') || 'Active' : $t('admin.network.alerts.actions.all') || 'All'}
      </button>

      <button class="btn ghost" type="button" onclick={load} title={$t('common.refresh') || 'Refresh'}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
    {/snippet}
  </NetworkPageHeader>

  <div class="stats">
    <div class="stat-card">
      <div class="stat-top">
        <span>{$t('common.total') || 'Total'}</span>
        <Icon name="activity" size={16} />
      </div>
      <div class="stat-value">{stats.total}</div>
    </div>
    <div class="stat-card tone-warn">
      <div class="stat-top">
        <span>{$t('admin.network.alerts.filters.all_status') || 'Open'}</span>
        <Icon name="alert-triangle" size={16} />
      </div>
      <div class="stat-value">{stats.open}</div>
    </div>
    <div class="stat-card tone-ok">
      <div class="stat-top">
        <span>{$t('admin.network.incidents.analytics.ack') || 'Ack'}</span>
        <Icon name="check-circle" size={16} />
      </div>
      <div class="stat-value">{stats.ack}</div>
    </div>
    <div class="stat-card tone-bad">
      <div class="stat-top">
        <span>{$t('admin.network.alerts.severity.critical') || 'Critical'}</span>
        <Icon name="shield-alert" size={16} />
      </div>
      <div class="stat-value">{stats.critical}</div>
    </div>
  </div>

  <div class="table-wrap">
    <NetworkFilterPanel>
      <div class="control">
        <label for="alert-filter-status">{$t('admin.network.alerts.columns.status') || 'Status'}</label>
        <select id="alert-filter-status" class="input" bind:value={filterStatus}>
          <option value="all">{$t('admin.network.alerts.filters.all_status') || 'All status'}</option>
          <option value="open">open</option>
          <option value="ack">ack</option>
          <option value="resolved">resolved</option>
        </select>
      </div>

      <div class="control">
        <label for="alert-filter-severity">{$t('admin.network.alerts.columns.severity') || 'Severity'}</label>
        <select id="alert-filter-severity" class="input" bind:value={filterSeverity}>
          <option value="all">{$t('admin.network.alerts.filters.all_severity') || 'All severity'}</option>
          <option value="info">{severityLabel('info')}</option>
          <option value="warning">{severityLabel('warning')}</option>
          <option value="critical">{severityLabel('critical')}</option>
        </select>
      </div>

      <div class="control">
        <label for="alert-filter-type">{$t('admin.network.alerts.columns.type') || 'Type'}</label>
        <select id="alert-filter-type" class="input" bind:value={filterType}>
          <option value="all">{$t('admin.network.alerts.filters.all_types') || 'All types'}</option>
          {#each alertTypeOptions as typeOption}
            <option value={typeOption}>{typeLabel(typeOption)}</option>
          {/each}
        </select>
      </div>

      <div class="control">
        <label for="alert-filter-sort">{$t('admin.network.alerts.filters.sort') || 'Sort'}</label>
        <select id="alert-filter-sort" class="input" bind:value={filterSort}>
          <option value="last_seen_desc">
            {$t('admin.network.alerts.filters.sort_last_seen_desc') || 'Last seen (newest)'}
          </option>
          <option value="last_seen_asc">
            {$t('admin.network.alerts.filters.sort_last_seen_asc') || 'Last seen (oldest)'}
          </option>
          <option value="severity_desc">
            {$t('admin.network.alerts.filters.sort_severity_desc') || 'Severity (highest)'}
          </option>
        </select>
      </div>

      <div class="control">
        <label for="alert-filter-from">{$t('admin.network.alerts.filters.from') || 'From'}</label>
        <input id="alert-filter-from" class="input" type="date" bind:value={filterFrom} />
      </div>

      <div class="control">
        <label for="alert-filter-to">{$t('admin.network.alerts.filters.to') || 'To'}</label>
        <input id="alert-filter-to" class="input" type="date" bind:value={filterTo} />
      </div>

      <div class="control control-actions">
        <div class="control-spacer" aria-hidden="true"></div>
        <button class="btn ghost" type="button" onclick={resetFilters}>
          <Icon name="x-circle" size={14} />
          {$t('admin.network.alerts.filters.reset') || 'Reset'}
        </button>
      </div>
    </NetworkFilterPanel>

    <Table
      {columns}
      data={filteredRows}
      keyField="id"
      {loading}
      pagination={true}
      pageSize={10}
      searchable={true}
      searchPlaceholder={$t('admin.network.alerts.search') || 'Search alerts...'}
      mobileView={isMobile ? 'card' : 'scroll'}
      emptyText={$t('admin.network.alerts.empty') || 'No alerts'}
    >
      {#snippet cell({ item, key }: any)}
        {#if key === 'title'}
          <div class="cell-title">
            <div class="row-top">
              <span class="name">{item.title}</span>
              <span class="chip mono">{typeLabel(item.alert_type)}</span>
            </div>
            <div class="muted">{item.message}</div>
          </div>
        {:else if key === 'type'}
          <span class="chip mono">{typeLabel(item.alert_type)}</span>
        {:else if key === 'severity'}
          <span class="pill" class:critical={item.severity === 'critical'} class:warn={item.severity === 'warning'}>
            {severityLabel(item.severity)}
          </span>
        {:else if key === 'status'}
          <span class="pill" class:ack={item.status === 'ack'} class:resolved={item.status === 'resolved'}>
            {item.status}
          </span>
        {:else if key === 'seen'}
          <span class="muted" title={formatDateTime(item.last_seen_at, { timeZone: $appSettings.app_timezone })}>
            {timeAgo(item.last_seen_at)}
          </span>
        {:else if key === 'actions'}
          <RowActionButtons
            onOpen={() => openRouter(item.router_id)}
            showSnooze={item.status !== 'resolved' && $can('manage', 'network_routers')}
            onSnooze={() => snooze(item.router_id, 30)}
            showAcknowledge={item.status !== 'ack' && item.status !== 'resolved' && $can('manage', 'network_routers')}
            onAcknowledge={() => ack(item.id)}
            showResolve={item.status !== 'resolved' && $can('manage', 'network_routers')}
            onResolve={() => resolve(item.id)}
          />
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
    max-width: 1460px;
    margin: 0 auto;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
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

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    font-weight: 800;
    cursor: pointer;
  }

  .btn:hover {
    background: var(--bg-hover);
  }

  .table-wrap {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 18px;
    overflow: hidden;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.2);
  }

  .cell-title .row-top {
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
    font-weight: 900;
    padding: 3px 8px;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-hover), transparent 20%);
    color: var(--text-secondary);
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
  }

  .muted {
    color: var(--text-secondary);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    padding: 6px 10px;
    border-radius: 999px;
    font-weight: 900;
    font-size: 0.78rem;
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
  }

  .pill.warn {
    background: rgba(245, 158, 11, 0.12);
    border-color: rgba(245, 158, 11, 0.28);
    color: rgba(245, 158, 11, 0.95);
  }

  .pill.critical {
    background: rgba(239, 68, 68, 0.12);
    border-color: rgba(239, 68, 68, 0.28);
    color: rgba(239, 68, 68, 0.95);
  }

  .pill.ack {
    background: rgba(34, 197, 94, 0.12);
    border-color: rgba(34, 197, 94, 0.28);
    color: rgba(34, 197, 94, 0.95);
  }

  .pill.resolved {
    opacity: 0.7;
  }

  @media (max-width: 900px) {
    .page-content {
      padding: 18px;
    }
    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @media (max-width: 640px) {
    .stats {
      grid-template-columns: 1fr;
    }
  }
</style>
