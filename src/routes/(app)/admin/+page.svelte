<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { api } from '$lib/api/client';
  import type { Invoice } from '$lib/api/types';
  import { fetchAllRows } from '$lib/utils/fetchAllPages';
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
  const heroSummary = $derived.by(() => {
    const focus = dashboardModel.focusCards.length;
    const actions = dashboardModel.quickActions.length;
    return ($t('admin.overview.hero_summary', { values: { focus, actions } }) || `${focus} priorities • ${actions} actions`);
  });

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

    if (requirements.subscription) {
      tasks.push(
        api.plans
          .getSubscriptionDetails()
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
        /* `per_page` WAJIB diisi. Backend memakai `per_page.unwrap_or(25)` lalu
           `clamp(1, 100)`, jadi tanpa argumen ringkasan tagihan tenant dihitung
           dari 25 baris pertama saja. Di DB produksi bedanya 25 vs 485 invoice. */
        fetchAllRows<Invoice>((page, per_page) =>
          api.payment.listCustomerPackageInvoices({
            sort_by: 'due_date',
            sort_dir: 'asc',
            page,
            per_page,
          }),
        )
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

  function getTrendShare(value: number, total: number) {
    if (total <= 0) return '0%';
    return `${Math.round((value / total) * 100)}%`;
  }

  function getTrendMax(items: Array<{ value: number }>) {
    return Math.max(...items.map((item) => item.value), 0);
  }

  function getTrendTotal(items: Array<{ value: number }>) {
    return items.reduce((total, item) => total + item.value, 0);
  }

  function getRingLength(radius: number) {
    return 2 * Math.PI * radius;
  }

  function getRingOffset(value: number, total: number, radius: number) {
    if (total <= 0) return getRingLength(radius);
    return getRingLength(radius) * (1 - value / total);
  }
</script>

<div class="admin-dashboard dashboard-shell fade-in">
  <section class="executive-hero">
    <div class="hero-copy">
      <h1>{$t(audienceContent.titleKey) || audienceContent.fallbackTitle}</h1>
      <div class="hero-summary">{heroSummary}</div>
    </div>

    <div class="hero-side">
      <div class="hero-actions">
        {#if lastLoadedAt}
          <div class="hero-updated">
            <span class="hero-meta-label"
              >{$t('admin.dashboard.last_updated')}</span
            >
            <strong>{formatLastLoaded(lastLoadedAt)}</strong>
          </div>
        {/if}
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
      </div>

    </div>
  </section>

  {#if loading}
    <div class="loading-panel">
      <div class="spinner"></div>
      <p>{$t('admin.dashboard.loading')}</p>
    </div>
  {:else if !hasVisibleContent}
    <section class="empty-panel">
      <div class="empty-icon">
        <Icon name="lock" size={26} />
      </div>
      <h2>{$t('admin.dashboard.empty.title')}</h2>
      <p>
        {$t('admin.dashboard.empty.description')}
      </p>
    </section>
  {:else}
    {#if dashboardModel.primaryStats.length > 0}
      <section class="section-block kpi-strip">
        <div class="section-heading">
          <div>
            <div class="section-kicker">{$t('admin.dashboard.sections.primary.kicker')}</div>
            <h2>{$t('admin.dashboard.sections.primary.title')}</h2>
          </div>
          <div class="section-meta">{dashboardModel.primaryStats.length} {$t('admin.dashboard.sections.primary.meta')}</div>
        </div>

        <div class="stats-grid">
          {#each dashboardModel.primaryStats as card (card.id)}
            <button
              class={`metric-card tone-${card.tone}`}
              type="button"
              onclick={() => goto(card.href)}
            >
              <div class="metric-topline">
                <span class={`metric-icon tone-${card.tone}`}>
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
      <section class="section-block focus-band">
        <div class="section-heading">
          <div>
            <div class="section-kicker">{$t('admin.dashboard.sections.focus.kicker')}</div>
            <h2>{$t('admin.dashboard.sections.focus.title')}</h2>
          </div>
          <div class="section-meta">{dashboardModel.focusCards.length} {$t('admin.dashboard.sections.focus.meta')}</div>
        </div>

        <div class="focus-grid">
          {#each dashboardModel.focusCards as card (card.id)}
            <button
              class={`focus-card tone-${card.tone}`}
              type="button"
              onclick={() => goto(card.href)}
            >
              <div class="focus-card-top">
                <span class={`focus-tone tone-${card.tone}`}>
                  <Icon name={card.icon} size={16} />
                </span>
                <span class="focus-value">{formatMetricValue(card.value)}</span>
              </div>
              <div class="focus-title-row">
                <span class="focus-title">{$t(card.titleKey) || card.fallbackTitle}</span>
              </div>
              <p>{$t(card.descriptionKey) || card.fallbackDescription}</p>
              <div class="focus-link">
                <span>{$t('admin.dashboard.open_area')}</span>
                <Icon name="arrow-right" size={14} />
              </div>
            </button>
          {/each}
        </div>
      </section>
    {/if}

    <div class="split-layout decision-grid">
      {#if dashboardModel.quickActions.length > 0}
        <section class="section-block section-actions">
          <div class="section-heading">
            <div>
              <div class="section-kicker">{$t('admin.dashboard.sections.actions.kicker')}</div>
              <h2>{$t('admin.dashboard.sections.actions.title')}</h2>
            </div>
            <div class="section-meta">{dashboardModel.quickActions.length} {$t('admin.dashboard.sections.actions.meta')}</div>
          </div>

          <div class="action-list action-rail">
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
              <div class="section-kicker">{$t('admin.dashboard.sections.trends.kicker')}</div>
              <h2>{$t('admin.dashboard.sections.trends.title')}</h2>
            </div>
            <div class="section-meta">{dashboardModel.trendCards.length} {$t('admin.dashboard.sections.trends.meta')}</div>
          </div>

          <div class="trend-list">
            {#each dashboardModel.trendCards as trend (trend.id)}
              <button class="trend-card" type="button" onclick={() => goto(trend.href)}>
                <div class="trend-card-top">
                  <div class="trend-title-group">
                    <div class="trend-title">{$t(trend.titleKey) || trend.fallbackTitle}</div>
                    <div class="trend-caption">{$t('admin.dashboard.sections.trends.tracked_items')}</div>
                  </div>
                  {#if trend.items[0]}
                    <div class="trend-visual">
                      <div class="trend-ring">
                        <svg viewBox="0 0 72 72" aria-hidden="true">
                          <circle class="trend-ring-base" cx="36" cy="36" r="26"></circle>
                          <circle
                            class={`trend-ring-value tone-${trend.items[0].tone}`}
                            cx="36"
                            cy="36"
                            r="26"
                            stroke-dasharray={getRingLength(26)}
                            stroke-dashoffset={getRingOffset(
                              trend.items[0].value,
                              getTrendTotal(trend.items),
                              26,
                            )}
                          ></circle>
                        </svg>
                        <div class="trend-ring-label">
                          <strong>{trend.items[0].value}</strong>
                          <span>{$t(trend.items[0].labelKey) || trend.items[0].fallbackLabel}</span>
                        </div>
                      </div>
                    </div>
                  {/if}
                </div>
                <div class="trend-items">
                  {#each trend.items as item (item.id)}
                    <div class="trend-row">
                      <div class="trend-row-top">
                        <span>{$t(item.labelKey) || item.fallbackLabel}</span>
                        <div class="trend-row-values">
                          <span class="trend-share">
                            {getTrendShare(item.value, getTrendTotal(trend.items))}
                          </span>
                          <strong>{item.value}</strong>
                        </div>
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
    padding: clamp(1rem, 2.4vw, 2.1rem);
    max-width: 1480px;
    margin: 0 auto;
  }

  .dashboard-shell {
    display: grid;
    gap: 1.15rem;
  }

  .executive-hero {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(320px, 0.8fr);
    gap: 0.8rem;
    align-items: end;
    padding: 0.15rem 0 0.35rem;
  }

  .hero-copy {
    display: grid;
    align-content: start;
    gap: 0.38rem;
  }

  .hero-copy h1 {
    margin: 0;
    max-width: 14ch;
    font-size: clamp(1.45rem, 1.7vw, 1.9rem);
    line-height: 1.04;
    letter-spacing: -0.025em;
  }

  .hero-summary {
    color: color-mix(in srgb, var(--text-secondary) 88%, white 12%);
    font-size: 0.68rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .hero-side {
    display: grid;
    gap: 0.55rem;
    align-content: start;
  }

  .hero-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.55rem;
    flex-wrap: wrap;
  }

  .hero-updated {
    display: grid;
    gap: 0.14rem;
    min-width: 180px;
    padding: 0.52rem 0.7rem;
    border: 1px solid color-mix(in srgb, var(--border-color) 84%, transparent);
    border-radius: 10px;
    background: color-mix(in srgb, var(--bg-surface) 72%, transparent);
  }

  .hero-meta-label {
    color: var(--text-secondary);
    font-size: 0.66rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .hero-updated strong {
    color: var(--text-primary);
    font-size: 0.78rem;
    line-height: 1.25;
  }

  .refresh-button {
    display: inline-flex;
    align-items: center;
    gap: 0.65rem;
    border: 1px solid color-mix(in srgb, var(--color-primary) 24%, var(--border-color));
    background: color-mix(in srgb, var(--color-primary) 10%, var(--bg-secondary));
    color: var(--text-primary);
    padding: 0.56rem 0.78rem;
    border-radius: 10px;
    cursor: pointer;
    font-weight: 600;
    font-size: 0.8rem;
  }

  .refresh-button:disabled {
    opacity: 0.65;
    cursor: wait;
  }

  .loading-panel,
  .empty-panel,
  .section-block {
    background: color-mix(in srgb, var(--bg-secondary) 96%, #09111d 4%);
    border: 1px solid color-mix(in srgb, var(--border-color) 84%, transparent);
    border-radius: 15px;
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
    padding: 1.05rem;
    margin-bottom: 0;
  }

  .section-heading {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-end;
    margin-bottom: 0.95rem;
  }

  .section-heading h2 {
    font-size: 1.38rem;
    line-height: 1.1;
    letter-spacing: -0.02em;
  }

  .section-meta {
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    min-height: 28px;
    padding: 0 0.7rem;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--border-color) 84%, transparent);
    color: var(--text-secondary);
    font-size: 0.72rem;
    font-weight: 700;
    background: color-mix(in srgb, var(--bg-surface) 78%, transparent);
  }

  .section-kicker {
    margin-bottom: 0.34rem;
    color: #93c5fd;
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.9rem;
  }

  .metric-card,
  .focus-card,
  .action-row,
  .trend-card {
    width: 100%;
    border: 1px solid color-mix(in srgb, var(--border-color) 84%, transparent);
    background: color-mix(in srgb, var(--bg-tertiary) 88%, transparent);
    color: var(--text-primary);
    border-radius: 14px;
    cursor: pointer;
    text-align: left;
    transition:
      transform 0.18s ease,
      border-color 0.18s ease,
      background 0.18s ease,
      box-shadow 0.18s ease;
  }

  .metric-card:hover,
  .focus-card:hover,
  .action-row:hover,
  .trend-card:hover {
    transform: translateY(-1px);
    border-color: color-mix(in srgb, var(--color-primary) 34%, var(--border-color));
    box-shadow: 0 14px 26px rgba(2, 6, 23, 0.16);
  }

  .metric-card {
    padding: 1rem;
    min-height: 168px;
    display: grid;
    align-content: space-between;
  }

  .metric-topline {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .metric-icon,
  .action-icon,
  .focus-tone {
    width: 2rem;
    height: 2rem;
    border-radius: 8px;
    display: grid;
    place-items: center;
  }

  .metric-title {
    color: var(--text-secondary);
    font-size: 0.8rem;
    margin-bottom: 0.45rem;
    line-height: 1.35;
  }

  .metric-value {
    font-size: clamp(1.8rem, 2vw, 2.3rem);
    font-weight: 800;
    line-height: 1;
    letter-spacing: -0.03em;
    word-break: break-word;
  }

  .metric-meta {
    margin-top: 0.8rem;
    display: flex;
    gap: 0.45rem;
    align-items: baseline;
    color: var(--text-secondary);
    font-size: 0.84rem;
    flex-wrap: wrap;
  }

  .metric-meta strong {
    color: var(--text-primary);
  }

  .badge {
    display: inline-flex;
    align-items: center;
    padding: 0.25rem 0.6rem;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.06);
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: capitalize;
  }

  .focus-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 1rem;
  }

  .focus-card {
    padding: 1.15rem 1.1rem;
    min-height: 196px;
    display: grid;
    align-content: space-between;
    gap: 0.82rem;
  }

  .focus-card-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.8rem;
  }

  .focus-title-row {
    display: flex;
    justify-content: space-between;
    gap: 0.4rem;
    align-items: flex-start;
  }

  .focus-title {
    font-size: 1.03rem;
    font-weight: 750;
    line-height: 1.25;
  }

  .focus-value {
    font-size: 1.6rem;
    font-weight: 800;
    letter-spacing: -0.03em;
  }

  .focus-card p {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.6;
    display: -webkit-box;
    line-clamp: 3;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .focus-link {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    color: var(--text-primary);
    font-weight: 600;
    font-size: 0.88rem;
  }

  .split-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.1fr) minmax(0, 0.9fr);
    gap: 1.25rem;
  }

  .action-list,
  .trend-list {
    display: grid;
    gap: 0.85rem;
  }

  .action-rail {
    gap: 0.7rem;
  }

  .action-row {
    padding: 1rem 1rem;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.85rem;
    position: relative;
    overflow: hidden;
  }

  .action-row::before {
    content: '';
    position: absolute;
    inset: 0 auto 0 0;
    width: 3px;
    background: rgba(148, 163, 184, 0.32);
  }

  .action-row.tone-emerald::before {
    background: rgba(16, 185, 129, 0.9);
  }

  .action-row.tone-amber::before {
    background: rgba(245, 158, 11, 0.9);
  }

  .action-row.tone-cyan::before {
    background: rgba(56, 189, 248, 0.9);
  }

  .action-row.tone-indigo::before {
    background: color-mix(in srgb, var(--color-primary) 88%, white 12%);
  }

  .action-row.tone-rose::before {
    background: rgba(244, 63, 94, 0.9);
  }

  .action-row.tone-slate::before {
    background: rgba(148, 163, 184, 0.76);
  }

  .action-copy {
    display: grid;
    gap: 0.28rem;
  }

  .action-copy strong {
    font-size: 0.92rem;
    line-height: 1.3;
  }

  .action-copy small {
    color: var(--text-secondary);
    line-height: 1.5;
    font-size: 0.8rem;
  }

  .trend-card {
    padding: 1rem 1rem 0.95rem;
  }

  .trend-card-top {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  .trend-title-group {
    display: grid;
    gap: 0.3rem;
  }

  .trend-title {
    font-weight: 750;
    font-size: 0.98rem;
  }

  .trend-caption {
    color: var(--text-secondary);
    font-size: 0.78rem;
  }

  .trend-visual {
    flex: 0 0 auto;
  }

  .trend-ring {
    position: relative;
    width: 72px;
    height: 72px;
  }

  .trend-ring svg {
    width: 72px;
    height: 72px;
    transform: rotate(-90deg);
  }

  .trend-ring-base,
  .trend-ring-value {
    fill: none;
    stroke-width: 7;
  }

  .trend-ring-base {
    stroke: rgba(148, 163, 184, 0.18);
  }

  .trend-ring-value {
    stroke-linecap: round;
  }

  .trend-ring-label {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    text-align: center;
    padding: 0.55rem;
  }

  .trend-ring-label strong {
    display: block;
    font-size: 1rem;
    line-height: 1;
    letter-spacing: -0.02em;
  }

  .trend-ring-label span {
    display: block;
    margin-top: 0.16rem;
    color: var(--text-secondary);
    font-size: 0.58rem;
    line-height: 1.2;
  }

  .trend-items {
    display: grid;
    gap: 0.75rem;
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
    align-items: center;
  }

  .trend-row-values {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
  }

  .trend-row-top strong {
    color: var(--text-primary);
  }

  .trend-share {
    display: inline-flex;
    align-items: center;
    min-height: 20px;
    padding: 0 0.45rem;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--border-color) 84%, transparent);
    color: var(--text-secondary);
    font-size: 0.68rem;
    font-weight: 700;
  }

  .trend-track {
    height: 0.42rem;
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

  .trend-ring-value.tone-emerald {
    stroke: var(--emerald);
    background: transparent;
  }

  .trend-ring-value.tone-amber {
    stroke: var(--amber);
    background: transparent;
  }

  .trend-ring-value.tone-cyan {
    stroke: var(--cyan);
    background: transparent;
  }

  .trend-ring-value.tone-indigo {
    stroke: var(--indigo);
    background: transparent;
  }

  .trend-ring-value.tone-rose {
    stroke: var(--rose);
    background: transparent;
  }

  .trend-ring-value.tone-slate {
    stroke: var(--slate);
    background: transparent;
  }

  @media (max-width: 960px) {
    .admin-dashboard {
      padding: 1rem;
    }

    .executive-hero,
    .split-layout {
      grid-template-columns: 1fr;
      display: grid;
    }

    .hero-actions {
      justify-content: stretch;
    }

    .hero-updated,
    .refresh-button {
      width: 100%;
    }

    .stats-grid,
    .focus-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .trend-card-top {
      align-items: center;
    }

    .section-meta {
      display: none;
    }
  }

  @media (max-width: 640px) {
    .hero-copy h1 {
      max-width: none;
      font-size: 1.45rem;
    }

    .stats-grid,
    .focus-grid {
      grid-template-columns: 1fr;
    }

    .section-block {
      padding: 0.95rem;
    }

    .executive-hero {
      padding: 0 0 0.2rem;
    }

    .trend-card-top {
      grid-template-columns: 1fr;
      display: grid;
      justify-items: start;
    }
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
