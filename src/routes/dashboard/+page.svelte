<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { user, isAuthenticated } from '$lib/stores/auth';
  import { t } from 'svelte-i18n';

  onMount(() => {
    if ($isAuthenticated) {
      goto('/dashboard');
    } else {
      // Redirect to login
      goto('/login');
    }
  });
</script>

<div class="redirect-container">
  <p>{$t('pages.dashboard.redirecting') || 'Redirecting to your dashboard...'}</p>
</div>

<style>
  .redirect-container {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background: var(--bg-primary);
    color: var(--text-primary);
  }
</style>
