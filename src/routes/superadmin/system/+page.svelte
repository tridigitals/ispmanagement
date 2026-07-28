<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { api } from '$lib/api/client';
  import { isSuperAdmin } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { systemHealthCache, type SystemHealth } from '$lib/stores/systemHealth';
  import { t } from 'svelte-i18n';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { appSettings } from '$lib/stores/settings';
  import { formatDateTime } from '$lib/utils/date';

  import {
    loadSuperadminSystemDiagnosticsModules,
    loadSuperadminSystemHealthModules,
  } from './systemPageModules';

  let activeView = $state<'health' | 'diagnostics'>('health');
  let health = $state<SystemHealth | null>(null);
  let loading = $state(true);
  let error = $state('');
  let diagnostics = $state<any | null>(null);
  let diagLoading = $state(false);
  let diagError = $state('');
  let refreshInterval: ReturnType<typeof setInterval>;

  let SystemStatusBannerComponent = $state<any>(null);
  let SystemResourcesComponent = $state<any>(null);
  let RequestMetricsComponent = $state<any>(null);
  let SystemStatsGridComponent = $state<any>(null);
  let DatabaseTablesComponent = $state<any>(null);
  let RecentActivityComponent = $state<any>(null);
  let SystemDiagnosticsPanelComponent = $state<any>(null);

  let healthModulesLoading = $state(false);
  let diagnosticsModulesLoading = $state(false);

  async function ensureHealthModulesLoaded() {
    const healthReady =
      SystemStatusBannerComponent &&
      SystemResourcesComponent &&
      RequestMetricsComponent &&
      SystemStatsGridComponent &&
      DatabaseTablesComponent &&
      RecentActivityComponent;

    if (healthReady || healthModulesLoading) return;

    healthModulesLoading = true;
    try {
      const {
        SystemStatusBannerComponent: SystemStatusBanner,
        SystemResourcesComponent: SystemResources,
        RequestMetricsComponent: RequestMetrics,
        SystemStatsGridComponent: SystemStatsGrid,
        DatabaseTablesComponent: DatabaseTables,
        RecentActivityComponent: RecentActivity,
      } = await loadSuperadminSystemHealthModules();
      SystemStatusBannerComponent = SystemStatusBanner;
      SystemResourcesComponent = SystemResources;
      RequestMetricsComponent = RequestMetrics;
      SystemStatsGridComponent = SystemStatsGrid;
      DatabaseTablesComponent = DatabaseTables;
      RecentActivityComponent = RecentActivity;
    } finally {
      healthModulesLoading = false;
    }
  }

  async function ensureDiagnosticsModulesLoaded() {
    if (SystemDiagnosticsPanelComponent || diagnosticsModulesLoading) return;

    diagnosticsModulesLoading = true;
    try {
      const { SystemDiagnosticsPanelComponent: SystemDiagnosticsPanel } =
        await loadSuperadminSystemDiagnosticsModules();
      SystemDiagnosticsPanelComponent = SystemDiagnosticsPanel;
    } finally {
      diagnosticsModulesLoading = false;
    }
  }

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
    void ensureHealthModulesLoaded();

    // Auto-refresh every 30 seconds
    refreshInterval = setInterval(() => {
      if (activeView === 'health') void loadHealth();
    }, 30000);

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
      error = extractApiErrorMessage(e, 'Failed to load system health');
    } finally {
      loading = false;
    }
  }

  async function loadDiagnostics() {
    diagLoading = true;
    try {
      diagnostics = await api.superadmin.getSystemDiagnostics();
      diagError = '';
    } catch (e: any) {
      console.error('Failed to load diagnostics:', e);
      diagError = extractApiErrorMessage(e, 'Failed to load diagnostics');
    } finally {
      diagLoading = false;
    }
  }

  function switchView(view: 'health' | 'diagnostics') {
    activeView = view;
    if (view === 'diagnostics') {
      void ensureDiagnosticsModulesLoaded();
      if (!diagnostics && !diagLoading) {
        void loadDiagnostics();
      }
    }
  }

  function refreshCurrent() {
    if (activeView === 'health') void loadHealth();
    else void loadDiagnostics();
  }
</script>

<div class="page-container fade-in">
  <div class="page-header">
    <div class="header-content">
      <h1>{$t('superadmin.system.title')}</h1>
      <p class="subtitle">
        {$t('superadmin.system.subtitle')}
      </p>
      <div class="view-toggle" role="group" aria-label={$t('superadmin.system.views')}>
        <button class:active={activeView === 'health'} onclick={() => switchView('health')}>
          {$t('superadmin.system.tabs.health')}
        </button>
        <button
          class:active={activeView === 'diagnostics'}
          onclick={() => switchView('diagnostics')}
        >
          {$t('superadmin.system.tabs.diagnostics')}
        </button>
      </div>
    </div>
    <button
      class="btn-refresh"
      onclick={refreshCurrent}
      title={$t('common.refresh')}
      aria-label={$t('common.refresh')}
    >
      <Icon name="refresh-cw" size={18} />
    </button>
  </div>

  {#if activeView === 'health'}
    {#if loading && !health}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>
          {$t('superadmin.system.loading')}
        </p>
      </div>
    {:else if error}
      <div class="error-card">
        <Icon name="alert-circle" size={24} />
        <p>{error}</p>
        <button class="btn btn-primary" onclick={loadHealth}>
          {$t('superadmin.system.retry')}
        </button>
      </div>
    {:else if health}
      {#if
        SystemStatusBannerComponent &&
        SystemResourcesComponent &&
        RequestMetricsComponent &&
        SystemStatsGridComponent &&
        DatabaseTablesComponent &&
        RecentActivityComponent
      }
        <SystemStatusBannerComponent {health} />
        <SystemResourcesComponent {health} />
        <RequestMetricsComponent {health} />
        <SystemStatsGridComponent {health} />

        <div class="grid-2">
          <DatabaseTablesComponent {health} />
          <RecentActivityComponent {health} />
        </div>
      {:else}
        <div class="loading-state">
          <div class="spinner"></div>
          <p>
            {$t('superadmin.system.loading')}
          </p>
        </div>
      {/if}

      <div class="last-updated">
        <Icon name="clock" size={14} />
        {$t('superadmin.system.last_updated')}
        {formatDateTime(health.collected_at, { timeZone: $appSettings.app_timezone })}
      </div>
    {/if}
  {:else if diagLoading && !diagnostics}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>
        {$t('superadmin.system.diagnostics.loading')}
      </p>
    </div>
  {:else if diagError}
    <div class="error-card">
      <Icon name="alert-circle" size={24} />
      <p>{diagError}</p>
      <button class="btn btn-primary" onclick={loadDiagnostics}>
        {$t('superadmin.system.retry')}
      </button>
    </div>
  {:else if diagnostics}
    {#if SystemDiagnosticsPanelComponent}
      <SystemDiagnosticsPanelComponent {diagnostics} />
    {:else}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>
          {$t('superadmin.system.diagnostics.loading')}
        </p>
      </div>
    {/if}
  {/if}
</div>

<style>
  .page-container {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1400px;
    margin: 0 auto;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
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

  .view-toggle {
    display: inline-flex;
    gap: 0.5rem;
    margin-top: 1rem;
    padding: 0.35rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
  }

  .view-toggle button {
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-secondary);
    padding: 0.45rem 0.75rem;
    border-radius: 10px;
    cursor: pointer;
    font-weight: 700;
    font-size: 0.85rem;
    transition: 0.15s ease;
  }

  .view-toggle button:hover {
    color: var(--text-primary);
    border-color: color-mix(in srgb, var(--color-primary) 25%, var(--border-color));
  }

  .view-toggle button.active {
    background: var(--color-primary-subtle);
    border-color: color-mix(in srgb, var(--color-primary) 35%, var(--border-color));
    color: var(--text-primary);
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
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 360px), 1fr));
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
    background: color-mix(in srgb, var(--color-danger) 6%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-danger) 20%, var(--border-color));
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

  @media (max-width: 720px) {
    .page-header {
      flex-direction: column;
      align-items: stretch;
      margin-bottom: 1.25rem;
    }

    .header-content h1 {
      font-size: 1.35rem;
    }

    .view-toggle {
      display: flex;
      flex-wrap: wrap;
    }

    .view-toggle button {
      flex: 1 1 auto;
    }

    .btn-refresh {
      align-self: flex-end;
    }

    .last-updated {
      flex-wrap: wrap;
      margin-top: 1.25rem;
    }

    .loading-state {
      padding: 2.5rem 1rem;
    }
  }
</style>
