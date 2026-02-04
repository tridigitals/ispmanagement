<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { isAuthenticated } from '$lib/stores/auth';
</script>

<div class="error-page fade-in">
  <div class="content">
    <div class="error-code">{$page.status}</div>
    <h1>
      {#if $page.status === 404}
        Page Not Found
      {:else}
        Something went wrong
      {/if}
    </h1>
    <p class="message">
      {#if $page.status === 404}
        The page you are looking for doesn't exist or has been moved.
      {:else}
        {$page.error?.message || 'An unexpected error occurred. Please try again later.'}
      {/if}
    </p>

    <div class="actions">
      {#if $isAuthenticated}
        <button class="btn btn-primary" on:click={() => goto('/dashboard')}>
          Back to Dashboard
        </button>
      {:else}
        <button class="btn btn-primary" on:click={() => goto('/')}> Back to Home </button>
      {/if}
      <button class="btn btn-secondary" on:click={() => history.back()}> Go Back </button>
    </div>
  </div>
</div>

<style>
  .error-page {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    text-align: center;
    background: var(--bg-primary);
  }

  .content {
    max-width: 500px;
  }

  .error-code {
    font-size: 8rem;
    font-weight: 900;
    line-height: 1;
    background: linear-gradient(135deg, var(--color-primary), var(--color-primary-light));
    background-clip: text;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    margin-bottom: 1rem;
    opacity: 0.5;
  }

  h1 {
    font-size: 2.5rem;
    margin-bottom: 1rem;
    color: var(--text-primary);
  }

  .message {
    font-size: 1.1rem;
    color: var(--text-secondary);
    margin-bottom: 2.5rem;
  }

  .actions {
    display: flex;
    gap: 1rem;
    justify-content: center;
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
    animation: fadeIn 0.5s ease-out;
  }
</style>
