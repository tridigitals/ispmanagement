<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Announcement, type CreateAnnouncementDto } from '$lib/api/client';
  import { can, isSuperAdmin } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import { toast } from '$lib/stores/toast';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import RichTextEditor from '$lib/components/ui/RichTextEditor.svelte';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import { stripHtmlToText } from '$lib/utils/sanitizeHtml';

  let loading = $state(true);
  let saving = $state(false);
  let rows = $state<Announcement[]>([]);

  let scope = $state<'tenant' | 'global'>('tenant');
  let audience = $state<'all' | 'admins'>('all');
  let severity = $state<'info' | 'success' | 'warning' | 'error'>('info');
  let mode = $state<'post' | 'banner'>('post');
  let deliverInApp = $state(true);
  let deliverEmail = $state(false);
  let title = $state('');
  let body = $state('');
  let startsAt = $state<string>('');
  let endsAt = $state<string>('');
  let coverFile = $state<File | null>(null);
  let coverPreviewUrl = $state<string>('');

  const scopeOptions = [
    { label: get(t)('announcements.scopes.tenant') || 'Tenant', value: 'tenant' },
    { label: get(t)('announcements.scopes.global') || 'Global', value: 'global' },
  ];
  const audienceOptions = [
    { label: get(t)('announcements.audiences.all') || 'All users', value: 'all' },
    { label: get(t)('announcements.audiences.admins') || 'Admins only', value: 'admins' },
  ];
  const severityOptions = [
    { label: get(t)('announcements.severity.info') || 'Info', value: 'info' },
    { label: get(t)('announcements.severity.success') || 'Success', value: 'success' },
    { label: get(t)('announcements.severity.warning') || 'Warning', value: 'warning' },
    { label: get(t)('announcements.severity.error') || 'Error', value: 'error' },
  ];
  const modeOptions = [
    { label: get(t)('announcements.modes.post') || 'Post', value: 'post' },
    { label: get(t)('announcements.modes.banner') || 'Banner', value: 'banner' },
  ];

  onMount(async () => {
    if (!$can('manage', 'announcements')) {
      goto('/unauthorized');
      return;
    }
    await load();
  });

  async function load() {
    loading = true;
    try {
      rows = await api.announcements.listAdmin($isSuperAdmin ? 'all' : 'tenant');
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  function toIsoOrNull(v: string) {
    const s = (v || '').trim();
    if (!s) return null;
    const d = new Date(s);
    return Number.isNaN(d.getTime()) ? null : d.toISOString();
  }

  function statusOf(a: Announcement) {
    const now = Date.now();
    const start = new Date(a.starts_at).getTime();
    const end = a.ends_at ? new Date(a.ends_at).getTime() : null;
    if (end && end <= now) return 'expired';
    if (start > now) return 'scheduled';
    return 'active';
  }

  async function create() {
    if (!title.trim() || stripHtmlToText(body).length === 0) return;
    if (!deliverInApp && !deliverEmail) {
      toast.error(get(t)('announcements.toasts.delivery_required') || 'Choose at least one delivery channel.');
      return;
    }
    saving = true;
    try {
      let coverFileId: string | null = null;
      if (coverFile) {
        const rec = await api.storage.uploadFile(coverFile);
        coverFileId = rec.id;
      }

      const dto: CreateAnnouncementDto = {
        scope: $isSuperAdmin ? scope : 'tenant',
        cover_file_id: coverFileId,
        title: title.trim(),
        body: body.trim(),
        severity,
        audience,
        mode,
        format: 'html',
        deliver_in_app: deliverInApp,
        deliver_email: deliverEmail,
        starts_at: toIsoOrNull(startsAt),
        ends_at: toIsoOrNull(endsAt),
      };

      await api.announcements.createAdmin(dto);
      toast.success(get(t)('announcements.toasts.created') || 'Announcement created');
      title = '';
      body = '';
      startsAt = '';
      endsAt = '';
      mode = 'post';
      deliverInApp = true;
      deliverEmail = false;
      coverFile = null;
      coverPreviewUrl = '';
      await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      saving = false;
    }
  }

  function onPickCover(e: Event) {
    const input = e.target as HTMLInputElement;
    const f = (input.files || [])[0] || null;
    coverFile = f;
    if (coverPreviewUrl) URL.revokeObjectURL(coverPreviewUrl);
    coverPreviewUrl = f ? URL.createObjectURL(f) : '';
  }

  async function remove(id: string) {
    if (!confirm(get(t)('announcements.confirm_delete') || 'Delete this announcement?')) return;
    try {
      await api.announcements.deleteAdmin(id);
      toast.success(get(t)('announcements.toasts.deleted') || 'Deleted');
      await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }
</script>

<div class="page fade-in">
  <div class="head">
    <div>
      <div class="h1">{$t('announcements.title') || 'Announcements'}</div>
      <div class="sub">
        {$t('announcements.subtitle') || 'Broadcast messages to users as banners and notifications.'}
      </div>
    </div>
    <div class="actions">
      <button class="btn" type="button" onclick={load} title={$t('common.refresh') || 'Refresh'}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
    </div>
  </div>

  <div class="grid">
    <div class="panel">
      <div class="panel-title">{$t('announcements.create.title') || 'Create broadcast'}</div>
      <div class="form">
        {#if $isSuperAdmin}
          <Select
            label={$t('announcements.fields.scope') || 'Scope'}
            bind:value={scope}
            options={scopeOptions}
          />
        {/if}
        <Select
          label={$t('announcements.fields.audience') || 'Audience'}
          bind:value={audience}
          options={audienceOptions}
        />
        <Select
          label={$t('announcements.fields.severity') || 'Severity'}
          bind:value={severity}
          options={severityOptions}
        />
        <Select
          label={$t('announcements.fields.mode') || 'Mode'}
          bind:value={mode}
          options={modeOptions}
        />
        <label class="label">
          {$t('announcements.fields.cover') || 'Cover image (optional)'}
          <input class="input" type="file" accept="image/*" onchange={onPickCover} />
          {#if coverPreviewUrl}
            <div class="cover-preview">
              <img src={coverPreviewUrl} alt="cover preview" />
            </div>
          {/if}
        </label>
        <div class="row delivery">
          <div class="delivery-item">
            <div class="delivery-text">
              <div class="delivery-title">
                {$t('announcements.fields.deliver_in_app') || 'Deliver in-app'}
              </div>
              <div class="delivery-sub">
                {$t('announcements.fields.deliver_in_app_desc') ||
                  'Show to users in the app and send a notification.'}
              </div>
            </div>
            <Toggle bind:checked={deliverInApp} ariaLabel="Deliver in-app" />
          </div>
          <div class="delivery-item">
            <div class="delivery-text">
              <div class="delivery-title">
                {$t('announcements.fields.deliver_email') || 'Send email'}
              </div>
              <div class="delivery-sub">
                {$t('announcements.fields.deliver_email_desc') ||
                  'Send this announcement to all recipients via email (ignores preferences).'} 
              </div>
            </div>
            <Toggle bind:checked={deliverEmail} ariaLabel="Send email" />
          </div>
        </div>
        <label class="label">
          {$t('announcements.fields.title') || 'Title'}
          <input class="input" bind:value={title} placeholder="e.g. Planned maintenance" />
        </label>
        <RichTextEditor
          label={$t('announcements.fields.body') || 'Body'}
          bind:value={body}
          placeholder={$t('announcements.placeholders.body') || 'Write something clear and short…'}
          help={$t('announcements.hints.rich') ||
            'Tip: Keep it concise. Links are allowed; images should be added as cover.'}
          minHeight={190}
        />
        <div class="row">
          <label class="label">
            {$t('announcements.fields.starts_at') || 'Starts at'}
            <input class="input" type="datetime-local" bind:value={startsAt} />
          </label>
          <label class="label">
            {$t('announcements.fields.ends_at') || 'Ends at'}
            <input class="input" type="datetime-local" bind:value={endsAt} />
          </label>
        </div>
      </div>
      <div class="foot">
        <button class="btn-primary" type="button" onclick={create} disabled={saving}>
          <Icon name="megaphone" size={16} />
          {saving ? $t('common.saving') || 'Saving...' : $t('announcements.actions.publish') || 'Publish'}
        </button>
      </div>
      <p class="hint">
        {$t('announcements.hints.schedule') ||
          'Leave dates empty to publish immediately. End date controls when the banner stops showing.'}
      </p>
    </div>

    <div class="panel">
      <div class="panel-title">{$t('announcements.list.title') || 'History'}</div>

      {#if loading}
        <div class="loading">
          <div class="spinner"></div>
          <div>{$t('common.loading') || 'Loading...'}</div>
        </div>
      {:else if rows.length === 0}
        <div class="empty">
          <Icon name="info" size={18} />
          <span>{$t('announcements.empty') || 'No announcements yet.'}</span>
        </div>
      {:else}
        <div class="table">
          {#each rows as a (a.id)}
            <div class="item">
              <div class="left">
                <div class="badges">
                  <span class="pill sev {a.severity}">{a.severity}</span>
                  <span class="pill st {statusOf(a)}">{statusOf(a)}</span>
                  {#if a.tenant_id === null}
                    <span class="pill scope global">global</span>
                  {/if}
                  {#if a.audience === 'admins'}
                    <span class="pill aud admins">admins</span>
                  {/if}
                </div>
                <div class="ttl">{a.title}</div>
                <div class="meta">
                  <span>
                    {formatDateTime(a.created_at, { timeZone: $appSettings.app_timezone })}
                  </span>
                  {#if a.starts_at}
                    <span class="dot"></span>
                    <span>
                      {$t('announcements.fields.starts_at') || 'Starts'}:
                      {formatDateTime(a.starts_at, { timeZone: $appSettings.app_timezone })}
                    </span>
                  {/if}
                  {#if a.ends_at}
                    <span class="dot"></span>
                    <span>
                      {$t('announcements.fields.ends_at') || 'Ends'}:
                      {formatDateTime(a.ends_at, { timeZone: $appSettings.app_timezone })}
                    </span>
                  {/if}
                </div>
              </div>

              <div class="right">
                <button class="btn danger" type="button" onclick={() => remove(a.id)}>
                  <Icon name="trash-2" size={16} />
                  {$t('common.delete') || 'Delete'}
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .page {
    padding: 1.5rem;
    max-width: 1200px;
    margin: 0 auto;
  }

  .head {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    flex-wrap: wrap;
    margin-bottom: 1rem;
  }

  .h1 {
    font-size: 1.25rem;
    font-weight: 950;
    color: var(--text-primary);
  }

  .sub {
    margin-top: 0.2rem;
    color: var(--text-secondary);
    font-weight: 650;
  }

  .actions {
    display: inline-flex;
    gap: 0.6rem;
  }

  .grid {
    display: grid;
    grid-template-columns: minmax(340px, 420px) 1fr;
    gap: 1rem;
    align-items: start;
  }

  @media (max-width: 980px) {
    .grid {
      grid-template-columns: 1fr;
    }
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
    font-weight: 950;
    color: var(--text-primary);
  }

  .form {
    display: grid;
    gap: 0.75rem;
  }

  .label {
    display: grid;
    gap: 0.35rem;
    color: var(--text-secondary);
    font-weight: 750;
    font-size: 0.9rem;
  }

  .row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .row.delivery {
    grid-template-columns: 1fr;
  }

  .delivery-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.75rem 0.85rem;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.02);
  }

  :global([data-theme='light']) .delivery-item {
    background: rgba(0, 0, 0, 0.01);
  }

  .delivery-text {
    min-width: 0;
  }

  .delivery-title {
    font-weight: 900;
    color: var(--text-primary);
    line-height: 1.15;
  }

  .delivery-sub {
    margin-top: 0.2rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
    font-weight: 650;
    line-height: 1.35;
  }

  .cover-preview {
    margin-top: 0.6rem;
    border-radius: 14px;
    overflow: hidden;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.02);
  }

  .cover-preview img {
    display: block;
    width: 100%;
    max-height: 160px;
    object-fit: cover;
  }

  @media (max-width: 520px) {
    .row {
      grid-template-columns: 1fr;
    }
  }

  .input,
  .textarea {
    width: 100%;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: var(--radius-md);
    padding: 0.65rem 0.85rem;
    font-size: 0.95rem;
  }

  .textarea {
    resize: vertical;
    min-height: 110px;
  }

  .input:focus,
  .textarea:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .foot {
    display: flex;
    justify-content: flex-end;
  }

  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.85rem;
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
    font-weight: 850;
  }

  .btn.danger {
    border-color: rgba(239, 68, 68, 0.25);
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
    font-weight: 950;
  }

  .btn-primary:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  .loading {
    display: grid;
    place-items: center;
    padding: 2rem 1rem;
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
    padding: 1.25rem;
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    gap: 0.6rem;
  }

  .table {
    display: grid;
    gap: 0.6rem;
  }

  .item {
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 0.85rem;
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    background: rgba(255, 255, 255, 0.02);
  }

  .left {
    min-width: 0;
  }

  .ttl {
    margin-top: 0.35rem;
    font-weight: 950;
    color: var(--text-primary);
    line-height: 1.2;
  }

  .meta {
    margin-top: 0.35rem;
    display: inline-flex;
    flex-wrap: wrap;
    gap: 0.45rem;
    color: var(--text-secondary);
    font-weight: 700;
    font-size: 0.85rem;
    align-items: center;
  }

  .dot {
    width: 4px;
    height: 4px;
    border-radius: 999px;
    background: var(--border-color);
  }

  .badges {
    display: inline-flex;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .pill {
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.04);
    color: rgba(255, 255, 255, 0.85);
    border-radius: 999px;
    padding: 0.12rem 0.5rem;
    font-weight: 900;
    text-transform: uppercase;
    font-size: 0.7rem;
    letter-spacing: 0.04em;
  }

  .pill.sev.info {
    border-color: rgba(59, 130, 246, 0.35);
    color: rgba(59, 130, 246, 0.95);
    background: rgba(59, 130, 246, 0.08);
  }
  .pill.sev.success {
    border-color: rgba(34, 197, 94, 0.35);
    color: rgba(34, 197, 94, 0.95);
    background: rgba(34, 197, 94, 0.08);
  }
  .pill.sev.warning {
    border-color: rgba(245, 158, 11, 0.35);
    color: rgba(245, 158, 11, 0.95);
    background: rgba(245, 158, 11, 0.08);
  }
  .pill.sev.error {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgba(239, 68, 68, 0.95);
    background: rgba(239, 68, 68, 0.08);
  }

  .pill.st.active {
    border-color: rgba(34, 197, 94, 0.25);
    color: rgba(34, 197, 94, 0.9);
    background: rgba(34, 197, 94, 0.06);
  }
  .pill.st.scheduled {
    border-color: rgba(245, 158, 11, 0.25);
    color: rgba(245, 158, 11, 0.92);
    background: rgba(245, 158, 11, 0.06);
  }
  .pill.st.expired {
    border-color: rgba(148, 163, 184, 0.22);
    color: rgba(148, 163, 184, 0.95);
    background: rgba(148, 163, 184, 0.08);
  }

  .pill.scope.global {
    border-color: rgba(236, 72, 153, 0.25);
    color: rgba(236, 72, 153, 0.95);
    background: rgba(236, 72, 153, 0.07);
  }

  .pill.aud.admins {
    border-color: rgba(99, 102, 241, 0.25);
    color: rgba(99, 102, 241, 0.95);
    background: rgba(99, 102, 241, 0.07);
  }
</style>
