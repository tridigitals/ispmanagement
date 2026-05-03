<script lang="ts">
  import { goto } from '$app/navigation';
  import { user } from '$lib/stores/auth';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { t } from 'svelte-i18n';
</script>

<div class="unauthorized-page fade-in">
  <div class="content">
    <div class="lock-icon">
      <Icon name="shield-alert" size={32} />
    </div>
    <div class="error-code">403</div>
    <h1>{$t('pages.unauthorized.title') || 'Access Denied'}</h1>
    <p class="message">
      {$t('pages.unauthorized.sorry') || 'Sorry,'}
      <strong>{$user?.name || $t('profile.fallback.user') || 'User'}</strong>.
      {$t('pages.unauthorized.message') ||
        "You don't have the required permissions to view this page. This area is restricted to system administrators."}
    </p>

    <div class="actions">
      <button class="btn btn-primary" on:click={() => goto('/dashboard')}>
        {$t('pages.unauthorized.back') || 'Return to Safety'}
      </button>
    </div>
  </div>
</div>

<style>
  .unauthorized-page {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: clamp(1rem, 4vw, 2rem);
    text-align: center;
    background: var(--bg-primary);
  }

  .content {
    max-width: 500px;
    padding: clamp(1.5rem, 5vw, 2.5rem);
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
  }

  .lock-icon {
    width: 64px;
    height: 64px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 1rem;
    border-radius: var(--radius-lg);
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-danger) 22%, var(--border-color));
  }

  .error-code {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-danger);
    margin-bottom: 0.5rem;
    letter-spacing: 0;
  }

  h1 {
    font-size: 2rem;
    margin-bottom: 1rem;
    color: var(--text-primary);
  }

  .message {
    font-size: 1rem;
    color: var(--text-secondary);
    margin-bottom: 2rem;
    line-height: 1.6;
  }

  .message strong {
    color: var(--text-primary);
  }

  .actions {
    display: flex;
    gap: 1rem;
    justify-content: center;
  }

  .btn-primary {
    color: var(--bg-app);
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: scale(0.95);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .fade-in {
    animation: fadeIn 0.4s ease-out;
  }
</style>
