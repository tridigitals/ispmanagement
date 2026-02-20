<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { api, type Announcement } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toast } from '$lib/stores/toast';
  import { t } from 'svelte-i18n';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { user, tenant } from '$lib/stores/auth';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import { sanitizeHtml } from '$lib/utils/sanitizeHtml';
  import { getApiBaseUrl } from '$lib/utils/apiUrl';

  let loading = $state(true);
  let ann = $state<Announcement | null>(null);

  const id = $derived($page.params.id || '');

  let tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  let tenantPrefix = $derived(tenantCtx.tenantPrefix);

  const API_BASE = getApiBaseUrl();

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

  function sevLabel(sev: string) {
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
  <div class="topbar">
    <button class="btn" type="button" onclick={goBack}>
      <Icon name="arrow-left" size={16} />
      {$t('common.back') || 'Back'}
    </button>
    <div class="crumb">
      <span class="muted">{$t('announcements.title') || 'Announcements'}</span>
      <span class="sep"></span>
      <span class="muted">{ann?.title || ''}</span>
    </div>
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
    <section class="hero {ann.severity}">
      <div class="hero-bg">
        {#if ann.cover_file_id}
          <img
            class="hero-img"
            src={`${API_BASE}/storage/files/${ann.cover_file_id}/content`}
            alt=""
            loading="lazy"
          />
        {/if}
        <div class="hero-shade"></div>
      </div>
      <div class="hero-inner">
        <div class="meta">
          <span class="pill {ann.severity}">
            <Icon name={iconForSeverity(ann.severity)} size={14} />
            <span class="sev">{sevLabel(ann.severity)}</span>
          </span>
          <span class="dot"></span>
          <span class="time">
            {formatDateTime(ann.starts_at, { timeZone: $appSettings.app_timezone })}
          </span>
          {#if ann.mode === 'banner'}
            <span class="dot"></span>
            <span class="mode">{$t('announcements.modes.banner') || 'Banner'}</span>
          {/if}
        </div>
        <h1 class="title">{ann.title}</h1>
        <div class="subtitle">
          {$t('announcements.feed_subtitle') ||
            'Product updates, maintenance windows, and important notices.'}
        </div>
      </div>
    </section>

    <div class="grid">
      <article class="post">
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

      <aside class="rail">
        <div class="card">
          <div class="card-title">{$t('common.details') || 'Details'}</div>
          <div class="row">
            <span class="k">{$t('announcements.fields.starts_at') || 'Starts at'}</span>
            <span class="v"
              >{formatDateTime(ann.starts_at, { timeZone: $appSettings.app_timezone })}</span
            >
          </div>
          <div class="row">
            <span class="k">{$t('announcements.fields.ends_at') || 'Ends at'}</span>
            <span class="v">
              {ann.ends_at
                ? formatDateTime(ann.ends_at, { timeZone: $appSettings.app_timezone })
                : $t('common.na') || '—'}
            </span>
          </div>
          <div class="row">
            <span class="k">{$t('announcements.fields.severity') || 'Severity'}</span>
            <span class="v sev {ann.severity}">{sevLabel(ann.severity)}</span>
          </div>
        </div>

        <!-- Tip card removed to keep detail page clean -->
      </aside>
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

  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
    margin-bottom: 0.9rem;
  }

  .crumb {
    display: inline-flex;
    align-items: center;
    gap: 0.6rem;
    color: var(--text-secondary);
    font-weight: 700;
    max-width: 55ch;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .muted {
    opacity: 0.9;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .crumb .sep {
    width: 4px;
    height: 4px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.22);
    flex: none;
  }

  :global([data-theme='light']) .crumb .sep {
    background: rgba(0, 0, 0, 0.18);
  }

  .hero {
    position: relative;
    border: 1px solid var(--border-color);
    border-radius: 24px;
    overflow: hidden;
    box-shadow: var(--shadow-md);
    margin-bottom: 1rem;
    min-height: 220px;
  }

  .hero-bg {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(1100px 260px at 12% 0%, rgba(99, 102, 241, 0.24), transparent 60%),
      radial-gradient(1100px 260px at 90% 60%, rgba(16, 185, 129, 0.12), transparent 60%),
      rgba(0, 0, 0, 0.25);
  }

  :global([data-theme='light']) .hero-bg {
    background:
      radial-gradient(1100px 260px at 12% 0%, rgba(99, 102, 241, 0.14), transparent 60%),
      radial-gradient(1100px 260px at 90% 60%, rgba(16, 185, 129, 0.08), transparent 60%),
      rgba(0, 0, 0, 0.02);
  }

  .hero-img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: saturate(1.05) contrast(1.02);
  }

  .hero-shade {
    position: absolute;
    inset: 0;
    background:
      linear-gradient(180deg, rgba(0, 0, 0, 0.05), rgba(0, 0, 0, 0.7)),
      radial-gradient(900px 240px at 20% 0%, rgba(99, 102, 241, 0.22), transparent 60%);
  }

  :global([data-theme='light']) .hero-shade {
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0), rgba(0, 0, 0, 0.25)),
      radial-gradient(900px 240px at 20% 0%, rgba(99, 102, 241, 0.14), transparent 60%);
  }

  .hero-inner {
    position: relative;
    padding: 1.15rem 1.2rem 1.25rem;
    color: var(--text-primary);
  }

  .hero .meta {
    color: rgba(255, 255, 255, 0.82);
  }

  :global([data-theme='light']) .hero .meta {
    color: rgba(0, 0, 0, 0.68);
  }

  .hero .pill {
    background: rgba(0, 0, 0, 0.22);
    border-color: rgba(255, 255, 255, 0.18);
  }

  :global([data-theme='light']) .hero .pill {
    background: rgba(255, 255, 255, 0.65);
    border-color: rgba(0, 0, 0, 0.12);
  }

  .subtitle {
    margin-top: 0.55rem;
    color: rgba(255, 255, 255, 0.82);
    font-weight: 700;
    max-width: 70ch;
  }

  :global([data-theme='light']) .subtitle {
    color: rgba(0, 0, 0, 0.68);
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 320px;
    gap: 1rem;
    align-items: start;
  }

  @media (max-width: 980px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }

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
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
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

  .rail {
    position: sticky;
    top: 1rem;
    display: grid;
    gap: 0.9rem;
  }

  @media (max-width: 980px) {
    .rail {
      position: static;
    }
  }

  .card {
    border: 1px solid var(--border-color);
    border-radius: 18px;
    background: rgba(255, 255, 255, 0.02);
    box-shadow: var(--shadow-sm);
    padding: 0.95rem 1rem;
  }

  :global([data-theme='light']) .card {
    background: rgba(0, 0, 0, 0.01);
  }

  .card-title {
    font-weight: 950;
    color: var(--text-primary);
    margin-bottom: 0.75rem;
  }

  .row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.8rem;
    padding: 0.45rem 0;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  :global([data-theme='light']) .row {
    border-top-color: rgba(0, 0, 0, 0.06);
  }

  .row:first-of-type {
    border-top: none;
    padding-top: 0;
  }

  .k {
    color: var(--text-secondary);
    font-weight: 750;
    font-size: 0.88rem;
  }

  .v {
    color: var(--text-primary);
    font-weight: 850;
    text-align: right;
    font-size: 0.9rem;
  }

  .v.sev {
    text-transform: capitalize;
  }

  .v.sev.info {
    color: rgba(59, 130, 246, 0.95);
  }
  .v.sev.success {
    color: rgba(34, 197, 94, 0.95);
  }
  .v.sev.warning {
    color: rgba(245, 158, 11, 0.95);
  }
  .v.sev.error {
    color: rgba(239, 68, 68, 0.95);
  }

  /* Tip card removed */

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
