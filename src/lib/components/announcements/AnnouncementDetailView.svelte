<script lang="ts">
  import type { Announcement } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import { sanitizeHtml } from '$lib/utils/sanitizeHtml';
  import { getApiBaseUrl } from '$lib/utils/apiUrl';

  let {
    announcement = null,
    loading = false,
    backHref = '',
    backLabel = '',
  }: {
    announcement: Announcement | null;
    loading: boolean;
    backHref: string;
    backLabel?: string;
  } = $props();

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
    goto(backHref);
  }
</script>

<div class="page-content fade-in">
  <div class="topbar">
    <button class="btn" type="button" onclick={goBack}>
      <Icon name="arrow-left" size={16} />
      {backLabel || $t('common.back') || 'Back'}
    </button>
    <div class="crumb">
      <span class="muted">{$t('announcements.title') || 'Announcements'}</span>
      <span class="sep"></span>
      <span class="muted">{announcement?.title || ''}</span>
    </div>
  </div>

  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <div>{$t('common.loading') || 'Loading...'}</div>
    </div>
  {:else if !announcement}
    <div class="empty">
      <Icon name="info" size={18} />
      <span>{$t('announcements.not_found') || 'Announcement not found.'}</span>
    </div>
  {:else}
    <section class="hero {announcement.severity}">
      <div class="hero-bg">
        {#if announcement.cover_file_id}
          <img
            class="hero-img"
            src={`${API_BASE}/storage/files/${announcement.cover_file_id}/content`}
            alt=""
            loading="lazy"
          />
        {/if}
        <div class="hero-shade"></div>
      </div>
      <div class="hero-inner">
        <div class="meta">
          <span class="pill {announcement.severity}">
            <Icon name={iconForSeverity(announcement.severity)} size={14} />
            <span class="sev">{sevLabel(announcement.severity)}</span>
          </span>
          <span class="dot"></span>
          <span class="time">
            {formatDateTime(announcement.starts_at, { timeZone: $appSettings.app_timezone })}
          </span>
          {#if announcement.mode === 'banner'}
            <span class="dot"></span>
            <span class="mode">{$t('announcements.modes.banner') || 'Banner'}</span>
          {/if}
        </div>
        <h1 class="title">{announcement.title}</h1>
        <div class="subtitle">
          {$t('announcements.feed_subtitle') ||
            'Product updates, maintenance windows, and important notices.'}
        </div>
      </div>
    </section>

    <div class="grid">
      <article class="post">
        {#if announcement.format === 'html'}
          <div class="body prose">
            {@html sanitizeHtml(announcement.body)}
          </div>
        {:else}
          <div class="body" class:mono={announcement.format === 'plain'}>
            {announcement.body}
          </div>
        {/if}
      </article>

      <aside class="rail">
        <div class="card">
          <div class="card-title">{$t('common.details') || 'Details'}</div>
          <div class="row">
            <span class="k">{$t('announcements.fields.starts_at') || 'Starts at'}</span>
            <span class="v"
              >{formatDateTime(announcement.starts_at, {
                timeZone: $appSettings.app_timezone,
              })}</span
            >
          </div>
          <div class="row">
            <span class="k">{$t('announcements.fields.ends_at') || 'Ends at'}</span>
            <span class="v">
              {announcement.ends_at
                ? formatDateTime(announcement.ends_at, { timeZone: $appSettings.app_timezone })
                : $t('common.na') || '—'}
            </span>
          </div>
          <div class="row">
            <span class="k">{$t('announcements.fields.severity') || 'Severity'}</span>
            <span class="v sev {announcement.severity}">{sevLabel(announcement.severity)}</span>
          </div>
        </div>
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
    background: var(--border-color);
    flex: none;
  }

  :global([data-theme='light']) .crumb .sep {
    background: var(--border-color);
  }

  .hero {
    position: relative;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: var(--shadow-md);
    margin-bottom: 1rem;
    min-height: 220px;
  }

  .hero-bg {
    position: absolute;
    inset: 0;
    z-index: 0;
  }

  .hero-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0.6;
  }

  .hero-shade {
    position: absolute;
    inset: 0;
    background: var(--bg-surface);
  }

  .hero-inner {
    position: relative;
    z-index: 1;
    padding: 1.15rem 1.15rem 1.2rem;
    display: flex;
    min-height: 220px;
    flex-direction: column;
    justify-content: flex-end;
    gap: 0.55rem;
  }

  .hero.info {
    background: var(--bg-surface);
  }

  .hero.success {
    background: var(--bg-surface);
  }

  .hero.warning {
    background: var(--bg-surface);
  }

  .hero.error {
    background: var(--bg-surface);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    flex-wrap: wrap;
    color: var(--text-primary);
    font-weight: 700;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.4rem 0.65rem;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
  }

  .dot {
    width: 4px;
    height: 4px;
    border-radius: 999px;
    background: var(--border-color);
  }

  .title {
    margin: 0;
    color: var(--text-primary);
    font-size: clamp(1.6rem, 2.8vw, 2.5rem);
    line-height: 1.1;
    font-weight: 900;
    letter-spacing: 0.01em;
  }

  .subtitle {
    color: var(--text-secondary);
    max-width: 72ch;
    font-weight: 600;
  }

  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 280px;
    gap: 1rem;
  }

  .post {
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    background: var(--bg-surface);
    box-shadow: var(--shadow-sm);
    padding: 1rem 1rem 1.1rem;
  }

  .body {
    white-space: pre-wrap;
    line-height: 1.7;
    color: var(--text-primary);
    font-size: 1rem;
  }

  .body.mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  }

  .rail {
    display: grid;
    gap: 0.8rem;
    align-self: start;
  }

  .card {
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    background: var(--bg-surface);
    box-shadow: var(--shadow-sm);
    padding: 0.95rem;
  }

  .card-title {
    font-weight: 900;
    margin-bottom: 0.7rem;
    color: var(--text-primary);
  }

  .row {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.55rem 0;
    border-top: 1px dashed var(--border-color);
  }

  .row:first-of-type {
    border-top: none;
    padding-top: 0;
  }

  .k {
    color: var(--text-secondary);
    font-weight: 700;
  }

  .v {
    color: var(--text-primary);
    text-align: right;
  }

  .sev.success {
    color: var(--color-success);
  }

  .sev.warning {
    color: var(--color-warning);
  }

  .sev.error {
    color: var(--color-danger);
  }

  .sev.info {
    color: var(--color-primary);
  }

  .loading,
  .empty {
    display: grid;
    place-items: center;
    min-height: 200px;
    gap: 0.6rem;
    color: var(--text-secondary);
  }

  .spinner {
    width: 24px;
    height: 24px;
    border-radius: 999px;
    border: 2px solid var(--border-color);
    border-top-color: var(--color-primary);
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 900px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
