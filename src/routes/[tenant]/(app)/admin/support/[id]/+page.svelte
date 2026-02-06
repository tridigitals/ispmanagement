<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { SupportTicketDetail, SupportTicketMessage, TeamMember } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import { toast } from '$lib/stores/toast';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import Lightbox from '$lib/components/ui/Lightbox.svelte';

  const id = $derived($page.params.id || '');

  function goBack() {
    const parts = $page.url.pathname.split('/').filter(Boolean);
    const target = '/' + parts.slice(0, -1).join('/');
    goto(target || '/');
  }

  let loading = $state(true);
  let saving = $state(false);
  let sending = $state(false);
  let detail = $state<SupportTicketDetail | null>(null);
  const isClosed = $derived(detail?.ticket?.status === 'closed');

  let status = $state<'open' | 'pending' | 'closed'>('open');
  let priority = $state<'low' | 'normal' | 'high' | 'urgent'>('normal');
  let assignedTo = $state<string | null>(null);

  let teamMembers = $state<TeamMember[]>([]);
  let memberOptions = $derived([
    { label: get(t)('common.na') || '—', value: '' },
    ...teamMembers.map((m) => ({ label: `${m.name} (${m.email})`, value: m.user_id })),
  ]);

  let reply = $state('');
  let internalNote = $state(false);
  let attachments = $state<File[]>([]);
  let lightboxOpen = $state(false);
  let lightboxFiles = $state<any[]>([]);
  let lightboxIndex = $state(0);

  const statusOptions = [
    { label: get(t)('support.status.open') || 'Open', value: 'open' },
    { label: get(t)('support.status.pending') || 'Pending', value: 'pending' },
    { label: get(t)('support.status.closed') || 'Closed', value: 'closed' },
  ];

  const priorityOptions = [
    { label: get(t)('support.priorities.low') || 'Low', value: 'low' },
    { label: get(t)('support.priorities.normal') || 'Normal', value: 'normal' },
    { label: get(t)('support.priorities.high') || 'High', value: 'high' },
    { label: get(t)('support.priorities.urgent') || 'Urgent', value: 'urgent' },
  ];

  onMount(() => {
    const onRealtime = (e: Event) => {
      const ce = e as CustomEvent<{ ticket_id: string }>;
      if (ce?.detail?.ticket_id && ce.detail.ticket_id === id) {
        void load();
      }
    };
    window.addEventListener('support_ticket_message', onRealtime as any);

    if (!$can('read_all', 'support')) {
      goto('/unauthorized');
      return () => window.removeEventListener('support_ticket_message', onRealtime as any);
    }
    void Promise.all([load(), loadTeam()]);

    return () => {
      window.removeEventListener('support_ticket_message', onRealtime as any);
    };
  });

  async function loadTeam() {
    try {
      teamMembers = await api.team.list();
    } catch {
      // non-blocking
    }
  }

  async function load() {
    loading = true;
    try {
      if (!id) return;
      detail = await api.support.get(id);
      status = (detail.ticket.status as any) || 'open';
      priority = (detail.ticket.priority as any) || 'normal';
      assignedTo = detail.ticket.assigned_to || null;
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  async function saveTicket() {
    if (!detail) return;
    saving = true;
    try {
      if (!id) return;
      const updated = await api.support.update(id, {
        status,
        priority,
        assignedTo: assignedTo || null,
      });
      detail = { ...detail, ticket: updated as any };
      toast.success(get(t)('admin.support.toasts.updated') || 'Ticket updated');
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      saving = false;
    }
  }

  async function sendReply() {
    if (isClosed) {
      toast.error(get(t)('support.toasts.ticket_closed') || 'Ticket is closed');
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
</script>

<div class="page-content fade-in">
  <div class="head">
    <button class="btn" type="button" onclick={goBack}>
      <Icon name="arrow-left" size={16} />
      {$t('common.back') || 'Back'}
    </button>
    <div class="right">
      <button class="btn" type="button" onclick={load}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
      <button class="btn-primary" type="button" onclick={saveTicket} disabled={saving || loading}>
        <Icon name="save" size={16} />
        {saving ? $t('common.saving') || 'Saving...' : $t('common.save') || 'Save'}
      </button>
    </div>
  </div>

  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <p>{$t('support.loading_detail') || 'Loading ticket...'}</p>
    </div>
  {:else if detail}
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
        <span>
          {$t('support.detail.updated') || 'Updated'}:
          {formatDateTime(detail.ticket.updated_at, { timeZone: $appSettings.app_timezone })}
        </span>
      </div>
    </div>

    <div class="layout">
      <aside class="side">
        <div class="panel">
          <div class="panel-head">
            <div class="panel-title">{$t('admin.support.panels.manage') || 'Manage'}</div>
            <button
              class="btn-primary"
              type="button"
              onclick={saveTicket}
              disabled={saving}
              title={$t('common.save') || 'Save'}
            >
              <Icon name="save" size={16} />
              {saving ? $t('common.saving') || 'Saving...' : $t('common.save') || 'Save'}
            </button>
          </div>
          <div class="form">
            <Select
              label={$t('admin.support.fields.status') || 'Status'}
              bind:value={status}
              options={statusOptions}
            />
            <Select
              label={$t('admin.support.fields.priority') || 'Priority'}
              bind:value={priority}
              options={priorityOptions}
            />
            <Select
              label={$t('admin.support.fields.assignee') || 'Assignee'}
              bind:value={assignedTo}
              options={memberOptions}
            />
          </div>
          <p class="hint">
            {$t('admin.support.hints.assign') ||
              'Assignee expects a team member user_id. Leave empty to keep unassigned.'}
          </p>
        </div>

        <div class="panel">
          <div class="panel-title">{$t('admin.support.panels.reply') || 'Reply'}</div>
          {#if isClosed}
            <div class="closed-note">
              <Icon name="lock" size={16} />
              <span>
                {$t('support.detail.closed_notice') || 'This ticket is closed. You can’t reply.'}
              </span>
            </div>
          {/if}
          <textarea
            class="textarea"
            rows="5"
            bind:value={reply}
            placeholder={$t('support.fields.reply_placeholder') || 'Write your reply...'}
            disabled={isClosed}
          ></textarea>
          <div class="reply-row">
            <div class="file-col">
              <label class="file-label" for="admin-support-reply-files">
                {$t('support.fields.attachments') || 'Attachments'}
              </label>
              <input
                id="admin-support-reply-files"
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

            {#if $can('internal', 'support')}
              <label class="check">
                <input type="checkbox" bind:checked={internalNote} disabled={isClosed} />
                <span>{$t('admin.support.fields.internal') || 'Internal note'}</span>
              </label>
            {/if}
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
            <div class="thread-title">{$t('support.detail.thread') || 'Conversation'}</div>
            <div class="thread-sub">
              {detail.messages.length} {$t('support.detail.messages') || 'messages'}
            </div>
          </div>

          <div class="chat">
            {#each detail.messages as m (m.id)}
              {@const isCustomer = !!detail.ticket.created_by && m.author_id === detail.ticket.created_by}
              {@const mine = !isCustomer}
              {@const who = isCustomer
                ? $t('support.labels.customer') || 'Customer'
                : $t('support.labels.staff') || 'Staff'}
              <div class="msg" class:mine={mine} class:internal={m.is_internal}>
                <div class="msg-top" class:mine={mine}>
                  <div class="avatar" class:mine={mine} class:internal={m.is_internal}>
                    <Icon
                      name={m.is_internal ? 'eye-off' : isCustomer ? 'user' : 'headphones'}
                      size={14}
                    />
                  </div>
                  <span class="who">{who}</span>
                  {#if m.is_internal}
                    <span class="tag">{$t('support.tags.internal') || 'internal'}</span>
                  {/if}
                  <span class="dot"></span>
                  <span class="time">
                    {formatDateTime(m.created_at, { timeZone: $appSettings.app_timezone })}
                  </span>
                </div>

                <div class="bubble" class:mine={mine} class:internal={m.is_internal}>
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
    max-width: 1100px;
    margin: 0 auto;
  }

  .head {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 1rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .right {
    display: inline-flex;
    gap: 0.6rem;
    flex-wrap: wrap;
    justify-content: flex-end;
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
    font-weight: 800;
  }

  .btn-primary {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--color-primary);
    color: white;
    border: none;
    padding: 0.6rem 0.95rem;
    border-radius: 10px;
    cursor: pointer;
    font-weight: 900;
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
    font-weight: 900;
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
    grid-template-columns: minmax(340px, 420px) 1fr;
    gap: 1rem;
    align-items: start;
  }

  @media (max-width: 980px) {
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

  @media (max-width: 980px) {
    .side {
      position: static;
    }
  }

  .main {
    min-width: 0;
  }

  .panel {
    border: 1px solid var(--border-color);
    background:
      radial-gradient(800px 170px at 20% 0%, rgba(255, 255, 255, 0.05), transparent 55%),
      var(--bg-surface);
    border-radius: var(--radius-lg);
    padding: 1rem;
    display: grid;
    gap: 0.75rem;
    box-shadow: var(--shadow-sm);
  }

  .panel-title {
    font-weight: 900;
    color: var(--text-primary);
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .form {
    display: grid;
    gap: 0.75rem;
  }

  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.85rem;
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

  .reply-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
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
    margin-bottom: 0.6rem;
  }

  .file-col {
    display: grid;
    gap: 0.35rem;
    min-width: min(420px, 100%);
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

  .check {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-weight: 700;
    font-size: 0.9rem;
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
    flex-wrap: wrap;
    font-weight: 800;
  }

  .msg-top.mine {
    justify-content: flex-end;
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

  .avatar.internal {
    border-color: rgba(245, 158, 11, 0.35);
    background: rgba(245, 158, 11, 0.12);
    color: rgba(245, 158, 11, 0.95);
  }

  .who {
    font-weight: 900;
    color: var(--text-primary);
  }

  .tag {
    border: 1px solid rgba(245, 158, 11, 0.35);
    background: rgba(245, 158, 11, 0.08);
    color: rgba(245, 158, 11, 0.95);
    padding: 0.12rem 0.5rem;
    border-radius: 999px;
    font-weight: 900;
    text-transform: uppercase;
    font-size: 0.7rem;
    letter-spacing: 0.04em;
  }

  .msg-body {
    white-space: pre-wrap;
    color: var(--text-primary);
    line-height: 1.5;
  }

  .bubble {
    border: 1px solid var(--border-color);
    background:
      radial-gradient(650px 140px at 20% 0%, rgba(255, 255, 255, 0.06), transparent 55%),
      rgba(255, 255, 255, 0.02);
    border-radius: 16px;
    padding: 0.85rem 0.95rem;
    box-shadow: var(--shadow-sm);
    max-width: min(760px, 100%);
  }

  .bubble.mine {
    border-color: rgba(99, 102, 241, 0.28);
    background:
      radial-gradient(650px 140px at 20% 0%, rgba(99, 102, 241, 0.18), transparent 55%),
      rgba(99, 102, 241, 0.07);
  }

  .bubble.internal {
    border-color: rgba(245, 158, 11, 0.32);
    background:
      radial-gradient(700px 150px at 20% 0%, rgba(245, 158, 11, 0.18), transparent 55%),
      rgba(245, 158, 11, 0.07);
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
