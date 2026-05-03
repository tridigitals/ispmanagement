<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { appSettings } from '$lib/stores/settings';
  import { isSuperAdmin } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';

  let message =
    get(t)('pages.maintenance.default_message') ||
    "We're updating our systems to serve you better. Please check back soon!";
  let dots = '';
  let interval: any;

  onMount(async () => {
    // Get maintenance settings
    const settings = $appSettings as any;
    const isMaintenanceEnabled =
      settings.maintenance_mode === true || settings.maintenance_mode === 'true';

    // If maintenance mode is not enabled, redirect away from this page
    if (!isMaintenanceEnabled) {
      goto('/login');
      return;
    }

    // If superadmin, redirect to dashboard
    if ($isSuperAdmin) {
      goto('/dashboard');
      return;
    }

    // Get maintenance message from settings
    if (settings.maintenance_message) {
      message = settings.maintenance_message;
    }

    // Animated dots
    interval = setInterval(() => {
      dots = dots.length >= 3 ? '' : dots + '.';
    }, 500);
  });

  onDestroy(() => {
    if (interval) clearInterval(interval);
  });
</script>

<div class="maintenance-container">
  <div class="maintenance-card">
    <div class="maintenance-icon">
      <Icon name="settings" size={34} />
    </div>

    <h1>{$t('pages.maintenance.title') || 'Under Maintenance'}</h1>
    <p class="message">{message}</p>

    <div class="progress-container">
      <div class="progress-bar">
        <div class="progress-fill"></div>
      </div>
      <span class="progress-text">{$t('pages.maintenance.working') || 'Working on it'}{dots}</span>
    </div>

    <p class="footer-text">
      {$t('pages.maintenance.thanks') || "Thank you for your patience. We'll be back shortly!"}
    </p>
  </div>
</div>

<style>
  .maintenance-container {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-primary);
    padding: 1rem;
    position: relative;
    overflow: hidden;
  }

  .maintenance-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: clamp(1.5rem, 5vw, 2.5rem);
    text-align: center;
    max-width: 520px;
    width: 100%;
    box-shadow: var(--shadow-sm);
    position: relative;
    z-index: 1;
  }

  .maintenance-icon {
    width: 68px;
    height: 68px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin: 0 auto 1.5rem;
    color: var(--color-primary);
    background: var(--color-primary-subtle);
    border: 1px solid color-mix(in srgb, var(--color-primary) 24%, var(--border-color));
    border-radius: var(--radius-lg);
  }

  h1 {
    font-size: 2rem;
    font-weight: 700;
    margin: 0 0 1rem 0;
    color: var(--text-primary);
  }

  .message {
    color: var(--text-secondary);
    font-size: 1.1rem;
    line-height: 1.6;
    margin: 0 0 2rem 0;
  }

  .progress-container {
    margin-bottom: 2rem;
  }

  .progress-bar {
    width: 100%;
    height: 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    overflow: hidden;
    margin-bottom: 0.75rem;
  }

  .progress-fill {
    height: 100%;
    background: var(--color-primary);
    background-size: 200% 100%;
    border-radius: 3px;
    animation: shimmer 2s ease-in-out infinite;
  }

  @keyframes shimmer {
    0% {
      background-position: 100% 0;
      width: 20%;
    }
    50% {
      background-position: 0 0;
      width: 80%;
    }
    100% {
      background-position: 100% 0;
      width: 20%;
    }
  }

  .progress-text {
    color: var(--text-tertiary);
    font-size: 0.9rem;
    font-family: monospace;
  }

  .footer-text {
    color: var(--text-muted);
    font-size: 0.85rem;
    margin: 0;
  }

  @media (max-width: 480px) {
    .maintenance-card {
      padding: 2rem 1.5rem;
    }

    h1 {
      font-size: 1.5rem;
    }
  }
</style>
