<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import { formatMoney } from '$lib/utils/money';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { can } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import type { BillingAnalytics } from '$lib/api/types';

  let analytics = $state<BillingAnalytics | null>(null);
  let loading = $state(true);
  let error = $state('');

  onMount(() => {
    if (!$can('read', 'billing') && !$can('manage', 'billing')) {
      goto('/unauthorized');
      return;
    }
    loadAnalytics();
  });

  async function loadAnalytics() {
    loading = true;
    error = '';
    try {
      analytics = await api.payment.getBillingAnalytics();
    } catch (e: any) {
      error = e.toString();
      toast.error(get(t)('admin.billing.analytics.load_error') || 'Failed to load analytics');
    } finally {
      loading = false;
    }
  }

  function fmt(amount: number) {
    return formatMoney(amount, { compact: true });
  }

  function fmtPct(value: number) {
    return `${value.toFixed(1)}%`;
  }

  function monthLabel(yyyymm: string) {
    const [y, m] = yyyymm.split('-');
    const months = ['Jan', 'Feb', 'Mar', 'Apr', 'Mei', 'Jun', 'Jul', 'Agu', 'Sep', 'Okt', 'Nov', 'Des'];
    return months[parseInt(m, 10) - 1] || yyyymm;
  }

  const agingTotal = $derived(
    analytics
      ? analytics.aging.current + analytics.aging.days_31_60 + analytics.aging.days_61_90 + analytics.aging.over_90
      : 0
  );

  const maxTrendRevenue = $derived(
    analytics?.revenue_trend?.length
      ? Math.max(...analytics.revenue_trend.map((p) => p.revenue), 1)
      : 1
  );
</script>

<div class="billing-dashboard">
  <header class="page-header">
    <div>
      <h1>{$t('admin.billing.analytics.title') || 'Billing Dashboard'}</h1>
      <p class="subtitle">{$t('admin.billing.analytics.subtitle') || 'Revenue metrics and collection overview'}</p>
    </div>
    <button class="btn btn-secondary" onclick={loadAnalytics} disabled={loading}>
      <Icon name="refresh-cw" size={16} />
      <span>{$t('common.refresh') || 'Refresh'}</span>
    </button>
  </header>

  {#if loading}
    <div class="state-card">{$t('common.loading') || 'Loading...'}</div>
  {:else if error}
    <div class="state-card error">{error}</div>
  {:else if analytics}
    <!-- Top KPI cards -->
    <div class="kpi-grid">
      <div class="kpi-card accent">
        <div class="kpi-icon"><Icon name="trending-up" size={20} /></div>
        <div class="kpi-body">
          <span class="kpi-label">MRR</span>
          <span class="kpi-value">{fmt(analytics.mrr)}</span>
          <span class="kpi-sub">{$t('admin.billing.analytics.monthly_recurring') || 'Monthly Recurring Revenue'}</span>
        </div>
      </div>

      <div class="kpi-card">
        <div class="kpi-icon"><Icon name="calendar" size={20} /></div>
        <div class="kpi-body">
          <span class="kpi-label">ARR</span>
          <span class="kpi-value">{fmt(analytics.arr)}</span>
          <span class="kpi-sub">{$t('admin.billing.analytics.annual_recurring') || 'Annual Recurring Revenue'}</span>
        </div>
      </div>

      <div class="kpi-card success">
        <div class="kpi-icon"><Icon name="dollar-sign" size={20} /></div>
        <div class="kpi-body">
          <span class="kpi-label">{$t('admin.billing.analytics.revenue_this_month') || 'Revenue This Month'}</span>
          <span class="kpi-value">{fmt(analytics.total_revenue)}</span>
        </div>
      </div>

      <div class="kpi-card">
        <div class="kpi-icon"><Icon name="users" size={20} /></div>
        <div class="kpi-body">
          <span class="kpi-label">{$t('admin.billing.analytics.active_subs') || 'Active Subscriptions'}</span>
          <span class="kpi-value">{analytics.active_subscriptions}</span>
          <span class="kpi-sub">{analytics.total_customers} {$t('admin.billing.analytics.customers') || 'customers'}</span>
        </div>
      </div>
    </div>

    <!-- Collection & Churn row -->
    <div class="metrics-row">
      <div class="metric-card">
        <h3>{$t('admin.billing.analytics.collection_rate') || 'Collection Rate'}</h3>
        <div class="progress-wrapper">
          <div class="progress-bar">
            <div
              class="progress-fill"
              class:good={analytics.collection_rate >= 90}
              class:warn={analytics.collection_rate >= 70 && analytics.collection_rate < 90}
              class:bad={analytics.collection_rate < 70}
              style="width: {Math.min(analytics.collection_rate, 100)}%"
            ></div>
          </div>
          <span class="progress-label">{fmtPct(analytics.collection_rate)}</span>
        </div>
        <p class="metric-detail">
          {$t('admin.billing.analytics.avg_days_pay') || 'Avg days to pay'}: <strong>{analytics.avg_days_to_pay}</strong> {$t('common.days') || 'days'}
        </p>
      </div>

      <div class="metric-card">
        <h3>{$t('admin.billing.analytics.churn_rate') || 'Churn Rate'}</h3>
        <div class="churn-value" class:good={analytics.churn_rate < 5} class:warn={analytics.churn_rate >= 5}>
          {fmtPct(analytics.churn_rate)}
        </div>
        <p class="metric-detail">
          {$t('admin.billing.analytics.this_month') || 'This month'}
        </p>
      </div>
    </div>

    <!-- Aging Report -->
    <div class="aging-card">
      <h3>{$t('admin.billing.analytics.aging_report') || 'Aging Report (Overdue)'}</h3>
      {#if agingTotal > 0}
        <div class="aging-bars">
          <div class="aging-row">
            <span class="aging-label">0–30 {$t('common.days') || 'days'}</span>
            <div class="aging-bar-track">
              <div class="aging-bar current" style="width: {(analytics.aging.current / agingTotal) * 100}%"></div>
            </div>
            <span class="aging-value">{fmt(analytics.aging.current)}</span>
          </div>
          <div class="aging-row">
            <span class="aging-label">31–60 {$t('common.days') || 'days'}</span>
            <div class="aging-bar-track">
              <div class="aging-bar warn" style="width: {(analytics.aging.days_31_60 / agingTotal) * 100}%"></div>
            </div>
            <span class="aging-value">{fmt(analytics.aging.days_31_60)}</span>
          </div>
          <div class="aging-row">
            <span class="aging-label">61–90 {$t('common.days') || 'days'}</span>
            <div class="aging-bar-track">
              <div class="aging-bar danger" style="width: {(analytics.aging.days_61_90 / agingTotal) * 100}%"></div>
            </div>
            <span class="aging-value">{fmt(analytics.aging.days_61_90)}</span>
          </div>
          <div class="aging-row">
            <span class="aging-label">&gt;90 {$t('common.days') || 'days'}</span>
            <div class="aging-bar-track">
              <div class="aging-bar critical" style="width: {(analytics.aging.over_90 / agingTotal) * 100}%"></div>
            </div>
            <span class="aging-value">{fmt(analytics.aging.over_90)}</span>
          </div>
        </div>
        <p class="aging-total">{$t('admin.billing.analytics.total_overdue') || 'Total Overdue'}: <strong>{fmt(agingTotal)}</strong></p>
      {:else}
        <div class="empty-state">
          <Icon name="check-circle" size={24} />
          <span>{$t('admin.billing.analytics.no_overdue') || 'No overdue invoices 🎉'}</span>
        </div>
      {/if}
    </div>

    <!-- Revenue Trend -->
    {#if analytics.revenue_trend.length > 0}
      <div class="trend-card">
        <h3>{$t('admin.billing.analytics.revenue_trend') || 'Revenue Trend (6 months)'}</h3>
        <div class="trend-chart">
          {#each analytics.revenue_trend as point}
            <div class="trend-bar-wrapper">
              <div class="trend-bar" style="height: {(point.revenue / maxTrendRevenue) * 100}%"></div>
              <span class="trend-month">{monthLabel(point.month)}</span>
              <span class="trend-value">{fmt(point.revenue)}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .billing-dashboard {
    max-width: 1100px;
    margin: 0 auto;
    padding: 1.5rem;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1.5rem;
  }
  .page-header h1 { margin: 0; font-size: 1.5rem; }
  .subtitle { color: var(--text-secondary, #94a3b8); margin: 0.25rem 0 0; font-size: 0.875rem; }

  .state-card {
    padding: 3rem;
    text-align: center;
    color: var(--text-secondary, #94a3b8);
    background: var(--surface, #1e293b);
    border-radius: 12px;
  }
  .state-card.error { color: #ef4444; }

  /* KPI Grid */
  .kpi-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
    margin-bottom: 1.5rem;
  }
  .kpi-card {
    background: var(--surface, #1e293b);
    border: 1px solid var(--border, #334155);
    border-radius: 12px;
    padding: 1.25rem;
    display: flex;
    gap: 1rem;
    align-items: flex-start;
  }
  .kpi-card.accent { border-left: 3px solid #3b82f6; }
  .kpi-card.success { border-left: 3px solid #22c55e; }
  .kpi-icon {
    width: 40px; height: 40px;
    display: grid; place-items: center;
    background: var(--surface-hover, #334155);
    border-radius: 10px;
    color: var(--text-secondary, #94a3b8);
    flex-shrink: 0;
  }
  .kpi-card.accent .kpi-icon { color: #3b82f6; }
  .kpi-card.success .kpi-icon { color: #22c55e; }
  .kpi-body { display: flex; flex-direction: column; gap: 0.15rem; }
  .kpi-label { font-size: 0.75rem; color: var(--text-secondary, #94a3b8); text-transform: uppercase; letter-spacing: 0.04em; }
  .kpi-value { font-size: 1.5rem; font-weight: 700; }
  .kpi-sub { font-size: 0.75rem; color: var(--text-secondary, #94a3b8); }

  /* Metrics row */
  .metrics-row {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }
  .metric-card {
    background: var(--surface, #1e293b);
    border: 1px solid var(--border, #334155);
    border-radius: 12px;
    padding: 1.25rem;
  }
  .metric-card h3 { margin: 0 0 1rem; font-size: 0.95rem; }

  .progress-wrapper { display: flex; align-items: center; gap: 1rem; }
  .progress-bar {
    flex: 1; height: 10px;
    background: var(--surface-hover, #334155);
    border-radius: 99px; overflow: hidden;
  }
  .progress-fill {
    height: 100%; border-radius: 99px;
    transition: width 0.5s ease;
  }
  .progress-fill.good { background: #22c55e; }
  .progress-fill.warn { background: #f59e0b; }
  .progress-fill.bad  { background: #ef4444; }
  .progress-label { font-size: 1.25rem; font-weight: 700; min-width: 60px; text-align: right; }
  .metric-detail { margin: 0.75rem 0 0; font-size: 0.8rem; color: var(--text-secondary, #94a3b8); }

  .churn-value {
    font-size: 2rem; font-weight: 700;
    text-align: center; padding: 0.5rem 0;
  }
  .churn-value.good { color: #22c55e; }
  .churn-value.warn { color: #f59e0b; }

  /* Aging */
  .aging-card {
    background: var(--surface, #1e293b);
    border: 1px solid var(--border, #334155);
    border-radius: 12px;
    padding: 1.25rem;
    margin-bottom: 1.5rem;
  }
  .aging-card h3 { margin: 0 0 1rem; font-size: 0.95rem; }
  .aging-bars { display: flex; flex-direction: column; gap: 0.75rem; }
  .aging-row { display: flex; align-items: center; gap: 0.75rem; }
  .aging-label { width: 80px; font-size: 0.8rem; color: var(--text-secondary, #94a3b8); text-align: right; }
  .aging-bar-track {
    flex: 1; height: 8px;
    background: var(--surface-hover, #334155);
    border-radius: 99px; overflow: hidden;
  }
  .aging-bar {
    height: 100%; border-radius: 99px;
    transition: width 0.5s ease;
  }
  .aging-bar.current  { background: #3b82f6; }
  .aging-bar.warn     { background: #f59e0b; }
  .aging-bar.danger   { background: #f97316; }
  .aging-bar.critical { background: #ef4444; }
  .aging-value { min-width: 100px; font-size: 0.85rem; font-weight: 600; text-align: right; }
  .aging-total { margin: 1rem 0 0; font-size: 0.85rem; color: var(--text-secondary, #94a3b8); text-align: right; }
  .empty-state {
    display: flex; align-items: center; justify-content: center; gap: 0.5rem;
    padding: 2rem; color: #22c55e;
  }

  /* Trend */
  .trend-card {
    background: var(--surface, #1e293b);
    border: 1px solid var(--border, #334155);
    border-radius: 12px;
    padding: 1.25rem;
  }
  .trend-card h3 { margin: 0 0 1.5rem; font-size: 0.95rem; }
  .trend-chart {
    display: flex;
    align-items: flex-end;
    gap: 0.75rem;
    height: 160px;
  }
  .trend-bar-wrapper {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    height: 100%;
    justify-content: flex-end;
    position: relative;
  }
  .trend-bar {
    width: 100%;
    max-width: 60px;
    background: linear-gradient(to top, #3b82f6, #60a5fa);
    border-radius: 6px 6px 0 0;
    transition: height 0.5s ease;
    min-height: 4px;
  }
  .trend-month {
    margin-top: 0.5rem;
    font-size: 0.7rem;
    color: var(--text-secondary, #94a3b8);
  }
  .trend-value {
    position: absolute;
    top: -1.5rem;
    font-size: 0.65rem;
    font-weight: 600;
    color: var(--text-secondary, #94a3b8);
    white-space: nowrap;
  }

  @media (max-width: 640px) {
    .kpi-grid { grid-template-columns: 1fr; }
    .metrics-row { grid-template-columns: 1fr; }
    .trend-chart { height: 120px; }
  }
</style>
