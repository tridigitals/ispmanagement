<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Announcement, type PaginatedResponse } from '$lib/api/client';
  import { t } from 'svelte-i18n';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ResponsiveTabs from '$lib/components/ui/ResponsiveTabs.svelte';
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

<div class="page-container fade-in">
  <section class="hero-card announcements-hero">
    <div class="hero-left">
      <div class="hero-badge">
        <Icon name="megaphone" size={20} />
      </div>
      <div>
        <div class="kicker">
          <span class="dot"></span>
          {$t('announcements.title')}
        </div>
        <h1 class="hero-title">{$t('announcements.title')}</h1>
        <p class="hero-sub">{$t('announcements.feed_subtitle')}</p>
      </div>
    </div>
    <div class="hero-right">
      <div class="search">
        <Icon name="search" size={16} />
        <input
          class="search-input"
          value={q}
          oninput={(e) => (q = (e.currentTarget as HTMLInputElement).value)}
          placeholder={$t('announcements.search_placeholder') ||
            $t('notifications_page.search_placeholder') ||
            'Search...'}
        />
      </div>
      <button class="btn btn-secondary btn-sm" type="button" onclick={() => load(true)} disabled={loading}>
        <Icon name="refresh-cw" size={14} />
        <span>{$t('common.refresh')}</span>
      </button>
    </div>
  </section>

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
    <div class="glass-card empty-state">
      <Icon name="megaphone" size={32} />
      <div class="empty-text">
        <div class="title">{$t('announcements.empty_feed') || 'No announcements yet'}</div>
        <div class="sub">{$t('announcements.feed_subtitle') || 'Stay tuned for updates.'}</div>
      </div>
    </div>
  {:else}
    <div class="summary">
      <span class="count-num">{rows.length}</span>
      <span class="count-label">{$t('announcements.list.title') || 'announcements'}</span>
      <span class="summary-hint">
        {$t('common.updated')}:
        {formatDateTime(new Date().toISOString(), { timeZone: $appSettings.app_timezone })}
      </span>
    </div>

    <div class="feed">
      {#each rows as a, i (a.id)}
        <button
          class="glass-card post {a.severity}"
          type="button"
          onclick={() => openDetail(a.id)}
          style={`--d:${i * 55}ms`}
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
  .page-container {
    padding: clamp(1rem, 2.2vw, 2rem);
    max-width: 1200px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .announcements-hero {
    flex-wrap: wrap;
    gap: 1rem;
  }

  .hero-left {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    min-width: 260px;
  }

  .hero-right {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
  }

  .kicker {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--text-secondary);
    font-weight: 850;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-size: 0.72rem;
    margin-bottom: 0.35rem;
  }

  .kicker .dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--color-primary);
    box-shadow: 0 0 0 6px var(--color-primary-subtle);
  }

  .search {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    padding: 0.5rem 0.68rem;
    border-radius: 10px;
    min-width: min(340px, 100%);
    color: var(--text-secondary);
  }

  .search-input {
    width: 100%;
    background: transparent;
    border: none;
    outline: none;
    color: inherit;
    font-weight: 750;
    font-size: 0.9rem;
  }

  .filters {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .filter-clear {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-secondary);
    padding: 0.6rem 0.85rem;
    border-radius: 8px;
    font-weight: 700;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }

  .filter-clear:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
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
    font-weight: 750;
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
    font-size: 1.25rem;
    font-weight: 1000;
    color: var(--text-primary);
  }

  .count-label {
    font-weight: 850;
    margin-right: auto;
  }

  .summary-hint {
    font-weight: 650;
    font-size: 0.9rem;
  }

  .feed {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
    gap: 1rem;
  }

  .post {
    text-align: left;
    cursor: pointer;
    padding: 0;
    overflow: hidden;
    transition:
      transform 180ms ease,
      box-shadow 180ms ease,
      border-color 180ms ease;
    animation: rise 420ms ease both;
    animation-delay: var(--d, 0ms);
  }

  .post:hover {
    border-color: rgba(99, 102, 241, 0.35);
    transform: translateY(-1px);
  }

  .post.info:hover { border-color: color-mix(in srgb, var(--color-primary) 45%, var(--border-color)); }
  .post.success:hover { border-color: color-mix(in srgb, var(--color-success) 45%, var(--border-color)); }
  .post.warning:hover { border-color: color-mix(in srgb, var(--color-warning) 45%, var(--border-color)); }
  .post.error:hover { border-color: color-mix(in srgb, var(--color-danger) 45%, var(--border-color)); }

  .cover {
    height: 160px;
    overflow: hidden;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-tertiary);
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
    background: var(--bg-surface);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-weight: 750;
    font-size: 0.85rem;
    padding: 0.8rem 1rem 0;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    padding: 0.2rem 0.5rem;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 800;
    text-transform: capitalize;
    color: var(--text-primary);
  }

  .pill.info { border-color: color-mix(in srgb, var(--color-primary) 28%, var(--border-color)); }
  .pill.success { border-color: color-mix(in srgb, var(--color-success) 28%, var(--border-color)); }
  .pill.warning { border-color: color-mix(in srgb, var(--color-warning) 28%, var(--border-color)); }
  .pill.error { border-color: color-mix(in srgb, var(--color-danger) 28%, var(--border-color)); }

  .banner-pill {
    border-color: color-mix(in srgb, var(--color-primary) 25%, var(--border-color));
    color: var(--color-primary);
  }

  .title {
    padding: 0 1rem;
    margin-top: 0.5rem;
    font-size: 1.05rem;
    font-weight: 950;
    color: var(--text-primary);
    line-height: 1.2;
  }

  .body {
    padding: 0 1rem;
    margin-top: 0.35rem;
    color: var(--text-secondary);
    font-weight: 650;
    line-height: 1.45;
    display: -webkit-box;
    line-clamp: 3;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .more {
    padding: 0 1rem 1rem;
    margin-top: 0.75rem;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    color: var(--color-primary);
    font-weight: 850;
  }

  .load-more {
    display: flex;
    justify-content: center;
  }

  @keyframes rise {
    from { opacity: 0; transform: translateY(10px); }
    to { opacity: 1; transform: translateY(0); }
  }

  @media (max-width: 640px) {
    .page-container { padding: 0.75rem; }
    .hero-right { width: 100%; }
    .search { min-width: 100%; }
    .feed { grid-template-columns: 1fr; }
  }
</style>
