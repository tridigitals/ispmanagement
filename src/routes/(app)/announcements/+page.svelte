<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Announcement, type PaginatedResponse } from '$lib/api/client';
  import { t } from 'svelte-i18n';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ResponsiveTabs from '$lib/components/ui/ResponsiveTabs.svelte';
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

<div class="page-content fade-in">
  <section class="hero">
    <div class="hero-bg"></div>
    <div class="hero-inner">
      <div class="hgroup">
        <div class="kicker">
          <span class="dot"></span>
          {$t('announcements.title')}
        </div>
        <h1 class="h1">{$t('announcements.title')}</h1>
        <div class="sub">
          {$t('announcements.feed_subtitle')}
        </div>
      </div>

      <div class="hero-actions">
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
        <button
          class="btn"
          type="button"
          onclick={() => load(true)}
          title={$t('common.refresh')}
        >
          <Icon name="refresh-cw" size={16} />
          {$t('common.refresh')}
        </button>
      </div>
    </div>
  </section>

  <div class="filters">
    <div class="filter-group">
      <ResponsiveTabs
        items={severityTabs}
        bind:activeId={sev}
        {isMobile}
        priorityCount={2}
        ariaLabel="Announcement severity filters"
      />
    </div>

    <div class="filter-group align-end">
      <ResponsiveTabs
        items={modeTabs}
        bind:activeId={mode}
        {isMobile}
        priorityCount={2}
        ariaLabel="Announcement mode filters"
      />
      {#if q.trim() || sev !== 'all' || mode !== 'all'}
        <button class="filter-clear" type="button" onclick={clearFilters}>
          <Icon name="x" size={16} />
          {$t('common.clear')}
        </button>
      {/if}
    </div>
  </div>

  {#if loading && rows.length === 0}
    <div class="loading">
      <div class="spinner"></div>
      <div>{$t('common.loading')}</div>
    </div>
  {:else if rows.length === 0}
    <div class="empty">
      <Icon name="info" size={18} />
      <span>{$t('announcements.empty_feed')}</span>
    </div>
  {:else}
    <div class="summary">
      <div class="count">
        <span class="num">{rows.length}</span>
        <span class="txt">{$t('announcements.list.title')}</span>
      </div>
      <div class="hint">
        {$t('common.updated')}:
        {formatDateTime(new Date().toISOString(), { timeZone: $appSettings.app_timezone })}
      </div>
    </div>

    <div class="feed">
      {#each rows as a, i (a.id)}
        <button
          class="post {a.severity}"
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
              <div class="cover-shade"></div>
            </div>
          {:else}
            <div class="cover fallback">
              <div class="fallback-icon">
                <Icon name={badgeIcon(a.severity)} size={20} />
              </div>
              <div class="cover-shade"></div>
            </div>
          {/if}
          <div class="meta">
            <span class="pill {a.severity}">
              <Icon name={badgeIcon(a.severity)} size={14} />
              <span class="sev">{badgeLabel(a.severity)}</span>
            </span>
            <span class="dot"></span>
            <span class="time">
              {formatDateTime(a.starts_at, { timeZone: $appSettings.app_timezone })}
            </span>
            {#if a.mode === 'banner'}
              <span class="dot"></span>
              <span class="mode">{$t('announcements.modes.banner')}</span>
            {/if}
          </div>
          <div class="title">{a.title}</div>
          <div class="body">{snippet(a.body)}</div>
          <div class="more">
            {$t('announcements.actions.read')}
            <Icon name="arrow-right" size={16} />
          </div>
        </button>
      {/each}
    </div>

    {#if hasMore}
      <div class="load-more">
        <button class="btn" type="button" onclick={loadMore} disabled={loadingMore}>
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
  .page-content {
    padding: 1.5rem;
    max-width: 1100px;
    margin: 0 auto;
  }

  @media (max-width: 640px) {
    .page-content {
      padding: 1rem;
    }

    .filters {
      grid-template-columns: 1fr;
    }

    .filter-group,
    .align-end {
      justify-content: stretch;
    }
  }

  .hero {
    position: relative;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: var(--bg-surface);
    box-shadow: var(--shadow-md);
    margin-bottom: 1rem;
  }

  .hero-bg {
    position: absolute;
    inset: 0;
    background: var(--bg-surface);
    filter: saturate(1.1);
  }

  :global([data-theme='light']) .hero-bg {
    background: var(--bg-surface);
  }

  .hero-inner {
    position: relative;
    padding: 1rem 1.05rem 1.05rem;
    display: grid;
    gap: 0.85rem;
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
  }

  .kicker .dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--color-primary);
    box-shadow: 0 0 0 6px var(--color-primary-subtle);
  }

  .h1 {
    margin-top: 0.25rem;
    font-size: clamp(1.4rem, 2vw, 1.8rem);
    font-weight: 1000;
    letter-spacing: 0.01em;
    color: var(--text-primary);
    line-height: 1.12;
  }

  .sub {
    margin-top: 0.25rem;
    color: var(--text-secondary);
    font-weight: 650;
    max-width: 70ch;
    font-size: 0.9rem;
  }

  .hero-actions {
    display: flex;
    gap: 0.55rem;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .search {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    padding: 0.5rem 0.68rem;
    border-radius: 10px;
    min-width: min(520px, 100%);
    color: var(--text-secondary);
  }

  :global([data-theme='light']) .search {
    border-color: var(--border-color);
    background: var(--bg-tertiary);
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
    min-height: 0;
  }

  .filters {
    display: grid;
    grid-template-columns: minmax(0, 1.35fr) minmax(0, 1fr);
    gap: 0.65rem;
    padding: 0.15rem 0.05rem 0.75rem;
  }

  .filter-group {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    min-width: 0;
  }

  .align-end {
    justify-content: flex-end;
  }

  .filter-clear {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-secondary);
    padding: 0.52rem 0.76rem;
    border-radius: 999px;
    font-weight: 700;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    flex: 0 0 auto;
  }

  .filter-clear:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .summary {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.8rem;
    margin: 0.15rem 0 0.8rem;
  }

  .count {
    display: inline-flex;
    align-items: baseline;
    gap: 0.55rem;
    color: var(--text-secondary);
    font-weight: 850;
  }

  .count .num {
    font-size: 1.25rem;
    font-weight: 1000;
    color: var(--text-primary);
  }

  .hint {
    color: var(--text-secondary);
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
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 0.95rem 1.05rem 1rem;
    background: var(--bg-surface);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
    overflow: hidden;
    transition:
      transform 180ms ease,
      box-shadow 180ms ease,
      border-color 180ms ease;
    animation: rise 420ms ease both;
    animation-delay: var(--d, 0ms);
  }

  .cover {
    margin: -0.95rem -1.05rem 0.9rem;
    height: 160px;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    position: relative;
  }

  :global([data-theme='light']) .cover {
    border-bottom-color: var(--border-color);
    background: var(--bg-tertiary);
  }

  .cover-shade {
    position: absolute;
    inset: 0;
    background: var(--bg-surface);
    opacity: 0.65;
    pointer-events: none;
  }

  :global([data-theme='light']) .cover-shade {
    background: var(--bg-surface);
    opacity: 0.55;
  }

  .cover.fallback {
    display: grid;
    place-items: center;
    background: var(--bg-surface);
  }

  :global([data-theme='light']) .cover.fallback {
    background: var(--bg-surface);
  }

  .fallback-icon {
    width: 52px;
    height: 52px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    display: grid;
    place-items: center;
    color: var(--text-primary);
    box-shadow: var(--shadow-sm);
    position: relative;
    z-index: 2;
  }

  :global([data-theme='light']) .fallback-icon {
    border-color: var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  :global([data-theme='light']) .post {
    background: var(--bg-surface);
  }

  .post:hover {
    border-color: color-mix(in srgb, var(--color-primary) 45%, var(--border-color));
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .post:active {
    transform: translateY(0);
  }

  .post.info:hover {
    border-color: color-mix(in srgb, var(--color-primary) 45%, var(--border-color));
  }
  .post.success:hover {
    border-color: color-mix(in srgb, var(--color-success) 45%, var(--border-color));
  }
  .post.warning:hover {
    border-color: color-mix(in srgb, var(--color-warning) 45%, var(--border-color));
  }
  .post.error:hover {
    border-color: color-mix(in srgb, var(--color-danger) 45%, var(--border-color));
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-weight: 750;
    font-size: 0.85rem;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    padding: 0.28rem 0.55rem;
    border-radius: 999px;
    text-transform: capitalize;
    color: var(--text-primary);
  }

  :global([data-theme='light']) .pill {
    border-color: var(--border-color);
    background: var(--bg-tertiary);
  }

  .pill.info {
    border-color: color-mix(in srgb, var(--color-primary) 28%, var(--border-color));
  }
  .pill.success {
    border-color: color-mix(in srgb, var(--color-success) 28%, var(--border-color));
  }
  .pill.warning {
    border-color: color-mix(in srgb, var(--color-warning) 28%, var(--border-color));
  }
  .pill.error {
    border-color: color-mix(in srgb, var(--color-danger) 28%, var(--border-color));
  }

  .dot {
    width: 4px;
    height: 4px;
    border-radius: 999px;
    background: var(--border-color);
  }

  :global([data-theme='light']) .dot {
    background: var(--border-color);
  }

  .title {
    margin-top: 0.65rem;
    font-size: 1.05rem;
    font-weight: 950;
    color: var(--text-primary);
    line-height: 1.2;
    letter-spacing: 0.01em;
  }

  .body {
    margin-top: 0.45rem;
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
    margin-top: 0.85rem;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    color: var(--color-primary);
    font-weight: 850;
  }

  .empty,
  .loading {
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 1.1rem 1.2rem;
    background: var(--bg-surface);
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 0.65rem;
  }

  .load-more {
    display: flex;
    justify-content: center;
    margin-top: 1rem;
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
