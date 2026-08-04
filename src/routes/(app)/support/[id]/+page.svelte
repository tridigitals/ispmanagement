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
  import { loadLightboxModule } from '$lib/components/ui/lightboxModule';

  let detail = $state<SupportTicketDetail | null>(null);
  let loading = $state(true);
  let sending = $state(false);
  let message = $state('');
  let attachments = $state<File[]>([]);
  let lightboxOpen = $state(false);
  let lightboxFiles = $state<any[]>([]);
  let lightboxIndex = $state(0);
  let LightboxComponent = $state<any>(null);

  const id = $derived($page.params.id || '');
  const isClosed = $derived(
    ['closed', 'resolved', 'done', 'completed'].includes(
      String(detail?.ticket?.status || '').toLowerCase()
    )
  );

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

  $effect(() => {
    if (!lightboxOpen) return;
    void loadLightboxModule().then(({ LightboxComponent: Lightbox }) => {
      LightboxComponent = Lightbox;
    });
  });

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

<div class="page">
  <div class="detail-head">
    <button class="btn btn-secondary" type="button" onclick={goBack}>
      <Icon name="arrow-left" size={16} />
      {$t('common.back')}
    </button>
    <span class="head-id mono">#{id.slice(0, 8)}</span>
    <button class="btn btn-secondary" type="button" onclick={load} title={$t('common.refresh')}>
      <Icon name="refresh-cw" size={16} />
    </button>
  </div>

  {#if loading}
    <div class="state">
      <div class="spinner"></div>
      <p>{$t('support.loading_detail')}</p>
    </div>
  {:else if detail}
    <div class="layout">
      <aside class="side">
        <div class="ticket-card panel">
          <div class="title-row">
            <div class="title">{detail.ticket.subject}</div>
            <div class="meta">
              <span class="pill status-{normStatus(detail.ticket.status)}">
                {statusLabel(detail.ticket.status)}
              </span>
              <span class="pill priority-{detail.ticket.priority}">
                {priorityLabel(detail.ticket.priority)}
              </span>
              {#if detail.ticket.category}
                <span class="pill">
                  {categoryLabel(detail.ticket.category)}
                </span>
              {/if}
            </div>
          </div>
          <div class="subrow">
            <span class="ticket-id mono">#{detail.ticket.id.slice(0, 8)}</span>
            <span class="dot"></span>
            <span>
              {$t('support.detail.updated')}:
              {formatDateTime(detail.ticket.updated_at, { timeZone: $appSettings.app_timezone })}
            </span>
          </div>
          {#if detail.ticket.subscription_id}
            <div class="subrow">
              <a href="/subscriptions/{detail.ticket.subscription_id}" class="subscription-link">
                {$t('support.detail.view_subscription')} →
              </a>
            </div>
          {/if}
        </div>

        <div class="reply panel">
          <div class="reply-head">
            <div class="reply-title">{$t('support.fields.reply')}</div>
            {#if isClosed}
              <span class="pill status-closed">
                {$t('support.status.closed')}
              </span>
            {/if}
          </div>

          {#if isClosed}
            <div class="closed-note">
              <Icon name="lock" size={16} />
              <span>
                {$t('support.detail.closed_notice')}
              </span>
            </div>

            {#if detail.ticket.satisfaction_rating}
              <div class="satisfaction-display">
                <div class="rating-stars">
                  {#each [1,2,3,4,5] as star}
                    <span class="star" class:filled={star <= (detail.ticket.satisfaction_rating ?? 0)}>
                      <Icon name="star" size={14} />
                    </span>
                  {/each}
                  <span class="rating-num">{detail.ticket.satisfaction_rating}/5</span>
                </div>
                {#if detail.ticket.satisfaction_comment}
                  <p class="rating-comment">"{detail.ticket.satisfaction_comment}"</p>
                {/if}
              </div>
            {/if}
          {/if}

          <textarea
            id="support-reply"
            class="textarea"
            rows="4"
            bind:value={message}
            placeholder={$t('support.fields.reply_placeholder')}
            disabled={isClosed}
          ></textarea>
          <div class="file-row">
            <label class="file-label" for="support-reply-files">
              {$t('support.fields.attachments')}
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
        <div class="thread-card panel">
          <div class="thread-head">
            <div class="thread-title">
              {$t('support.detail.thread')}
            </div>
            <div class="thread-sub">
              {detail.messages.length}
              {$t('support.detail.messages')}
            </div>
          </div>

          <div class="chat">
            {#each detail.messages as m (m.id)}
              {@const mine = !!detail.ticket.created_by && m.author_id === detail.ticket.created_by}
              {@const label = m.author_name || (mine
                ? $t('common.you') || 'You'
                : $t('support.labels.support') || 'Support')}
              <div class="msg" class:mine>
                <div class="msg-top">
                  <div class="avatar" class:mine>
                    <Icon name={mine ? 'user' : 'headphones'} size={14} />
                  </div>
                  <span class="who">{label}</span>
                  <span class="dot"></span>
                  <span class="time">
                    {formatDateTime(m.created_at, { timeZone: $appSettings.app_timezone })}
                  </span>
                </div>
                <div class="bubble" class:mine>
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
      <p>{$t('support.detail.not_found')}</p>
    </div>
  {/if}
</div>

{#if lightboxOpen && LightboxComponent}
  <LightboxComponent
    bind:index={lightboxIndex}
    files={lightboxFiles}
    onclose={() => (lightboxOpen = false)}
  />
{/if}

<style>
  .page {
    padding: clamp(1rem, 2.2vw, 1.75rem);
    max-width: 1100px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .detail-head {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .head-id {
    flex: 1;
    text-align: center;
    font-size: 0.85rem;
    font-weight: 700;
    color: var(--text-secondary);
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    padding: 0.35rem 0.75rem;
    border-radius: 999px;
    letter-spacing: 0.04em;
  }

  .panel {
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: var(--radius-lg, 12px);
    padding: 1rem;
  }

  .ticket-card {
    padding: 1rem;
  }

  .title-row {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
  }

  .title {
    font-size: 1.05rem;
    font-weight: 750;
    color: var(--text-primary);
    line-height: 1.3;
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

  .subrow {
    margin-top: 0.55rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .layout {
    display: grid;
    grid-template-columns: minmax(300px, 360px) 1fr;
    gap: 1rem;
    align-items: start;
  }

  @media (max-width: 900px) {
    .layout { grid-template-columns: 1fr; }
    .side { position: static; }
  }

  .side {
    display: grid;
    gap: 1rem;
    position: sticky;
    top: 1rem;
    align-self: start;
  }

  .main { min-width: 0; }

  .thread-card {
    overflow: hidden;
    padding: 0;
  }

  .thread-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.9rem 1rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .thread-title {
    font-weight: 700;
    color: var(--text-primary);
  }

  .thread-sub {
    color: var(--text-secondary);
    font-weight: 650;
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

  .msg.mine { justify-items: end; }

  .msg-top {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.82rem;
    font-weight: 650;
  }

  .avatar {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-secondary);
  }

  .avatar.mine {
    border-color: color-mix(in srgb, var(--color-primary) 30%, transparent);
    background: color-mix(in srgb, var(--color-primary) 12%, transparent);
    color: var(--color-primary);
  }

  .who {
    font-weight: 700;
    color: var(--text-primary);
  }

  .bubble {
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.02);
    border-radius: var(--radius-lg, 12px);
    padding: 0.85rem 0.95rem;
    max-width: min(720px, 100%);
  }

  .bubble.mine {
    border-color: color-mix(in srgb, var(--color-primary) 28%, transparent);
  }

  .msg-body {
    white-space: pre-wrap;
    color: var(--text-primary);
    line-height: 1.55;
  }

  .attachments {
    margin-top: 0.6rem;
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem;
  }

  .file-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-secondary);
    padding: 0.3rem 0.55rem;
    border-radius: 999px;
    cursor: pointer;
    max-width: 100%;
  }

  .file-chip:hover {
    border-color: color-mix(in srgb, var(--color-primary) 35%, transparent);
    color: var(--color-primary);
  }

  .file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 260px;
  }

  .reply {
    display: grid;
    gap: 0.5rem;
  }

  .reply-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .reply-title {
    font-weight: 700;
    color: var(--text-primary);
  }

  .label {
    font-size: 0.85rem;
    font-weight: 650;
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
    min-height: 90px;
  }

  .textarea:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .file-row { display: grid; gap: 0.35rem; }

  .file-label {
    font-size: 0.85rem;
    font-weight: 650;
    color: var(--text-secondary);
  }

  .file {
    width: 100%;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: var(--radius-md, 8px);
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
    border: 1px solid rgba(255, 255, 255, 0.08);
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
    border-radius: 10px;
    border: 1px solid color-mix(in srgb, var(--color-warning) 28%, transparent);
    background: color-mix(in srgb, var(--color-warning) 10%, transparent);
    color: var(--color-warning);
    font-weight: 650;
    font-size: 0.88rem;
  }

  .satisfaction-display {
    margin-top: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .rating-stars {
    display: flex;
    align-items: center;
    gap: 0.15rem;
  }
  .star {
    font-size: 1.15rem;
    color: rgba(255, 255, 255, 0.15);
  }
  .star.filled { color: #f59e0b; }
  .rating-num {
    margin-left: 0.5rem;
    font-weight: 650;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
  .rating-comment {
    font-size: 0.85rem;
    color: var(--text-secondary);
    font-style: italic;
    margin: 0;
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  }

  .dot {
    width: 4px;
    height: 4px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.2);
  }

  .ticket-id {
    color: var(--text-secondary);
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.03);
    padding: 0.1rem 0.45rem;
    border-radius: 999px;
    font-weight: 700;
    letter-spacing: 0.02em;
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

  .empty {
    border: 1px dashed rgba(255, 255, 255, 0.1);
    border-radius: var(--radius-lg, 12px);
    padding: 2rem 1.5rem;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.5rem 0.85rem;
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

  .subscription-link {
    color: var(--color-primary);
    text-decoration: none;
    font-weight: 650;
  }
  .subscription-link:hover { text-decoration: underline; }

  @media (max-width: 560px) {
    .page { padding: 0.75rem; }
    .title-row {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.5rem;
    }
    .meta { justify-content: flex-start; }
    .detail-head { flex-wrap: wrap; }
    .head-id { order: -1; width: 100%; }
  }
</style>
