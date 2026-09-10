<script lang="ts">
  /*
    Detail tiket dukungan v2 — gelombang 23.

    Versi lama: (app)/admin/support/[id]/+page.svelte (1.068 baris).
    Backend http/support.rs sudah konsisten (AppError + guard izin
    read/update/reply/assign per-permission, reply tolak kosong 400,
    claim guard row-level 409 jujur) — wave ini murni redesign FE.

    Alur dipertahankan identik dengan legacy:
    - realtime: event window 'support_ticket_message' utk ticket ini
      memicu reload otomatis.
    - balasan: upload lampiran (storage) dulu, baru reply.
    - dialog lampiran (lightbox) tetap lazy module dari ui/lightboxModule.
    - label/tone status-prioritas & predikat pesan customer/staf kini
      dari helper murni supportTicketInsights (10 tes).
  */
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { SupportTicketDetail, SupportTicketMessage, TeamMember } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { resolveBackTarget } from '$lib/utils/backNavigation';
  import {
    isCustomerMessage,
    messageAuthorName,
    ticketCategoryLabel,
    ticketPriorityLabel,
    ticketPriorityTone,
    ticketStatusLabel,
    ticketStatusTone,
  } from '$lib/utils/supportTicketInsights';
  import { loadLightboxModule } from '$lib/components/ui/lightboxModule';
  import { t } from 'svelte-i18n';
  import {
    AppShell,
    Badge,
    Button,
    Card,
    DetailHeader,
    Field,
    Icon,
    type Column,
  } from '$lib/components/ds';

  type TicketStatus = 'open' | 'pending' | 'closed' | 'resolved';
  type TicketPriority = 'low' | 'normal' | 'high' | 'urgent';
  type TicketCategory = 'general' | 'billing' | 'technical' | 'installation';

  const id = $derived($page.params.id || '');
  const listPath = $derived($page.url.pathname.replace(/\/[^/]+\/?$/, ''));
  const backTarget = $derived(resolveBackTarget($page.url, listPath));

  const canRead = $derived($can('read', 'support') || $can('read_all', 'support'));
  const canManage = $derived($can('manage', 'support'));
  const canChangeAssignee = $derived($can('assign', 'support'));
  const canInternal = $derived($can('internal', 'support'));

  let loading = $state(true);
  let saving = $state(false);
  let sending = $state(false);
  let claiming = $state(false);
  const supportBusy = $derived(saving || sending || claiming);

  let detail = $state<SupportTicketDetail | null>(null);
  const ticket = $derived(detail?.ticket || null);
  const isClosed = $derived(ticket?.status === 'closed');
  const messages = $derived(detail?.messages || []);
  const createdBy = $derived(ticket?.created_by || null);

  let status = $state<TicketStatus>('open');
  let priority = $state<TicketPriority>('normal');
  let category = $state<TicketCategory | ''>('');
  let assignedTo = $state<string>('');

  let teamMembers = $state<TeamMember[]>([]);
  const memberOptions = $derived([
    { value: '', label: '—' },
    ...teamMembers.map((m) => ({ value: m.user_id, label: `${m.name} (${m.role_name ?? m.role})` })),
  ]);

  const statusOptions = $derived([
    { value: 'open', label: $t('support.v2.st_open') },
    { value: 'pending', label: $t('support.v2.st_pending') },
    { value: 'closed', label: $t('support.v2.st_closed') },
    { value: 'resolved', label: $t('support.v2.st_resolved') },
  ]);
  const priorityOptions = $derived([
    { value: 'low', label: $t('support.v2.p_low') },
    { value: 'normal', label: $t('support.v2.p_normal') },
    { value: 'high', label: $t('support.v2.p_high') },
    { value: 'urgent', label: $t('support.v2.p_urgent2') },
  ]);
  const categoryOptions = $derived([
    { value: '', label: '—' },
    { value: 'general', label: $t('support.categories.general') },
    { value: 'billing', label: $t('support.categories.billing') },
    { value: 'technical', label: $t('support.categories.technical') },
    { value: 'installation', label: $t('support.categories.installation') },
  ]);

  let reply = $state('');
  let internalNote = $state(false);
  let attachments = $state<File[]>([]);
  let lightboxOpen = $state(false);
  let lightboxFiles = $state<any[]>([]);
  let lightboxIndex = $state(0);
  let LightboxComponent = $state<any>(null);

  onMount(() => {
    const onRealtime = (e: Event) => {
      const ce = e as CustomEvent<{ ticket_id: string }>;
      if (ce?.detail?.ticket_id && ce.detail.ticket_id === id) {
        void load();
      }
    };
    window.addEventListener('support_ticket_message', onRealtime as any);

    if (!canRead) {
      goto('/unauthorized');
      return () => window.removeEventListener('support_ticket_message', onRealtime as any);
    }
    void Promise.all([load(), loadTeam()]);
    return () => window.removeEventListener('support_ticket_message', onRealtime as any);
  });

  async function loadTeam() {
    try {
      teamMembers = await api.support.listAssignees();
    } catch {
      /* assignee hanya opsi; gagal muat tidak menggagalkan halaman */
    }
  }

  async function load() {
    loading = true;
    try {
      if (!id) return;
      detail = await api.support.get(id);
      status = (ticket?.status as TicketStatus) || 'open';
      priority = (ticket?.priority as TicketPriority) || 'normal';
      category = (ticket?.category as TicketCategory) || '';
      assignedTo = ticket?.assigned_to || '';
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      loading = false;
    }
  }

  async function saveTicket() {
    if (!detail || supportBusy) return;
    saving = true;
    try {
      if (!id) return;
      const updated = await api.support.update(id, {
        status,
        priority,
        category: category || undefined,
        assignedTo: assignedTo || null,
      });
      detail = { ...detail, ticket: updated as any };
      toast.success($t('support.v2.t_saved'));
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      saving = false;
    }
  }

  async function claimTicket() {
    if (!detail?.ticket || supportBusy) return;
    claiming = true;
    try {
      const updated = await api.support.claim(detail.ticket.id);
      detail = { ...detail, ticket: updated as any };
      assignedTo = updated.assigned_to || '';
      toast.success($t('support.v2.t_claimed'));
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      claiming = false;
    }
  }

  function onPickFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    attachments = Array.from(input.files || []);
  }

  async function sendReply() {
    if (supportBusy) return;
    if (isClosed) {
      toast.error($t('support.toasts.ticket_closed'));
      return;
    }
    if (!reply.trim()) return;
    sending = true;
    try {
      if (!id) return;
      const ids: string[] = [];
      for (const f of attachments) {
        const record = await api.storage.uploadFile(f);
        ids.push(record.id);
      }
      const msg: SupportTicketMessage = await api.support.reply(id, reply, internalNote, ids);
      if (detail) detail = { ...detail, messages: [...detail.messages, msg] };
      reply = '';
      internalNote = false;
      attachments = [];
      toast.success($t('support.toasts.replied'));
      await load();
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      sending = false;
    }
  }

  function openAttachment(files: any[], file: any) {
    lightboxFiles = files || [];
    lightboxIndex = Math.max(
      0,
      (lightboxFiles || []).findIndex((f: any) => f?.id === file?.id),
    );
    lightboxOpen = true;
    if (!LightboxComponent) {
      void loadLightboxModule().then(({ LightboxComponent: Lb }) => {
        LightboxComponent = Lb;
      });
    }
  }

  async function openSubscription(subscriptionId: string | null) {
    if (!subscriptionId) return;
    try {
      const sub = await api.customers.subscriptions.get(subscriptionId);
      if (sub?.customer_id) {
        goto(`/v2/admin/customers/${sub.customer_id}`);
      }
    } catch {
      toast.error($t('support.toasts.subscription_load_failed'));
    }
  }

  function initials(name: string): string {
    return name
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((s) => s[0]?.toUpperCase() || '')
      .join('');
  }
</script>
<AppShell>
  {#if loading && !detail}
    <div class="py-16 text-center text-ink-500">{ $t('support.loading_detail') }</div>
  {:else if !detail}
    <div class="py-16 text-center">
      <div class="text-base font-medium text-ink-900">{ $t('support.detail.not_found') }</div>
      <Button variant="ghost" class="mt-3" href={backTarget}>{ $t('components.detail_header.back') }</Button>
    </div>
  {:else if ticket}
    <DetailHeader
      title={ticket.subject}
      subtitle={`#${ticket.id.slice(0, 8)} · ${$t('support.v2.created')} ${formatDateTime(ticket.created_at, { timeZone: $appSettings.app_timezone })}`}
      status={ticket.status}
      statusTone={ticketStatusTone(ticket.status)}
      statusLabel={ticketStatusLabel(ticket.status, $t)}
      backHref={backTarget}
      meta={[
        { label: $t('support.fields.priority'), value: ticketPriorityLabel(ticket.priority, $t) },
        { label: $t('support.fields.category'), value: ticketCategoryLabel(ticket.category, $t) },
        { label: $t('support.detail.updated'), value: formatDateTime(ticket.updated_at, { timeZone: $appSettings.app_timezone }) },
        ...(ticket.assigned_to ? [{ label: $t('support.v2.assigned_to'), value: $t('support.labels.staff') }] : []),
      ]}
    >
      {#snippet actions()}
        <Button variant="ghost" icon="refresh" onclick={() => void load()} disabled={loading || supportBusy}>
          { $t('support.v2.refresh') }
        </Button>
        <Button variant="primary" icon="check" onclick={() => void saveTicket()} disabled={saving || loading || supportBusy}>
          {saving ? 'Menyimpan…' : $t('common.save')}
        </Button>
      {/snippet}
    </DetailHeader>

    {#if ticket.subscription_id}
      <button
        type="button"
        class="focus-ring mt-3 inline-flex items-center gap-1.5 rounded-lg bg-white px-3 py-1.5 text-sm text-ink-700 ring-1 ring-ink-200 hover:bg-ink-50"
        onclick={() => void openSubscription(ticket.subscription_id)}
      >
        <Icon name="inbox" size={14} />
        Lihat langganan terkait: <code class="font-mono">{ticket.subscription_id.slice(0, 8)}</code>
      </button>
    {/if}

    <div class="mt-4 grid gap-4 lg:grid-cols-[320px_minmax(0,1fr)]">
      <div class="space-y-4">
        <Card title={ $t('support.v2.manage') }>
          <div class="space-y-3">
            <Field id="t-status" label={ $t('admin.customers.columns.status') } type="select" stacked value={status} options={statusOptions} onchange={(v) => (status = v as TicketStatus)} disabled={supportBusy} />
            <Field id="t-priority" label={ $t('support.fields.priority') } type="select" stacked value={priority} options={priorityOptions} onchange={(v) => (priority = v as TicketPriority)} disabled={supportBusy} />
            <Field id="t-category" label={ $t('support.fields.category') } type="select" stacked value={category} options={categoryOptions} onchange={(v) => (category = v as TicketCategory | '')} disabled={supportBusy} />
            {#if canChangeAssignee}
              <Field id="t-assignee" label={ $t('support.v2.agent') } type="select" stacked value={assignedTo} options={memberOptions} onchange={(v) => (assignedTo = v)} disabled={supportBusy} />
            {/if}
            {#if !ticket.assigned_to && !isClosed}
              <Button variant="secondary" onclick={() => void claimTicket()} disabled={claiming || supportBusy}>
                {claiming ? 'Mengklaim…' : $t('support.v2.claim_btn')}
              </Button>
            {/if}
          </div>
        </Card>

        {#if ticket.satisfaction_rating}
          <Card title={ $t('support.v2.rating_label') }>
            <div class="flex items-center gap-2 text-sm">
              <span class="text-ink-900">{ticket.satisfaction_rating}/5</span>
              <div class="flex gap-0.5" aria-hidden="true">
                {#each [1, 2, 3, 4, 5] as star}
                  <span class="inline-block size-2.5 rounded-full {star <= (ticket?.satisfaction_rating ?? 0) ? 'bg-amber-400' : 'bg-ink-200'}"></span>
                {/each}
              </div>
            </div>
            {#if ticket.satisfaction_comment}
              <p class="mt-2 text-sm italic text-ink-600">“{ticket.satisfaction_comment}”</p>
            {/if}
          </Card>
        {/if}

        <Card title={ $t('support.fields.reply') }>
          {#if isClosed}
            <div class="flex items-center gap-2 rounded-lg bg-ink-50 px-3 py-2 text-sm text-ink-600">
              <Icon name="lock" size={14} />
              Tiket ditutup — balasan nonaktif.
            </div>
          {:else}
            <Field id="t-reply" label={ $t('support.v2.reply_label') } type="textarea" stacked rows={5} value={reply} onchange={(v) => (reply = v)} placeholder={ $t('support.v2.reply_ph') } />
            <div class="mt-3 flex flex-wrap items-center gap-3">
              <label class="focus-ring inline-flex min-h-[36px] cursor-pointer items-center gap-1.5 rounded-lg bg-ink-50 px-3 text-sm text-ink-700 ring-1 ring-ink-200 hover:bg-ink-100">
                <Icon name="folder" size={14} />
                Lampiran
                <input id="t-reply-files" type="file" multiple class="hidden" onchange={onPickFiles} />
              </label>
              {#if attachments.length}
                <span class="text-sm text-ink-600">{attachments.map((f) => f.name).join(', ')}</span>
              {/if}
              {#if canInternal}
                <label class="inline-flex min-h-[24px] cursor-pointer items-center gap-2 text-sm text-ink-700">
                  <input type="checkbox" class="size-4" checked={internalNote} onchange={(e) => (internalNote = (e.target as HTMLInputElement).checked)} />
                  Catatan internal
                </label>
              {/if}
              <Button variant="primary" icon="mail" onclick={() => void sendReply()} disabled={sending || supportBusy || !reply.trim()}>
                {sending ? 'Mengirim…' : $t('support.v2.send_reply')}
              </Button>
            </div>
          {/if}
        </Card>
      </div>

      <Card title={ $t('support.detail.thread') } padded={false}>
        <div class="divide-y divide-ink-100">
          {#each messages as m (m.id)}
            {@const isCustomer = isCustomerMessage(createdBy, m.author_id)}
            {@const who = messageAuthorName({ authorName: m.author_name, isCustomer })}
            <div class="px-4 py-3 {m.is_internal ? 'bg-amber-50/60' : ''}">
              <div class="flex flex-wrap items-center gap-2 text-sm">
                <span class="inline-flex size-6 items-center justify-center rounded-full bg-ink-900 text-[11px] font-medium text-white">{initials(who)}</span>
                <span class="font-medium text-ink-900">{who}</span>
                {#if m.is_internal}
                  <Badge tone="warning" label={ $t('support.tags.internal') } />
                {:else if isCustomer}
                  <Badge tone="info" label={ $t('support.labels.customer') } />
                {:else}
                  <Badge tone="neutral" label={ $t('support.labels.staff') } />
                {/if}
                <span class="text-xs text-ink-400">{formatDateTime(m.created_at, { timeZone: $appSettings.app_timezone })}</span>
              </div>
              <p class="mt-1.5 whitespace-pre-wrap text-sm leading-relaxed text-ink-800">{m.body}</p>
              {#if (m.attachments || []).length}
                <div class="mt-2 flex flex-wrap gap-2">
                  {#each m.attachments as f (f.id)}
                    <button
                      type="button"
                      class="focus-ring inline-flex items-center gap-1.5 rounded-lg bg-ink-50 px-2.5 py-1 text-xs text-ink-700 ring-1 ring-ink-200 hover:bg-ink-100"
                      onclick={() => openAttachment(m.attachments, f)}
                    >
                      <Icon name="folder" size={12} />
                      {f.original_name}
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          {:else}
            <div class="px-4 py-10 text-center text-sm text-ink-500">{ $t('support.v2.no_msgs') }</div>
          {/each}
        </div>
      </Card>
    </div>
  {/if}
</AppShell>

{#if lightboxOpen && LightboxComponent}
  <LightboxComponent bind:index={lightboxIndex} files={lightboxFiles} onclose={() => (lightboxOpen = false)} />
{/if}
