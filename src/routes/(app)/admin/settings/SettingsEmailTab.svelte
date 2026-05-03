<script lang="ts">
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Select from '$lib/components/ui/Select.svelte';

  export let localSettings: Record<string, string>;
  export let emailProviderOptions: Array<{ value: string; label: string }>;
  export let smtpEncryptionOptions: Array<{ value: string; label: string }>;
  export let testEmailAddress = '';
  export let sendingTestEmail = false;
  export let testingSmtp = false;
  export let canReadEmailOutbox = false;
  export let handleChange: (key: string, value: any) => void;
  export let onSendTestEmail: () => void | Promise<void>;
  export let onTestSmtpConnection: () => void | Promise<void>;
  export let onViewOutbox: () => void | Promise<void>;

  function tt(key: string, fallback: string) {
    const value = get(t)(key);
    return value && value !== key ? value : fallback;
  }
</script>

<div class="email-settings">
  <span class="section-label">{tt('admin.settings.email.provider_label', 'Email Delivery Provider')}</span>
  <div class="provider-grid">
    {#each emailProviderOptions as option}
      <button
        class="provider-card"
        class:selected={localSettings['email_provider'] === option.value}
        onclick={() => handleChange('email_provider', option.value)}
      >
        <div class="p-icon">
          {#if option.value === 'smtp'}
            <Icon name="mail" size={24} />
          {:else}
            <Icon name="zap" size={24} />
          {/if}
        </div>
        <div class="p-info">
          <span class="p-name">{option.label}</span>
          <span class="p-desc">
            {#if option.value === 'smtp'}
              Direct SMTP server connection.
            {:else}
              High-performance API delivery.
            {/if}
          </span>
        </div>
        <div class="p-check">
          <Icon
            name={localSettings['email_provider'] === option.value ? 'check-circle' : 'circle'}
            size={20}
          />
        </div>
      </button>
    {/each}
  </div>

  <div class="config-panel fade-in">
    <h3>{tt('admin.settings.sections.sender_info', 'Sender Information')}</h3>
    <div class="config-grid mb-6">
      <div class="setting-item">
        <label for="email-from-name">{tt('admin.settings.keys.email_from_name', 'From Name')}</label>
        <Input
          id="email-from-name"
          value={localSettings['email_from_name']}
          oninput={(e: any) => handleChange('email_from_name', e.target.value)}
          placeholder={tt('admin.settings.email.placeholders.from_name', 'e.g. Acme Support')}
        />
      </div>
      <div class="setting-item">
        <label for="email-from-address">{tt('admin.settings.keys.email_from_address', 'From Address')}</label>
        <Input
          id="email-from-address"
          value={localSettings['email_from_address']}
          oninput={(e: any) => handleChange('email_from_address', e.target.value)}
          placeholder={tt(
            'admin.settings.email.placeholders.from_address',
            'noreply@yourdomain.com',
          )}
        />
      </div>
    </div>

    <div class="divider-line"></div>

    <h3 class="mt-6">{tt('admin.settings.email.connection_details', 'Connection Details')}</h3>
    <div class="config-grid">
      {#if localSettings['email_provider'] === 'smtp'}
        <div class="setting-item">
          <label for="smtp-host">{tt('admin.settings.keys.email_smtp_host', 'SMTP Host')}</label>
          <Input
            id="smtp-host"
            value={localSettings['email_smtp_host']}
            oninput={(e: any) => handleChange('email_smtp_host', e.target.value)}
            placeholder={tt('admin.settings.email.placeholders.smtp_host', 'smtp.mailtrap.io')}
          />
        </div>
        <div class="setting-item">
          <label for="smtp-port">{tt('admin.settings.keys.email_smtp_port', 'SMTP Port')}</label>
          <Input
            id="smtp-port"
            type="number"
            value={localSettings['email_smtp_port']}
            oninput={(e: any) => handleChange('email_smtp_port', e.target.value)}
            placeholder={tt('admin.settings.email.placeholders.smtp_port', '587')}
          />
        </div>
        <div class="setting-item">
          <label for="smtp-encryption">{tt('admin.settings.keys.email_smtp_encryption', 'Encryption')}</label>
          <Select
            id="smtp-encryption"
            options={smtpEncryptionOptions}
            value={localSettings['email_smtp_encryption']}
            onchange={(e: any) => handleChange('email_smtp_encryption', e.detail)}
          />
        </div>
        <div class="setting-item">
          <label for="smtp-username">{tt('admin.settings.keys.email_smtp_username', 'Username')}</label>
          <Input
            id="smtp-username"
            value={localSettings['email_smtp_username']}
            oninput={(e: any) => handleChange('email_smtp_username', e.target.value)}
          />
        </div>
        <div class="setting-item full-width">
          <label for="smtp-password">{tt('admin.settings.keys.email_smtp_password', 'Password')}</label>
          <Input
            id="smtp-password"
            type="password"
            value={localSettings['email_smtp_password']}
            oninput={(e: any) => handleChange('email_smtp_password', e.target.value)}
            placeholder="••••••••••••"
            showPasswordToggle={true}
          />
        </div>
      {:else}
        <div class="setting-item full-width">
          <label for="api-key">{tt('admin.settings.keys.email_api_key', 'API Key')}</label>
          <Input
            id="api-key"
            type="password"
            value={localSettings['email_api_key']}
            oninput={(e: any) => handleChange('email_api_key', e.target.value)}
            placeholder="re_123456789..."
            showPasswordToggle={true}
          />
        </div>
      {/if}
    </div>
  </div>

  <div class="config-panel fade-in mt-6">
    <h3>{tt('admin.settings.email.queue.title', 'Delivery Queue & Retry')}</h3>
    <p class="muted">
      {tt(
        'admin.settings.email.queue.desc',
        'Queue outgoing emails and automatically retry transient failures.',
      )}
    </p>
    <div class="config-grid">
      <div class="setting-item full-width">
        <div class="toggle-row">
          <div class="toggle-text">
            <div class="toggle-title">
              {tt('admin.settings.email.queue.enabled', 'Enable Email Outbox')}
            </div>
            <div class="toggle-sub">
              {tt(
                'admin.settings.email.queue.enabled_desc',
                'Recommended for production to prevent lost emails.',
              )}
            </div>
          </div>
          <label class="toggle">
            <input
              type="checkbox"
              checked={localSettings['email_outbox_enabled'] === 'true'}
              onchange={(e) => handleChange('email_outbox_enabled', e.currentTarget.checked)}
            />
            <span class="slider"></span>
          </label>
        </div>
      </div>

      <div class="setting-item">
        <label for="email-outbox-max">
          {tt('admin.settings.email.queue.max_attempts', 'Max Attempts')}
        </label>
        <Input
          id="email-outbox-max"
          type="number"
          value={localSettings['email_outbox_max_attempts']}
          oninput={(e: any) => handleChange('email_outbox_max_attempts', e.target.value)}
          placeholder="5"
        />
      </div>

      <div class="setting-item">
        <label for="email-outbox-delay">
          {tt('admin.settings.email.queue.base_delay', 'Base Delay (seconds)')}
        </label>
        <Input
          id="email-outbox-delay"
          type="number"
          value={localSettings['email_outbox_base_delay_seconds']}
          oninput={(e: any) => handleChange('email_outbox_base_delay_seconds', e.target.value)}
          placeholder="30"
        />
      </div>
    </div>

    {#if canReadEmailOutbox}
      <div class="queue-actions">
        <button class="btn btn-secondary" type="button" onclick={onViewOutbox}>
          <Icon name="mail" size={16} />
          {tt('admin.settings.email.queue.view_outbox', 'View Outbox')}
        </button>
      </div>
    {/if}
  </div>

  <div class="test-email-card mt-6">
    <div class="test-header">
      <Icon name="send" size={18} />
      <h4>{tt('admin.settings.sections.test_configuration', 'Test Configuration')}</h4>
    </div>
    <p>{tt('admin.settings.email.test.desc', 'Send a test email or verify SMTP connectivity.')}</p>
    <div class="test-form">
      <Input
        type="email"
        value={testEmailAddress}
        oninput={(e: any) => (testEmailAddress = e.target.value)}
        placeholder={tt('admin.settings.email.test.recipient_placeholder', 'Enter recipient email')}
      />
      <div class="test-actions">
        <button
          class="btn btn-secondary"
          onclick={onSendTestEmail}
          disabled={sendingTestEmail || !testEmailAddress}
        >
          {sendingTestEmail
            ? tt('admin.settings.email.test.sending', 'Sending...')
            : tt('admin.settings.email.test.send', 'Send Test')}
        </button>

        <button
          class="btn btn-secondary"
          onclick={onTestSmtpConnection}
          disabled={testingSmtp}
          title={tt(
            'admin.settings.email.smtp_test.hint',
            'Checks connectivity and auth without sending an email.',
          )}
        >
          <Icon name="activity" size={16} />
          {testingSmtp
            ? tt('admin.settings.email.smtp_test.testing', 'Testing...')
            : tt('admin.settings.email.smtp_test.button', 'Test SMTP')}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .setting-item {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .section-label {
    font-weight: 750;
    color: var(--text-primary);
    margin-bottom: 0.9rem;
    display: block;
    font-size: 0.95rem;
  }

  .provider-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 1rem;
    margin-bottom: 1.75rem;
  }

  .provider-card {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1.1rem 1.15rem;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    text-align: left;
    cursor: pointer;
    transition: all 0.2s;
    position: relative;
  }

  .provider-card:hover {
    border-color: rgba(99, 102, 241, 0.28);
    background: rgba(99, 102, 241, 0.06);
    transform: translateY(-1px);
  }

  .provider-card.selected {
    border-color: rgba(99, 102, 241, 0.42);
    background: var(--bg-surface);
    box-shadow: 0 14px 34px rgba(0, 0, 0, 0.22);
  }

  :global([data-theme='light']) .provider-card {
    background: rgba(255, 255, 255, 0.75);
  }

  :global([data-theme='light']) .provider-card:hover {
    background: rgba(99, 102, 241, 0.06);
  }

  :global([data-theme='light']) .provider-card.selected {
    background: var(--bg-surface);
    box-shadow: 0 14px 34px rgba(0, 0, 0, 0.08);
  }

  .p-icon {
    width: 42px;
    height: 42px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    border: 1px solid var(--glass-border);
  }

  .selected .p-icon {
    background: rgba(99, 102, 241, 0.16);
    color: var(--text-primary);
    border-color: rgba(99, 102, 241, 0.3);
  }

  .p-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .p-name {
    font-weight: 750;
    color: var(--text-primary);
    font-size: 0.95rem;
    line-height: 1.2;
  }

  .p-desc {
    font-size: 0.82rem;
    color: var(--text-secondary);
    margin-top: 0.15rem;
  }

  .p-check {
    color: rgba(255, 255, 255, 0.18);
  }

  :global([data-theme='light']) .p-check {
    color: rgba(0, 0, 0, 0.18);
  }

  .selected .p-check {
    color: rgba(99, 102, 241, 0.9);
  }

  .config-panel {
    background: rgba(255, 255, 255, 0.02);
    padding: 1.25rem;
    border-radius: var(--radius-lg);
    border: 1px solid var(--glass-border);
  }

  :global([data-theme='light']) .config-panel {
    background: rgba(255, 255, 255, 0.75);
  }

  .config-panel h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    font-weight: 800;
    letter-spacing: 0.01em;
  }

  .config-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.25rem;
  }

  .divider-line {
    height: 1px;
    background: var(--glass-border);
    margin: 1.5rem 0;
  }

  .mb-6 {
    margin-bottom: 1.5rem;
  }

  .mt-6 {
    margin-top: 1.5rem;
  }

  .full-width {
    grid-column: 1 / -1;
  }

  .muted {
    color: var(--text-secondary);
  }

  .toggle {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
    flex-shrink: 0;
  }

  .toggle input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .toggle-text {
    min-width: 0;
  }

  .toggle-title {
    font-weight: 800;
    color: var(--text-primary);
  }

  .toggle-sub {
    margin-top: 0.2rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
    line-height: 1.35;
    font-weight: 600;
  }

  .slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(255, 255, 255, 0.1);
    transition: 0.3s;
    border-radius: var(--radius-lg);
    border: 1px solid var(--glass-border);
  }

  :global([data-theme='light']) .slider {
    background-color: rgba(0, 0, 0, 0.06);
  }

  .slider:before {
    position: absolute;
    content: '';
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 3px;
    background-color: white;
    transition: 0.3s;
    border-radius: 50%;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  input:checked + .slider {
    background-color: var(--color-primary);
    border-color: rgba(99, 102, 241, 0.4);
  }

  input:checked + .slider:before {
    transform: translateX(20px);
  }

  .test-email-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    padding: 1.1rem 1.25rem;
  }

  :global([data-theme='light']) .test-email-card {
    background: rgba(255, 255, 255, 0.75);
  }

  .test-header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    color: var(--text-primary);
    margin-bottom: 0.35rem;
    font-weight: 750;
  }

  .test-header h4 {
    margin: 0;
    font-size: 1rem;
    font-weight: 800;
  }

  .test-email-card p {
    font-size: 0.88rem;
    color: var(--text-secondary);
    margin-bottom: 1rem;
  }

  .test-form {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .test-form :global(.input-wrapper) {
    flex: 1;
    min-width: 220px;
  }

  .test-actions {
    display: inline-flex;
    gap: 0.6rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .queue-actions {
    margin-top: 0.9rem;
    display: flex;
    justify-content: flex-end;
  }

  @media (max-width: 640px) {
    .config-grid {
      grid-template-columns: 1fr;
    }

    .test-form {
      flex-direction: column;
      align-items: stretch;
    }

    .test-form :global(.input-wrapper) {
      min-width: unset;
    }
  }
</style>
