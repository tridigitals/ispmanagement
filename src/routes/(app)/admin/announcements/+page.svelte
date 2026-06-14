<script lang="ts">
  import type { Component } from 'svelte';
  import { onMount } from 'svelte';
  import {
    api,
    type Announcement,
    type CreateAnnouncementDto,
    type PaginatedResponse,
  } from '$lib/api/client';
  import { can, isSuperAdmin } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import { toast } from '$lib/stores/toast';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import DateTimeLocalInput from '$lib/components/ui/DateTimeLocalInput.svelte';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import { stripHtmlToText } from '$lib/utils/sanitizeHtml';
  import { loadAnnouncementEditorComponent } from './announcementsPageModules';
  import { getApiBaseUrl } from '$lib/utils/apiUrl';

  let loading = $state(true);
  let saving = $state(false);
  let showDeleteConfirm = $state(false);
  let deleteTargetId = $state<string | null>(null);
  let rows = $state<Announcement[]>([]);
  let total = $state(0);
  let pageNum = $state(1);
  let perPage = $state(20);

  let searchQuery = $state('');
  let statusFilter = $state<'all' | 'active' | 'scheduled' | 'expired'>('all');
  let severityFilter = $state<'all' | 'info' | 'success' | 'warning' | 'error'>('all');
  let modeFilter = $state<'all' | 'post' | 'banner'>('all');
  let scopeFilter = $state<'all' | 'tenant' | 'global'>('all');

  let totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));
  let activeTab = $state<'create' | 'history'>('create');

  let scope = $state<'tenant' | 'global'>('tenant');
  let audience = $state<'all' | 'admins' | 'customers' | 'active_subscribers' | 'suspended_subscribers'>('all');
  let severity = $state<'info' | 'success' | 'warning' | 'error'>('info');
  let mode = $state<'post' | 'banner'>('post');
  let deliverInApp = $state(true);
  let deliverEmail = $state(false);
  let deliverEmailForce = $state(true);
  let title = $state('');
  let body = $state('');
  let startsAt = $state<string>('');
  let endsAt = $state<string>('');
  let coverFile = $state<File | null>(null);
  let coverPreviewUrl = $state<string>('');
  let AnnouncementEditorComponent = $state<Component | null>(null);
  let announcementEditorLoading = $state(false);

  const scopeOptions = [
    { label: get(t)('announcements.scopes.tenant') || 'Tenant', value: 'tenant' },
    { label: get(t)('announcements.scopes.global') || 'Global', value: 'global' },
  ];
  const audienceOptions = [
    { label: get(t)('announcements.audiences.all') || 'All users', value: 'all' },
    { label: get(t)('announcements.audiences.admins') || 'Admins only', value: 'admins' },
    { label: get(t)('announcements.audiences.customers') || 'Customers', value: 'customers' },
    { label: get(t)('announcements.audiences.active_subscribers') || 'Active subscribers', value: 'active_subscribers' },
    { label: get(t)('announcements.audiences.suspended_subscribers') || 'Suspended subscribers', value: 'suspended_subscribers' },
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

  const statusFilterOptions = [
    { label: get(t)('common.all') || 'All', value: 'all' },
    { label: get(t)('common.active') || 'Active', value: 'active' },
    { label: get(t)('announcements.status.scheduled') || 'Scheduled', value: 'scheduled' },
    { label: get(t)('announcements.status.expired') || 'Expired', value: 'expired' },
  ];

  const severityFilterOptions = [
    { label: get(t)('common.all') || 'All', value: 'all' },
    ...severityOptions,
  ];

  const modeFilterOptions = [
    { label: get(t)('common.all') || 'All', value: 'all' },
    ...modeOptions,
  ];

  const scopeFilterOptions = [
    { label: get(t)('common.all') || 'All', value: 'all' },
    { label: get(t)('announcements.scopes.tenant') || 'Tenant', value: 'tenant' },
    { label: get(t)('announcements.scopes.global') || 'Global', value: 'global' },
  ];

  onMount(async () => {
    if (!$can('manage', 'announcements')) {
      goto('/unauthorized');
      return;
    }
    void ensureAnnouncementEditorLoaded();
    await load();
  });

  async function ensureAnnouncementEditorLoaded() {
    if (AnnouncementEditorComponent || announcementEditorLoading) return;

    announcementEditorLoading = true;
    try {
      const { EditorComponent } = await loadAnnouncementEditorComponent();
      AnnouncementEditorComponent = EditorComponent;
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      announcementEditorLoading = false;
    }
  }

  async function load() {
    loading = true;
    try {
      const effectiveScope = $isSuperAdmin ? scopeFilter : 'tenant';
      const res: PaginatedResponse<Announcement> = await api.announcements.listAdmin({
        scope: effectiveScope,
        page: pageNum,
        per_page: perPage,
        search: searchQuery.trim() || undefined,
        severity: severityFilter === 'all' ? undefined : severityFilter,
        mode: modeFilter === 'all' ? undefined : modeFilter,
        status: statusFilter === 'all' ? undefined : statusFilter,
      });
      total = res.total || 0;
      rows = res.data;
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (activeTab !== 'history') return;
    const q = searchQuery;
    const s = statusFilter;
    const sev = severityFilter;
    const m = modeFilter;
    const sc = scopeFilter;
    const timer = setTimeout(() => { pageNum = 1; void load(); }, 250);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (activeTab !== 'create') return;
    void ensureAnnouncementEditorLoaded();
  });

  function goToPage(p: number) {
    if (p < 1 || p > totalPages || p === pageNum) return;
    pageNum = p;
    void load();
  }

  function onPerPageChange(e: Event) {
    const val = Number((e.target as HTMLSelectElement).value);
    if (!val || val === perPage) return;
    perPage = val;
    pageNum = 1;
    void load();
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

  function coverUrl(a: Announcement): string {
    return `${getApiBaseUrl()}/storage/files/${a.cover_file_id}/content`;
  }

  async function create() {
    if (!title.trim() || stripHtmlToText(body).length === 0) return;
    if (!deliverInApp && !deliverEmail) {
      toast.error(
        get(t)('announcements.toasts.delivery_required') || 'Choose at least one delivery channel.',
      );
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
        deliver_email_force: deliverEmailForce,
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
      deliverEmailForce = true;
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

  function confirmDelete(id: string) {
    deleteTargetId = id;
    showDeleteConfirm = true;
  }

  async function handleConfirmDelete() {
    if (!deleteTargetId) return;
    try {
      await api.announcements.deleteAdmin(deleteTargetId);
      toast.success(get(t)('announcements.toasts.deleted') || 'Deleted');
      showDeleteConfirm = false;
      deleteTargetId = null;
      await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }
</script>

<div class="page-container fade-in">
  <div class="head">
    <div>
      <div class="h1">{$t('announcements.title') || 'Announcements'}</div>
      <div class="sub">
        {$t('announcements.subtitle') ||
          'Broadcast messages to users as banners and notifications.'}
      </div>
    </div>
    <div class="actions">
      <button
        class="btn"
        type="button"
        onclick={() => { pageNum = 1; void load(); }}
        title={$t('common.refresh') || 'Refresh'}
      >
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
    </div>
  </div>

  <!-- Horizontal tab bar replacing side nav -->
  <div class="tab-bar">
    <button
      class="tab-btn {activeTab === 'create' ? 'active' : ''}"
      type="button"
      onclick={() => (activeTab = 'create')}
    >
      <Icon name="megaphone" size={16} />
      {$t('announcements.create.title') || 'Create broadcast'}
    </button>
    <button
      class="tab-btn {activeTab === 'history' ? 'active' : ''}"
      type="button"
      onclick={() => (activeTab = 'history')}
    >
      <Icon name="list" size={16} />
      {$t('announcements.list.title') || 'History'}
    </button>
  </div>

  <main class="content">
    {#if activeTab === 'create'}
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
          <label class="label span-2">
            {$t('announcements.fields.cover') || 'Cover image (optional)'}
            <input class="input" type="file" accept="image/*" onchange={onPickCover} />
            {#if coverPreviewUrl}
              <div class="cover-preview">
                <img src={coverPreviewUrl} alt="cover preview" />
              </div>
            {/if}
          </label>
          <div class="row delivery span-2">
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
            {#if deliverEmail}
              <div class="delivery-item">
                <div class="delivery-text">
                  <div class="delivery-title">
                    {$t('announcements.fields.deliver_email_force') || 'Ignore email preferences'}
                  </div>
                  <div class="delivery-sub">
                    {$t('announcements.fields.deliver_email_force_desc') ||
                      'When disabled, only users who enabled email announcements will receive it.'}
                  </div>
                </div>
                <Toggle bind:checked={deliverEmailForce} ariaLabel="Ignore preferences" />
              </div>
            {/if}
          </div>
          <label class="label span-2">
            {$t('announcements.fields.title') || 'Title'}
            <input class="input" bind:value={title} placeholder="e.g. Planned maintenance" />
          </label>
          <div class="span-2">
            {#if AnnouncementEditorComponent}
              <AnnouncementEditorComponent
                label={$t('announcements.fields.body') || 'Body'}
                bind:value={body}
                placeholder={$t('announcements.placeholders.body') ||
                  'Write something clear and short…'}
                help={$t('announcements.hints.rich') ||
                  'Tip: Keep it concise. Links are allowed; images should be added as cover.'}
                minHeight={190}
              />
            {:else}
              <div class="editor-placeholder" aria-busy={announcementEditorLoading}>
                <div class="label">{$t('announcements.fields.body') || 'Body'}</div>
                <div class="editor-placeholder-shell">
                  <div class="editor-placeholder-toolbar"></div>
                  <div class="editor-placeholder-body"></div>
                </div>
                <div class="help">
                  {$t('announcements.placeholders.body') || 'Write something clear and short…'}
                </div>
              </div>
            {/if}
          </div>
          <div class="row span-2">
            <DateTimeLocalInput
              label={$t('announcements.fields.starts_at') || 'Starts at'}
              bind:value={startsAt}
            />
            <DateTimeLocalInput
              label={$t('announcements.fields.ends_at') || 'Ends at'}
              bind:value={endsAt}
            />
          </div>
        </div>
        <div class="foot">
          <button class="btn-primary" type="button" onclick={create} disabled={saving}>
            <Icon name="megaphone" size={16} />
            {saving
              ? $t('common.saving') || 'Saving...'
              : $t('announcements.actions.publish') || 'Publish'}
          </button>
        </div>
        <p class="hint">
          {$t('announcements.hints.schedule') ||
            'Leave dates empty to publish immediately. End date controls when the banner stops showing.'}
        </p>
      </div>
    {:else}
      <div class="panel">
        <div class="panel-title">{$t('announcements.list.title') || 'History'}</div>
        <div class="history-controls">
          <div class="search">
            <Icon name="search" size={16} />
            <input
              class="search-input"
              value={searchQuery}
              oninput={(e) => (searchQuery = (e.currentTarget as HTMLInputElement).value)}
              placeholder={$t('announcements.search_placeholder') || 'Search announcements...'}
            />
          </div>
          <div class="filters">
            <div class="filter-slot">
              <Select
                label={$t('announcements.fields.status') || 'Status'}
                options={statusFilterOptions}
                bind:value={statusFilter}
              />
            </div>
            <div class="filter-slot">
              <Select
                label={$t('announcements.fields.severity') || 'Severity'}
                options={severityFilterOptions}
                bind:value={severityFilter}
              />
            </div>
            <div class="filter-slot">
              <Select
                label={$t('announcements.fields.mode') || 'Mode'}
                options={modeFilterOptions}
                bind:value={modeFilter}
              />
            </div>
            {#if $isSuperAdmin}
              <div class="filter-slot">
                <Select
                  label={$t('announcements.fields.scope') || 'Scope'}
                  options={scopeFilterOptions}
                  bind:value={scopeFilter}
                />
              </div>
            {/if}
          </div>
        </div>

        {#if loading && rows.length === 0}
          <div class="loading">
            <div class="spinner"></div>
            <div>{$t('common.loading') || 'Loading...'}</div>
          </div>
        {:else if rows.length === 0}
          <div class="empty-cta">
            <div class="empty-icon">
              <Icon name="megaphone" size={40} />
            </div>
            <div class="empty-title">{$t('announcements.empty') || 'No announcements yet.'}</div>
            <div class="empty-desc">
              {$t('announcements.empty_desc') || 'Create your first broadcast to reach your users.'}
            </div>
            <button class="btn-primary" type="button" onclick={() => (activeTab = 'create')}>
              <Icon name="megaphone" size={16} />
              {$t('announcements.empty_cta') || 'Create your first announcement'}
            </button>
          </div>
        {:else}
          <div class="table">
            {#each rows as a (a.id)}
              <div class="item">
                <!-- Cover thumbnail -->
                <div class="thumb">
                  {#if a.cover_file_id}
                    <img src={coverUrl(a)} alt="" class="thumb-img" />
                  {:else}
                    <div class="thumb-placeholder">
                      <Icon name="image" size={18} />
                    </div>
                  {/if}
                </div>

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
                    <!-- Delivery channel pills -->
                    {#if a.deliver_in_app}
                      <span class="pill channel">in-app ✓</span>
                    {/if}
                    {#if a.deliver_email}
                      <span class="pill channel">email ✓</span>
                    {/if}
                  </div>
                  <div class="ttl">{a.title}</div>
                  <!-- Body excerpt -->
                  <div class="excerpt">{stripHtmlToText(a.body).slice(0, 120)}</div>
                  <div class="meta">
                    {#if a.starts_at}
                      <span>
                        {formatDateTime(a.starts_at, { timeZone: $appSettings.app_timezone })}
                      </span>
                    {/if}
                  </div>
                </div>

                <div class="right">
                  <button
                    class="btn-icon"
                    type="button"
                    title={$t('common.edit') || 'Edit'}
                    onclick={() => goto(`/admin/announcements/${a.id}`)}
                  >
                    <Icon name="edit" size={16} />
                  </button>
                  <button
                    class="btn-icon danger"
                    type="button"
                    title={$t('common.delete') || 'Delete'}
                    onclick={() => confirmDelete(a.id)}
                  >
                    <Icon name="trash-2" size={16} />
                  </button>
                </div>
              </div>
            {/each}
          </div>

          <!-- Pagination bar -->
          <div class="pager">
            <div class="pager-left">
              <span class="pager-info">
                {total} {$t('common.results') || 'results'}
              </span>
            </div>
            <div class="pager-right">
              <label class="per-page">
                <span class="pager-label">{$t('components.pagination.rows_per_page') || 'Per page:'}</span>
                <select class="per-page-select" value={perPage} onchange={onPerPageChange}>
                  <option value={10}>10</option>
                  <option value={20}>20</option>
                  <option value={50}>50</option>
                  <option value={100}>100</option>
                </select>
              </label>
              <span class="pager-info">{$t('common.page') || 'Page'} {pageNum}/{totalPages}</span>
              <button class="btn-icon" type="button" onclick={() => goToPage(pageNum - 1)} disabled={pageNum <= 1}>
                <Icon name="chevron-left" size={16} />
              </button>
              <button class="btn-icon" type="button" onclick={() => goToPage(pageNum + 1)} disabled={pageNum >= totalPages}>
                <Icon name="chevron-right" size={16} />
              </button>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </main>
</div>

<ConfirmDialog
  bind:show={showDeleteConfirm}
  title={$t('announcements.confirm_delete_title') || 'Delete Announcement'}
  message={$t('announcements.confirm_delete') || 'Are you sure you want to permanently delete this announcement? This action cannot be undone.'}
  confirmText={$t('common.delete') || 'Delete'}
  cancelText={$t('common.cancel') || 'Cancel'}
  type="danger"
  onconfirm={handleConfirmDelete}
  oncancel={() => { deleteTargetId = null; }}
/>

<style>
  .page-container {
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

  /* Tab bar */
  .tab-bar {
    display: flex;
    gap: 0.35rem;
    padding: 0.3rem;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: 14px;
    margin-bottom: 1rem;
    width: fit-content;
  }

  .tab-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.55rem 1rem;
    border-radius: 10px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    font-weight: 850;
    font-size: 0.9rem;
    white-space: nowrap;
    transition: background 0.15s, color 0.15s;
  }

  .tab-btn:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .tab-btn.active {
    color: var(--text-primary);
    background: var(--bg-surface);
    border-color: var(--border-color);
    box-shadow: var(--shadow-sm);
  }

  @media (max-width: 480px) {
    .tab-bar {
      width: 100%;
    }
    .tab-btn {
      flex: 1;
      justify-content: center;
      padding: 0.5rem 0.5rem;
      font-size: 0.82rem;
    }
  }

  .content {
    min-width: 0;
  }

  .panel {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
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

  .history-controls {
    display: grid;
    gap: 0.65rem;
    padding-top: 0.15rem;
  }

  .search {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    padding: 0.5rem 0.68rem;
    border-radius: 12px;
    color: var(--text-secondary);
  }

  .search-input {
    width: 100%;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font-weight: 650;
    min-height: 24px;
    min-width: 0;
  }

  .filters {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 0.65rem;
  }

  .filter-slot {
    min-width: 0;
  }

  /* Pagination bar */
  .pager {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-color);
    flex-wrap: wrap;
  }

  .pager-left {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .pager-right {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .pager-info {
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 650;
    white-space: nowrap;
  }

  .pager-label {
    color: var(--text-secondary);
    font-size: 0.82rem;
    font-weight: 600;
  }

  .per-page {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }

  .per-page-select {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: var(--radius-md);
    padding: 0.3rem 0.5rem;
    font-size: 0.82rem;
    font-weight: 700;
    cursor: pointer;
  }

  .per-page-select:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  @media (max-width: 480px) {
    .pager {
      flex-direction: column;
      align-items: stretch;
      gap: 0.5rem;
    }
    .pager-right {
      justify-content: center;
    }
  }

  /* Create form */
  .form {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .span-2 {
    grid-column: 1 / -1;
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
    background: var(--bg-tertiary);
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
    background: var(--bg-tertiary);
  }

  .cover-preview img {
    display: block;
    width: 100%;
    max-height: 160px;
    object-fit: cover;
  }

  @media (max-width: 520px) {
    .form {
      grid-template-columns: 1fr;
    }
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

  .editor-placeholder {
    display: grid;
    gap: 0.45rem;
  }

  .editor-placeholder-shell {
    display: grid;
    gap: 0.55rem;
    padding: 0.7rem;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: var(--bg-tertiary);
  }

  .editor-placeholder-toolbar,
  .editor-placeholder-body {
    border-radius: 10px;
    background: var(--bg-surface);
    background-size: 200% 100%;
    animation: announcement-editor-placeholder 1.2s ease-in-out infinite;
  }

  .editor-placeholder-toolbar {
    height: 2.5rem;
  }

  .editor-placeholder-body {
    min-height: 9rem;
  }

  @keyframes announcement-editor-placeholder {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }

  /* Buttons */
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

  .btn-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .btn-icon:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .btn-icon.danger {
    color: var(--color-danger);
    border-color: color-mix(in srgb, var(--color-danger) 25%, var(--border-color));
  }

  .btn-icon.danger:hover {
    background: color-mix(in srgb, var(--color-danger) 10%, var(--bg-surface));
    color: var(--color-danger);
  }

  /* Loading */
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

  /* Empty state CTA */
  .empty-cta {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 2.5rem 1rem;
    gap: 0.75rem;
    border: 1px dashed var(--border-color);
    border-radius: var(--radius-lg);
  }

  .empty-icon {
    width: 72px;
    height: 72px;
    border-radius: 20px;
    background: var(--color-primary-subtle);
    color: var(--color-primary);
    display: grid;
    place-items: center;
    margin-bottom: 0.25rem;
  }

  .empty-title {
    font-weight: 950;
    color: var(--text-primary);
    font-size: 1.05rem;
  }

  .empty-desc {
    color: var(--text-secondary);
    font-weight: 650;
    font-size: 0.9rem;
    max-width: 320px;
  }

  /* Table / items */
  .table {
    display: grid;
    gap: 0.6rem;
  }

  .item {
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 0.85rem;
    display: flex;
    gap: 0.75rem;
    background: var(--bg-tertiary);
    align-items: flex-start;
  }

  /* Cover thumbnail */
  .thumb {
    flex-shrink: 0;
    width: 48px;
    height: 48px;
    border-radius: 10px;
    overflow: hidden;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
  }

  .thumb-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .thumb-placeholder {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-secondary);
    opacity: 0.5;
  }

  .left {
    min-width: 0;
    flex: 1;
  }

  .ttl {
    margin-top: 0.25rem;
    font-weight: 950;
    color: var(--text-primary);
    line-height: 1.2;
  }

  .excerpt {
    margin-top: 0.2rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 600;
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .meta {
    margin-top: 0.3rem;
    display: inline-flex;
    flex-wrap: wrap;
    gap: 0.45rem;
    color: var(--text-secondary);
    font-weight: 700;
    font-size: 0.85rem;
    align-items: center;
  }

  .right {
    display: flex;
    gap: 0.35rem;
    flex-shrink: 0;
    align-items: flex-start;
  }

  @media (max-width: 768px) {
    .item {
      flex-wrap: wrap;
    }
    .right {
      width: 100%;
      justify-content: flex-end;
      padding-top: 0.35rem;
      border-top: 1px solid var(--border-color);
      margin-top: 0.35rem;
    }
    .filters {
      grid-template-columns: 1fr 1fr;
    }
  }

  @media (max-width: 480px) {
    .page-container {
      padding: 1rem;
    }
    .filters {
      grid-template-columns: 1fr;
    }
  }

  /* Badges */
  .badges {
    display: inline-flex;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .pill {
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border-radius: 999px;
    padding: 0.12rem 0.5rem;
    font-weight: 900;
    text-transform: uppercase;
    font-size: 0.7rem;
    letter-spacing: 0.04em;
  }

  .pill.sev.info {
    border-color: color-mix(in srgb, var(--color-primary) 35%, var(--border-color));
    color: var(--color-primary);
    background: var(--color-primary-subtle);
  }
  .pill.sev.success {
    border-color: color-mix(in srgb, var(--color-success) 35%, var(--border-color));
    color: var(--color-success);
    background: var(--bg-success);
  }
  .pill.sev.warning {
    border-color: color-mix(in srgb, var(--color-warning) 35%, var(--border-color));
    color: var(--color-warning);
    background: color-mix(in srgb, var(--color-warning) 8%, transparent);
  }
  .pill.sev.error {
    border-color: color-mix(in srgb, var(--color-danger) 35%, var(--border-color));
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 8%, transparent);
  }

  .pill.st.active {
    border-color: color-mix(in srgb, var(--color-success) 25%, var(--border-color));
    color: var(--color-success);
    background: var(--bg-success);
  }
  .pill.st.scheduled {
    border-color: color-mix(in srgb, var(--color-warning) 25%, var(--border-color));
    color: var(--color-warning);
    background: color-mix(in srgb, var(--color-warning) 7%, transparent);
  }
  .pill.st.expired {
    border-color: var(--border-color);
    color: var(--text-secondary);
    background: var(--bg-tertiary);
  }

  .pill.scope.global {
    border-color: color-mix(in srgb, var(--color-primary) 25%, var(--border-color));
    color: var(--color-primary);
    background: var(--color-primary-subtle);
  }

  .pill.aud.admins {
    border-color: color-mix(in srgb, var(--color-primary) 25%, var(--border-color));
    color: var(--color-primary);
    background: var(--color-primary-subtle);
  }

  .pill.channel {
    border-color: color-mix(in srgb, var(--color-success) 20%, var(--border-color));
    color: var(--color-success);
    background: color-mix(in srgb, var(--color-success) 6%, transparent);
    text-transform: none;
    font-size: 0.68rem;
  }
</style>
