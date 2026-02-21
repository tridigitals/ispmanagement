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
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { user, tenant } from '$lib/stores/auth';
  import { get } from 'svelte/store';
  import type { TeamMember } from '$lib/api/client';

  type IncidentRow = {
    id: string;
    tenant_id: string;
    router_id: string;
    interface_name?: string | null;
    incident_type: string;
    severity: string;
    status: string;
    title: string;
    message: string;
    owner_user_id?: string | null;
    notes?: string | null;
    last_seen_at: string;
    resolved_at?: string | null;
    updated_at: string;
  };
  type RouterRow = {
    id: string;
    name: string;
    host: string;
    port: number;
    identity?: string | null;
    ros_version?: string | null;
    is_online?: boolean;
    latency_ms?: number | null;
    last_seen_at?: string | null;
    last_error?: string | null;
  };
  type RouterMetricRow = {
    ts: string;
    cpu_load?: number | null;
    rx_bps?: number | null;
    tx_bps?: number | null;
    free_memory_bytes?: number | null;
    total_memory_bytes?: number | null;
  };

  let loading = $state(true);
  let refreshing = $state(false);
  let rows = $state<IncidentRow[]>([]);
  let detailOpen = $state(false);
  let detailLoading = $state(false);
  let detailIncident = $state<IncidentRow | null>(null);
  let detailRouter = $state<RouterRow | null>(null);
  let detailMetric = $state<RouterMetricRow | null>(null);
  let detailSaving = $state(false);
  let selectedOwnerId = $state('');
  let draftNotes = $state('');
  let activeOnly = $state(true);
  let isMobile = $state(false);
  let teamMembers = $state<TeamMember[]>([]);
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
    { key: 'title', label: $t('admin.network.incidents.columns.incident') || 'Incident' },
    { key: 'type', label: $t('admin.network.incidents.columns.type') || 'Type' },
    { key: 'severity', label: $t('admin.network.incidents.columns.severity') || 'Severity' },
    { key: 'status', label: $t('admin.network.incidents.columns.status') || 'Status' },
    { key: 'seen', label: $t('admin.network.incidents.columns.seen') || 'Last Seen' },
    { key: 'actions', label: '', align: 'right' as const, width: '140px' },
  ]);

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
    if ($can('manage', 'network_routers')) {
      void loadTeamMembers();
    }
    refreshHandle = setInterval(() => void refreshSilent(), 5000);

    if (typeof window !== 'undefined') {
      const onKey = (e: KeyboardEvent) => {
        if (e.key === 'Escape' && detailOpen) closeDetail();
      };
      window.addEventListener('keydown', onKey);
      return () => window.removeEventListener('keydown', onKey);
    }
  });

  onDestroy(() => {
    if (refreshHandle) clearInterval(refreshHandle);
  });

  async function load() {
    loading = true;
    try {
      rows = (await api.mikrotik.incidents.list({ activeOnly, limit: 500 })) as any;
      const target = get(page).url.searchParams.get('incident');
      if (target && !detailOpen) {
        const found = rows.find((r) => r.id === target);
        if (found) void openDetail(found);
      }
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
      rows = (await api.mikrotik.incidents.list({ activeOnly, limit: 500 })) as any;
    } catch {
      // ignore background refresh
    } finally {
      refreshing = false;
    }
  }

  async function loadTeamMembers() {
    try {
      teamMembers = await api.team.list();
    } catch {
      // non-blocking
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
      await api.mikrotik.incidents.ack(id);
      toast.success($t('admin.network.alerts.toasts.acked') || 'Alert acknowledged');
      await load();
      if (detailIncident?.id === id) {
        detailIncident = rows.find((r) => r.id === id) || null;
      }
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function resolve(id: string) {
    try {
      await api.mikrotik.incidents.resolve(id);
      toast.success($t('admin.network.alerts.toasts.resolved') || 'Alert resolved');
      await load();
      if (detailIncident?.id === id) {
        detailIncident = rows.find((r) => r.id === id) || null;
      }
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  function openRouter(routerId: string) {
    goto($page.url.pathname.replace(/\/admin\/network\/incidents\/?$/, `/admin/network/routers/${routerId}`));
  }

  async function openDetail(item: IncidentRow) {
    detailOpen = true;
    detailLoading = true;
    detailIncident = item;
    detailRouter = null;
    detailMetric = null;
    selectedOwnerId = item.owner_user_id || '';
    draftNotes = item.notes || '';
    if (typeof window !== 'undefined') {
      const u = new URL(window.location.href);
      u.searchParams.set('incident', item.id);
      history.replaceState({}, '', `${u.pathname}${u.search}${u.hash}`);
    }
    try {
      const [router, metrics] = await Promise.all([
        api.mikrotik.routers.get(item.router_id),
        api.mikrotik.routers.metrics(item.router_id, 1),
      ]);
      detailRouter = (router || null) as RouterRow | null;
      detailMetric = Array.isArray(metrics) && metrics.length > 0 ? (metrics[0] as RouterMetricRow) : null;
    } catch {
      // keep incident detail visible even if router snapshot fails
    } finally {
      detailLoading = false;
    }
  }

  function ownerLabel(ownerUserId?: string | null) {
    if (!ownerUserId) return $t('admin.network.incidents.drawer.unassigned') || 'Unassigned';
    const member = teamMembers.find((m) => m.user_id === ownerUserId || m.id === ownerUserId);
    if (!member) return ownerUserId;
    return `${member.name} (${member.email})`;
  }

  async function saveIncidentMeta() {
    if (!detailIncident) return;
    detailSaving = true;
    try {
      const updated = await api.mikrotik.incidents.update(detailIncident.id, {
        ownerUserId: selectedOwnerId || null,
        notes: draftNotes.trim() ? draftNotes.trim() : null,
      });
      const merged: IncidentRow = {
        ...detailIncident,
        owner_user_id: updated?.owner_user_id ?? null,
        notes: updated?.notes ?? null,
        status: updated?.status ?? detailIncident.status,
        updated_at: updated?.updated_at ?? detailIncident.updated_at,
      };
      detailIncident = merged;
      rows = rows.map((r) => (r.id === merged.id ? { ...r, ...merged } : r));
      toast.success($t('common.saved') || 'Saved');
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      detailSaving = false;
    }
  }

  function closeDetail() {
    detailOpen = false;
    detailIncident = null;
    if (typeof window !== 'undefined') {
      const u = new URL(window.location.href);
      u.searchParams.delete('incident');
      history.replaceState({}, '', `${u.pathname}${u.search}${u.hash}`);
    }
  }

  function networkRoute(to: 'noc' | 'alerts' | 'incidents') {
    return `${tenantPrefix}/admin/network/${to}`;
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

  function memoryUsePct(total?: number | null, free?: number | null) {
    if (!total || total <= 0 || free == null) return null;
    const used = total - free;
    return Math.max(0, Math.min(100, Math.round((used / total) * 100)));
  }
</script>

<div class="page-content fade-in">
  <div class="head">
    <div>
      <h1 class="title">{$t('admin.network.incidents.title') || 'Network Incidents'}</h1>
      <p class="sub">
        {$t('admin.network.incidents.subtitle') || 'Operational incident records deduplicated from alerts.'}
      </p>
    </div>

    <div class="head-actions">
      <AlertsIncidentsSwitch
        current="incidents"
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
        title={$t('admin.network.incidents.actions.toggle') || 'Toggle active/resolved'}
      >
        <Icon name={activeOnly ? 'filter' : 'archive'} size={16} />
        {activeOnly
          ? $t('admin.network.incidents.actions.active') || 'Active'
          : $t('admin.network.incidents.actions.all') || 'All'}
      </button>

      <button class="btn ghost" type="button" onclick={load} title={$t('common.refresh') || 'Refresh'}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
    </div>
  </div>

  <div class="table-wrap">
    <Table
      {columns}
      data={rows}
      keyField="id"
      {loading}
      pagination={true}
      pageSize={10}
      searchable={true}
      searchPlaceholder={$t('admin.network.incidents.search') || 'Search incidents...'}
      mobileView={isMobile ? 'card' : 'scroll'}
      emptyText={$t('admin.network.incidents.empty') || 'No incidents'}
    >
      {#snippet cell({ item, key }: any)}
        {#if key === 'title'}
          <div class="cell-title">
            <div class="row-top">
              <button class="name-link" type="button" onclick={() => void openDetail(item)}>{item.title}</button>
              <span class="chip mono">{item.interface_name || '-'}</span>
            </div>
            <div class="muted">{item.message}</div>
          </div>
        {:else if key === 'type'}
          <span class="chip mono">{typeLabel(item.incident_type)}</span>
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
          <div class="actions">
            <button class="icon-btn" type="button" onclick={() => openRouter(item.router_id)} title={$t('common.open') || 'Open'}>
              <Icon name="arrow-right" size={16} />
            </button>
            <button class="icon-btn" type="button" onclick={() => void openDetail(item)} title={$t('common.details') || 'Details'}>
              <Icon name="file-text" size={16} />
            </button>
            {#if item.status !== 'ack' && item.status !== 'resolved' && ($can('manage', 'network_routers'))}
              <button
                class="icon-btn"
                type="button"
                onclick={() => ack(item.id)}
                title={$t('admin.network.alerts.actions.ack') || 'Acknowledge'}
              >
                <Icon name="check" size={16} />
              </button>
            {/if}
            {#if item.status !== 'resolved' && ($can('manage', 'network_routers'))}
              <button
                class="icon-btn"
                type="button"
                onclick={() => resolve(item.id)}
                title={$t('admin.network.alerts.actions.resolve') || 'Resolve'}
              >
                <Icon name="check-circle" size={16} />
              </button>
            {/if}
          </div>
        {:else}
          {item[key] ?? ''}
        {/if}
      {/snippet}
    </Table>
  </div>

  {#if detailOpen && detailIncident}
    {@const incident = detailIncident}
    <button class="drawer-backdrop" type="button" onclick={closeDetail} aria-label={$t('common.close') || 'Close'}></button>
    <aside class="drawer" aria-label={$t('common.details') || 'Details'}>
      <div class="drawer-head">
        <div>
          <div class="drawer-title">{$t('common.details') || 'Details'}</div>
          <div class="drawer-sub">{incident.title}</div>
        </div>
        <button class="icon-btn" type="button" onclick={closeDetail} title={$t('common.close') || 'Close'}>
          <Icon name="x" size={16} />
        </button>
      </div>

      <div class="drawer-body">
        {#if detailLoading}
          <div class="muted">{$t('common.loading') || 'Loading...'}</div>
        {/if}

        <div class="detail-grid">
          <div class="drow"><span class="muted">{$t('admin.network.incidents.columns.status') || 'Status'}</span><span class="mono">{incident.status}</span></div>
          <div class="drow"><span class="muted">{$t('admin.network.incidents.columns.type') || 'Type'}</span><span class="mono">{typeLabel(incident.incident_type)}</span></div>
          <div class="drow"><span class="muted">{$t('admin.network.incidents.columns.severity') || 'Severity'}</span><span class="mono">{severityLabel(incident.severity)}</span></div>
          <div class="drow"><span class="muted">{$t('admin.network.incidents.columns.seen') || 'Last Seen'}</span><span class="mono">{formatDateTime(incident.last_seen_at, { timeZone: $appSettings.app_timezone })}</span></div>
          <div class="drow"><span class="muted">{$t('admin.network.incidents.drawer.assignee') || 'Assignee'}</span><span class="mono">{ownerLabel(incident.owner_user_id)}</span></div>
          <div class="drow"><span class="muted">Router</span><span class="mono">{detailRouter?.identity || detailRouter?.name || incident.router_id}</span></div>
          <div class="drow"><span class="muted">Interface</span><span class="mono">{incident.interface_name || '-'}</span></div>
          {#if detailRouter}
            <div class="drow"><span class="muted">Host</span><span class="mono">{detailRouter.host}:{detailRouter.port}</span></div>
            <div class="drow"><span class="muted">Latency</span><span class="mono">{detailRouter.latency_ms == null ? '—' : `${detailRouter.latency_ms} ms`}</span></div>
          {/if}
          {#if detailMetric}
            <div class="drow"><span class="muted">CPU</span><span class="mono">{detailMetric.cpu_load == null ? '—' : `${detailMetric.cpu_load}%`}</span></div>
            <div class="drow"><span class="muted">RX/TX</span><span class="mono">{formatBps(detailMetric.rx_bps)} / {formatBps(detailMetric.tx_bps)}</span></div>
            <div class="drow"><span class="muted">Memory Use</span><span class="mono">{memoryUsePct(detailMetric.total_memory_bytes, detailMetric.free_memory_bytes) == null ? '—' : `${memoryUsePct(detailMetric.total_memory_bytes, detailMetric.free_memory_bytes)}%`}</span></div>
          {/if}
        </div>

        <div class="detail-message">{incident.message}</div>

        <div class="detail-edit">
          <div class="field">
            <label for="incident-owner">{$t('admin.network.incidents.drawer.assignee') || 'Assignee'}</label>
            {#if $can('manage', 'network_routers')}
              <select id="incident-owner" class="input" bind:value={selectedOwnerId}>
                <option value="">{($t('admin.network.incidents.drawer.unassigned') || 'Unassigned')}</option>
                {#each teamMembers as member}
                  <option value={member.user_id}>{member.name} ({member.email})</option>
                {/each}
              </select>
            {:else}
              <div class="readonly">{ownerLabel(incident.owner_user_id)}</div>
            {/if}
          </div>
          <div class="field">
            <label for="incident-notes">{$t('admin.network.incidents.drawer.notes') || 'Notes'}</label>
            {#if $can('manage', 'network_routers')}
              <textarea
                id="incident-notes"
                class="textarea"
                rows="4"
                bind:value={draftNotes}
                placeholder={$t('admin.network.incidents.drawer.notes_placeholder') || 'Add operator notes, handover context, and actions...'}
              ></textarea>
            {:else}
              <div class="readonly">{incident.notes || ($t('common.na') || '—')}</div>
            {/if}
          </div>
          {#if $can('manage', 'network_routers')}
            <div class="save-row">
              <button class="btn ghost" type="button" onclick={saveIncidentMeta} disabled={detailSaving}>
                <Icon name="save" size={16} />
                {detailSaving
                  ? $t('common.saving') || 'Saving...'
                  : $t('admin.network.incidents.drawer.save') || 'Save Notes'}
              </button>
            </div>
          {/if}
        </div>
      </div>

      <div class="drawer-actions">
        <button class="btn ghost" type="button" onclick={() => openRouter(incident.router_id)}>
          <Icon name="arrow-right" size={16} />
          {$t('common.open') || 'Open'}
        </button>
        {#if incident.status !== 'ack' && incident.status !== 'resolved' && ($can('manage', 'network_routers'))}
          <button class="btn ghost" type="button" onclick={() => ack(incident.id)}>
            <Icon name="check" size={16} />
            {$t('admin.network.alerts.actions.ack') || 'Acknowledge'}
          </button>
        {/if}
        {#if incident.status !== 'resolved' && ($can('manage', 'network_routers'))}
          <button class="btn ghost" type="button" onclick={() => resolve(incident.id)}>
            <Icon name="check-circle" size={16} />
            {$t('admin.network.alerts.actions.resolve') || 'Resolve'}
          </button>
        {/if}
      </div>
    </aside>
  {/if}
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
    gap: 10px;
    flex-wrap: wrap;
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
  }
  .cell-title {
    display: grid;
    gap: 6px;
  }
  .row-top {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .name-link {
    border: 0;
    background: transparent;
    color: var(--text-primary);
    font-weight: 850;
    padding: 0;
    cursor: pointer;
    text-align: left;
  }
  .name-link:hover {
    color: var(--accent);
  }
  .chip {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 3px 8px;
    font-size: 0.76rem;
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
  }
  .pill {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 4px 10px;
    font-size: 0.78rem;
    font-weight: 850;
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
    text-transform: capitalize;
  }
  .pill.warn {
    border-color: color-mix(in srgb, var(--color-warning) 45%, var(--border-color));
    color: var(--color-warning);
  }
  .pill.critical {
    border-color: color-mix(in srgb, var(--color-danger) 45%, var(--border-color));
    color: var(--color-danger);
  }
  .pill.ack {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border-color));
  }
  .pill.resolved {
    opacity: 0.75;
  }
  .actions {
    display: inline-flex;
    gap: 6px;
    justify-content: flex-end;
    width: 100%;
  }
  .icon-btn {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .icon-btn:hover {
    background: var(--bg-hover);
  }
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    border: 0;
    background: rgba(0, 0, 0, 0.45);
    z-index: 50;
  }
  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    width: min(560px, 92vw);
    height: 100vh;
    background: var(--bg-surface);
    border-left: 1px solid var(--border-color);
    z-index: 51;
    display: grid;
    grid-template-rows: auto 1fr auto;
  }
  .drawer-head {
    padding: 16px;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  .drawer-title {
    font-size: 0.78rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }
  .drawer-sub {
    margin-top: 6px;
    font-size: 1.05rem;
    font-weight: 900;
    color: var(--text-primary);
  }
  .drawer-body {
    padding: 16px;
    display: grid;
    gap: 14px;
    overflow: auto;
  }
  .detail-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }
  .drow {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 10px;
    display: grid;
    gap: 4px;
  }
  .detail-message {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 12px;
    color: var(--text-primary);
    line-height: 1.45;
    white-space: pre-wrap;
  }
  .detail-edit {
    display: grid;
    gap: 10px;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 12px;
    background: color-mix(in srgb, var(--bg-card) 70%, transparent);
  }
  .field {
    display: grid;
    gap: 6px;
  }
  .field label {
    color: var(--text-secondary);
    font-size: 0.8rem;
    font-weight: 700;
  }
  .input,
  .textarea {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 10px 12px;
    outline: none;
  }
  .input:focus,
  .textarea:focus {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border-color));
  }
  .readonly {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: color-mix(in srgb, var(--bg-surface) 75%, transparent);
    color: var(--text-primary);
    padding: 10px 12px;
    white-space: pre-wrap;
  }
  .save-row {
    display: flex;
    justify-content: flex-end;
  }
  .drawer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
    padding: 12px 16px;
    border-top: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card) 80%, transparent);
  }
  @media (max-width: 720px) {
    .detail-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
