<script lang="ts">
  import { install } from '$lib/api/client';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
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
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="48"
            height="48"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            ><path d="M12 2L2 7l10 5 10-5-10-5z" /><path d="M2 17l10 5 10-5" /><path
              d="M2 12l10 5 10-5"
            /></svg
          >
        </div>
        <h1>
          {$t('install.welcome.title', { values: { app: appName } }) || 'Welcome to SaaS App'}
        </h1>
        <p>
          {$t('install.welcome.subtitle') ||
            "Let's get your application set up. We'll start by configuring the basics."}
        </p>
        <button class="btn-primary" on:click={() => (step = 2)}>
          {$t('install.welcome.cta') || 'Get Started'}
        </button>
      </div>
    {:else if step === 2}
      <div class="step-content">
        <h2>
          {$t('install.general.title') || 'General Settings'}
        </h2>
        <p class="subtitle">
          {$t('install.general.subtitle') || 'Configure your application details.'}
        </p>

        <form on:submit|preventDefault={() => (step = 3)}>
          <div class="form-group">
            <label for="appName">
              {$t('install.general.app_name') || 'Application Name'}
            </label>
            <input
              type="text"
              id="appName"
              bind:value={appName}
              placeholder={$t('install.general.app_name_placeholder') || 'My SaaS App'}
              required
            />
          </div>

          <div class="form-group">
            <label for="appUrl">
              {$t('install.general.app_url') || 'Application URL'}
            </label>
            <input
              type="text"
              id="appUrl"
              bind:value={appUrl}
              placeholder={$t('install.general.app_url_placeholder') || 'http://localhost:3000'}
              required
            />
          </div>

          <div class="actions">
            <button type="button" class="btn-secondary" on:click={() => (step = 1)}
              >{$t('common.back') || 'Back'}</button
            >
            <button type="submit" class="btn-primary">
              {$t('install.common.next') || 'Next'}
            </button>
          </div>
        </form>
      </div>
    {:else if step === 3}
      <div class="step-content">
        <h2>
          {$t('install.admin.title') || 'Create Admin Account'}
        </h2>
        <p class="subtitle">
          {$t('install.admin.subtitle') || 'This account will have full access to the system.'}
        </p>

        {#if error}
          <div class="error-alert">
            {error}
          </div>
        {/if}

        <form on:submit|preventDefault={handleSubmit}>
          <div class="form-group">
            <label for="name">
              {$t('install.admin.full_name') || 'Full Name'}
            </label>
            <input
              type="text"
              id="name"
              bind:value={name}
              placeholder={$t('install.admin.full_name_placeholder') || 'John Doe'}
              disabled={loading}
            />
          </div>

          <div class="form-group">
            <label for="email">
              {$t('install.admin.email') || 'Email Address'}
            </label>
            <input
              type="email"
              id="email"
              bind:value={email}
              placeholder={$t('install.admin.email_placeholder') || 'john@example.com'}
              disabled={loading}
            />
          </div>

          <div class="form-group">
            <label for="password">
              {$t('install.admin.password') || 'Password'}
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
                {#if showPassword}
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    ><path
                      d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"
                    /><line x1="1" y1="1" x2="23" y2="23" /></svg
                  >
                {:else}
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    ><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle
                      cx="12"
                      cy="12"
                      r="3"
                    /></svg
                  >
                {/if}
              </button>
            </div>
          </div>

          <div class="form-group">
            <label for="confirmPassword">
              {$t('install.admin.confirm_password') || 'Confirm Password'}
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
                {#if showConfirmPassword}
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    ><path
                      d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"
                    /><line x1="1" y1="1" x2="23" y2="23" /></svg
                  >
                {:else}
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    ><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle
                      cx="12"
                      cy="12"
                      r="3"
                    /></svg
                  >
                {/if}
              </button>
            </div>
          </div>

          <div class="actions">
            <button
              type="button"
              class="btn-secondary"
              on:click={() => (step = 2)}
              disabled={loading}>{$t('common.back') || 'Back'}</button
            >
            <button type="submit" class="btn-primary" disabled={loading}>
              {#if loading}
                {$t('install.admin.installing') || 'Installing...'}
              {:else}
                {$t('install.admin.complete') || 'Complete Setup'}
              {/if}
            </button>
          </div>
        </form>
      </div>
    {:else if step === 4}
      <div class="step-content success">
        <div class="success-icon">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="48"
            height="48"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            ><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" /><polyline
              points="22 4 12 14.01 9 11.01"
            /></svg
          >
        </div>
        <h2>
          {$t('install.success.title') || 'Installation Complete!'}
        </h2>
        <p>
          {$t('install.success.redirecting') || 'Redirecting you to login...'}
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
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 1rem;
    padding: 2.5rem;
    width: 100%;
    max-width: 480px;
    box-shadow: var(--shadow-lg);
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
    border-radius: 50%;
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
    border-radius: 0.5rem;
    border: 1px solid var(--border-primary);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    font-size: 1rem;
    transition: all 0.2s;
  }

  input:focus {
    border-color: var(--color-primary);
    outline: none;
    box-shadow: 0 0 0 2px var(--color-primary-transparent);
  }

  .actions {
    display: flex;
    gap: 1rem;
    margin-top: 0.5rem;
  }

  button {
    flex: 1;
    padding: 0.75rem;
    border-radius: 0.5rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    border: none;
  }

  .btn-primary {
    background: var(--color-primary);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--border-primary);
  }

  button:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  .error-alert {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    color: #ef4444;
    padding: 0.75rem;
    border-radius: 0.5rem;
    font-size: 0.9rem;
  }

  .success {
    padding: 2rem 0;
  }

  .success-icon {
    color: #10b981;
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
