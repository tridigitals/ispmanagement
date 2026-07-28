<script lang="ts">
  import { login, isAuthenticated, user, token } from '$lib/stores/auth';
  import { auth as authApi } from '$lib/api/auth';
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
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { publicApi } from '$lib/api/public';
  import { getDefaultTenantLandingPath } from '$lib/utils/appLanding';
  import { toast } from '$lib/stores/toast';

  let identifier = '';
  let password = '';
  let rememberMe = true;
  let error = '';
  let loading = false;
  let activeField = '';

  // 2FA State
  let step = 'login'; // 'login' | '2fa-select' | '2fa-totp' | '2fa-email' | '2fa-setup'
  // State to track 2FA input
  let twoFactorCode = '';
  let tempToken = '';
  let available2FAMethods: string[] = [];
  let selected2FAMethod = '';
  let emailOtpSent = false;
  let emailOtpSending = false;

  // 2FA Setup (forced enrollment) state
  let setupMethod = 'totp'; // 'totp' | 'email'
  let setupQr = '';
  let setupSecret = '';
  let setupCode = '';
  let setupLoading = false;
  let setupEmailSent = false;

  let isTauriApp = false;
  let isCustomDomain = false;
  let customerRegistrationEnabled = false;

  let showPassword = false;

  function normalizeHost(value: string | null | undefined): string {
    return String(value || '')
      .trim()
      .toLowerCase()
      .replace(/^https?:\/\//, '')
      .replace(/\/+$/, '')
      .replace(/\.+$/, '');
  }

  function resolvePlatformHost(): string {
    return normalizeHost($appSettings.auth?.main_domain) || normalizeHost((import.meta.env.FALLBACK_PLATFORM_DOMAIN as string) || '');
  }

  function redirectToHost(host: string, path: string) {
    const cleanHost = normalizeHost(host);
    const cleanPath = path.startsWith('/') ? path : `/${path}`;
    if (!cleanHost) {
      goto(cleanPath);
      return;
    }

    if (typeof window === 'undefined') {
      goto(cleanPath);
      return;
    }

    if (normalizeHost(window.location.hostname) === cleanHost) {
      goto(cleanPath);
      return;
    }

    window.location.assign(`${window.location.protocol}//${cleanHost}${cleanPath}`);
  }

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

    // Surface "you were logged out" via a banner-style toast so the user
    // understands the redirect instead of being dumped on /login silently.
    const reasonParam = new URLSearchParams($page.url?.search || '').get('reason');
    if (reasonParam === 'expired') {
      toast.warning(
        $t('auth.session_expired_message') ||
          'Your session has expired. Please log in again to continue.',
      );
      // Strip the reason from the URL so it does not keep firing on refresh.
      try {
        const url = new URL(window.location.href);
        url.searchParams.delete('reason');
        window.history.replaceState({}, '', url.pathname + (url.search ? url.search : '') + url.hash);
      } catch {
        // non-blocking
      }
    }

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
      // Re-use the main redirection logic
      // Note: We don't have the full tenant object here, but we now have u.tenant_custom_domain
      // from the updated User model.
      redirectUser(u, undefined);
    }
  });

  async function redirectUser(u: any, t?: any) {
    const slug = u?.tenant_slug;
    const customDomainStatus = t?.custom_domain_status;
    const activeCustomDomain =
      (customDomainStatus === 'active' ? t?.custom_domain : null) || u?.tenant_custom_domain || null;
    const currentHost = normalizeHost(window.location.hostname);
    const platformHost = resolvePlatformHost();
    // @ts-ignore
    const isTauri = typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__;
    const isLocalhost =
      currentHost === 'localhost' || currentHost === '127.0.0.1' || currentHost.includes('tauri');
    const onPlatformHost = isPlatformDomain(currentHost) || currentHost === platformHost || ($appSettings.auth?.main_domain && currentHost === normalizeHost($appSettings.auth?.main_domain));

    const goToRoleHome = () => {
      const ctx = resolveTenantContext({
        hostname: currentHost,
        userTenantSlug: slug,
        tenantSlug: t?.slug,
        routeTenantSlug: $page.params.tenant,
      });

      let target = getDefaultTenantLandingPath(u, ctx.tenantPrefix);
      const currentPath = typeof window !== 'undefined' ? window.location.pathname : '/login';
      if (target === currentPath) target = '/';
      if (target === currentPath) target = '/';
      goto(target);
    };

    if (u.is_super_admin) {
      if (!isTauri && !isLocalhost && !onPlatformHost) {
        redirectToHost(platformHost, '/superadmin');
        return;
      }

      goto('/superadmin');
      return;
    }

    if (activeCustomDomain && currentHost === normalizeHost(activeCustomDomain) && slug) {
      try {
        const { cacheDomainMapping } = await import('$lib/utils/domain');
        cacheDomainMapping(currentHost, slug);
      } catch {
        // ignore cache failures
      }
      goToRoleHome();
      return;
    }

    if (!slug) {
      const { logout } = await import('$lib/stores/auth');
      logout();
      error = 'Akun Anda belum terhubung ke tenant/workspace.';
      return;
    }

    if (!isTauri && !isLocalhost && !onPlatformHost && activeCustomDomain && currentHost !== normalizeHost(activeCustomDomain)) {
      redirectToHost(activeCustomDomain, getDefaultTenantLandingPath(u, ''));
      return;
    }

    if (!isTauri && !isLocalhost && !activeCustomDomain && !onPlatformHost) {
      redirectToHost(platformHost, getDefaultTenantLandingPath(u, ''));
      return;
    }

    goToRoleHome();
  }

  async function start2FASetupEnrollment() {
    setupLoading = true;
    error = '';
    try {
      if (setupMethod === 'email') {
        await authApi.requestEmail2FASetupTemp(tempToken);
        setupEmailSent = true;
      } else {
        const result = await authApi.enable2FATemp(tempToken);
        setupQr = result.qr;
        setupSecret = result.secret;
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      setupLoading = false;
    }
  }

  async function changeSetupMethod(method: 'totp' | 'email') {
    setupMethod = method;
    setupEmailSent = false;
    setupQr = '';
    setupSecret = '';
    setupCode = '';
    await start2FASetupEnrollment();
  }

  async function handle2FASetupVerify() {
    if (!setupCode || setupCode.length < 6) return;
    error = '';
    loading = true;
    try {
      let response: any;
      if (setupMethod === 'email') {
        response = await authApi.verifyEmail2FASetupTemp(tempToken, setupCode);
      } else {
        response = await authApi.verify2FASetupTemp(tempToken, setupSecret, setupCode);
      }

      if (response.token) {
        const { setAuthData } = await import('$lib/stores/auth');
        setAuthData(response.token, response.user, rememberMe, response.tenant);
        redirectUser(response.user, response.tenant);
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = '';
    loading = true;

    try {
      const response = await login(identifier, password, rememberMe);

      if (response.requires_2fa_setup) {
        // Forced 2FA enrollment — tenant requires 2FA, user hasn't set it up yet
        tempToken = response.temp_token || '';
        step = '2fa-setup';
        start2FASetupEnrollment();
        return;
      }

      if (response.requires_2fa) {
        tempToken = response.temp_token || '';
        available2FAMethods = response.available_2fa_methods || ['totp'];

        // If only one method, go directly to it
        if (available2FAMethods.length === 1) {
          selected2FAMethod = available2FAMethods[0];
          step = available2FAMethods[0] === 'email' ? '2fa-email' : '2fa-totp';

          // Auto-send email OTP if email is the only method
          if (available2FAMethods[0] === 'email') {
            await sendEmailOtp();
          }
        } else {
          step = '2fa-select';
        }
        return;
      }

      if (response.user) {
        redirectUser(response.user, response.tenant);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg.includes('Account pending approval') || msg.includes('AccountPendingApproval')) {
        error = $t('auth.login.error_pending_approval');
      } else if (msg.includes('rejected')) {
        error = $t('auth.login.error_rejected');
      } else {
        error = msg || $t('auth.login.error_generic');
      }
    } finally {
      loading = false;
    }
  }

  async function selectMethod(method: string) {
    selected2FAMethod = method;
    twoFactorCode = '';
    error = '';

    if (method === 'email') {
      step = '2fa-email';
      await sendEmailOtp();
    } else {
      step = '2fa-totp';
    }
  }

  // Resend countdown state
  let resendCountdown = 0;
  let resendInterval: ReturnType<typeof setInterval> | null = null;
  const RESEND_DELAY = 60; // seconds

  function startResendCountdown() {
    resendCountdown = RESEND_DELAY;
    if (resendInterval) clearInterval(resendInterval);
    resendInterval = setInterval(() => {
      resendCountdown--;
      if (resendCountdown <= 0) {
        if (resendInterval) clearInterval(resendInterval);
        resendInterval = null;
      }
    }, 1000);
  }

  async function sendEmailOtp(isResend = false) {
    emailOtpSending = true;
    error = '';
    try {
      await authApi.requestEmailOtp(tempToken);
      emailOtpSent = true;
      startResendCountdown();

      // Show toast on resend
      if (isResend) {
        const { toast } = await import('svelte-sonner');
        toast.success($t('auth.2fa.code_resent') || 'Verification code sent!');
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      emailOtpSending = false;
    }
  }

  let trustDevice = false;

  async function handle2FAVerify() {
    if (!twoFactorCode || twoFactorCode.length < 6) return;
    error = '';
    loading = true;
    try {
      let response;
      if (selected2FAMethod === 'email') {
        response = await authApi.verifyEmailOtp(tempToken, twoFactorCode, trustDevice);
      } else {
        response = await authApi.verifyLogin2FA(tempToken, twoFactorCode, trustDevice);
      }

      if (response.token) {
        const { setAuthData } = await import('$lib/stores/auth');
        setAuthData(response.token, response.user, rememberMe, response.tenant);

        redirectUser(response.user, response.tenant);
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
        <h2>
          {#if step === 'login'}
            {$t('auth.login.title')}
          {:else if step === '2fa-select'}
            Two-Factor Authentication
          {:else if step === '2fa-totp'}
            Authenticator App
          {:else if step === '2fa-email'}
            Email Verification
          {:else if step === '2fa-setup'}
            Security Setup Required
          {/if}
        </h2>
        <p>
          {#if step === 'login'}
            {$t('auth.login.subtitle')}
          {:else if step === '2fa-select'}
            Choose your preferred verification method
          {:else if step === '2fa-totp'}
            Enter the 6-digit code from your authenticator app
          {:else if step === '2fa-email'}
            Enter the 6-digit code sent to your email
          {:else if step === '2fa-setup'}
            Your organization requires two-factor authentication. Please set it up to continue.
          {/if}
        </p>
      </div>

      {#if error}
        <div class="alert error" in:fly={{ y: -10 }}>
          {error}
        </div>
      {/if}

      {#if step === 'login'}
        <form on:submit={handleSubmit}>
          <!-- Identifier (email or phone) -->
          <div class="input-group" class:focus={activeField === 'identifier'}>
            <label for="identifier">{$t('auth.login.identifier_label') || 'Email atau Nomor HP'}</label>
            <div class="field">
              <span class="icon"><Icon name="user" size={18} /></span>
              <input
                type="text"
                id="identifier"
                bind:value={identifier}
                on:focus={() => (activeField = 'identifier')}
                on:blur={() => (activeField = '')}
                placeholder={$t('auth.login.identifier_placeholder') || 'nama@email.com atau 08xxx'}
                required
              />
            </div>
            <span class="hint">{$t('auth.login.identifier_hint') || 'Login dengan email atau nomor HP Anda'}</span>
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
      {:else if step === '2fa-select'}
        <!-- 2FA Method Selection -->
        <div class="method-selection">
          <p style="margin-bottom: 1.5rem; color: var(--text-secondary);">
            Choose your verification method:
          </p>

          {#each available2FAMethods as method}
            <button type="button" class="method-btn" on:click={() => selectMethod(method)}>
              <Icon name={method === 'totp' ? 'shield' : 'mail'} size={24} />
              <span>
                {method === 'totp' ? 'Authenticator App' : 'Email Code'}
              </span>
            </button>
          {/each}

          <button
            type="button"
            class="btn-text"
            on:click={() => (step = 'login')}
            style="width: 100%; margin-top: 1rem; background: none; border: none; color: var(--text-secondary); cursor: pointer;"
          >
            Back to Login
          </button>
        </div>
      {:else if step === '2fa-totp'}
        <!-- TOTP Form -->
        <form on:submit|preventDefault={handle2FAVerify}>
          <div class="input-group" class:focus={activeField === '2fa'}>
            <label for="2fa-code">
              {$t('auth.2fa.enter_code')}
            </label>
            <div class="field">
              <span class="icon"><Icon name="shield" size={18} /></span>
              <input
                type="text"
                id="2fa-code"
                bind:value={twoFactorCode}
                on:focus={() => (activeField = '2fa')}
                on:blur={() => (activeField = '')}
                placeholder={$t('common.otp_placeholder')}
                maxlength="6"
                required
                style="letter-spacing: 0.5em; text-align: center;"
              />
            </div>
          </div>

          <div class="form-utils" style="margin-bottom: 1rem; justify-content: center;">
            <label class="checkbox">
              <input type="checkbox" bind:checked={trustDevice} />
              <span class="checkmark"></span>
              <span>{$t('auth.2fa.trust_device')}</span>
            </label>
          </div>

          <button type="submit" class="btn-primary" disabled={loading || twoFactorCode.length < 6}>
            {#if loading}
              <div class="spinner"></div>
            {:else}
              {$t('auth.2fa.verify_and_login')}
            {/if}
          </button>

          {#if available2FAMethods.length > 1}
            <button
              type="button"
              class="btn-text"
              on:click={() => (step = '2fa-select')}
              style="width: 100%; margin-top: 0.5rem; background: none; border: none; color: var(--text-secondary); cursor: pointer;"
            >
              {$t('auth.2fa.try_another_method')}
            </button>
          {/if}

          <button
            type="button"
            class="btn-text"
            on:click={() => (step = 'login')}
            style="width: 100%; margin-top: 0.5rem; background: none; border: none; color: var(--text-secondary); cursor: pointer;"
          >
            {$t('auth.2fa.back_to_login')}
          </button>
        </form>
      {:else if step === '2fa-email'}
        <!-- Email OTP Form -->
        <form on:submit|preventDefault={handle2FAVerify}>
          {#if emailOtpSent}
            <div
              class="otp-sent-notice"
              style="margin-bottom: 1rem; padding: 1rem; background: var(--bg-success); border-radius: 8px; color: var(--text-success);"
            >
              <Icon name="check-circle" size={18} />
              <span>
                {$t('auth.2fa.email_sent')}
              </span>
            </div>
          {/if}

          <div class="input-group" class:focus={activeField === '2fa'}>
            <label for="email-otp-code">
              {$t('auth.2fa.enter_email_code')}
            </label>
            <div class="field">
              <span class="icon"><Icon name="mail" size={18} /></span>
              <input
                type="text"
                id="email-otp-code"
                bind:value={twoFactorCode}
                on:focus={() => (activeField = '2fa')}
                on:blur={() => (activeField = '')}
                placeholder={$t('common.otp_placeholder')}
                maxlength="6"
                required
                style="letter-spacing: 0.5em; text-align: center;"
              />
            </div>
          </div>

          <div class="form-utils" style="margin-bottom: 1rem; justify-content: center;">
            <label class="checkbox">
              <input type="checkbox" bind:checked={trustDevice} />
              <span class="checkmark"></span>
              <span>{$t('auth.2fa.trust_device')}</span>
            </label>
          </div>

          <button type="submit" class="btn-primary" disabled={loading || twoFactorCode.length < 6}>
            {#if loading}
              <div class="spinner"></div>
            {:else}
              {$t('auth.2fa.verify_and_login')}
            {/if}
          </button>

          <button
            type="button"
            class="btn-text"
            on:click={() => sendEmailOtp(true)}
            disabled={emailOtpSending || resendCountdown > 0}
            style="width: 100%; margin-top: 0.5rem; background: none; border: none; color: var(--primary); cursor: pointer;"
          >
            {#if emailOtpSending}
              Sending...
            {:else if resendCountdown > 0}
              {$t('auth.2fa.resend_code')} ({resendCountdown}s)
            {:else}
              {$t('auth.2fa.resend_code')}
            {/if}
          </button>

          {#if available2FAMethods.length > 1}
            <button
              type="button"
              class="btn-text"
              on:click={() => (step = '2fa-select')}
              style="width: 100%; margin-top: 0.5rem; background: none; border: none; color: var(--text-secondary); cursor: pointer;"
            >
              {$t('auth.2fa.try_another_method')}
            </button>
          {/if}

          <button
            type="button"
            class="btn-text"
            on:click={() => (step = 'login')}
            style="width: 100%; margin-top: 0.5rem; background: none; border: none; color: var(--text-secondary); cursor: pointer;"
          >
            {$t('auth.2fa.back_to_login')}
          </button>
        </form>
      {:else if step === '2fa-setup'}
        <!-- Forced 2FA Enrollment -->
        <div class="setup-flow">
          {#if setupLoading}
            <div style="text-align: center; padding: 2rem 0;">
              <div class="spinner" style="margin: 0 auto 1rem;"></div>
              <span>{$t('auth.2fa.preparing')}</span>
            </div>
          {:else if setupMethod === 'totp'}
            <div class="method-tabs" style="display: flex; gap: 0.5rem; margin-bottom: 1.5rem;">
              <button
                class="btn {setupMethod === 'totp' ? 'btn-primary' : 'btn-outline'} btn-sm"
                on:click={() => changeSetupMethod('totp')}
              >
                <Icon name="smartphone" size={16} />
                Authenticator
              </button>
              <button
                class="btn {setupMethod === 'email' ? 'btn-primary' : 'btn-outline'} btn-sm"
                on:click={() => changeSetupMethod('email')}
              >
                <Icon name="mail" size={16} />
                Email
              </button>
            </div>
            <div class="qr-section" style="text-align: center; margin-bottom: 1.5rem;">
              <span class="step-label" style="display: block; margin-bottom: 1rem;">1. Scan this QR code with your authenticator app</span>
              <div class="qr-wrapper" style="background: white; padding: 1rem; border-radius: 8px; display: inline-block;">
                <img src="data:image/png;base64,{setupQr}" alt={$t('auth.2fa.qr_code')} style="width: 180px; height: 180px;" />
              </div>
              <p style="margin-top: 0.75rem; font-size: 0.8rem; color: var(--text-muted); word-break: break-all;">
                Key: {setupSecret}
              </p>
            </div>
            <form on:submit|preventDefault={handle2FASetupVerify}>
              <div class="input-group" class:focus={activeField === 'setup'}>
                <label for="setup-code">2. Enter the verification code</label>
                <div class="field">
                  <span class="icon"><Icon name="shield" size={18} /></span>
                  <input
                    type="text"
                    id="setup-code"
                    bind:value={setupCode}
                    on:focus={() => (activeField = 'setup')}
                    on:blur={() => (activeField = '')}
                    placeholder="000000"
                    maxlength="6"
                    required
                    style="letter-spacing: 0.5em; text-align: center;"
                  />
                </div>
              </div>
              <button type="submit" class="btn-primary full-width" disabled={loading || setupCode.length < 6}>
                {#if loading}
                  <div class="spinner"></div>
                {:else}
                  Activate & Login
                {/if}
              </button>
            </form>
          {:else}
            <!-- Email 2FA Setup -->
            <div class="method-tabs" style="display: flex; gap: 0.5rem; margin-bottom: 1.5rem;">
              <button
                class="btn {setupMethod === 'totp' ? 'btn-primary' : 'btn-outline'} btn-sm"
                on:click={() => changeSetupMethod('totp')}
              >
                <Icon name="smartphone" size={16} />
                Authenticator
              </button>
              <button
                class="btn {setupMethod === 'email' ? 'btn-primary' : 'btn-outline'} btn-sm"
                on:click={() => changeSetupMethod('email')}
              >
                <Icon name="mail" size={16} />
                Email
              </button>
            </div>
            {#if setupEmailSent}
              <div style="margin-bottom: 1rem; padding: 1rem; background: var(--bg-success); border-radius: 8px; color: var(--text-success);">
                <Icon name="check-circle" size={18} />
                <span>{$t('auth.2fa.code_sent')}</span>
              </div>
            {/if}
            <p style="margin-bottom: 1rem; color: var(--text-secondary);">
              We'll send a verification code to your registered email address.
            </p>
            <form on:submit|preventDefault={handle2FASetupVerify}>
              <div class="input-group" class:focus={activeField === 'setup'}>
                <label for="setup-email-code">{$t('auth.2fa.enter_code')}</label>
                <div class="field">
                  <span class="icon"><Icon name="mail" size={18} /></span>
                  <input
                    type="text"
                    id="setup-email-code"
                    bind:value={setupCode}
                    on:focus={() => (activeField = 'setup')}
                    on:blur={() => (activeField = '')}
                    placeholder="000000"
                    maxlength="6"
                    required
                    style="letter-spacing: 0.5em; text-align: center;"
                  />
                </div>
              </div>
              <button type="submit" class="btn-primary full-width" disabled={loading || setupCode.length < 6}>
                {#if loading}
                  <div class="spinner"></div>
                {:else}
                  Verify & Login
                {/if}
              </button>
            </form>
          {/if}
        </div>
      {/if}

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
    color: #08090d;
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

  .method-selection {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .method-btn {
    display: flex;
    align-items: center;
    gap: 1rem;
    width: 100%;
    padding: 1rem 1.5rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    color: var(--text-primary);
    cursor: pointer;
    transition: all 0.2s;
    font-size: 1rem;
  }

  .method-btn:hover {
    background: var(--bg-tertiary);
    border-color: var(--color-primary);
  }

  .otp-sent-notice {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .hint {
    display: block;
    font-size: 0.75rem;
    color: var(--text-muted, #999);
    margin-top: 0.25rem;
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
