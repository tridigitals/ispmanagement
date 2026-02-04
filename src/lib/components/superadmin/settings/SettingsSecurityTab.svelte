<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { t } from 'svelte-i18n';

  export let maxLoginAttempts: number;
  export let lockoutDurationMinutes: number;
  export let apiRateLimitPerMinute: number;
  export let enableIpBlocking: boolean;
  export let twoFAEnabled: boolean;
  export let twoFAMethodTotp: boolean;
  export let twoFAMethodEmail: boolean;
  export let twoFAEmailOtpExpiryMinutes: number;

  const dispatch = createEventDispatcher();

  function handleChange() {
    dispatch('change');
  }
</script>

<div class="card section fade-in">
  <div class="card-header">
    <h3>
      {$t('superadmin.settings.sections.security') || 'Security & Rate Limiting'}
    </h3>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="max-login-attempts">
          {$t('superadmin.settings.security.max_login_attempts.label') || 'Max Login Attempts'}
        </label>
        <p class="setting-description">
          {$t('superadmin.settings.security.max_login_attempts.desc') ||
            'Number of failed login attempts before account lockout.'}
        </p>
      </div>
      <div class="input-group">
        <input
          type="number"
          id="max-login-attempts"
          bind:value={maxLoginAttempts}
          on:input={handleChange}
          min="1"
          max="20"
          class="form-input"
        />
        <span class="input-suffix">{$t('common.units.attempts') || 'attempts'}</span>
      </div>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="lockout-duration">
          {$t('superadmin.settings.security.lockout_duration.label') || 'Lockout Duration'}
        </label>
        <p class="setting-description">
          {$t('superadmin.settings.security.lockout_duration.desc') ||
            'How long a user stays locked out after max failed attempts.'}
        </p>
      </div>
      <div class="input-group">
        <input
          type="number"
          id="lockout-duration"
          bind:value={lockoutDurationMinutes}
          on:input={handleChange}
          min="1"
          max="1440"
          class="form-input"
        />
        <span class="input-suffix">{$t('common.units.minutes') || 'minutes'}</span>
      </div>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="api-rate-limit">
          {$t('superadmin.settings.security.api_rate_limit.label') || 'API Rate Limit'}
        </label>
        <p class="setting-description">
          {$t('superadmin.settings.security.api_rate_limit.desc') ||
            'Maximum API requests allowed per minute per user.'}
        </p>
      </div>
      <div class="input-group">
        <input
          type="number"
          id="api-rate-limit"
          bind:value={apiRateLimitPerMinute}
          on:input={handleChange}
          min="10"
          max="1000"
          class="form-input"
        />
        <span class="input-suffix">{$t('common.units.req_per_min') || 'req/min'}</span>
      </div>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-label">
          {$t('superadmin.settings.security.ip_blocking.label') || 'Enable IP Blocking'}
        </span>
        <p class="setting-description">
          {$t('superadmin.settings.security.ip_blocking.desc') ||
            'Automatically block IP addresses with suspicious activity.'}
        </p>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          bind:checked={enableIpBlocking}
          on:change={handleChange}
          aria-label={$t('superadmin.settings.security.ip_blocking.aria') || 'Enable IP Blocking'}
        />
        <span class="slider"></span>
      </label>
    </div>
  </div>
</div>

<div class="card section fade-in" style="margin-top: 1.5rem;">
  <div class="card-header">
    <h3>
      {$t('superadmin.settings.sections.twofa') || 'Two-Factor Authentication'}
    </h3>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-label">
          {$t('superadmin.settings.twofa.enable_2fa.label') || 'Enable 2FA'}
        </span>
        <p class="setting-description">
          {$t('superadmin.settings.twofa.enable_2fa.desc') ||
            'Allow users to set up two-factor authentication for enhanced security.'}
        </p>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          bind:checked={twoFAEnabled}
          on:change={handleChange}
          aria-label={$t('superadmin.settings.twofa.enable_2fa.aria') || 'Enable 2FA'}
        />
        <span class="slider"></span>
      </label>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-label">
          {$t('superadmin.settings.twofa.enable_totp.label') || 'TOTP (Authenticator App)'}
        </span>
        <p class="setting-description">
          {$t('superadmin.settings.twofa.enable_totp.desc') ||
            'Allow users to verify with Google Authenticator, Authy, etc.'}
        </p>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          bind:checked={twoFAMethodTotp}
          on:change={handleChange}
          aria-label={$t('superadmin.settings.twofa.enable_totp.aria') || 'Enable TOTP'}
        />
        <span class="slider"></span>
      </label>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-label">
          {$t('superadmin.settings.twofa.email_otp.label') || 'Email OTP'}
        </span>
        <p class="setting-description">
          {$t('superadmin.settings.twofa.email_otp.desc') ||
            'Allow users to receive verification codes via email.'}
        </p>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          bind:checked={twoFAMethodEmail}
          on:change={handleChange}
          aria-label={$t('superadmin.settings.twofa.enable_email_otp.aria') || 'Enable Email OTP'}
        />
        <span class="slider"></span>
      </label>
    </div>

    {#if twoFAMethodEmail}
      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="email-otp-expiry">
            {$t('superadmin.settings.twofa.email_otp_expiry.label') || 'Email OTP Expiry'}
          </label>
          <p class="setting-description">
            {$t('superadmin.settings.twofa.email_otp_expiry.desc') ||
              'How long email verification codes remain valid.'}
          </p>
        </div>
        <div class="input-group">
          <input
            type="number"
            id="email-otp-expiry"
            bind:value={twoFAEmailOtpExpiryMinutes}
            on:input={handleChange}
            min="1"
            max="60"
            class="form-input"
          />
          <span class="input-suffix">{$t('common.units.minutes') || 'minutes'}</span>
        </div>
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
