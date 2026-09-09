<script lang="ts">
  /*
    Insiden Jaringan v2 (gelombang 19).

    Versi lama: `(app)/admin/network/incidents/+page.svelte` (1.603 baris).

    Temuan yang dikunci gelombang ini (backend sudah dipatch):
    1. Ack id tak dikenal / sudah resolved balas 200 hampa — bulk ack
       melaporkan "berhasil" padahal tidak ada yang berubah. Kini 404/409.
    2. Ack tidak pernah tercatat di audit log — jejak "siapa mengakui
       insiden ini" hilang (update & resolve dicatat; ack tidak).
    3. owner_user_id tak divalidasi — assign ke user asing/terhapus lolos
       ke DB; notifikasi dilewati diam-diam. Kini 400.
    4. ?limit=999999 menarik seluruh tabel — di-clamp 1..1000.

    FE lama juga: filter/search hanya sisi klien atas maksimal 500 baris
    tanpa batas waktu; v2 memakai limit server 1000 + filter terukur.
  */
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { api } from '$lib/api/client';
  import type { TeamMember } from '$lib/api/client';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { formatDateTime } from '$lib/utils/date';
  import { toast } from '$lib/stores/toast';
  import {
    formatDurationCompact,
    friendlyIncidentError,
    incidentCounts,
    incidentOpenMs,
    incidentStartMs,
    meanTimeToAck,
    meanTimeToResolve,
    severityLabel,
    severityWeight,
    slaLevel,
    statusLabel,
  } from '$lib/utils/incidentInsights';
  import AppShell from '$lib/components/ds/AppShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import DataTable from '$lib/components/ds/DataTable.svelte';
  import RowActions from '$lib/components/ds/RowActions.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Field from '$lib/components/ds/Field.svelte';
  import AttentionPanel from '$lib/components/ds/AttentionPanel.svelte';
  import type { AttentionItem } from '$lib/components/ds/AttentionPanel.svelte';
  import type { Column } from '$lib/components/ds/table-types';
  import type { StatusTone } from '$lib/components/ds/tokens';

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
    is_auto_escalated?: boolean;
    escalated_at?: string | null;
    first_seen_at?: string;
    acked_at?: string | null;
    acked_by?: string | null;
    last_seen_at: string;
    resolved_at?: string | null;
    updated_at: string;
  };

  type RouterRow = { id: string; name: string; host: string; is_online?: boolean };

  const canManage = $derived($can('manage', 'network_incidents'));

  let loading = $state(true);
  let loadError = $state<string | null>(null);
  let rows = $state<IncidentRow[]>([]);
  let routers = $state<RouterRow[]>([]);
  let teamMembers = $state<TeamMember[]>([]);
  let activeOnly = $state(true);
  let search = $state('');
  let severityFilter = $state<'all' | 'critical' | 'warning' | 'info'>('all');
  let routerFilter = $state('all');
  let statusFilter = $state<'all' | 'open' | 'ack' | 'in_progress' | 'resolved'>('all');
  // A-16: paginasi & agregat server-side.
  let incPage = $state(1);
  const perPage = 25;
  let total = $state(0);
  let stats = $state<{
    total: number; open: number; ack: number; in_progress: number; resolved: number;
    mtta_minutes: number | null; mttr_minutes: number | null; breach_count: number | null;
  } | null>(null);
  let nowMs = $state(Date.now());
  let slaWarnMinutes = $state(30);
  let slaBreachMinutes = $state(120);

  // seleksi massal
  let selectedIds = $state<string[]>([]);
  let bulkBusy = $state(false);
  let bulkOwner = $state('');

  // modal detail
  let showDetail = $state(false);
  let detail = $state<IncidentRow | null>(null);
  let detailOwner = $state('');
  let detailNotes = $state('');
  let detailSaving = $state(false);
  let detailError = $state<string | null>(null);

  // modal simulate
  let showSimulate = $state(false);
  let simRouter = $state('');
  let simType = $state('');
  let simSeverity = $state<'info' | 'warning' | 'critical'>('warning');
  let simInterface = $state('');
  let simMessage = $state('');
  let simBusy = $state(false);
  let simError = $state<string | null>(null);

  let escalationBusy = $state(false);

  // Kartu & MTTA/MTTR: agregat server (jujur atas SEMUA baris cocok filter);
  // fallback lokal hanya selama stats belum tiba.
  const counts = $derived(
    stats
      ? {
          open: Number(stats.open ?? 0),
          ack: Number(stats.ack ?? 0),
          inProgress: Number(stats.in_progress ?? 0),
          resolved: Number(stats.resolved ?? 0),
        }
      : incidentCounts(rows),
  );
  const mtta = $derived(stats?.mtta_minutes ?? meanTimeToAck(rows, nowMs));
  const mttr = $derived(stats?.mttr_minutes ?? meanTimeToResolve(rows));

  const routerById = $derived(new Map(routers.map((r) => [r.id, r])));
  const memberById = $derived(new Map(teamMembers.map((m) => [m.user_id, m])));

  // Tabel = halaman dari server; urutan severity->waktu dipastikan di SQL.
  const filtered = $derived(rows);

  const breachCount = $derived(
    stats?.breach_count ??
      rows.filter((r) => slaLevel(r, nowMs, slaWarnMinutes, slaBreachMinutes) === 'breach').length,
  );

  const attentionItems = $derived<AttentionItem[]>([
    ...(breachCount
      ? [
          {
            icon: 'alert' as const,
            title: $t('admin.network.incidents.ui.sla_title', { values: { n: breachCount } }),
            detail: $t('admin.network.incidents.ui.sla_hint', {
              values: { n: breachCount, b: slaBreachMinutes },
            }),
            action: 'Lihat riwayat',
            href: '/v2/admin/network/incidents?active_only=0',
          },
        ]
      : []),

  ]);

  const selectedRows = $derived(rows.filter((r) => selectedIds.includes(r.id)));

  function filterParams() {
    return {
      activeOnly,
      severity: severityFilter === 'all' ? undefined : severityFilter,
      status: statusFilter === 'all' ? undefined : statusFilter,
      routerId: routerFilter === 'all' ? undefined : routerFilter,
      q: search.trim() || undefined,
    };
  }

  // A-16: daftar + agregat keduanya datang dari server dengan filter yang sama.
  async function loadData() {
    loading = true;
    loadError = null;
    try {
      const [list, st] = await Promise.all([
        api.mikrotik.incidents.search({ ...filterParams(), page: incPage, perPage }),
        api.mikrotik.incidents
          .stats({ ...filterParams(), slaBreachMinutes: slaBreachMinutes })
          .catch((e) => {
            console.error('incident stats gagal:', e);
            return null;
          }),
      ]);
      rows = (list?.data || []) as IncidentRow[];
      total = Number(list?.total ?? rows.length);
      stats = st;
      const stillThere = new Set(rows.map((r) => r.id));
      selectedIds = selectedIds.filter((id) => stillThere.has(id));
    } catch (e) {
      loadError = friendlyIncidentError(extractApiErrorMessage(e));
    } finally {
      loading = false;
    }
  }

  /** Perubahan filter apa pun: reset ke halaman 1 lalu tarik ulang. */
  function refilter() {
    incPage = 1;
    void loadData();
  }

  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  function onSearchInput() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(refilter, 350);
  }

  onMount(() => {
    const timer = setInterval(() => (nowMs = Date.now()), 30000);
    void (async () => {
      try {
        const [rts, team, warn, breach] = await Promise.all([
          api.mikrotik.routers.list().catch((e) => { console.error('routers list gagal:', e); return [] as RouterRow[]; }),
          api.team.list().catch((e) => { console.error('team list gagal:', e); return [] as TeamMember[]; }),
          api.settings.getValue('mikrotik_incident_sla_warn_minutes').catch((e) => { console.error('sla warn gagal:', e); return null; }),
          api.settings.getValue('mikrotik_incident_sla_breach_minutes').catch((e) => { console.error('sla breach gagal:', e); return null; }),
        ]);
        routers = (rts || []) as RouterRow[];
        teamMembers = (team || []) as TeamMember[];
        const w = Number(warn);
        const b = Number(breach);
        if (Number.isFinite(w) && w > 0) slaWarnMinutes = w;
        if (Number.isFinite(b) && b > 0) slaBreachMinutes = b;
        await loadData();
        // deep-link notifikasi: ?incident=<id>
        const target = $page.url.searchParams.get('incident');
        if (target) {
          const found = rows.find((r) => r.id === target);
          if (found) openDetail(found);
        }
      } catch (e) {
        loadError = friendlyIncidentError(extractApiErrorMessage(e));
      } finally {
        loading = false;
      }
    })();
    return () => {
      clearInterval(timer);
      clearTimeout(searchTimer);
    };
  });

  function toggleSelected(id: string) {
    selectedIds = selectedIds.includes(id)
      ? selectedIds.filter((x) => x !== id)
      : [...selectedIds, id];
  }

  function toggleSelectAll() {
    const ids = filtered.map((r) => r.id);
    const allSel = ids.length > 0 && ids.every((id) => selectedIds.includes(id));
    selectedIds = allSel ? selectedIds.filter((id) => !ids.includes(id)) : Array.from(new Set([...selectedIds, ...ids]));
  }

  async function ackOne(id: string) {
    try {
      await api.mikrotik.incidents.ack(id);
      await loadData();
    } catch (e) {
      toast.error(friendlyIncidentError(extractApiErrorMessage(e)));
    }
  }

  async function resolveOne(id: string) {
    try {
      await api.mikrotik.incidents.resolve(id);
      await loadData();
    } catch (e) {
      toast.error(friendlyIncidentError(extractApiErrorMessage(e)));
    }
  }

  async function bulk(action: 'ack' | 'resolve') {
    const targets = selectedRows.filter((r) => (action === 'resolve' ? r.status !== 'resolved' : true));
    if (!targets.length) return;
    bulkBusy = true;
    const results = await Promise.allSettled(
      targets.map((r) => (action === 'ack' ? api.mikrotik.incidents.ack(r.id) : api.mikrotik.incidents.resolve(r.id))),
    );
    const ok = results.filter((r) => r.status === 'fulfilled').length;
    const failed = results.filter((r) => r.status === 'rejected').length;
    bulkBusy = false;
    selectedIds = [];
    await loadData();
    if (failed) toast.error(`${ok} berhasil, ${failed} gagal: ${friendlyIncidentError(extractApiErrorMessage((results.find((r) => r.status === 'rejected') as PromiseRejectedResult).reason))}`);
    else toast.success(`${ok} insiden ${action === 'ack' ? 'diakui' : 'diselesaikan'}.`);
  }

  async function bulkAssign() {
    if (!bulkOwner) return;
    bulkBusy = true;
    const results = await Promise.allSettled(
      selectedRows.map((r) => api.mikrotik.incidents.update(r.id, { ownerUserId: bulkOwner })),
    );
    const ok = results.filter((r) => r.status === 'fulfilled').length;
    bulkBusy = false;
    selectedIds = [];
    await loadData();
    toast.success(`${ok} insiden dialihkan.`);
  }

  async function runEscalation() {
    escalationBusy = true;
    try {
      const res = await api.mikrotik.incidents.runAutoEscalation();
      toast.success(`${Number(res?.escalated ?? 0)} insiden dieskalasi.`);
      await loadData();
    } catch (e) {
      toast.error(friendlyIncidentError(extractApiErrorMessage(e)));
    } finally {
      escalationBusy = false;
    }
  }

  function openDetail(item: IncidentRow) {
    showDetail = true;
    detail = item;
    detailOwner = item.owner_user_id ?? '';
    detailNotes = item.notes ?? '';
    detailError = null;
  }

  function closeDetail() {
    showDetail = false;
    detail = null;
    const u = new URL($page.url);
    if (u.searchParams.has('incident')) {
      u.searchParams.delete('incident');
      history.replaceState({}, '', `${u.pathname}${u.search}${u.hash}`);
    }
  }

  async function saveDetail() {
    if (!detail) return;
    const targetId = detail.id;
    detailSaving = true;
    detailError = null;
    try {
      const updated = await api.mikrotik.incidents.update(targetId, {
        ownerUserId: detailOwner || null,
        notes: detailNotes,
      });
      rows = rows.map((r) => (r.id === targetId ? ((updated as IncidentRow) ?? { ...r, owner_user_id: detailOwner || null, notes: detailNotes }) : r));
      detail = rows.find((r) => r.id === targetId) ?? detail;
      toast.success('Insiden diperbarui.');
    } catch (e) {
      detailError = friendlyIncidentError(extractApiErrorMessage(e));
    } finally {
      detailSaving = false;
    }
  }

  async function submitSimulate() {
    if (!simRouter || !simType) {
      simError = 'Router dan tipe insiden wajib diisi.';
      return;
    }
    simBusy = true;
    simError = null;
    try {
      await api.mikrotik.incidents.simulate({
        routerId: simRouter,
        incidentType: simType.trim(),
        severity: simSeverity,
        interfaceName: simInterface.trim() || null,
        message: simMessage.trim() || null,
      });
      showSimulate = false;
      simType = '';
      simInterface = '';
      simMessage = '';
      toast.success('Insiden simulasi dibuat.');
      await loadData();
    } catch (e) {
      simError = friendlyIncidentError(extractApiErrorMessage(e));
    } finally {
      simBusy = false;
    }
  }

  function sevTone(sev: string): StatusTone {
    if (sev === 'critical') return 'negative';
    if (sev === 'warning') return 'warning';
    return 'info';
  }
  function statTone(st: string): StatusTone {
    if (st === 'open') return 'negative';
    if (st === 'ack') return 'warning';
    if (st === 'in_progress') return 'info';
    return 'positive';
  }
  function slaTone(row: IncidentRow): StatusTone {
    const lvl = slaLevel(row, nowMs, slaWarnMinutes, slaBreachMinutes);
    return lvl === 'breach' ? 'negative' : lvl === 'warn' ? 'warning' : 'neutral';
  }
  function ownerLabel(id?: string | null): string {
    if (!id) return 'Belum ada';
    const m = memberById.get(id);
    return m ? m.name : id.slice(0, 8);
  }
  function routerLabel(id: string): string {
    return routerById.get(id)?.name ?? id.slice(0, 8);
  }
  function typeLabel(t: string): string {
    const map: Record<string, string> = {
      offline: 'Offline',
      latency: 'Latensi',
      cpu: 'CPU',
      router_down: 'Router mati',
      packet_loss: 'Paket hilang',
    };
    return map[t] ?? t;
  }

  const columns = $derived<Column[]>([
    { key: 'sel', label: '', width: '36px' },
    { key: 'title', label: $t('admin.network.incidents.columns.incident') },
    { key: 'severity', label: $t('admin.network.incidents.columns.severity'), width: '150px' },
    { key: 'status', label: $t('admin.network.incidents.columns.status'), width: '120px' },
    { key: 'duration', label: $t('admin.network.incidents.ui.col_duration'), width: '110px' },
    { key: 'owner', label: $t('admin.network.incidents.ui.col_owner'), width: '140px' },
    { key: 'last_seen', label: $t('admin.network.incidents.columns.seen'), width: '150px' },
    { key: 'actions', label: '', width: '150px', align: 'right' },
  ]);

  const tiles = $derived([
    { st: 'open' as const, label: $t('admin.network.incidents.ui.stats.open_title'), value: counts.open },
    { st: 'ack' as const, label: $t('admin.network.incidents.ui.stats.ack_title'), value: counts.ack },
    { st: 'in_progress' as const, label: $t('admin.network.incidents.ui.stats_prog'), value: counts.inProgress },
    { st: 'resolved' as const, label: $t('admin.network.incidents.ui.stats_resolved'), value: counts.resolved },
  ]);
</script>

<AppShell title={$t("admin.network.incidents.title")}>
  <PageHeader title={$t("admin.network.incidents.title")} desc={$t("admin.network.incidents.ui.page_desc")}>
    {#snippet actions()}
      {#if canManage}
        <Button variant="ghost" icon="alert" disabled={escalationBusy} onclick={() => void runEscalation()}>
          {escalationBusy ? $t('admin.network.incidents.ui.running') : $t('admin.network.incidents.ui.run_escalation')}
        </Button>
        <Button variant="ghost" icon="plus" onclick={() => (showSimulate = true)}>{$t("admin.network.incidents.actions.simulate")}</Button>
      {/if}
    {/snippet}
  </PageHeader>

  {#if loadError}
    <div class="mt-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-700">
      {loadError}
      <button type="button" class="ml-2 underline" onclick={() => (loadError = null)}>{$t("common.close")}</button>
    </div>
  {/if}

  <div class="mt-4 grid grid-cols-2 gap-3 lg:grid-cols-6">
    {#each tiles as tile (tile.st)}
      <button
        type="button"
        class="focus-ring rounded-xl text-left {statusFilter === tile.st ? 'ring-2 ring-ink-900' : ''}"
        aria-pressed={statusFilter === tile.st}
        onclick={() => {
          statusFilter = statusFilter === tile.st ? 'all' : tile.st;
          refilter();
        }}
      >
        <StatTile
          label={tile.label}
          value={String(tile.value)}
          hint={$t("admin.network.incidents.ui.tile_hint")}
          tone={tile.st === 'resolved' ? 'positive' : tile.st === 'open' ? 'negative' : 'neutral'}
        />
      </button>
    {/each}
    <StatTile label={$t("admin.network.incidents.ui.stats.mtta_title")} value={mtta == null ? '—' : formatDurationCompact(mtta * 60000)} hint={$t("admin.network.incidents.ui.stats.mtta_hint")} />
    <StatTile label={$t("admin.network.incidents.ui.stats.mttr_title")} value={mttr == null ? '—' : formatDurationCompact(mttr * 60000)} hint={$t("admin.network.incidents.ui.stats.mttr_hint")} />
  </div>

  {#if attentionItems.length}
    <div class="mt-4">
      <AttentionPanel items={attentionItems} />
    </div>
  {/if}

  <div class="mt-4 flex flex-wrap items-center gap-2">
    <label class="flex items-center gap-2 text-sm text-ink-700">
      <input type="checkbox" class="h-6 w-6 accent-ink-900" bind:checked={activeOnly} onchange={refilter} />
      {$t("admin.network.incidents.ui.active_only")}
    </label>
    <select class="focus-ring h-9 rounded-lg bg-white text-sm ring-1 ring-inset ring-ink-200" bind:value={severityFilter} onchange={refilter} aria-label={$t("admin.network.incidents.ui.severity_filter")}>
      <option value="all">{$t("admin.network.incidents.ui.all_severities")}</option>
      <option value="critical">{$t("admin.network.incidents.ui.severities.critical")}</option>
      <option value="warning">{$t("admin.network.incidents.ui.severities.warning")}</option>
      <option value="info">{$t("admin.network.incidents.ui.severities.info")}</option>
    </select>
    <select class="focus-ring h-9 rounded-lg bg-white text-sm ring-1 ring-inset ring-ink-200" bind:value={routerFilter} onchange={refilter} aria-label={$t("admin.network.incidents.ui.router_filter")}>
      <option value="all">{$t("admin.network.incidents.ui.all_routers")}</option>
      {#each routers as r (r.id)}
        <option value={r.id}>{r.name}</option>
      {/each}
    </select>
    {#if statusFilter !== 'all'}
      <button type="button" class="focus-ring rounded-full bg-ink-100 px-3 py-1 text-sm" onclick={() => { statusFilter = 'all'; refilter(); }}>
        {$t("admin.network.incidents.ui.status_chip", { values: { s: statusLabel(statusFilter) } })}
      </button>
    {/if}
    <div class="relative ml-auto min-w-[220px]">
      <span class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-ink-400">⌕</span>
      <input
        bind:value={search}
        oninput={onSearchInput}
        placeholder={$t("admin.network.incidents.ui.search_placeholder")}
        aria-label={$t("admin.network.incidents.search")}
        class="focus-ring h-9 w-full rounded-lg border-0 bg-white pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400"
      />
    </div>
  </div>

  {#if selectedIds.length > 0 && canManage}
    <div class="mt-3 flex flex-wrap items-center gap-2 rounded-lg border border-ink-200 bg-white p-2">
      <span class="rounded-full bg-ink-100 px-3 py-1 text-sm text-ink-700">{selectedIds.length} {$t("admin.network.incidents.ui.selected_suffix")}</span>
      <Button variant="ghost" size="sm" disabled={bulkBusy} onclick={() => void bulk('ack')}>{$t("admin.network.incidents.ui.bulk_ack")}</Button>
      <Button variant="ghost" size="sm" disabled={bulkBusy} onclick={() => void bulk('resolve')}>Selesaikan terpilih</Button>
      <select class="focus-ring h-8 rounded-lg bg-white text-sm ring-1 ring-inset ring-ink-200" bind:value={bulkOwner} aria-label="PIC baru">
        <option value="">Pilih PIC…</option>
        {#each teamMembers as m (m.user_id)}
          <option value={m.user_id}>{m.name}</option>
        {/each}
      </select>
      <Button variant="ghost" size="sm" disabled={bulkBusy || !bulkOwner} onclick={() => void bulkAssign()}>Alihkan</Button>
      <Button variant="ghost" size="sm" onclick={() => (selectedIds = [])}>{$t("admin.network.incidents.ui.clear_selection")}</Button>
    </div>
  {/if}

  <div class="mt-3">
    <DataTable
      {columns}
      rows={filtered}
      pageSize={perPage}
      total={total}
      page={incPage}
      onpage={(p) => {
        incPage = p;
        void loadData();
      }}
      {loading}
      emptyTitle={$t('admin.network.incidents.ui.empty_title')}
      emptyHint={activeOnly
        ? $t('admin.network.incidents.ui.all_active_done')
        : $t('admin.network.incidents.ui.no_incidents_range')}
      footNote={$t('admin.network.incidents.ui.foot_note', { values: { n: total, w: slaWarnMinutes, b: slaBreachMinutes } })}
    >
      {#snippet cell(row: IncidentRow, col: Column)}
        {#if col.key === 'sel'}
          <input
            type="checkbox"
            class="h-6 w-6 accent-ink-900"
            checked={selectedIds.includes(row.id)}
            onchange={() => toggleSelected(row.id)}
            aria-label={$t('admin.network.incidents.ui.select_aria', { values: { title: row.title } })}
          />
        {:else if col.key === 'title'}
          <div class="min-w-0 max-w-[340px]">
            <div class="truncate font-medium text-ink-900">{row.title}</div>
            <div class="truncate text-sm text-ink-500">
              {routerLabel(row.router_id)}{#if row.interface_name} · {row.interface_name}{/if} · {typeLabel(row.incident_type)}
            </div>
          </div>
        {:else if col.key === 'severity'}
          <div class="flex items-center gap-1.5">
            <Badge tone={sevTone(row.severity)} label={severityLabel(row.severity)} />
            {#if row.is_auto_escalated}
              <Badge tone="neutral" label="auto" />
            {/if}
          </div>
        {:else if col.key === 'status'}
          <Badge tone={statTone(row.status)} label={statusLabel(row.status)} />
        {:else if col.key === 'duration'}
          <Badge tone={slaTone(row)} label={formatDurationCompact(incidentOpenMs(row, nowMs))} />
        {:else if col.key === 'owner'}
          <span class="text-sm text-ink-700">{ownerLabel(row.owner_user_id)}</span>
        {:else if col.key === 'last_seen'}
          <span class="text-sm text-ink-500">{formatDateTime(row.last_seen_at, { timeZone: $appSettings.app_timezone })}</span>
        {:else if col.key === 'actions'}
          <RowActions
            primary={{ label: $t('admin.network.incidents.ui.row_detail'), icon: 'search', onclick: () => openDetail(row) }}
            rest={[
              ...(canManage && row.status !== 'resolved' && row.status !== 'ack'
                ? [{ label: $t('admin.network.incidents.ui.row_ack'), onclick: () => void ackOne(row.id) }]
                : []),
              ...(canManage && row.status !== 'resolved'
                ? [{ label: $t('admin.network.incidents.ui.row_resolve'), onclick: () => void resolveOne(row.id) }]
                : []),
            ]}
          />
        {/if}
      {/snippet}
    </DataTable>
  </div>

  <Modal bind:show={showDetail} title={detail?.title ?? $t('admin.network.incidents.ui.row_detail')}>
    {#if detail}
      <div class="space-y-3 text-sm">
        <div class="flex flex-wrap items-center gap-2">
          <Badge tone={sevTone(detail.severity)} label={severityLabel(detail.severity)} />
          <Badge tone={statTone(detail.status)} label={statusLabel(detail.status)} />
          <Badge tone={slaTone(detail)} label="SLA {formatDurationCompact(incidentOpenMs(detail, nowMs))}" />
          {#if detail.is_auto_escalated}
            <Badge tone="neutral" label="Eskalasi otomatis" />
          {/if}
        </div>
        <p class="text-ink-700">{detail.message}</p>
        <dl class="grid grid-cols-2 gap-x-4 gap-y-2 text-ink-700">
          <dt class="text-ink-500">Router</dt>
          <dd>{routerLabel(detail.router_id)}{#if detail.interface_name} · {detail.interface_name}{/if}</dd>
          <dt class="text-ink-500">Tipe</dt>
          <dd>{typeLabel(detail.incident_type)}</dd>
          <dt class="text-ink-500">Pertama terlihat</dt>
          <dd>{formatDateTime(detail.first_seen_at || detail.updated_at, { timeZone: $appSettings.app_timezone })}</dd>
          <dt class="text-ink-500">Diakui</dt>
          <dd>{detail.acked_at ? formatDateTime(detail.acked_at, { timeZone: $appSettings.app_timezone }) : '—'}</dd>
          <dt class="text-ink-500">Selesai</dt>
          <dd>{detail.resolved_at ? formatDateTime(detail.resolved_at, { timeZone: $appSettings.app_timezone }) : '—'}</dd>
        </dl>
        {#if canManage}
          <Field
            id="inc-owner"
            label="Penanggung jawab"
            type="select"
            stacked
            value={detailOwner}
            options={[{ value: '', label: 'Belum ada' }, ...teamMembers.map((m) => ({ value: m.user_id, label: m.name }))]}
            onchange={(v) => (detailOwner = String(v ?? ''))}
          />
          <Field id="inc-notes" label="Catatan" type="textarea" stacked value={detailNotes} onchange={(v) => (detailNotes = String(v ?? ''))} />
          {#if detailError}
            <div class="rounded-lg border border-red-200 bg-red-50 p-2 text-red-700">{detailError}</div>
          {/if}
          <div class="flex justify-end gap-2">
            <Button variant="ghost" onclick={closeDetail}>Tutup</Button>
            <Button variant="primary" disabled={detailSaving} onclick={() => void saveDetail()}>
              {detailSaving ? 'Menyimpan…' : 'Simpan'}
            </Button>
          </div>
        {:else}
          <div class="flex justify-end">
            <Button variant="ghost" onclick={closeDetail}>Tutup</Button>
          </div>
        {/if}
      </div>
    {/if}
  </Modal>

  <Modal bind:show={showSimulate} title="Simulasi insiden">
    <div class="space-y-3 text-sm">
      <p class="text-ink-500">Membuat insiden manual untuk menguji alur acknowledge/eskalasi. Gunakan hanya saat latihan.</p>
      <Field
        id="sim-router"
        label="Router"
        type="select"
        stacked
        value={simRouter}
        options={[{ value: '', label: 'Pilih router…' }, ...routers.map((r) => ({ value: r.id, label: r.name }))]}
        onchange={(v) => (simRouter = String(v ?? ''))}
      />
      <Field id="sim-type" label="Tipe insiden" type="text" stacked value={simType} placeholder="mis. latency" onchange={(v) => (simType = String(v ?? ''))} />
      <Field
        id="sim-sev"
        label="Severity"
        type="select"
        stacked
        value={simSeverity}
        options={[{ value: 'info', label: 'Info' }, { value: 'warning', label: 'Peringatan' }, { value: 'critical', label: 'Kritis' }]}
        onchange={(v) => (simSeverity = String(v ?? 'warning') as 'info' | 'warning' | 'critical')}
      />
      <Field id="sim-iface" label="Antarmuka (opsional)" type="text" stacked value={simInterface} placeholder="mis. ether1" onchange={(v) => (simInterface = String(v ?? ''))} />
      <Field id="sim-msg" label="Pesan (opsional)" type="textarea" stacked value={simMessage} onchange={(v) => (simMessage = String(v ?? ''))} />
      {#if simError}
        <div class="rounded-lg border border-red-200 bg-red-50 p-2 text-red-700">{simError}</div>
      {/if}
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (showSimulate = false)}>Batal</Button>
        <Button variant="primary" disabled={simBusy} onclick={() => void submitSimulate()}>
          {simBusy ? 'Membuat…' : 'Buat simulasi'}
        </Button>
      </div>
    </div>
  </Modal>
</AppShell>
