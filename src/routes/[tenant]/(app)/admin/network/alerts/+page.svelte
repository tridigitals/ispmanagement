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
</script>

<div class="page-content fade-in">
  <div class="head">
    <div>
      <h1 class="title">{$t('admin.network.alerts.title') || 'Router Alerts'}</h1>
      <p class="sub">
        {$t('admin.network.alerts.subtitle') || 'Incidents detected from router polling.'}
      </p>
    </div>

    <div class="head-actions">
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
          <div class="actions">
            <button
              class="icon-btn"
              type="button"
              onclick={() => openRouter(item.router_id)}
              title={$t('common.open') || 'Open'}
            >
              <Icon name="arrow-right" size={16} />
            </button>
            {#if item.status !== 'resolved' && ($can('manage', 'network_routers'))}
              <button
                class="icon-btn"
                type="button"
                onclick={() => snooze(item.router_id, 30)}
                title={$t('admin.network.alerts.actions.snooze_30m') || 'Snooze 30m'}
              >
                <Icon name="clock" size={16} />
              </button>
            {/if}
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

  .actions {
    display: inline-flex;
    gap: 8px;
    justify-content: flex-end;
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
  }
</style>
