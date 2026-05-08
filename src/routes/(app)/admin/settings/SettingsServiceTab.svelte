<script lang="ts">
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import Input from '$lib/components/ui/Input.svelte';

  export let localSettings: Record<string, string>;
  export let formattedLastRunAt = '-';
  export let handleChange: (key: string, value: any) => void;

  function tt(key: string, fallback: string) {
    const value = get(t)(key);
    return value && value !== key ? value : fallback;
  }

  function suspendMode() {
    return localSettings['billing_auto_suspend_mode'] || 'grace_period';
  }

  function clampInt(value: string | number, min: number, max: number, fallback: number) {
    const parsed = Number.parseInt(String(value ?? ''), 10);
    if (!Number.isFinite(parsed)) return fallback;
    return Math.max(min, Math.min(max, parsed));
  }
</script>

<div class="service-settings">
  <div class="intro-card">
    <span class="section-label">{tt('admin.settings.service.lifecycle_label', 'Service Lifecycle')}</span>
    <h3>{tt('admin.settings.categories.service', 'Service')}</h3>
    <p>
      {tt(
        'admin.settings.service.global_policy_help',
        'Policy ini berlaku global untuk semua customer dan layanan.',
      )}
    </p>
  </div>

  <section class="service-section">
    <div class="section-head">
      <div>
        <h4>{tt('admin.settings.service.auto_invoice_title', 'Auto Invoice')}</h4>
        <p>{tt('admin.settings.service.auto_invoice_help', 'Atur pembuatan invoice layanan pelanggan secara otomatis.')}</p>
      </div>
    </div>

    <div class="setting-item full-width">
      <label class="checkbox-label" for="customer-invoice-auto-generate-enabled">
        <input
          id="customer-invoice-auto-generate-enabled"
          type="checkbox"
          checked={localSettings['customer_invoice_auto_generate_enabled'] !== 'false'}
          onchange={(e: any) =>
            handleChange('customer_invoice_auto_generate_enabled', e.currentTarget.checked)}
        />
        <span>
          {tt(
            'admin.settings.payment.customer_invoice_auto_generate_enabled_label',
            'Enable automatic customer invoice generation',
          )}
        </span>
      </label>
      <p class="help-text">
        {tt(
          'admin.settings.payment.customer_invoice_auto_generate_enabled_help',
          'Runs in background and creates due invoices automatically based on lead days.',
        )}
      </p>
    </div>

    <div class="config-grid">
      <div class="setting-item">
        <label for="customer-invoice-days-before-due">
          {tt(
            'admin.settings.payment.invoice_generation_days_before_due_label',
            'Generate customer invoice (days before due)',
          )}
        </label>
        <Input
          id="customer-invoice-days-before-due"
          type="number"
          min="0"
          max="60"
          value={localSettings['customer_invoice_generate_days_before_due'] || '7'}
          oninput={(e: any) =>
            handleChange(
              'customer_invoice_generate_days_before_due',
              clampInt(e.target.value, 0, 60, 7),
            )}
          placeholder="7"
        />
      </div>

      <div class="setting-item">
        <label for="customer-invoice-scheduler-interval-minutes">
          {tt(
            'admin.settings.payment.customer_invoice_scheduler_interval_minutes_label',
            'Customer invoice scheduler interval (minutes)',
          )}
        </label>
        <Input
          id="customer-invoice-scheduler-interval-minutes"
          type="number"
          min="5"
          max="1440"
          value={localSettings['customer_invoice_scheduler_interval_minutes'] || '60'}
          oninput={(e: any) =>
            handleChange(
              'customer_invoice_scheduler_interval_minutes',
              clampInt(e.target.value, 5, 1440, 60),
            )}
          placeholder="60"
        />
      </div>
    </div>

    <div class="setting-item full-width">
      <span class="inline-label">
        {tt(
          'admin.settings.payment.customer_invoice_last_run_at_label',
          'Last customer invoice generation run',
        )}
      </span>
      <div class="readonly-value">{formattedLastRunAt}</div>
    </div>
  </section>

  <section class="service-section">
    <div class="section-head">
      <div>
        <h4>{tt('admin.settings.service.auto_suspend_title', 'Auto Suspend')}</h4>
        <p>{tt('admin.settings.service.auto_suspend_help', 'Suspend layanan pelanggan secara otomatis sesuai policy global.')}</p>
      </div>
    </div>

    <div class="setting-item full-width">
      <label class="checkbox-label" for="billing-auto-suspend-enabled">
        <input
          id="billing-auto-suspend-enabled"
          type="checkbox"
          checked={localSettings['billing_auto_suspend_enabled'] === 'true'}
          onchange={(e: any) => handleChange('billing_auto_suspend_enabled', e.currentTarget.checked)}
        />
        <span>{tt('admin.settings.service.auto_suspend_enabled_label', 'Enable automatic suspend')}</span>
      </label>
    </div>

    <div class="setting-item full-width">
      <label for="billing-auto-suspend-mode">
        {tt('admin.settings.service.auto_suspend_mode_label', 'Suspend method')}
      </label>
      <select
        id="billing-auto-suspend-mode"
        class="input"
        value={suspendMode()}
        onchange={(e: any) => handleChange('billing_auto_suspend_mode', e.currentTarget.value)}
      >
        <option value="grace_period">
          {tt('admin.settings.service.auto_suspend_mode_grace', 'Grace period after due date')}
        </option>
        <option value="fixed_day">
          {tt('admin.settings.service.auto_suspend_mode_fixed_day', 'Fixed day every month')}
        </option>
      </select>
      <p class="help-text">
        {tt(
          'admin.settings.service.auto_suspend_mode_help',
          'Use grace period for offset-based suspend, or fixed day for a single global suspend date each month.',
        )}
      </p>
    </div>

    {#if suspendMode() === 'grace_period'}
      <div class="setting-item full-width">
        <label for="billing-auto-suspend-grace-days">
          {tt('admin.settings.service.auto_suspend_grace_days_label', 'Grace days')}
        </label>
        <Input
          id="billing-auto-suspend-grace-days"
          type="number"
          min="0"
          max="365"
          value={localSettings['billing_auto_suspend_grace_days'] || '3'}
          oninput={(e: any) =>
            handleChange('billing_auto_suspend_grace_days', clampInt(e.target.value, 0, 365, 3))}
          placeholder="3"
        />
      </div>
    {:else}
      <div class="setting-item full-width">
        <label for="billing-auto-suspend-fixed-day">
          {tt('admin.settings.service.auto_suspend_fixed_day_label', 'Fixed suspend day (1-28)')}
        </label>
        <Input
          id="billing-auto-suspend-fixed-day"
          type="number"
          min="1"
          max="28"
          value={localSettings['billing_auto_suspend_fixed_day'] || '1'}
          oninput={(e: any) =>
            handleChange('billing_auto_suspend_fixed_day', clampInt(e.target.value, 1, 28, 1))}
          placeholder="1"
        />
        <p class="help-text">
          {tt(
            'admin.settings.service.auto_suspend_fixed_day_help',
            'Limited to 1-28 so the same day is valid for every month.',
          )}
        </p>
      </div>
    {/if}
  </section>

  <section class="service-section">
    <div class="section-head">
      <div>
        <h4>{tt('admin.settings.service.auto_resume_title', 'Auto Resume')}</h4>
        <p>{tt('admin.settings.service.auto_resume_help', 'Aktifkan kembali layanan yang disuspend billing setelah pembayaran diterima.')}</p>
      </div>
    </div>
    <div class="setting-item full-width">
      <label class="checkbox-label" for="billing-auto-resume-on-payment">
        <input
          id="billing-auto-resume-on-payment"
          type="checkbox"
          checked={localSettings['billing_auto_resume_on_payment'] !== 'false'}
          onchange={(e: any) =>
            handleChange('billing_auto_resume_on_payment', e.currentTarget.checked)}
        />
        <span>{tt('admin.settings.service.auto_resume_on_payment_label', 'Resume suspended services after payment')}</span>
      </label>
    </div>
  </section>

  <section class="service-section">
    <div class="section-head">
      <div>
        <h4>{tt('admin.settings.service.reminder_title', 'Reminder')}</h4>
        <p>{tt('admin.settings.service.reminder_help', 'Atur reminder invoice layanan pelanggan sebelum dan sesudah jatuh tempo.')}</p>
      </div>
    </div>
    <div class="setting-item full-width">
      <label class="checkbox-label" for="billing-reminder-enabled">
        <input
          id="billing-reminder-enabled"
          type="checkbox"
          checked={localSettings['billing_reminder_enabled'] !== 'false'}
          onchange={(e: any) => handleChange('billing_reminder_enabled', e.currentTarget.checked)}
        />
        <span>{tt('admin.settings.service.reminder_enabled_label', 'Enable billing reminders')}</span>
      </label>
    </div>
    <div class="setting-item full-width">
      <label for="billing-reminder-schedule">
        {tt('admin.settings.service.reminder_schedule_label', 'Reminder schedule')}
      </label>
      <Input
        id="billing-reminder-schedule"
        value={localSettings['billing_reminder_schedule'] || 'H-3,H-1,H+1,H+3'}
        oninput={(e: any) => handleChange('billing_reminder_schedule', e.target.value)}
        placeholder="H-3,H-1,H+1,H+3"
      />
      <p class="help-text">
        {tt(
          'admin.settings.service.reminder_schedule_help',
          'Comma-separated offsets around due date, for example H-3,H-1,H+1.',
        )}
      </p>
    </div>
  </section>
</div>

<style>
  .service-settings {
    display: grid;
    gap: 1rem;
  }

  .intro-card,
  .service-section {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 1rem;
    background: var(--bg-surface);
  }

  .section-label {
    display: inline-block;
    margin-bottom: 0.35rem;
    color: var(--text-secondary);
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .section-head {
    margin-bottom: 0.85rem;
  }

  .section-head h4,
  .intro-card h3 {
    margin: 0;
  }

  .section-head p,
  .intro-card p,
  .help-text {
    margin: 0.3rem 0 0;
    color: var(--text-secondary);
  }

  .config-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 0.85rem;
  }

  .setting-item {
    display: grid;
    gap: 0.4rem;
  }

  .full-width {
    width: 100%;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .inline-label {
    font-weight: 600;
  }

  .readonly-value {
    color: var(--text-primary);
    font-weight: 600;
  }
</style>
