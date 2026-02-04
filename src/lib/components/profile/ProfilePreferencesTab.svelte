<script lang="ts">
  import { t } from 'svelte-i18n';

  let { theme, onToggleTheme } = $props();
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
      <h3>{$t('profile.preferences.dark_mode')}</h3>
      <p>{$t('profile.preferences.dark_mode_desc')}</p>
    </div>
    <label class="toggle">
      <input type="checkbox" checked={theme === 'dark'} onchange={onToggleTheme} />
      <span class="slider"></span>
    </label>
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

  /* Toggle Switch */
  .toggle {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
  }

  .toggle input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--bg-active);
    transition: 0.3s;
    border-radius: 24px;
  }

  .slider:before {
    position: absolute;
    content: '';
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 3px;
    background-color: white;
    transition: 0.3s;
    border-radius: 50%;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  input:checked + .slider {
    background-color: var(--color-primary);
  }

  input:checked + .slider:before {
    transform: translateX(20px);
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
