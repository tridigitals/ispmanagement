<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type {
    PaginatedResponse,
    SupportTicketListItem,
    SupportTicketStats,
  } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import TableToolbar from '$lib/components/ui/TableToolbar.svelte';
  import { toast } from '$lib/stores/toast';
  import { t } from 'svelte-i18n';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';

  let loading = $state(true);
  let loadingMore = $state(false);
  let tickets = $state<SupportTicketListItem[]>([]);
  let searchQuery = $state('');
  let statusFilter = $state<'all' | 'open' | 'pending' | 'closed'>('all');
  let assignedFilter = $state<'all' | 'assigned' | 'unassigned'>('all');
  let categoryFilter = $state<'all' | 'general' | 'billing' | 'technical' | 'installation'>('all');
  let stats = $state<SupportTicketStats>({ all: 0, open: 0, pending: 0, closed: 0 });
  let total = $state(0);
  let pageNum = $state(1);
  const perPage = 20;
  let ready = $state(false);

  let hasMore = $derived(tickets.length < total);

  const categoryOptions = $derived([
    { label: $t('support.categories.all') || 'All', value: 'all' },
    { label: $t('support.categories.general') || 'General', value: 'general' },
    { label: $t('support.categories.billing') || 'Billing', value: 'billing' },
    { label: $t('support.categories.technical') || 'Technical', value: 'technical' },
    { label: $t('support.categories.installation') || 'Installation', value: 'installation' },
  ]);

  const columns = $derived.by(() => [
    { key: 'subject', label: $t('admin.support.columns.subject') || 'Subject' },
    { key: 'category', label: $t('admin.support.columns.category') || 'Category' },
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
        category: categoryFilter === 'all' ? undefined : categoryFilter,
        search: searchQuery.trim() || undefined,
        page: pageNum,
        perPage,
        assigned: assignedFilter === 'all' ? undefined : assignedFilter,
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
        category: categoryFilter === 'all' ? undefined : categoryFilter,
        search: searchQuery.trim() || undefined,
        page: pageNum,
        perPage,
        assigned: assignedFilter === 'all' ? undefined : assignedFilter,
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

  async function openSubscription(subscriptionId: string | null) {
    if (!subscriptionId) return;
    try {
      const sub = await api.customers.subscriptions.get(subscriptionId);
      if (sub?.customer_id) {
        goto(`/admin/customers/${sub.customer_id}`);
      }
    } catch (e: any) {
      toast.error(e?.message || 'Failed to load subscription');
    }
  }

  function setStatusFilter(v: typeof statusFilter) {
    if (statusFilter === v) return;
    statusFilter = v;
    void load(true);
  }

  function setCategoryFilter(v: typeof categoryFilter) {
    if (categoryFilter === v) return;
    categoryFilter = v;
    void load(true);
  }

  function setAssignedFilter(v: typeof assignedFilter) {
    if (assignedFilter === v) return;
    assignedFilter = v;
    void load(true);
  }
</script>

<div class="page-content fade-in">
  <div class="head">
    <div>
      <h1 class="title">{$t('admin.support.title') || 'Support Tickets'}</h1>
      <p class="sub">{$t('admin.support.subtitle') || 'Tiket support pelanggan.'}</p>
    </div>

    <button
      class="btn"
      type="button"
      onclick={() => load(true)}
      title={$t('common.refresh') || 'Refresh'}
    >
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
    <button
      class="stat-card tone-assigned"
      class:active={assignedFilter === 'unassigned'}
      type="button"
      onclick={() => setAssignedFilter(assignedFilter === 'unassigned' ? 'all' : 'unassigned')
      title="Belum Assign"
    >
      <div class="stat-top">
        <span class="stat-label">Belum Assign</span>
        <Icon name="user" size={14} />
      </div>
      <div class="stat-value">—</div>
    </button>
  </div>

  <div class="filters">
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
    <div class="category-filter">
      {#each categoryOptions as opt}
        <button
          class="cat-btn"
          class:active={categoryFilter === opt.value}
          type="button"
          onclick={() => setCategoryFilter(opt.value as any)}
        >
          {opt.label}
        </button>
      {/each}
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
          {#if item.subscription_id}
            <span
              class="sub-link"
              role="button"
              tabindex="0"
              title={$t('support.detail.view_subscription') || 'Langganan terkait'}
              onclick={(e) => { e.stopPropagation(); openSubscription(item.subscription_id); }}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); openSubscription(item.subscription_id); } }}
            >
              <Icon name="link" size={12} />
            </span>
          {/if}
        </button>
      {:else if key === 'status'}
        <span class="badge status {item.status}"
          >{$t(`support.status.${item.status}`) || item.status}</span
        >
      {:else if key === 'category'}
        {#if item.category}
          <span class="badge category {item.category}">
            {$t(`support.categories.${item.category}`) || item.category}
          </span>
        {:else}
          <span class="mono">—</span>
        {/if}
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
    padding: 1.1rem 1.25rem 1.25rem;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.85rem;
    flex-wrap: wrap;
  }

  .title {
    margin: 0;
    font-size: 1.28rem;
    font-weight: 800;
    color: var(--text-primary);
  }

  .sub {
    margin: 0.25rem 0 0 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 0.55rem 0.85rem;
    border-radius: 10px;
    cursor: pointer;
    font-weight: 800;
  }

  .filters {
    display: flex;
    align-items: center;
    justify-content: stretch;
    gap: 0.65rem;
    margin-bottom: 0.75rem;
    flex-wrap: wrap;
    padding: 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: var(--bg-surface);
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.6rem;
    margin: 0.7rem 0 0.8rem 0;
  }

  .stat-card {
    text-align: left;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    background: var(--bg-surface);
    padding: 0.75rem 0.8rem;
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
    font-size: 0.78rem;
    gap: 0.65rem;
  }

  .stat-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stat-value {
    margin-top: 0.3rem;
    font-size: 1.4rem;
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

  .tone-assigned {
    border-color: rgba(251, 191, 36, 0.3);
  }

  .search {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    border-radius: 10px;
    padding: 0.5rem 0.7rem;
    color: var(--text-secondary);
    width: min(420px, 100%);
    max-width: 100%;
  }

  .search-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text-primary);
    outline: none;
    min-width: 0;
  }

  .clear {
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-secondary);
    width: 28px;
    height: 28px;
    border-radius: 8px;
    cursor: pointer;
  }

  .clear:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .category-filter {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .cat-btn {
    display: inline-flex;
    align-items: center;
    padding: 0.3rem 0.6rem;
    border-radius: 8px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    font-size: 0.78rem;
    font-weight: 800;
    cursor: pointer;
    transition:
      border-color 0.12s ease,
      color 0.12s ease;
  }

  .cat-btn:hover {
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--text-primary);
  }

  .cat-btn.active {
    border-color: rgba(99, 102, 241, 0.5);
    background: rgba(99, 102, 241, 0.1);
    color: rgba(99, 102, 241, 0.95);
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

  .sub-link {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    color: rgba(99, 102, 241, 0.7);
    margin-left: 0.3rem;
    cursor: pointer;
    transition: color 0.15s ease;
  }

  .sub-link:hover {
    color: rgba(99, 102, 241, 1);
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
    padding: 0.18rem 0.5rem;
    font-size: 0.72rem;
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

  .badge.category.general {
    border-color: rgba(156, 163, 175, 0.35);
    color: var(--text-secondary);
    background: rgba(156, 163, 175, 0.06);
  }
  .badge.category.billing {
    border-color: rgba(59, 130, 246, 0.35);
    color: rgba(59, 130, 246, 0.95);
    background: rgba(59, 130, 246, 0.08);
  }
  .badge.category.technical {
    border-color: rgba(139, 92, 246, 0.35);
    color: rgba(139, 92, 246, 0.95);
    background: rgba(139, 92, 246, 0.08);
  }
  .badge.category.installation {
    border-color: rgba(34, 197, 94, 0.35);
    color: rgba(34, 197, 94, 0.95);
    background: rgba(34, 197, 94, 0.08);
  }

  .count {
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.35rem;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    padding: 0.22rem 0.46rem;
    border-radius: 999px;
    font-weight: 900;
    font-size: 0.8rem;
  }

  .icon-btn {
    width: 34px;
    height: 34px;
    border-radius: 8px;
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

  @media (max-width: 720px) {
    .filters {
      padding: 0.65rem;
    }

    .search {
      width: 100%;
    }
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

  @media (max-width: 640px) {
    .stats {
      grid-template-columns: 1fr;
    }
  }
</style>
