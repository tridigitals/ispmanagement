<script lang="ts">
  import Sidebar from '$lib/components/layout/Sidebar.svelte';
  import Topbar from '$lib/components/layout/Topbar.svelte';
  import { isSuperAdmin, checkAuth } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  let authorized = $state(false);
  let { children } = $props();
  let mobileOpen = $state(false);

  // Strict auth check (must validate token with backend, not only local cache)
  onMount(() => {
    let cancelled = false;

    const runGuard = async () => {
      const valid = await checkAuth();
      if (cancelled) return;

      if (!valid) {
        goto('/login?reason=expired');
        return;
      }

      if ($isSuperAdmin) {
        authorized = true;
        return;
      }

      // Logged in but not super admin
      goto('/dashboard');
    };

    void runGuard();

    return () => {
      cancelled = true;
    };
  });
</script>

{#if authorized}
  <div class="app-shell">
    <Sidebar bind:isMobileOpen={mobileOpen} />

    <div class="main-viewport">
      <div class="content-surface">
        <Topbar onMobileMenuClick={() => (mobileOpen = !mobileOpen)} />
        <div class="scroll-area">
          {@render children()}
        </div>
      </div>
    </div>
  </div>
{:else}
  <!-- Loading state while checking auth -->
  <div class="auth-checking">
    <div class="spinner"></div>
  </div>
{/if}

<style>
  .auth-checking {
    height: 100vh;
    width: 100vw;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-app);
  }
  .spinner {
    width: 40px;
    height: 40px;
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

  .app-shell {
    display: flex;
    height: calc(100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom));
    min-height: calc(100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom));
    width: 100%;
    background: var(--bg-app);
    overflow: hidden;
  }

  .main-viewport {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: clamp(6px, 1vw, 12px);
    padding-left: 0;
    min-height: 0;
  }

  .content-surface {
    flex: 1;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: var(--shadow-sm);
    position: relative;
    min-height: 0;
  }

  .scroll-area {
    flex: 1;
    overflow-y: auto;
    position: relative;
    padding-bottom: env(safe-area-inset-bottom);
    min-height: 0;
    overscroll-behavior: contain;
  }

  @media (max-width: 900px) {
    .main-viewport {
      padding: clamp(4px, 2vw, 10px);
    }

    .content-surface {
      border-radius: var(--radius-md);
      border-left: none;
      border-right: none;
    }
  }
</style>
