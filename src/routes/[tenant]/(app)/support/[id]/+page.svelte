<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { api } from '$lib/api/client';
  import type { SupportTicketDetail, SupportTicketMessage } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toast } from '$lib/stores/toast';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import Lightbox from '$lib/components/ui/Lightbox.svelte';

  let detail = $state<SupportTicketDetail | null>(null);
  let loading = $state(true);
  let sending = $state(false);
  let message = $state('');
  let attachments = $state<File[]>([]);
  let lightboxOpen = $state(false);
  let lightboxFiles = $state<any[]>([]);
  let lightboxIndex = $state(0);

  const id = $derived($page.params.id || '');
  const isClosed = $derived(detail?.ticket?.status === 'closed');

  function goBack() {
    const parts = $page.url.pathname.split('/').filter(Boolean);
    const target = '/' + parts.slice(0, -1).join('/');
    goto(target || '/');
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
    lightboxIndex = Math.max(
      0,
      (lightboxFiles || []).findIndex((f: any) => f?.id === file?.id),
    );
    lightboxOpen = true;
  }

  async function sendReply() {
    if (isClosed) {
      toast.error(get(t)('support.toasts.ticket_closed') || 'Ticket is closed');
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
      toast.success(get(t)('support.toasts.replied') || 'Reply sent');
      await load();
    } catch (e: any) {
      toast.error(
        get(t)('support.toasts.reply_failed', { values: { message: e?.message || e } }) ||
          `Reply failed: ${e?.message || e}`,
      );
    } finally {
      sending = false;
    }
  }
</script>

<div class="page-content fade-in">
  <div class="head">
    <button class="btn" type="button" onclick={goBack}>
      <Icon name="arrow-left" size={16} />
      {$t('common.back') || 'Back'}
    </button>

    <button class="btn" type="button" onclick={load} title={$t('common.refresh') || 'Refresh'}>
      <Icon name="refresh-cw" size={16} />
      {$t('common.refresh') || 'Refresh'}
    </button>
  </div>

  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <p>{$t('support.loading_detail') || 'Loading ticket...'}</p>
    </div>
  {:else if detail}
    <div class="layout">
      <aside class="side">
        <div class="ticket-card">
          <div class="title-row">
            <div class="title">{detail.ticket.subject}</div>
            <div class="meta">
              <span class="badge status {detail.ticket.status}">
                {$t(`support.status.${detail.ticket.status}`) || detail.ticket.status}
              </span>
              <span class="badge priority {detail.ticket.priority}">
                {$t(`support.priorities.${detail.ticket.priority}`) || detail.ticket.priority}
              </span>
            </div>
          </div>
          <div class="subrow">
            <span class="ticket-id mono">#{detail.ticket.id.slice(0, 8)}</span>
            <span class="dot"></span>
            <span>
              {$t('support.detail.updated') || 'Updated'}:
              {formatDateTime(detail.ticket.updated_at, { timeZone: $appSettings.app_timezone })}
            </span>
          </div>
        </div>

        <div class="reply">
          <div class="reply-head">
            <div class="reply-title">{$t('support.fields.reply') || 'Reply'}</div>
            {#if isClosed}
              <span class="badge status closed">
                {$t('support.status.closed') || 'Closed'}
              </span>
            {/if}
          </div>

          {#if isClosed}
            <div class="closed-note">
              <Icon name="lock" size={16} />
              <span>
                {$t('support.detail.closed_notice') || 'This ticket is closed. You can’t reply.'}
              </span>
            </div>
          {/if}

          <textarea
            id="support-reply"
            class="textarea"
            rows="4"
            bind:value={message}
            placeholder={$t('support.fields.reply_placeholder') || 'Write your reply...'}
            disabled={isClosed}
          ></textarea>
          <div class="file-row">
            <label class="file-label" for="support-reply-files">
              {$t('support.fields.attachments') || 'Attachments'}
            </label>
            <input
              id="support-reply-files"
              class="file"
              type="file"
              multiple
              onchange={onPickFiles}
              disabled={isClosed}
            />
            {#if attachments.length}
              <div class="file-picked">
                {#each attachments as f (f.name)}
                  <span class="picked">{f.name}</span>
                {/each}
              </div>
            {/if}
          </div>
          <div class="reply-actions">
            <button
              class="btn-primary"
              type="button"
              onclick={sendReply}
              disabled={sending || isClosed}
            >
              <Icon name="send" size={16} />
              {sending
                ? $t('support.actions.sending') || 'Sending...'
                : $t('support.actions.send') || 'Send'}
            </button>
          </div>
        </div>
      </aside>

      <section class="main">
        <div class="thread-card">
          <div class="thread-head">
            <div class="thread-title">
              {$t('support.detail.thread') || 'Conversation'}
            </div>
            <div class="thread-sub">
              {detail.messages.length} {$t('support.detail.messages') || 'messages'}
            </div>
          </div>

          <div class="chat">
            {#each detail.messages as m (m.id)}
              {@const mine = !!detail.ticket.created_by && m.author_id === detail.ticket.created_by}
              {@const label = mine
                ? $t('common.you') || 'You'
                : $t('support.labels.support') || 'Support'}
              <div class="msg" class:mine={mine}>
                <div class="msg-top">
                  <div class="avatar" class:mine={mine}>
                    <Icon name={mine ? 'user' : 'headphones'} size={14} />
                  </div>
                  <span class="who">{label}</span>
                  <span class="dot"></span>
                  <span class="time">
                    {formatDateTime(m.created_at, { timeZone: $appSettings.app_timezone })}
                  </span>
                </div>
                <div class="bubble" class:mine={mine}>
                  <div class="msg-body">{m.body}</div>
                  {#if (m.attachments || []).length}
                    <div class="attachments">
                      {#each m.attachments as f (f.id)}
                        <button
                          class="file-chip"
                          type="button"
                          onclick={() => openAttachment(m.attachments, f)}
                        >
                          <Icon name="paperclip" size={14} />
                          <span class="file-name">{f.original_name}</span>
                        </button>
                      {/each}
                    </div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>
      </section>
    </div>
  {:else}
    <div class="empty">
      <Icon name="alert-circle" size={24} />
      <p>{$t('support.detail.not_found') || 'Ticket not found'}</p>
    </div>
  {/if}
</div>

{#if lightboxOpen}
  <Lightbox
    bind:index={lightboxIndex}
    files={lightboxFiles}
    onclose={() => (lightboxOpen = false)}
  />
{/if}

<style>
  .page-content {
    padding: 1.5rem;
    max-width: 1000px;
    margin: 0 auto;
  }

  .head {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 1rem;
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
    font-weight: 700;
  }

  .btn-primary {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--color-primary);
    color: white;
    border: none;
    padding: 0.65rem 1rem;
    border-radius: 10px;
    cursor: pointer;
    font-weight: 800;
  }

  .btn-primary:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  .ticket-card {
    border: 1px solid var(--border-color);
    background:
      radial-gradient(900px 180px at 18% 0%, rgba(99, 102, 241, 0.18), transparent 55%),
      radial-gradient(700px 160px at 85% 0%, rgba(16, 185, 129, 0.1), transparent 45%),
      var(--bg-surface);
    border-radius: var(--radius-lg);
    padding: 1rem;
    box-shadow: var(--shadow-md);
  }

  .title-row {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
  }

  .title {
    font-size: 1.1rem;
    font-weight: 900;
    color: var(--text-primary);
    line-height: 1.2;
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

  .subrow {
    margin-top: 0.6rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: 0.55rem;
  }

  .layout {
    margin-top: 1rem;
    display: grid;
    grid-template-columns: minmax(320px, 380px) 1fr;
    gap: 1rem;
    align-items: start;
  }

  @media (max-width: 900px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }

  .side {
    display: grid;
    gap: 1rem;
    position: sticky;
    top: 1rem;
    align-self: start;
  }

  @media (max-width: 900px) {
    .side {
      position: static;
    }
  }

  .main {
    min-width: 0;
  }

  .thread-card {
    border: 1px solid var(--border-color);
    background:
      radial-gradient(900px 180px at 18% 0%, rgba(59, 130, 246, 0.12), transparent 55%),
      radial-gradient(700px 160px at 85% 0%, rgba(236, 72, 153, 0.08), transparent 45%),
      var(--bg-surface);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-md);
    overflow: hidden;
  }

  .thread-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.95rem 1rem;
    border-bottom: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.02);
  }

  .thread-title {
    font-weight: 900;
    color: var(--text-primary);
  }

  .thread-sub {
    color: var(--text-secondary);
    font-weight: 800;
    font-size: 0.85rem;
  }

  .chat {
    display: grid;
    gap: 0.75rem;
    padding: 1rem;
  }

  .msg {
    display: grid;
    gap: 0.35rem;
    justify-items: start;
  }

  .msg.mine {
    justify-items: end;
  }

  .msg-top {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 800;
  }

  .avatar {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.04);
    color: rgba(255, 255, 255, 0.85);
  }

  .avatar.mine {
    border-color: rgba(99, 102, 241, 0.28);
    background: rgba(99, 102, 241, 0.12);
    color: rgba(99, 102, 241, 0.95);
  }

  .who {
    font-weight: 900;
    color: var(--text-primary);
  }

  .bubble {
    border: 1px solid var(--border-color);
    background:
      radial-gradient(650px 140px at 20% 0%, rgba(255, 255, 255, 0.06), transparent 55%),
      rgba(255, 255, 255, 0.02);
    border-radius: 16px;
    padding: 0.85rem 0.95rem;
    box-shadow: var(--shadow-sm);
    max-width: min(720px, 100%);
  }

  .bubble.mine {
    border-color: rgba(99, 102, 241, 0.28);
    background:
      radial-gradient(650px 140px at 20% 0%, rgba(99, 102, 241, 0.18), transparent 55%),
      rgba(99, 102, 241, 0.07);
  }

  .msg-body {
    white-space: pre-wrap;
    color: var(--text-primary);
    line-height: 1.55;
  }

  .attachments {
    margin-top: 0.65rem;
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .file-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-secondary);
    padding: 0.35rem 0.6rem;
    border-radius: 999px;
    cursor: pointer;
    max-width: 100%;
  }

  .file-chip:hover {
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--color-primary);
  }

  .file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 260px;
  }

  .reply {
    border: 1px solid var(--border-color);
    background:
      radial-gradient(900px 180px at 20% 0%, rgba(99, 102, 241, 0.14), transparent 55%),
      var(--bg-surface);
    border-radius: var(--radius-lg);
    padding: 1rem;
    display: grid;
    gap: 0.5rem;
    box-shadow: var(--shadow-md);
  }

  .reply-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .reply-title {
    font-weight: 900;
    color: var(--text-primary);
  }

  .label {
    font-size: 0.85rem;
    font-weight: 700;
    color: var(--text-secondary);
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
    min-height: 90px;
  }

  .textarea:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .file-row {
    display: grid;
    gap: 0.35rem;
  }

  .file-label {
    font-size: 0.85rem;
    font-weight: 700;
    color: var(--text-secondary);
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

  .file-picked {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
  }

  .picked {
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    padding: 0.2rem 0.45rem;
    border-radius: 999px;
  }

  .reply-actions {
    display: flex;
    justify-content: flex-end;
  }

  .closed-note {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.55rem 0.7rem;
    border-radius: 12px;
    border: 1px solid rgba(245, 158, 11, 0.28);
    background: rgba(245, 158, 11, 0.08);
    color: rgba(245, 158, 11, 0.95);
    font-weight: 800;
    font-size: 0.9rem;
  }

  .mono {
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
  }

  .dot {
    width: 4px;
    height: 4px;
    border-radius: 999px;
    background: var(--border-color);
  }

  .ticket-id {
    color: rgba(255, 255, 255, 0.8);
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.03);
    padding: 0.1rem 0.45rem;
    border-radius: 999px;
    font-weight: 900;
    letter-spacing: 0.02em;
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

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .empty {
    border: 1px dashed var(--border-color);
    border-radius: var(--radius-lg);
    padding: 2rem 1.5rem;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
</style>
