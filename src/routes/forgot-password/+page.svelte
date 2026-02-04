<script lang="ts">
  import { goto } from '$app/navigation';
  import { auth } from '$lib/api/client';
  import { t } from 'svelte-i18n';

  let email = '';
  let submitted = false;
  let loading = false;
  let error = '';

  async function handleReset() {
    if (!email) return;
    loading = true;
    error = '';

    try {
      await auth.forgotPassword(email);
      submitted = true;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }
</script>

<div class="auth-page">
  <div class="auth-card fade-in">
    {#if !submitted}
      <div class="auth-header">
        <div class="icon">🔑</div>
        <h1>{$t('auth.forgot_password.title') || 'Reset Password'}</h1>
        <p>
          {$t('auth.forgot_password.subtitle') ||
            "Enter your email and we'll send you a link to reset your password."}
        </p>
      </div>

      {#if error}
        <div
          class="alert alert-error"
          style="color: #ef4444; background: rgba(239, 68, 68, 0.1); padding: 0.75rem; border-radius: 0.5rem; margin-bottom: 1.5rem; text-align: center;"
        >
          {error}
        </div>
      {/if}

      <form on:submit|preventDefault={handleReset}>
        <div class="form-group">
          <label class="form-label" for="email"
            >{$t('auth.forgot_password.email_label') || 'Email Address'}</label
          >
          <input
            type="email"
            id="email"
            class="form-input"
            bind:value={email}
            placeholder={$t('auth.forgot_password.email_placeholder') || 'you@example.com'}
            required
            disabled={loading}
          />
        </div>

        <button type="submit" class="btn btn-primary w-full" disabled={loading}>
          {#if loading}
            {$t('auth.forgot_password.sending') || 'Sending...'}
          {:else}
            {$t('auth.forgot_password.submit') || 'Send Reset Link'}
          {/if}
        </button>
      </form>
    {:else}
      <div class="success-state fade-in">
        <div class="icon">📩</div>
        <h1>
          {$t('auth.forgot_password.check_email_title') || 'Check your email'}
        </h1>
        <p>
          {$t('auth.forgot_password.check_email_message') || "We've sent a password reset link to"}
          <strong>{email}</strong>.
        </p>
        <button class="btn btn-secondary w-full" on:click={() => goto('/login')}>
          {$t('auth.forgot_password.back_to_login') || 'Back to Login'}
        </button>
      </div>
    {/if}

    <div class="auth-footer">
      <a href="/login" class="back-link"
        >← {$t('auth.forgot_password.back_to_login') || 'Back to Login'}</a
      >
    </div>
  </div>
</div>

<style>
  .auth-page {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    background: var(--bg-primary);
  }

  .auth-card {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: var(--border-radius);
    padding: 3rem;
    width: 100%;
    max-width: 420px;
    backdrop-filter: blur(10px);
  }

  .icon {
    font-size: 3rem;
    margin-bottom: 1.5rem;
    text-align: center;
  }

  .auth-header {
    text-align: center;
    margin-bottom: 2rem;
  }

  .auth-header h1 {
    font-size: 1.75rem;
    margin-bottom: 0.75rem;
  }

  .auth-header p {
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .success-state {
    text-align: center;
  }

  .success-state h1 {
    margin-bottom: 1rem;
  }

  .success-state p {
    color: var(--text-secondary);
    margin-bottom: 2rem;
  }

  .auth-footer {
    text-align: center;
    margin-top: 2rem;
  }

  .back-link {
    color: var(--text-secondary);
    text-decoration: none;
    font-size: 0.9rem;
    transition: color 0.2s;
  }

  .back-link:hover {
    color: var(--text-primary);
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .fade-in {
    animation: fadeIn 0.4s ease-out;
  }
</style>
