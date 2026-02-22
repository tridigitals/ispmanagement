<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { appSettings } from '$lib/stores/settings';
  import { can } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';

  type RouterRow = { id: string; name: string; host?: string; port?: number };
  type LogRow = {
    id: string;
    tenant_id: string;
    router_id: string;
    router_log_id?: string | null;
    logged_at: string;
    router_time?: string | null;
    topics?: string | null;
    level?: string | null;
    message: string;
    created_at: string;
    updated_at: string;
  };

  let loading = $state(true);
  let syncing = $state(false);
  let isMobile = $state(false);

  let routers = $state<RouterRow[]>([]);
  let rows = $state<LogRow[]>([]);

  let q = $state('');
  let routerId = $state('');
  let level = $state('');
  let topic = $state('');
  const FULL_SYNC_FETCH_LIMIT = 25000;

  let pageNum = $state(1); // 1-based for API
  let perPage = $state(25);
  let loadingMore = $state(false);
  let ready = $state(false);
  let hasNext = $state(false);
  let total = $state<number>(-1); // optional; fetched only on filter changes
  let lastTotalKey = $state('');

  const columns = $derived.by(() => [
    { key: 'time', label: $t('admin.network.logs.columns.time') || 'Time', width: '180px' },
    { key: 'router', label: $t('admin.network.logs.columns.router') || 'Router', width: '180px' },
    { key: 'level', label: $t('admin.network.logs.columns.level') || 'Level', width: '110px' },
    { key: 'topics', label: $t('admin.network.logs.columns.topics') || 'Topics', width: '180px' },
    { key: 'message', label: $t('admin.network.logs.columns.message') || 'Message' },
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
    ready = true;
  });

  onDestroy(() => {
  });

  async function load() {
    loading = true;
    try {
      await Promise.all([loadRouters(), loadRowsPage(1)]);
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  async function loadRouters() {
    routers = (await api.mikrotik.routers.list()) as RouterRow[];
  }

  $effect(() => {
    if (!ready) return;
    const _q = q;
    const _t = topic;
    const timer = setTimeout(() => void loadRowsPage(1), 300);
    return () => clearTimeout(timer);
  });

  async function loadRowsPage(nextPage: number) {
    if (loadingMore) return;
    if (nextPage < 1) return;

    const params = {
      routerId: routerId || undefined,
      level: level || undefined,
      topic: topic.trim() || undefined,
      q: q.trim() || undefined,
    };

    const totalKey = JSON.stringify({
      routerId: params.routerId || '',
      level: params.level || '',
      topic: params.topic || '',
      q: params.q || '',
    });
    const shouldFetchTotal = nextPage === 1 && (totalKey !== lastTotalKey || total < 0);

    loadingMore = true;
    try {
      const res = await api.mikrotik.logs.list({
        ...params,
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
        lastTotalKey = totalKey;
      }
    } catch (e: any) {
      toast.error(e?.message || e);
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
      toast.success($t('admin.network.logs.toasts.sync_ok') || 'Log sync completed');
      await loadRowsPage(1);
    } catch (e: any) {
      toast.error(
        ($t('admin.network.logs.toasts.sync_failed') || 'Failed to sync logs') +
          `: ${String(e?.message || e)}`,
      );
    } finally {
      syncing = false;
    }
  }

  async function syncAll() {
    const ids = routers.map((r) => r.id);
    if (!ids.length) return;
    syncing = true;
    try {
      for (const id of ids) {
        await api.mikrotik.logs.sync(id, FULL_SYNC_FETCH_LIMIT);
      }
      toast.success($t('admin.network.logs.toasts.sync_ok') || 'Log sync completed');
      await loadRowsPage(1);
    } catch (e: any) {
      toast.error(
        ($t('admin.network.logs.toasts.sync_failed') || 'Failed to sync logs') +
          `: ${String(e?.message || e)}`,
      );
    } finally {
      syncing = false;
    }
  }

  function routerName(id: string) {
    return routers.find((r) => r.id === id)?.name || id;
  }

  function levelClass(v?: string | null) {
    const x = String(v || '').toLowerCase();
    if (x === 'critical' || x === 'error') return 'crit';
    if (x === 'warning') return 'warn';
    if (x === 'debug') return 'debug';
    return 'info';
  }
</script>

<div class="page-content fade-in logs-page">
  <div class="logs-shell">
    <NetworkPageHeader
      title={$t('admin.network.logs.title') || 'Router Logs'}
      subtitle={$t('admin.network.logs.subtitle') || 'Read and store MikroTik logs for troubleshooting and audits.'}
    >
      {#snippet actions()}
        <button class="btn ghost" type="button" onclick={() => void loadRowsPage(1)} title={$t('common.refresh') || 'Refresh'}>
          <Icon name="refresh-cw" size={16} />
          {$t('admin.network.logs.actions.refresh') || 'Refresh'}
        </button>
        {#if routerId}
          <button class="btn ghost" type="button" onclick={syncSelected} disabled={syncing}>
            <Icon name="download" size={16} />
            {$t('admin.network.logs.actions.sync_selected') || 'Sync selected router'}
          </button>
        {/if}
        <button class="btn" type="button" onclick={syncAll} disabled={syncing || routers.length === 0}>
          <Icon name="database" size={16} />
          {$t('admin.network.logs.actions.sync_all') || 'Sync all routers'}
        </button>
      {/snippet}
    </NetworkPageHeader>

    <div class="filters">
      <label>
        <span>{$t('admin.network.logs.filters.router') || 'Router'}</span>
        <select bind:value={routerId} onchange={() => void loadRowsPage(1)}>
          <option value="">{$t('admin.network.logs.filters.all_routers') || 'All routers'}</option>
          {#each routers as r}
            <option value={r.id}>{r.name}</option>
          {/each}
        </select>
      </label>

      <label>
        <span>{$t('admin.network.logs.filters.level') || 'Level'}</span>
        <select bind:value={level} onchange={() => void loadRowsPage(1)}>
          <option value="">{$t('admin.network.logs.filters.all_levels') || 'All levels'}</option>
          <option value="critical">critical</option>
          <option value="error">error</option>
          <option value="warning">warning</option>
          <option value="info">info</option>
          <option value="debug">debug</option>
        </select>
      </label>

      <label>
        <span>{$t('admin.network.logs.filters.topic') || 'Topic'}</span>
        <input bind:value={topic} placeholder="system,error,interface..." />
      </label>

      <label class="search">
        <span>{$t('common.search') || 'Search'}</span>
        <input
          bind:value={q}
          placeholder={$t('admin.network.logs.search') || 'Search log message...'}
        />
      </label>
    </div>

    <div class="table-wrap">
      <Table
        {columns}
        data={rows}
        keyField="id"
        loading={loading || loadingMore}
        pagination={false}
        searchable={false}
        mobileView={isMobile ? 'card' : 'scroll'}
        emptyText={$t('admin.network.logs.empty') || 'No logs'}
      >
        {#snippet cell({ item, key }: any)}
          {#if key === 'time'}
            <div class="stack">
              <span title={formatDateTime(item.logged_at, { timeZone: $appSettings.app_timezone })}
                >{timeAgo(item.logged_at)}</span
              >
              {#if item.router_time}
                <span class="muted mono">{item.router_time}</span>
              {/if}
            </div>
          {:else if key === 'router'}
            <span class="mono">{routerName(item.router_id)}</span>
          {:else if key === 'level'}
            <span class="pill {levelClass(item.level)}">{item.level || 'info'}</span>
          {:else if key === 'topics'}
            <span class="mono muted">{item.topics || '-'}</span>
          {:else if key === 'message'}
            <span>{item.message}</span>
          {:else}
            {item[key] ?? ''}
          {/if}
        {/snippet}
      </Table>

      <div class="pager">
        <div class="pager-left">
          <span class="muted">
            {rows.length}
            {#if total >= 0}
              / {total}
            {/if}
            {$t('common.results') || 'results'}
          </span>
        </div>
        <div class="pager-right">
          <span class="muted">{$t('common.page') || 'Page'} {pageNum}</span>
          <label class="per-page">
            <span class="muted">{$t('components.pagination.rows_per_page') || 'Rows per page:'}</span>
            <select bind:value={perPage} onchange={() => void loadRowsPage(1)}>
              <option value={25}>25</option>
              <option value={50}>50</option>
              <option value={100}>100</option>
              <option value={200}>200</option>
            </select>
          </label>
          <button class="btn btn-secondary" type="button" onclick={() => void loadRowsPage(pageNum - 1)} disabled={loadingMore || loading || pageNum <= 1}>
            <Icon name="chevron-left" size={16} />
            {$t('common.previous') || 'Previous'}
          </button>
          <button class="btn btn-secondary" type="button" onclick={() => void loadRowsPage(pageNum + 1)} disabled={loadingMore || loading || !hasNext}>
            {$t('common.next') || 'Next'}
            <Icon name="chevron-right" size={16} />
          </button>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .logs-page {
    padding: 1rem;
  }
  .logs-shell {
    border: 1px solid var(--border-color);
    border-radius: 18px;
    background: var(--bg-surface);
    box-shadow: var(--shadow-md);
    padding: 1rem 1rem 0.8rem;
  }
  .filters {
    display: grid;
    grid-template-columns: repeat(4, minmax(180px, 1fr));
    gap: 0.75rem;
    margin-bottom: 1rem;
  }
  .filters label { display: grid; gap: 0.35rem; }
  .filters span { color: var(--text-secondary); font-size: 0.82rem; font-weight: 700; }
  .filters input, .filters select {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    padding: 0.6rem 0.75rem;
  }
  .table-wrap {
    margin-top: 0.4rem;
  }

  .pager {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.9rem 0.25rem 0.25rem;
  }

  .pager-right {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .per-page {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
  }

  .per-page select {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    padding: 0.45rem 0.6rem;
    min-height: 38px;
  }
  .search { grid-column: span 1; }
  .stack { display: grid; gap: 0.2rem; }
  .mono { font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace); }
  .muted { color: var(--text-secondary); }
  .pill {
    display: inline-flex;
    align-items: center;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    text-transform: uppercase;
    font-size: 0.72rem;
    font-weight: 800;
  }
  .pill.info { color: #60a5fa; border-color: rgba(96, 165, 250, 0.35); background: rgba(96, 165, 250, 0.08); }
  .pill.warn { color: #f59e0b; border-color: rgba(245, 158, 11, 0.35); background: rgba(245, 158, 11, 0.08); }
  .pill.crit { color: #ef4444; border-color: rgba(239, 68, 68, 0.35); background: rgba(239, 68, 68, 0.08); }
  .pill.debug { color: #a78bfa; border-color: rgba(167, 139, 250, 0.35); background: rgba(167, 139, 250, 0.08); }

  @media (max-width: 1100px) {
    .filters { grid-template-columns: repeat(2, minmax(180px, 1fr)); }
  }
  @media (max-width: 780px) {
    .logs-page { padding: 0.75rem; }
    .logs-shell { padding: 0.85rem 0.75rem 0.7rem; }
    .head { flex-direction: column; }
    .filters { grid-template-columns: 1fr; }
  }
</style>
