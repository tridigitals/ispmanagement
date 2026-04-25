<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { t } from 'svelte-i18n';
  import { toast } from 'svelte-sonner';
  import { api } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import type { BankAccount } from '$lib/api/client';
  import type { DuitkuPaymentMethod } from '$lib/api/types';

  export let paymentMidtransEnabled: boolean;
  export let paymentMidtransMerchantId: string;
  export let paymentMidtransServerKey: string;
  export let paymentMidtransClientKey: string;
  export let paymentMidtransIsProduction: boolean;
  export let paymentDuitkuEnabled: boolean;
  export let paymentDuitkuMerchantCode: string;
  export let paymentDuitkuApiKey: string;
  export let paymentDuitkuPaymentMethods: string;
  export let paymentDuitkuIsProduction: boolean;
  export let paymentManualEnabled: boolean;
  export let paymentManualInstructions: string;
  export let installationSlaReminderEnabled: boolean;
  export let installationSlaOverdueMinutes: number;
  export let installationSlaReminderCooldownMinutes: number;
  export let installationSlaSchedulerIntervalMinutes: number;
  export let bankAccounts: BankAccount[] = [];
  export let newBankName: string = '';
  export let newAccountNumber: string = '';
  export let newAccountHolder: string = '';
  export let addingBank: boolean = false;
  export let isMobile: boolean = false;

  const dispatch = createEventDispatcher();
  let duitkuMethods: DuitkuPaymentMethod[] = [];
  let loadingDuitkuMethods = false;

  function handleChange() {
    dispatch('change');
  }

  function addBank() {
    dispatch('addBank');
  }

  function deleteBank(id: string) {
    dispatch('deleteBank', id);
  }

  function selectedDuitkuMethods() {
    try {
      const parsed = JSON.parse(paymentDuitkuPaymentMethods || '[]');
      return Array.isArray(parsed)
        ? parsed.map((v) => String(v).trim().toUpperCase()).filter(Boolean)
        : [];
    } catch {
      const value = String(paymentDuitkuPaymentMethods || '').trim().toUpperCase();
      return value ? [value] : [];
    }
  }

  function toggleDuitkuMethod(code: string, checked: boolean) {
    const selected = selectedDuitkuMethods();
    paymentDuitkuPaymentMethods = JSON.stringify(
      checked
        ? Array.from(new Set([...selected, code]))
        : selected.filter((item) => item !== code),
    );
    handleChange();
  }

  async function loadDuitkuMethods() {
    loadingDuitkuMethods = true;
    try {
      duitkuMethods = await api.payment.listDuitkuPaymentMethods(10000);
      if (!duitkuMethods.length) toast.info('No Duitku payment methods returned for this merchant.');
    } catch (error: any) {
      toast.error(error?.message || 'Failed to load Duitku payment methods');
    } finally {
      loadingDuitkuMethods = false;
    }
  }
</script>

<div class="card section fade-in">
  <div class="card-header">
    <h3>
      {$t('superadmin.settings.payment.midtrans.title') || 'Midtrans Gateway'}
    </h3>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="midtrans-toggle">
          {$t('superadmin.settings.payment.midtrans.enable.label') || 'Enable Midtrans'}
        </label>
        <p class="setting-description">
          {$t('superadmin.settings.payment.midtrans.enable.desc') ||
            'Use Midtrans for automated payment processing.'}
        </p>
      </div>
      <label class="toggle">
        <input
          id="midtrans-toggle"
          type="checkbox"
          bind:checked={paymentMidtransEnabled}
          on:change={handleChange}
        />
        <span class="slider"></span>
      </label>
    </div>

    {#if paymentMidtransEnabled}
      <div class="sub-settings fade-in">
        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label" for="midtrans-merchant-id">Merchant ID</label>
          </div>
          <input
            type="text"
            id="midtrans-merchant-id"
            bind:value={paymentMidtransMerchantId}
            on:input={handleChange}
            class="form-input"
          />
        </div>
        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label" for="midtrans-client-key">Client Key</label>
          </div>
          <input
            type="text"
            id="midtrans-client-key"
            bind:value={paymentMidtransClientKey}
            on:input={handleChange}
            class="form-input"
          />
        </div>
        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label" for="midtrans-server-key">Server Key</label>
          </div>
          <input
            type="password"
            id="midtrans-server-key"
            bind:value={paymentMidtransServerKey}
            on:input={handleChange}
            class="form-input"
          />
        </div>
        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label" for="midtrans-production">Environment</label>
            <p class="setting-description">
              {paymentMidtransIsProduction
                ? 'Production Mode (Live Payments)'
                : 'Sandbox Mode (Testing)'}
            </p>
          </div>
          <label class="toggle">
            <input
              id="midtrans-production"
              type="checkbox"
              bind:checked={paymentMidtransIsProduction}
              on:change={handleChange}
            />
            <span class="slider"></span>
          </label>
        </div>
      </div>
    {/if}
  </div>
</div>

<div class="card section fade-in" style="margin-top: 1.5rem;">
  <div class="card-header">
    <h3>Duitku Gateway</h3>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="duitku-toggle">Enable Duitku</label>
        <p class="setting-description">Use Duitku hosted checkout for automated payments.</p>
      </div>
      <label class="toggle">
        <input
          id="duitku-toggle"
          type="checkbox"
          bind:checked={paymentDuitkuEnabled}
          on:change={handleChange}
        />
        <span class="slider"></span>
      </label>
    </div>

    {#if paymentDuitkuEnabled}
      <div class="sub-settings fade-in">
        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label" for="duitku-merchant-code">Merchant Code</label>
          </div>
          <input
            type="text"
            id="duitku-merchant-code"
            bind:value={paymentDuitkuMerchantCode}
            on:input={handleChange}
            class="form-input"
            placeholder="D1234"
          />
        </div>
        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label" for="duitku-api-key">API Key</label>
          </div>
          <input
            type="password"
            id="duitku-api-key"
            bind:value={paymentDuitkuApiKey}
            on:input={handleChange}
            class="form-input"
          />
        </div>
        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label" for="duitku-production">Environment</label>
            <p class="setting-description">
              {paymentDuitkuIsProduction
                ? 'Production Mode (Live Payments)'
                : 'Sandbox Mode (Testing)'}
            </p>
          </div>
          <label class="toggle">
            <input
              id="duitku-production"
              type="checkbox"
              bind:checked={paymentDuitkuIsProduction}
              on:change={handleChange}
            />
            <span class="slider"></span>
          </label>
        </div>
        <div class="setting-row vertical">
          <div class="setting-info full-width">
            <div class="method-list-header">
              <div>
                <div class="setting-label">Enabled Payment Methods</div>
                <p class="setting-description">
                  Select which Duitku channels customers can choose during checkout.
                </p>
              </div>
              <button
                class="btn btn-secondary"
                type="button"
                on:click={loadDuitkuMethods}
                disabled={loadingDuitkuMethods}
              >
                <Icon name="refresh-cw" size={14} />
                {loadingDuitkuMethods ? 'Loading...' : 'Load Duitku Methods'}
              </button>
            </div>
            {#if duitkuMethods.length}
              <div class="method-checklist">
                {#each duitkuMethods as method}
                  {@const selected = selectedDuitkuMethods().includes(method.code)}
                  <label class="method-check">
                    <input
                      type="checkbox"
                      checked={selected}
                      on:change={(e) => toggleDuitkuMethod(method.code, e.currentTarget.checked)}
                    />
                    <span>
                      <strong>{method.name}</strong>
                      <small>{method.code}{method.fee ? ` - Fee ${method.fee}` : ''}</small>
                    </span>
                  </label>
                {/each}
              </div>
            {:else if selectedDuitkuMethods().length}
              <div class="method-checklist">
                {#each selectedDuitkuMethods() as code}
                  <label class="method-check">
                    <input
                      type="checkbox"
                      checked={true}
                      on:change={(e) => toggleDuitkuMethod(code, e.currentTarget.checked)}
                    />
                    <span>
                      <strong>{code}</strong>
                      <small>Saved method</small>
                    </span>
                  </label>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>

<div class="card section fade-in" style="margin-top: 1.5rem;">
  <div class="card-header">
    <h3>
      {$t('superadmin.settings.payment.manual.title') || 'Manual Bank Transfer'}
    </h3>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="manual-transfer-toggle">
          {$t('superadmin.settings.payment.manual.enable.label') || 'Enable Manual Transfer'}
        </label>
        <p class="setting-description">
          {$t('superadmin.settings.payment.manual.enable.desc') ||
            'Allow users to pay via bank transfer and upload proof.'}
        </p>
      </div>
      <label class="toggle">
        <input
          id="manual-transfer-toggle"
          type="checkbox"
          bind:checked={paymentManualEnabled}
          on:change={handleChange}
        />
        <span class="slider"></span>
      </label>
    </div>

    {#if paymentManualEnabled}
      <div class="sub-settings fade-in">
        <div class="setting-row">
          <div class="setting-info full-width">
            <label class="setting-label" for="manual-instructions">
              {$t('superadmin.settings.payment.manual.instructions_label') ||
                'Payment Instructions'}
            </label>
            <p class="setting-description">
              {$t('superadmin.settings.payment.manual.instructions_desc') ||
                'Instructions shown to user when they select Manual Transfer.'}
            </p>
            <textarea
              id="manual-instructions"
              bind:value={paymentManualInstructions}
              on:input={handleChange}
              class="form-input"
              rows="3"
              placeholder={$t('superadmin.settings.placeholders.manual_instructions') ||
                'Please transfer to one of the bank accounts below and upload proof.'}
            ></textarea>
          </div>
        </div>

        <div class="bank-accounts-list">
          <h4 class="subsection-title">
            {$t('superadmin.settings.payment.manual.bank_accounts_title') || 'Bank Accounts'}
          </h4>
          {#if bankAccounts.length > 0}
            {#if isMobile}
              <div class="bank-cards">
                {#each bankAccounts as bank}
                  <div class="bank-card">
                    <div class="bank-card-top">
                      <div>
                        <div class="bank-name">
                          {bank.bank_name}
                        </div>
                        <div class="bank-sub">
                          {bank.account_holder}
                        </div>
                      </div>
                      <button
                        class="btn-icon danger"
                        type="button"
                        title={$t('superadmin.settings.actions.remove') || 'Remove'}
                        on:click={() => deleteBank(bank.id)}
                      >
                        <Icon name="trash" size={16} />
                      </button>
                    </div>
                    <div class="bank-number mono">
                      {bank.account_number}
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              <table class="simple-table">
                <thead>
                  <tr>
                    <th>{$t('superadmin.settings.bank.table.bank') || 'Bank'}</th>
                    <th>{$t('superadmin.settings.bank.table.number') || 'Number'}</th>
                    <th>{$t('superadmin.settings.bank.table.holder') || 'Holder'}</th>
                    <th>{$t('superadmin.settings.bank.table.action') || 'Action'}</th>
                  </tr>
                </thead>
                <tbody>
                  {#each bankAccounts as bank}
                    <tr>
                      <td>{bank.bank_name}</td>
                      <td>{bank.account_number}</td>
                      <td>{bank.account_holder}</td>
                      <td>
                        <button
                          class="btn-icon danger"
                          type="button"
                          on:click={() => deleteBank(bank.id)}
                        >
                          <Icon name="trash" size={16} />
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          {:else}
            <p class="text-muted">
              {$t('superadmin.settings.bank.empty') || 'No bank accounts added yet.'}
            </p>
          {/if}
        </div>

        <div class="add-bank-form">
          <h4>
            {$t('superadmin.settings.bank.add_new_account') || 'Add New Account'}
          </h4>
          <div class="form-row-inline">
            <input
              type="text"
              bind:value={newBankName}
              placeholder={$t('superadmin.settings.placeholders.bank_name') ||
                'Bank Name (e.g. BCA)'}
              class="form-input"
            />
            <input
              type="text"
              bind:value={newAccountNumber}
              placeholder={$t('superadmin.settings.placeholders.bank_account_number') ||
                'Account Number'}
              class="form-input"
            />
            <input
              type="text"
              bind:value={newAccountHolder}
              placeholder={$t('superadmin.settings.placeholders.bank_account_holder') ||
                'Account Holder'}
              class="form-input"
            />
            <button class="btn btn-primary" on:click={addBank} disabled={addingBank}>
              <Icon name="plus" size={16} />
              {$t('superadmin.settings.actions.add') || 'Add'}
            </button>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>

<div class="card section fade-in" style="margin-top: 1.5rem;">
  <div class="card-header">
    <h3>{$t('superadmin.settings.payment.installation_sla.title') || 'Installation SLA Reminders'}</h3>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div class="setting-info">
        <label class="setting-label" for="installation-sla-enabled">
          {$t('superadmin.settings.payment.installation_sla.enable.label') ||
            'Enable installation SLA reminders'}
        </label>
        <p class="setting-description">
          {$t('superadmin.settings.payment.installation_sla.enable.desc') ||
            'Send automatic notifications when installation work orders are overdue.'}
        </p>
      </div>
      <label class="toggle">
        <input
          id="installation-sla-enabled"
          type="checkbox"
          bind:checked={installationSlaReminderEnabled}
          on:change={handleChange}
        />
        <span class="slider"></span>
      </label>
    </div>

    <div class="sub-settings fade-in">
      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="installation-sla-overdue-minutes">
            {$t('superadmin.settings.payment.installation_sla.overdue_minutes.label') ||
              'Overdue threshold (minutes)'}
          </label>
          <p class="setting-description">
            {$t('superadmin.settings.payment.installation_sla.overdue_minutes.desc') ||
              'A scheduled work order is considered overdue after this many minutes.'}
          </p>
        </div>
        <input
          id="installation-sla-overdue-minutes"
          class="form-input"
          type="number"
          min="15"
          max="10080"
          step="5"
          disabled={!installationSlaReminderEnabled}
          bind:value={installationSlaOverdueMinutes}
          on:input={handleChange}
        />
      </div>

      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="installation-sla-cooldown-minutes">
            {$t('superadmin.settings.payment.installation_sla.cooldown_minutes.label') ||
              'Reminder cooldown (minutes)'}
          </label>
          <p class="setting-description">
            {$t('superadmin.settings.payment.installation_sla.cooldown_minutes.desc') ||
              'Minimum delay before the same reminder is sent again for the same work order.'}
          </p>
        </div>
        <input
          id="installation-sla-cooldown-minutes"
          class="form-input"
          type="number"
          min="15"
          max="10080"
          step="5"
          disabled={!installationSlaReminderEnabled}
          bind:value={installationSlaReminderCooldownMinutes}
          on:input={handleChange}
        />
      </div>

      <div class="setting-row">
        <div class="setting-info">
          <label class="setting-label" for="installation-sla-interval-minutes">
            {$t('superadmin.settings.payment.installation_sla.scheduler_interval_minutes.label') ||
              'Scheduler interval (minutes)'}
          </label>
          <p class="setting-description">
            {$t('superadmin.settings.payment.installation_sla.scheduler_interval_minutes.desc') ||
              'How often the backend checks all tenants for overdue installation work orders.'}
          </p>
        </div>
        <input
          id="installation-sla-interval-minutes"
          class="form-input"
          type="number"
          min="5"
          max="1440"
          step="1"
          disabled={!installationSlaReminderEnabled}
          bind:value={installationSlaSchedulerIntervalMinutes}
          on:input={handleChange}
        />
      </div>
    </div>
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

  textarea.form-input {
    resize: vertical;
    min-height: 80px;
    margin-top: 0.75rem;
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

  .sub-settings {
    padding: 1.5rem;
    background: rgba(0, 0, 0, 0.15);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    margin: 1rem 0;
  }

  .sub-settings .setting-row {
    gap: 2rem;
    padding: 1rem 0;
  }

  .sub-settings .setting-row:first-child {
    padding-top: 0;
  }

  .sub-settings .setting-row:last-child {
    padding-bottom: 0;
  }

  .sub-settings .form-input {
    max-width: 320px;
  }

  .bank-accounts-list {
    margin-top: 2rem;
    border-top: 1px solid var(--border-color);
    padding-top: 1.5rem;
  }

  .subsection-title {
    font-size: 0.9rem;
    font-weight: 600;
    margin-bottom: 1.25rem;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .bank-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem;
  }

  .bank-card {
    background: var(--bg-surface-raised);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
    position: relative;
    transition: transform 0.2s;
  }

  .bank-card:hover {
    transform: translateY(-2px);
  }

  .bank-card-top {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1.25rem;
  }

  .bank-name {
    font-weight: 700;
    color: var(--text-primary);
    font-size: 1.1rem;
  }

  .bank-sub {
    font-size: 0.8rem;
    color: var(--text-secondary);
    margin-top: 0.15rem;
  }

  .bank-number {
    font-size: 1.25rem;
    letter-spacing: 0.1em;
    color: var(--text-primary);
    margin-top: auto;
  }

  .mono {
    font-family: var(--font-mono);
  }

  .simple-table {
    width: 100%;
    border-collapse: collapse;
  }

  .simple-table th,
  .simple-table td {
    padding: 1rem;
    text-align: left;
    border-bottom: 1px solid var(--border-color);
  }

  .simple-table th {
    font-weight: 600;
    color: var(--text-secondary);
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .btn-icon {
    background: none;
    border: none;
    padding: 0.5rem;
    cursor: pointer;
    color: var(--text-secondary);
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
  }

  .btn-icon:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .btn-icon.danger:hover {
    background: rgba(var(--danger-rgb), 0.1);
    color: var(--color-danger);
  }

  .add-bank-form {
    margin-top: 2rem;
    padding: 1.5rem;
    background: rgba(0, 0, 0, 0.15);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
  }

  .add-bank-form h4 {
    margin-bottom: 1.25rem;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .form-row-inline {
    display: flex;
    gap: 1rem;
    align-items: flex-end;
  }

  .form-row-inline .form-input {
    flex: 1;
    max-width: none;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 1.25rem;
    font-weight: 600;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.2s;
    border: none;
    font-size: 0.9rem;
  }

  .btn-primary {
    background: var(--color-primary);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .setting-row.vertical {
    align-items: stretch;
  }

  .method-list-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .method-checklist {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 0.75rem;
  }

  .method-check {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 0.9rem;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    background: rgba(255, 255, 255, 0.03);
    cursor: pointer;
  }

  .method-check input {
    margin-top: 0.2rem;
  }

  .method-check span {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .method-check strong {
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .method-check small {
    color: var(--text-secondary);
    font-size: 0.78rem;
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

  @media (max-width: 768px) {
    .form-row-inline {
      flex-direction: column;
      align-items: stretch;
    }

    .sub-settings .form-input {
      max-width: 100%;
    }
  }
</style>
