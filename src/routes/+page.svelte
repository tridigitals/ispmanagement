<script lang="ts">
  import { login, isAuthenticated, isAdmin, user } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { appLogo } from '$lib/stores/logo';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { get, derived } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';

  let email = '';
  let password = '';
  let rememberMe = true;
  let error = '';
  let loading = false;
  let activeField = '';
  let isTauriApp = false;

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

    if ($isAuthenticated) {
      const u = get(user);
      const slug = u?.tenant_slug;

      if (slug) {
        if (get(isAdmin)) {
          goto(`/${slug}/admin`);
        } else {
          goto(`/${slug}/dashboard`);
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
      const response = await login(email, password, rememberMe);
      const slug = response.user?.tenant_slug;

      if (slug) {
        // If the current domain already matches the user's tenant slug, avoid adding it to the path
        const currentSlug = $page.url.pathname.split('/')[1] || '';
        // OR better yet, check against domain map if we had one client side
        // For now, simpler check:
        if ($page.url.hostname.includes(slug)) {
          // Check if we are ALREADY on the tenant domain
          if (response.user.role === 'admin') {
            goto(`/admin`);
          } else {
            goto(`/dashboard`);
          }
        } else {
          if (response.user.role === 'admin') {
            goto(`/${slug}/admin`);
          } else {
            goto(`/${slug}/dashboard`);
          }
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
        <div class="input-group" class:focus={activeField === 'email'}>
          <label for="email">{$t('auth.login.email_label')}</label>
          <div class="field">
            <span class="icon"><Icon name="mail" size={18} /></span>
            <input
              type="email"
              id="email"
              bind:value={email}
              on:focus={() => (activeField = 'email')}
              on:blur={() => (activeField = '')}
              placeholder={$t('auth.login.email_placeholder')}
              required
            />
          </div>
        </div>

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

      {#if $allowRegistration && !isTauriApp}
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
    background: rgba(239, 68, 68, 0.1);
    color: #fca5a5;
    border: 1px solid rgba(239, 68, 68, 0.2);
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: white;
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
