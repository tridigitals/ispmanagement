<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { auth } from '$lib/api/client';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';

  import AuthLayout from '$lib/components/auth/AuthLayout.svelte';

  let token = $state('');
  let password = $state('');
  let confirmPassword = $state('');
  let error = $state('');
  let success = $state(false);
  let loading = $state(false);
  let touched = $state(false);
  let showPassword = $state(false);
  let showConfirmPassword = $state(false);
  let countdown = $state(3);

  let timer: ReturnType<typeof setInterval> | undefined;

  // Skor 0..4: panjang, variasi huruf/angka, simbol, campuran kapital.
  const strengthLevel = $derived.by(() => {
    let s = 0;
    if (password.length >= 8) s++;
    if (password.length >= 12) s++;
    if (/[a-z]/.test(password) && /[A-Z]/.test(password)) s++;
    if (/\d/.test(password)) s++;
    if (/[^A-Za-z0-9]/.test(password)) s++;
    return Math.min(4, s);
  });

  onMount(() => {
    token = $page.url.searchParams.get('token') || '';
    if (!token) {
      error = get(t)('auth.reset_password.invalid_token') || 'Invalid or missing reset token.';
    }
  });

  onDestroy(() => {
    if (timer) clearInterval(timer);
  });

  async function handleSubmit() {
    if (!token) return;
    if (password.length < 8) {
      error = get(t)('auth.reset_password.min_length') || 'Password must be at least 8 characters';
      return;
    }
    if (password !== confirmPassword) {
      error = get(t)('auth.reset_password.passwords_do_not_match') || 'Passwords do not match';
      return;
    }

    loading = true;
    error = '';

    try {
      await auth.resetPassword(token, password);
      success = true;
      timer = setInterval(() => {
        countdown -= 1;
        if (countdown <= 0) {
          if (timer) clearInterval(timer);
          goto('/login');
        }
      }, 1000);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }
</script>

<AuthLayout>
  <div class="form-wrapper">
    {#if success}
      <div class="done-block" role="status">
        <span class="done-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="9" /><path d="m8.5 12.4 2.3 2.3 4.7-5" />
          </svg>
        </span>
        <h1>{$t('auth.reset_password.success')}</h1>
        <p class="countdown">{$t('auth.reset_password.redirect_countdown', { values: { secs: countdown } })}</p>
        <a class="login-now" href="/login">{$t('auth.reset_password.back_to_login')}</a>
      </div>
    {:else if !token}
      <div class="done-block" role="alert">
        <span class="done-icon bad" aria-hidden="true">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M10.3 4.3 2.6 18a1.9 1.9 0 0 0 1.7 2.9h15.4a1.9 1.9 0 0 0 1.7-2.9L13.7 4.3a1.9 1.9 0 0 0-3.4 0Z" /><path d="M12 9.5v4" /><path d="M12 17.2h.01" />
          </svg>
        </span>
        <h1>{$t('auth.reset_password.invalid_link_title')}</h1>
        <p class="bad-msg">{$t('auth.reset_password.invalid_token')}</p>
        <p class="hint">{$t('auth.reset_password.invalid_link_hint')}</p>
        <a class="btn-primary" href="/forgot-password">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M21 12a9 9 0 1 1-2.6-6.3" /><path d="M21 4.5V10h-5.5" />
          </svg>
          {$t('auth.reset_password.request_new')}
        </a>
      </div>
    {:else}
      <header class="head">
        <span class="head-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <rect x="4" y="10.5" width="16" height="10" rx="2.2" /><path d="M8 10.5V7.8a4 4 0 0 1 8 0v2.7" /><path d="M12 14.6v2.4" />
          </svg>
        </span>
        <div>
          <h1>{$t('auth.reset_password.title')}</h1>
          <p>{$t('auth.reset_password.subtitle')}</p>
        </div>
      </header>

      {#if error}
        <div class="alert-error" role="alert">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="12" cy="12" r="9" /><path d="M12 7.5v5" /><path d="M12 16.3h.01" />
          </svg>
          <span>{error}</span>
        </div>
      {/if}

      <form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
        <div class="field">
          <label class="lbl" for="password">{$t('auth.reset_password.new_password')}</label>
          <div class="input-wrap">
            <input
              id="password"
              class="inp"
              class:good={touched && strengthLevel >= 3 && password === confirmPassword}
              type={showPassword ? 'text' : 'password'}
              bind:value={password}
              oninput={() => (touched = true)}
              placeholder="{'•'.repeat(8)}"
              minlength="8"
              autocomplete="new-password"
              required
              disabled={loading}
            />
            <button
              type="button"
              class="eye"
              aria-label={$t(showPassword ? 'auth.reset_password.hide' : 'auth.reset_password.show')}
              aria-pressed={showPassword}
              onclick={() => (showPassword = !showPassword)}
            >
              {#if showPassword}
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3l18 18" /><path d="M10.6 5.1A9.6 9.6 0 0 1 12 5c5.5 0 9.3 4.4 10.4 6.4l.3.6a13 13 0 0 1-2.1 2.8M6.7 6.9A12.6 12.6 0 0 0 1.3 11.4l-.3.6c1.1 2 4.9 6.4 10.4 6.4a10.4 10.4 0 0 0 4.4-1" /><path d="M9.9 10a3 3 0 0 0 4.2 4.2" /></svg>
              {:else}
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M2.5 12S6.3 5.5 12 5.5 21.5 12 21.5 12 17.7 18.5 12 18.5 2.5 12 2.5 12Z" /><circle cx="12" cy="12" r="3" /></svg>
              {/if}
            </button>
          </div>
          {#if touched && password}
            <div class="meter" aria-hidden="true">
              {#each [0, 1, 2, 3] as i}
                <span class="pip" class:on={i < strengthLevel} class:low={i < strengthLevel && strengthLevel === 1} class:mid={i < strengthLevel && strengthLevel === 2} class:high={i < strengthLevel && strengthLevel >= 3}></span>
              {/each}
            </div>
          {/if}
          <p class="hint">{$t('auth.reset_password.min_hint', { values: { min: 8 } })}</p>
        </div>

        <div class="field">
          <label class="lbl" for="confirmPassword">{$t('auth.reset_password.confirm_password')}</label>
          <div class="input-wrap">
            <input
              id="confirmPassword"
              class="inp"
              class:good={confirmPassword && confirmPassword === password}
              class:bad={touched && confirmPassword && confirmPassword !== password}
              type={showConfirmPassword ? 'text' : 'password'}
              bind:value={confirmPassword}
              oninput={() => (touched = true)}
              placeholder="{'•'.repeat(8)}"
              autocomplete="new-password"
              required
              disabled={loading}
            />
            <button
              type="button"
              class="eye"
              aria-label={$t(showConfirmPassword ? 'auth.reset_password.hide' : 'auth.reset_password.show')}
              aria-pressed={showConfirmPassword}
              onclick={() => (showConfirmPassword = !showConfirmPassword)}
            >
              {#if showConfirmPassword}
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3l18 18" /><path d="M10.6 5.1A9.6 9.6 0 0 1 12 5c5.5 0 9.3 4.4 10.4 6.4l.3.6a13 13 0 0 1-2.1 2.8M6.7 6.9A12.6 12.6 0 0 0 1.3 11.4l-.3.6c1.1 2 4.9 6.4 10.4 6.4a10.4 10.4 0 0 0 4.4-1" /><path d="M9.9 10a3 3 0 0 0 4.2 4.2" /></svg>
              {:else}
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M2.5 12S6.3 5.5 12 5.5 21.5 12 21.5 12 17.7 18.5 12 18.5 2.5 12 2.5 12Z" /><circle cx="12" cy="12" r="3" /></svg>
              {/if}
            </button>
          </div>
          {#if touched && confirmPassword}
            <p class="match" class:ok={confirmPassword === password}>
              {#if confirmPassword === password}
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="m4.5 12.8 5 5L19.5 7.5" /></svg>
                {$t('auth.reset_password.match_ok')}
              {:else}
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M5.5 5.5l13 13" /><path d="M18.5 5.5l-13 13" /></svg>
                {$t('auth.reset_password.mismatch')}
              {/if}
            </p>
          {/if}
        </div>

        <button type="submit" class="btn-primary" disabled={loading || !password || !confirmPassword || password.length < 8 || password !== confirmPassword || !token}>
          {#if loading}
            <span class="btn-spin" aria-hidden="true"></span>
            {$t('auth.reset_password.resetting')}
          {:else}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <rect x="4" y="10.5" width="16" height="10" rx="2.2" /><path d="M8 10.5V7.8a4 4 0 0 1 8 0v2.7" /><path d="M12 14.6v2.4" />
            </svg>
            {$t('auth.reset_password.submit')}
          {/if}
        </button>
      </form>

      <div class="foot">
        <a class="back-link" href="/login">← {$t('auth.reset_password.back_to_login')}</a>
      </div>
    {/if}
  </div>
</AuthLayout>

<style>
  .form-wrapper {
    width: 100%;
    padding: clamp(1.5rem, 4vw, 2.25rem);
  }

  /* ---------- header ---------- */
  .head {
    display: flex;
    align-items: center;
    gap: 0.875rem;
    margin-bottom: 1.5rem;
  }
  .head-icon {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: 12px;
    color: var(--color-primary);
    background: var(--color-primary-subtle);
    border: 1px solid color-mix(in srgb, var(--color-primary) 24%, transparent);
  }
  .head-icon svg {
    width: 22px;
    height: 22px;
  }
  .head h1 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 650;
    letter-spacing: -0.01em;
    line-height: 1.3;
    color: var(--text-primary);
  }
  .head p {
    margin: 0.125rem 0 0;
    font-size: 0.8125rem;
    line-height: 1.45;
    color: var(--text-secondary);
  }

  /* ---------- error ---------- */
  .alert-error {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.625rem 0.75rem;
    margin-bottom: 1.25rem;
    font-size: 0.8125rem;
    line-height: 1.45;
    border-radius: 10px;
    color: var(--color-danger, #dc2626);
    background: color-mix(in srgb, var(--color-danger, #dc2626) 8%, var(--color-ink-0, #fff));
    border: 1px solid color-mix(in srgb, var(--color-danger, #dc2626) 22%, transparent);
  }
  .alert-error svg {
    width: 16px;
    height: 16px;
    flex: none;
    margin-top: 0.0625rem;
  }

  /* ---------- fields ---------- */
  .field {
    margin-bottom: 1.125rem;
  }
  .lbl {
    display: block;
    margin-bottom: 0.375rem;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .input-wrap {
    position: relative;
  }
  .inp {
    width: 100%;
    padding: 0.6875rem 2.75rem 0.6875rem 0.875rem;
    font-size: 0.9375rem;
    color: var(--text-primary);
    background: var(--color-ink-0, #fff);
    border: 1px solid var(--border-color);
    border-radius: 10px;
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .inp::placeholder {
    color: var(--color-ink-300);
    letter-spacing: 0.08em;
  }
  .inp:hover:not(:focus) {
    border-color: var(--color-ink-300);
  }
  .inp:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 14%, transparent);
  }
  .inp.good:focus {
    border-color: var(--color-success, #16a34a);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-success, #16a34a) 14%, transparent);
  }
  .inp.bad {
    border-color: var(--color-danger, #dc2626);
  }
  .inp:disabled {
    opacity: 0.6;
  }
  .eye {
    position: absolute;
    right: 0.5rem;
    top: 50%;
    transform: translateY(-50%);
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    color: var(--text-muted);
    background: transparent;
    cursor: pointer;
  }
  .eye:hover {
    color: var(--text-secondary);
    background: var(--color-ink-50);
  }
  .eye:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
  .eye svg {
    width: 17px;
    height: 17px;
  }

  /* strength meter */
  .meter {
    display: flex;
    gap: 5px;
    margin-top: 0.5rem;
  }
  .pip {
    height: 4px;
    flex: 1;
    border-radius: 999px;
    background: var(--color-ink-200);
    transition: background 0.25s;
  }
  .pip.on.low {
    background: #ef4444;
  }
  .pip.on.mid {
    background: #d97706;
  }
  .pip.on.high {
    background: #059669;
  }
  .hint {
    margin: 0.4375rem 0 0;
    font-size: 0.75rem;
    line-height: 1.45;
    color: var(--text-muted);
  }
  .match {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    margin: 0.4375rem 0 0;
    font-size: 0.75rem;
    color: var(--color-danger, #dc2626);
  }
  .match.ok {
    color: #059669;
  }
  .match svg {
    width: 13px;
    height: 13px;
    flex: none;
  }

  /* ---------- primary button ---------- */
  .btn-primary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.75rem 1rem;
    margin-top: 0.25rem;
    font-size: 0.9375rem;
    font-weight: 600;
    color: #fff;
    background: var(--color-primary);
    border: 0;
    border-radius: 10px;
    cursor: pointer;
    text-decoration: none;
    box-shadow: 0 6px 16px color-mix(in srgb, var(--color-primary) 22%, transparent);
    transition: filter 0.15s, transform 0.05s, box-shadow 0.15s, opacity 0.15s;
  }
  .btn-primary svg {
    width: 17px;
    height: 17px;
  }
  .btn-primary:hover:not(:disabled) {
    filter: brightness(1.06);
    box-shadow: 0 8px 20px color-mix(in srgb, var(--color-primary) 28%, transparent);
  }
  .btn-primary:active:not(:disabled) {
    transform: translateY(1px);
  }
  .btn-primary:disabled {
    opacity: 0.45;
    cursor: not-allowed;
    box-shadow: none;
  }
  .btn-spin {
    width: 15px;
    height: 15px;
    border-radius: 50%;
    border: 2px solid rgba(255, 255, 255, 0.45);
    border-top-color: #fff;
    animation: btn-rot 0.7s linear infinite;
  }
  @keyframes btn-rot {
    to {
      transform: rotate(360deg);
    }
  }

  /* ---------- foot link ---------- */
  .foot {
    margin-top: 1.5rem;
    text-align: center;
  }
  .back-link {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-secondary);
    text-decoration: none;
  }
  .back-link:hover {
    color: var(--color-primary);
  }

  /* ---------- success / invalid-token blocks ---------- */
  .done-block {
    text-align: center;
    padding: 0.75rem 0 0.5rem;
  }
  .done-icon {
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    margin: 0 auto 1rem;
    border-radius: 14px;
    color: #059669;
    background: color-mix(in srgb, #059669 10%, var(--color-ink-0, #fff));
    border: 1px solid color-mix(in srgb, #059669 24%, transparent);
  }
  .done-icon svg {
    width: 26px;
    height: 26px;
  }
  .done-icon.bad {
    color: var(--color-danger, #dc2626);
    background: color-mix(in srgb, var(--color-danger, #dc2626) 8%, var(--color-ink-0, #fff));
    border-color: color-mix(in srgb, var(--color-danger, #dc2626) 22%, transparent);
  }
  .done-block h1 {
    margin: 0 0 0.375rem;
    font-size: 1.1875rem;
    font-weight: 650;
    letter-spacing: -0.01em;
    color: var(--text-primary);
  }
  .countdown {
    margin: 0 0 1.25rem;
    font-size: 0.8125rem;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .bad-msg {
    margin: 0 0 0.375rem;
    font-size: 0.875rem;
    font-weight: 550;
    color: var(--color-danger, #dc2626);
  }
  .done-block .hint {
    margin: 0 auto 1.25rem;
    max-width: 21rem;
  }
  .done-block .btn-primary {
    width: auto;
    padding-inline: 1.5rem;
  }
  .login-now {
    display: inline-block;
    margin-top: 0.75rem;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--color-primary);
    text-decoration: none;
  }
  .login-now:hover {
    text-decoration: underline;
  }
</style>
