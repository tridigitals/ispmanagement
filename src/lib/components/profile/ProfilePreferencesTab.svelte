<script lang="ts">
  import { t } from 'svelte-i18n';
</script>

<div class="card section fade-in-up">
  <div class="card-header">
    <h2 class="card-title">
      {$t('profile.preferences.title')}
    </h2>
    <p class="card-subtitle">
      {$t('profile.preferences.subtitle')}
    </p>
  </div>

  <div class="setting-item">
    <div class="setting-info">
      <h3>{$t('profile.preferences.dark_mode') || 'Dark interface'}</h3>
      <p>
        {$t('profile.preferences.dark_mode_desc') ||
          'The workspace uses a focused dark interface across all pages.'}
      </p>
    </div>
    <span class="mode-badge">Dark</span>
  </div>
</div>

<style>
  .card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    padding: 2rem;
    box-shadow: var(--shadow-sm);
  }

  .card-header {
    margin-bottom: 2rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border-subtle);
  }

  .card-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 0.25rem;
  }

  .card-subtitle {
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  /* Settings Items */
  .setting-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem 0;
    border-bottom: 1px solid var(--border-subtle);
  }

  .setting-item:last-child {
    border-bottom: none;
  }

  .setting-info h3 {
    font-size: 1rem;
    font-weight: 500;
    margin-bottom: 0.25rem;
    color: var(--text-primary);
  }

  .setting-info p {
    color: var(--text-secondary);
    font-size: 0.875rem;
  }

  .mode-badge {
    display: inline-flex;
    align-items: center;
    min-height: 32px;
    padding: 0.35rem 0.7rem;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    font-size: 0.78rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .fade-in-up {
    animation: fadeInUp 0.4s ease-out;
  }

  @keyframes fadeInUp {
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
