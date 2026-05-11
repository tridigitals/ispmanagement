<script lang="ts">
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import { toast } from 'svelte-sonner';
  import { api } from '$lib/api/client';
  import type { DuitkuPaymentMethod } from '$lib/api/types';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Input from '$lib/components/ui/Input.svelte';

  export let localSettings: Record<string, string>;
  export let bankAccounts: any[] = [];
  export let newBank: { bank_name: string; account_number: string; account_holder: string };
  export let showAddBank = false;
  export let handleChange: (key: string, value: any) => void;
  export let addBankAccount: () => void;
  export let removeBankAccount: (id: string) => void;

  let duitkuMethods: DuitkuPaymentMethod[] = [];
  let loadingDuitkuMethods = false;

  function tt(key: string, fallback: string) {
    const value = get(t)(key);
    return value && value !== key ? value : fallback;
  }

  function selectedDuitkuMethods() {
    const raw =
      localSettings['payment_duitku_payment_methods'] ||
      (localSettings['payment_duitku_payment_method']
        ? JSON.stringify([localSettings['payment_duitku_payment_method']])
        : '[]');
    try {
      const parsed = JSON.parse(raw);
      return Array.isArray(parsed)
        ? parsed.map((v) => String(v).trim().toUpperCase()).filter(Boolean)
        : [];
    } catch {
      const value = String(raw).trim().toUpperCase();
      return value ? [value] : [];
    }
  }

  function toggleDuitkuMethod(code: string, checked: boolean) {
    const selected = selectedDuitkuMethods();
    const next = checked
      ? Array.from(new Set([...selected, code]))
      : selected.filter((item) => item !== code);
    handleChange('payment_duitku_payment_methods', JSON.stringify(next));
  }

  async function loadDuitkuMethods() {
    loadingDuitkuMethods = true;
    try {
      duitkuMethods = await api.payment.listDuitkuPaymentMethods(10000);
      if (!duitkuMethods.length) {
        toast.info('No Duitku payment methods returned for this merchant.');
      }
    } catch (error: any) {
      toast.error(error?.message || 'Failed to load Duitku payment methods');
    } finally {
      loadingDuitkuMethods = false;
    }
  }
</script>

<div class="payment-settings">
  <span class="section-label">{tt('admin.settings.payment.methods_label', 'Payment Methods')}</span>

  <div class="method-card">
    <div class="method-header">
      <div class="m-icon midtrans">M</div>
      <div class="m-info">
        <h4>{tt('admin.settings.sections.midtrans', 'Midtrans Payment Gateway')}</h4>
        <p>Accept payments via Credit Card, GoPay, ShopeePay, VA, etc.</p>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          checked={localSettings['payment_midtrans_enabled'] === 'true'}
          onchange={(e) => handleChange('payment_midtrans_enabled', e.currentTarget.checked)}
        />
        <span class="slider"></span>
      </label>
    </div>

    {#if localSettings['payment_midtrans_enabled'] === 'true'}
      <div class="method-config fade-in">
        <div class="config-grid">
          <div class="setting-item">
            <label for="midtrans-merchant-id">
              {tt('admin.settings.payment.midtrans.merchant_id', 'Merchant ID')}
            </label>
            <Input
              id="midtrans-merchant-id"
              value={localSettings['payment_midtrans_merchant_id']}
              oninput={(e: any) => handleChange('payment_midtrans_merchant_id', e.target.value)}
              placeholder="G123456789"
            />
          </div>
          <div class="setting-item">
            <label for="midtrans-client-key">
              {tt('admin.settings.payment.midtrans.client_key', 'Client Key')}
            </label>
            <Input
              id="midtrans-client-key"
              value={localSettings['payment_midtrans_client_key']}
              oninput={(e: any) => handleChange('payment_midtrans_client_key', e.target.value)}
              placeholder="SB-Mid-client-..."
            />
          </div>
          <div class="setting-item full-width">
            <label for="midtrans-server-key">
              {tt('admin.settings.payment.midtrans.server_key', 'Server Key')}
            </label>
            <Input
              id="midtrans-server-key"
              type="password"
              value={localSettings['payment_midtrans_server_key']}
              oninput={(e: any) => handleChange('payment_midtrans_server_key', e.target.value)}
              placeholder="SB-Mid-server-..."
              showPasswordToggle={true}
            />
          </div>
          <div class="setting-item full-width checkbox-row">
            <label class="checkbox-label">
              <input
                type="checkbox"
                checked={localSettings['payment_midtrans_is_production'] === 'true'}
                onchange={(e: any) =>
                  handleChange('payment_midtrans_is_production', e.currentTarget.checked)}
              />
              <span>Enable Production Mode (Live)</span>
            </label>
          </div>
        </div>
      </div>
    {/if}
  </div>

  <div class="method-card mt-6">
    <div class="method-header">
      <div class="m-icon duitku">D</div>
      <div class="m-info">
        <h4>Duitku Payment Gateway</h4>
        <p>Accept payments via Duitku checkout redirect.</p>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          checked={localSettings['payment_duitku_enabled'] === 'true'}
          onchange={(e) => handleChange('payment_duitku_enabled', e.currentTarget.checked)}
        />
        <span class="slider"></span>
      </label>
    </div>

    {#if localSettings['payment_duitku_enabled'] === 'true'}
      <div class="method-config fade-in">
        <div class="config-grid">
          <div class="setting-item">
            <label for="duitku-merchant-code">Merchant Code</label>
            <Input
              id="duitku-merchant-code"
              value={localSettings['payment_duitku_merchant_code']}
              oninput={(e: any) => handleChange('payment_duitku_merchant_code', e.target.value)}
              placeholder="D1234"
            />
          </div>
          <div class="setting-item full-width">
            <label for="duitku-api-key">API Key</label>
            <Input
              id="duitku-api-key"
              type="password"
              value={localSettings['payment_duitku_api_key']}
              oninput={(e: any) => handleChange('payment_duitku_api_key', e.target.value)}
              placeholder="Duitku API key"
              showPasswordToggle={true}
            />
          </div>
          <div class="setting-item full-width checkbox-row">
            <label class="checkbox-label">
              <input
                type="checkbox"
                checked={localSettings['payment_duitku_is_production'] === 'true'}
                onchange={(e: any) =>
                  handleChange('payment_duitku_is_production', e.currentTarget.checked)}
              />
              <span>Enable Production Mode (Live)</span>
            </label>
          </div>
          <div class="setting-item full-width">
            <div class="bm-header">
              <span class="label-text">Enabled Payment Methods</span>
              <button
                class="btn btn-secondary btn-sm"
                type="button"
                onclick={loadDuitkuMethods}
                disabled={loadingDuitkuMethods}
              >
                <Icon name="refresh-cw" size={14} />
                {loadingDuitkuMethods ? 'Loading...' : 'Load Duitku Methods'}
              </button>
            </div>
            <p class="help-text">
              Select which Duitku channels customers can choose during checkout.
            </p>
            {#if duitkuMethods.length}
              <div class="method-checklist">
                {#each duitkuMethods as method}
                  {@const selected = selectedDuitkuMethods().includes(method.code)}
                  <label class="method-check">
                    <input
                      type="checkbox"
                      checked={selected}
                      onchange={(e) => toggleDuitkuMethod(method.code, e.currentTarget.checked)}
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
                      onchange={(e) => toggleDuitkuMethod(code, e.currentTarget.checked)}
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

  <div class="method-card mt-6">
    <div class="method-header">
      <div class="m-icon manual">
        <Icon name="landmark" size={24} />
      </div>
      <div class="m-info">
        <h4>{tt('admin.settings.sections.bank_transfer_manual', 'Bank Transfer (Manual)')}</h4>
        <p>Accept payments via direct bank transfer verification.</p>
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          checked={localSettings['payment_manual_enabled'] === 'true'}
          onchange={(e) => handleChange('payment_manual_enabled', e.currentTarget.checked)}
        />
        <span class="slider"></span>
      </label>
    </div>

    {#if localSettings['payment_manual_enabled'] === 'true'}
      <div class="method-config fade-in">
        <div class="setting-item full-width">
          <label for="payment-manual-instructions">
            {tt('admin.settings.payment.manual.instructions_label', 'Payment Instructions')}
          </label>
          <textarea
            id="payment-manual-instructions"
            class="form-textarea"
            rows="4"
            value={localSettings['payment_manual_instructions']}
            oninput={(e: any) => handleChange('payment_manual_instructions', e.target.value)}
            placeholder={tt(
              'admin.settings.payment.manual.placeholder_instructions',
              'Please transfer to BCA 1234567890 a/n PT Company...',
            )}
          ></textarea>
          <p class="help-text">These instructions will be shown to the user during checkout.</p>
        </div>

        <div class="bank-accounts-manager mt-6">
          <div class="bm-header">
            <span class="label-text">
              {tt('admin.settings.payment.manual.bank_accounts', 'Bank Accounts')}
            </span>
            <button class="btn btn-primary btn-sm" onclick={() => (showAddBank = !showAddBank)}>
              <Icon name={showAddBank ? 'minus' : 'plus'} size={14} />
              {showAddBank ? tt('common.cancel', 'Cancel') : tt('admin.settings.payment.manual.add_bank', 'Add Bank')}
            </button>
          </div>

          {#if showAddBank}
            <div class="add-bank-form fade-in">
              <div class="form-row">
                <Input
                  aria-label={tt(
                    'admin.settings.payment.manual.bank_form.bank_name_label',
                    'Bank Name',
                  )}
                  value={newBank.bank_name}
                  oninput={(e: any) => (newBank.bank_name = e.target.value)}
                  placeholder={tt(
                    'admin.settings.payment.manual.bank_form.bank_name_placeholder',
                    'Bank Name (e.g. BCA)',
                  )}
                />
                <Input
                  aria-label={tt(
                    'admin.settings.payment.manual.bank_form.account_number_label',
                    'Account Number',
                  )}
                  value={newBank.account_number}
                  oninput={(e: any) => (newBank.account_number = e.target.value)}
                  placeholder={tt(
                    'admin.settings.payment.manual.bank_form.account_number_placeholder',
                    'Account Number',
                  )}
                />
              </div>
              <div class="form-row">
                <Input
                  aria-label={tt(
                    'admin.settings.payment.manual.bank_form.account_holder_label',
                    'Account Holder Name',
                  )}
                  value={newBank.account_holder}
                  oninput={(e: any) => (newBank.account_holder = e.target.value)}
                  placeholder={tt(
                    'admin.settings.payment.manual.bank_form.account_holder_placeholder',
                    'Account Holder Name',
                  )}
                />
                <button class="btn btn-secondary" onclick={addBankAccount}>
                  {tt('admin.settings.payment.manual.bank_form.add', 'Add')}
                </button>
              </div>
            </div>
          {/if}

          <div class="bank-list-grid">
            {#if bankAccounts.length === 0}
              <div class="empty-state">
                <div class="icon-placeholder">
                  <Icon name="landmark" size={24} />
                </div>
                <p>No bank accounts added yet.</p>
                <button class="btn btn-primary btn-sm mt-2" onclick={() => (showAddBank = true)}>
                  {tt('admin.settings.payment.manual.add_one', 'Add One')}
                </button>
              </div>
            {:else}
              {#each bankAccounts as bank}
                <div class="bank-card-item">
                  <div class="bc-icon">
                    <Icon name="landmark" size={20} />
                  </div>
                  <div class="bc-details">
                    <span class="bc-name">{bank.bank_name}</span>
                    <span class="bc-number">{bank.account_number}</span>
                    <span class="bc-holder">{bank.account_holder}</span>
                  </div>
                  <div class="bc-actions">
                    <button
                      class="btn-icon delete"
                      onclick={() => removeBankAccount(bank.id)}
                      title={tt(
                        'admin.settings.payment.manual.bank_form.remove_account',
                        'Remove Account',
                      )}
                    >
                      <Icon name="trash" size={16} />
                    </button>
                  </div>
                </div>
              {/each}

              <button class="add-bank-card" onclick={() => (showAddBank = true)}>
                <Icon name="plus" size={24} />
                <span>{tt('admin.settings.payment.manual.bank_form.add_account', 'Add Account')}</span>
              </button>
            {/if}
          </div>
        </div>
      </div>
    {/if}
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

  .mt-2 {
    margin-top: 0.5rem;
  }

  .mt-6 {
    margin-top: 1.5rem;
  }

  .full-width {
    grid-column: 1 / -1;
  }

  .help-text {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-top: 0.25rem;
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

  .method-card {
    background: var(--bg-surface);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  :global([data-theme='light']) .method-card {
    background: rgba(255, 255, 255, 0.75);
  }

  .method-header {
    padding: 1.1rem 1.25rem;
    display: flex;
    align-items: center;
    gap: 1rem;
    border-bottom: 1px solid var(--glass-border);
    background: color-mix(in srgb, var(--bg-surface) 92%, transparent);
  }

  .m-icon {
    width: 42px;
    height: 42px;
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }

  .m-icon.midtrans {
    background: var(--bg-surface);
    border-color: rgba(0, 44, 95, 0.45);
    color: white;
  }

  .m-icon.duitku {
    background: var(--bg-surface);
    border-color: rgba(15, 118, 110, 0.45);
    color: white;
    font-weight: 800;
  }

  .m-icon.manual {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }

  .m-info {
    flex: 1;
    min-width: 180px;
  }

  .m-info h4 {
    margin: 0;
    font-size: 1rem;
    color: var(--text-primary);
    font-weight: 800;
  }

  .m-info p {
    margin: 0.25rem 0 0;
    font-size: 0.88rem;
    color: var(--text-secondary);
  }

  .method-config {
    padding: 1.25rem;
    background: var(--bg-surface);
  }

  :global([data-theme='light']) .method-config {
    background: rgba(0, 0, 0, 0.015);
  }

  .config-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.25rem;
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    margin-top: 0.5rem;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    cursor: pointer;
    font-size: 0.9rem;
    color: var(--text-primary);
    font-weight: 600;
  }

  .form-textarea {
    width: 100%;
    padding: 0.75rem 0.9rem;
    border-radius: 14px;
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.02);
    color: var(--text-primary);
    font-family: inherit;
    font-size: 0.92rem;
    resize: vertical;
    transition:
      border-color 0.2s,
      box-shadow 0.2s;
  }

  :global([data-theme='light']) .form-textarea {
    background: rgba(255, 255, 255, 0.75);
  }

  .form-textarea:focus {
    outline: none;
    border-color: rgba(99, 102, 241, 0.35);
    box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.14);
  }

  .bank-accounts-manager {
    margin-top: 1.5rem;
    border-top: 1px dashed var(--glass-border);
    padding-top: 1.25rem;
  }

  .bm-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }

  .label-text {
    font-weight: 800;
    color: var(--text-primary);
    letter-spacing: 0.01em;
  }

  .add-bank-form {
    display: grid;
    gap: 0.75rem;
    padding: 1rem;
    border-radius: var(--radius-lg);
    border: 1px solid var(--glass-border);
    background: var(--bg-surface);
    margin-bottom: 1rem;
  }

  :global([data-theme='light']) .add-bank-form {
    background: rgba(255, 255, 255, 0.75);
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
    align-items: end;
  }

  .form-row :global(.btn) {
    width: 100%;
  }

  .bank-list-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 1rem;
  }

  .bank-card-item {
    background: var(--bg-surface);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    padding: 1.1rem 1.15rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    position: relative;
    transition: all 0.2s;
  }

  :global([data-theme='light']) .bank-card-item {
    background: rgba(255, 255, 255, 0.75);
  }

  .bank-card-item:hover {
    border-color: rgba(99, 102, 241, 0.25);
    transform: translateY(-1px);
    box-shadow: var(--shadow-sm);
  }

  :global([data-theme='light']) .bank-card-item:hover {
    box-shadow: var(--shadow-sm);
  }

  .bc-icon {
    width: 38px;
    height: 38px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    border: 1px solid var(--glass-border);
  }

  .bc-details {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .bc-name {
    font-weight: 850;
    color: var(--text-primary);
    font-size: 0.98rem;
  }

  .bc-number {
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
    font-size: 1rem;
    letter-spacing: 0.05em;
    color: var(--text-primary);
  }

  .bc-holder {
    font-size: 0.78rem;
    color: var(--text-secondary);
    text-transform: uppercase;
  }

  .bc-actions {
    position: absolute;
    top: 0.9rem;
    right: 0.9rem;
  }

  .btn-icon {
    width: 34px;
    height: 34px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 12px;
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-icon:hover {
    background: rgba(99, 102, 241, 0.1);
    color: var(--text-primary);
    border-color: rgba(99, 102, 241, 0.3);
  }

  .btn-icon.delete:hover {
    background: rgba(239, 68, 68, 0.12);
    color: #ef4444;
    border-color: rgba(239, 68, 68, 0.25);
  }

  .add-bank-card {
    border: 2px dashed var(--glass-border);
    background: rgba(255, 255, 255, 0.01);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.6rem;
    min-height: 150px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
  }

  .add-bank-card:hover {
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--text-primary);
    background: rgba(99, 102, 241, 0.06);
    transform: translateY(-1px);
  }

  .add-bank-card span {
    font-weight: 750;
    font-size: 0.92rem;
  }

  .method-checklist {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 0.75rem;
    margin-top: 0.9rem;
  }

  .method-check {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 0.9rem;
    border: 1px solid var(--glass-border);
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.025);
    cursor: pointer;
  }

  .method-check input {
    margin-top: 0.2rem;
  }

  .method-check span {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
  }

  .method-check strong {
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .method-check small {
    color: var(--text-secondary);
    font-size: 0.78rem;
  }

  .empty-state {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 2rem;
    background: rgba(255, 255, 255, 0.02);
    border-radius: var(--radius-lg);
    border: 1px solid var(--glass-border);
    color: var(--text-secondary);
  }

  :global([data-theme='light']) .empty-state {
    background: rgba(255, 255, 255, 0.75);
  }

  .icon-placeholder {
    width: 48px;
    height: 48px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 1rem;
    color: var(--text-secondary);
    border: 1px solid var(--glass-border);
  }

  @media (max-width: 640px) {
    .config-grid {
      grid-template-columns: 1fr;
    }

    .form-row {
      grid-template-columns: 1fr;
    }
  }
</style>
