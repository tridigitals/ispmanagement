<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { auth } from '$lib/api/client';
  import { goto } from '$app/navigation';
  import { token as authToken, user } from '$lib/stores/auth';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';

  let status: 'verifying' | 'success' | 'error' = 'verifying';
  let message = get(t)('auth.verify_email.verifying') || 'Verifying your email...';

  onMount(async () => {
    const token = $page.url.searchParams.get('token');
    if (!token) {
      status = 'error';
      message = get(t)('auth.verify_email.invalid_link') || 'Invalid verification link.';
      return;
    }

    try {
      const response = await auth.verifyEmail(token);
      status = 'success';
      message =
        response.message || get(t)('auth.verify_email.success') || 'Email verified successfully!';

      // Auto login if token returned
      if (response.token) {
        authToken.set(response.token);
        user.set(response.user);
        // Also update local storage if needed (stores/auth.ts handles subscription)
      }

      setTimeout(() => {
        goto('/dashboard');
      }, 3000);
    } catch (err) {
      status = 'error';
      message = err instanceof Error ? err.message : String(err);
    }
  });
</script>

<div class="page-container">
  <div class="card">
    <div class="status-icon {status}">
      {#if status === 'verifying'}
        <div class="spinner"></div>
      {:else if status === 'success'}
        ✓
      {:else}
        ✕
      {/if}
    </div>
    <h1>{$t('auth.verify_email.title') || 'Email Verification'}</h1>
    <p>{message}</p>

    {#if status === 'success'}
      <p class="redirect-text">
        {$t('auth.verify_email.redirecting') || 'Redirecting to dashboard...'}
      </p>
    {/if}

    {#if status === 'error'}
      <button class="btn btn-primary" on:click={() => goto('/login')}>
        {$t('auth.verify_email.go_to_login') || 'Go to Login'}
      </button>
    {/if}
  </div>
</div>

<style>
  .page-container {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 2rem;
  }

  .card {
    background: var(--bg-card);
    padding: 2.5rem;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    text-align: center;
    max-width: 400px;
    width: 100%;
  }

  .status-icon {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 32px;
    margin: 0 auto 1.5rem;
  }

  .status-icon.verifying {
    background: var(--bg-surface);
  }

  .status-icon.success {
    background: rgba(34, 197, 94, 0.1);
    color: #22c55e;
    border: 1px solid rgba(34, 197, 94, 0.2);
  }

  .status-icon.error {
    background: rgba(239, 68, 68, 0.1);
    color: #ef4444;
    border: 1px solid rgba(239, 68, 68, 0.2);
  }

  h1 {
    font-size: 1.5rem;
    margin-bottom: 0.5rem;
  }

  p {
    color: var(--text-secondary);
    margin-bottom: 1.5rem;
  }

  .redirect-text {
    font-size: 0.9rem;
    opacity: 0.8;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .btn {
    padding: 0.6rem 1.2rem;
    border-radius: var(--radius-md);
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: all 0.2s;
  }

  .btn-primary {
    background: var(--color-primary);
    color: white;
  }
</style>
