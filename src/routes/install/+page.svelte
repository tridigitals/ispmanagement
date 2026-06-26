<script lang="ts">
  import { install } from '$lib/api/install';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';

  let appName = 'SaaS App';
  let appUrl = '';
  let name = '';
  let email = '';
  let password = '';
  let confirmPassword = '';
  let error = '';
  let loading = false;
  let step = 1; // 1: Welcome, 2: General Settings, 3: Account Setup, 4: Success
  let showPassword = false;
  let showConfirmPassword = false;

  onMount(() => {
    appUrl = window.location.origin;

    // Check Status Function
    const checkStatus = async () => {
      try {
        const isInstalled = await install.checkIsInstalled();
        if (isInstalled) {
          goto('/login');
        }
      } catch (e) {
        console.error(e);
      }
    };

    // Initial check
    checkStatus();

    // Poll every 2 seconds
    const interval = setInterval(checkStatus, 2000);

    return () => clearInterval(interval);
  });

  async function handleSubmit() {
    error = '';
    if (!name || !email || !password || !confirmPassword) {
      error = get(t)('install.errors.fill_all') || 'Please fill in all fields';
      return;
    }

    if (password !== confirmPassword) {
      error = get(t)('auth.validation.passwords_do_not_match') || 'Passwords do not match';
      return;
    }

    if (password.length < 8) {
      error =
        get(t)('auth.validation.min_length', { values: { length: 8 } }) ||
        'Password must be at least 8 characters';
      return;
    }

    loading = true;
    try {
      await install.installApp(name, email, password, appName, appUrl);
      step = 4;
      // Delay redirect slightly to show success
      setTimeout(() => {
        goto('/login');
      }, 2000);
    } catch (e: any) {
      error = e.message || get(t)('install.errors.failed') || 'Installation failed';
    } finally {
      loading = false;
    }
  }
</script>

<div class="install-container">
  <div class="card">
    {#if step === 1}
      <div class="step-content">
        <div class="icon-wrapper">
          <Icon name="layers" size={36} />
        </div>
        <h1>
          {$t('install.welcome.title', { values: { app: appName } }) || 'Welcome to SaaS App'}
        </h1>
        <p>
          {$t('install.welcome.subtitle') ||
            "Let's get your application set up. We'll start by configuring the basics."}
        </p>
        <button class="btn-primary" on:click={() => (step = 2)}>
          {$t('install.welcome.cta')}
        </button>
      </div>
    {:else if step === 2}
      <div class="step-content">
        <h2>
          {$t('install.general.title')}
        </h2>
        <p class="subtitle">
          {$t('install.general.subtitle')}
        </p>

        <form on:submit|preventDefault={() => (step = 3)}>
          <div class="form-group">
            <label for="appName">
              {$t('install.general.app_name')}
            </label>
            <input
              type="text"
              id="appName"
              bind:value={appName}
              placeholder={$t('install.general.app_name_placeholder')}
              required
            />
          </div>

          <div class="form-group">
            <label for="appUrl">
              {$t('install.general.app_url')}
            </label>
            <input
              type="text"
              id="appUrl"
              bind:value={appUrl}
              placeholder={$t('install.general.app_url_placeholder')}
              required
            />
          </div>

          <div class="actions">
            <button type="button" class="btn-secondary" on:click={() => (step = 1)}
              >{$t('common.back')}</button
            >
            <button type="submit" class="btn-primary">
              {$t('install.common.next')}
            </button>
          </div>
        </form>
      </div>
    {:else if step === 3}
      <div class="step-content">
        <h2>
          {$t('install.admin.title')}
        </h2>
        <p class="subtitle">
          {$t('install.admin.subtitle')}
        </p>

        {#if error}
          <div class="error-alert">
            {error}
          </div>
        {/if}

        <form on:submit|preventDefault={handleSubmit}>
          <div class="form-group">
            <label for="name">
              {$t('install.admin.full_name')}
            </label>
            <input
              type="text"
              id="name"
              bind:value={name}
              placeholder={$t('install.admin.full_name_placeholder')}
              disabled={loading}
            />
          </div>

          <div class="form-group">
            <label for="email">
              {$t('install.admin.email')}
            </label>
            <input
              type="email"
              id="email"
              bind:value={email}
              placeholder={$t('install.admin.email_placeholder')}
              disabled={loading}
            />
          </div>

          <div class="form-group">
            <label for="password">
              {$t('install.admin.password')}
            </label>
            <div class="password-wrapper">
              <input
                type={showPassword ? 'text' : 'password'}
                id="password"
                bind:value={password}
                placeholder="••••••••"
                disabled={loading}
              />
              <button
                type="button"
                class="eye-btn"
                on:click={() => (showPassword = !showPassword)}
              tabindex="-1"
              >
                <Icon name={showPassword ? 'eye-off' : 'eye'} size={18} />
              </button>
            </div>
          </div>

          <div class="form-group">
            <label for="confirmPassword">
              {$t('install.admin.confirm_password')}
            </label>
            <div class="password-wrapper">
              <input
                type={showConfirmPassword ? 'text' : 'password'}
                id="confirmPassword"
                bind:value={confirmPassword}
                placeholder="••••••••"
                disabled={loading}
              />
              <button
                type="button"
                class="eye-btn"
                on:click={() => (showConfirmPassword = !showConfirmPassword)}
              tabindex="-1"
              >
                <Icon name={showConfirmPassword ? 'eye-off' : 'eye'} size={18} />
              </button>
            </div>
          </div>

          <div class="actions">
            <button
              type="button"
              class="btn-secondary"
              on:click={() => (step = 2)}
              disabled={loading}>{$t('common.back')}</button
            >
            <button type="submit" class="btn-primary" disabled={loading}>
              {#if loading}
                {$t('install.admin.installing')}
              {:else}
                {$t('install.admin.complete')}
              {/if}
            </button>
          </div>
        </form>
      </div>
    {:else if step === 4}
      <div class="step-content success">
        <div class="success-icon">
          <Icon name="check-circle" size={44} />
        </div>
        <h2>
          {$t('install.success.title')}
        </h2>
        <p>
          {$t('install.success.redirecting')}
        </p>
      </div>
    {/if}
  </div>
</div>

<style>
  .install-container {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-primary);
    padding: 1rem;
  }

  .card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: clamp(1.5rem, 5vw, 2.5rem);
    width: 100%;
    max-width: 480px;
    box-shadow: var(--shadow-sm);
  }

  .step-content {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    text-align: center;
  }

  .icon-wrapper {
    width: 80px;
    height: 80px;
    background: var(--bg-tertiary);
    border-radius: var(--radius-lg);
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0 auto;
    color: var(--color-primary);
  }

  h1,
  h2 {
    color: var(--text-primary);
    margin: 0;
  }

  h1 {
    font-size: 1.75rem;
  }
  h2 {
    font-size: 1.5rem;
  }

  p {
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.6;
  }

  .subtitle {
    font-size: 0.95rem;
    margin-bottom: 0.5rem;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    text-align: left;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  label {
    font-size: 0.9rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  input {
    padding: 0.75rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    font-size: 1rem;
    transition: all 0.2s;
  }

  input:focus {
    border-color: var(--color-primary);
    outline: none;
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .actions {
    display: flex;
    gap: 1rem;
    margin-top: 0.5rem;
  }

  button {
    flex: 1;
    padding: 0.75rem;
    border-radius: var(--radius-md);
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    border: none;
  }

  .btn-primary {
    background: var(--color-primary);
    color: var(--bg-app);
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  button:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  .error-alert {
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-danger) 22%, var(--border-color));
    color: var(--color-danger);
    padding: 0.75rem;
    border-radius: var(--radius-md);
    font-size: 0.9rem;
  }

  .success {
    padding: 2rem 0;
  }

  .success-icon {
    color: var(--text-success);
    margin-bottom: 1rem;
  }

  .password-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
  }

  .password-wrapper input {
    width: 100%;
    padding-right: 2.5rem;
  }

  .eye-btn {
    position: absolute;
    right: 0.5rem;
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 0.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.2s;
    z-index: 10;
  }

  .eye-btn:hover {
    color: var(--text-primary);
  }
</style>
