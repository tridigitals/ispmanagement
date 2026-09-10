<script lang="ts">
  /*
    Support portal v2 — gelombang 25a.
    Versi lama: (app)/support/+page.svelte (931 baris).
    Perilaku identik: KPI filter + cari + filter status/kategori + daftar tiket
    + modal buat tiket (quick actions + lampiran).
    Pola DS: PortalShell + PageHeader + Field + Card + Badge + Button + Modal lama.
  */
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
  import { toast } from '$lib/stores/toast';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Field from '$lib/components/ds/Field.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import type { StatusTone } from '$lib/components/ds/tokens';

  import { t } from 'svelte-i18n';
  let tickets = $state<SupportTicketListItem[]>([]);
  let stats = $state<SupportTicketStats>({ all: 0, open: 0, pending: 0, closed: 0, resolved: 0, unassigned: 0 });
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
  let priority = $state('normal');
  let category = $state('general');
  let subscriptionId = $state('');
  let subscriptions = $state<Array<{ id: string; label: string }>>([]);
  let attachments = $state<{ file: File; name: string }[]>([]);

  type QuickAction = { icon: 'alert' | 'wifi' | 'plus'; label: string; subject: string; message: string; category: string };
  const quickActions: QuickAction[] = [
    { icon: 'alert', label: $t('support.v2.qa1_l'), subject: $t('support.v2.qa1_s'), message: $t('support.v2.qa1_m'), category: 'technical' },
    { icon: 'wifi', label: $t('support.v2.qa2_l'), subject: $t('support.v2.qa2_s'), message: $t('support.v2.qa2_m'), category: 'technical' },
    { icon: 'plus', label: $t('support.v2.qa3_l'), subject: '', message: '', category: 'general' },
  ];

  let statusFilter = $state('all');
  let categoryFilter = $state('all');
  let hasMore = $derived(tickets.length < total);

  function normStatus(status: string) {
    const s = String(status || '').toLowerCase();
    if (s === 'resolved' || s === 'done' || s === 'completed') return 'closed';
    if (s === 'in_progress' || s === 'waiting') return 'pending';
    return s;
  }
  function statusTone(status: string): StatusTone {
    const s = normStatus(status);
    if (s === 'open') return 'info';
    if (s === 'pending') return 'warning';
    if (s === 'closed') return 'positive';
    return 'neutral';
  }
  function statusLabel(status: string) {
    const s = normStatus(status);
    return { open: $t('support.status.open'), pending: $t('support.status.pending'), closed: $t('support.status.closed') }[s] ?? (status || '—');
  }
  function priorityTone(p: string): StatusTone {
    if (p === 'urgent') return 'negative';
    if (p === 'high') return 'warning';
    if (p === 'low') return 'neutral';
    return 'info';
  }
  function priorityLabel(p: string) {
    return { low: $t('support.priorities.low'), normal: $t('support.priorities.normal'), high: $t('support.priorities.high'), urgent: $t('support.v2.p_urgent') }[p] ?? (p || '—');
  }
  function categoryLabel(c: string) {
    return { general: $t('support.categories.general'), billing: $t('support.categories.billing'), technical: $t('support.categories.technical'), installation: $t('support.categories.installation') }[c] ?? (c || '—');
  }

  function onPickFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    const files = Array.from(input.files || []);
    for (const file of files) {
      if (!attachments.some((a) => a.file.name === file.name && a.file.size === file.size)) {
        attachments = [...attachments, { file, name: file.name }];
      }
    }
    input.value = '';
  }
  function removeAttachment(idx: number) {
    attachments = attachments.filter((_, i) => i !== idx);
  }
  function applyQuickAction(a: QuickAction) {
    subject = a.subject;
    message = a.message;
    category = a.category;
  }

  onMount(async () => {
    if (!$can('read', 'support') && !$can('create', 'support')) {
      goto('/unauthorized');
      return;
    }
    await refreshStats();
    await loadTickets(true);
    try {
      const res = await api.customers.portal.mySubscriptions({ per_page: 50, status: 'active' });
      subscriptions = (res.data || []).map((s: any) => ({
        id: s.id,
        label: s.package_name || s.plan_name || $t('support.fields.subscription'),
      }));
    } catch (_) {
      /* ignore */
    }
  });

  $effect(() => {
    const _q = searchQuery, _s = statusFilter, _c = categoryFilter;
    const timer = setTimeout(() => void loadTickets(true), 250);
    return () => clearTimeout(timer);
  });

  async function refreshStats() {
    try {
      stats = await api.support.stats();
    } catch {
      /* non-blocking */
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
      const res = await api.support.list({
        status: statusFilter === 'all' ? undefined : (statusFilter as 'open' | 'pending' | 'closed'),
        search: searchQuery.trim() || undefined,
        category: categoryFilter === 'all' ? undefined : (categoryFilter as 'general' | 'billing' | 'technical' | 'installation'),
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
      const res = await api.support.list({
        status: statusFilter === 'all' ? undefined : (statusFilter as 'open' | 'pending' | 'closed'),
        search: searchQuery.trim() || undefined,
        category: categoryFilter === 'all' ? undefined : (categoryFilter as 'general' | 'billing' | 'technical' | 'installation'),
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
    goto(`/v2/support/${id}`);
  }
  async function submitCreate() {
    if (!subject.trim() || !message.trim()) return;
    if (subject.trim().length < 3 || message.trim().length < 10) {
      toast.error($t('support.v2.t_valid'));
      return;
    }
    creating = true;
    try {
      const ids: string[] = [];
      for (const att of attachments) {
        const record = await api.storage.uploadFile(att.file);
        ids.push(record.id);
      }
      const detail = await api.support.create(
        subject,
        message,
        priority as 'low' | 'normal' | 'high' | 'urgent',
        category as 'general' | 'billing' | 'technical' | 'installation',
        subscriptionId || undefined,
        ids,
      );
      toast.success($t('support.v2.t_created'));
      showCreate = false;
      subject = '';
      message = '';
      priority = 'normal';
      category = 'general';
      subscriptionId = '';
      attachments = [];
      await refreshStats();
      await loadTickets(true);
      goto(`/v2/support/${detail.ticket.id}`);
    } catch (e: any) {
      toast.error($t('support.v2.t_failed', { values: { message: String(e?.message || e) } }));
    } finally {
      creating = false;
    }
  }
</script>

<PortalShell title={ $t('sidebar.sections.help') }>
  <PageHeader
    title={ $t('sidebar.sections.help') }
    desc={ $t('support.v2.desc') }
  >
    {#snippet actions()}
      {#if $can('create', 'support')}
        <Button icon="plus" onclick={() => (showCreate = true)}>{ $t('support.v2.new_btn') }</Button>
      {/if}
    {/snippet}
  </PageHeader>

  {#if !loading}
    <div class="mb-5 grid grid-cols-2 gap-3 lg:grid-cols-4">
      {#each [
        { key: 'all', label: $t('support.stats.total'), val: stats.all, sub: $t('support.v2.h_all') },
        { key: 'open', label: $t('support.status.open'), val: stats.open, sub: $t('support.v2.h_need_res') },
        { key: 'pending', label: $t('support.status.pending'), val: stats.pending, sub: $t('support.v2.h_proc') },
        { key: 'closed', label: $t('support.status.closed'), val: stats.closed, sub: $t('support.v2.h_done') },
      ] as k (k.key)}
        <button
          type="button"
          class="kpi {statusFilter === k.key ? 'kpi-active' : ''}"
          onclick={() => (statusFilter = k.key)}
        >
          <span class="text-xs font-medium text-ink-400">{k.label}</span>
          <span class="text-2xl font-semibold tabular-nums text-ink-900">{k.val}</span>
          <span class="text-xs text-ink-400">{k.sub}</span>
        </button>
      {/each}
    </div>
  {/if}

  <div class="mb-4 grid gap-3 sm:grid-cols-[minmax(0,1fr)_160px_170px]">
    <Field
      id="sup-search"
      label={ $t('common.search') }
      type="text"
      value={searchQuery}
      placeholder={ $t('support.v2.search_ph') }
      onchange={(v) => (searchQuery = v)}
    />
    <Field
      id="sup-status"
      label={ $t('admin.customers.columns.status') }
      type="select"
      value={statusFilter}
      options={[
        { value: 'all', label: $t('support.categories.all') },
        { value: 'open', label: $t('support.status.open') },
        { value: 'pending', label: $t('support.status.pending') },
        { value: 'closed', label: $t('support.status.closed') },
      ]}
      onchange={(v) => (statusFilter = v)}
    />
    <Field
      id="sup-category"
      label={ $t('support.fields.category') }
      type="select"
      value={categoryFilter}
      options={[
        { value: 'all', label: $t('support.categories.all') },
        { value: 'general', label: $t('support.categories.general') },
        { value: 'billing', label: $t('support.categories.billing') },
        { value: 'technical', label: $t('support.categories.technical') },
        { value: 'installation', label: $t('support.categories.installation') },
      ]}
      onchange={(v) => (categoryFilter = v)}
    />
  </div>

  {#if loading}
    <Card title={ $t('support.loading') }><p class="text-sm text-ink-500">{ $t('support.v2.loading_fetch') }</p></Card>
  {:else if tickets.length === 0}
    <Card title={ $t('support.empty.title') }>
      <p class="mb-4 text-sm text-ink-500">{ $t('support.v2.empty_hint') }</p>
      {#if $can('create', 'support')}
        <Button icon="plus" onclick={() => (showCreate = true)}>{ $t('support.v2.new_btn') }</Button>
      {/if}
    </Card>
  {:else}
    <div class="grid gap-3">
      {#each tickets as item (item.id)}
        <button type="button" class="ticket-card" onclick={() => openTicket(item.id)}>
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-medium text-ink-900">{item.subject}</div>
            <div class="mt-1 flex flex-wrap items-center gap-1.5">
              <Badge tone={statusTone(item.status)} label={statusLabel(item.status)} />
              <Badge tone={priorityTone(item.priority)} label={priorityLabel(item.priority)} />
              {#if item.category}
                <Badge tone="neutral" label={categoryLabel(item.category)} />
              {/if}
            </div>
          </div>
          <div class="shrink-0 text-right text-xs text-ink-400">
            <div>
              {formatDateTime(item.last_message_at || item.updated_at, {
                timeZone: $appSettings.app_timezone,
              })}
            </div>
            <div class="mt-1 font-medium text-ink-500">{item.message_count} pesan</div>
          </div>
        </button>
      {/each}
    </div>
    {#if hasMore}
      <div class="mt-4 flex items-center justify-center gap-3">
        <Button variant="secondary" icon="chevronDown" disabled={loadingMore} onclick={loadMore}>
          {loadingMore ? $t('support.loading') : $t('support.v2.load_more')}
        </Button>
        <span class="text-xs text-ink-400">{tickets.length}/{total}</span>
      </div>
    {/if}
  {/if}
</PortalShell>

<Modal bind:show={showCreate} title={ $t('support.v2.new_btn') } onclose={() => (showCreate = false)}>
  <div class="flex flex-col gap-4">
    <div>
      <p class="mb-2 text-xs font-medium text-ink-500">{ $t('support.v2.quick') }</p>
      <div class="flex flex-wrap gap-2">
        {#each quickActions as a (a.label)}
          <button
            type="button"
            class="quick-chip {subject === a.subject && category === a.category ? 'quick-chip-active' : ''}"
            onclick={() => applyQuickAction(a)}
          >
            {a.label}
          </button>
        {/each}
      </div>
    </div>

    <Field id="sup-subject" label={ $t('support.fields.subject') } type="text" value={subject} placeholder={ $t('support.v2.subject_ph') } onchange={(v) => (subject = v)} />
    <Field id="sup-message" label={ $t('support.fields.message') } type="textarea" value={message} placeholder={ $t('support.v2.body_ph') } onchange={(v) => (message = v)} />
    <Field
      id="sup-category"
      label={ $t('support.fields.category') }
      type="select"
      value={category}
      options={[
        { value: 'general', label: $t('support.categories.general') },
        { value: 'billing', label: $t('support.categories.billing') },
        { value: 'technical', label: $t('support.categories.technical') },
        { value: 'installation', label: $t('support.categories.installation') },
      ]}
      onchange={(v) => (category = v)}
    />
    {#if subscriptions.length > 0}
      <Field
        id="sup-subscription"
        label={ $t('support.fields.subscription') }
        type="select"
        value={subscriptionId}
        options={[
          { value: '', label: 'Tidak terkait' },
          ...subscriptions.map((s) => ({ value: s.id, label: s.label })),
        ]}
        onchange={(v) => (subscriptionId = v)}
      />
    {/if}
    <Field
      id="sup-priority"
      label={ $t('support.fields.priority') }
      type="select"
      value={priority}
      options={[
        { value: 'low', label: $t('support.priorities.low') },
        { value: 'normal', label: $t('support.priorities.normal') },
        { value: 'high', label: $t('support.priorities.high') },
        { value: 'urgent', label: $t('support.v2.p_urgent') },
      ]}
      onchange={(v) => (priority = v)}
    />

    <div>
      <label for="sup-attachments" class="mb-2 flex cursor-pointer items-center gap-1.5 text-xs font-medium text-ink-500">
        Lampiran
        <input id="sup-attachments" type="file" multiple class="hidden" onchange={onPickFiles} />
        <span class="rounded-md border border-ink-200 bg-white px-2.5 py-1 text-xs text-ink-600 hover:bg-ink-50">
          Pilih file
        </span>
      </label>
      {#if attachments.length}
        <div class="flex flex-wrap gap-2">
          {#each attachments as att, i (att.name)}
            <span class="flex items-center gap-1.5 rounded-md bg-ink-100 px-2 py-1 text-xs text-ink-600">
              {att.name}
              <button type="button" class="text-ink-400 hover:text-ink-700" onclick={() => removeAttachment(i)} aria-label={ $t('common.delete') }>
                ×
              </button>
            </span>
          {/each}
        </div>
      {/if}
    </div>

    <div class="flex justify-end gap-2 border-t border-ink-200 pt-4">
      <Button variant="ghost" onclick={() => (showCreate = false)}>Batal</Button>
      <Button loading={creating} onclick={submitCreate} disabled={creating}>
        {creating ? 'Membuat…' : $t('support.v2.new_btn')}
      </Button>
    </div>
  </div>
</Modal>

<style>
  .kpi {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    border-radius: 0.75rem;
    border: 1px solid var(--ink-200, #e7e5e4);
    background: #fff;
    padding: 0.75rem 1rem;
    cursor: pointer;
    text-align: left;
  }
  .kpi:hover {
    border-color: #d6d3d1;
  }
  .kpi-active {
    border-color: var(--ink-900, #1c1917);
    box-shadow: inset 0 0 0 1px var(--ink-900, #1c1917);
  }
  .ticket-card {
    display: flex;
    align-items: center;
    gap: 1rem;
    width: 100%;
    border-radius: 0.75rem;
    border: 1px solid var(--ink-200, #e7e5e4);
    background: #fff;
    padding: 0.85rem 1rem;
    cursor: pointer;
    text-align: left;
  }
  .ticket-card:hover {
    border-color: #d6d3d1;
    background: #fafaf9;
  }
  .quick-chip {
    border-radius: 9999px;
    border: 1px solid var(--ink-200, #e7e5e4);
    background: #fff;
    padding: 0.35rem 0.85rem;
    font-size: 0.75rem;
    color: var(--ink-600, #57534e);
    cursor: pointer;
  }
  .quick-chip:hover {
    background: var(--ink-50, #fafaf9);
  }
  .quick-chip-active {
    border-color: var(--ink-900, #1c1917);
    background: var(--ink-900, #1c1917);
    color: #fff;
  }
</style>
