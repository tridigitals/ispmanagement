<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import { t } from 'svelte-i18n';
  import { appSettings } from '$lib/stores/settings';
  import { formatDate } from '$lib/utils/date';

  let {
    user,
    passwordData = $bindable(),
    twoFactorData = $bindable(),
    trustedDevices = [],
    loading = false,
    loadingDevices = false,
    policy,
    setupMethod = $bindable(),
    onChangePassword,
    onStart2FA,
    onVerify2FA,
    onDisable2FA,
    onSendDisableEmailOtp,
    onChange2FAMethod,
    onRevokeDevice,
    disableOtpSending,
    disableOtpSent,
  } = $props();

  let showCurrentPassword = $state(false);
  let showNewPassword = $state(false);
  let showConfirmPassword = $state(false);

  let deviceToRevoke = $state<any>(null);
  let showRevokeConfirm = $state(false);

  function confirmRevoke(device: any) {
    deviceToRevoke = device;
    showRevokeConfirm = true;
  }

  function handleRevokeConfirm() {
    if (deviceToRevoke) {
      onRevokeDevice(deviceToRevoke);
      deviceToRevoke = null;
      showRevokeConfirm = false;
    }
  }
</script>

<!-- Change Password Section -->
<div class="card section fade-in">
  <div class="card-header">
    <h3>{$t('profile.security.title')}</h3>
  </div>
  <div class="card-body">
    <form
      onsubmit={(e) => {
        e.preventDefault();
        onChangePassword();
      }}
    >
      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="current-pass">
            {$t('profile.security.current_password')}
          </label>
          <p class="setting-description">{$t('profile.security.settings_desc_current')}</p>
        </div>
        <div class="input-wrapper">
          <input
            type={showCurrentPassword ? 'text' : 'password'}
            id="current-pass"
            class="form-input"
            placeholder="••••••••"
            bind:value={passwordData.current}
          />
          <button
            type="button"
            class="toggle-password"
            onclick={() => (showCurrentPassword = !showCurrentPassword)}
            tabindex="-1"
          >
            <Icon name={showCurrentPassword ? 'eye-off' : 'eye'} size={18} />
          </button>
        </div>
      </div>

      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="new-pass">
            {$t('profile.security.new_password')}
          </label>
          <p class="setting-description">{$t('profile.security.settings_desc_new')} {policy.password_min_length}
            {$t('common.units.chars')}.</p>
        </div>
        <div class="input-wrapper">
          <input
            type={showNewPassword ? 'text' : 'password'}
            id="new-pass"
            class="form-input"
            placeholder="••••••••"
            bind:value={passwordData.new}
          />
          <button
            type="button"
            class="toggle-password"
            onclick={() => (showNewPassword = !showNewPassword)}
            tabindex="-1"
          >
            <Icon name={showNewPassword ? 'eye-off' : 'eye'} size={18} />
          </button>
        </div>
      </div>

      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="confirm-pass">
            {$t('profile.security.confirm_password')}
          </label>
          <p class="setting-description">{$t('profile.security.settings_desc_confirm')}</p>
        </div>
        <div class="input-wrapper">
          <input
            type={showConfirmPassword ? 'text' : 'password'}
            id="confirm-pass"
            class="form-input"
            placeholder="••••••••"
            bind:value={passwordData.confirm}
          />
          <button
            type="button"
            class="toggle-password"
            onclick={() => (showConfirmPassword = !showConfirmPassword)}
            tabindex="-1"
          >
            <Icon name={showConfirmPassword ? 'eye-off' : 'eye'} size={18} />
          </button>
        </div>
      </div>

      <!-- Password Requirements -->
      <div class="requirements-box">
        <span class="req-title">{$t('profile.security.requirements_title')}</span>
        <ul class="req-list">
          <li class:valid={passwordData.new.length >= policy.password_min_length}>
            <Icon name="check" size={14} />
            {$t('profile.security.req_length', {
              values: { length: policy.password_min_length },
            }) || `At least ${policy.password_min_length} characters`}
          </li>
          {#if policy.password_require_uppercase}
            <li class:valid={/[A-Z]/.test(passwordData.new)}>
              <Icon name="check" size={14} />
              {$t('profile.security.req_uppercase')}
            </li>
          {/if}
          {#if policy.password_require_number}
            <li class:valid={/[0-9]/.test(passwordData.new)}>
              <Icon name="check" size={14} />
              {$t('profile.security.req_number')}
            </li>
          {/if}
          {#if policy.password_require_special}
            <li class:valid={/[!@#$%^&*()_+\-=[\]{}|;:',.&lt;&gt;?/`~]/.test(passwordData.new)}>
              <Icon name="check" size={14} />
              {$t('profile.security.req_special')}
            </li>
          {/if}
        </ul>
      </div>

      <div class="form-actions">
        <button type="submit" class="btn btn-primary" disabled={loading}>
          {loading
            ? $t('profile.security.updating')
            : $t('profile.security.update_button') || 'Update Password'}
        </button>
      </div>
    </form>
  </div>
</div>

<!-- Two-Factor Authentication Section -->
<div class="card section fade-in">
  <div class="card-header">
    <h3>
      {$t('profile.security.twofa.title')}
    </h3>
  </div>
  <div class="card-body">
    {#if twoFactorData.enabled}
      <!-- 2FA Enabled State -->
      <div class="setting-row status-row success">
        <div class="status-icon">
          <Icon name="shield-check" size={24} />
        </div>
        <div class="setting-info">
          <span class="setting-label"
            >{$t('profile.security.twofa.enabled_title')}</span
          >
          <p class="setting-description">
            {$t('profile.security.twofa.enabled_desc')}
          </p>
        </div>
      </div>

      <!-- Enabled Methods -->
      <div class="methods-section">
        <span class="section-title">{$t('profile.security.twofa.enabled_methods')}</span>
        <div class="method-list">
          <div class="method-item">
            <Icon name="smartphone" size={18} />
            <span class="method-name"
              >{$t('profile.security.twofa.methods.authenticator_app')}</span
            >
            {#if user?.totp_enabled}
              <span class="badge success">{$t('profile.security.twofa.enabled_badge')}</span>
            {:else}
              <button
                class="btn btn-sm btn-outline"
                onclick={() => onStart2FA('totp')}
                disabled={loading}>{$t('profile.security.twofa.enable_button')}</button
              >
            {/if}
          </div>
          <div class="method-item">
            <Icon name="mail" size={18} />
            <span class="method-name"
              >{$t('profile.security.twofa.methods.email_verification')}</span
            >
            {#if user?.email_2fa_enabled}
              <span class="badge success">{$t('profile.security.twofa.enabled_badge')}</span>
            {:else}
              <button
                class="btn btn-sm btn-outline"
                onclick={() => onStart2FA('email')}
                disabled={loading}>{$t('profile.security.twofa.enable_button')}</button
              >
            {/if}
          </div>
        </div>
      </div>

      {#if twoFactorData.showRecovery}
        <!-- Recovery Codes -->
        <div class="recovery-box">
          <div class="recovery-header">
            <Icon name="alert-triangle" size={20} />
            <h4>
              {$t('profile.security.twofa.recovery_title')}
            </h4>
          </div>
          <p>{$t('profile.security.twofa.recovery_desc')}</p>
          <div class="code-grid">
            {#each twoFactorData.recoveryCodes as code}
              <div class="code-item">{code}</div>
            {/each}
          </div>
          <button
            class="btn btn-primary full-width"
            onclick={() => (twoFactorData.showRecovery = false)}
          >
            {$t('profile.security.twofa.recovery.saved_button') || "I've Saved These Codes"}
          </button>
        </div>
      {:else}
        <!-- Disable 2FA Row -->
        <div class="setting-row disable-row">
          <div class="setting-info">
            <span class="setting-label"
              >{$t('profile.security.twofa.disable_title')}</span
            >
            <p class="setting-description">{$t('profile.security.twofa.disable_desc')}</p>
          </div>
          <div class="disable-actions">
            {#if user?.preferred_2fa_method === 'email'}
              <button
                class="btn btn-outline btn-sm"
                onclick={onSendDisableEmailOtp}
                disabled={disableOtpSending}
              >
                {disableOtpSending ? $t('profile.security.twofa.sending') || 'Sending...' : disableOtpSent ? $t('profile.security.twofa.resend') || 'Resend' : $t('profile.security.twofa.send_code') || 'Send Code'}
              </button>
            {/if}
            <input
              type="text"
              class="form-input code-input"
              bind:value={twoFactorData.disableCode}
              placeholder={$t('profile.security.twofa.disable_code_placeholder')}
            />
            <button
              class="btn btn-danger btn-sm"
              onclick={onDisable2FA}
              disabled={twoFactorData.disableCode.length < 6 || loading}
            >
              {$t('profile.security.twofa.disable_button')}
            </button>
          </div>
        </div>
      {/if}
    {:else if !twoFactorData.showSetup}
      <!-- 2FA Not Enabled -->
      <div class="empty-state">
        <div class="empty-icon"><Icon name="shield" size={32} /></div>
        <h4>{$t('profile.security.twofa.enhance_security')}</h4>
        <p>{$t('profile.security.twofa.enhance_desc')}</p>
        <div class="setup-actions">
          <button class="btn btn-primary" onclick={() => onStart2FA('totp')} disabled={loading}>
            <Icon name="smartphone" size={18} />
            {$t('profile.security.twofa.authenticator_app')}
          </button>
          <button class="btn btn-secondary" onclick={() => onStart2FA('email')} disabled={loading}>
            <Icon name="mail" size={18} />
            {$t('profile.security.twofa.email_verification')}
          </button>
        </div>
      </div>
    {:else}
      <!-- 2FA Setup Flow -->
      {#if setupMethod === 'totp'}
        <div class="setup-flow">
          <div class="qr-section">
            <span class="step-label">{$t('profile.security.twofa.step_scan')}</span>
            <div class="qr-wrapper">
              <img src="data:image/png;base64,{twoFactorData.qr}" alt={$t('profile.security.twofa.qr_code')} class="qr-img" />
            </div>
            <p class="secret-text">Key: {twoFactorData.secret}</p>
          </div>
          <div class="verify-section">
            <span class="step-label">{$t('profile.security.twofa.step_enter_code')}</span>
            <input
              type="text"
              class="form-input code-input-lg"
              bind:value={twoFactorData.code}
              placeholder="000 000"
              maxlength="6"
            />
            <div class="setup-actions">
              <button class="btn btn-outline" onclick={() => (twoFactorData.showSetup = false)}
                >{$t('profile.security.twofa.cancel')}</button
              >
              <button
                class="btn btn-primary"
                onclick={onVerify2FA}
                disabled={twoFactorData.code.length < 6 || loading}
              >
                {loading ? $t('profile.security.twofa.verifying') : $t('profile.security.twofa.activate')}
              </button>
            </div>
          </div>
        </div>
      {:else}
        <div class="setup-flow centered">
          <div class="empty-icon"><Icon name="mail" size={32} /></div>
          <h4>{$t('profile.security.twofa.verify_email')}</h4>
          <p>
            {$t('profile.security.twofa.sent_code_to')} <strong>{user?.email}</strong>
          </p>
          <input
            type="text"
            class="form-input code-input-lg"
            bind:value={twoFactorData.code}
            placeholder="000000"
            maxlength="6"
          />
          <div class="setup-actions">
            <button class="btn btn-outline" onclick={() => (twoFactorData.showSetup = false)}
              >{$t('profile.security.twofa.cancel')}</button
            >
            <button
              class="btn btn-primary"
              onclick={onVerify2FA}
              disabled={twoFactorData.code.length < 6 || loading}
            >
              {loading ? $t('profile.security.twofa.verifying') : $t('profile.security.twofa.verify_enable')}
            </button>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>

<!-- Trusted Devices Section -->
<div class="card section fade-in">
  <div class="card-header">
    <h3>{$t('profile.security.twofa.trusted_devices.title')}</h3>
  </div>
  <div class="card-body">
    {#if loadingDevices}
      <div class="loading-state">
        <span class="spinner"></span>
        <span>{$t('profile.security.twofa.trusted_devices.loading')}</span>
      </div>
    {:else if trustedDevices.length === 0}
      <div class="empty-state small">
        <Icon name="monitor" size={24} />
        <p>{$t('profile.security.twofa.trusted_devices.empty')}</p>
      </div>
    {:else}
      <p class="section-desc">{$t('profile.security.twofa.trusted_devices.desc')}</p>
      <div class="device-list">
        {#each trustedDevices as device (device.id)}
          <div class="device-item">
            <div class="device-icon">
              {#if (device.user_agent || '').toLowerCase().includes('mobile')}
                <Icon name="smartphone" size={18} />
              {:else}
                <Icon name="monitor" size={18} />
              {/if}
            </div>
            <div class="device-info">
              <span class="device-name">{device.user_agent || $t('profile.security.twofa.trusted_devices.unknown_device')}</span>
              <span class="device-meta">
                {#if device.ip_address}{device.ip_address} •
                {/if}
                {$t('profile.security.twofa.trusted_devices.last_used')}: {formatDate(device.last_used_at, {
                  timeZone: $appSettings.app_timezone,
                })}
              </span>
            </div>
            <button class="btn btn-danger btn-sm" onclick={() => confirmRevoke(device)}>
              {$t('profile.security.twofa.trusted_devices.revoke')}</button
            >
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<ConfirmDialog
  bind:show={showRevokeConfirm}
  title={$t('profile.security.twofa.trusted_devices.revoke_title')}
  message={$t('profile.security.twofa.trusted_devices.revoke_message')}
  confirmText={$t('profile.security.twofa.trusted_devices.revoke')}
  type="danger"
  {loading}
  onconfirm={handleRevokeConfirm}
/>

<style>
  /* Card Styles (Matching Superadmin Settings) */
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
    background: var(--bg-surface);
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
    overflow: hidden;
  }

  /* Setting Row */
  .setting-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 1.25rem 0;
    border-bottom: 1px solid var(--border-color);
    gap: 1rem;
  }

  .setting-row:first-child {
    padding-top: 0;
  }

  .setting-row:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }

  .setting-info {
    flex: 1;
    min-width: 0;
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

  /* Input Wrapper */
  .input-wrapper {
    position: relative;
    width: 220px;
    flex-shrink: 0;
  }

  .form-input {
    width: 100%;
    padding: 0.6rem 0.75rem;
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

  .toggle-password {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 4px;
    display: flex;
    border-radius: 4px;
  }

  .toggle-password:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  /* Requirements Box */
  .requirements-box {
    margin-top: 1rem;
    padding: 1rem;
    background: var(--bg-app);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-color);
  }

  .req-title {
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--text-primary);
    display: block;
    margin-bottom: 0.75rem;
  }

  .req-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1.5rem;
  }

  .req-list li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8rem;
    color: var(--text-tertiary);
  }

  .req-list li.valid {
    color: var(--color-success);
  }

  /* Form Actions */
  .form-actions {
    margin-top: 1.5rem;
    display: flex;
    justify-content: flex-end;
    border-top: 1px solid var(--border-color);
    padding-top: 1.5rem;
  }

  /* Buttons */
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.6rem 1.25rem;
    border-radius: var(--radius-md);
    font-weight: 500;
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.2s;
    border: 1px solid transparent;
  }

  .btn-primary {
    background: var(--color-primary);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--color-primary-hover);
  }

  .btn-secondary {
    background: var(--bg-app);
    color: var(--text-primary);
    border-color: var(--border-color);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .btn-outline {
    background: transparent;
    border-color: var(--border-color);
    color: var(--text-primary);
  }

  .btn-outline:hover {
    background: var(--bg-hover);
  }

  .btn-danger {
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
    color: var(--color-danger);
    border-color: transparent;
  }

  .btn-danger:hover {
    background: var(--color-danger);
    color: var(--bg-app);
  }

  .btn-sm {
    padding: 0.4rem 0.8rem;
    font-size: 0.8rem;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .full-width {
    width: 100%;
  }

  /* Status Row */
  .status-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1rem;
    border-radius: var(--radius-md);
    border: none;
  }

  .status-row.success {
    background: var(--bg-success);
    border: 1px solid color-mix(in srgb, var(--color-success) 22%, var(--border-color));
  }

  .status-icon {
    width: 40px;
    height: 40px;
    background: var(--bg-surface);
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-success);
    flex-shrink: 0;
  }

  /* Methods Section */
  .methods-section {
    margin-top: 1.5rem;
    padding-top: 1.5rem;
    border-top: 1px solid var(--border-color);
  }

  .section-title {
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    font-weight: 700;
    display: block;
    margin-bottom: 1rem;
  }

  .method-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .method-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background: var(--bg-app);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
  }

  .method-name {
    flex: 1;
    font-weight: 500;
    color: var(--text-primary);
  }

  .badge {
    font-size: 0.7rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-weight: 600;
    text-transform: uppercase;
  }

  .badge.success {
    background: var(--bg-success);
    color: var(--color-success);
  }

  /* Disable Row */
  .disable-row {
    margin-top: 1.5rem;
    padding: 1rem;
    background: var(--bg-app);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-color);
  }

  .disable-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-shrink: 0;
  }

  .code-input {
    width: 120px;
    text-align: center;
    font-weight: 600;
    letter-spacing: 0.1em;
  }

  /* Empty State */
  .empty-state {
    text-align: center;
    padding: 2rem 1rem;
  }

  .empty-state.small {
    padding: 1.5rem;
  }

  .empty-icon {
    width: 56px;
    height: 56px;
    background: var(--bg-app);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0 auto 1rem;
    color: var(--color-primary);
  }

  .empty-state h4 {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 0.5rem 0;
  }

  .empty-state p {
    color: var(--text-secondary);
    max-width: 320px;
    margin: 0 auto 1.5rem;
    line-height: 1.5;
    font-size: 0.9rem;
  }

  .setup-actions {
    display: flex;
    gap: 0.75rem;
    justify-content: center;
    flex-wrap: wrap;
  }

  /* Recovery Box */
  .recovery-box {
    margin-top: 1.5rem;
    padding: 1.5rem;
    background: color-mix(in srgb, var(--color-warning) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-warning) 30%, var(--border-color));
    border-radius: var(--radius-md);
    text-align: center;
  }

  .recovery-header {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    color: var(--color-warning);
    margin-bottom: 0.75rem;
  }

  .recovery-header h4 {
    margin: 0;
    font-size: 1rem;
  }

  .recovery-box p {
    color: var(--text-secondary);
    font-size: 0.9rem;
    margin-bottom: 1rem;
  }

  .code-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
    gap: 0.5rem;
    margin-bottom: 1.5rem;
  }

  .code-item {
    font-family: monospace;
    font-size: 0.9rem;
    background: var(--bg-surface);
    padding: 0.5rem;
    border-radius: 4px;
    border: 1px solid var(--border-color);
  }

  /* Setup Flow */
  .setup-flow {
    text-align: center;
  }

  .setup-flow.centered {
    max-width: 360px;
    margin: 0 auto;
  }

  .qr-section {
    margin-bottom: 2rem;
  }

  .step-label {
    font-weight: 700;
    color: var(--text-secondary);
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    display: block;
    margin-bottom: 1rem;
  }

  .qr-wrapper {
    background: white;
    padding: 1rem;
    border-radius: var(--radius-md);
    display: inline-block;
    box-shadow: var(--shadow-sm);
  }

  .qr-img {
    width: 160px;
    height: 160px;
    display: block;
  }

  .secret-text {
    font-family: monospace;
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-top: 1rem;
  }

  .verify-section {
    max-width: 280px;
    margin: 0 auto;
  }

  .code-input-lg {
    width: 100%;
    text-align: center;
    font-size: 1.5rem;
    letter-spacing: 0.2em;
    font-weight: 700;
    padding: 0.75rem;
    margin-bottom: 1rem;
  }

  /* Device List */
  .section-desc {
    color: var(--text-secondary);
    font-size: 0.85rem;
    margin-bottom: 1rem;
  }

  .device-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    overflow: hidden;
  }

  .device-item {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1rem;
    background: var(--bg-app);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    overflow: hidden;
    max-width: 100%;
  }

  .device-icon {
    width: 36px;
    height: 36px;
    background: var(--bg-surface);
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .device-info {
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  .device-name {
    display: block;
    font-weight: 600;
    color: var(--text-primary);
    font-size: 0.9rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .device-meta {
    display: block;
    font-size: 0.8rem;
    color: var(--text-secondary);
    margin-top: 0.15rem;
  }

  /* Loading State */
  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    padding: 2rem;
    color: var(--text-secondary);
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
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

  /* Responsive */
  @media (max-width: 768px) {
    .setting-row {
      flex-direction: column;
      gap: 0.75rem;
    }

    .input-wrapper {
      width: 100%;
    }

    .disable-actions {
      flex-wrap: wrap;
      width: 100%;
    }

    .code-input {
      flex: 1;
      min-width: 100px;
    }

    .device-item {
      flex-wrap: wrap;
    }

    .device-info {
      min-width: 200px;
    }
  }

  @media (max-width: 480px) {
    .card-body {
      padding: 1rem;
    }

    .setup-actions {
      flex-direction: column;
    }

    .setup-actions .btn {
      width: 100%;
    }
  }
</style>
