<script lang="ts">
  /*
    Detail tiket portal v2 — gelombang 25a.
    Versi lama: (app)/support/[id]/+page.svelte (830 baris).
    Perilaku identik: chat tiket + realtime reload + reply + lampiran + lightbox.
    Pola DS: PortalShell + Badge + Button + Card + helper supportTicketInsights.
  */
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { SupportTicketDetail, SupportTicketMessage } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
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
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import { t } from 'svelte-i18n';
  import Card from '$lib/components/ds/Card.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Button from '$lib/components/ds/Button.svelte';

  const id = $derived($page.params.id || '');

  let detail = $state<SupportTicketDetail | null>(null);
  let loading = $state(true);
  let sending = $state(false);
  let message = $state('');
  let attachments = $state<File[]>([]);

  const ticket = $derived(detail?.ticket || null);
  const messages = $derived(detail?.messages || []);
  const isClosed = $derived(ticket?.status === 'closed');

  let lightboxOpen = $state(false);
  let lightboxFiles = $state<any[]>([]);
  let lightboxIndex = $state(0);
  let LightboxComponent = $state<any>(null);

  $effect(() => {
    if (!lightboxOpen) return;
    void loadLightboxModule().then(({ LightboxComponent: Lightbox }) => {
      LightboxComponent = Lightbox;
    });
  });

  function goBack() {
    goto('/v2/support');
  }

  onMount(() => {
    const onRealtime = (e: Event) => {
      const ce = e as CustomEvent<{ ticket_id: string }>;
      if (ce?.detail?.ticket_id && ce.detail.ticket_id === id) {
        void load();
      }
    };
    window.addEventListener('support_ticket_message', onRealtime as any);

    if (!$can('read', 'support') && !$can('read_all', 'support')) {
      goto('/unauthorized');
      return () => window.removeEventListener('support_ticket_message', onRealtime as any);
    }
    void load();

    return () => {
      window.removeEventListener('support_ticket_message', onRealtime as any);
    };
  });

  async function load() {
    loading = true;
    try {
      if (!id) return;
      detail = await api.support.get(id);
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  function onPickFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    attachments = Array.from(input.files || []);
  }

  function openAttachment(files: any[], file: any) {
    lightboxFiles = files || [];
    lightboxIndex = Math.max(0, (lightboxFiles || []).findIndex((f: any) => f?.id === file?.id));
    lightboxOpen = true;
  }

  async function sendReply() {
    if (isClosed) {
      toast.error('Tiket sudah ditutup.');
      return;
    }
    if (!message.trim()) return;
    sending = true;
    try {
      if (!id) return;
      const ids: string[] = [];
      for (const f of attachments) {
        const record = await api.storage.uploadFile(f);
        ids.push(record.id);
      }
      const msg: SupportTicketMessage = await api.support.reply(id, message, false, ids);
      detail = detail
        ? { ...detail, messages: [...detail.messages, msg], ticket: { ...detail.ticket } }
        : detail;
      message = '';
      attachments = [];
      toast.success('Balasan terkirim.');
      await load();
    } catch (e: any) {
      toast.error(`Gagal mengirim: ${e?.message || e}`);
    } finally {
      sending = false;
    }
  }
</script>

<PortalShell title="Detail tiket">
  <div class="mb-4 flex flex-wrap items-center gap-3">
    <Button variant="ghost" size="sm" icon="chevronLeft" onclick={goBack}>{ $t('components.detail_header.back') }</Button>
    <span class="font-mono text-xs text-ink-400">#{id.slice(0, 8)}</span>
    <div class="ml-auto">
      <Button variant="ghost" size="sm" icon="refresh" onclick={load}>{ $t('common.refresh') }</Button>
    </div>
  </div>

  {#if loading}
    <Card title="Memuat…"><p class="text-sm text-ink-500">Mengambil detail tiket…</p></Card>
  {:else if detail}
    {@const d = detail}
    <Card title={d.ticket.subject}>
      {#snippet aside()}
        <Badge tone={ticketStatusTone(d.ticket.status)} label={ticketStatusLabel(d.ticket.status, $t)} />
      {/snippet}
      <div class="mb-3 flex flex-wrap items-center gap-1.5">
        <Badge tone={ticketPriorityTone(d.ticket.priority)} label={ticketPriorityLabel(d.ticket.priority, $t)} />
        {#if d.ticket.category}
          <Badge tone="neutral" label={ticketCategoryLabel(d.ticket.category, $t)} />
        {/if}
        <span class="ml-auto text-xs text-ink-400">
          Diperbarui {formatDateTime(d.ticket.updated_at, { timeZone: $appSettings.app_timezone })}
        </span>
      </div>

      <div class="thread">
        {#each d.messages as m (m.id)}
          {@const tk = detail}
          {@const mine = isCustomerMessage(tk?.ticket.created_by, m.author_id)}
          <div class="msg {mine ? 'msg-mine' : ''}">
            <div class="mb-1 flex items-center gap-2 text-xs text-ink-400">
              <span class="font-medium text-ink-600">{messageAuthorName({ authorName: m.author_name, isCustomer: mine })}</span>
              <span>·</span>
              <span>{formatDateTime(m.created_at, { timeZone: $appSettings.app_timezone })}</span>
            </div>
            <div class="bubble {mine ? 'bubble-mine' : ''}">
              <div class="whitespace-pre-wrap text-sm">{m.body}</div>
              {#if (m.attachments || []).length}
                <div class="mt-2 flex flex-wrap gap-2">
                  {#each m.attachments as f (f.id)}
                    <button
                      type="button"
                      class="file-chip"
                      onclick={() => openAttachment(m.attachments, f)}
                    >
                      {f.original_name}
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        {/each}
      </div>

      {#if isClosed}
        <div class="mt-4 flex items-center gap-2 rounded-lg border border-ink-200 bg-ink-50 px-3 py-2.5 text-sm text-ink-600">
          <Badge tone="positive" label="Selesai" />
          Tiket sudah ditutup.
          {#if d.ticket.satisfaction_rating}
            <span class="ml-auto text-xs text-ink-400">
              Penilaian: {d.ticket.satisfaction_rating}/5
            </span>
          {/if}
        </div>
      {:else}
        <div class="mt-4 flex flex-col gap-3">
          <textarea
            class="reply-box"
            rows="3"
            value={message}
            oninput={(e) => (message = (e.currentTarget as HTMLTextAreaElement).value)}
            placeholder="Tulis balasan…"
          ></textarea>
          <div class="flex flex-wrap items-center gap-3">
            <label class="text-xs text-ink-500">
              <input
                type="file"
                multiple
                class="hidden"
                onchange={onPickFiles}
              />
              <span class="cursor-pointer rounded-md border border-ink-200 bg-white px-2.5 py-1 hover:bg-ink-50">
                Lampirkan file
              </span>
            </label>
            {#if attachments.length}
              <span class="text-xs text-ink-400">{attachments.map((f) => f.name).join(', ')}</span>
            {/if}
            <div class="ml-auto">
              <Button icon="zap" loading={sending} disabled={sending || !message.trim()} onclick={sendReply}>
                Kirim
              </Button>
            </div>
          </div>
        </div>
      {/if}
    </Card>
  {/if}

  {#if LightboxComponent && lightboxOpen}
    <LightboxComponent bind:open={lightboxOpen} files={lightboxFiles} index={lightboxIndex} />
  {/if}
</PortalShell>

<style>
  .thread {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    max-height: 420px;
    overflow-y: auto;
    padding-right: 0.25rem;
  }
  .msg {
    max-width: 78%;
  }
  .msg-mine {
    align-self: flex-end;
    text-align: right;
  }
  .msg-mine .mb-1 {
    flex-direction: row-reverse;
  }
  .bubble {
    border-radius: 0.75rem;
    background: #fff;
    border: 1px solid var(--ink-200, #e7e5e4);
    padding: 0.6rem 0.8rem;
    text-align: left;
  }
  .bubble-mine {
    background: var(--ink-900, #1c1917);
    border-color: var(--ink-900, #1c1917);
    color: #fff;
  }
  .file-chip {
    border-radius: 0.4rem;
    border: 1px solid rgba(255, 255, 255, 0.25);
    background: rgba(255, 255, 255, 0.1);
    padding: 0.2rem 0.55rem;
    font-size: 0.7rem;
    color: inherit;
    cursor: pointer;
  }
  .msg:not(.msg-mine) .file-chip {
    border-color: var(--ink-200, #e7e5e4);
    background: var(--ink-50, #fafaf9);
    color: var(--ink-600, #57534e);
  }
  .reply-box {
    width: 100%;
    border-radius: 0.6rem;
    border: 1px solid var(--ink-200, #e7e5e4);
    background: #fff;
    padding: 0.6rem 0.8rem;
    font-size: 0.875rem;
    resize: vertical;
  }
  .reply-box:focus {
    outline: 2px solid var(--ink-900, #1c1917);
    outline-offset: -1px;
  }
</style>
