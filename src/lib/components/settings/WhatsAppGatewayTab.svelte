<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import { DEFAULT_WHATSAPP_GATEWAY_FORM } from '$lib/utils/whatsappGateway';
  import type {
    WhatsAppEventDefinition,
    WhatsAppEventScope,
    WhatsAppGatewayProvider,
  } from '$lib/api/types';

  let {
    localSettings,
    handleChange,
    eventScope,
    title = 'WhatsApp Gateway',
    description = 'Configure WhatsApp message delivery and choose which notification events use WhatsApp.',
  }: {
    localSettings: Record<string, string>;
    handleChange: (key: string, value: string | boolean) => void;
    eventScope: WhatsAppEventScope;
    title?: string;
    description?: string;
  } = $props();

  const providerOptions: Array<{
    value: WhatsAppGatewayProvider;
    label: string;
    icon: string;
    desc: string;
  }> = [
    {
      value: 'disabled',
      label: 'Disabled',
      icon: 'circle-off',
      desc: 'Do not send WhatsApp notifications.',
    },
    {
      value: 'fonnte',
      label: 'Fonnte',
      icon: 'message-circle',
      desc: 'Use Fonnte API token delivery.',
    },
    {
      value: 'custom_http',
      label: 'Custom HTTP',
      icon: 'webhook',
      desc: 'POST to your own WhatsApp gateway endpoint.',
    },
  ];

  const methodOptions = [
    { value: 'POST', label: 'POST' },
    { value: 'PUT', label: 'PUT' },
    { value: 'PATCH', label: 'PATCH' },
  ];
  const customHeadersPlaceholder = '{"Authorization":"Bearer YOUR_TOKEN"}';
  const customBodyPlaceholder = '{"phone":"{{phone}}","message":"{{message}}"}';

  let events = $state<WhatsAppEventDefinition[]>([]);
  let testPhone = $state('');
  let testMessage = $state('Test WhatsApp message from ISP Management.');
  let testEventCode = $state('');
  let sendingTest = $state(false);
  let testResult = $state('');
  let testError = $state('');

  const provider = $derived(
    (localSettings['wa_gateway_provider'] ||
      DEFAULT_WHATSAPP_GATEWAY_FORM.provider) as WhatsAppGatewayProvider,
  );
  const enabled = $derived(localSettings['wa_gateway_enabled'] === 'true');
  const scopedEvents = $derived(events.filter((event) => event.scope === eventScope));

  onMount(() => {
    void loadEvents();
  });

  async function loadEvents() {
    try {
      events = await api.whatsapp.listEvents();
    } catch {
      events = [];
    }
  }

  function setProvider(nextProvider: WhatsAppGatewayProvider) {
    handleChange('wa_gateway_provider', nextProvider);
    handleChange('wa_gateway_enabled', nextProvider === 'disabled' ? 'false' : 'true');
  }

  async function sendTest() {
    testResult = '';
    testError = '';

    if (!testPhone.trim()) {
      testError = 'Enter a recipient phone number first.';
      return;
    }

    sendingTest = true;
    try {
      const result = await api.whatsapp.sendTest({
        phone: testPhone.trim(),
        message: testMessage.trim() || 'Test WhatsApp message from ISP Management.',
        eventCode: testEventCode || undefined,
      });
      if (result.ok) {
        testResult = result.message_id
          ? `Sent successfully. Message ID: ${result.message_id}`
          : 'Sent successfully.';
      } else {
        testError = result.error || 'WhatsApp provider rejected the test message.';
      }
    } catch (error: any) {
      testError = error?.message || 'Failed to send test WhatsApp message.';
    } finally {
      sendingTest = false;
    }
  }
</script>

<div class="whatsapp-settings">
  <div class="config-panel fade-in">
    <div class="panel-heading">
      <div>
        <h3>{title}</h3>
        <p>{description}</p>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          checked={enabled && provider !== 'disabled'}
          disabled={provider === 'disabled'}
          onchange={(event) => handleChange('wa_gateway_enabled', event.currentTarget.checked)}
        />
        <span class="slider"></span>
      </label>
    </div>

    <span class="section-label">Provider</span>
    <div class="provider-grid">
      {#each providerOptions as option}
        <button
          class="provider-card"
          class:selected={provider === option.value}
          type="button"
          onclick={() => setProvider(option.value)}
        >
          <div class="p-icon">
            <Icon name={option.icon} size={24} />
          </div>
          <div class="p-info">
            <span class="p-name">{option.label}</span>
            <span class="p-desc">{option.desc}</span>
          </div>
          <div class="p-check">
            <Icon name={provider === option.value ? 'check-circle' : 'circle'} size={20} />
          </div>
        </button>
      {/each}
    </div>
  </div>

  {#if provider === 'fonnte'}
    <div class="config-panel fade-in mt-6">
      <h3>Fonnte Configuration</h3>
      <div class="config-grid">
        <div class="setting-item full-width">
          <label for="wa-fonnte-token">API Token</label>
          <Input
            id="wa-fonnte-token"
            type="password"
            value={localSettings['wa_gateway_fonnte_token'] || ''}
            showPasswordToggle={true}
            oninput={(event: any) =>
              handleChange('wa_gateway_fonnte_token', event.target.value)}
            placeholder="Fonnte token"
          />
        </div>
        <div class="setting-item">
          <label for="wa-fonnte-base-url">Base URL</label>
          <Input
            id="wa-fonnte-base-url"
            value={localSettings['wa_gateway_fonnte_base_url'] || ''}
            oninput={(event: any) =>
              handleChange('wa_gateway_fonnte_base_url', event.target.value)}
            placeholder="https://api.fonnte.com"
          />
        </div>
        <div class="setting-item">
          <label for="wa-fonnte-sender">Sender</label>
          <Input
            id="wa-fonnte-sender"
            value={localSettings['wa_gateway_fonnte_sender'] || ''}
            oninput={(event: any) =>
              handleChange('wa_gateway_fonnte_sender', event.target.value)}
            placeholder="Optional sender/device"
          />
        </div>
      </div>
    </div>
  {:else if provider === 'custom_http'}
    <div class="config-panel fade-in mt-6">
      <h3>Custom HTTP Configuration</h3>
      <div class="config-grid">
        <div class="setting-item">
          <label for="wa-custom-method">Method</label>
          <Select
            id="wa-custom-method"
            options={methodOptions}
            value={localSettings['wa_gateway_custom_method'] || 'POST'}
            onchange={(event: any) => handleChange('wa_gateway_custom_method', event.detail)}
          />
        </div>
        <div class="setting-item">
          <label for="wa-custom-statuses">Success Statuses</label>
          <Input
            id="wa-custom-statuses"
            value={localSettings['wa_gateway_custom_success_statuses'] || '200,201,202'}
            oninput={(event: any) =>
              handleChange('wa_gateway_custom_success_statuses', event.target.value)}
          />
        </div>
        <div class="setting-item full-width">
          <label for="wa-custom-url">Endpoint URL</label>
          <Input
            id="wa-custom-url"
            value={localSettings['wa_gateway_custom_url'] || ''}
            oninput={(event: any) => handleChange('wa_gateway_custom_url', event.target.value)}
            placeholder="https://gateway.example.com/send"
          />
        </div>
        <div class="setting-item full-width">
          <label for="wa-custom-headers">Headers JSON</label>
          <textarea
            id="wa-custom-headers"
            class="form-textarea code"
            rows="4"
            value={localSettings['wa_gateway_custom_headers'] || ''}
            oninput={(event) =>
              handleChange('wa_gateway_custom_headers', event.currentTarget.value)}
            placeholder={customHeadersPlaceholder}
          ></textarea>
        </div>
        <div class="setting-item full-width">
          <label for="wa-custom-body">Body Template JSON</label>
          <textarea
            id="wa-custom-body"
            class="form-textarea code"
            rows="7"
            value={localSettings['wa_gateway_custom_body_template'] || ''}
            oninput={(event) =>
              handleChange('wa_gateway_custom_body_template', event.currentTarget.value)}
            placeholder={customBodyPlaceholder}
          ></textarea>
          <p class="help-text">
            Common placeholders: <code>{'{{phone}}'}</code>, <code>{'{{message}}'}</code>,
            <code>{'{{event_code}}'}</code>.
          </p>
        </div>
      </div>
    </div>
  {/if}

  <div class="config-panel fade-in mt-6">
    <h3>Send Test</h3>
    <div class="config-grid">
      <div class="setting-item">
        <label for="wa-test-phone">Recipient Phone</label>
        <Input
          id="wa-test-phone"
          value={testPhone}
          oninput={(event: any) => (testPhone = event.target.value)}
          placeholder="628123456789"
        />
      </div>
      <div class="setting-item">
        <label for="wa-test-event">Event Code</label>
        <select
          id="wa-test-event"
          class="form-input"
          bind:value={testEventCode}
          disabled={scopedEvents.length === 0}
        >
          <option value="">Manual test</option>
          {#each scopedEvents as event}
            <option value={event.code}>{event.label}</option>
          {/each}
        </select>
      </div>
      <div class="setting-item full-width">
        <label for="wa-test-message">Message</label>
        <textarea
          id="wa-test-message"
          class="form-textarea"
          rows="3"
          bind:value={testMessage}
        ></textarea>
      </div>
    </div>
    <div class="test-actions">
      <button
        class="btn btn-primary"
        type="button"
        disabled={sendingTest || provider === 'disabled' || !enabled}
        onclick={sendTest}
      >
        {#if sendingTest}
          <div class="spinner-sm"></div>
          Sending...
        {:else}
          <Icon name="send" size={16} />
          Send Test
        {/if}
      </button>
      {#if testResult}
        <span class="status success">{testResult}</span>
      {/if}
      {#if testError}
        <span class="status error">{testError}</span>
      {/if}
    </div>
  </div>
</div>

<style>
  .whatsapp-settings {
    display: flex;
    flex-direction: column;
  }

  .panel-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1.25rem;
  }

  .panel-heading h3,
  .config-panel h3 {
    margin: 0 0 0.35rem;
    color: var(--text-primary);
    font-size: 1rem;
    font-weight: 650;
  }

  .panel-heading p,
  .help-text {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.875rem;
    line-height: 1.5;
  }

  .section-label {
    display: block;
    margin-bottom: 0.75rem;
    color: var(--text-secondary);
    font-size: 0.8rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  .provider-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 0.75rem;
  }

  .provider-card {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    width: 100%;
    padding: 1rem;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 0.2s,
      background 0.2s;
  }

  .provider-card:hover,
  .provider-card.selected {
    border-color: var(--color-primary);
    background: var(--color-primary-subtle);
  }

  .p-icon,
  .p-check {
    color: var(--color-primary);
    flex: 0 0 auto;
  }

  .p-info {
    display: flex;
    min-width: 0;
    flex: 1;
    flex-direction: column;
    gap: 0.2rem;
  }

  .p-name {
    font-weight: 650;
  }

  .p-desc {
    color: var(--text-secondary);
    font-size: 0.8rem;
    line-height: 1.35;
  }

  .config-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1rem;
  }

  .setting-item {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .setting-item.full-width {
    grid-column: 1 / -1;
  }

  .setting-item label {
    color: var(--text-primary);
    font-size: 0.9rem;
    font-weight: 600;
  }

  .form-input,
  .form-textarea {
    width: 100%;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.95rem;
    padding: 0.75rem 1rem;
  }

  .form-textarea {
    min-height: 92px;
    resize: vertical;
    font-family: inherit;
  }

  .form-textarea.code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    font-size: 0.85rem;
  }

  .form-input:focus,
  .form-textarea:focus {
    border-color: var(--color-primary);
    outline: none;
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .test-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.75rem;
    margin-top: 1rem;
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
  }

  .status.success {
    color: var(--color-success);
  }

  .status.error {
    color: var(--color-danger);
  }

  code {
    background: var(--bg-app);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    padding: 0.05rem 0.3rem;
  }

  @media (max-width: 760px) {
    .config-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
