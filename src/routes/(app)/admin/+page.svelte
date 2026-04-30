<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { api } from '$lib/api/client';
  import type { AdminDashboardSummary } from '$lib/utils/adminDashboard';
  import {
    buildAdminDashboardModel,
    getAdminDashboardDataRequirements,
    summarizeAlerts,
    summarizeIncidents,
    summarizeInvoices,
    summarizePppoeAccounts,
    summarizeWorkOrders,
  } from '$lib/utils/adminDashboard';
  import { can, user } from '$lib/stores/auth';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';

  let loading = $state(true);
  let refreshing = $state(false);
  let lastLoadedAt = $state('');
  let summary = $state<AdminDashboardSummary>({});

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);

  const capabilities = $derived.by(() => ({
    teamRead: $can('read', 'team'),
    rolesRead: $can('read', 'roles'),
    settingsRead: $can('read', 'settings') || $can('update', 'settings'),
    customersRead: $can('read', 'customers') || $can('manage', 'customers'),
    billingRead: $can('read', 'billing') || $can('manage', 'billing'),
    workOrdersRead: $can('read', 'work_orders') || $can('manage', 'work_orders'),
    pppoeRead: $can('read', 'pppoe') || $can('manage', 'pppoe'),
    supportReadAll: $can('read_all', 'support'),
    networkNocRead: $can('read', 'network_noc') || $can('manage', 'network_noc'),
    networkAlertsRead: $can('read', 'network_alerts') || $can('manage', 'network_alerts'),
    networkIncidentsRead: $can('read', 'network_incidents') || $can('manage', 'network_incidents'),
    routerInventoryRead: $can('read', 'router_inventory') || $can('manage', 'router_inventory'),
    auditLogsRead: $can('read', 'audit_logs'),
    emailOutboxRead:
      $can('read', 'email_outbox') ||
      $can('retry', 'email_outbox') ||
      $can('delete', 'email_outbox'),
  }));

  const dashboardModel = $derived.by(() =>
    buildAdminDashboardModel({
      tenantPrefix,
      capabilities,
      summary,
    }),
  );

  const audienceContent = $derived.by(() => {
    if (dashboardModel.audience === 'admin') {
      return {
        titleKey: 'admin.dashboard.audience.admin.title',
        fallbackTitle: 'Tenant control room',
        subtitleKey: 'admin.dashboard.audience.admin.subtitle',
        fallbackSubtitle:
          'Keep access, billing, customer growth, and cross-team workload in one view.',
      };
    }
    if (dashboardModel.audience === 'operations') {
      return {
        titleKey: 'admin.dashboard.audience.operations.title',
        fallbackTitle: 'Field operations today',
        subtitleKey: 'admin.dashboard.audience.operations.subtitle',
        fallbackSubtitle:
          'Start with installations, PPPoE follow-up, and the customer context needed in the field.',
      };
    }
    if (dashboardModel.audience === 'support') {
      return {
        titleKey: 'admin.dashboard.audience.support.title',
        fallbackTitle: 'Customer service workspace',
        subtitleKey: 'admin.dashboard.audience.support.subtitle',
        fallbackSubtitle:
          'Prioritize billing follow-up, activation handoff, and open support conversations.',
      };
    }
    if (dashboardModel.audience === 'noc') {
      return {
        titleKey: 'admin.dashboard.audience.noc.title',
        fallbackTitle: 'Network watch',
        subtitleKey: 'admin.dashboard.audience.noc.subtitle',
        fallbackSubtitle: 'Focus on incidents, alerts, and monitored routers before they escalate.',
      };
    }
    return {
      titleKey: 'admin.dashboard.audience.hybrid.title',
      fallbackTitle: 'Operational snapshot',
      subtitleKey: 'admin.dashboard.audience.hybrid.subtitle',
      fallbackSubtitle:
        'Only the modules you can access are shown here, ordered by what likely needs attention first.',
    };
  });

  const hasVisibleContent = $derived.by(
    () =>
      dashboardModel.primaryStats.length > 0 ||
      dashboardModel.focusCards.length > 0 ||
      dashboardModel.quickActions.length > 0 ||
      dashboardModel.trendCards.length > 0,
  );

  onMount(() => {
    void initData();
  });

  async function initData() {
    const currentUser = get(user);
    const requirements = getAdminDashboardDataRequirements(capabilities);
    const nextSummary: AdminDashboardSummary = {};
    const tasks: Promise<void>[] = [];

    loading = !lastLoadedAt;
    refreshing = !!lastLoadedAt;

    if (requirements.team) {
      tasks.push(
        api.team
          .list()
          .then((rows) => {
            nextSummary.teamMembers = rows.length;
          })
          .catch((error) => console.warn('Failed to load admin team summary', error)),
      );
    }

    if (requirements.subscription && currentUser?.tenant_id) {
      tasks.push(
        api.plans
          .getSubscriptionDetails(currentUser.tenant_id)
          .then((value) => {
            nextSummary.subscription = value;
          })
          .catch((error) => console.warn('Failed to load admin subscription summary', error)),
      );
    }

    if (requirements.customers) {
      tasks.push(
        api.customers
          .list({ page: 1, perPage: 1 })
          .then((response) => {
            nextSummary.customerTotal = response.total;
          })
          .catch((error) =>
            console.warn('Failed to load customer totals for admin dashboard', error),
          ),
      );
    }

    if (requirements.lifecycle) {
      tasks.push(
        api.customers.observability
          .lifecycle()
          .then((response) => {
            nextSummary.activationsWaiting =
              getLifecycleStageCount(response.lifecycle_funnel, 'pending_installation') +
              getLifecycleStageCount(
                response.lifecycle_funnel,
                'installation_done_awaiting_payment',
                'awaiting_activation',
              );
          })
          .catch((error) =>
            console.warn('Failed to load lifecycle observability for admin dashboard', error),
          ),
      );
    }

    if (requirements.invoices) {
      tasks.push(
        api.payment
          .listCustomerPackageInvoices({ sort_by: 'due_date', sort_dir: 'asc' })
          .then((rows) => {
            nextSummary.invoice = summarizeInvoices(rows);
          })
          .catch((error) =>
            console.warn('Failed to load billing summary for admin dashboard', error),
          ),
      );
    }

    if (requirements.workOrders) {
      tasks.push(
        api.workOrders
          .list({ include_closed: true, limit: 200 })
          .then((rows) => {
            nextSummary.workOrders = summarizeWorkOrders(rows);
          })
          .catch((error) =>
            console.warn('Failed to load work order summary for admin dashboard', error),
          ),
      );
    }

    if (requirements.pppoe) {
      tasks.push(
        api.pppoe.accounts
          .list({ page: 1, per_page: 1000 })
          .then((response) => {
            nextSummary.pppoe = summarizePppoeAccounts(response.data);
          })
          .catch((error) =>
            console.warn('Failed to load PPPoE summary for admin dashboard', error),
          ),
      );
    }

    if (requirements.alerts) {
      tasks.push(
        api.mikrotik.alerts
          .list({ activeOnly: true, limit: 200 })
          .then((rows) => {
            nextSummary.alerts = summarizeAlerts(rows);
          })
          .catch((error) =>
            console.warn('Failed to load alert summary for admin dashboard', error),
          ),
      );
    }

    if (requirements.incidents) {
      tasks.push(
        api.mikrotik.incidents
          .list({ activeOnly: true, limit: 200 })
          .then((rows) => {
            nextSummary.incidents = summarizeIncidents(rows);
          })
          .catch((error) =>
            console.warn('Failed to load incident summary for admin dashboard', error),
          ),
      );
    }

    if (requirements.routers) {
      tasks.push(
        api.mikrotik.routers
          .list()
          .then((rows) => {
            nextSummary.routersTotal = rows.length;
          })
          .catch((error) =>
            console.warn('Failed to load router totals for admin dashboard', error),
          ),
      );
    }

    if (requirements.support) {
      tasks.push(
        api.support
          .stats()
          .then((value) => {
            nextSummary.support = value;
          })
          .catch((error) =>
            console.warn('Failed to load support stats for admin dashboard', error),
          ),
      );
    }

    try {
      await Promise.all(tasks);
      summary = nextSummary;
      lastLoadedAt = new Date().toISOString();
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  function getLifecycleStageCount(
    stages: Array<{ stage: string; count: number }> | undefined,
    ...wantedStages: string[]
  ) {
    if (!stages?.length) return 0;
    return stages
      .filter((item) => wantedStages.includes(item.stage))
      .reduce((total, item) => total + item.count, 0);
  }

  function formatMetricValue(value: string | number) {
    if (typeof value === 'number') {
      return new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 }).format(
        value,
      );
    }

    return value;
  }

  function formatLastLoaded(iso: string) {
    if (!iso) return '';
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  }

  function getTrendItemWidth(value: number, maxValue: number) {
    if (maxValue <= 0) return '0%';
    return `${Math.max(10, Math.round((value / maxValue) * 100))}%`;
  }

  function getTrendMax(items: Array<{ value: number }>) {
    return Math.max(...items.map((item) => item.value), 0);
  }
</script>

<div class="admin-dashboard fade-in">
  <div class="masthead">
    <div class="masthead-copy">
      <div class="eyebrow">
        {$t('admin.dashboard.eyebrow') || 'Adaptive admin dashboard'}
      </div>
      <h1>{$t(audienceContent.titleKey) || audienceContent.fallbackTitle}</h1>
      <p>{$t(audienceContent.subtitleKey) || audienceContent.fallbackSubtitle}</p>
    </div>

    <div class="masthead-tools">
      <div class="context-pill">
        <Icon name="activity" size={14} />
        <span
          >{$t('admin.dashboard.role_scoped') || 'Only modules you can access are shown here'}</span
        >
      </div>

      <button
        class="refresh-button"
        type="button"
        onclick={() => void initData()}
        disabled={refreshing}
      >
        <Icon name="refresh-cw" size={16} />
        <span
          >{refreshing
            ? $t('common.loading') || 'Loading...'
            : $t('common.refresh') || 'Refresh'}</span
        >
      </button>

      {#if lastLoadedAt}
        <div class="last-updated">
          {$t('admin.dashboard.last_updated') || 'Last updated'}: {formatLastLoaded(lastLoadedAt)}
        </div>
      {/if}
    </div>
  </div>

  {#if loading}
    <div class="loading-panel">
      <div class="spinner"></div>
      <p>{$t('admin.dashboard.loading') || 'Loading your workspace snapshot...'}</p>
    </div>
  {:else if !hasVisibleContent}
    <section class="empty-panel">
      <div class="empty-icon">
        <Icon name="lock" size={26} />
      </div>
      <h2>{$t('admin.dashboard.empty.title') || 'No dashboard modules available yet'}</h2>
      <p>
        {$t('admin.dashboard.empty.description') ||
          'This account does not currently expose a dashboard section. Ask an owner or admin to grant one or more tenant permissions.'}
      </p>
    </section>
  {:else}
    {#if dashboardModel.primaryStats.length > 0}
      <section class="section-block">
        <div class="section-heading">
          <div>
            <div class="section-kicker">
              {$t('admin.dashboard.sections.primary.kicker') || 'Snapshot'}
            </div>
            <h2>{$t('admin.dashboard.sections.primary.title') || 'Selected KPIs'}</h2>
          </div>
        </div>

        <div class="stats-grid">
          {#each dashboardModel.primaryStats as card (card.id)}
            <button
              class={`metric-card tone-${card.tone}`}
              type="button"
              onclick={() => goto(card.href)}
            >
              <div class="metric-topline">
                <span class="metric-icon">
                  <Icon name={card.icon} size={18} />
                </span>
                {#if card.badge}
                  <span class="badge">{card.badge}</span>
                {/if}
              </div>
              <div class="metric-title">{$t(card.titleKey) || card.fallbackTitle}</div>
              <div class="metric-value">{formatMetricValue(card.value)}</div>
              {#if card.meta}
                <div class="metric-meta">
                  <strong>{card.meta.value}</strong>
                  <span>{$t(card.meta.labelKey) || card.meta.fallbackLabel}</span>
                </div>
              {/if}
            </button>
          {/each}
        </div>
      </section>
    {/if}

    {#if dashboardModel.focusCards.length > 0}
      <section class="section-block">
        <div class="section-heading">
          <div>
            <div class="section-kicker">
              {$t('admin.dashboard.sections.focus.kicker') || 'Action'}
            </div>
            <h2>{$t('admin.dashboard.sections.focus.title') || 'My Focus Today'}</h2>
          </div>
        </div>

        <div class="focus-grid">
          {#each dashboardModel.focusCards as card (card.id)}
            <button
              class={`focus-card tone-${card.tone}`}
              type="button"
              onclick={() => goto(card.href)}
            >
              <div class="focus-title-row">
                <span class="focus-title">{$t(card.titleKey) || card.fallbackTitle}</span>
                <span class="focus-value">{formatMetricValue(card.value)}</span>
              </div>
              <p>{$t(card.descriptionKey) || card.fallbackDescription}</p>
              <div class="focus-link">
                <span>{$t('admin.dashboard.open_area') || 'Open area'}</span>
                <Icon name="arrow-right" size={14} />
              </div>
            </button>
          {/each}
        </div>
      </section>
    {/if}

    <div class="split-layout">
      {#if dashboardModel.quickActions.length > 0}
        <section class="section-block section-actions">
          <div class="section-heading">
            <div>
              <div class="section-kicker">
                {$t('admin.dashboard.sections.actions.kicker') || 'Go next'}
              </div>
              <h2>{$t('admin.dashboard.sections.actions.title') || 'Quick Actions'}</h2>
            </div>
          </div>

          <div class="action-list">
            {#each dashboardModel.quickActions as action (action.id)}
              <button class="action-row" type="button" onclick={() => goto(action.href)}>
                <span class={`action-icon tone-${action.tone}`}>
                  <Icon name={action.icon} size={16} />
                </span>
                <span class="action-copy">
                  <strong>{$t(action.titleKey) || action.fallbackTitle}</strong>
                  <small>{$t(action.descriptionKey) || action.fallbackDescription}</small>
                </span>
                <Icon name="arrow-right" size={14} />
              </button>
            {/each}
          </div>
        </section>
      {/if}

      {#if dashboardModel.trendCards.length > 0}
        <section class="section-block section-trends">
          <div class="section-heading">
            <div>
              <div class="section-kicker">
                {$t('admin.dashboard.sections.trends.kicker') || 'Distribution'}
              </div>
              <h2>{$t('admin.dashboard.sections.trends.title') || 'Compact trends'}</h2>
            </div>
          </div>

          <div class="trend-list">
            {#each dashboardModel.trendCards as trend (trend.id)}
              <button class="trend-card" type="button" onclick={() => goto(trend.href)}>
                <div class="trend-title">{$t(trend.titleKey) || trend.fallbackTitle}</div>
                <div class="trend-items">
                  {#each trend.items as item (item.id)}
                    <div class="trend-row">
                      <div class="trend-row-top">
                        <span>{$t(item.labelKey) || item.fallbackLabel}</span>
                        <strong>{item.value}</strong>
                      </div>
                      <div class="trend-track">
                        <div
                          class={`trend-bar tone-${item.tone}`}
                          style={`width: ${getTrendItemWidth(item.value, getTrendMax(trend.items))}`}
                        ></div>
                      </div>
                    </div>
                  {/each}
                </div>
              </button>
            {/each}
          </div>
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  .admin-dashboard {
    --emerald: #10b981;
    --emerald-soft: rgba(34, 197, 94, 0.14);
    --amber: #f59e0b;
    --amber-soft: rgba(245, 158, 11, 0.14);
    --cyan: #38bdf8;
    --cyan-soft: rgba(56, 189, 248, 0.11);
    --indigo: var(--color-primary);
    --indigo-soft: rgba(139, 156, 255, 0.12);
    --rose: #f43f5e;
    --rose-soft: rgba(244, 63, 94, 0.14);
    --slate: #94a3b8;
    --slate-soft: rgba(148, 163, 184, 0.12);
    padding: clamp(1rem, 2.2vw, 2rem);
    max-width: 1440px;
    margin: 0 auto;
  }

  .masthead {
    display: flex;
    justify-content: space-between;
    gap: 1.5rem;
    align-items: flex-start;
    margin-bottom: 1.5rem;
  }

  .masthead-copy {
    max-width: 720px;
  }

  .eyebrow,
  .section-kicker {
    color: var(--color-primary);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 0.72rem;
    font-weight: 700;
  }

  .masthead h1 {
    margin: 0.45rem 0 0.65rem;
    font-size: clamp(1.65rem, 2.3vw, 2.35rem);
    line-height: 1.12;
  }

  .masthead p {
    margin: 0;
    max-width: 62ch;
    color: var(--text-secondary);
    font-size: 1rem;
    line-height: 1.65;
  }

  .masthead-tools {
    min-width: 280px;
    display: grid;
    gap: 0.85rem;
    justify-items: end;
  }

  .context-pill,
  .last-updated {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    padding: 0.7rem 0.95rem;
    border-radius: var(--radius-md);
    border: 1px solid rgba(148, 163, 184, 0.15);
    background: var(--bg-secondary);
    color: var(--text-secondary);
    font-size: 0.88rem;
  }

  .refresh-button {
    display: inline-flex;
    align-items: center;
    gap: 0.65rem;
    border: 1px solid color-mix(in srgb, var(--color-primary) 24%, var(--border-color));
    background: color-mix(in srgb, var(--color-primary) 13%, var(--bg-secondary));
    color: var(--text-primary);
    padding: 0.8rem 1rem;
    border-radius: var(--radius-md);
    cursor: pointer;
    font-weight: 600;
  }

  .refresh-button:disabled {
    opacity: 0.65;
    cursor: wait;
  }

  .loading-panel,
  .empty-panel,
  .section-block {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
  }

  .loading-panel,
  .empty-panel {
    display: grid;
    justify-items: center;
    gap: 0.8rem;
    text-align: center;
    padding: 3rem 1.5rem;
  }

  .empty-panel h2,
  .section-heading h2 {
    margin: 0;
  }

  .empty-panel p {
    margin: 0;
    max-width: 64ch;
    color: var(--text-secondary);
    line-height: 1.65;
  }

  .empty-icon {
    width: 3.5rem;
    height: 3.5rem;
    display: grid;
    place-items: center;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.12);
    color: var(--text-primary);
  }

  .spinner {
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
    border: 3px solid rgba(148, 163, 184, 0.16);
    border-top-color: var(--color-primary);
    animation: spin 1s linear infinite;
  }

  .section-block {
    padding: 1.35rem;
    margin-bottom: 1.25rem;
  }

  .section-heading {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-end;
    margin-bottom: 1rem;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
  }

  .metric-card,
  .focus-card,
  .action-row,
  .trend-card {
    width: 100%;
    border: 1px solid rgba(148, 163, 184, 0.12);
    background: color-mix(in srgb, var(--bg-tertiary) 82%, transparent);
    color: var(--text-primary);
    border-radius: var(--radius-md);
    cursor: pointer;
    text-align: left;
    transition:
      transform 0.18s ease,
      border-color 0.18s ease,
      background 0.18s ease;
  }

  .metric-card:hover,
  .focus-card:hover,
  .action-row:hover,
  .trend-card:hover {
    transform: translateY(-1px);
    border-color: color-mix(in srgb, var(--color-primary) 34%, var(--border-color));
  }

  .metric-card {
    padding: 1.1rem;
  }

  .metric-topline {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .metric-icon,
  .action-icon {
    width: 2.2rem;
    height: 2.2rem;
    border-radius: 8px;
    display: grid;
    place-items: center;
  }

  .metric-title {
    color: var(--text-secondary);
    font-size: 0.93rem;
    margin-bottom: 0.45rem;
  }

  .metric-value {
    font-size: clamp(1.5rem, 2vw, 2.2rem);
    font-weight: 700;
    line-height: 1.1;
    word-break: break-word;
  }

  .metric-meta {
    margin-top: 0.8rem;
    display: flex;
    gap: 0.45rem;
    align-items: baseline;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .metric-meta strong {
    color: var(--text-primary);
  }

  .badge {
    display: inline-flex;
    align-items: center;
    padding: 0.25rem 0.6rem;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.08);
    font-size: 0.74rem;
    font-weight: 700;
    text-transform: capitalize;
  }

  .focus-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 1rem;
  }

  .focus-card {
    padding: 1.15rem;
    min-height: 180px;
    display: grid;
    align-content: space-between;
    gap: 0.9rem;
  }

  .focus-title-row {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: flex-start;
  }

  .focus-title {
    font-size: 1rem;
    font-weight: 700;
  }

  .focus-value {
    font-size: 1.35rem;
    font-weight: 800;
  }

  .focus-card p {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.6;
  }

  .focus-link {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    color: var(--text-primary);
    font-weight: 600;
  }

  .split-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.15fr) minmax(0, 0.85fr);
    gap: 1.25rem;
  }

  .action-list,
  .trend-list {
    display: grid;
    gap: 0.85rem;
  }

  .action-row {
    padding: 0.95rem 1rem;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.85rem;
  }

  .action-copy {
    display: grid;
    gap: 0.22rem;
  }

  .action-copy small {
    color: var(--text-secondary);
    line-height: 1.55;
  }

  .trend-card {
    padding: 1rem;
  }

  .trend-title {
    font-weight: 700;
    margin-bottom: 0.9rem;
  }

  .trend-items {
    display: grid;
    gap: 0.8rem;
  }

  .trend-row {
    display: grid;
    gap: 0.38rem;
  }

  .trend-row-top {
    display: flex;
    justify-content: space-between;
    gap: 0.8rem;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .trend-row-top strong {
    color: var(--text-primary);
  }

  .trend-track {
    height: 0.45rem;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.12);
    overflow: hidden;
  }

  .trend-bar {
    height: 100%;
    border-radius: 999px;
  }

  .tone-emerald {
    background: var(--emerald-soft);
    color: var(--emerald);
  }

  .tone-amber {
    background: var(--amber-soft);
    color: var(--amber);
  }

  .tone-cyan {
    background: var(--cyan-soft);
    color: var(--cyan);
  }

  .tone-indigo {
    background: var(--indigo-soft);
    color: var(--indigo);
  }

  .tone-rose {
    background: var(--rose-soft);
    color: var(--rose);
  }

  .tone-slate {
    background: var(--slate-soft);
    color: var(--slate);
  }

  @media (max-width: 960px) {
    .admin-dashboard {
      padding: 1rem;
    }

    .masthead,
    .split-layout {
      grid-template-columns: 1fr;
      display: grid;
    }

    .masthead-tools {
      justify-items: start;
    }
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
