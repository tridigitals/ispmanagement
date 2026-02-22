<script lang="ts">
  import {
    registerCustomerByDomain,
    isAuthenticated,
    user,
    logout,
  } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { appLogo } from '$lib/stores/logo';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toast } from '$lib/stores/toast';
  import { isPlatformDomain } from '$lib/utils/domain';
  import { publicApi } from '$lib/api/client';

  let name = '';
  let email = '';
  let password = '';
  let confirmPassword = '';
  let error = '';
  let loading = false;
  let activeField = '';
  let isTauriApp = false;
  let isCustomDomain = false;
  let customerRegistrationEnabled = false;

  // Visibility states
  let showPassword = false;
  let showConfirmPassword = false;

  $: appName = $appSettings.app_name || 'Platform Core';
  $: appDescription =
    $appSettings.app_description ||
    'Enterprise-grade boilerplate built with Rust and SvelteKit. Secure, scalable, and lightweight.';

  // Default policy if store not loaded yet
  $: policy = $appSettings.auth || {
    password_min_length: 8,
    password_require_uppercase: true,
    password_require_number: true,
    password_require_special: false,
  };

  $: if ($appSettings.auth && $appSettings.auth.allow_registration === false) {
    toast.error(
      $t('auth.register.disabled_message') || 'Public registration is currently disabled',
    );
    goto('/login');
  }

  onMount(async () => {
    // @ts-ignore
    isTauriApp = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;
    if (isTauriApp) {
      goto('/login');
      return;
    }

    await Promise.all([appSettings.init(), appLogo.init()]);
    const hostname = window.location.hostname || '';
    const isLocal =
      hostname.includes('localhost') || hostname.includes('127.0.0.1') || hostname.includes('tauri');
    const isMainDomain = isPlatformDomain(hostname);
    isCustomDomain = !isLocal && !isMainDomain;

    if (!isCustomDomain) {
      toast.error(
        $t('auth.register.disabled_message') ||
          'Customer registration is only available from a tenant custom domain.',
      );
      goto('/login');
      return;
    }
    customerRegistrationEnabled = false;
    try {
      const status = await publicApi.getCustomerRegistrationStatusByDomain(hostname);
      customerRegistrationEnabled = status?.enabled === true;
    } catch {
      customerRegistrationEnabled = false;
    }
    if (!customerRegistrationEnabled) {
      toast.error(
        $t('auth.register.disabled_message') ||
          'Customer self registration is disabled for this tenant.',
      );
      goto('/login');
      return;
    }

    if ($isAuthenticated) {
      if ($user?.is_super_admin) {
        goto('/superadmin');
      } else if ($user?.tenant_slug || isCustomDomain) {
        goto('/dashboard');
      } else {
        // Prevent auth redirect loop for users without tenant context
        logout();
        goto('/login');
      }
      return;
    }
  });

  function validatePassword(pwd: string): string | null {
    if (pwd.length < policy.password_min_length) {
      return `Password must be at least ${policy.password_min_length} characters`;
    }
    if (policy.password_require_uppercase && !/[A-Z]/.test(pwd)) {
      return 'Password must contain at least one uppercase letter';
    }
    if (policy.password_require_number && !/[0-9]/.test(pwd)) {
      return 'Password must contain at least one number';
    }
    if (policy.password_require_special && !/[!@#$%^&*()_+\-=[\]{}|;:',.<>?/`~]/.test(pwd)) {
      return 'Password must contain at least one special character';
    }
    return null;
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = '';

    if (!isCustomDomain || !customerRegistrationEnabled) {
      error =
        'Customer registration is only available from a tenant custom domain in web browser.';
      return;
    }

    // Validate passwords match
    if (password !== confirmPassword) {
      error = 'Passwords do not match';
      return;
    }

    // Validate password against policy
    const policyError = validatePassword(password);
    if (policyError) {
      error = policyError;
      return;
    }

    loading = true;

    try {
      const response = await registerCustomerByDomain(email, password, name);
      if (response.token) {
        if (response.user?.is_super_admin) {
          goto('/superadmin');
          return;
        }
        goto('/dashboard');
        return;
      } else if (response.message) {
        toast.success(response.message);
        goto('/login');
        return;
      } else {
        error =
          'Registration succeeded, but login session was not created. Please continue via login page.';
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }
</script>

{#if $appSettings.auth?.allow_registration && isCustomDomain && customerRegistrationEnabled}
  <div class="auth-container">
    <div class="brand-section">
      <div class="brand-content" in:fade={{ duration: 1000 }}>
        <div class="logo-area">
          {#if $appLogo}
            <img src={$appLogo} alt="App Logo" class="app-logo" />
          {:else}
            <Icon name="app" size={48} strokeWidth={1.5} />
          {/if}
          <h1>{appName}</h1>
        </div>
        <p>{appDescription}</p>
      </div>
    </div>

    <div class="form-section">
      <div class="form-wrapper">
        <div class="form-header">
          <h2>{$t('auth.register.title')}</h2>
          <p>{$t('auth.register.subtitle')}</p>
        </div>

        {#if error}
          <div class="alert error" in:fly={{ y: -10 }}>
            {error}
          </div>
        {/if}

        <form on:submit={handleSubmit}>
          <!-- Full Name -->
          <div class="input-group" class:focus={activeField === 'name'}>
            <label for="name">{$t('auth.register.name_label')}</label>
            <div class="field">
              <span class="icon"><Icon name="user" size={18} /></span>
              <input
                type="text"
                id="name"
                bind:value={name}
                on:focus={() => (activeField = 'name')}
                on:blur={() => (activeField = '')}
                placeholder={$t('auth.register.name_placeholder')}
                required
                disabled={loading}
              />
            </div>
          </div>

          <!-- Email -->
          <div class="input-group" class:focus={activeField === 'email'}>
            <label for="email">{$t('auth.register.email_label')}</label>
            <div class="field">
              <span class="icon"><Icon name="mail" size={18} /></span>
              <input
                type="email"
                id="email"
                bind:value={email}
                on:focus={() => (activeField = 'email')}
                on:blur={() => (activeField = '')}
                placeholder={$t('auth.register.email_placeholder')}
                required
                disabled={loading}
              />
            </div>
          </div>

          <!-- Password -->
          <div class="input-group" class:focus={activeField === 'password'}>
            <label for="password">{$t('auth.register.password_label')}</label>
            <div class="field">
              <span class="icon"><Icon name="lock" size={18} /></span>
              <input
                type={showPassword ? 'text' : 'password'}
                id="password"
                bind:value={password}
                on:focus={() => (activeField = 'password')}
                on:blur={() => (activeField = '')}
                placeholder={$t('auth.register.password_placeholder')}
                required
                class="password-input"
                disabled={loading}
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
            <div class="password-hint">
              {$t('auth.validation.min_length', {
                values: { length: policy.password_min_length },
              })}
              {#if policy.password_require_uppercase}, {$t(
                  'auth.validation.require_uppercase',
                )}{/if}
              {#if policy.password_require_number}, {$t('auth.validation.require_number')}{/if}
              {#if policy.password_require_special}, {$t('auth.validation.require_special')}{/if}
            </div>
          </div>

          <!-- Confirm Password -->
          <div class="input-group" class:focus={activeField === 'confirmPassword'}>
            <label for="confirmPassword">{$t('auth.register.confirm_password_label')}</label>
            <div class="field">
              <span class="icon"><Icon name="lock" size={18} /></span>
              <input
                type={showConfirmPassword ? 'text' : 'password'}
                id="confirmPassword"
                bind:value={confirmPassword}
                on:focus={() => (activeField = 'confirmPassword')}
                on:blur={() => (activeField = '')}
                placeholder={$t('auth.register.password_placeholder')}
                required
                class="password-input"
                disabled={loading}
              />
              <button
                type="button"
                class="toggle-password"
                on:click={() => (showConfirmPassword = !showConfirmPassword)}
                tabindex="-1"
              >
                <Icon name={showConfirmPassword ? 'eye-off' : 'eye'} size={18} />
              </button>
            </div>
          </div>

          <button type="submit" class="btn-primary" disabled={loading}>
            {#if loading}
              <div class="spinner"></div>
            {:else}
              {$t('auth.register.submit_button')}
            {/if}
          </button>
        </form>

        <p class="footer-text">
          {$t('auth.register.footer_text')}
          <a href="/login">{$t('auth.register.login_link')}</a>
        </p>
      </div>
    </div>
  </div>
{:else}
  <!-- Loading state or empty while redirecting -->
  <div
    style="display: flex; align-items: center; justify-content: center; height: 100vh; background: var(--bg-primary);"
  >
    <div class="spinner"></div>
  </div>
{/if}

<style>
  .auth-container {
    display: grid;
    grid-template-columns: 1fr 1.2fr;
    min-height: 100vh;
    background: var(--bg-primary);
  }

  /* Reuse brand section styles from login */
  .brand-section {
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4rem;
    position: sticky;
    top: 0;
    height: 100vh;
  }

  .brand-content {
    max-width: 400px;
  }

  .logo-area {
    color: var(--color-primary);
    margin-bottom: 2rem;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .app-logo {
    max-width: 120px;
    max-height: 120px;
    margin-bottom: 1rem;
    object-fit: contain;
  }

  .logo-area h1 {
    margin-top: 1.5rem;
    font-size: 2.5rem;
    font-weight: 800;
    color: var(--text-primary);
    letter-spacing: -1px;
  }

  .brand-content p {
    color: var(--text-secondary);
    font-size: 1.1rem;
    line-height: 1.6;
  }

  .form-section {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    background: var(--bg-primary);
  }

  .form-wrapper {
    width: 100%;
    max-width: 400px; /* Slightly wider for register form */
  }

  .form-header {
    margin-bottom: 2.5rem;
  }

  .form-header h2 {
    font-size: 1.75rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .form-header p {
    color: var(--text-secondary);
    margin-top: 0.25rem;
  }

  .input-group {
    margin-bottom: 1.25rem;
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
    background: var(--bg-tertiary); /* Assumes variable exists or falls back */
    background-color: rgba(255, 255, 255, 0.03); /* Fallback */
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
    background: rgba(99, 102, 241, 0.05);
  }

  .input-group.focus .field .icon {
    color: var(--color-primary);
  }

  .password-hint {
    margin-top: 0.5rem;
    font-size: 0.75rem;
    color: var(--text-muted);
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
    margin-top: 1rem;
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

  @media (max-width: 800px) {
    .auth-container {
      grid-template-columns: 1fr;
    }
    .brand-section {
      display: none;
    }
  }
</style>
