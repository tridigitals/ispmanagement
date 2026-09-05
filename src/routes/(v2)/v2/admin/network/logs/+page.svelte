<script lang="ts">
  /*
    Log jaringan v2 — gelombang 24b.

    Versi lama: (app)/admin/network/logs/+page.svelte (691 baris).
    Perilaku dipertahankan identik: filter router/level/topik/bulan/
    tahun/search (debounce 300ms), panel retensi per router, pager,
    sinkronisasi per router & semua router, hapus log (konfirmasi).
    Tone badge level kini dari helper murni networkLogInsights (6 tes).
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import { logFilterKey, logLevelLabel, logLevelTone } from '$lib/utils/networkLogInsights';
  import Modal from '$lib/components/ui/Modal.svelte';
  import {
    AppShell,
    Badge,
    Button,
    Card,
    DataTable,
    Field,
    Icon,
    PageHeader,
  } from '$lib/components/ds';
  import type { Column } from '$lib/components/ds/table-types';

  interface RouterRow { id: string; name: string; host?: string; port?: number }
  interface LogRow {
    id: string;
    tenant_id: string;
    router_id: string;
    logged_at: string;
    router_time?: string | null;
    topics?: string | null;
    level?: string | null;
    message: string;
  }

  let loading = $state(true);
  let syncing = $state(false);
  let routers = $state<RouterRow[]>([]);
  let rows = $state<LogRow[]>([]);

  let q = $state('');
  let routerId = $state('');
  let level = $state('');
  let topic = $state('');
  let month = $state(String(new Date().getMonth() + 1));
  let year = $state(String(new Date().getFullYear()));
  const FULL_SYNC_FETCH_LIMIT = 25000;

  let pageNum = $state(1);
  let perPage = $state(25);
  let loadingMore = $state(false);
  let ready = $state(false);
  let hasNext = $state(false);
  let total = $state(-1);
  let lastTotalKey = $state('');
  let retentionValue = $state('unlimited');
  let retentionLoading = $state(false);
  let retentionSaving = $state(false);
  let clearingLogs = $state(false);
  let showClearConfirm = $state(false);
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  const MONTHS = ['Januari','Februari','Maret','April','Mei','Juni','Juli','Agustus','September','Oktober','November','Desember'];
  const monthOptions = $derived([
    { value: '', label: 'Semua bulan' },
    ...MONTHS.map((label, i) => ({ value: String(i + 1), label })),
  ]);
  const yearOptions = $derived.by(() => {
    const y = new Date().getFullYear();
    return [{ value: '', label: 'Semua' }, ...Array.from({ length: 8 }, (_, i) => ({ value: String(y - i), label: String(y - i) }))];
  });
  const levelOptions = [
    { value: '', label: 'Semua level' },
    { value: 'critical', label: 'critical' },
    { value: 'error', label: 'error' },
    { value: 'warning', label: 'warning' },
    { value: 'info', label: 'info' },
    { value: 'debug', label: 'debug' },
  ];
  const retentionOptions = [
    { value: 'unlimited', label: 'Tanpa batas' },
    { value: '30', label: '30 hari' },
    { value: '90', label: '90 hari' },
    { value: '360', label: '360 hari' },
  ];
  const routerOptions = $derived([
    { value: '', label: 'Semua router' },
    ...routers.map((r) => ({ value: r.id, label: r.name })),
  ]);

  const columns: Column[] = [
    { key: 'logged_at', label: 'Waktu' },
    { key: 'router_id', label: 'Router' },
    { key: 'level', label: 'Level' },
    { key: 'topics', label: 'Topik' },
    { key: 'message', label: 'Pesan' },
  ];

  onMount(() => {
    if (!$can('read', 'network_logs') && !$can('manage', 'network_logs')) {
      goto('/unauthorized');
      return;
    }
    void load();
    ready = true;
  });

  async function load() {
    loading = true;
    try {
      await Promise.all([loadRouters(), loadRowsPage(1)]);
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      loading = false;
    }
  }

  async function loadRouters() {
    routers = (await api.mikrotik.routers.list()) as RouterRow[];
  }

  function onSearchInput(v: string) {
    q = v;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => void loadRowsPage(1), 300);
  }

  function onTopicInput(v: string) {
    topic = v;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => void loadRowsPage(1), 300);
  }

  async function loadRowsPage(nextPage: number) {
    if (loadingMore || nextPage < 1) return;
    const key = logFilterKey({ routerId, level, topic: topic.trim(), q: q.trim(), month, year });
    const shouldFetchTotal = nextPage === 1 && (key !== lastTotalKey || total < 0);
    loadingMore = true;
    try {
      const res = await api.mikrotik.logs.list({
        routerId: routerId || undefined,
        level: level || undefined,
        topic: topic.trim() || undefined,
        q: q.trim() || undefined,
        month: month ? Number(month) : undefined,
        year: year ? Number(year) : undefined,
        page: nextPage,
        perPage,
        includeTotal: shouldFetchTotal,
      });
      const chunk = res.data || [];
      rows = chunk;
      pageNum = nextPage;
      hasNext = chunk.length >= perPage;
      if (shouldFetchTotal) {
        total = Number(res.total ?? -1);
        lastTotalKey = key;
      }
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
      hasNext = false;
    } finally {
      loadingMore = false;
    }
  }

  async function syncSelected() {
    if (!routerId) return;
    syncing = true;
    try {
      await api.mikrotik.logs.sync(routerId, FULL_SYNC_FETCH_LIMIT);
      toast.success('Sinkronisasi log selesai.');
      await loadRowsPage(1);
    } catch (e) {
      toast.error(`Gagal sinkronisasi log: ${extractApiErrorMessage(e)}`);
    } finally {
      syncing = false;
    }
  }

  async function syncAll() {
    const ids = routers.map((r) => r.id);
    if (!ids.length) return;
    syncing = true;
    try {
      const result = await Promise.allSettled(ids.map((id) => api.mikrotik.logs.sync(id, FULL_SYNC_FETCH_LIMIT)));
      const ok = result.filter((i) => i.status === 'fulfilled').length;
      const failed = result.length - ok;
      if (ok > 0) toast.success('Sinkronisasi log selesai.');
      if (failed > 0) toast.error(`Gagal sinkronisasi ${failed} router.`);
      await loadRowsPage(1);
    } catch (e) {
      toast.error(`Gagal sinkronisasi log: ${extractApiErrorMessage(e)}`);
    } finally {
      syncing = false;
    }
  }

  function routerName(id: string) {
    return routers.find((r) => r.id === id)?.name || id;
  }

  async function loadRetention(selectedRouterId: string) {
    if (!selectedRouterId) {
      retentionValue = 'unlimited';
      return;
    }
    retentionLoading = true;
    try {
      const res = await api.mikrotik.logs.getRetention(selectedRouterId);
      retentionValue = res.retention_days ? String(res.retention_days) : 'unlimited';
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
      retentionValue = 'unlimited';
    } finally {
      retentionLoading = false;
    }
  }

  async function saveRetention() {
    if (!routerId) return;
    retentionSaving = true;
    try {
      const res = await api.mikrotik.logs.updateRetention(routerId, retentionValue === 'unlimited' ? null : Number(retentionValue));
      retentionValue = res.retention_days ? String(res.retention_days) : 'unlimited';
      toast.success('Retensi log diperbarui.');
      await loadRowsPage(1);
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
      await loadRetention(routerId);
    } finally {
      retentionSaving = false;
    }
  }

  async function clearLogs() {
    if (!routerId) return;
    clearingLogs = true;
    try {
      const res = await api.mikrotik.logs.clear(routerId);
      toast.success(`Menghapus ${res.deleted} log dari ${routerName(routerId)}.`);
      showClearConfirm = false;
      await loadRowsPage(1);
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      clearingLogs = false;
    }
  }

  $effect(() => {
    if (!ready) return;
    void loadRetention(routerId);
  });
</script>
<AppShell title="Log jaringan">
  <PageHeader
    title="Log jaringan"
    eyebrow="Jaringan"
    desc="Log router MikroTik yang tersinkron — filter, retensi, dan sinkronisasi manual."
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={() => void loadRowsPage(1)} disabled={loading || loadingMore}>
        Segarkan
      </Button>
      {#if routerId}
        <Button variant="secondary" onclick={() => void syncSelected()} disabled={syncing}>
          {syncing ? 'Sinkron…' : 'Sinkron router ini'}
        </Button>
      {/if}
      <Button variant="secondary" onclick={() => void syncAll()} disabled={syncing || routers.length === 0}>
        {syncing ? 'Sinkron…' : 'Sinkron semua'}
      </Button>
      <Button variant="danger" onclick={() => (showClearConfirm = true)} disabled={!routerId || clearingLogs}>
        Hapus log
      </Button>
    {/snippet}
  </PageHeader>

  <Card title="Filter">
    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-6">
      <Field id="lg-router" label="Router" type="select" stacked value={routerId} options={routerOptions} onchange={(v) => { routerId = v; void loadRowsPage(1); }} />
      <Field id="lg-level" label="Level" type="select" stacked value={level} options={levelOptions} onchange={(v) => { level = v; void loadRowsPage(1); }} />
      <Field id="lg-topic" label="Topik" type="text" stacked value={topic} onchange={onTopicInput} placeholder="system,error,interface…" />
      <Field id="lg-month" label="Bulan" type="select" stacked value={month} options={monthOptions} onchange={(v) => { month = v; void loadRowsPage(1); }} />
      <Field id="lg-year" label="Tahun" type="select" stacked value={year} options={yearOptions} onchange={(v) => { year = v; void loadRowsPage(1); }} />
      <Field id="lg-q" label="Cari" type="text" stacked value={q} onchange={onSearchInput} placeholder="Cari pesan…" />
    </div>
  </Card>

  <Card title="Retensi log">
    <div class="flex flex-wrap items-center gap-3">
      <div class="min-w-48 flex-1">
        <Field id="lg-retention" label={routerId ? `Berlaku untuk ${routerName(routerId)}` : 'Pilih router dulu'} type="select" stacked value={retentionValue} options={retentionOptions} onchange={(v) => { retentionValue = v; void saveRetention(); }} disabled={!routerId || retentionLoading || retentionSaving} />
      </div>
      {#if retentionLoading || retentionSaving}
        <span class="text-sm text-ink-500">{retentionLoading ? 'Memuat…' : 'Menyimpan…'}</span>
      {/if}
    </div>
  </Card>

  <Card title="Hasil" padded={false}>
    {#if !loading && rows.length === 0}
      <div class="px-4 py-10 text-center">
        <div class="text-sm font-medium text-ink-900">Tidak ada log</div>
        <p class="mt-1 text-sm text-ink-500">Ubah filter atau sinkronkan dari router.</p>
      </div>
    {:else}
      <DataTable
        {columns}
        rows={rows}
        {loading}
        emptyTitle="Tidak ada log"
        emptyHint="Ubah filter atau sinkronkan dari router."
      >
        {#snippet cell(item, column)}
          {#if column.key === 'logged_at'}
            <span title={formatDateTime(item.logged_at, { timeZone: $appSettings.app_timezone })}>{timeAgo(item.logged_at)}</span>
            {#if item.router_time}
              <span class="block font-mono text-xs text-ink-400">{item.router_time}</span>
            {/if}
          {:else if column.key === 'router_id'}
            <span class="font-mono text-xs">{routerName(item.router_id)}</span>
          {:else if column.key === 'level'}
            <Badge tone={logLevelTone(item.level)} label={logLevelLabel(item.level)} />
          {:else if column.key === 'topics'}
            <span class="font-mono text-xs text-ink-500">{item.topics || '-'}</span>
          {:else if column.key === 'message'}
            <span class="text-sm">{item.message}</span>
          {/if}
        {/snippet}
      </DataTable>
      <div class="flex flex-wrap items-center justify-between gap-3 border-t border-ink-100 px-4 py-3 text-sm text-ink-500">
        <span>{rows.length}{#if total >= 0} / {total}{/if} hasil · halaman {pageNum}</span>
        <div class="flex items-center gap-2">
          <Button variant="ghost" onclick={() => void loadRowsPage(pageNum - 1)} disabled={loadingMore || loading || pageNum <= 1}>
            Sebelumnya
          </Button>
          <Button variant="ghost" onclick={() => void loadRowsPage(pageNum + 1)} disabled={loadingMore || loading || !hasNext}>
            Berikutnya
          </Button>
        </div>
      </div>
    {/if}
  </Card>
</AppShell>

<Modal bind:show={showClearConfirm} title="Hapus log router?">
  <p class="text-sm text-ink-700">
    Semua log tersimpan untuk <strong>{routerName(routerId)}</strong> akan dihapus permanen. Lanjutkan?
  </p>
  <div class="mt-4 flex justify-end gap-2">
    <Button variant="ghost" onclick={() => (showClearConfirm = false)}>Batal</Button>
    <Button variant="danger" onclick={() => void clearLogs()} disabled={clearingLogs}>
      {clearingLogs ? 'Menghapus…' : 'Hapus'}
    </Button>
  </div>
</Modal>
