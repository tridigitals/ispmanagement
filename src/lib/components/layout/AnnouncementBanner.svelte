<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import {
    activeAnnouncements,
    announcementsLoading,
    loadActiveAnnouncements,
    dismissAnnouncement,
  } from '$lib/stores/announcements';
  import { goto } from '$app/navigation';
  import { user } from '$lib/stores/auth';
  import { stripHtmlToText } from '$lib/utils/sanitizeHtml';
  import { hasInternalAppAccess } from '$lib/utils/appLanding';
  import { getAnnouncementDetailPath } from '$lib/utils/announcementRouting';

  let maxVisible = 2;
  const tenantPrefix = '';

  let banners = $derived.by(() =>
    ($activeAnnouncements as any[]).filter((a) => (a?.mode || 'post') === 'banner'),
  );

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

  onMount(() => {
    void loadActiveAnnouncements();

    const onChange = () => void loadActiveAnnouncements();
    window.addEventListener('announcements_changed', onChange);
    return () => window.removeEventListener('announcements_changed', onChange);
  });
</script>

{#if $announcementsLoading && banners.length === 0}
  <div class="wrap loading" aria-live="polite">
    <div class="line"></div>
  </div>
{:else if banners.length}
  <div class="wrap" role="region" aria-label={$t('announcements.banner_aria') || 'Announcements'}>
    {#each banners.slice(0, maxVisible) as a (a.id)}
      <div class="banner {a.severity}">
        <div class="icon">
          <Icon name={iconForSeverity(a.severity)} size={18} />
        </div>
        <div class="content">
          <div class="title">{a.title}</div>
          <div class="body">{stripHtmlToText(a.body)}</div>
        </div>
        <button
          class="read"
          type="button"
          title={$t('announcements.actions.read') || 'Read'}
          onclick={() =>
            goto(
              getAnnouncementDetailPath(a.id, {
                tenantPrefix,
                internal: hasInternalAppAccess($user),
              }),
            )}
        >
          <Icon name="arrow-right" size={18} />
        </button>
        <button
          class="close"
          type="button"
          title={$t('common.close') || 'Close'}
          onclick={() => dismissAnnouncement(a.id)}
        >
          <Icon name="x" size={18} />
        </button>
      </div>
    {/each}

    {#if banners.length > maxVisible}
      <div class="more">
        {get(t)('announcements.more', { values: { count: banners.length - maxVisible } }) ||
          `+${banners.length - maxVisible} more`}
      </div>
    {/if}
  </div>
{/if}

<style>
  .wrap {
    padding: 0.85rem 1rem 0;
    display: grid;
    gap: 0.65rem;
  }

  .wrap.loading {
    padding-top: 0.7rem;
  }

  .line {
    height: 12px;
    border-radius: 999px;
    background: var(--bg-surface);
    background-size: 220% 100%;
    animation: shimmer 1.2s ease-in-out infinite;
    border: 1px solid var(--border-color);
  }

  @keyframes shimmer {
    0% {
      background-position: 0% 0%;
    }
    100% {
      background-position: 100% 0%;
    }
  }

  .banner {
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: var(--bg-surface);
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    gap: 0.75rem;
    align-items: start;
    padding: 0.75rem 0.85rem;
    box-shadow: var(--shadow-sm);
  }

  .banner.info {
    border-color: color-mix(in srgb, var(--color-primary) 22%, var(--border-color));
    background: var(--bg-surface);
  }

  .banner.success {
    border-color: color-mix(in srgb, var(--color-success) 22%, var(--border-color));
    background: var(--bg-surface);
  }

  .banner.warning {
    border-color: color-mix(in srgb, var(--color-warning) 22%, var(--border-color));
    background: var(--bg-surface);
  }

  .banner.error {
    border-color: color-mix(in srgb, var(--color-danger) 22%, var(--border-color));
    background: var(--bg-surface);
  }

  .icon {
    width: 30px;
    height: 30px;
    border-radius: 999px;
    display: grid;
    place-items: center;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
  }

  .content {
    min-width: 0;
  }

  .title {
    font-weight: 950;
    color: var(--text-primary);
    letter-spacing: 0.01em;
    line-height: 1.15;
  }

  .body {
    margin-top: 0.15rem;
    color: var(--text-secondary);
    font-weight: 650;
    font-size: 0.9rem;
    line-height: 1.35;
    display: -webkit-box;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .close {
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border-radius: 10px;
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    cursor: pointer;
  }

  .close:hover {
    background: var(--bg-hover);
  }

  .read {
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border-radius: 10px;
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    cursor: pointer;
  }

  .read:hover {
    background: var(--bg-hover);
  }

  .more {
    color: var(--text-secondary);
    font-weight: 800;
    font-size: 0.85rem;
    padding: 0 0.25rem;
  }
</style>
