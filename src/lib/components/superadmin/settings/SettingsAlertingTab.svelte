<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';

  // Props
  export let alertingEnabled: boolean;
  export let alertingEmail: string;
  export let alertingErrorThreshold: number;
  export let alertingRateLimitThreshold: number;
  export let alertingResponseTimeThreshold: number;
  export let alertingCooldownMinutes: number;

  const dispatch = createEventDispatcher();

  function handleChange() {
    dispatch('change');
  }
</script>

<div class="card section fade-in">
  <div class="card-header">
    <h3>{$t('superadmin.settings.alerting.title') || 'Error Alerting'}</h3>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="alerting-enabled">
          {$t('superadmin.settings.alerting.enabled.label') || 'Enable Alerting'}
        </label>
        <p class="setting-description">
          {$t('superadmin.settings.alerting.enabled.desc') ||
            'Send email alerts when error thresholds are exceeded.'}
        </p>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          id="alerting-enabled"
          bind:checked={alertingEnabled}
          on:change={handleChange}
        />
        <span class="slider"></span>
      </label>
    </div>

    {#if alertingEnabled}
      <div class="setting-row">
        <div class="setting-info full-width">
          <label class="setting-label" for="alerting-email">
            {$t('superadmin.settings.alerting.email.label') || 'Alert Email'}
          </label>
          <p class="setting-description">
            {$t('superadmin.settings.alerting.email.desc') || 'Email address to receive alerts.'}
          </p>
          <input
            type="email"
            id="alerting-email"
            bind:value={alertingEmail}
            on:input={handleChange}
            class="form-input"
            placeholder="admin@example.com"
          />
        </div>
      </div>

      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="alerting-error-threshold">
            {$t('superadmin.settings.alerting.error_threshold.label') || 'Error Rate Threshold (%)'}
          </label>
          <p class="setting-description">
            {$t('superadmin.settings.alerting.error_threshold.desc') ||
              'Alert when error rate exceeds this percentage.'}
          </p>
        </div>
        <input
          type="number"
          id="alerting-error-threshold"
          bind:value={alertingErrorThreshold}
          on:input={handleChange}
          class="form-input small-input"
          min="0"
          max="100"
          step="0.1"
        />
      </div>

      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="alerting-rate-limit-threshold">
            {$t('superadmin.settings.alerting.rate_limit_threshold.label') ||
              'Rate Limit Threshold'}
          </label>
          <p class="setting-description">
            {$t('superadmin.settings.alerting.rate_limit_threshold.desc') ||
              'Alert when rate-limited requests exceed this count.'}
          </p>
        </div>
        <input
          type="number"
          id="alerting-rate-limit-threshold"
          bind:value={alertingRateLimitThreshold}
          on:input={handleChange}
          class="form-input small-input"
          min="0"
        />
      </div>

      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="alerting-response-time-threshold">
            {$t('superadmin.settings.alerting.response_time_threshold.label') ||
              'P95 Response Time Threshold (ms)'}
          </label>
          <p class="setting-description">
            {$t('superadmin.settings.alerting.response_time_threshold.desc') ||
              'Alert when P95 response time exceeds this value.'}
          </p>
        </div>
        <input
          type="number"
          id="alerting-response-time-threshold"
          bind:value={alertingResponseTimeThreshold}
          on:input={handleChange}
          class="form-input small-input"
          min="0"
        />
      </div>

      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="alerting-cooldown">
            {$t('superadmin.settings.alerting.cooldown.label') || 'Alert Cooldown (minutes)'}
          </label>
          <p class="setting-description">
            {$t('superadmin.settings.alerting.cooldown.desc') ||
              'Minutes to wait before sending the same alert type again.'}
          </p>
        </div>
        <input
          type="number"
          id="alerting-cooldown"
          bind:value={alertingCooldownMinutes}
          on:input={handleChange}
          class="form-input small-input"
          min="1"
        />
      </div>
    {/if}
  </div>
</div>

<style>
  .card {
    background: var(--bg-surface);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
    margin-bottom: 1.5rem;
  }

  .card-header {
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border-color);
    background: rgba(0, 0, 0, 0.2);
  }

  .card-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .card-body {
    padding: 1.5rem;
  }

  .setting-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 1.25rem 0;
    border-bottom: 1px solid var(--border-color);
  }

  .setting-row:last-child {
    border-bottom: none;
  }

  .setting-info {
    flex: 1;
    padding-right: 1.5rem;
  }

  .setting-info.full-width {
    width: 100%;
    padding-right: 0;
  }

  .setting-label {
    font-weight: 600;
    color: var(--text-primary);
    font-size: 0.95rem;
    display: block;
    margin-bottom: 0.25rem;
  }

  .setting-description {
    color: var(--text-secondary);
    font-size: 0.85rem;
    margin: 0;
    line-height: 1.4;
  }

  .form-input {
    width: 100%;
    max-width: 400px;
    padding: 0.5rem 0.75rem;
    background: var(--bg-app);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .form-input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  .form-input.small-input {
    width: 120px;
    text-align: right;
  }

  /* Toggle Switch */
  .toggle {
    position: relative;
    display: inline-block;
    width: 52px;
    height: 28px;
    flex-shrink: 0;
  }

  .toggle input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--bg-tertiary);
    transition: 0.3s;
    border-radius: 28px;
  }

  .slider:before {
    position: absolute;
    content: '';
    height: 20px;
    width: 20px;
    left: 4px;
    bottom: 4px;
    background-color: white;
    transition: 0.3s;
    border-radius: 50%;
  }

  input:checked + .slider {
    background-color: var(--color-primary);
  }

  input:checked + .slider:before {
    transform: translateX(24px);
  }

  .fade-in {
    animation: fadeIn 0.3s ease-out;
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
</style>
