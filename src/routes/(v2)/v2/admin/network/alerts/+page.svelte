<script lang="ts">
  /*
    Alert jaringan v2 — gelombang 24b.

    Versi lama: (app)/admin/network/alerts/+page.svelte (603 baris).
    Perilaku identik: pantau alert 5 detik, toggle aktif/semua, filter
    status/severity/tipe/rentang tanggal/sort, aksi ack/resolve/snooze
    30 mnt (maintenance router), tautan ke detail router. Bobot/filter/
    label kini dari helper murni networkAlertInsights (5 tes).
  */
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import {
    alertSeverityLabel,
    alertSeverityTone,
    alertStatusTone,
    alertTypeLabel,
    filterAlertRows,
  } from '$lib/utils/networkAlertInsights';
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
  import { t } from 'svelte-i18n';

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

  const columns = $derived<Column[]>([
    { key: 'title', label: $t('network.alerts.v2.col_alert') },
    { key: 'type', label: $t('network.alerts.v2.col_type') },
    { key: 'severity', label: $t('network.alerts.v2.col_sev') },
    { key: 'status', label: $t('network.alerts.v2.col_status') },
    { key: 'seen', label: $t('network.alerts.v2.col_seen') },
    { key: 'actions', label: '' },
  ]);

  let loading = $state(true);
  let refreshing = $state(false);
  let rows = $state<AlertRow[]>([]);
  let activeOnly = $state(true);
  let filterStatus = $state('all');
  let filterSeverity = $state('all');
  let filterType = $state('all');
  let filterFrom = $state('');
  let filterTo = $state('');
  let filterSort = $state('last_seen_desc');
  let refreshHandle: ReturnType<typeof setInterval> | null = null;

  const canManage = $derived($can('manage', 'network_alerts'));

  const typeOptions = $derived([
    { value: 'all', label: $t('network.alerts.v2.all_types') },
    ...Array.from(new Set(rows.map((r) => r.alert_type).filter(Boolean)))
      .sort((a, b) => a.localeCompare(b))
      .map((tp) => ({ value: tp, label: alertTypeLabel(tp, (k) => $t(k)) })),
  ]);
  const statusOptions = $derived([
    { value: 'all', label: $t('network.alerts.v2.all_status') },
    { value: 'open', label: 'open' },
    { value: 'ack', label: 'ack' },
    { value: 'resolved', label: 'resolved' },
  ]);
  const severityOptions = $derived([
    { value: 'all', label: $t('network.alerts.v2.all_sev') },
    { value: 'info', label: 'Info' },
    { value: 'warning', label: $t('network.alerts.v2.sev_warn') },
    { value: 'critical', label: $t('network.alerts.v2.sev_crit') },
  ]);
  const sortOptions = $derived([
    { value: 'last_seen_desc', label: $t('network.alerts.v2.sort_new') },
    { value: 'last_seen_asc', label: $t('network.alerts.v2.sort_old') },
    { value: 'severity_desc', label: $t('network.alerts.v2.sort_sev') },
  ]);

  const filtered = $derived(
    filterAlertRows(rows, {
      status: filterStatus,
      severity: filterSeverity,
      type: filterType,
      from: filterFrom,
      to: filterTo,
      sort: filterSort,
    }),
  );

  const stats = $derived({
    total: rows.length,
    open: rows.filter((r) => r.status === 'open').length,
    ack: rows.filter((r) => r.status === 'ack').length,
    critical: rows.filter((r) => r.severity === 'critical').length,
  });

  onMount(() => {
    if (!$can('read', 'network_alerts') && !canManage) {
      goto('/unauthorized');
      return;
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
      rows = (await api.mikrotik.alerts.list({ activeOnly })) as AlertRow[];
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      loading = false;
    }
  }

  async function refreshSilent() {
    if (refreshing) return;
    refreshing = true;
    try {
      rows = (await api.mikrotik.alerts.list({ activeOnly })) as AlertRow[];
    } catch (e) {
      console.error('refreshSilent gagal:', e);
    } finally {
      refreshing = false;
    }
  }

  async function ack(id: string) {
    try {
      await api.mikrotik.alerts.ack(id);
      toast.success($t('network.alerts.v2.t_ack'));
      void load();
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    }
  }

  async function resolve(id: string) {
    try {
      await api.mikrotik.alerts.resolve(id);
      toast.success($t('network.alerts.v2.t_res'));
      void load();
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    }
  }

  async function snooze(routerId: string, minutes: number) {
    try {
      const until = new Date(Date.now() + minutes * 60 * 1000).toISOString();
      await api.mikrotik.routers.update(routerId, {
        maintenance_until: until,
        maintenance_reason: `Snoozed from alert for ${minutes}m`,
      });
      toast.success($t('network.alerts.v2.t_snooze'));
      void load();
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    }
  }

  function openRouter(routerId: string) {
    goto(`/v2/admin/network/routers/${routerId}`);
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
<AppShell title={ $t('network.alerts.v2.title') }>
  <PageHeader
    title={ $t('network.alerts.v2.title') }
    eyebrow={ $t('admin.eyebrows.network') }
    desc={ $t('network.alerts.v2.desc') }
  >
    {#snippet actions()}
      <Button variant="ghost" href="/v2/admin/network/noc">NOC</Button>
      <Button variant="ghost" href="/v2/admin/network/incidents">Insiden</Button>
      <Button variant="ghost" onclick={() => { activeOnly = !activeOnly; void load(); }}>
        {activeOnly ? $t('network.alerts.v2.active_only') : $t('network.alerts.v2.all')}
      </Button>
      <Button variant="ghost" icon="refresh" onclick={() => void load()} disabled={loading}>
        { $t('common.refresh') }
      </Button>
    {/snippet}
  </PageHeader>

  <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
    <StatTile label={ $t('network.alerts.v2.st_total') } value={String(stats.total)} hint={activeOnly ? $t('network.alerts.v2.h_active') : $t('network.alerts.v2.h_all')} />
    <StatTile label={ $t('network.alerts.v2.st_open') } value={String(stats.open)} hint={ $t('network.alerts.v2.h_unack') } tone="warning" />
    <StatTile label={ $t('network.alerts.v2.st_ack') } value={String(stats.ack)} hint={ $t('network.alerts.v2.h_handling') } tone="positive" />
    <StatTile label={ $t('network.alerts.v2.st_crit') } value={String(stats.critical)} hint={ $t('network.alerts.v2.h_priority') } tone="negative" />
  </div>

  <Card title={ $t('network.alerts.v2.f_filter') }>
    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      <Field id="al-status" label={ $t('network.alerts.v2.f_status') } type="select" stacked value={filterStatus} options={statusOptions} onchange={(v) => (filterStatus = v)} />
      <Field id="al-severity" label={ $t('network.alerts.v2.f_sev') } type="select" stacked value={filterSeverity} options={severityOptions} onchange={(v) => (filterSeverity = v)} />
      <Field id="al-type" label={ $t('network.alerts.v2.f_type') } type="select" stacked value={filterType} options={typeOptions} onchange={(v) => (filterType = v)} />
      <Field id="al-sort" label={ $t('network.alerts.v2.f_sort') } type="select" stacked value={filterSort} options={sortOptions} onchange={(v) => (filterSort = v)} />
      <div>
        <label class="mb-1 block text-xs font-medium text-ink-500" for="al-from">{ $t('common.from') }</label>
        <input id="al-from" class="inp" type="date" bind:value={filterFrom} />
      </div>
      <div>
        <label class="mb-1 block text-xs font-medium text-ink-500" for="al-to">{ $t('common.to') }</label>
        <input id="al-to" class="inp" type="date" bind:value={filterTo} />
      </div>
      <div class="flex items-end">
        <Button variant="ghost" onclick={resetFilters}>{ $t('network.alerts.v2.reset') }</Button>
      </div>
    </div>
  </Card>

  <Card title={`Alert (${filtered.length})`} padded={false}>
    <DataTable pageSize={25}
      {columns}
      rows={filtered}
      {loading}
      emptyTitle="Tidak ada alert cocok filter"
      emptyHint="Ubah filter atau atur ulang."
    >
      {#snippet cell(item, column)}
        {#if column.key === 'title'}
          <div>
            <div class="flex flex-wrap items-center gap-1.5">
              <span class="text-sm font-semibold text-ink-900">{item.title}</span>
              <Badge tone="neutral" label={alertTypeLabel(item.alert_type)} />
            </div>
            <div class="mt-0.5 text-sm text-ink-500">{item.message}</div>
          </div>
        {:else if column.key === 'type'}
          <span class="font-mono text-xs">{alertTypeLabel(item.alert_type)}</span>
        {:else if column.key === 'severity'}
          <Badge tone={alertSeverityTone(item.severity)} label={alertSeverityLabel(item.severity, (k) => $t(k))} />
        {:else if column.key === 'status'}
          <Badge tone={alertStatusTone(item.status)} label={item.status} />
        {:else if column.key === 'seen'}
          <span class="text-xs text-ink-500" title={formatDateTime(item.last_seen_at, { timeZone: $appSettings.app_timezone })}>{timeAgo(item.last_seen_at, (k) => $t(k))}</span>
        {:else if column.key === 'actions'}
          <RowActions
            primary={{ label: $t('network.alerts.v2.open_router'), icon: 'chevronRight', onclick: () => openRouter(item.router_id) }}
            rest={canManage && item.status !== 'resolved'
              ? [
                  ...(item.status !== 'ack' ? [{ label: $t('network.alerts.v2.a_ack'), icon: 'check' as const, onclick: () => void ack(item.id) }] : []),
                  { label: $t('network.alerts.v2.a_resolve'), icon: 'check' as const, onclick: () => void resolve(item.id) },
                  { label: $t('network.alerts.v2.a_snooze'), icon: 'clock' as const, onclick: () => void snooze(item.router_id, 30) },
                ]
              : []}
          />
        {/if}
      {/snippet}
    </DataTable>
  </Card>
</AppShell>

<style>
  .inp {
    width: 100%;
    height: 36px;
    border-radius: 8px;
    border: 1px solid var(--color-ink-200);
    background: #fff;
    padding: 0 10px;
    font-size: 13px;
    color: var(--color-ink-900);
  }
  .inp:focus-visible {
    outline: 2px solid var(--color-brand-600);
    outline-offset: 1px;
  }
</style>
