<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { t } from 'svelte-i18n';

  type BackupMode = 'minute' | 'hour' | 'day' | 'week';
  type Weekday = 'mon' | 'tue' | 'wed' | 'thu' | 'fri' | 'sat' | 'sun';

  export let appTimezone: string;

  export let backupGlobalEnabled: boolean;
  export let backupGlobalMode: BackupMode;
  export let backupGlobalEvery: number;
  export let backupGlobalAt: string; // HH:MM
  export let backupGlobalWeekday: Weekday;
  export let backupGlobalRetentionDays: number;

  export let backupTenantEnabled: boolean;
  export let backupTenantMode: BackupMode;
  export let backupTenantEvery: number;
  export let backupTenantAt: string; // HH:MM
  export let backupTenantWeekday: Weekday;
  export let backupTenantRetentionDays: number;

  const dispatch = createEventDispatcher();

  function handleChange() {
    dispatch('change');
  }

  function triggerGlobal() {
    dispatch('triggerGlobal');
  }

  function triggerTenants() {
    dispatch('triggerTenants');
  }
</script>

<div class="card section fade-in">
  <div class="card-header">
    <h3>{$t('superadmin.settings.sections.backup_global')}</h3>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="backup-global-enabled">
          {$t('superadmin.settings.backups.global.enable')}
        </label>
        <p class="setting-description">
          {$t('superadmin.settings.backups.global.enable_desc')}
        </p>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          id="backup-global-enabled"
          bind:checked={backupGlobalEnabled}
          on:change={handleChange}
          aria-label={$t('superadmin.settings.backups.global.enable_aria')}
        />
        <span class="slider"></span>
      </label>
    </div>

    {#if backupGlobalEnabled}
      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="backup-global-mode">
            {$t('superadmin.settings.backups.mode.label')}
          </label>
          <p class="setting-description">
            {$t('superadmin.settings.backups.mode.desc')}
          </p>
        </div>
        <select
          id="backup-global-mode"
          bind:value={backupGlobalMode}
          on:change={handleChange}
          class="form-input native-select"
        >
          <option value="minute"
            >{$t('superadmin.settings.backups.modes.minute')}</option
          >
          <option value="hour"
            >{$t('superadmin.settings.backups.modes.hour')}</option
          >
          <option value="day"
            >{$t('superadmin.settings.backups.modes.day')}</option
          >
          <option value="week"
            >{$t('superadmin.settings.backups.modes.week')}</option
          >
        </select>
      </div>

      {#if backupGlobalMode === 'minute' || backupGlobalMode === 'hour'}
        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label" for="backup-global-every">
              {$t('superadmin.settings.backups.every.label')}
            </label>
            <p class="setting-description">
              {$t('superadmin.settings.backups.every.desc')}
            </p>
          </div>
          <div class="input-group">
            <input
              type="number"
              id="backup-global-every"
              bind:value={backupGlobalEvery}
              on:input={handleChange}
              min="1"
              max="10080"
              class="form-input"
            />
            <span class="input-suffix">
              {backupGlobalMode === 'minute'
                ? $t('common.units.minutes') || 'minutes'
                : $t('common.units.hours') || 'hours'}
            </span>
          </div>
        </div>
      {/if}

      {#if backupGlobalMode === 'day' || backupGlobalMode === 'week'}
        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label" for="backup-global-at">
              {$t('superadmin.settings.backups.at.label')}
              <span class="hint">({appTimezone})</span>
            </label>
            <p class="setting-description">
              {$t('superadmin.settings.backups.at.desc')}
            </p>
          </div>
          <input
            type="time"
            id="backup-global-at"
            bind:value={backupGlobalAt}
            on:input={handleChange}
            class="form-input small-input"
          />
        </div>
      {/if}

      {#if backupGlobalMode === 'week'}
        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label" for="backup-global-weekday">
              {$t('superadmin.settings.backups.weekday.label')}
            </label>
            <p class="setting-description">
              {$t('superadmin.settings.backups.weekday.desc')}
            </p>
          </div>
          <select
            id="backup-global-weekday"
            bind:value={backupGlobalWeekday}
            on:change={handleChange}
            class="form-input native-select"
          >
            <option value="mon">{$t('common.weekdays.mon')}</option>
            <option value="tue">{$t('common.weekdays.tue')}</option>
            <option value="wed">{$t('common.weekdays.wed')}</option>
            <option value="thu">{$t('common.weekdays.thu')}</option>
            <option value="fri">{$t('common.weekdays.fri')}</option>
            <option value="sat">{$t('common.weekdays.sat')}</option>
            <option value="sun">{$t('common.weekdays.sun')}</option>
          </select>
        </div>
      {/if}

      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="backup-global-retention">
            {$t('superadmin.settings.backups.global.retention')}
          </label>
          <p class="setting-description">
            {$t('superadmin.settings.backups.retention_desc')}
          </p>
        </div>
        <div class="input-group">
          <input
            type="number"
            id="backup-global-retention"
            bind:value={backupGlobalRetentionDays}
            on:input={handleChange}
            min="1"
            max="3650"
            class="form-input"
          />
          <span class="input-suffix">{$t('common.units.days')}</span>
        </div>
      </div>

      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">
            {$t('superadmin.settings.backups.global.run_now')}
          </span>
          <p class="setting-description">
            {$t('superadmin.settings.backups.global.run_now_desc')}
          </p>
        </div>
        <button class="btn btn-secondary" on:click={triggerGlobal}>
          {$t('superadmin.settings.backups.global.run_now_btn')}
        </button>
      </div>
    {/if}
  </div>
</div>

<div class="card section fade-in" style="margin-top: 1.5rem;">
  <div class="card-header">
    <h3>{$t('superadmin.settings.sections.backup_tenant')}</h3>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="backup-tenant-enabled">
          {$t('superadmin.settings.backups.tenant.enable')}
        </label>
        <p class="setting-description">
          {$t('superadmin.settings.backups.tenant.enable_desc')}
        </p>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          id="backup-tenant-enabled"
          bind:checked={backupTenantEnabled}
          on:change={handleChange}
          aria-label={$t('superadmin.settings.backups.tenant.enable_aria')}
        />
        <span class="slider"></span>
      </label>
    </div>

    {#if backupTenantEnabled}
      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="backup-tenant-mode">
            {$t('superadmin.settings.backups.mode.label')}
          </label>
          <p class="setting-description">
            {$t('superadmin.settings.backups.mode.desc')}
          </p>
        </div>
        <select
          id="backup-tenant-mode"
          bind:value={backupTenantMode}
          on:change={handleChange}
          class="form-input native-select"
        >
          <option value="minute"
            >{$t('superadmin.settings.backups.modes.minute')}</option
          >
          <option value="hour"
            >{$t('superadmin.settings.backups.modes.hour')}</option
          >
          <option value="day"
            >{$t('superadmin.settings.backups.modes.day')}</option
          >
          <option value="week"
            >{$t('superadmin.settings.backups.modes.week')}</option
          >
        </select>
      </div>

      {#if backupTenantMode === 'minute' || backupTenantMode === 'hour'}
        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label" for="backup-tenant-every">
              {$t('superadmin.settings.backups.every.label')}
            </label>
            <p class="setting-description">
              {$t('superadmin.settings.backups.every.desc')}
            </p>
          </div>
          <div class="input-group">
            <input
              type="number"
              id="backup-tenant-every"
              bind:value={backupTenantEvery}
              on:input={handleChange}
              min="1"
              max="10080"
              class="form-input"
            />
            <span class="input-suffix">
              {backupTenantMode === 'minute'
                ? $t('common.units.minutes') || 'minutes'
                : $t('common.units.hours') || 'hours'}
            </span>
          </div>
        </div>
      {/if}

      {#if backupTenantMode === 'day' || backupTenantMode === 'week'}
        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label" for="backup-tenant-at">
              {$t('superadmin.settings.backups.at.label')}
              <span class="hint">({appTimezone})</span>
            </label>
            <p class="setting-description">
              {$t('superadmin.settings.backups.at.desc')}
            </p>
          </div>
          <input
            type="time"
            id="backup-tenant-at"
            bind:value={backupTenantAt}
            on:input={handleChange}
            class="form-input small-input"
          />
        </div>
      {/if}

      {#if backupTenantMode === 'week'}
        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label" for="backup-tenant-weekday">
              {$t('superadmin.settings.backups.weekday.label')}
            </label>
            <p class="setting-description">
              {$t('superadmin.settings.backups.weekday.desc')}
            </p>
          </div>
          <select
            id="backup-tenant-weekday"
            bind:value={backupTenantWeekday}
            on:change={handleChange}
            class="form-input native-select"
          >
            <option value="mon">{$t('common.weekdays.mon')}</option>
            <option value="tue">{$t('common.weekdays.tue')}</option>
            <option value="wed">{$t('common.weekdays.wed')}</option>
            <option value="thu">{$t('common.weekdays.thu')}</option>
            <option value="fri">{$t('common.weekdays.fri')}</option>
            <option value="sat">{$t('common.weekdays.sat')}</option>
            <option value="sun">{$t('common.weekdays.sun')}</option>
          </select>
        </div>
      {/if}

      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="backup-tenant-retention">
            {$t('superadmin.settings.backups.tenant.retention')}
          </label>
          <p class="setting-description">
            {$t('superadmin.settings.backups.retention_desc')}
          </p>
        </div>
        <div class="input-group">
          <input
            type="number"
            id="backup-tenant-retention"
            bind:value={backupTenantRetentionDays}
            on:input={handleChange}
            min="1"
            max="3650"
            class="form-input"
          />
          <span class="input-suffix">{$t('common.units.days')}</span>
        </div>
      </div>

      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">
            {$t('superadmin.settings.backups.tenant.run_now')}
          </span>
          <p class="setting-description">
            {$t('superadmin.settings.backups.tenant.run_now_desc')}
          </p>
        </div>
        <button class="btn btn-secondary" on:click={triggerTenants}>
          {$t('superadmin.settings.backups.tenant.run_now_btn')}
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  /* Match existing settings tab styling */
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

  @media (max-width: 600px) {
    .setting-row {
      flex-direction: column;
      gap: 0.5rem;
    }

    .setting-info {
      padding-right: 0;
    }

    .card-body {
      padding: 1rem;
    }

    .sub-settings {
      padding: 1rem;
    }

    .sub-settings .setting-row {
      gap: 0.5rem;
    }
  }

  .setting-info {
    flex: 1;
    padding-right: 1.5rem;
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
    max-width: min(400px, 100%);
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

  .native-select {
    appearance: auto;
    padding-right: 2.25rem;
  }

  .small-input {
    width: 160px;
    text-align: right;
  }

  .input-group {
    display: flex;
    align-items: center;
    width: 220px;
  }

  .input-group .form-input {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
    text-align: right;
    max-width: 120px;
  }

  .hint {
    font-weight: 500;
    color: var(--text-secondary);
    margin-left: 0.35rem;
  }

  .input-suffix {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-left: none;
    padding: 0.5rem 0.75rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
    border-top-right-radius: var(--radius-sm);
    border-bottom-right-radius: var(--radius-sm);
    white-space: nowrap;
    height: 38px;
    display: flex;
    align-items: center;
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
    border-radius: var(--radius-lg);
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

  @media (max-width: 900px) {
    .setting-row {
      flex-direction: column;
      gap: 0.75rem;
    }

    .setting-info {
      padding-right: 0;
    }

    .form-input {
      max-width: none;
    }

    .input-group {
      width: 100%;
    }
  }
</style>
