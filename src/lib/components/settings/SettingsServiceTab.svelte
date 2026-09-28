<script lang="ts">
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import Input from '$lib/components/ui/Input.svelte';
  import {
    REMINDER_PRESETS,
    REMINDER_PRESET_DETAILS,
    addReminderCode,
    buildReminderCode,
    formatReminderCodeLabel,
    groupReminderCodes,
    parseReminderSchedule,
    removeReminderCode,
    stringifyReminderSchedule,
    type ReminderTiming,
  } from '$lib/utils/reminderSchedule';

  let {
    localSettings,
    formattedLastRunAt = '-',
    billingLogsPath = '',
    invoicesPath = '',
    handleChange,
  }: {
    localSettings: Record<string, string>;
    formattedLastRunAt?: string;
    billingLogsPath?: string;
    invoicesPath?: string;
    handleChange: (key: string, value: any) => void;
  } = $props();
  let selectedSuspendMode = $state<'grace_period' | 'fixed_day'>('grace_period');
  let selectedSuspendPppoeAction = $state<'disable_secret' | 'move_to_isolation_pool'>(
    'disable_secret',
  );
  let reminderDraftDays = $state(3);
  let reminderDraftTiming = $state<ReminderTiming>('before');

  function tt(key: string, fallback: string) {
    const value = get(t)(key);
    return value && value !== key ? value : fallback;
  }

  function suspendMode() {
    return selectedSuspendMode;
  }

  function suspendPppoeAction() {
    return selectedSuspendPppoeAction;
  }

  function clampInt(value: string | number, min: number, max: number, fallback: number) {
    const parsed = Number.parseInt(String(value ?? ''), 10);
    if (!Number.isFinite(parsed)) return fallback;
    return Math.max(min, Math.min(max, parsed));
  }

  function selectSuspendMode(mode: 'grace_period' | 'fixed_day') {
    selectedSuspendMode = mode;
    handleChange('billing_auto_suspend_mode', mode);
  }

  function selectSuspendPppoeAction(action: 'disable_secret' | 'move_to_isolation_pool') {
    selectedSuspendPppoeAction = action;
    handleChange('billing_auto_suspend_pppoe_action', action);
  }

  function activeSuspendPreview() {
    if (suspendMode() === 'fixed_day') {
      const day = clampInt(localSettings['billing_auto_suspend_fixed_day'] || '1', 1, 28, 1);
      return tt('admin.settings.service_tab.suspend_fixed_day_preview', 'Tanggal tetap {day} setiap bulan').replace('{day}', String(day));
    }

    const days = clampInt(localSettings['billing_auto_suspend_grace_days'] || '3', 0, 365, 3);
    return tt('admin.settings.service_tab.suspend_grace_preview', 'Tenggang {days} hari setelah jatuh tempo').replace('{days}', String(days));
  }

  function activeSuspendPppoePreview() {
    if (suspendPppoeAction() === 'move_to_isolation_pool')
      return tt('admin.settings.service_tab.pppoe_isolation_preview', 'Mengikuti pool isolir di service/router');

    return tt('admin.settings.service_tab.pppoe_disable_preview', 'Secret PPPoE dinonaktifkan');
  }

  function invoicePreview() {
    const enabled = localSettings['customer_invoice_auto_generate_enabled'] !== 'false';
    const leadDays = clampInt(
      localSettings['customer_invoice_generate_days_before_due'] || '7',
      0,
      60,
      7,
    );
    return enabled ? tt('admin.settings.service_tab.invoice_active_preview', 'Aktif, dibuat H-{days}').replace('{days}', String(leadDays)) : tt('admin.settings.service_tab.disabled', 'Nonaktif');
  }

  function autoResumePreview() {
    return localSettings['billing_auto_resume_on_payment'] !== 'false'
      ? tt('admin.settings.service_tab.auto_resume_on_payment', 'Otomatis aktif setelah pembayaran')
      : tt('admin.settings.service_tab.auto_resume_manual', 'Aktifkan manual');
  }

  function reminderPreview() {
    const enabled = localSettings['billing_reminder_enabled'] !== 'false';
    const codes = parseReminderSchedule(localSettings['billing_reminder_schedule']);
    const grouped = groupReminderCodes(codes);
    if (!enabled) return tt('admin.settings.service_tab.disabled', 'Nonaktif');
    return tt('admin.settings.service_tab.reminder_preview', '{before} sebelum, {after} sesudah jatuh tempo').replace('{before}', String(grouped.before.length)).replace('{after}', String(grouped.after.length));
  }

  function schedulerStatusLabel() {
    if (!formattedLastRunAt || formattedLastRunAt === '-') return tt('admin.settings.service_tab.no_runs_recorded', 'Belum ada run tercatat');
    return formattedLastRunAt;
  }

  function schedulerStatusHelp() {
    if (!formattedLastRunAt || formattedLastRunAt === '-') {
      return tt('admin.settings.service_tab.scheduler_help_no_runs', 'Jalankan generate invoice manual sekali untuk memastikan scheduler sudah aktif.');
    }
    return tt('admin.settings.service_tab.scheduler_help_updated', 'Terakhir diperbarui saat generate invoice pelanggan berjalan.');
  }

  function reminderCodes() {
    const parsed = parseReminderSchedule(localSettings['billing_reminder_schedule']);
    return parsed.length > 0 ? parsed : [...REMINDER_PRESETS.standard];
  }

  function updateReminderCodes(codes: string[]) {
    handleChange('billing_reminder_schedule', stringifyReminderSchedule(codes));
  }

  function reminderGroups() {
    return groupReminderCodes(reminderCodes());
  }

  function reminderCountLabel(codes: string[]) {
    if (codes.length === 0) return tt('admin.settings.service_tab.reminder_none', 'Belum ada');
    if (codes.length === 1) return tt('admin.settings.service_tab.reminder_count_one', '1 pengingat');
    return tt('admin.settings.service_tab.reminder_count', '{count} pengingat').replace('{count}', String(codes.length));
  }

  function applyReminderPreset(preset: keyof typeof REMINDER_PRESETS) {
    updateReminderCodes([...REMINDER_PRESETS[preset]]);
  }

  function addDraftReminder() {
    updateReminderCodes(
      addReminderCode(reminderCodes(), buildReminderCode(reminderDraftTiming, reminderDraftDays)),
    );
  }

  function removeReminder(code: string) {
    updateReminderCodes(removeReminderCode(reminderCodes(), code));
  }

  function isReminderPresetActive(preset: keyof typeof REMINDER_PRESETS) {
    return (
      stringifyReminderSchedule(reminderCodes()) ===
      stringifyReminderSchedule([...REMINDER_PRESETS[preset]])
    );
  }

  $effect(() => {
    selectedSuspendMode =
      localSettings['billing_auto_suspend_mode'] === 'fixed_day' ? 'fixed_day' : 'grace_period';
    selectedSuspendPppoeAction =
      localSettings['billing_auto_suspend_pppoe_action'] === 'move_to_isolation_pool'
        ? 'move_to_isolation_pool'
        : 'disable_secret';
  });
</script>

<div class="service-settings">
  <div class="intro-card">
    <h3>{tt('admin.settings.categories.service', 'Service')}</h3>
    <p>{tt('admin.settings.service_tab.subtitle', 'Pengaturan global untuk billing dan lifecycle layanan.')}</p>

    <div class="summary-grid">
      <div class="summary-card">
        <div>
          <span class="summary-label">{tt('admin.settings.service_tab.health_scheduler', 'Health scheduler')}</span>
          <strong>{schedulerStatusLabel()}</strong>
          <small>{schedulerStatusHelp()}</small>
        </div>
      </div>
      <div class="summary-card">
        <div>
          <span class="summary-label">{tt('admin.settings.service_tab.auto_invoice', 'Invoice otomatis')}</span>
          <strong>{invoicePreview()}</strong>
        </div>
      </div>
      <div class="summary-card">
        <div>
          <span class="summary-label">{tt('admin.settings.service_tab.auto_suspend', 'Suspend otomatis')}</span>
          <strong
            >{localSettings['billing_auto_suspend_enabled'] === 'true'
              ? activeSuspendPreview()
              : (tt('admin.settings.service_tab.disabled', 'Nonaktif'))}</strong>
          >
        </div>
      </div>
      <div class="summary-card">
        <div>
          <span class="summary-label">{tt('admin.settings.service_tab.auto_resume', 'Auto resume')}</span>
          <strong>{autoResumePreview()}</strong>
        </div>
      </div>
      <div class="summary-card">
        <div>
          <span class="summary-label">{tt('admin.settings.service_tab.reminder', 'Reminder')}</span>
          <strong>{reminderPreview()}</strong>
        </div>
      </div>
    </div>

    <div class="quick-actions">
      <a class="quick-link" href={invoicesPath || '/admin/invoices'}>
        <span>
          <strong>{tt('admin.settings.service_tab.view_invoices', 'Lihat invoice')}</strong>
          <small>{tt('admin.settings.service_tab.invoice_status', 'Status tagihan pelanggan')}</small>
        </span>
      </a>
      <a class="quick-link" href={billingLogsPath || '/admin/invoices/collection'}>
        <span>
          <strong>{tt('admin.settings.service_tab.view_automation_log', 'Lihat log automation')}</strong>
          <small>{tt('admin.settings.service_tab.scheduler_history', 'Riwayat scheduler billing')}</small>
        </span>
      </a>
    </div>
  </div>

  <section class="service-section">
    <div class="section-head">
      <div>
        <h4>{tt('admin.settings.service.auto_invoice_title', 'Invoice Otomatis')}</h4>
      </div>
    </div>

    <div class="service-panel">
      <div class="setting-item full-width toggle-row control-strip">
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
              'Aktifkan pembuatan invoice pelanggan otomatis',
            )}
          </span>
        </label>
      </div>

      <div class="policy-preview">
        <span class="policy-preview-label">{tt('common.status', 'Status')}</span>
        <strong>{invoicePreview()}</strong>
      </div>

      <div class="config-grid">
        <div class="setting-item">
          <label for="customer-invoice-days-before-due">
            {tt(
              'admin.settings.payment.invoice_generation_days_before_due_label',
              'Buat invoice pelanggan (hari sebelum jatuh tempo)',
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
              'Interval pengecekan scheduler invoice (menit)',
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
            'Waktu terakhir generate invoice pelanggan',
          )}
        </span>
        <div class="readonly-value">{formattedLastRunAt}</div>
      </div>
    </div>
  </section>

  <section class="service-section">
    <div class="section-head">
      <div>
        <h4>{tt('admin.settings.service.auto_suspend_title', 'Suspend Otomatis')}</h4>
      </div>
    </div>

    <div class="setting-item full-width toggle-row control-strip">
      <label class="checkbox-label" for="billing-auto-suspend-enabled">
        <input
          id="billing-auto-suspend-enabled"
          type="checkbox"
          checked={localSettings['billing_auto_suspend_enabled'] === 'true'}
          onchange={(e: any) =>
            handleChange('billing_auto_suspend_enabled', e.currentTarget.checked)}
        />
        <span>
          {tt('admin.settings.service.auto_suspend_enabled_label', 'Aktifkan suspend otomatis')}
        </span>
      </label>
    </div>

    <div class="suspend-panel">
      <div class="setting-item full-width">
        <span class="field-title">
          {tt('admin.settings.service.auto_suspend_mode_label', 'Metode suspend')}
        </span>
        <div class="mode-picker" role="radiogroup" aria-label={$t('admin.settings.service_tab.suspend_method')}>
          <button
            type="button"
            class:selected={suspendMode() === 'grace_period'}
            class="mode-card"
            onclick={() => selectSuspendMode('grace_period')}
          >
            <strong>
              {tt('admin.settings.service.auto_suspend_mode_grace_short', 'Masa Tenggang')}
            </strong>
            <span>
              {tt(
                'admin.settings.service.auto_suspend_mode_grace',
                'Suspend beberapa hari setelah jatuh tempo',
              )}
            </span>
          </button>
          <button
            type="button"
            class:selected={suspendMode() === 'fixed_day'}
            class="mode-card"
            onclick={() => selectSuspendMode('fixed_day')}
          >
            <strong
              >{tt('admin.settings.service.auto_suspend_mode_fixed_short', 'Tanggal Tetap')}</strong
            >
            <span>
              {tt(
                'admin.settings.service.auto_suspend_mode_fixed_day',
                'Suspend di tanggal yang sama setiap bulan',
              )}
            </span>
          </button>
        </div>
      </div>

      <div class="policy-preview">
        <span class="policy-preview-label">{tt('admin.settings.service_tab.active_policy', 'Kebijakan aktif')}</span>
        <strong>{activeSuspendPreview()}</strong>
      </div>

      <div class="mode-detail-card">
        {#if suspendMode() === 'grace_period'}
          <div class="setting-item full-width">
            <label for="billing-auto-suspend-grace-days">
              {tt('admin.settings.service.auto_suspend_grace_days_label', 'Hari tenggang')}
            </label>
            <Input
              id="billing-auto-suspend-grace-days"
              type="number"
              min="0"
              max="365"
              value={localSettings['billing_auto_suspend_grace_days'] || '3'}
              oninput={(e: any) =>
                handleChange(
                  'billing_auto_suspend_grace_days',
                  clampInt(e.target.value, 0, 365, 3),
                )}
              placeholder="3"
            />
          </div>
        {:else}
          <div class="setting-item full-width">
            <label for="billing-auto-suspend-fixed-day">
              {tt(
                'admin.settings.service.auto_suspend_fixed_day_label',
                'Tanggal suspend tetap (1-28)',
              )}
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
            <p class="help-text">{tt('admin.settings.service_tab.suspend_fixed_day_help', 'Gunakan 1-28 agar selalu valid di semua bulan.')}</p>
          </div>
        {/if}
      </div>

      <div class="mode-detail-card">
        <div class="setting-item full-width">
          <span class="field-title">{tt('admin.settings.service_tab.pppoe_action_title', 'Aksi PPPoE saat suspend')}</span>

          <div class="mode-picker" role="radiogroup" aria-label={tt('admin.settings.service_tab.pppoe_action_title', 'Aksi PPPoE saat suspend')}>
            <button
              type="button"
              class:selected={suspendPppoeAction() === 'disable_secret'}
              class="mode-card"
              onclick={() => selectSuspendPppoeAction('disable_secret')}
            >
              <strong>{tt('admin.settings.service_tab.pppoe_disable_title', 'Disable PPP')}</strong>
              <span>{tt('admin.settings.service_tab.pppoe_disable_desc', 'Secret PPPoE dinonaktifkan dan sesi aktif diputus.')}</span>
            </button>
            <button
              type="button"
              class:selected={suspendPppoeAction() === 'move_to_isolation_pool'}
              class="mode-card"
              onclick={() => selectSuspendPppoeAction('move_to_isolation_pool')}
            >
              <strong>{tt('admin.settings.service_tab.pppoe_isolation_title', 'Pool Isolir')}</strong>
              <span>{tt('admin.settings.service_tab.pppoe_isolation_desc', 'PPPoE tetap aktif, tapi dipindah ke pool isolir lalu reconnect otomatis.')}</span>
            </button>
          </div>
        </div>

        <div class="policy-preview">
          <span class="policy-preview-label">{tt('admin.settings.service_tab.active_action', 'Aksi aktif')}</span>
          <strong>{activeSuspendPppoePreview()}</strong>
        </div>

      <div class="setting-item full-width">
          <p class="help-text">{tt('admin.settings.service_tab.isolation_pool_help', 'Pool isolir diatur di mapping service/router.')}</p>
        </div>
      </div>
    </div>
  </section>

  <section class="service-section">
    <div class="section-head">
      <div>
        <h4>{tt('admin.settings.service.auto_resume_title', 'Aktifkan Kembali Otomatis')}</h4>
      </div>
    </div>
    <div class="service-panel compact-panel">
      <div class="setting-item full-width control-strip">
        <label class="checkbox-label" for="billing-auto-resume-on-payment">
          <input
            id="billing-auto-resume-on-payment"
            type="checkbox"
            checked={localSettings['billing_auto_resume_on_payment'] !== 'false'}
            onchange={(e: any) =>
              handleChange('billing_auto_resume_on_payment', e.currentTarget.checked)}
          />
          <span>
            {tt(
              'admin.settings.service.auto_resume_on_payment_label',
              'Aktifkan kembali layanan yang disuspend setelah pembayaran',
            )}
          </span>
        </label>
      </div>

      <div class="policy-preview">
        <span class="policy-preview-label">{tt('common.status', 'Status')}</span>
        <strong>{autoResumePreview()}</strong>
      </div>
    </div>
  </section>

  <section class="service-section">
    <div class="section-head">
      <div>
        <h4>{tt('admin.settings.service.reminder_title', 'Pengingat')}</h4>
      </div>
    </div>
    <div class="service-panel">
      <div class="setting-item full-width control-strip">
        <label class="checkbox-label" for="billing-reminder-enabled">
          <input
            id="billing-reminder-enabled"
            type="checkbox"
            checked={localSettings['billing_reminder_enabled'] !== 'false'}
            onchange={(e: any) => handleChange('billing_reminder_enabled', e.currentTarget.checked)}
          />
          <span>
            {tt('admin.settings.service.reminder_enabled_label', 'Aktifkan pengingat billing')}
          </span>
        </label>
      </div>

      <div class="policy-preview">
        <span class="policy-preview-label">{tt('admin.settings.service_tab.summary', 'Ringkasan')}</span>
        <strong>{reminderPreview()}</strong>
      </div>

      <div class="setting-item full-width">
        <span class="field-title">
          {tt('admin.settings.service.reminder_schedule_label', 'Jadwal pengingat')}
        </span>
        <div class="preset-grid">
          <button
            type="button"
            class="preset-card"
            class:active={isReminderPresetActive('light')}
            onclick={() => applyReminderPreset('light')}
          >
            <div class="preset-card-head">
              <strong>{REMINDER_PRESET_DETAILS.light.label}</strong>
              <span class="preset-card-meta">{tt('admin.settings.service_tab.preset_reminder_count', '{count} pengingat').replace('{count}', String(REMINDER_PRESETS.light.length))}</span>
            </div>
            <p>{REMINDER_PRESET_DETAILS.light.description}</p>
          </button>
          <button
            type="button"
            class="preset-card"
            class:active={isReminderPresetActive('standard')}
            onclick={() => applyReminderPreset('standard')}
          >
            <div class="preset-card-head">
              <strong>{REMINDER_PRESET_DETAILS.standard.label}</strong>
              <span class="preset-card-meta">{tt('admin.settings.service_tab.preset_reminder_count', '{count} pengingat').replace('{count}', String(REMINDER_PRESETS.standard.length))}</span>
            </div>
            <p>{REMINDER_PRESET_DETAILS.standard.description}</p>
          </button>
          <button
            type="button"
            class="preset-card"
            class:active={isReminderPresetActive('aggressive')}
            onclick={() => applyReminderPreset('aggressive')}
          >
            <div class="preset-card-head">
              <strong>{REMINDER_PRESET_DETAILS.aggressive.label}</strong>
              <span class="preset-card-meta">{tt('admin.settings.service_tab.preset_reminder_count', '{count} pengingat').replace('{count}', String(REMINDER_PRESETS.aggressive.length))}</span>
            </div>
            <p>{REMINDER_PRESET_DETAILS.aggressive.description}</p>
          </button>
        </div>

        <div class="reminder-groups">
          <div class="reminder-group">
            <div class="reminder-group-head">
              <span class="policy-preview-label">{tt('admin.settings.service_tab.before_due', 'Sebelum jatuh tempo')}</span>
              <strong>{reminderCountLabel(reminderGroups().before)}</strong>
            </div>
            <div class="chip-list">
              {#if reminderGroups().before.length > 0}
                {#each reminderGroups().before as code}
                  <button type="button" class="schedule-chip" onclick={() => removeReminder(code)}>
                    <span>{formatReminderCodeLabel(code)}</span>
                    <small>{tt('admin.settings.service_tab.delete_label', 'hapus')}</small>
                  </button>
                {/each}
              {:else}
                <span class="empty-chip">{tt('admin.settings.service_tab.reminder_none', 'Belum ada')}</span>
              {/if}
            </div>
          </div>

          <div class="reminder-group">
            <div class="reminder-group-head">
              <span class="policy-preview-label">{tt('admin.settings.service_tab.after_due', 'Sesudah jatuh tempo')}</span>
              <strong>{reminderCountLabel(reminderGroups().after)}</strong>
            </div>
            <div class="chip-list">
              {#if reminderGroups().after.length > 0}
                {#each reminderGroups().after as code}
                  <button type="button" class="schedule-chip" onclick={() => removeReminder(code)}>
                    <span>{formatReminderCodeLabel(code)}</span>
                    <small>{tt('admin.settings.service_tab.delete_label', 'hapus')}</small>
                  </button>
                {/each}
              {:else}
                <span class="empty-chip">{tt('admin.settings.service_tab.reminder_none', 'Belum ada')}</span>
              {/if}
            </div>
          </div>
        </div>

        <div class="reminder-builder">
          <div class="builder-head">
            <span class="policy-preview-label">{tt('admin.settings.service_tab.add_rule_manual', 'Tambah aturan manual')}</span>
          </div>
          <div class="builder-controls">
            <div class="timing-toggle" role="radiogroup" aria-label={tt('admin.settings.service_tab.reminder_timing_label', 'Waktu reminder')}>
              <button
                type="button"
                class:active={reminderDraftTiming === 'before'}
                class="timing-chip"
                onclick={() => (reminderDraftTiming = 'before')}
              >
                {tt('admin.settings.service_tab.before_due', 'Sebelum jatuh tempo')}
              </button>
              <button
                type="button"
                class:active={reminderDraftTiming === 'after'}
                class="timing-chip"
                onclick={() => (reminderDraftTiming = 'after')}
              >
                {tt('admin.settings.service_tab.after_due', 'Sesudah jatuh tempo')}
              </button>
            </div>
            <div class="days-input-wrap">
              <Input
                type="number"
                min="1"
                max="30"
                value={String(reminderDraftDays)}
                oninput={(e: any) =>
                  (reminderDraftDays = Math.max(1, Math.min(30, Number(e.target.value || 1))))}
                placeholder="3"
              />
              <span class="days-suffix">{tt('admin.settings.service_tab.days_suffix', 'hari')}</span>
            </div>
            <button type="button" class="btn-add-chip" onclick={addDraftReminder}>
              {tt('admin.settings.service_tab.add_reminder', 'Tambah pengingat')}
            </button>
          </div>
          <div class="quick-add-row">
            <button
              type="button"
              class="quick-add-chip"
              onclick={() => updateReminderCodes(addReminderCode(reminderCodes(), 'H-1'))}
            >
              {tt('admin.settings.service_tab.quick_add_1_before', '+ 1 hari sebelum')}
            </button>
            <button
              type="button"
              class="quick-add-chip"
              onclick={() => updateReminderCodes(addReminderCode(reminderCodes(), 'H-3'))}
            >
              {tt('admin.settings.service_tab.quick_add_3_before', '+ 3 hari sebelum')}
            </button>
            <button
              type="button"
              class="quick-add-chip"
              onclick={() => updateReminderCodes(addReminderCode(reminderCodes(), 'H+1'))}
            >
              {tt('admin.settings.service_tab.quick_add_1_after', '+ 1 hari sesudah')}
            </button>
            <button
              type="button"
              class="quick-add-chip"
              onclick={() => updateReminderCodes(addReminderCode(reminderCodes(), 'H+3'))}
            >
              {tt('admin.settings.service_tab.quick_add_3_after', '+ 3 hari sesudah')}
            </button>
          </div>
        </div>

        <details class="raw-schedule">
          <summary>{tt('admin.settings.service_tab.technical_format_saved', 'Format teknis tersimpan')}</summary>
          <code>{stringifyReminderSchedule(reminderCodes())}</code>
        </details>

        <p class="help-text">{tt('admin.settings.service_tab.technical_format_help', 'Format teknis tetap disimpan sebagai kode H-.')}</p>
      </div>
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
    border-radius: 14px;
    padding: 1rem;
    background: var(--bg-surface);
  }

  .section-head {
    margin-bottom: 0.85rem;
  }

  .section-head h4,
  .intro-card h3 {
    margin: 0;
  }

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

  .toggle-row {
    margin-bottom: 0.25rem;
  }

  .setting-item {
    display: grid;
    gap: 0.4rem;
  }

  .summary-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
    gap: 0.75rem;
    margin-top: 1rem;
  }

  .summary-card {
    padding: 0.85rem 0.9rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 3%);
  }

  .summary-card strong,
  .summary-label {
    display: block;
  }

  .summary-card strong {
    line-height: 1.35;
  }

  .summary-card small {
    display: block;
    margin-top: 0.28rem;
    color: var(--text-secondary);
    font-size: 0.77rem;
    line-height: 1.45;
  }

  .summary-label {
    margin-bottom: 0.22rem;
    color: var(--text-secondary);
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .quick-actions {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 0.75rem;
    margin-top: 0.9rem;
  }

  .quick-link {
    display: block;
    padding: 0.85rem 0.95rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 3%);
    color: inherit;
    text-decoration: none;
    transition:
      transform 0.16s ease,
      border-color 0.16s ease,
      background 0.16s ease;
  }

  .quick-link:hover {
    border-color: color-mix(in srgb, var(--accent-color, #3b82f6), var(--border-color) 38%);
  }

  .quick-link strong {
    display: block;
    margin-bottom: 0.12rem;
    color: var(--text-primary);
  }

  .quick-link small {
    color: var(--text-secondary);
    line-height: 1.45;
  }

  .full-width {
    width: 100%;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .control-strip {
    padding: 0.8rem 0.9rem;
    border-radius: 12px;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    background: color-mix(in srgb, var(--bg-surface), transparent 4%);
  }

  .inline-label {
    font-weight: 600;
  }

  .readonly-value {
    color: var(--text-primary);
    font-weight: 600;
  }

  .field-title {
    font-weight: 600;
    color: var(--text-primary);
  }

  .suspend-panel {
    display: grid;
    gap: 0.9rem;
    padding: 1rem;
    border-radius: 14px;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    background: color-mix(in srgb, var(--bg-surface), #10253d 12%);
  }

  .service-panel {
    display: grid;
    gap: 0.9rem;
    padding: 1rem;
    border-radius: 14px;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    background: color-mix(in srgb, var(--bg-surface), #10253d 8%);
  }

  .compact-panel {
    gap: 0.75rem;
  }

  .mode-picker {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .mode-card {
    display: grid;
    gap: 0.3rem;
    text-align: left;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 12%);
    background: color-mix(in srgb, var(--bg-surface), transparent 4%);
    border-radius: 14px;
    padding: 0.9rem 1rem;
    color: var(--text-primary);
    transition:
      border-color 0.16s ease,
      transform 0.16s ease,
      background 0.16s ease;
  }

  .mode-card:hover {
    transform: translateY(-1px);
    border-color: color-mix(in srgb, var(--accent-color, #3b82f6), var(--border-color) 40%);
  }

  .mode-card.selected {
    border-color: var(--accent-color, #3b82f6);
    background: color-mix(in srgb, var(--accent-color, #3b82f6) 12%, var(--bg-surface));
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent-color, #3b82f6), transparent 35%);
  }

  .mode-card strong {
    font-size: 0.96rem;
  }

  .mode-card span {
    color: var(--text-secondary);
    font-size: 0.85rem;
  }

  .policy-preview {
    display: flex;
    justify-content: space-between;
    gap: 0.8rem;
    align-items: center;
    border-radius: 12px;
    padding: 0.8rem 0.9rem;
    background: color-mix(in srgb, var(--bg-surface), transparent 2%);
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 22%);
  }

  .policy-preview-label {
    color: var(--text-secondary);
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .mode-detail-card {
    display: grid;
    gap: 0.8rem;
    border-radius: 12px;
    padding: 0.9rem;
    background: color-mix(in srgb, var(--bg-surface), transparent 3%);
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 20%);
  }

  .chip-list,
  .quick-add-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
  }

  .preset-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .quick-add-chip,
  .btn-add-chip,
  .schedule-chip {
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    background: color-mix(in srgb, var(--bg-surface), transparent 4%);
    color: var(--text-primary);
    border-radius: 999px;
    padding: 0.45rem 0.8rem;
  }

  .preset-card {
    display: grid;
    gap: 0.55rem;
    text-align: left;
    padding: 0.9rem 1rem;
    border-radius: 14px;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    background: color-mix(in srgb, var(--bg-surface), transparent 4%);
    color: var(--text-primary);
    transition:
      border-color 0.16s ease,
      transform 0.16s ease,
      background 0.16s ease;
  }

  .preset-card:hover {
    transform: translateY(-1px);
    border-color: color-mix(in srgb, var(--accent-color, #3b82f6), var(--border-color) 40%);
  }

  .preset-card.active {
    border-color: var(--accent-color, #3b82f6);
    background: color-mix(in srgb, var(--accent-color, #3b82f6) 12%, var(--bg-surface));
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent-color, #3b82f6), transparent 35%);
  }

  .preset-card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .preset-card strong {
    font-size: 0.95rem;
  }

  .preset-card p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.84rem;
    line-height: 1.45;
  }

  .preset-card-meta {
    color: var(--text-secondary);
    font-size: 0.76rem;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .reminder-groups {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.9rem;
  }

  .reminder-group {
    display: grid;
    gap: 0.45rem;
    padding: 0.8rem;
    border-radius: 12px;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    background: color-mix(in srgb, var(--bg-surface), transparent 3%);
  }

  .reminder-group-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .reminder-group-head strong {
    font-size: 0.85rem;
    color: var(--text-primary);
  }

  .schedule-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding-inline: 0.7rem;
  }

  .schedule-chip small {
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.03em;
    font-size: 0.68rem;
  }

  .empty-chip {
    color: var(--text-secondary);
    font-size: 0.82rem;
  }

  .reminder-builder {
    display: grid;
    gap: 0.7rem;
    padding: 0.8rem;
    border-radius: 12px;
    border: 1px dashed color-mix(in srgb, var(--border-color), transparent 12%);
  }

  .builder-controls {
    display: grid;
    grid-template-columns: minmax(0, 1.5fr) 140px auto;
    gap: 0.65rem;
    align-items: center;
  }

  .timing-toggle {
    display: flex;
    flex-wrap: wrap;
    gap: 0.55rem;
  }

  .timing-chip {
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
    background: color-mix(in srgb, var(--bg-surface), transparent 4%);
    color: var(--text-primary);
    border-radius: 999px;
    padding: 0.58rem 0.88rem;
  }

  .timing-chip.active {
    border-color: var(--accent-color, #3b82f6);
    background: color-mix(in srgb, var(--accent-color, #3b82f6) 16%, var(--bg-surface));
  }

  .days-input-wrap {
    position: relative;
  }

  .days-input-wrap :global(input) {
    padding-right: 3.2rem;
  }

  .days-suffix {
    position: absolute;
    right: 0.85rem;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-secondary);
    font-size: 0.82rem;
    pointer-events: none;
  }

  .btn-add-chip {
    border-radius: 12px;
    font-weight: 700;
    padding-inline: 1rem;
  }

  .builder-head {
    display: grid;
    gap: 0.2rem;
  }

  .raw-schedule {
    display: grid;
    gap: 0.3rem;
    border-top: 1px dashed color-mix(in srgb, var(--border-color), transparent 16%);
    padding-top: 0.75rem;
  }

  .raw-schedule summary {
    cursor: pointer;
    color: var(--text-secondary);
    font-size: 0.82rem;
    font-weight: 600;
  }

  .raw-schedule code {
    display: inline-block;
    margin-top: 0.45rem;
    width: fit-content;
    padding: 0.35rem 0.55rem;
    border-radius: 10px;
    background: color-mix(in srgb, var(--bg-surface), transparent 2%);
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 18%);
  }

  @media (max-width: 720px) {
    .mode-picker {
      grid-template-columns: 1fr;
    }

    .policy-preview {
      align-items: flex-start;
      flex-direction: column;
    }

    .preset-grid,
    .reminder-groups,
    .builder-controls {
      grid-template-columns: 1fr;
    }
  }
</style>
