<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Announcement, type PaginatedResponse } from '$lib/api/client';
  import { t } from 'svelte-i18n';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { stripHtmlToText } from '$lib/utils/sanitizeHtml';
  import { getApiBaseUrl } from '$lib/utils/apiUrl';

  const API_BASE = getApiBaseUrl();

  let rows = $state<Announcement[]>([]);
  let total = $state(0);
  let pageNum = $state(1);
  const perPage = 20;

  let loading = $state(true);
  let loadingMore = $state(false);
  let isMobile = $state(false);
  let q = $state('');
  let sev = $state<'all' | 'info' | 'success' | 'warning' | 'error'>('all');
  let mode = $state<'all' | 'post' | 'banner'>('all');

  let hasMore = $derived(rows.length < total);
  const severityTabs = $derived.by(() => [
    { id: 'all', label: $t('common.all') || 'All' },
    { id: 'info', label: badgeLabel('info') },
    { id: 'success', label: badgeLabel('success') },
    { id: 'warning', label: badgeLabel('warning') },
    { id: 'error', label: badgeLabel('error') },
  ]);
  const modeTabs = $derived.by(() => [
    { id: 'all', label: $t('common.all') || 'All' },
    { id: 'post', label: $t('announcements.modes.post') || 'Post' },
    { id: 'banner', label: $t('announcements.modes.banner') || 'Banner' },
  ]);

  function snippet(body: string) {
    const s = stripHtmlToText(body || '');
    if (s.length <= 220) return s;
    return s.slice(0, 220) + '…';
  }

  function badgeIcon(sev: string) {
    switch (sev) {
      case 'success':
        return 'check-circle';
      case 'warning':
        return 'alert-circle';
      case 'error':
        return 'alert-circle';
      default:
        return 'info';
    }
  }

  function badgeLabel(sev: string) {
    switch (sev) {
      case 'success':
        return $t('announcements.severity.success') || 'Success';
      case 'warning':
        return $t('announcements.severity.warning') || 'Warning';
      case 'error':
        return $t('announcements.severity.error') || 'Error';
      default:
        return $t('announcements.severity.info') || 'Info';
    }
  }

  function clearFilters() {
    q = '';
    sev = 'all';
    mode = 'all';
  }

  function openDetail(id: string) {
    const p = $page.url.pathname || '';
    const base = p.endsWith('/') ? p.slice(0, -1) : p;
    goto(`${base}/${id}`);
  }

  async function load(reset: boolean) {
    loading = true;
    if (reset) {
      pageNum = 1;
      rows = [];
      total = 0;
    }
    try {
      const res: PaginatedResponse<Announcement> = await api.announcements.listRecent({
        page: pageNum,
        per_page: perPage,
        search: q.trim() || undefined,
        severity: sev === 'all' ? undefined : sev,
        mode: mode === 'all' ? undefined : mode,
      });
      total = res.total || 0;
      rows = reset ? res.data : [...rows, ...res.data];
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    const mq = window.matchMedia('(max-width: 900px)');
    const updateViewport = () => {
      isMobile = mq.matches;
    };
    updateViewport();
    mq.addEventListener('change', updateViewport);

    void load(true);
    const onChange = () => void load(true);
    window.addEventListener('announcements_changed', onChange);
    return () => {
      mq.removeEventListener('change', updateViewport);
      window.removeEventListener('announcements_changed', onChange);
    };
  });

  $effect(() => {
    const query = q;
    const s = sev;
    const m = mode;
    const timer = setTimeout(() => void load(true), 250);
    return () => clearTimeout(timer);
  });

  async function loadMore() {
    if (loadingMore || loading || !hasMore) return;
    loadingMore = true;
    try {
      pageNum += 1;
      const res: PaginatedResponse<Announcement> = await api.announcements.listRecent({
        page: pageNum,
        per_page: perPage,
        search: q.trim() || undefined,
        severity: sev === 'all' ? undefined : sev,
        mode: mode === 'all' ? undefined : mode,
      });
      total = res.total || total;
      rows = [...rows, ...res.data];
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loadingMore = false;
    }
  }
</script>

<div class="page fade-in">
  <div class="page-head">
    <div class="page-head-text">
      <h1>{$t('announcements.title') || 'Pengumuman'}</h1>
      <p class="page-sub">{$t('announcements.feed_subtitle') || 'Update terbaru dari ISP'}</p>
    </div>
    <div class="head-actions">
      <div class="search">
        <Icon name="search" size={16} />
        <input
          class="search-input"
          value={q}
          oninput={(e) => (q = (e.currentTarget as HTMLInputElement).value)}
          placeholder={$t('announcements.search_placeholder') ||
            $t('notifications_page.search_placeholder') ||
            'Cari...'}
        />
      </div>
      <button class="btn btn-ghost" type="button" onclick={() => load(true)} disabled={loading}>
        <Icon name="refresh-cw" size={14} />
        <span>{$t('common.refresh') || 'Refresh'}</span>
      </button>
    </div>
  </div>

  <div class="filters">
    <Select
      bind:value={sev}
      options={severityTabs.map(t => ({ label: t.label, value: t.id }))}
      placeholder={$t('announcements.filters.severity') || 'Severity'}
      width="160px"
    />
    <Select
      bind:value={mode}
      options={modeTabs.map(t => ({ label: t.label, value: t.id }))}
      placeholder={$t('announcements.filters.mode') || 'Mode'}
      width="140px"
    />
    {#if q.trim() || sev !== 'all' || mode !== 'all'}
      <button class="filter-clear" type="button" onclick={clearFilters}>
        <Icon name="x" size={16} />
        {$t('common.clear')}
      </button>
    {/if}
  </div>

  {#if loading && rows.length === 0}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>{$t('common.loading')}</p>
    </div>
  {:else if rows.length === 0}
    <div class="panel empty-state">
      <Icon name="megaphone" size={32} />
      <div class="empty-text">
        <div class="title">{$t('announcements.empty_feed') || 'Belum ada pengumuman'}</div>
        <div class="sub">{$t('announcements.feed_subtitle') || 'Nantikan update selanjutnya.'}</div>
      </div>
    </div>
  {:else}
    <div class="summary">
      <span class="count-num">{rows.length}</span>
      <span class="count-label">{$t('announcements.list.title') || 'pengumuman'}</span>
      <span class="summary-hint">
        {$t('common.updated') || 'Diperbarui'}:
        {formatDateTime(new Date().toISOString(), { timeZone: $appSettings.app_timezone })}
      </span>
    </div>

    <div class="feed">
      {#each rows as a (a.id)}
        <button
          class="panel post {a.severity}"
          type="button"
          onclick={() => openDetail(a.id)}
        >
          {#if a.cover_file_id}
            <div class="cover">
              <img
                src={`${API_BASE}/storage/files/${a.cover_file_id}/content`}
                alt=""
                loading="lazy"
              />
            </div>
          {:else}
            <div class="cover fallback">
              <Icon name={badgeIcon(a.severity)} size={28} />
            </div>
          {/if}
          <div class="meta">
            <span class="pill {a.severity}">
              <Icon name={badgeIcon(a.severity)} size={12} />
              {badgeLabel(a.severity)}
            </span>
            <span>{formatDateTime(a.starts_at, { timeZone: $appSettings.app_timezone })}</span>
            {#if a.mode === 'banner'}
              <span class="pill banner-pill">
                <Icon name="flag" size={12} />
                {$t('announcements.modes.banner') || 'Banner'}
              </span>
            {/if}
          </div>
          <div class="title">{a.title}</div>
          <div class="body">{snippet(a.body)}</div>
          <div class="more">
            {$t('announcements.actions.read') || 'Read'}
            <Icon name="arrow-right" size={14} />
          </div>
        </button>
      {/each}
    </div>

    {#if hasMore}
      <div class="load-more">
        <button class="btn btn-secondary" type="button" onclick={loadMore} disabled={loadingMore}>
          <Icon name="chevron-down" size={16} />
          {loadingMore
            ? $t('common.loading') || 'Loading...'
            : $t('common.load_more') || 'Load more'}
        </button>
      </div>
    {/if}
  {/if}
</div>

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
    gap: 0.55rem;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.02);
    padding: 0.5rem 0.68rem;
    border-radius: 10px;
    min-width: min(280px, 100%);
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

  .filters {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex-wrap: wrap;
  }
  .filter-clear {
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: transparent;
    color: var(--text-secondary);
    padding: 0.5rem 0.75rem;
    border-radius: 8px;
    font-weight: 650;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }
  .filter-clear:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.04);
  }

  .panel {
    background: var(--bg-surface);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: var(--radius-lg, 12px);
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    padding: 3rem 1rem;
    color: var(--text-secondary);
  }
  .empty-state {
    padding: 1.5rem;
    display: flex;
    gap: 0.9rem;
    align-items: flex-start;
  }
  .empty-text .title {
    font-weight: 700;
    margin-bottom: 0.25rem;
    color: var(--text-primary);
  }
  .empty-text .sub {
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .summary {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    color: var(--text-secondary);
  }
  .count-num {
    font-size: 1.15rem;
    font-weight: 750;
    color: var(--text-primary);
  }
  .count-label {
    font-weight: 650;
    margin-right: auto;
  }
  .summary-hint {
    font-size: 0.85rem;
  }

  .feed {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 0.85rem;
  }
  .post {
    text-align: left;
    cursor: pointer;
    padding: 0;
    overflow: hidden;
    transition: border-color 0.12s ease;
  }
  .post:hover {
    border-color: rgba(99, 102, 241, 0.35);
  }

  .cover {
    height: 140px;
    overflow: hidden;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(255, 255, 255, 0.02);
    display: grid;
    place-items: stretch;
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .cover.fallback {
    place-items: center;
    color: var(--text-secondary);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.82rem;
    padding: 0.8rem 1rem 0;
    flex-wrap: wrap;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.03);
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: capitalize;
    color: var(--text-primary);
  }
  .pill.info { border-color: color-mix(in srgb, var(--color-primary) 28%, transparent); color: var(--color-primary); }
  .pill.success { border-color: color-mix(in srgb, var(--color-success) 28%, transparent); color: var(--color-success); }
  .pill.warning { border-color: color-mix(in srgb, var(--color-warning) 28%, transparent); color: var(--color-warning); }
  .pill.error { border-color: color-mix(in srgb, var(--color-danger) 28%, transparent); color: var(--color-danger); }
  .banner-pill { color: var(--color-primary); }

  .title {
    padding: 0 1rem;
    margin-top: 0.45rem;
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1.25;
  }
  .body {
    padding: 0 1rem;
    margin-top: 0.3rem;
    color: var(--text-secondary);
    font-size: 0.9rem;
    line-height: 1.45;
    display: -webkit-box;
    line-clamp: 3;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .more {
    padding: 0 1rem 1rem;
    margin-top: 0.65rem;
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    color: var(--color-primary);
    font-weight: 650;
    font-size: 0.88rem;
  }
  .load-more {
    display: flex;
    justify-content: center;
  }

  @media (max-width: 640px) {
    .page { padding: 0.75rem; }
    .head-actions { width: 100%; }
    .search { min-width: 100%; flex: 1; }
    .feed { grid-template-columns: 1fr; }
  }
</style>
