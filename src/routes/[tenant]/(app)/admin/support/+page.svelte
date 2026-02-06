<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { PaginatedResponse, SupportTicketListItem, SupportTicketStats } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import TableToolbar from '$lib/components/ui/TableToolbar.svelte';
  import { toast } from '$lib/stores/toast';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';

  let loading = $state(true);
  let loadingMore = $state(false);
  let tickets = $state<SupportTicketListItem[]>([]);
  let searchQuery = $state('');
  let statusFilter = $state<'all' | 'open' | 'pending' | 'closed'>('all');
  let stats = $state<SupportTicketStats>({ all: 0, open: 0, pending: 0, closed: 0 });
  let total = $state(0);
  let pageNum = $state(1);
  const perPage = 20;
  let ready = $state(false);

  let hasMore = $derived(tickets.length < total);

  const columns = $derived.by(() => [
    { key: 'subject', label: $t('admin.support.columns.subject') || 'Subject' },
    { key: 'user', label: $t('admin.support.columns.user') || 'User' },
    { key: 'status', label: $t('admin.support.columns.status') || 'Status' },
    { key: 'priority', label: $t('admin.support.columns.priority') || 'Priority' },
    { key: 'updated', label: $t('admin.support.columns.updated') || 'Updated' },
    { key: 'messages', label: $t('admin.support.columns.messages') || 'Messages', align: 'right' },
    { key: 'actions', label: '', align: 'right' as const, width: '84px' },
  ]);

  onMount(async () => {
    if (!$can('read_all', 'support')) {
      goto('/unauthorized');
      return;
    }
    await refreshStats();
    await load(true);
    ready = true;
  });

  $effect(() => {
    if (!ready) return;
    const q = searchQuery;
    const timer = setTimeout(() => {
      void load(true);
    }, 250);
    return () => clearTimeout(timer);
  });

  async function refreshStats() {
    try {
      stats = await api.support.stats();
    } catch {
      // non-blocking
    }
  }

  async function load(reset: boolean) {
    loading = true;
    if (reset) {
      pageNum = 1;
      tickets = [];
      total = 0;
    }
    try {
      const res: PaginatedResponse<SupportTicketListItem> = await api.support.list({
        status: statusFilter === 'all' ? undefined : statusFilter,
        search: searchQuery.trim() || undefined,
        page: pageNum,
        perPage,
      });
      total = res.total || 0;
      tickets = reset ? res.data : [...tickets, ...res.data];
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  async function loadMore() {
    if (loadingMore || loading || !hasMore) return;
    loadingMore = true;
    try {
      pageNum += 1;
      const res: PaginatedResponse<SupportTicketListItem> = await api.support.list({
        status: statusFilter === 'all' ? undefined : statusFilter,
        search: searchQuery.trim() || undefined,
        page: pageNum,
        perPage,
      });
      total = res.total || total;
      tickets = [...tickets, ...res.data];
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loadingMore = false;
    }
  }

  function open(id: string) {
    goto(`${$page.url.pathname}/${id}`);
  }

  function setStatusFilter(v: typeof statusFilter) {
    if (statusFilter === v) return;
    statusFilter = v;
    void load(true);
  }
</script>

<div class="page-content fade-in">
  <div class="head">
    <div>
      <h1 class="title">{$t('admin.support.title') || 'Support Tickets'}</h1>
      <p class="sub">{$t('admin.support.subtitle') || 'Manage tenant support requests'}</p>
    </div>

    <button class="btn" type="button" onclick={() => load(true)} title={$t('common.refresh') || 'Refresh'}>
      <Icon name="refresh-cw" size={16} />
      {$t('common.refresh') || 'Refresh'}
    </button>
  </div>

  <div class="stats">
    <button
      class="stat-card"
      class:active={statusFilter === 'all'}
      type="button"
      onclick={() => setStatusFilter('all')}
    >
      <div class="stat-top">
        <span class="stat-label">{$t('support.stats.total') || 'Total'}</span>
        <Icon name="list" size={14} />
      </div>
      <div class="stat-value">{stats.all}</div>
    </button>
    <button
      class="stat-card tone-open"
      class:active={statusFilter === 'open'}
      type="button"
      onclick={() => setStatusFilter('open')}
    >
      <div class="stat-top">
        <span class="stat-label">{$t('support.filters.open') || 'Open'}</span>
        <Icon name="info" size={14} />
      </div>
      <div class="stat-value">{stats.open}</div>
    </button>
    <button
      class="stat-card tone-pending"
      class:active={statusFilter === 'pending'}
      type="button"
      onclick={() => setStatusFilter('pending')}
    >
      <div class="stat-top">
        <span class="stat-label">{$t('support.filters.pending') || 'Pending'}</span>
        <Icon name="alert-triangle" size={14} />
      </div>
      <div class="stat-value">{stats.pending}</div>
    </button>
    <button
      class="stat-card tone-closed"
      class:active={statusFilter === 'closed'}
      type="button"
      onclick={() => setStatusFilter('closed')}
    >
      <div class="stat-top">
        <span class="stat-label">{$t('support.filters.closed') || 'Closed'}</span>
        <Icon name="check-circle" size={14} />
      </div>
      <div class="stat-value">{stats.closed}</div>
    </button>
  </div>

  <div class="filters">
    <div class="filter">
      <button class:active={statusFilter === 'all'} onclick={() => setStatusFilter('all')}>
        {$t('support.filters.all') || 'All'}
      </button>
      <button class:active={statusFilter === 'open'} onclick={() => setStatusFilter('open')}>
        {$t('support.filters.open') || 'Open'}
      </button>
      <button class:active={statusFilter === 'pending'} onclick={() => setStatusFilter('pending')}>
        {$t('support.filters.pending') || 'Pending'}
      </button>
      <button class:active={statusFilter === 'closed'} onclick={() => setStatusFilter('closed')}>
        {$t('support.filters.closed') || 'Closed'}
      </button>
    </div>

    <div class="search">
      <Icon name="search" size={16} />
      <input
        class="search-input"
        bind:value={searchQuery}
        placeholder={$t('admin.support.search') || 'Search tickets...'}
      />
      {#if searchQuery}
        <button class="clear" type="button" onclick={() => (searchQuery = '')}>
          <Icon name="x" size={14} />
        </button>
      {/if}
    </div>
  </div>

  <Table
    {columns}
    data={tickets}
    {loading}
    emptyText={$t('admin.support.empty') || 'No tickets'}
    pagination={false}
  >
    {#snippet cell({ item, key }: any)}
      {#if key === 'subject'}
        <button class="link" type="button" onclick={() => open(item.id)}>
          {item.subject}
        </button>
      {:else if key === 'status'}
        <span class="badge status {item.status}"
          >{$t(`support.status.${item.status}`) || item.status}</span
        >
      {:else if key === 'user'}
        <span class="user">{item.created_by_name || $t('common.na') || '—'}</span>
      {:else if key === 'priority'}
        <span class="badge priority {item.priority}">
          {$t(`support.priorities.${item.priority}`) || item.priority}
        </span>
      {:else if key === 'updated'}
        <span class="mono">
          {formatDateTime(item.last_message_at || item.updated_at, {
            timeZone: $appSettings.app_timezone,
          })}
        </span>
      {:else if key === 'messages'}
        <span class="count">
          <Icon name="message-circle" size={14} />
          {item.message_count}
        </span>
      {:else if key === 'actions'}
        <button
          class="icon-btn"
          type="button"
          onclick={() => open(item.id)}
          title={$t('common.open') || 'Open'}
        >
          <Icon name="arrow-right" size={16} />
        </button>
      {:else}
        {item[key] ?? ''}
      {/if}
    {/snippet}
  </Table>

  {#if hasMore}
    <div class="footer">
      <button class="btn-more" type="button" onclick={loadMore} disabled={loadingMore}>
        {#if loadingMore}
          <div class="spinner-sm"></div>
        {/if}
        {$t('common.load_more') || 'Load more'}
      </button>
      <div class="foot-note">{tickets.length}/{total}</div>
    </div>
  {/if}
</div>

<style>
  .page-content {
    padding: 1.5rem;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }

  .title {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .sub {
    margin: 0.35rem 0 0 0;
    color: var(--text-secondary);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 0.6rem 0.9rem;
    border-radius: 10px;
    cursor: pointer;
    font-weight: 800;
  }

  .filters {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
    margin: 0.9rem 0 1rem 0;
  }

  .stat-card {
    text-align: left;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    background:
      radial-gradient(700px 150px at 30% 0%, rgba(255, 255, 255, 0.05), transparent 55%),
      var(--bg-surface);
    padding: 0.9rem;
    cursor: pointer;
    transition:
      transform 0.12s ease,
      border-color 0.12s ease;
  }

  .stat-card:hover {
    transform: translateY(-1px);
    border-color: rgba(99, 102, 241, 0.35);
  }

  .stat-card.active {
    border-color: rgba(99, 102, 241, 0.5);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.12);
  }

  .stat-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: var(--text-secondary);
    font-weight: 800;
    font-size: 0.85rem;
    gap: 0.75rem;
  }

  .stat-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stat-value {
    margin-top: 0.35rem;
    font-size: 1.55rem;
    font-weight: 950;
    letter-spacing: -0.03em;
    color: var(--text-primary);
  }

  .tone-open {
    border-color: rgba(59, 130, 246, 0.22);
  }

  .tone-pending {
    border-color: rgba(245, 158, 11, 0.22);
  }

  .tone-closed {
    border-color: rgba(34, 197, 94, 0.22);
  }

  .filter {
    display: inline-flex;
    gap: 0.35rem;
    padding: 0.3rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
  }

  .filter button {
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-secondary);
    padding: 0.45rem 0.7rem;
    border-radius: 10px;
    cursor: pointer;
    font-weight: 800;
    font-size: 0.85rem;
  }

  .filter button.active {
    background: rgba(99, 102, 241, 0.15);
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--text-primary);
  }

  .search {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    border-radius: 12px;
    padding: 0.55rem 0.75rem;
    color: var(--text-secondary);
    min-width: min(420px, 100%);
  }

  .search-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text-primary);
    outline: none;
  }

  .clear {
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-secondary);
    width: 28px;
    height: 28px;
    border-radius: 10px;
    cursor: pointer;
  }

  .link {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    background: transparent;
    border: none;
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
    font-weight: 900;
  }

  .mono {
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
    font-size: 0.85rem;
    color: var(--text-secondary);
    font-weight: 700;
  }

  .user {
    font-weight: 900;
    color: var(--text-primary);
  }

  .badge {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 0.2rem 0.55rem;
    font-size: 0.75rem;
    font-weight: 900;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    background: rgba(255, 255, 255, 0.03);
    display: inline-block;
  }

  .badge.status.open {
    border-color: rgba(59, 130, 246, 0.35);
    color: rgba(59, 130, 246, 0.95);
    background: rgba(59, 130, 246, 0.08);
  }
  .badge.status.pending {
    border-color: rgba(245, 158, 11, 0.35);
    color: rgba(245, 158, 11, 0.95);
    background: rgba(245, 158, 11, 0.08);
  }
  .badge.status.closed {
    border-color: rgba(34, 197, 94, 0.35);
    color: rgba(34, 197, 94, 0.95);
    background: rgba(34, 197, 94, 0.08);
  }

  .badge.priority.urgent {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgba(239, 68, 68, 0.95);
    background: rgba(239, 68, 68, 0.08);
  }
  .badge.priority.high {
    border-color: rgba(245, 158, 11, 0.35);
    color: rgba(245, 158, 11, 0.95);
    background: rgba(245, 158, 11, 0.08);
  }
  .badge.priority.normal {
    border-color: rgba(156, 163, 175, 0.35);
    color: var(--text-secondary);
    background: rgba(156, 163, 175, 0.06);
  }
  .badge.priority.low {
    border-color: rgba(34, 197, 94, 0.25);
    color: rgba(34, 197, 94, 0.9);
    background: rgba(34, 197, 94, 0.06);
  }

  .count {
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.35rem;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    padding: 0.25rem 0.5rem;
    border-radius: 999px;
    font-weight: 900;
  }

  .icon-btn {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-secondary);
    cursor: pointer;
  }

  .icon-btn:hover {
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--color-primary);
  }

  .spinner-sm {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .footer {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.9rem;
    padding: 1.1rem 0.25rem 0.25rem;
    color: var(--text-secondary);
  }

  .btn-more {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-primary);
    padding: 0.65rem 0.85rem;
    font-weight: 900;
    cursor: pointer;
  }

  .btn-more:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .foot-note {
    font-weight: 800;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 900px) {
    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
