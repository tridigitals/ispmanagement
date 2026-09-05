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
    { value: 'open', label: 'Terbuka' },
    { value: 'pending', label: 'Menunggu' },
    { value: 'closed', label: 'Ditutup' },
    { value: 'resolved', label: 'Selesai' },
  ]);
  const priorityOptions = $derived([
    { value: 'low', label: 'Rendah' },
    { value: 'normal', label: 'Normal' },
    { value: 'high', label: 'Tinggi' },
    { value: 'urgent', label: 'Urgent' },
  ]);
  const categoryOptions = $derived([
    { value: '', label: '—' },
    { value: 'general', label: 'Umum' },
    { value: 'billing', label: 'Tagihan' },
    { value: 'technical', label: 'Teknis' },
    { value: 'installation', label: 'Instalasi' },
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
      toast.success('Perubahan tiket disimpan.');
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
      toast.success('Tiket diklaim.');
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
      toast.error('Tiket sudah ditutup.');
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
      toast.success('Balasan terkirim.');
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
      toast.error('Gagal memuat langganan terkait.');
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
    <div class="py-16 text-center text-ink-500">Memuat tiket…</div>
  {:else if !detail}
    <div class="py-16 text-center">
      <div class="text-base font-medium text-ink-900">Tiket tidak ditemukan.</div>
      <Button variant="ghost" class="mt-3" href={backTarget}>Kembali</Button>
    </div>
  {:else if ticket}
    <DetailHeader
      title={ticket.subject}
      subtitle={`#${ticket.id.slice(0, 8)} · dibuat ${formatDateTime(ticket.created_at, { timeZone: $appSettings.app_timezone })}`}
      status={ticket.status}
      statusTone={ticketStatusTone(ticket.status)}
      statusLabel={ticketStatusLabel(ticket.status)}
      backHref={backTarget}
      meta={[
        { label: 'Prioritas', value: ticketPriorityLabel(ticket.priority) },
        { label: 'Kategori', value: ticketCategoryLabel(ticket.category) },
        { label: 'Diperbarui', value: formatDateTime(ticket.updated_at, { timeZone: $appSettings.app_timezone }) },
        ...(ticket.assigned_to ? [{ label: 'Ditugaskan ke', value: 'Staf' }] : []),
      ]}
    >
      {#snippet actions()}
        <Button variant="ghost" icon="refresh" onclick={() => void load()} disabled={loading || supportBusy}>
          Segarkan
        </Button>
        <Button variant="primary" icon="check" onclick={() => void saveTicket()} disabled={saving || loading || supportBusy}>
          {saving ? 'Menyimpan…' : 'Simpan'}
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
        <Card title="Kelola tiket">
          <div class="space-y-3">
            <Field id="t-status" label="Status" type="select" stacked value={status} options={statusOptions} onchange={(v) => (status = v as TicketStatus)} disabled={supportBusy} />
            <Field id="t-priority" label="Prioritas" type="select" stacked value={priority} options={priorityOptions} onchange={(v) => (priority = v as TicketPriority)} disabled={supportBusy} />
            <Field id="t-category" label="Kategori" type="select" stacked value={category} options={categoryOptions} onchange={(v) => (category = v as TicketCategory | '')} disabled={supportBusy} />
            {#if canChangeAssignee}
              <Field id="t-assignee" label="Petugas" type="select" stacked value={assignedTo} options={memberOptions} onchange={(v) => (assignedTo = v)} disabled={supportBusy} />
            {/if}
            {#if !ticket.assigned_to && !isClosed}
              <Button variant="secondary" onclick={() => void claimTicket()} disabled={claiming || supportBusy}>
                {claiming ? 'Mengklaim…' : 'Klaim tiket'}
              </Button>
            {/if}
          </div>
        </Card>

        {#if ticket.satisfaction_rating}
          <Card title="Penilaian pelanggan">
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

        <Card title="Balas">
          {#if isClosed}
            <div class="flex items-center gap-2 rounded-lg bg-ink-50 px-3 py-2 text-sm text-ink-600">
              <Icon name="lock" size={14} />
              Tiket ditutup — balasan nonaktif.
            </div>
          {:else}
            <Field id="t-reply" label="Balasan" type="textarea" stacked rows={5} value={reply} onchange={(v) => (reply = v)} placeholder="Tulis balasan…" />
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
                {sending ? 'Mengirim…' : 'Kirim balasan'}
              </Button>
            </div>
          {/if}
        </Card>
      </div>

      <Card title="Percakapan" padded={false}>
        <div class="divide-y divide-ink-100">
          {#each messages as m (m.id)}
            {@const isCustomer = isCustomerMessage(createdBy, m.author_id)}
            {@const who = messageAuthorName({ authorName: m.author_name, isCustomer })}
            <div class="px-4 py-3 {m.is_internal ? 'bg-amber-50/60' : ''}">
              <div class="flex flex-wrap items-center gap-2 text-sm">
                <span class="inline-flex size-6 items-center justify-center rounded-full bg-ink-900 text-[11px] font-medium text-white">{initials(who)}</span>
                <span class="font-medium text-ink-900">{who}</span>
                {#if m.is_internal}
                  <Badge tone="warning" label="Internal" />
                {:else if isCustomer}
                  <Badge tone="info" label="Pelanggan" />
                {:else}
                  <Badge tone="neutral" label="Staf" />
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
            <div class="px-4 py-10 text-center text-sm text-ink-500">Belum ada pesan.</div>
          {/each}
        </div>
      </Card>
    </div>
  {/if}
</AppShell>

{#if lightboxOpen && LightboxComponent}
  <LightboxComponent bind:index={lightboxIndex} files={lightboxFiles} onclose={() => (lightboxOpen = false)} />
{/if}
