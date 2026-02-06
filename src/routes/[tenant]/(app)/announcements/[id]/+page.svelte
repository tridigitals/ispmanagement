<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { api, type Announcement } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toast } from '$lib/stores/toast';
  import { t } from 'svelte-i18n';
  import { getSlugFromDomain } from '$lib/utils/domain';
  import { user, tenant } from '$lib/stores/auth';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import { sanitizeHtml } from '$lib/utils/sanitizeHtml';

  let loading = $state(true);
  let ann = $state<Announcement | null>(null);

  const id = $derived($page.params.id || '');

  let domainSlug = $derived(getSlugFromDomain($page.url.hostname));
  let effectiveTenantSlug = $derived($tenant?.slug || $user?.tenant_slug || '');
  let isCustomDomain = $derived(domainSlug && domainSlug === effectiveTenantSlug);
  let tenantPrefix = $derived(
    effectiveTenantSlug && !isCustomDomain ? `/${effectiveTenantSlug}` : '',
  );

  const forceRemote = import.meta.env.VITE_USE_REMOTE_API === 'true';
  const API_BASE = forceRemote
    ? import.meta.env.VITE_API_URL || 'http://localhost:3000/api'
    : 'http://localhost:3000/api';

  function iconForSeverity(sev: string) {
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

  function goBack() {
    goto(`${tenantPrefix}/announcements`);
  }

  async function load() {
    loading = true;
    try {
      if (!id) return;
      ann = await api.announcements.get(id);
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
  });
</script>

<div class="page-content fade-in">
  <div class="head">
    <button class="btn" type="button" onclick={goBack}>
      <Icon name="arrow-left" size={16} />
      {$t('common.back') || 'Back'}
    </button>
  </div>

  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <div>{$t('common.loading') || 'Loading...'}</div>
    </div>
  {:else if !ann}
    <div class="empty">
      <Icon name="info" size={18} />
      <span>{$t('announcements.not_found') || 'Announcement not found.'}</span>
    </div>
  {:else}
    <article class="post {ann.severity}">
      {#if ann.cover_file_id}
        <div class="cover">
          <img
            src={`${API_BASE}/storage/files/${ann.cover_file_id}/content`}
            alt=""
            loading="lazy"
          />
        </div>
      {/if}
      <header class="top">
        <div class="meta">
          <span class="pill {ann.severity}">
            <Icon name={iconForSeverity(ann.severity)} size={14} />
            <span class="sev">{ann.severity}</span>
          </span>
          <span class="dot"></span>
          <span class="time">
            {formatDateTime(ann.starts_at, { timeZone: $appSettings.app_timezone })}
          </span>
          {#if ann.ends_at}
            <span class="dot"></span>
            <span class="time">
              {$t('announcements.ends') || 'Ends'}:
              {formatDateTime(ann.ends_at, { timeZone: $appSettings.app_timezone })}
            </span>
          {/if}
        </div>

        <h1 class="title">{ann.title}</h1>
      </header>

      {#if ann.format === 'html'}
        <div class="body prose">
          {@html sanitizeHtml(ann.body)}
        </div>
      {:else}
        <div class="body" class:mono={ann.format === 'plain'}>
          {ann.body}
        </div>
      {/if}
    </article>
  {/if}
</div>

<style>
  .post {
    border: 1px solid var(--border-color);
    border-radius: 20px;
    padding: 1.25rem 1.25rem 1.35rem;
    background:
      radial-gradient(1200px 280px at 15% 0%, rgba(255, 255, 255, 0.06), transparent 60%),
      rgba(255, 255, 255, 0.02);
    box-shadow: var(--shadow-md);
    overflow: hidden;
  }

  .cover {
    margin: -1.25rem -1.25rem 1rem;
    height: 220px;
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
      radial-gradient(1200px 280px at 15% 0%, rgba(0, 0, 0, 0.03), transparent 60%),
      rgba(0, 0, 0, 0.01);
  }

  .post.info {
    border-color: rgba(59, 130, 246, 0.22);
  }
  .post.success {
    border-color: rgba(34, 197, 94, 0.22);
  }
  .post.warning {
    border-color: rgba(245, 158, 11, 0.22);
  }
  .post.error {
    border-color: rgba(239, 68, 68, 0.22);
  }

  .top {
    margin-bottom: 0.95rem;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.55rem;
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
    margin-top: 0.75rem;
    font-size: clamp(1.35rem, 1.7vw, 1.75rem);
    font-weight: 1000;
    letter-spacing: 0.01em;
    line-height: 1.15;
    color: var(--text-primary);
  }

  .body {
    margin-top: 0.85rem;
    color: var(--text-primary);
    font-weight: 650;
    line-height: 1.6;
    font-size: 1rem;
    white-space: pre-wrap;
  }

  .prose :global(p) {
    margin: 0.85rem 0;
  }

  .prose :global(ul),
  .prose :global(ol) {
    margin: 0.75rem 0;
    padding-left: 1.2rem;
  }

  .prose :global(li) {
    margin: 0.3rem 0;
  }

  .prose :global(blockquote) {
    margin: 0.9rem 0;
    padding: 0.75rem 0.9rem;
    border-left: 3px solid rgba(99, 102, 241, 0.55);
    background: rgba(99, 102, 241, 0.08);
    border-radius: 12px;
    color: var(--text-primary);
  }

  .prose :global(pre) {
    margin: 0.9rem 0;
    padding: 0.85rem 0.95rem;
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(0, 0, 0, 0.35);
    overflow: auto;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
  }

  :global([data-theme='light']) .prose :global(pre) {
    border-color: rgba(0, 0, 0, 0.1);
    background: rgba(0, 0, 0, 0.06);
  }

  .prose :global(a) {
    color: var(--color-primary);
    font-weight: 800;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .body.mono {
    font-variant-ligatures: none;
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
