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
  <div class="head">
    <div class="hgroup">
      <div class="h1">{$t('announcements.title') || 'Announcements'}</div>
      <div class="sub">
        {$t('announcements.feed_subtitle') || 'Product updates, maintenance windows, and important notices.'}
      </div>
    </div>
    <div class="actions">
      <button class="btn" type="button" onclick={load} title={$t('common.refresh') || 'Refresh'}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
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
  {:else}
    <div class="feed">
      {#each rows as a (a.id)}
        <button class="post" type="button" onclick={() => openDetail(a.id)}>
          {#if a.cover_file_id}
            <div class="cover">
              <img
                src={`${API_BASE}/storage/files/${a.cover_file_id}/content`}
                alt=""
                loading="lazy"
              />
            </div>
          {/if}
          <div class="meta">
            <span class="pill {a.severity}">
              <Icon name={badgeIcon(a.severity)} size={14} />
              <span class="sev">{a.severity}</span>
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
  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1.25rem;
  }

  .h1 {
    font-size: 1.6rem;
    font-weight: 950;
    letter-spacing: 0.01em;
    color: var(--text-primary);
  }

  .sub {
    margin-top: 0.25rem;
    color: var(--text-secondary);
    font-weight: 650;
    max-width: 60ch;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  .feed {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 1rem;
  }

  .post {
    text-align: left;
    border: 1px solid var(--border-color);
    border-radius: 18px;
    padding: 1rem 1.05rem;
    background:
      radial-gradient(1000px 220px at 18% 0%, rgba(255, 255, 255, 0.06), transparent 55%),
      rgba(255, 255, 255, 0.02);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
    overflow: hidden;
  }

  .cover {
    margin: -1rem -1.05rem 0.85rem;
    height: 140px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(0, 0, 0, 0.2);
  }

  :global([data-theme='light']) .cover {
    border-bottom-color: rgba(0, 0, 0, 0.06);
    background: rgba(0, 0, 0, 0.04);
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
</style>
