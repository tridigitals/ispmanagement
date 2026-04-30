<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { api } from '$lib/api/client';
  import type {
    PaginatedResponse,
    SupportTicketListItem,
    SupportTicketStats,
  } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import { toast } from '$lib/stores/toast';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';

  let tickets = $state<SupportTicketListItem[]>([]);
  let stats = $state<SupportTicketStats>({ all: 0, open: 0, pending: 0, closed: 0 });
  let total = $state(0);
  let pageNum = $state(1);
  const perPage = 20;

  let loading = $state(true);
  let loadingMore = $state(false);
  let creating = $state(false);
  let showCreate = $state(false);

  let searchQuery = $state('');
  let subject = $state('');
  let message = $state('');
  let priority = $state<'low' | 'normal' | 'high' | 'urgent'>('normal');
  let attachments = $state<File[]>([]);

  let statusFilter = $state<'all' | 'open' | 'pending' | 'closed'>('all');

  let hasMore = $derived(tickets.length < total);
  let ready = $state(false);

  const priorityOptions = [
    { label: get(t)('support.priorities.low') || 'Low', value: 'low' },
    { label: get(t)('support.priorities.normal') || 'Normal', value: 'normal' },
    { label: get(t)('support.priorities.high') || 'High', value: 'high' },
    { label: get(t)('support.priorities.urgent') || 'Urgent', value: 'urgent' },
  ];

  function onPickFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    attachments = Array.from(input.files || []);
  }

  onMount(async () => {
    if (!$can('read', 'support') && !$can('create', 'support')) {
      goto('/unauthorized');
      return;
    }
    await refreshStats();
    await loadTickets(true);
    ready = true;
  });

  $effect(() => {
    if (!ready) return;
    const q = searchQuery;
    const timer = setTimeout(() => {
      void loadTickets(true);
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

  async function loadTickets(reset: boolean) {
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

  function openTicket(id: string) {
    goto(`${$page.url.pathname}/${id}`);
  }

  function setStatusFilter(v: typeof statusFilter) {
    if (statusFilter === v) return;
    statusFilter = v;
    void loadTickets(true);
  }

  async function submitCreate() {
    if (!subject.trim() || !message.trim()) return;
    creating = true;
    try {
      const ids: string[] = [];
      for (const f of attachments) {
        const record = await api.storage.uploadFile(f);
        ids.push(record.id);
      }

      const detail = await api.support.create(subject, message, priority, ids);
      toast.success(get(t)('support.toasts.created') || 'Ticket created');
      showCreate = false;
      subject = '';
      message = '';
      priority = 'normal';
      attachments = [];
      await refreshStats();
      await loadTickets(true);
      goto(`${$page.url.pathname}/${detail.ticket.id}`);
    } catch (e: any) {
      toast.error(
        get(t)('support.toasts.create_failed', { values: { message: e?.message || e } }) ||
          `Create failed: ${e?.message || e}`,
      );
    } finally {
      creating = false;
    }
  }
</script>

<div class="page-content fade-in">
  <header class="hero">
    <div class="hero-left">
      <div class="hero-icon">
        <Icon name="life-buoy" size={18} />
      </div>
      <div>
        <h1 class="title">{$t('support.title') || 'Support Tickets'}</h1>
        <p class="sub">{$t('support.subtitle') || 'Get help from your team or platform admins.'}</p>
      </div>
    </div>

    <div class="hero-right">
      <div class="search">
        <Icon name="search" size={16} />
        <input
          class="search-input"
          bind:value={searchQuery}
          placeholder={$t('support.search_placeholder') || 'Search tickets...'}
        />
        {#if searchQuery}
          <button class="clear" type="button" onclick={() => (searchQuery = '')}>
            <Icon name="x" size={14} />
          </button>
        {/if}
      </div>

      <div class="actions">
        <div class="filter" aria-label={$t('support.filters.aria') || 'Filter'}>
          <button class:active={statusFilter === 'all'} onclick={() => setStatusFilter('all')}>
            {$t('support.filters.all') || 'All'}
          </button>
          <button class:active={statusFilter === 'open'} onclick={() => setStatusFilter('open')}>
            {$t('support.filters.open') || 'Open'}
          </button>
          <button
            class:active={statusFilter === 'pending'}
            onclick={() => setStatusFilter('pending')}
          >
            {$t('support.filters.pending') || 'Pending'}
          </button>
          <button
            class:active={statusFilter === 'closed'}
            onclick={() => setStatusFilter('closed')}
          >
            {$t('support.filters.closed') || 'Closed'}
          </button>
        </div>

        {#if $can('create', 'support')}
          <button class="btn-primary" onclick={() => (showCreate = true)} type="button">
            <Icon name="plus" size={16} />
            {$t('support.actions.new') || 'New Ticket'}
          </button>
        {/if}
      </div>
    </div>
  </header>

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

  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <p>{$t('support.loading') || 'Loading tickets...'}</p>
    </div>
  {:else if tickets.length === 0}
    <div class="empty">
      <Icon name="help-circle" size={28} />
      <div class="empty-title">{$t('support.empty.title') || 'No tickets yet'}</div>
      <div class="empty-sub">
        {$t('support.empty.subtitle') || 'Create a ticket if you need help or have a request.'}
      </div>
      {#if $can('create', 'support')}
        <button class="btn-primary" onclick={() => (showCreate = true)} type="button">
          {$t('support.actions.new') || 'New Ticket'}
        </button>
      {/if}
    </div>
  {:else}
    <div class="list">
      {#each tickets as item (item.id)}
        <button class="card" type="button" onclick={() => openTicket(item.id)}>
          <div class="card-top">
            <div class="subject">{item.subject}</div>
            <div class="meta">
              <span class="badge status {item.status}">
                {$t(`support.status.${item.status}`) || item.status}
              </span>
              <span class="badge priority {item.priority}">
                {$t(`support.priorities.${item.priority}`) || item.priority}
              </span>
            </div>
          </div>
          <div class="card-bottom">
            <div class="info">
              <span>
                {formatDateTime(item.last_message_at || item.updated_at, {
                  timeZone: $appSettings.app_timezone,
                })}
              </span>
            </div>
            <div class="count">
              <Icon name="message-circle" size={14} />
              {item.message_count}
            </div>
          </div>
        </button>
      {/each}
    </div>

    {#if hasMore}
      <div class="footer">
        <button class="btn-more" type="button" onclick={loadMore} disabled={loadingMore}>
          {#if loadingMore}
            <div class="spinner-sm"></div>
          {/if}
          {$t('common.load_more') || 'Load more'}
        </button>
        <div class="foot-note">
          {tickets.length}/{total}
        </div>
      </div>
    {/if}
  {/if}
</div>

<Modal
  bind:show={showCreate}
  title={$t('support.create.title') || 'Create Ticket'}
  onclose={() => (showCreate = false)}
>
  <div class="modal-body">
    <Input
      label={$t('support.fields.subject') || 'Subject'}
      placeholder={$t('support.fields.subject_placeholder') || 'e.g. Cannot login to my account'}
      bind:value={subject}
    />

    <div class="textarea-group">
      <label class="label" for="support-message">{$t('support.fields.message') || 'Message'}</label>
      <textarea
        id="support-message"
        class="textarea"
        rows="6"
        bind:value={message}
        placeholder={$t('support.fields.message_placeholder') ||
          'Describe your issue or request...'}
      ></textarea>
    </div>

    <Select
      label={$t('support.fields.priority') || 'Priority'}
      bind:value={priority}
      options={priorityOptions}
    />

    <div class="file-group">
      <label class="label" for="support-attachments">
        {$t('support.fields.attachments') || 'Attachments'}
      </label>
      <input id="support-attachments" class="file" type="file" multiple onchange={onPickFiles} />
      {#if attachments.length}
        <div class="file-list">
          {#each attachments as f (f.name)}
            <div class="file-item">
              <Icon name="paperclip" size={14} />
              <span class="file-name">{f.name}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <div class="modal-actions">
      <button class="btn" type="button" onclick={() => (showCreate = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn-primary" type="button" onclick={submitCreate} disabled={creating}>
        {creating
          ? $t('support.actions.creating') || 'Creating...'
          : $t('support.actions.create') || 'Create'}
      </button>
    </div>
  </div>
</Modal>

<style>
  .page-content {
    padding: 1.5rem;
    max-width: 1100px;
    margin: 0 auto;
  }

  .hero {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
    padding: 1rem;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    background:
      radial-gradient(1200px 220px at 20% 0%, rgba(99, 102, 241, 0.22), transparent 55%),
      radial-gradient(800px 180px at 85% 0%, rgba(16, 185, 129, 0.12), transparent 45%),
      var(--bg-surface);
    box-shadow: var(--shadow-md);
  }

  .hero-left {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    min-width: 260px;
  }

  .hero-icon {
    width: 42px;
    height: 42px;
    border-radius: 14px;
    display: grid;
    place-items: center;
    background: rgba(99, 102, 241, 0.12);
    border: 1px solid rgba(99, 102, 241, 0.25);
    color: rgba(99, 102, 241, 0.95);
    flex: 0 0 auto;
  }

  .hero-right {
    display: grid;
    gap: 0.6rem;
    justify-items: end;
  }

  .title {
    margin: 0;
    font-size: 1.6rem;
    font-weight: 900;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .sub {
    margin: 0.35rem 0 0 0;
    color: var(--text-secondary);
    font-size: 0.95rem;
  }

  .search {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    border-radius: 12px;
    padding: 0.55rem 0.65rem;
    min-width: min(520px, 86vw);
  }

  .search-input {
    width: 100%;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-weight: 700;
  }

  .clear {
    width: 28px;
    height: 28px;
    border-radius: 10px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-secondary);
    display: grid;
    place-items: center;
    cursor: pointer;
  }

  .clear:hover {
    border-color: var(--border-color);
    background: rgba(255, 255, 255, 0.04);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
    justify-content: flex-end;
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
    font-weight: 700;
    font-size: 0.85rem;
  }

  .filter button.active {
    background: rgba(99, 102, 241, 0.15);
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--text-primary);
  }

  .btn-primary {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--color-primary);
    color: white;
    border: none;
    padding: 0.6rem 0.9rem;
    border-radius: 10px;
    cursor: pointer;
    font-weight: 700;
  }

  .btn-primary:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  .btn {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 0.6rem 0.9rem;
    border-radius: 10px;
    cursor: pointer;
    font-weight: 700;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
    margin: 0.9rem 0 1.25rem 0;
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

  .loading {
    display: grid;
    place-items: center;
    padding: 3rem 1rem;
    gap: 0.75rem;
    color: var(--text-secondary);
  }

  .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
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

  .empty {
    border: 1px dashed var(--border-color);
    border-radius: var(--radius-lg);
    padding: 2.5rem 1.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 0.6rem;
    color: var(--text-secondary);
  }

  .empty-title {
    color: var(--text-primary);
    font-weight: 800;
    font-size: 1.1rem;
    margin-top: 0.25rem;
  }

  .empty-sub {
    max-width: 520px;
    font-size: 0.95rem;
  }

  .list {
    display: grid;
    gap: 0.75rem;
  }

  .card {
    width: 100%;
    text-align: left;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 0.95rem;
    cursor: pointer;
    transition:
      transform 0.12s ease,
      border-color 0.12s ease;
  }

  .card:hover {
    border-color: rgba(99, 102, 241, 0.35);
    transform: translateY(-1px);
  }

  .card-top {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
  }

  .subject {
    color: var(--text-primary);
    font-weight: 800;
    font-size: 1rem;
    line-height: 1.2;
    flex: 1;
  }

  .meta {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .badge {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 0.2rem 0.55rem;
    font-size: 0.75rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    background: rgba(255, 255, 255, 0.03);
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

  .card-bottom {
    margin-top: 0.6rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: var(--text-secondary);
    font-size: 0.85rem;
    gap: 1rem;
  }

  .info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }

  .dot {
    width: 4px;
    height: 4px;
    border-radius: 999px;
    background: var(--border-color);
  }

  .count {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    padding: 0.25rem 0.5rem;
    border-radius: 999px;
    font-weight: 800;
  }

  .mono {
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
    font-size: 0.85rem;
  }

  .modal-body {
    display: grid;
    gap: 1rem;
  }

  .textarea-group {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .label {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin-left: 0.2rem;
  }

  .textarea {
    width: 100%;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: var(--radius-md);
    padding: 0.75rem 1rem;
    font-size: 0.95rem;
    resize: vertical;
    min-height: 120px;
  }

  .textarea:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .file-group {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .file {
    width: 100%;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: var(--radius-md);
    padding: 0.6rem 0.8rem;
    font-size: 0.9rem;
  }

  .file-list {
    display: grid;
    gap: 0.4rem;
    margin-top: 0.2rem;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.65rem;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-secondary);
  }

  .file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
    margin-top: 0.25rem;
  }

  @media (max-width: 900px) {
    .hero-right {
      justify-items: stretch;
      width: 100%;
    }

    .actions {
      justify-content: flex-start;
    }

    .search {
      min-width: 100%;
    }

    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
