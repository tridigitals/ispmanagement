<script lang="ts">
  import { isAdmin, can, user } from '$lib/stores/auth';
  import { team, settings, api } from '$lib/api/client';
  import type { TenantSubscriptionDetails } from '$lib/api/client';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { page } from '$app/stores';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { t } from 'svelte-i18n';

  let memberCount = $state(0);
  let settingsCount = $state(0);
  let subscription = $state<TenantSubscriptionDetails | null>(null);
  let loading = $state(true);

  let tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  let tenantPrefix = $derived(tenantCtx.tenantPrefix);

  onMount(() => {
    initData();
  });

  async function initData() {
    if (!get(isAdmin)) {
      goto('/unauthorized');
      return;
    }

    try {
      const membersPromise = team.list();
      const settingsPromise = get(can)('read', 'settings') ? settings.getAll() : Promise.resolve([]);
      const [membersRes, settingsRes] = await Promise.all([membersPromise, settingsPromise]);

      memberCount = membersRes.length;
      settingsCount = settingsRes.length;

      const currentUser = get(user);
      if (currentUser?.tenant_id) {
        subscription = await api.plans.getSubscriptionDetails(currentUser.tenant_id);
      }
    } catch (err) {
      console.error('Failed to load admin stats:', err);
    } finally {
      loading = false;
    }
  }

  function formatBytes(bytes: number) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }
</script>

<div class="admin-content fade-in">
  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <p>{$t('admin.overview.loading') || 'Loading system data...'}</p>
    </div>
  {:else}
    <div class="stats-grid">
      <div
        class="stat-card emerald"
        onclick={() => goto(`${tenantPrefix}/admin/team`)}
        role="button"
        tabindex="0"
        onkeydown={(e) =>
          (e.key === 'Enter' || e.key === ' ') && goto(`${tenantPrefix}/admin/team`)}
      >
        <div class="stat-icon">
          <Icon name="users" size={32} />
        </div>
        <div class="stat-content">
          <span class="stat-value">{memberCount}</span>
          <span class="stat-label">
            {$t('admin.overview.stats.team_members') || 'Team Members'}
          </span>
        </div>
      </div>

      <div
        class="stat-card cyan"
        onclick={() => goto(`${tenantPrefix}/admin/settings`)}
        role="button"
        tabindex="0"
        onkeydown={(e) =>
          (e.key === 'Enter' || e.key === ' ') && goto(`${tenantPrefix}/admin/settings`)}
      >
        <div class="stat-icon">
          <Icon name="settings" size={32} />
        </div>
        <div class="stat-content">
          <span class="stat-value">{settingsCount}</span>
          <span class="stat-label">
            {$t('admin.overview.stats.global_settings') || 'Global Settings'}
          </span>
        </div>
      </div>

      <!-- Subscription Status Card -->
      <div
        class="stat-card indigo"
        onclick={() => goto(`${tenantPrefix}/admin/subscription`)}
        role="button"
        tabindex="0"
        onkeydown={(e) =>
          (e.key === 'Enter' || e.key === ' ') && goto(`${tenantPrefix}/admin/subscription`)}
      >
        <div class="stat-icon">
          <Icon name="credit-card" size={32} />
        </div>
        <div class="stat-content w-full">
          {#if subscription}
            <div class="plan-header">
              <span class="plan-name">{subscription.plan_name}</span>
              <span class="status-pill active">{subscription.status}</span>
            </div>
            <div class="progress-container">
              <div
                class="progress-bar"
                style="width: {subscription.storage_limit
                  ? Math.min(100, (subscription.storage_usage / subscription.storage_limit) * 100)
                  : 0}%"
              ></div>
            </div>
            <span class="usage-text">
              {formatBytes(subscription.storage_usage)}
              {$t('admin.overview.stats.used') || 'used'}
            </span>
          {:else}
            <span class="stat-value">
              {$t('admin.overview.stats.free') || 'Free'}
            </span>
            <span class="stat-label">
              {$t('admin.overview.stats.plan_status') || 'Plan Status'}
            </span>
          {/if}
        </div>
      </div>
    </div>

    <div class="section-header">
      <h2>{$t('admin.overview.quick_actions.title') || 'Quick Actions'}</h2>
    </div>

    <div class="actions-grid">
      {#if $can('read', 'team')}
        <button class="action-card" onclick={() => goto(`${tenantPrefix}/admin/team`)}>
          <div class="action-icon accent-emerald">
            <Icon name="users" size={18} />
          </div>
          <h3>
            {$t('admin.overview.quick_actions.team.title') || 'Manage Team'}
          </h3>
          <p>
            {$t('admin.overview.quick_actions.team.desc') || 'View, edit, and invite team members.'}
          </p>
        </button>
      {/if}

      {#if $can('read', 'roles')}
        <button class="action-card" onclick={() => goto(`${tenantPrefix}/admin/roles`)}>
          <div class="action-icon accent-amber">
            <Icon name="lock" size={18} />
          </div>
          <h3>
            {$t('admin.overview.quick_actions.roles.title') || 'Roles & Permissions'}
          </h3>
          <p>
            {$t('admin.overview.quick_actions.roles.desc') || 'Manage roles and access control.'}
          </p>
        </button>
      {/if}

      {#if $can('read', 'settings')}
        <button class="action-card" onclick={() => goto(`${tenantPrefix}/admin/settings`)}>
          <div class="action-icon accent-cyan">
            <Icon name="settings" size={18} />
          </div>
          <h3>
            {$t('admin.overview.quick_actions.settings.title') || 'Global Settings'}
          </h3>
          <p>
            {$t('admin.overview.quick_actions.settings.desc') ||
              'Configure application policies and defaults.'}
          </p>
        </button>
      {/if}

      <button class="action-card" onclick={() => goto(`${tenantPrefix}/admin/subscription`)}>
        <div class="action-icon accent-indigo">
          <Icon name="credit-card" size={18} />
        </div>
        <h3>{$t('admin.overview.quick_actions.billing.title') || 'Billing'}</h3>
        <p>
          {$t('admin.overview.quick_actions.billing.desc') || 'Manage subscription and invoices.'}
        </p>
      </button>
    </div>
  {/if}
</div>

<style>
  .admin-content {
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
    color: var(--text-primary);
    --accent-emerald: #10b981;
    --accent-cyan: #22d3ee;
    --accent-indigo: #6366f1;
    --accent-amber: #f59e0b;
    --glass: rgba(255, 255, 255, 0.04);
    --glass-border: rgba(255, 255, 255, 0.08);
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1.5rem;
    margin-bottom: 3rem;
  }

  .stat-card {
    background: linear-gradient(145deg, var(--bg-surface), #0b0c10);
    border: 1px solid var(--glass-border);
    border-radius: 16px;
    padding: 1.5rem;
    display: flex;
    align-items: center;
    gap: 1.25rem;
    cursor: pointer;
    transition: all 0.2s;
    color: var(--text-primary);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
  }

  .stat-card:hover {
    border-color: var(--color-primary);
    transform: translateY(-2px);
  }

  .stat-card.emerald {
    background:
      radial-gradient(circle at 20% 20%, rgba(16, 185, 129, 0.18), transparent 55%), #0c1411;
    border-color: rgba(16, 185, 129, 0.25);
  }

  .stat-card.cyan {
    background:
      radial-gradient(circle at 20% 20%, rgba(34, 211, 238, 0.2), transparent 55%), #0c1316;
    border-color: rgba(34, 211, 238, 0.25);
  }

  .stat-card.indigo {
    background:
      radial-gradient(circle at 20% 20%, rgba(99, 102, 241, 0.2), transparent 55%), #0e0c16;
    border-color: rgba(99, 102, 241, 0.25);
  }

  .stat-icon {
    font-size: 2rem;
    width: 56px;
    height: 56px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--glass-border);
    border-radius: 14px;
  }

  .stat-card.emerald .stat-icon {
    color: var(--accent-emerald);
    background:
      radial-gradient(circle at 20% 20%, rgba(16, 185, 129, 0.18), transparent 60%),
      rgba(255, 255, 255, 0.03);
    border-color: rgba(16, 185, 129, 0.35);
  }

  .stat-card.cyan .stat-icon {
    color: var(--accent-cyan);
    background:
      radial-gradient(circle at 20% 20%, rgba(34, 211, 238, 0.2), transparent 60%),
      rgba(255, 255, 255, 0.03);
    border-color: rgba(34, 211, 238, 0.35);
  }

  .stat-card.indigo .stat-icon {
    color: var(--accent-indigo);
    background:
      radial-gradient(circle at 20% 20%, rgba(99, 102, 241, 0.2), transparent 60%),
      rgba(255, 255, 255, 0.03);
    border-color: rgba(99, 102, 241, 0.35);
  }

  /* ---------- Light Theme Tweaks ---------- */
  :global([data-theme='light']) .admin-content {
    --glass: rgba(0, 0, 0, 0.03);
    --glass-border: rgba(0, 0, 0, 0.08);
  }

  :global([data-theme='light']) .stat-card {
    background: linear-gradient(135deg, #ffffff, #f7f7fb);
    border-color: rgba(0, 0, 0, 0.06);
    box-shadow:
      0 10px 30px rgba(0, 0, 0, 0.08),
      0 0 0 1px rgba(255, 255, 255, 0.6);
  }

  :global([data-theme='light']) .stat-card.emerald {
    background:
      radial-gradient(circle at 20% 20%, rgba(16, 185, 129, 0.08), transparent 55%), #ffffff;
    border-color: rgba(16, 185, 129, 0.18);
  }

  :global([data-theme='light']) .stat-card.cyan {
    background:
      radial-gradient(circle at 20% 20%, rgba(34, 211, 238, 0.08), transparent 55%), #ffffff;
    border-color: rgba(34, 211, 238, 0.18);
  }

  :global([data-theme='light']) .stat-card.indigo {
    background:
      radial-gradient(circle at 20% 20%, rgba(99, 102, 241, 0.08), transparent 55%), #ffffff;
    border-color: rgba(99, 102, 241, 0.18);
  }

  :global([data-theme='light']) .stat-icon {
    background: rgba(99, 102, 241, 0.06);
  }

  :global([data-theme='light']) .action-card {
    background: #ffffff;
    border-color: rgba(0, 0, 0, 0.06);
    box-shadow:
      0 10px 30px rgba(0, 0, 0, 0.08),
      0 1px 0 rgba(255, 255, 255, 0.9);
  }

  :global([data-theme='light']) .action-icon {
    background: rgba(0, 0, 0, 0.03);
    border-color: rgba(0, 0, 0, 0.08);
  }

  .stat-content {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .stat-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .stat-label {
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  /* Subscription Widget Specifics */
  .plan-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .plan-name {
    font-weight: 700;
    font-size: 1.1rem;
  }

  .status-pill {
    font-size: 0.7rem;
    padding: 0.15rem 0.5rem;
    border-radius: 10px;
    text-transform: uppercase;
    font-weight: 800;
  }

  .status-pill.active {
    background: rgba(34, 197, 94, 0.2);
    color: #22c55e;
  }

  .progress-container {
    height: 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    overflow: hidden;
    margin-bottom: 0.4rem;
  }

  .progress-bar {
    height: 100%;
    background: var(--color-primary);
    border-radius: 3px;
  }

  .usage-text {
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  /* Actions Grid */
  .section-header {
    margin-bottom: 1.5rem;
  }

  .section-header h2 {
    font-size: 1.25rem;
    font-weight: 600;
  }

  .actions-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1.5rem;
  }

  .action-card {
    background: var(--glass);
    border: 1px solid var(--glass-border);
    border-radius: 14px;
    padding: 1.5rem;
    text-align: left;
    cursor: pointer;
    transition: all 0.2s;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    color: var(--text-primary);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.25);
  }

  .action-card:hover {
    border-color: var(--color-primary);
    box-shadow: 0 14px 32px rgba(99, 102, 241, 0.25);
    transform: translateY(-2px);
  }

  .action-icon {
    width: 40px;
    height: 40px;
    border-radius: 12px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 0.5rem;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--glass-border);
  }

  .accent-emerald {
    color: var(--accent-emerald);
    border-color: rgba(16, 185, 129, 0.35);
    background: radial-gradient(circle at 20% 20%, rgba(16, 185, 129, 0.12), transparent 55%);
  }

  .accent-amber {
    color: var(--accent-amber);
    border-color: rgba(245, 158, 11, 0.35);
    background: radial-gradient(circle at 20% 20%, rgba(245, 158, 11, 0.12), transparent 55%);
  }

  .accent-cyan {
    color: var(--accent-cyan);
    border-color: rgba(34, 211, 238, 0.35);
    background: radial-gradient(circle at 20% 20%, rgba(34, 211, 238, 0.12), transparent 55%);
  }

  .accent-indigo {
    color: var(--accent-indigo);
    border-color: rgba(99, 102, 241, 0.35);
    background: radial-gradient(circle at 20% 20%, rgba(99, 102, 241, 0.12), transparent 55%);
  }

  .action-card h3 {
    font-size: 1rem;
    font-weight: 600;
    margin: 0;
  }

  .action-card p {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin: 0;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 40vh;
    gap: 1rem;
    color: var(--text-secondary);
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

  .fade-in {
    animation: fadeIn 0.4s ease-out;
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

  .w-full {
    width: 100%;
  }
</style>
