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
  let stats = $state<SupportTicketStats>({
    all: 0,
    open: 0,
    pending: 0,
    closed: 0,
    resolved: 0,
    unassigned: 0,
  });
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
  let category = $state<'general' | 'billing' | 'technical' | 'installation'>('general');
  let subscriptionId = $state<string | undefined>(undefined);
  let subscriptions = $state<Array<{ id: string; label: string }>>([]);
  let attachments = $state<{ file: File; name: string }[]>([]);

  // Quick action presets (mirrors mobile customer app)
  type QuickAction = { icon: string; label: string; subject: string; message: string; category: typeof category };
  const quickActions: QuickAction[] = [
    { icon: 'wifi-off',    label: 'Internet Mati',     subject: 'Internet mati / tidak connect', message: 'Halo, internet saya mati total. Tidak ada koneksi sama sekali. Mohon bantuan.', category: 'technical' },
    { icon: 'wifi',        label: 'Internet Lambat',   subject: 'Internet lambat',               message: 'Halo, koneksi internet saya sangat lambat. Kecepatan jauh di bawah normal.',     category: 'technical' },
    { icon: 'plus',        label: 'Lainnya',           subject: '',                              message: '',                                                                             category: 'general' },
  ];

  let statusFilter = $state<'all' | 'open' | 'pending' | 'closed'>('all');
  let categoryFilter = $state<'all' | 'general' | 'billing' | 'technical' | 'installation'>('all');

  let hasMore = $derived(tickets.length < total);
  let ready = $state(false);

  // svelte-i18n returns the key itself when missing — falsy-or fallback never fires
  function tt(key: string, fallback: string) {
    const v = get(t)(key);
    return !v || v === key ? fallback : v;
  }

  function normStatus(status: string) {
    const s = String(status || '').toLowerCase();
    if (s === 'resolved' || s === 'done' || s === 'completed') return 'closed';
    if (s === 'in_progress' || s === 'waiting') return 'pending';
    return s;
  }

  function statusLabel(status: string) {
    const s = normStatus(status);
    if (s === 'open') return tt('support.status.open', 'Open');
    if (s === 'pending') return tt('support.status.pending', 'Pending');
    if (s === 'closed') return tt('support.status.closed', 'Resolved');
    return status || '—';
  }

  function priorityLabel(p: string) {
    return tt(`support.priorities.${p}`, p || '—');
  }

  function categoryLabel(c: string) {
    return tt(`support.categories.${c}`, c || '—');
  }

  const priorityOptions = [
    { label: get(t)('support.priorities.low') || 'Low', value: 'low' },
    { label: get(t)('support.priorities.normal') || 'Normal', value: 'normal' },
    { label: get(t)('support.priorities.high') || 'High', value: 'high' },
    { label: get(t)('support.priorities.urgent') || 'Urgent', value: 'urgent' },
  ];

  const categoryOptions = [
    { label: get(t)('support.categories.all') || 'All', value: 'all' },
    { label: get(t)('support.categories.general') || 'General', value: 'general' },
    { label: get(t)('support.categories.billing') || 'Billing', value: 'billing' },
    { label: get(t)('support.categories.technical') || 'Technical', value: 'technical' },
    { label: get(t)('support.categories.installation') || 'Installation', value: 'installation' },
  ];

  const createCategoryOptions = [
    { label: get(t)('support.categories.general') || 'General', value: 'general' },
    { label: get(t)('support.categories.billing') || 'Billing', value: 'billing' },
    { label: get(t)('support.categories.technical') || 'Technical', value: 'technical' },
    { label: get(t)('support.categories.installation') || 'Installation', value: 'installation' },
  ];

  function onPickFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    const files = Array.from(input.files || []);
    // Deduplicate by name+size
    for (const file of files) {
      if (!attachments.some(a => a.file.name === file.name && a.file.size === file.size)) {
        attachments = [...attachments, { file, name: file.name }];
      }
    }
  }

  function removeAttachment(idx: number) {
    attachments = attachments.filter((_, i) => i !== idx);
  }

  function applyQuickAction(action: QuickAction) {
    subject = action.subject;
    message = action.message;
    category = action.category;
  }

  onMount(async () => {
    if (!$can('read', 'support') && !$can('create', 'support')) {
      goto('/unauthorized');
      return;
    }
    await refreshStats();
    await loadTickets(true);
    // Load subscriptions for the create modal
    try {
      const res = await api.customers.portal.mySubscriptions({ per_page: 50, status: 'active' });
      subscriptions = (res.data || []).map((s: any) => ({
        id: s.id,
        label: s.package_name || s.plan_name || 'Langganan',
      }));
    } catch (_) { /* ignore */ }
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
        category: categoryFilter === 'all' ? undefined : categoryFilter,
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
        category: categoryFilter === 'all' ? undefined : categoryFilter,
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
    if (subject.trim().length < 3 || message.trim().length < 10) {
      toast.error(get(t)('support.validation.short') || 'Subject min 3 characters, message min 10 characters');
      return;
    }
    creating = true;
    try {
      const ids: string[] = [];
      for (const att of attachments) {
        const record = await api.storage.uploadFile(att.file);
        ids.push(record.id);
      }

      const detail = await api.support.create(subject, message, priority, category, subscriptionId, ids);
      toast.success(get(t)('support.toasts.created') || 'Ticket created');
      showCreate = false;
      subject = '';
      message = '';
      priority = 'normal';
      category = 'general';
      subscriptionId = undefined;
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

<div class="page fade-in">
  <div class="page-head">
    <div class="page-head-text">
      <h1>{$t('support.title') || 'Support'}</h1>
      <p class="page-sub">
        {#if loading}
          {$t('support.loading') || 'Memuat...'}
        {:else}
          {stats.all} tiket
          {#if stats.open > 0} · {stats.open} open{/if}
          {#if stats.pending > 0} · {stats.pending} pending{/if}
        {/if}
      </p>
    </div>
    <div class="head-actions">
      <div class="search">
        <Icon name="search" size={16} />
        <input
          class="search-input"
          bind:value={searchQuery}
          placeholder={$t('support.search_placeholder') || 'Cari tiket...'}
        />
        {#if searchQuery}
          <button class="clear" type="button" onclick={() => (searchQuery = '')} aria-label="Clear">
            <Icon name="x" size={14} />
          </button>
        {/if}
      </div>
      <Select
        bind:value={statusFilter}
        options={[
          { label: $t('support.filters.all') || 'All', value: 'all' },
          { label: $t('support.filters.open') || 'Open', value: 'open' },
          { label: $t('support.filters.pending') || 'Pending', value: 'pending' },
          { label: $t('support.filters.closed') || 'Closed', value: 'closed' },
        ]}
        placeholder={$t('support.filters.status') || 'Status'}
        width="130px"
        onchange={() => void loadTickets(true)}
      />
      <Select
        bind:value={categoryFilter}
        options={categoryOptions}
        placeholder={$t('support.filters.category') || 'Category'}
        width="140px"
        onchange={() => void loadTickets(true)}
      />
      {#if $can('create', 'support')}
        <button class="btn btn-primary" onclick={() => (showCreate = true)} type="button">
          <Icon name="plus" size={16} />
          <span>{$t('support.actions.new') || 'Buat tiket'}</span>
        </button>
      {/if}
    </div>
  </div>

  {#if !loading}
    <div class="kpis">
      <button class="kpi" class:active={statusFilter === 'all'} type="button" onclick={() => setStatusFilter('all')}>
        <div class="kpi-label">{$t('support.stats.total') || 'Total'}</div>
        <div class="kpi-val">{stats.all}</div>
        <div class="kpi-sub">semua</div>
      </button>
      <button class="kpi" class:active={statusFilter === 'open'} type="button" onclick={() => setStatusFilter('open')}>
        <div class="kpi-label">{$t('support.filters.open') || 'Open'}</div>
        <div class="kpi-val {stats.open > 0 ? 'ok' : ''}">{stats.open}</div>
        <div class="kpi-sub">butuh respon</div>
      </button>
      <button class="kpi" class:active={statusFilter === 'pending'} type="button" onclick={() => setStatusFilter('pending')}>
        <div class="kpi-label">{$t('support.filters.pending') || 'Pending'}</div>
        <div class="kpi-val {stats.pending > 0 ? 'warn' : ''}">{stats.pending}</div>
        <div class="kpi-sub">menunggu</div>
      </button>
      <button class="kpi" class:active={statusFilter === 'closed'} type="button" onclick={() => setStatusFilter('closed')}>
        <div class="kpi-label">{$t('support.filters.closed') || 'Closed'}</div>
        <div class="kpi-val">{stats.closed}</div>
        <div class="kpi-sub">selesai</div>
      </button>
    </div>
  {/if}

  {#if loading}
    <div class="panel">
      <div class="state">
        <div class="spinner"></div>
        <p>{$t('support.loading') || 'Loading...'}</p>
      </div>
    </div>
  {:else if tickets.length === 0}
    <div class="panel">
      <div class="state">
        <Icon name="inbox" size={36} />
        <h3>{$t('support.empty.title') || 'Belum ada tiket'}</h3>
        <p>{$t('support.empty.subtitle') || 'Buat tiket jika butuh bantuan'}</p>
        {#if $can('create', 'support')}
          <button class="btn btn-primary" onclick={() => (showCreate = true)} type="button">
            {$t('support.actions.new') || 'Buat tiket'}
          </button>
        {/if}
      </div>
    </div>
  {:else}
    <div class="list">
      {#each tickets as item (item.id)}
        <button class="card panel" type="button" onclick={() => openTicket(item.id)}>
          <div class="card-top">
            <div class="subject">{item.subject}</div>
            <div class="meta">
              <span class="pill status-{normStatus(item.status)}">
                {statusLabel(item.status)}
              </span>
              <span class="pill priority-{item.priority}">
                {priorityLabel(item.priority)}
              </span>
              {#if item.category}
                <span class="pill">
                  {categoryLabel(item.category)}
                </span>
              {/if}
            </div>
          </div>
          <div class="card-bottom">
            <span class="info">
              {formatDateTime(item.last_message_at || item.updated_at, {
                timeZone: $appSettings.app_timezone,
              })}
            </span>
            <span class="count">
              <Icon name="message-circle" size={14} />
              {item.message_count}
            </span>
          </div>
        </button>
      {/each}
    </div>

    {#if hasMore}
      <div class="footer">
        <button class="btn btn-secondary" type="button" onclick={loadMore} disabled={loadingMore}>
          {loadingMore ? ($t('common.loading') || '...') : ($t('common.load_more') || 'Load more')}
        </button>
        <div class="foot-note">{tickets.length}/{total}</div>
      </div>
    {/if}
  {/if}
</div>

<Modal
  bind:show={showCreate}
  title={$t('support.create.title')}
  onclose={() => (showCreate = false)}
>
  <div class="modal-body">
    <!-- Quick action chips — one-tap to pre-fill form -->
    <div class="quick-actions">
      <span class="quick-label">{$t('support.quick.label')}</span>
      {#each quickActions as action}
        <button
          class="quick-chip"
          class:active={subject === action.subject && category === action.category}
          type="button"
          onclick={() => applyQuickAction(action)}
        >
          <Icon name={action.icon} size={14} />
          {action.label}
        </button>
      {/each}
    </div>

    <Input
      label={$t('support.fields.subject')}
      placeholder={$t('support.fields.subject_placeholder')}
      bind:value={subject}
    />

    <div class="textarea-group">
      <label class="label" for="support-message">{$t('support.fields.message')}</label>
      <textarea
        id="support-message"
        class="textarea"
        rows="6"
        bind:value={message}
        placeholder={$t('support.fields.message_placeholder')}
      ></textarea>
    </div>

    <Select
      label={$t('support.fields.category')}
      bind:value={category}
      options={createCategoryOptions}
    />

    {#if subscriptions.length > 0}
      <Select
        label={$t('support.fields.subscription')}
        bind:value={subscriptionId}
        options={[
          { label: $t('support.fields.no_subscription') || 'Tidak terkait', value: undefined },
          ...subscriptions.map(s => ({ label: s.label, value: s.id })),
        ]}
      />
    {/if}

    <Select
      label={$t('support.fields.priority')}
      bind:value={priority}
      options={priorityOptions}
    />

    <div class="file-group">
      <label class="label" for="support-attachments">
        <Icon name="paperclip" size={14} />
        {$t('support.fields.attachments')}
      </label>
      <div class="file-input-row">
        <label class="btn-file" for="support-attachments">
          <Icon name="plus" size={14} />
          {$t('support.actions.add_file')}
        </label>
        <input id="support-attachments" class="file-hidden" type="file" multiple onchange={onPickFiles} />
        {#if attachments.length}
          <span class="file-count">{$t('support.fields.file_count')?.replace('{count}', String(attachments.length)) || `${attachments.length} file(s)`}</span>
        {/if}
      </div>
      {#if attachments.length}
        <div class="file-list">
          {#each attachments as att, i}
            <div class="file-chip">
              <Icon name="file" size={14} />
              <span class="file-name">{att.name}</span>
              <button class="file-remove" type="button" onclick={() => removeAttachment(i)} title={$t('common.remove')}>
                <Icon name="x" size={12} />
              </button>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <div class="modal-actions">
      <button class="btn" type="button" onclick={() => (showCreate = false)}>
        {$t('common.cancel')}
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
  .page {
      padding: clamp(1rem, 2.2vw, 1.75rem);
      max-width: 1100px;
      margin: 0 auto;
      display: flex;
      flex-direction: column;
      gap: 1rem;
    }
    .page-head {
      display: flex;
      justify-content: space-between;
      align-items: flex-end;
      gap: 1rem;
      flex-wrap: wrap;
    }
    .page-head h1 {
      font-size: clamp(1.25rem, 2.2vw, 1.45rem);
      font-weight: 750;
      letter-spacing: -0.02em;
      margin: 0;
      color: var(--text-primary);
    }
    .page-sub {
      color: var(--text-secondary);
      font-size: 0.88rem;
      margin: 0.25rem 0 0;
    }
    .head-actions {
      display: flex;
      gap: 0.5rem;
      flex-wrap: wrap;
      align-items: center;
    }

    .search {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      border: 1px solid rgba(255, 255, 255, 0.08);
      background: rgba(255, 255, 255, 0.02);
      border-radius: 10px;
      padding: 0.5rem 0.65rem;
      min-width: min(240px, 100%);
      color: var(--text-secondary);
    }
    .search-input {
      width: 100%;
      background: transparent;
      border: none;
      outline: none;
      color: var(--text-primary);
      font-size: 0.9rem;
    }
    .clear {
      width: 28px;
      height: 28px;
      border-radius: 8px;
      border: none;
      background: transparent;
      color: var(--text-secondary);
      display: grid;
      place-items: center;
      cursor: pointer;
    }
    .clear:hover {
      background: rgba(255, 255, 255, 0.04);
      color: var(--text-primary);
    }

    .kpis {
      display: grid;
      grid-template-columns: repeat(4, 1fr);
      gap: 0.7rem;
    }
    .kpi {
      text-align: left;
      background: rgba(255, 255, 255, 0.015);
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: 10px;
      padding: 0.9rem 1rem;
      cursor: pointer;
      color: inherit;
    }
    .kpi.active {
      border-color: color-mix(in srgb, var(--color-primary) 45%, transparent);
    }
    .kpi-label {
      font-size: 0.7rem;
      font-weight: 650;
      letter-spacing: 0.04em;
      text-transform: uppercase;
      color: var(--text-tertiary);
      margin-bottom: 0.35rem;
    }
    .kpi-val {
      font-size: 1.25rem;
      font-weight: 750;
      letter-spacing: -0.02em;
      color: var(--text-primary);
    }
    .kpi-val.ok { color: var(--color-success); }
    .kpi-val.warn { color: var(--color-warning); }
    .kpi-sub {
      font-size: 0.74rem;
      color: var(--text-secondary);
      margin-top: 0.2rem;
    }

    .panel {
      background: rgba(255, 255, 255, 0.015);
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: var(--radius-lg, 12px);
    }
    .state {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      text-align: center;
      min-height: 220px;
      gap: 0.5rem;
      padding: 2rem 1.25rem;
      color: var(--text-secondary);
    }
    .state h3 {
      margin: 0.5rem 0 0;
      color: var(--text-primary);
      font-size: 1.05rem;
    }
    .state p {
      margin: 0 0 0.75rem;
      font-size: 0.88rem;
      max-width: 320px;
    }
    .spinner {
      width: 28px;
      height: 28px;
      border: 3px solid rgba(255, 255, 255, 0.08);
      border-top-color: var(--color-primary);
      border-radius: 50%;
      animation: spin 0.7s linear infinite;
    }
    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    .list {
      display: grid;
      gap: 0.65rem;
    }
    .card {
      width: 100%;
      text-align: left;
      cursor: pointer;
      padding: 0.95rem 1rem;
      transition: border-color 0.12s ease;
      color: inherit;
    }
    .card:hover {
      border-color: rgba(99, 102, 241, 0.35);
    }
    .card-top {
      display: flex;
      justify-content: space-between;
      gap: 1rem;
      align-items: flex-start;
    }
    .subject {
      color: var(--text-primary);
      font-weight: 650;
      font-size: 0.98rem;
      line-height: 1.3;
      flex: 1;
    }
    .meta {
      display: flex;
      gap: 0.35rem;
      flex-wrap: wrap;
      justify-content: flex-end;
    }
    .pill {
      display: inline-flex;
      align-items: center;
      padding: 0.15rem 0.5rem;
      border-radius: 999px;
      font-size: 0.7rem;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.02em;
      border: 1px solid rgba(255, 255, 255, 0.08);
      background: rgba(255, 255, 255, 0.03);
      color: var(--text-secondary);
    }
    .pill.status-open {
      background: color-mix(in srgb, #3b82f6 16%, transparent);
      color: #60a5fa;
      border-color: transparent;
    }
    .pill.status-pending {
      background: color-mix(in srgb, var(--color-warning) 16%, transparent);
      color: var(--color-warning);
      border-color: transparent;
    }
    .pill.status-closed {
      background: color-mix(in srgb, var(--color-success) 16%, transparent);
      color: var(--color-success);
      border-color: transparent;
    }
    .pill.priority-urgent {
      background: color-mix(in srgb, #ef4444 16%, transparent);
      color: #f87171;
      border-color: transparent;
    }
    .pill.priority-high {
      background: color-mix(in srgb, var(--color-warning) 14%, transparent);
      color: var(--color-warning);
      border-color: transparent;
    }
    .card-bottom {
      margin-top: 0.55rem;
      display: flex;
      align-items: center;
      justify-content: space-between;
      color: var(--text-secondary);
      font-size: 0.82rem;
      gap: 1rem;
    }
    .count {
      display: inline-flex;
      align-items: center;
      gap: 0.3rem;
      border: 1px solid rgba(255, 255, 255, 0.08);
      padding: 0.2rem 0.45rem;
      border-radius: 999px;
      font-weight: 650;
    }
    .footer {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 0.75rem;
      padding-top: 0.25rem;
      color: var(--text-secondary);
    }
    .foot-note { font-size: 0.85rem; }

    .btn {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 0.4rem;
      padding: 0.55rem 0.95rem;
      border-radius: 8px;
      font-weight: 650;
      font-size: 0.88rem;
      cursor: pointer;
      border: none;
      min-height: 40px;
      color: var(--text-primary);
      background: rgba(255, 255, 255, 0.04);
    }
    .btn-primary {
      background: var(--color-primary);
      color: #fff;
    }
    .btn-secondary {
      background: rgba(255, 255, 255, 0.06);
      border: 1px solid rgba(255, 255, 255, 0.08);
    }
    .btn:disabled {
      opacity: 0.55;
      cursor: not-allowed;
    }

    .modal-body { display: grid; gap: 1rem; }
    .textarea-group { display: flex; flex-direction: column; gap: 0.4rem; }
    .label {
      font-size: 0.85rem;
      font-weight: 600;
      color: var(--text-secondary);
    }
    .textarea {
      width: 100%;
      background: var(--bg-surface);
      border: 1px solid var(--border-color);
      color: var(--text-primary);
      border-radius: var(--radius-md, 8px);
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
    .file-group { display: flex; flex-direction: column; gap: 0.4rem; }
    .file-input-row { display: flex; align-items: center; gap: 0.75rem; }
    .btn-file {
      display: inline-flex;
      align-items: center;
      gap: 0.35rem;
      background: transparent;
      border: 1px dashed rgba(255, 255, 255, 0.12);
      color: var(--text-secondary);
      padding: 0.5rem 0.75rem;
      border-radius: 8px;
      cursor: pointer;
      font-size: 0.85rem;
    }
    .btn-file:hover {
      border-color: var(--color-primary);
      color: var(--color-primary);
    }
    .file-hidden {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
    }
    .file-count { font-size: 0.8rem; color: var(--text-secondary); }
    .file-list { display: flex; flex-wrap: wrap; gap: 0.4rem; }
    .file-chip {
      display: inline-flex;
      align-items: center;
      gap: 0.35rem;
      padding: 0.3rem 0.5rem;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 999px;
      background: rgba(255, 255, 255, 0.03);
      color: var(--text-secondary);
      font-size: 0.8rem;
    }
    .file-chip .file-name {
      max-width: 160px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .file-remove {
      display: grid;
      place-items: center;
      width: 18px;
      height: 18px;
      border-radius: 50%;
      border: none;
      background: rgba(255, 255, 255, 0.08);
      color: var(--text-secondary);
      cursor: pointer;
    }
    .file-remove:hover {
      background: rgba(239, 68, 68, 0.2);
      color: #ef4444;
    }
    .quick-actions {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      flex-wrap: wrap;
      padding-bottom: 0.4rem;
      margin-bottom: 0.2rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    }
    .quick-label {
      font-size: 0.75rem;
      font-weight: 650;
      color: var(--text-secondary);
      text-transform: uppercase;
      letter-spacing: 0.03em;
    }
    .quick-chip {
      display: inline-flex;
      align-items: center;
      gap: 0.35rem;
      padding: 0.35rem 0.65rem;
      border-radius: 999px;
      font-size: 0.82rem;
      font-weight: 600;
      border: 1px solid rgba(255, 255, 255, 0.08);
      background: transparent;
      color: var(--text-secondary);
      cursor: pointer;
    }
    .quick-chip:hover,
    .quick-chip.active {
      border-color: var(--color-primary);
      color: var(--color-primary);
    }
    .quick-chip.active {
      background: color-mix(in srgb, var(--color-primary) 12%, transparent);
    }
    .modal-actions {
      display: flex;
      justify-content: flex-end;
      gap: 0.6rem;
    }

    @media (max-width: 900px) {
      .kpis { grid-template-columns: 1fr 1fr; }
      .head-actions { width: 100%; }
      .search { min-width: 100%; flex: 1; }
    }
    @media (max-width: 560px) {
      .page { padding: 0.75rem; }
      .card-top, .card-bottom {
        flex-direction: column;
        align-items: flex-start;
        gap: 0.5rem;
      }
      .meta { justify-content: flex-start; }
      .modal-actions { flex-direction: column-reverse; }
      .modal-actions .btn,
      .modal-actions .btn-primary { width: 100%; }
    }
  </style>
