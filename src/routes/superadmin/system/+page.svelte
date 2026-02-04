<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { api } from '$lib/api/client';
  import { isSuperAdmin } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { systemHealthCache, type SystemHealth } from '$lib/stores/systemHealth';
  import { t } from 'svelte-i18n';
  import { appSettings } from '$lib/stores/settings';
  import { formatDateTime } from '$lib/utils/date';

  // New Components
  import SystemStatusBanner from '$lib/components/superadmin/system/SystemStatusBanner.svelte';
  import SystemResources from '$lib/components/superadmin/system/SystemResources.svelte';
  import RequestMetrics from '$lib/components/superadmin/system/RequestMetrics.svelte';
  import SystemStatsGrid from '$lib/components/superadmin/system/SystemStatsGrid.svelte';
  import DatabaseTables from '$lib/components/superadmin/system/DatabaseTables.svelte';
  import RecentActivity from '$lib/components/superadmin/system/RecentActivity.svelte';

  let health = $state<SystemHealth | null>(null);
  let loading = $state(true);
  let error = $state('');
  let refreshInterval: ReturnType<typeof setInterval>;

  onMount(() => {
    let unsubscribe: (() => void) | undefined;
    // Check superadmin status first
    unsubscribe = isSuperAdmin.subscribe((value) => {
      if (!value) {
        goto('/dashboard');
        return;
      }
    });

    const cached = $systemHealthCache;
    if (cached.health) {
      health = cached.health;
      loading = false;
      // Refresh in background to avoid UI flash
      void loadHealth();
    } else {
      void loadHealth();
    }
    // Auto-refresh every 30 seconds
    refreshInterval = setInterval(loadHealth, 30000);

    return () => {
      if (unsubscribe) unsubscribe();
      if (refreshInterval) clearInterval(refreshInterval);
    };
  });

  async function loadHealth() {
    try {
      health = await api.superadmin.getSystemHealth();
      systemHealthCache.set({ health, fetchedAt: Date.now() });
      error = '';
    } catch (e: any) {
      console.error('Failed to load system health:', e);
      error = e.message || 'Failed to load system health';
    } finally {
      loading = false;
    }
  }
</script>

<div class="page-container fade-in">
  <div class="page-header">
    <div class="header-content">
      <h1>{$t('superadmin.system.title') || 'System Health'}</h1>
      <p class="subtitle">
        {$t('superadmin.system.subtitle') || 'Monitor platform status and metrics'}
      </p>
    </div>
    <button
      class="btn-refresh"
      onclick={loadHealth}
      title={$t('common.refresh') || 'Refresh'}
      aria-label={$t('common.refresh') || 'Refresh'}
    >
      <Icon name="refresh-cw" size={18} />
    </button>
  </div>

  {#if loading && !health}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>
        {$t('superadmin.system.loading') || 'Loading system health...'}
      </p>
    </div>
  {:else if error}
    <div class="error-card">
      <Icon name="alert-circle" size={24} />
      <p>{error}</p>
      <button class="btn btn-primary" onclick={loadHealth}>
        {$t('superadmin.system.retry') || 'Retry'}
      </button>
    </div>
  {:else if health}
    <SystemStatusBanner {health} />
    <SystemResources {health} />
    <RequestMetrics {health} />
    <SystemStatsGrid {health} />

    <div class="grid-2">
      <DatabaseTables {health} />
      <RecentActivity {health} />
    </div>

    <div class="last-updated">
      <Icon name="clock" size={14} />
      {$t('superadmin.system.last_updated') || 'Last updated:'}
      {formatDateTime(health.collected_at, { timeZone: $appSettings.app_timezone })}
    </div>
  {/if}
</div>

<style>
  .page-container {
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 2rem;
  }

  .header-content h1 {
    font-size: 1.8rem;
    font-weight: 700;
    margin: 0 0 0.5rem 0;
    color: var(--text-primary);
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 0.95rem;
    margin: 0;
  }

  .btn-refresh {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
    width: 40px;
    height: 40px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-refresh:hover {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .grid-2 {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
    gap: 1.5rem;
    margin-bottom: 2rem;
  }

  .last-updated {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
    margin-top: 2rem;
    border-top: 1px solid var(--border-color);
    padding-top: 1rem;
  }

  /* Loading State */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem;
    color: var(--text-secondary);
    gap: 1rem;
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

  /* Error Card */
  .error-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 2rem;
    background: rgba(239, 68, 68, 0.05);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: var(--radius-lg);
    color: var(--color-danger);
    text-align: center;
    gap: 1rem;
  }

  .btn-primary {
    background: var(--color-primary);
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
  }
</style>
