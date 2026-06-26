<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import { t } from 'svelte-i18n';

  let { health } = $props<{
    health: any;
  }>();
</script>

{#if health.request_metrics}
  <div class="section-header-inline">
    <Icon name="bar-chart-2" size={18} />
    <h3>
      {$t('superadmin.system.request_metrics.title') || 'Request Metrics'}
    </h3>
  </div>
  <div class="metrics-grid">
    <div class="metric-card">
      <div class="metric-icon requests">
        <Icon name="zap" size={18} />
      </div>
      <div class="metric-content">
        <span class="metric-value">
          {health.request_metrics.requests_last_minute}
        </span>
        <span class="metric-label">{$t('superadmin.system.requests_per_min') || 'Requests/min'}</span>
      </div>
    </div>
    <div class="metric-card">
      <div class="metric-icon response-time">
        <Icon name="clock" size={18} />
      </div>
      <div class="metric-content">
        <span class="metric-value">
          {health.request_metrics.avg_response_time_ms.toFixed(1)}ms
        </span>
        <span class="metric-label">{$t('superadmin.system.avg_response') || 'Avg Response'}</span>
      </div>
    </div>
    <div class="metric-card">
      <div class="metric-icon p95">
        <Icon name="trending-up" size={18} />
      </div>
      <div class="metric-content">
        <span class="metric-value">
          {health.request_metrics.p95_response_time_ms.toFixed(1)}ms
        </span>
        <span class="metric-label">{$t('superadmin.system.p95_latency') || 'P95 Latency'}</span>
      </div>
    </div>
    <div class="metric-card">
      <div class="metric-icon total">
        <Icon name="activity" size={18} />
      </div>
      <div class="metric-content">
        <span class="metric-value">
          {health.request_metrics.total_requests.toLocaleString()}
        </span>
        <span class="metric-label">{$t('superadmin.system.total_requests') || 'Total Requests'}</span>
      </div>
    </div>
    <div class="metric-card">
      <div class="metric-icon errors">
        <Icon name="alert-triangle" size={18} />
      </div>
      <div class="metric-content">
        <span class="metric-value">
          {health.request_metrics.error_count}
        </span>
        <span class="metric-label">{$t('superadmin.system.errors') || 'Errors'}</span>
      </div>
    </div>
    <div class="metric-card">
      <div class="metric-icon rate-limited">
        <Icon name="shield" size={18} />
      </div>
      <div class="metric-content">
        <span class="metric-value">
          {health.request_metrics.rate_limited_count}
        </span>
        <span class="metric-label">{$t('superadmin.system.rate_limited') || 'Rate Limited'}</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .section-header-inline {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
    color: var(--text-secondary);
  }

  .section-header-inline h3 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .metric-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    padding: 1rem;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    transition: border-color 0.2s;
  }

  .metric-card:hover {
    border-color: var(--color-primary);
  }

  .metric-icon {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .metric-icon.requests {
    background: var(--color-primary-subtle);
    color: var(--color-primary);
  }

  .metric-icon.response-time {
    background: var(--bg-success);
    color: var(--color-success);
  }

  .metric-icon.p95 {
    background: var(--color-primary-subtle);
    color: var(--color-primary);
  }

  .metric-icon.total {
    background: var(--color-primary-subtle);
    color: var(--color-primary);
  }

  .metric-icon.errors {
    background: color-mix(in srgb, var(--color-danger) 14%, transparent);
    color: var(--color-danger);
  }

  .metric-icon.rate-limited {
    background: color-mix(in srgb, var(--color-warning) 14%, transparent);
    color: var(--color-warning);
  }

  .metric-content {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .metric-value {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .metric-label {
    font-size: 0.75rem;
    color: var(--text-secondary);
    white-space: nowrap;
  }
</style>
