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

<div class="page">
  <div class="page-head">
    <button class="btn btn-secondary" type="button" onclick={goBack}>
      <Icon name="arrow-left" size={16} />
      {backLabel || $t('common.back') || 'Back'}
    </button>
    <div class="crumb">
      <span class="muted">{$t('announcements.title')}</span>
      <span class="sep"></span>
      <span class="muted">{announcement?.title || ''}</span>
    </div>
  </div>

  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <div>{$t('common.loading')}</div>
    </div>
  {:else if !announcement}
    <div class="empty">
      <Icon name="info" size={18} />
      <span>{$t('announcements.not_found')}</span>
    </div>
  {:else}
    <div class="panel head-panel">
      {#if announcement.cover_file_id}
        <img
          class="cover"
          src={`${API_BASE}/storage/files/${announcement.cover_file_id}/content`}
          alt=""
          loading="lazy"
        />
      {/if}
      <div class="meta">
        <span class="pill {announcement.severity}">
          <Icon name={iconForSeverity(announcement.severity)} size={14} />
          <span>{sevLabel(announcement.severity)}</span>
        </span>
        <span class="time">
          {formatDateTime(announcement.starts_at, { timeZone: $appSettings.app_timezone })}
        </span>
        {#if announcement.mode === 'banner'}
          <span class="mode">{$t('announcements.modes.banner')}</span>
        {/if}
      </div>
      <h1 class="title">{announcement.title}</h1>
      <p class="subtitle">{$t('announcements.feed_subtitle')}</p>
    </div>

    <div class="grid">
      <article class="panel post">
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
        <div class="panel">
          <div class="card-title">{$t('common.details')}</div>
          <div class="row">
            <span class="k">{$t('announcements.fields.starts_at')}</span>
            <span class="v"
              >{formatDateTime(announcement.starts_at, {
                timeZone: $appSettings.app_timezone,
              })}</span
            >
          </div>
          <div class="row">
            <span class="k">{$t('announcements.fields.ends_at')}</span>
            <span class="v">
              {announcement.ends_at
                ? formatDateTime(announcement.ends_at, { timeZone: $appSettings.app_timezone })
                : $t('common.na') || '—'}
            </span>
          </div>
          <div class="row">
            <span class="k">{$t('announcements.fields.severity')}</span>
            <span class="v sev {announcement.severity}">{sevLabel(announcement.severity)}</span>
          </div>
        </div>
      </aside>
    </div>
  {/if}
</div>

<style>
  .page {
    padding: clamp(1rem, 2vw, 1.5rem);
    max-width: 1100px;
    margin: 0 auto;
  }

  .page-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
    margin-bottom: 1rem;
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

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.5rem 0.85rem;
    border-radius: var(--radius-md, 8px);
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-weight: 600;
    cursor: pointer;
    min-height: 40px;
  }

  .btn-secondary {
    background: var(--bg-tertiary, var(--bg-surface));
  }

  .panel {
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg, 12px);
    background: var(--bg-surface);
    box-shadow: var(--shadow-sm);
    padding: 1rem;
  }

  .head-panel {
    margin-bottom: 1rem;
    display: grid;
    gap: 0.55rem;
  }

  .cover {
    width: 100%;
    max-height: 180px;
    object-fit: cover;
    border-radius: var(--radius-md, 8px);
    border: 1px solid var(--border-color);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    flex-wrap: wrap;
    color: var(--text-secondary);
    font-weight: 600;
    font-size: 0.9rem;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0.6rem;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    font-size: 0.8rem;
    font-weight: 700;
  }

  .pill.success {
    color: var(--color-success);
    border-color: color-mix(in srgb, var(--color-success) 35%, var(--border-color));
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
  }

  .pill.warning {
    color: var(--color-warning);
    border-color: color-mix(in srgb, var(--color-warning) 35%, var(--border-color));
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
  }

  .pill.error {
    color: var(--color-danger);
    border-color: color-mix(in srgb, var(--color-danger) 35%, var(--border-color));
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
  }

  .pill.info {
    color: var(--color-primary);
    border-color: color-mix(in srgb, var(--color-primary) 35%, var(--border-color));
    background: color-mix(in srgb, var(--color-primary) 12%, transparent);
  }

  .title {
    margin: 0;
    color: var(--text-primary);
    font-size: clamp(1.25rem, 2.2vw, 1.75rem);
    line-height: 1.2;
    font-weight: 800;
  }

  .subtitle {
    margin: 0;
    color: var(--text-secondary);
    max-width: 72ch;
    font-weight: 500;
    font-size: 0.95rem;
  }

  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 280px;
    gap: 1rem;
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

  .card-title {
    font-weight: 800;
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

  @media (max-width: 560px) {
    .page-head {
      flex-direction: column;
      align-items: stretch;
    }

    .btn {
      width: 100%;
      justify-content: center;
    }
  }
</style>
