<script lang="ts">
  import { login, isAuthenticated, user } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { appLogo } from '$lib/stores/logo';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { get, derived } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { isPlatformDomain } from '$lib/utils/domain';
  import { publicApi } from '$lib/api/public';
  import { getDefaultTenantLandingPath } from '$lib/utils/appLanding';

  let identifier = '';
  let password = '';
  let rememberMe = true;
  let loginMethod: 'email' | 'phone' = 'email';
  let error = '';
  let loading = false;
  let activeField = '';
  let isTauriApp = false;
  let isCustomDomain = false;
  let customerRegistrationEnabled = false;

  let showPassword = false;

  $: appName = $appSettings.app_name || 'Platform Core';
  $: appDescription =
    $appSettings.app_description ||
    'Enterprise-grade boilerplate built with Rust and SvelteKit. Secure, scalable, and lightweight.';

  // Derived store for registration allowed state - secure by default
  const allowRegistration = derived(appSettings, ($s) => $s.auth?.allow_registration === true);

  onMount(async () => {
    // @ts-ignore
    isTauriApp = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;
    await Promise.all([appSettings.init(), appLogo.init()]);
    const currentHost = window.location.hostname;
    const isLocal =
      currentHost === 'localhost' || currentHost === '127.0.0.1' || currentHost.includes('tauri');
    isCustomDomain = !isLocal && !isPlatformDomain(currentHost);
    customerRegistrationEnabled = false;
    if (isCustomDomain) {
      try {
        const status = await publicApi.getCustomerRegistrationStatusByDomain(currentHost);
        customerRegistrationEnabled = status?.enabled === true;
      } catch {
        customerRegistrationEnabled = false;
      }
    }

    if ($isAuthenticated) {
      const u = get(user);
      const slug = u?.tenant_slug;
      const mainDomain = get(appSettings).auth?.main_domain;
      const isMainDomain =
        (mainDomain && currentHost === mainDomain) ||
        currentHost === 'billing.tridigitals.com' ||
        isPlatformDomain(currentHost);

      if (slug) {
        if (currentHost.includes(slug) || isMainDomain) {
          goto(getDefaultTenantLandingPath(u, ''));
        } else {
          goto(getDefaultTenantLandingPath(u, ''));
        }
      } else {
        goto('/dashboard');
      }
    }
  });

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = '';
    loading = true;

    try {
      const response = await login(identifier, password, rememberMe);
      const slug = response.user?.tenant_slug;
      const currentHost = window.location.hostname;
      const mainDomain = $appSettings.auth?.main_domain;
      const isMainDomain =
        (mainDomain && currentHost === mainDomain) ||
        currentHost === 'billing.tridigitals.com' ||
        isPlatformDomain(currentHost);

      if (slug) {
        if (currentHost.includes(slug) || isMainDomain) {
          goto(getDefaultTenantLandingPath(response.user, ''));
        } else {
          goto(getDefaultTenantLandingPath(response.user, ''));
        }
      } else {
        goto('/dashboard'); // Fallback
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }
</script>

<div class="login-container">
  <div class="form-section">
    <div class="form-wrapper">
      <div class="form-header">
        <h2>{$t('auth.login.title')}</h2>
        <p>{$t('auth.login.subtitle')}</p>
      </div>

      {#if error}
        <div class="alert error" in:fly={{ y: -10 }}>
          {error}
        </div>
      {/if}

      <form on:submit={handleSubmit}>
        <!-- Login Method Toggle -->
        <div class="login-method-toggle">
          <button
            type="button"
            class="toggle-btn"
            class:active={loginMethod === 'email'}
            on:click={() => { loginMethod = 'email'; identifier = ''; }}
          >
            <Icon name="mail" size={16} />
            Email
          </button>
          <button
            type="button"
            class="toggle-btn"
            class:active={loginMethod === 'phone'}
            on:click={() => { loginMethod = 'phone'; identifier = ''; }}
          >
            <Icon name="smartphone" size={16} />
            Phone
          </button>
        </div>

        {#if loginMethod === 'email'}
          <div class="input-group" class:focus={activeField === 'email'}>
            <label for="email">{$t('auth.login.email_label')}</label>
            <div class="field">
              <span class="icon"><Icon name="mail" size={18} /></span>
              <input
                type="email"
                id="email"
                bind:value={identifier}
                on:focus={() => (activeField = 'email')}
                on:blur={() => (activeField = '')}
                placeholder={$t('auth.login.email_placeholder')}
                required
              />
            </div>
          </div>
        {:else}
          <div class="input-group" class:focus={activeField === 'phone'}>
            <label for="phone">Phone</label>
            <div class="field">
              <span class="icon"><Icon name="smartphone" size={18} /></span>
              <input
                type="tel"
                id="phone"
                bind:value={identifier}
                on:focus={() => (activeField = 'phone')}
                on:blur={() => (activeField = '')}
                placeholder="08xxxxxxxxxx"
                required
              />
            </div>
          </div>
        {/if}

        <div class="input-group" class:focus={activeField === 'password'}>
          <label for="password">{$t('auth.login.password_label')}</label>
          <div class="field">
            <span class="icon"><Icon name="lock" size={18} /></span>
            <input
              type={showPassword ? 'text' : 'password'}
              id="password"
              bind:value={password}
              on:focus={() => (activeField = 'password')}
              on:blur={() => (activeField = '')}
              placeholder={$t('auth.login.password_placeholder')}
              required
              class="password-input"
            />
            <button
              type="button"
              class="toggle-password"
              on:click={() => (showPassword = !showPassword)}
              tabindex="-1"
            >
              <Icon name={showPassword ? 'eye-off' : 'eye'} size={18} />
            </button>
          </div>
        </div>

        <div class="form-utils">
          <label class="checkbox">
            <input type="checkbox" bind:checked={rememberMe} />
            <span class="checkmark"></span>
            <span>{$t('auth.login.remember_me')}</span>
          </label>
          <a href="/forgot-password">{$t('auth.login.forgot_password')}</a>
        </div>

        <button type="submit" class="btn-primary" disabled={loading}>
          {#if loading}
            <div class="spinner"></div>
          {:else}
            {$t('auth.login.submit_button')}
          {/if}
        </button>
      </form>

      {#if $allowRegistration && !isTauriApp && isCustomDomain && customerRegistrationEnabled}
        <p class="footer-text">
          {$t('auth.login.footer_text')}
          <a href="/register">{$t('auth.login.register_link')}</a>
        </p>
      {/if}
    </div>
  </div>
</div>

<style>
  .login-container {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    background: var(--bg-primary);
  }

  .form-section {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: clamp(1.25rem, 4vw, 2rem);
    width: 100%;
  }

  .form-wrapper {
    width: 100%;
    max-width: 480px;
    background: var(--bg-surface);
    padding: clamp(1.5rem, 4vw, 2.5rem);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    box-shadow: var(--shadow-md);
  }

  .form-header {
    margin-bottom: 2rem;
    text-align: center;
  }

  .form-header h2 {
    font-size: 1.75rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .form-header p {
    color: var(--text-secondary);
    margin-top: 0.5rem;
  }

  /* Login Method Toggle */
  .login-method-toggle {
    display: flex;
    gap: 0;
    margin-bottom: 1.5rem;
    background: var(--bg-tertiary);
    border-radius: 8px;
    padding: 4px;
    border: 1px solid var(--border-color);
  }

  .toggle-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .toggle-btn:hover {
    color: var(--text-primary);
  }

  .toggle-btn.active {
    background: var(--color-primary);
    color: white;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
  }

  .input-group {
    margin-bottom: 1.5rem;
  }

  .input-group label {
    display: block;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
  }

  .field {
    position: relative;
    display: flex;
    align-items: center;
  }

  .field .icon {
    position: absolute;
    left: 1rem;
    color: var(--text-muted);
    transition: color 0.2s;
  }

  .field input {
    width: 100%;
    padding: 0.75rem 1rem 0.75rem 3rem;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 1rem;
    transition: all 0.2s;
  }

  .field input.password-input {
    padding-right: 40px;
  }

  .toggle-password {
    position: absolute;
    right: 10px;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0;
    display: flex;
    align-items: center;
    transition: color 0.2s;
    z-index: 2;
  }

  .toggle-password:hover {
    color: var(--color-primary);
  }

  .input-group.focus .field input {
    border-color: var(--color-primary);
    background: var(--bg-primary);
  }

  .input-group.focus .field .icon {
    color: var(--color-primary);
  }

  .form-utils {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
    font-size: 0.85rem;
  }

  .checkbox {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    color: var(--text-secondary);
  }

  .checkbox input {
    display: none;
  }

  .checkmark {
    width: 16px;
    height: 16px;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    position: relative;
  }

  .checkbox input:checked + .checkmark {
    background: var(--color-primary);
    border-color: var(--color-primary);
  }

  .checkbox input:checked + .checkmark::after {
    content: '';
    position: absolute;
    left: 5px;
    top: 2px;
    width: 3px;
    height: 7px;
    border: solid white;
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
  }

  .form-utils a {
    color: var(--color-primary-light);
    text-decoration: none;
    font-weight: 600;
  }

  .btn-primary {
    width: 100%;
    padding: 0.75rem;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.2s;
    display: flex;
    justify-content: center;
  }

  .btn-primary:hover {
    opacity: 0.9;
  }
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .footer-text {
    text-align: center;
    margin-top: 2rem;
    font-size: 0.9rem;
    color: var(--text-secondary);
  }

  .footer-text a {
    color: var(--text-primary);
    font-weight: 600;
    text-decoration: none;
  }

  .alert {
    padding: 0.75rem;
    border-radius: 8px;
    margin-bottom: 1.5rem;
    font-size: 0.85rem;
    text-align: center;
  }

  .alert.error {
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
    color: var(--color-danger);
    border: 1px solid color-mix(in srgb, var(--color-danger) 22%, var(--border-color));
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-color);
    border-top-color: var(--text-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 480px) {
    .form-wrapper {
      padding: 1.25rem;
    }

    .form-header h2 {
      font-size: 1.4rem;
    }
  }
</style>
