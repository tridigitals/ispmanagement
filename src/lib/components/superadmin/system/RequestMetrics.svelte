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
        <span class="metric-label">Requests/min</span>
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
        <span class="metric-label">Avg Response</span>
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
        <span class="metric-label">P95 Latency</span>
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
        <span class="metric-label">Total Requests</span>
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
        <span class="metric-label">Errors</span>
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
        <span class="metric-label">Rate Limited</span>
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
    background: rgba(59, 130, 246, 0.15);
    color: #3b82f6;
  }

  .metric-icon.response-time {
    background: rgba(16, 185, 129, 0.15);
    color: #10b981;
  }

  .metric-icon.p95 {
    background: rgba(139, 92, 246, 0.15);
    color: #8b5cf6;
  }

  .metric-icon.total {
    background: rgba(99, 102, 241, 0.15);
    color: #6366f1;
  }

  .metric-icon.errors {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
  }

  .metric-icon.rate-limited {
    background: rgba(245, 158, 11, 0.15);
    color: #f59e0b;
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
