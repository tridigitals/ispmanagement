<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { EmailOutboxItem, EmailOutboxStats, PaginatedResponse } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import { formatDateTime } from '$lib/utils/date';

  let loading = $state(true);
  let loadingMore = $state(false);
  let busyId = $state<string | null>(null);

  let items = $state<EmailOutboxItem[]>([]);
  let stats = $state<EmailOutboxStats>({ all: 0, queued: 0, sending: 0, sent: 0, failed: 0 });

  let searchQuery = $state('');
  let statusFilter = $state<'all' | 'queued' | 'sending' | 'sent' | 'failed'>('all');

  let total = $state(0);
  let pageNum = $state(1);
  const perPage = 25;
  let ready = $state(false);

  let hasMore = $derived(items.length < total);

  const columns = $derived.by(() => [
    { key: 'to', label: $t('admin.email_outbox.columns.to') || 'To' },
    { key: 'subject', label: $t('admin.email_outbox.columns.subject') || 'Subject' },
    { key: 'status', label: $t('admin.email_outbox.columns.status') || 'Status' },
    { key: 'attempts', label: $t('admin.email_outbox.columns.attempts') || 'Attempts' },
    { key: 'scheduled', label: $t('admin.email_outbox.columns.scheduled') || 'Scheduled' },
    { key: 'updated', label: $t('admin.email_outbox.columns.updated') || 'Updated' },
    { key: 'actions', label: '', align: 'right' as const, width: '120px' },
  ]);

  onMount(async () => {
    if (!$can('read', 'email_outbox')) {
      goto('/unauthorized');
      return;
    }
    await refreshStats();
    await load(true);
    ready = true;
  });

  $effect(() => {
    if (!ready) return;
    const _q = searchQuery;
    const timer = setTimeout(() => void load(true), 250);
    return () => clearTimeout(timer);
  });

  async function refreshStats() {
    try {
      stats = await api.emailOutbox.stats();
    } catch {
      // non-blocking
    }
  }

  async function load(reset: boolean) {
    loading = true;
    if (reset) {
      pageNum = 1;
      items = [];
      total = 0;
    }
    try {
      const res: PaginatedResponse<EmailOutboxItem> = await api.emailOutbox.list({
        status: statusFilter === 'all' ? undefined : statusFilter,
        search: searchQuery.trim() || undefined,
        page: pageNum,
        perPage,
      });
      total = res.total || 0;
      items = reset ? res.data : [...items, ...res.data];
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
      const res: PaginatedResponse<EmailOutboxItem> = await api.emailOutbox.list({
        status: statusFilter === 'all' ? undefined : statusFilter,
        search: searchQuery.trim() || undefined,
        page: pageNum,
        perPage,
      });
      total = res.total || total;
      items = [...items, ...res.data];
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loadingMore = false;
    }
  }

  function setStatusFilter(v: typeof statusFilter) {
    if (statusFilter === v) return;
    statusFilter = v;
    void load(true);
    void refreshStats();
  }

  function tone(status: string) {
    const s = (status || '').toLowerCase();
    if (s === 'sent') return 'tone-sent';
    if (s === 'sending') return 'tone-sending';
    if (s === 'failed') return 'tone-failed';
    return 'tone-queued';
  }

  async function retry(id: string) {
    if (busyId) return;
    busyId = id;
    try {
      await api.emailOutbox.retry(id);
      toast.success($t('admin.email_outbox.toasts.requeued') || 'Requeued');
      await refreshStats();
      await load(true);
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      busyId = null;
    }
  }

  async function remove(id: string) {
    if (busyId) return;
    const ok = confirm($t('admin.email_outbox.confirm_delete') || 'Delete this outbox item?');
    if (!ok) return;
    busyId = id;
    try {
      await api.emailOutbox.delete(id);
      toast.success($t('common.deleted') || 'Deleted');
      await refreshStats();
      await load(true);
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      busyId = null;
    }
  }
</script>

<div class="page-content fade-in">
  <div class="head">
    <div>
      <h1 class="title">{$t('admin.email_outbox.title') || 'Email Outbox'}</h1>
      <p class="sub">
        {$t('admin.email_outbox.subtitle') || 'Monitor queued and failed emails, and retry delivery.'}
      </p>
    </div>

    <button class="btn" type="button" onclick={() => (refreshStats(), load(true))} title={$t('common.refresh') || 'Refresh'}>
      <Icon name="refresh-cw" size={16} />
      {$t('common.refresh') || 'Refresh'}
    </button>
  </div>

  <div class="stats">
    <button class="stat-card" class:active={statusFilter === 'all'} type="button" onclick={() => setStatusFilter('all')}>
      <div class="stat-top">
        <span class="stat-label">{$t('common.all') || 'All'}</span>
        <Icon name="list" size={14} />
      </div>
      <div class="stat-value">{stats.all}</div>
    </button>
    <button class="stat-card tone-queued" class:active={statusFilter === 'queued'} type="button" onclick={() => setStatusFilter('queued')}>
      <div class="stat-top">
        <span class="stat-label">{$t('admin.email_outbox.status.queued') || 'Queued'}</span>
        <Icon name="calendar" size={14} />
      </div>
      <div class="stat-value">{stats.queued}</div>
    </button>
    <button class="stat-card tone-sending" class:active={statusFilter === 'sending'} type="button" onclick={() => setStatusFilter('sending')}>
      <div class="stat-top">
        <span class="stat-label">{$t('admin.email_outbox.status.sending') || 'Sending'}</span>
        <Icon name="send" size={14} />
      </div>
      <div class="stat-value">{stats.sending}</div>
    </button>
    <button class="stat-card tone-sent" class:active={statusFilter === 'sent'} type="button" onclick={() => setStatusFilter('sent')}>
      <div class="stat-top">
        <span class="stat-label">{$t('admin.email_outbox.status.sent') || 'Sent'}</span>
        <Icon name="check-circle" size={14} />
      </div>
      <div class="stat-value">{stats.sent}</div>
    </button>
    <button class="stat-card tone-failed" class:active={statusFilter === 'failed'} type="button" onclick={() => setStatusFilter('failed')}>
      <div class="stat-top">
        <span class="stat-label">{$t('admin.email_outbox.status.failed') || 'Failed'}</span>
        <Icon name="alert-triangle" size={14} />
      </div>
      <div class="stat-value">{stats.failed}</div>
    </button>
  </div>

  <div class="filters">
    <div class="search">
      <Icon name="search" size={16} />
      <input
        class="search-input"
        bind:value={searchQuery}
        placeholder={$t('admin.email_outbox.search') || 'Search by recipient or subject...'}
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
    data={items}
    {loading}
    emptyText={$t('admin.email_outbox.empty') || 'No outbox items'}
  >
    {#snippet cell({ item, key }: { item: EmailOutboxItem; key: string })}
      {#if key === 'to'}
        <div class="cell-main">
          <div class="primary">{item.to_email}</div>
          <div class="muted small">{formatDateTime(item.created_at)}</div>
        </div>
      {:else if key === 'subject'}
        <div class="cell-main">
          <div class="primary">{item.subject}</div>
          {#if item.status === 'failed' && item.last_error}
            <div class="muted small mono clamp-1" title={item.last_error}>{item.last_error}</div>
          {:else}
            <div class="muted small clamp-1">{item.body}</div>
          {/if}
        </div>
      {:else if key === 'status'}
        <span class="pill {tone(item.status)}">{(item.status || 'queued').toUpperCase()}</span>
      {:else if key === 'attempts'}
        <span class="mono">{item.attempts}/{item.max_attempts}</span>
      {:else if key === 'scheduled'}
        <span class="muted">{formatDateTime(item.scheduled_at)}</span>
      {:else if key === 'updated'}
        <span class="muted">{formatDateTime(item.updated_at)}</span>
      {:else if key === 'actions'}
        {@const canRetry = item.status === 'failed' || item.status === 'queued'}
        {@const disabled = busyId === item.id || item.status === 'sending'}
        <div class="actions">
          <button
            class="icon-btn"
            title={$t('admin.email_outbox.actions.retry') || 'Retry'}
            disabled={!canRetry || disabled}
            onclick={() => retry(item.id)}
          >
            <Icon name="refresh-cw" size={16} />
          </button>
          <button
            class="icon-btn danger"
            title={$t('common.delete') || 'Delete'}
            disabled={disabled}
            onclick={() => remove(item.id)}
          >
            <Icon name="trash-2" size={16} />
          </button>
        </div>
      {:else}
        {item[key as keyof EmailOutboxItem] ?? ''}
      {/if}
    {/snippet}
  </Table>

  {#if hasMore && !loading}
    <div class="more">
      <button class="btn btn-secondary" type="button" onclick={loadMore} disabled={loadingMore}>
        {loadingMore ? $t('common.loading') || 'Loading...' : $t('common.load_more') || 'Load more'}
      </button>
    </div>
  {/if}
</div>

<style>
  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1.25rem;
  }

  .title {
    font-size: 1.45rem;
    font-weight: 800;
    letter-spacing: -0.02em;
    color: var(--text-primary);
    margin: 0;
  }

  .sub {
    margin: 0.25rem 0 0;
    color: var(--text-secondary);
    max-width: 62ch;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .stat-card {
    border: 1px solid var(--border-color);
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0.02));
    border-radius: var(--radius-lg);
    padding: 0.9rem 1rem;
    text-align: left;
    cursor: pointer;
    transition: transform 0.12s ease, border-color 0.12s ease;
    color: var(--text-primary);
  }

  .stat-card:hover {
    transform: translateY(-1px);
    border-color: rgba(120, 130, 255, 0.45);
  }

  .stat-card.active {
    border-color: rgba(120, 130, 255, 0.7);
    box-shadow: 0 0 0 3px rgba(120, 130, 255, 0.12);
  }

  .stat-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    color: var(--text-secondary);
    font-weight: 700;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .stat-value {
    margin-top: 0.5rem;
    font-size: 1.5rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .filters {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin: 1rem 0;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    border-radius: 999px;
    padding: 0.55rem 0.75rem;
    width: min(520px, 100%);
  }

  .search-input {
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    width: 100%;
  }

  .clear {
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 0.25rem;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 0.25rem 0.55rem;
    font-size: 0.75rem;
    font-weight: 800;
    letter-spacing: 0.06em;
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.03);
  }

  .tone-queued {
    border-color: rgba(120, 130, 255, 0.35);
    background: rgba(120, 130, 255, 0.12);
  }
  .tone-sending {
    border-color: rgba(0, 214, 255, 0.35);
    background: rgba(0, 214, 255, 0.12);
  }
  .tone-sent {
    border-color: rgba(51, 214, 116, 0.35);
    background: rgba(51, 214, 116, 0.12);
  }
  .tone-failed {
    border-color: rgba(255, 90, 90, 0.4);
    background: rgba(255, 90, 90, 0.12);
  }

  .cell-main .primary {
    font-weight: 800;
    color: var(--text-primary);
  }

  .muted {
    color: var(--text-secondary);
  }

  .small {
    font-size: 0.82rem;
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
  }

  .clamp-1 {
    display: -webkit-box;
    line-clamp: 1;
    -webkit-line-clamp: 1;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .actions {
    display: inline-flex;
    gap: 0.35rem;
    justify-content: flex-end;
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-primary);
    cursor: pointer;
    transition: transform 0.12s ease, border-color 0.12s ease;
  }

  .icon-btn:hover:enabled {
    transform: translateY(-1px);
    border-color: rgba(120, 130, 255, 0.45);
  }

  .icon-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .icon-btn.danger:hover:enabled {
    border-color: rgba(255, 90, 90, 0.6);
  }

  .more {
    display: flex;
    justify-content: center;
    margin-top: 1rem;
  }

  @media (max-width: 980px) {
    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .search {
      width: 100%;
    }
  }
</style>
