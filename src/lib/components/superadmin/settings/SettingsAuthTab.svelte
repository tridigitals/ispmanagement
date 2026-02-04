<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { t } from 'svelte-i18n';

  export let authAllowRegistration: boolean;
  export let authRequireEmailVerification: boolean;
  export let authJwtExpiryHours: number;
  export let authSessionTimeoutMinutes: number;

  const dispatch = createEventDispatcher();

  function handleChange() {
    dispatch('change');
  }
</script>

<div class="card section fade-in">
  <div class="card-header">
    <h3>
      {$t('superadmin.settings.sections.auth') || 'Authentication Settings'}
    </h3>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="allow-registration">
          {$t('superadmin.settings.auth.allow_public_registration.label') ||
            'Allow Public Registration'}
        </label>
        <p class="setting-description">
          {$t('superadmin.settings.auth.allow_public_registration.desc') ||
            'Allow new users to sign up freely.'}
        </p>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          id="allow-registration"
          bind:checked={authAllowRegistration}
          on:change={handleChange}
        />
        <span class="slider"></span>
      </label>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="require-email-verify">
          {$t('superadmin.settings.auth.require_email_verification.label') ||
            'Require Email Verification'}
        </label>
        <p class="setting-description">
          {$t('superadmin.settings.auth.require_email_verification.desc') ||
            'Users must verify email before logging in.'}
        </p>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          id="require-email-verify"
          bind:checked={authRequireEmailVerification}
          on:change={handleChange}
        />
        <span class="slider"></span>
      </label>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="jwt-expiry">
          {$t('superadmin.settings.auth.jwt_expiry.label') || 'JWT Expiry (Hours)'}
        </label>
        <p class="setting-description">
          {$t('superadmin.settings.auth.jwt_expiry.desc') ||
            'How long an auth token remains valid.'}
        </p>
      </div>
      <div class="input-group">
        <input
          type="number"
          id="jwt-expiry"
          bind:value={authJwtExpiryHours}
          on:input={handleChange}
          min="1"
          class="form-input"
        />
        <span class="input-suffix">{$t('common.units.hours') || 'hours'}</span>
      </div>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="session-timeout">
          {$t('superadmin.settings.auth.session_timeout.label') || 'Session Timeout (Minutes)'}
        </label>
        <p class="setting-description">
          {$t('superadmin.settings.auth.session_timeout.desc') || 'Auto-logout after inactivity.'}
        </p>
      </div>
      <div class="input-group">
        <input
          type="number"
          id="session-timeout"
          bind:value={authSessionTimeoutMinutes}
          on:input={handleChange}
          min="5"
          class="form-input"
        />
        <span class="input-suffix">{$t('common.units.min') || 'min'}</span>
      </div>
    </div>
  </div>
</div>

<style>
  /* Styling consistent with SettingsGeneralTab.svelte */
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

  .input-group {
    display: flex;
    align-items: center;
    width: 160px;
  }

  .input-group .form-input {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
    text-align: right;
    max-width: 100px;
  }

  .input-suffix {
    background: rgba(0, 0, 0, 0.2);
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
