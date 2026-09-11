<!--
  /unauthorized — halaman 403 publik. Diarahkan ke sini oleh guard layout v2
  dan beberapa halaman admin saat role tidak mencukupi. Aksi primer mengikuti
  state sesi: user login -> "Ke beranda" (home sesuai role) + "Keluar";
  sesi mati -> "Masuk". Tetap lewat AuthLayout supaya kanvas terang sejak
  piksel pertama dan konsisten dengan halaman auth lain.
-->
<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { isAuthenticated, logout, user } from '$lib/stores/auth';
  import { getDefaultTenantLandingPath } from '$lib/utils/appLanding';
  import { t } from 'svelte-i18n';
  import AuthLayout from '$lib/components/auth/AuthLayout.svelte';

  let signingOut = $state(false);

  const homeHref = $derived(getDefaultTenantLandingPath($user, ''));

  async function handleSignOut() {
    signingOut = true;
    await logout({ silent: true });
    goto(`/login?next=${encodeURIComponent($page.url.pathname)}`);
  }
</script>

<AuthLayout>
  <div class="wrap">
    <span class="shield" aria-hidden="true">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 3 5 5.6v5.1c0 4.5 2.9 8.2 7 9.7 4.1-1.5 7-5.2 7-9.7V5.6L12 3Z" />
        <path d="M9.4 12.1l1.9 1.9 3.6-3.9" />
      </svg>
    </span>

    <p class="code">403</p>
    <h1>{$t('pages.unauthorized.title')}</h1>

    {#if $user?.name}
      <p class="who">
        <span class="avatar" aria-hidden="true">{$user.name.trim().charAt(0).toUpperCase()}</span>
        <span>{$user.name}</span>
      </p>
    {/if}

    <p class="message">
      {$t('pages.unauthorized.message')}
    </p>
    <p class="sub">{$t('pages.unauthorized.subtitle')}</p>

    <div class="actions">
      {#if $isAuthenticated}
        <a class="btn-primary" href={homeHref}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="m4.5 12.5 7-7 7 7" /><path d="M6.5 10.5V19a1 1 0 0 0 1 1h9a1 1 0 0 0 1-1v-8.5" />
          </svg>
          {$t('pages.unauthorized.go_home')}
        </a>
        <button class="btn-ghost" onclick={handleSignOut} disabled={signingOut}>
          {#if signingOut}
            <span class="spin" aria-hidden="true"></span>
          {:else}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M14.5 8V5.5a1.5 1.5 0 0 0-1.5-1.5H6a1.5 1.5 0 0 0-1.5 1.5v13A1.5 1.5 0 0 0 6 20h7a1.5 1.5 0 0 0 1.5-1.5V16" /><path d="M10 12h10" /><path d="m17 9 3 3-3 3" />
            </svg>
          {/if}
          {$t('pages.unauthorized.sign_out')}
        </button>
      {:else}
        <a class="btn-primary" href="/login?next={encodeURIComponent($page.url.pathname)}">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M9.5 8V5.5A1.5 1.5 0 0 1 11 4h7a1.5 1.5 0 0 1 1.5 1.5v13A1.5 1.5 0 0 1 18 20h-7a1.5 1.5 0 0 1-1.5-1.5V16" /><path d="M3.5 12h10" /><path d="m11 9 3 3-3 3" />
          </svg>
          {$t('auth.login.title')}
        </a>
      {/if}
    </div>
  </div>
</AuthLayout>

<style>
  .wrap {
    padding: clamp(1.75rem, 4vw, 2.5rem);
    text-align: center;
  }

  .shield {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    margin: 0 auto 1rem;
    border-radius: var(--radius-xl);
    color: var(--color-primary);
    background: var(--color-primary-subtle);
    border: 1px solid color-mix(in srgb, var(--color-primary) 22%, transparent);
  }
  .shield svg {
    width: 28px;
    height: 28px;
  }

  .code {
    margin: 0;
    font-size: 0.8125rem;
    font-weight: 700;
    letter-spacing: 0.14em;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  h1 {
    margin: 0.25rem 0 0;
    font-size: 1.375rem;
    font-weight: 700;
    letter-spacing: -0.015em;
    color: var(--text-primary);
  }

  .who {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0.875rem 0 0;
    padding: 0.3125rem 0.75rem 0.3125rem 0.375rem;
    font-size: 0.8125rem;
    font-weight: 550;
    color: var(--text-secondary);
    background: var(--color-ink-50);
    border: 1px solid var(--color-ink-200);
    border-radius: 999px;
  }
  .avatar {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--color-brand-700);
    background: var(--color-brand-100, #e3e9ff);
  }

  .message {
    margin: 0.875rem auto 0;
    max-width: 24rem;
    font-size: 0.875rem;
    line-height: 1.6;
    color: var(--text-secondary);
  }
  .sub {
    margin: 0.375rem auto 0;
    font-size: 0.8125rem;
    color: var(--text-muted);
  }

  .actions {
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
    margin-top: 1.75rem;
  }

  .btn-primary,
  .btn-ghost {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.6875rem 1rem;
    font-size: 0.9375rem;
    font-weight: 600;
    border-radius: 10px;
    cursor: pointer;
    text-decoration: none;
    transition: filter 0.15s, background 0.15s, transform 0.05s, box-shadow 0.15s, opacity 0.15s;
  }
  .btn-primary svg,
  .btn-ghost svg {
    width: 17px;
    height: 17px;
  }
  .btn-primary {
    color: #fff;
    background: var(--color-primary);
    border: 0;
    box-shadow: 0 6px 16px color-mix(in srgb, var(--color-primary) 22%, transparent);
  }
  .btn-primary:hover {
    filter: brightness(1.06);
    box-shadow: 0 8px 20px color-mix(in srgb, var(--color-primary) 28%, transparent);
  }
  .btn-primary:active {
    transform: translateY(1px);
  }
  .btn-ghost {
    color: var(--text-secondary);
    background: var(--color-ink-0, #fff);
    border: 1px solid var(--color-ink-200);
  }
  .btn-ghost:hover:not(:disabled) {
    color: var(--text-primary);
    background: var(--color-ink-50);
    border-color: var(--color-ink-300);
  }
  .btn-ghost:disabled {
    opacity: 0.6;
    cursor: wait;
  }
  .btn-primary:focus-visible,
  .btn-ghost:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .spin {
    width: 15px;
    height: 15px;
    border-radius: 50%;
    border: 2px solid color-mix(in srgb, var(--text-secondary) 35%, transparent);
    border-top-color: var(--text-secondary);
    animation: rot 0.7s linear infinite;
  }
  @keyframes rot {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
</style>
