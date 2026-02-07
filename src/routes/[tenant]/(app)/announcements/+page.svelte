<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Announcement } from '$lib/api/client';
  import { t } from 'svelte-i18n';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { stripHtmlToText } from '$lib/utils/sanitizeHtml';

  const forceRemote = import.meta.env.VITE_USE_REMOTE_API === 'true';
  const API_BASE = forceRemote
    ? import.meta.env.VITE_API_URL || 'http://localhost:3000/api'
    : 'http://localhost:3000/api';

  let rows = $state<Announcement[]>([]);
  let loading = $state(true);
  let q = $state('');
  let sev = $state<'all' | 'info' | 'success' | 'warning' | 'error'>('all');
  let mode = $state<'all' | 'post' | 'banner'>('all');

  let filtered = $derived.by(() => {
    const query = q.trim().toLowerCase();
    let r = rows;
    if (sev !== 'all') r = r.filter((a) => a.severity === sev);
    if (mode !== 'all') r = r.filter((a) => (a.mode || 'post') === mode);
    if (query) {
      r = r.filter((a) => {
        const title = String(a.title || '').toLowerCase();
        const body = stripHtmlToText(a.body || '').toLowerCase();
        return title.includes(query) || body.includes(query);
      });
    }
    return r;
  });

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

  async function load() {
    loading = true;
    try {
      rows = await api.announcements.listActive();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
    const onChange = () => void load();
    window.addEventListener('announcements_changed', onChange);
    return () => window.removeEventListener('announcements_changed', onChange);
  });
</script>

<div class="page-content fade-in">
  <section class="hero">
    <div class="hero-bg"></div>
    <div class="hero-inner">
      <div class="hgroup">
        <div class="kicker">
          <span class="dot"></span>
          {$t('announcements.title') || 'Announcements'}
        </div>
        <h1 class="h1">{$t('announcements.title') || 'Announcements'}</h1>
        <div class="sub">
          {$t('announcements.feed_subtitle') ||
            'Product updates, maintenance windows, and important notices.'}
        </div>
      </div>

      <div class="hero-actions">
        <div class="search">
          <Icon name="search" size={16} />
          <input
            class="search-input"
            value={q}
            oninput={(e) => (q = (e.currentTarget as HTMLInputElement).value)}
            placeholder={$t('announcements.search_placeholder') || $t('notifications_page.search_placeholder') || 'Search...'}
          />
        </div>
        <button class="btn" type="button" onclick={load} title={$t('common.refresh') || 'Refresh'}>
          <Icon name="refresh-cw" size={16} />
          {$t('common.refresh') || 'Refresh'}
        </button>
      </div>
    </div>
  </section>

  <div class="filters">
    <div class="chips">
      <button class="chip {sev === 'all' ? 'active' : ''}" type="button" onclick={() => (sev = 'all')}>
        {$t('common.all') || 'All'}
      </button>
      <button class="chip {sev === 'info' ? 'active' : ''}" type="button" onclick={() => (sev = 'info')}>
        {badgeLabel('info')}
      </button>
      <button class="chip {sev === 'success' ? 'active' : ''}" type="button" onclick={() => (sev = 'success')}>
        {badgeLabel('success')}
      </button>
      <button class="chip {sev === 'warning' ? 'active' : ''}" type="button" onclick={() => (sev = 'warning')}>
        {badgeLabel('warning')}
      </button>
      <button class="chip {sev === 'error' ? 'active' : ''}" type="button" onclick={() => (sev = 'error')}>
        {badgeLabel('error')}
      </button>
    </div>

    <div class="chips right">
      <button class="chip {mode === 'all' ? 'active' : ''}" type="button" onclick={() => (mode = 'all')}>
        {$t('common.all') || 'All'}
      </button>
      <button class="chip {mode === 'post' ? 'active' : ''}" type="button" onclick={() => (mode = 'post')}>
        {$t('announcements.modes.post') || 'Post'}
      </button>
      <button class="chip {mode === 'banner' ? 'active' : ''}" type="button" onclick={() => (mode = 'banner')}>
        {$t('announcements.modes.banner') || 'Banner'}
      </button>

      {#if q.trim() || sev !== 'all' || mode !== 'all'}
        <button class="chip subtle" type="button" onclick={clearFilters}>
          <Icon name="x" size={16} />
          {$t('common.clear') || 'Clear'}
        </button>
      {/if}
    </div>
  </div>

  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <div>{$t('common.loading') || 'Loading...'}</div>
    </div>
  {:else if rows.length === 0}
    <div class="empty">
      <Icon name="info" size={18} />
      <span>{$t('announcements.empty_feed') || 'No announcements yet.'}</span>
    </div>
  {:else if filtered.length === 0}
    <div class="empty">
      <Icon name="search" size={18} />
      <span>{$t('common.empty.no_results') || 'No results.'}</span>
    </div>
  {:else}
    <div class="summary">
      <div class="count">
        <span class="num">{filtered.length}</span>
        <span class="txt">{$t('announcements.list.title') || 'Posts'}</span>
      </div>
      <div class="hint">
        {$t('common.updated') || 'Updated'}:
        {formatDateTime(new Date().toISOString(), { timeZone: $appSettings.app_timezone })}
      </div>
    </div>

    <div class="feed">
      {#each filtered as a, i (a.id)}
        <button class="post {a.severity}" type="button" onclick={() => openDetail(a.id)} style={`--d:${i * 55}ms`}>
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
              <span class="mode">{$t('announcements.modes.banner') || 'Banner'}</span>
            {/if}
          </div>
          <div class="title">{a.title}</div>
          <div class="body">{snippet(a.body)}</div>
          <div class="more">
            {$t('announcements.actions.read') || 'Read'}
            <Icon name="arrow-right" size={16} />
          </div>
        </button>
      {/each}
    </div>
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
  }

  .hero {
    position: relative;
    border: 1px solid var(--border-color);
    border-radius: 22px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.02);
    box-shadow: var(--shadow-md);
    margin-bottom: 1rem;
  }

  .hero-bg {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(900px 220px at 10% 0%, rgba(99, 102, 241, 0.28), transparent 60%),
      radial-gradient(900px 220px at 85% 30%, rgba(16, 185, 129, 0.12), transparent 60%),
      radial-gradient(900px 220px at 50% 110%, rgba(245, 158, 11, 0.14), transparent 60%),
      linear-gradient(180deg, rgba(255, 255, 255, 0.03), rgba(255, 255, 255, 0.01));
    filter: saturate(1.1);
  }

  :global([data-theme='light']) .hero-bg {
    background:
      radial-gradient(900px 220px at 10% 0%, rgba(99, 102, 241, 0.18), transparent 60%),
      radial-gradient(900px 220px at 85% 30%, rgba(16, 185, 129, 0.08), transparent 60%),
      radial-gradient(900px 220px at 50% 110%, rgba(245, 158, 11, 0.1), transparent 60%),
      linear-gradient(180deg, rgba(0, 0, 0, 0.02), rgba(0, 0, 0, 0.01));
  }

  .hero-inner {
    position: relative;
    padding: 1.15rem 1.2rem 1.2rem;
    display: grid;
    gap: 1rem;
  }

  .kicker {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-weight: 850;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-size: 0.78rem;
  }

  .kicker .dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: rgba(99, 102, 241, 0.9);
    box-shadow: 0 0 0 6px rgba(99, 102, 241, 0.12);
  }

  .h1 {
    margin-top: 0.25rem;
    font-size: clamp(1.55rem, 2.2vw, 2rem);
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
  }

  .hero-actions {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .search {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(0, 0, 0, 0.18);
    padding: 0.55rem 0.7rem;
    border-radius: 14px;
    min-width: min(520px, 100%);
    color: rgba(255, 255, 255, 0.85);
  }

  :global([data-theme='light']) .search {
    border-color: rgba(0, 0, 0, 0.12);
    background: rgba(255, 255, 255, 0.8);
    color: rgba(0, 0, 0, 0.75);
  }

  .search-input {
    width: 100%;
    background: transparent;
    border: none;
    outline: none;
    color: inherit;
    font-weight: 750;
    font-size: 0.95rem;
    min-height: 0;
  }

  .filters {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.8rem;
    flex-wrap: wrap;
    padding: 0.2rem 0.1rem 0.9rem;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }

  .chips.right {
    justify-content: flex-end;
  }

  .chip {
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-secondary);
    padding: 0.5rem 0.75rem;
    border-radius: 999px;
    font-weight: 850;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }

  :global([data-theme='light']) .chip {
    border-color: rgba(0, 0, 0, 0.12);
    background: rgba(0, 0, 0, 0.03);
  }

  .chip:hover {
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--text-primary);
  }

  .chip.active {
    border-color: rgba(99, 102, 241, 0.55);
    background: rgba(99, 102, 241, 0.18);
    color: var(--text-primary);
  }

  .chip.subtle {
    border-style: dashed;
  }

  .summary {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.8rem;
    margin: 0.2rem 0 0.95rem;
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
    border-radius: 18px;
    padding: 0.95rem 1.05rem 1rem;
    background:
      radial-gradient(1000px 220px at 18% 0%, rgba(255, 255, 255, 0.06), transparent 55%),
      rgba(255, 255, 255, 0.02);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
    overflow: hidden;
    transition: transform 180ms ease, box-shadow 180ms ease, border-color 180ms ease;
    animation: rise 420ms ease both;
    animation-delay: var(--d, 0ms);
  }

  .cover {
    margin: -0.95rem -1.05rem 0.9rem;
    height: 160px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(0, 0, 0, 0.2);
    position: relative;
  }

  :global([data-theme='light']) .cover {
    border-bottom-color: rgba(0, 0, 0, 0.06);
    background: rgba(0, 0, 0, 0.04);
  }

  .cover-shade {
    position: absolute;
    inset: 0;
    background:
      linear-gradient(180deg, rgba(0, 0, 0, 0.05), rgba(0, 0, 0, 0.45)),
      radial-gradient(900px 160px at 25% 0%, rgba(99, 102, 241, 0.22), transparent 60%);
    opacity: 0.65;
    pointer-events: none;
  }

  :global([data-theme='light']) .cover-shade {
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.0), rgba(0, 0, 0, 0.22)),
      radial-gradient(900px 160px at 25% 0%, rgba(99, 102, 241, 0.14), transparent 60%);
    opacity: 0.55;
  }

  .cover.fallback {
    display: grid;
    place-items: center;
    background:
      radial-gradient(800px 180px at 20% 10%, rgba(255, 255, 255, 0.08), transparent 60%),
      radial-gradient(800px 180px at 90% 80%, rgba(99, 102, 241, 0.16), transparent 60%),
      rgba(0, 0, 0, 0.25);
  }

  :global([data-theme='light']) .cover.fallback {
    background:
      radial-gradient(800px 180px at 20% 10%, rgba(0, 0, 0, 0.03), transparent 60%),
      radial-gradient(800px 180px at 90% 80%, rgba(99, 102, 241, 0.1), transparent 60%),
      rgba(0, 0, 0, 0.02);
  }

  .fallback-icon {
    width: 52px;
    height: 52px;
    border-radius: 18px;
    border: 1px solid rgba(255, 255, 255, 0.18);
    background: rgba(0, 0, 0, 0.25);
    display: grid;
    place-items: center;
    color: rgba(255, 255, 255, 0.92);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
    position: relative;
    z-index: 2;
  }

  :global([data-theme='light']) .fallback-icon {
    border-color: rgba(0, 0, 0, 0.1);
    background: rgba(255, 255, 255, 0.7);
    color: rgba(0, 0, 0, 0.75);
  }

  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  :global([data-theme='light']) .post {
    background:
      radial-gradient(1000px 220px at 18% 0%, rgba(0, 0, 0, 0.03), transparent 55%),
      rgba(0, 0, 0, 0.01);
  }

  .post:hover {
    border-color: rgba(99, 102, 241, 0.45);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .post:active {
    transform: translateY(0);
  }

  .post.info:hover {
    border-color: rgba(59, 130, 246, 0.45);
  }
  .post.success:hover {
    border-color: rgba(34, 197, 94, 0.45);
  }
  .post.warning:hover {
    border-color: rgba(245, 158, 11, 0.45);
  }
  .post.error:hover {
    border-color: rgba(239, 68, 68, 0.45);
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
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.06);
    padding: 0.28rem 0.55rem;
    border-radius: 999px;
    text-transform: capitalize;
    color: var(--text-primary);
  }

  :global([data-theme='light']) .pill {
    border-color: rgba(0, 0, 0, 0.12);
    background: rgba(0, 0, 0, 0.04);
  }

  .pill.info {
    border-color: rgba(59, 130, 246, 0.28);
  }
  .pill.success {
    border-color: rgba(34, 197, 94, 0.28);
  }
  .pill.warning {
    border-color: rgba(245, 158, 11, 0.28);
  }
  .pill.error {
    border-color: rgba(239, 68, 68, 0.28);
  }

  .dot {
    width: 4px;
    height: 4px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.22);
  }

  :global([data-theme='light']) .dot {
    background: rgba(0, 0, 0, 0.18);
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
    border-radius: 16px;
    padding: 1.1rem 1.2rem;
    background: rgba(255, 255, 255, 0.02);
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 0.65rem;
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
