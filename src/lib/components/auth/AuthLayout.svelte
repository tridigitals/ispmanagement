<!--
  AuthLayout — shell split-screen untuk SEMUA halaman auth publik
  (login, register, forgot/reset, verify-email, unauthorized, maintenance).

  Panel kiri = brand (mark + nama aplikasi + tagline) di atas wash brand;
  panel kanan = kartu form di kanvas ink-50. Wrapper `.v2-light` mem-flip
  token legacy (--bg-surface, --text-primary, ...) ke tema terang supaya
  style warisan tiap halaman auth ikut terang tanpa rewrite per-selektor.
  Di mobile panel brand collapse jadi strip header ramping.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import { appLogo } from '$lib/stores/logo';
  import { appSettings } from '$lib/stores/settings';

  let { children }: { children: import('svelte').Snippet } = $props();

  onMount(() => {
    appLogo.init();
    appSettings.init();
  });
</script>

<div class="contents v2-light">
  <div class="auth-grid">
    <!-- Brand panel -->
    <aside class="auth-brand">
      <div class="auth-brand-inner">
        <div class="auth-mark">
          {#if $appLogo}
            <img src={$appLogo} alt="" class="auth-mark-img" />
          {:else}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <circle cx="12" cy="12" r="9" />
              <path d="M3.6 8.5c4.8 2.4 12 2.4 16.8 0M3.6 15.5c4.8-2.4 12-2.4 16.8 0M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18" />
            </svg>
          {/if}
        </div>
        <h1 class="auth-appname">{$appSettings.app_name || 'ISP Management'}</h1>
        <p class="auth-tagline">{$t('auth.layout.title')}</p>
        <div class="auth-spark" aria-hidden="true">
          <span class="auth-spark-item">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19a16 16 0 0 1 16-16" /><path d="M4 12.8A7.2 7.2 0 0 1 11.2 20" /><circle cx="5" cy="19" r="1.4" fill="currentColor" stroke="none" /></svg>
            {$t('auth.layout.f1')}
          </span>
          <span class="auth-spark-item">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M17 20v-2a4 4 0 0 0-4-4H7a4 4 0 0 0-4 4v2" /><circle cx="10" cy="7" r="4" /><path d="M21 20v-2a4 4 0 0 0-3-3.9" /><path d="M15 3.1a4 4 0 0 1 0 7.8" /></svg>
            {$t('auth.layout.f2')}
          </span>
          <span class="auth-spark-item">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="6" width="20" height="12" rx="3" /><path d="M2 11h20" /><path d="M6 15h4" /></svg>
            {$t('auth.layout.f3')}
          </span>
        </div>
      </div>
    </aside>

    <!-- Form panel -->
    <main class="auth-main">
      <div class="auth-card">
        {@render children()}
      </div>
    </main>
  </div>
</div>

<style>
  .auth-grid {
    display: grid;
    grid-template-columns: 1fr;
    min-height: 100dvh;
    background: var(--color-ink-50);
  }

  /* ---------- Brand panel ---------- */
  .auth-brand {
    display: none;
    background: linear-gradient(160deg, var(--color-brand-50) 0%, var(--color-ink-50) 72%);
  }
  @media (min-width: 64rem) {
    .auth-grid {
      grid-template-columns: minmax(380px, 44%) 1fr;
    }
    .auth-brand {
      display: flex;
      align-items: center;
      padding: clamp(2rem, 6vw, 6rem);
      border-right: 1px solid var(--color-ink-100);
    }
  }
  .auth-brand-inner {
    max-width: 26rem;
  }
  .auth-mark {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 12px;
    background: var(--color-brand-600);
    color: #fff;
    margin-bottom: 1.5rem;
  }
  .auth-mark svg {
    width: 26px;
    height: 26px;
  }
  .auth-mark-img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    padding: 7px;
    background: #fff;
    border-radius: 12px;
  }
  .auth-appname {
    font-size: 1.625rem;
    font-weight: 700;
    letter-spacing: -0.01em;
    color: var(--color-ink-900);
    line-height: 1.2;
  }
  .auth-tagline {
    margin-top: 0.625rem;
    font-size: 0.9375rem;
    line-height: 1.55;
    color: var(--color-ink-500);
  }
  .auth-spark {
    display: flex;
    flex-direction: column;
    gap: 0.875rem;
    margin-top: 2.75rem;
  }
  .auth-spark-item {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--color-ink-700, #3f3f46);
  }
  .auth-spark-item svg {
    width: 17px;
    height: 17px;
    color: var(--color-brand-600);
    flex: none;
  }

  /* ---------- Form panel ---------- */
  .auth-main {
    display: grid;
    place-items: center;
    padding: clamp(1.25rem, 4vw, 2.5rem);
  }
  .auth-card {
    width: 100%;
    max-width: 26rem;
    background: var(--color-ink-0, #fff);
    border: 1px solid var(--color-ink-200);
    border-radius: 16px;
  }
</style>
