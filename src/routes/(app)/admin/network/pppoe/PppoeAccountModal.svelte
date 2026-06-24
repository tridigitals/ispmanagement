<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';

  type Option = { label: string; value: string };
  type AccountSource = 'router' | 'managed_radius';

  let {
    mode,
    show = $bindable(false),
    saving = false,
    routerOptions = [],
    customerOptions = [],
    locationOptions = [],
    packageOptions = [],
    packageSelectionHasMissingMapping = false,
    formRouterId = $bindable(''),
    formCustomerId = $bindable(''),
    formLocationId = $bindable(''),
    formUsername = $bindable(''),
    formPassword = $bindable(''),
    formPackageId = $bindable(''),
    formComment = $bindable(''),
    formDisabled = $bindable(false),
    formAccountSource = $bindable<AccountSource>('router'),
    routerDisplayName = '',
    customerDisplayName = '',
    locationDisplayName = '',
    onRouterChange,
    onCustomerChange,
    onPackageChange,
    onSubmit,
    sourceLabel,
    sourceDisabledHintLabel,
    sourceCreateActionLabel,
  }: {
    mode: 'create' | 'edit';
    show?: boolean;
    saving?: boolean;
    routerOptions?: Option[];
    customerOptions?: Option[];
    locationOptions?: Option[];
    packageOptions?: Option[];
    packageSelectionHasMissingMapping?: boolean;
    formRouterId?: string;
    formCustomerId?: string;
    formLocationId?: string;
    formUsername?: string;
    formPassword?: string;
    formPackageId?: string;
    formComment?: string;
    formDisabled?: boolean;
    formAccountSource?: AccountSource;
    routerDisplayName?: string;
    customerDisplayName?: string;
    locationDisplayName?: string;
    onRouterChange: () => void;
    onCustomerChange: () => void;
    onPackageChange: () => void;
    onSubmit: () => void;
    sourceLabel: (source: AccountSource) => string;
    sourceDisabledHintLabel: (source: AccountSource) => string;
    sourceCreateActionLabel: (source: AccountSource) => string;
  } = $props();

  const isCreate = $derived(mode === 'create');
  const modalTitle = $derived(
    isCreate
      ? ($t('admin.customers.pppoe.new.title') || 'Add PPPoE account')
      : ($t('admin.customers.pppoe.edit.title') || 'Edit PPPoE account'),
  );
  const submitDisabled = $derived.by(() =>
    isCreate
      ? saving ||
        packageSelectionHasMissingMapping ||
        !formRouterId ||
        !formCustomerId ||
        !formLocationId ||
        !formUsername.trim() ||
        !formPassword
      : saving || packageSelectionHasMissingMapping || !formUsername.trim(),
  );
</script>

<Modal bind:show title={modalTitle} width="760px" onclose={() => (show = false)}>
  <div class="form">
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.router') || 'Router'}</span>
        {#if isCreate}
          <Select2
            bind:value={formRouterId}
            options={routerOptions}
            placeholder={($t('common.select') || 'Select') + '...'}
            width="100%"
            searchPlaceholder={$t('common.search') || 'Search'}
            noResultsText={$t('common.no_results') || 'No results'}
            onchange={onRouterChange}
          />
        {:else}
          <input class="input" value={routerDisplayName} disabled />
        {/if}
      </label>
      <label>
        <span>{$t('admin.network.pppoe.fields.source') || 'Account source'}</span>
        <select class="input" bind:value={formAccountSource}>
          <option value="router">{sourceLabel('router')}</option>
          <option value="managed_radius">{sourceLabel('managed_radius')}</option>
        </select>
      </label>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.customer') || 'Customer'}</span>
        <Select2
          bind:value={formCustomerId}
          options={customerOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
          onchange={onCustomerChange}
        />
      </label>
      {#if isCreate}
        <div></div>
      {/if}
    </div>

    {#if isCreate}
      <div class="field-hint">
        {#if formAccountSource === 'managed_radius'}
          {$t('admin.network.pppoe.form.source_radius_hint') ||
            'This account will be provisioned to managed RADIUS and expects a native RADIUS endpoint plus NAS mapping for the selected router.'}
        {:else}
          {$t('admin.network.pppoe.form.source_router_hint') ||
            'This account will be provisioned to the router-local PPP secret table.'}
        {/if}
      </div>
    {/if}

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.location') || 'Location'}</span>
        <Select2
          bind:value={formLocationId}
          options={locationOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!formCustomerId}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
        />
      </label>
      {#if isCreate}
        <label>
          <span>{$t('admin.customers.pppoe.fields.username') || 'Username'}</span>
          <input class="input" bind:value={formUsername} />
        </label>
      {:else}
        <div></div>
      {/if}
    </div>

    <label>
      <span>{$t('admin.customers.pppoe.fields.package') || 'Package'}</span>
      <Select2
        bind:value={formPackageId}
        options={packageOptions}
        placeholder={($t('common.select') || 'Select') + '...'}
        width="100%"
        disabled={isCreate ? !formRouterId || packageOptions.length === 0 : packageOptions.length === 0}
        searchPlaceholder={$t('common.search') || 'Search'}
        noResultsText={$t('common.no_results') || 'No results'}
        onchange={onPackageChange}
      />
      <div class="field-hint">
        {$t('admin.network.pppoe.form.package_hint') ||
          'Choose a package to control PPP profile and addressing for the selected router.'}
      </div>
    </label>

    {#if !isCreate}
      <div class="grid2">
        <label>
          <span>{$t('admin.customers.pppoe.fields.username') || 'Username'}</span>
          <input class="input" bind:value={formUsername} />
        </label>
        <label>
          <span>{$t('admin.customers.pppoe.fields.password') || 'Password'}</span>
          <input
            class="input"
            type="password"
            bind:value={formPassword}
            placeholder={$t('admin.customers.pppoe.edit.password_hint') || 'Leave blank to keep'}
          />
        </label>
      </div>
    {/if}

    {#if isCreate}
      <label>
        <span>{$t('admin.customers.pppoe.fields.password') || 'Password'}</span>
        <input class="input" type="password" bind:value={formPassword} />
      </label>
    {/if}

    {#if packageSelectionHasMissingMapping}
      <div class="field-hint warning">
        {$t('admin.network.pppoe.form.package_mapping_missing') ||
          'This package does not have a router mapping yet. Existing account values will be kept until a mapping is added.'}
      </div>
    {/if}

    <label>
      <span>{$t('admin.customers.pppoe.fields.comment') || 'Comment'}</span>
      <input class="input" bind:value={formComment} />
    </label>

    <div class="toggle-row">
      <div class="toggle-text">
        <div class="toggle-title">{$t('admin.customers.pppoe.fields.disabled') || 'Disabled'}</div>
        <div class="toggle-sub">
          {sourceDisabledHintLabel(formAccountSource)}
          {#if isCreate && formAccountSource === 'managed_radius'}
            {' '}{$t('admin.network.pppoe.form.disabled_hint_radius') ||
              'For managed RADIUS, this disables centralized authentication for the account.'}
          {/if}
        </div>
      </div>
      <Toggle
        bind:checked={formDisabled}
        ariaLabel={$t('admin.customers.pppoe.fields.disabled') || 'Disabled'}
      />
    </div>

    <div class="actions">
      <button class="btn ghost" onclick={() => (show = false)} disabled={saving}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn" onclick={onSubmit} disabled={submitDisabled}>
        <Icon name={isCreate ? 'plus' : 'check-circle'} size={16} />
        {#if isCreate}
          {sourceCreateActionLabel(formAccountSource)}
        {:else}
          {$t('common.save') || 'Save'}
        {/if}
      </button>
    </div>
</div>
</Modal>

<style>
  .form {
    display: grid;
    gap: 0.9rem;
  }

  .form label {
    display: grid;
    gap: 0.35rem;
  }

  .form label > span {
    color: var(--text-secondary);
    font-weight: 850;
    font-size: 0.78rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .input {
    width: 100%;
    padding: 0.85rem 0.95rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-primary);
    outline: none;
    font-weight: 650;
  }

  :global([data-theme='light']) .input {
    background: rgba(0, 0, 0, 0.03);
  }

  .grid2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.8rem;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.9rem;
    padding: 0.9rem 1rem;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
  }

  :global([data-theme='light']) .toggle-row {
    background: rgba(0, 0, 0, 0.02);
  }

  .toggle-text {
    min-width: 0;
    display: grid;
    gap: 0.15rem;
  }

  .toggle-title {
    color: var(--text-primary);
    font-weight: 900;
  }

  .toggle-sub {
    color: var(--text-secondary);
    font-weight: 650;
    font-size: 0.92rem;
    line-height: 1.35;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
    padding-top: 0.25rem;
  }

  .field-hint {
    margin-top: 6px;
    color: var(--text-secondary);
    font-weight: 600;
    font-size: 0.9rem;
    line-height: 1.35;
  }

  .field-hint.warning {
    margin-top: 0;
    color: #f59e0b;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--color-primary);
    color: white;
    font-weight: 800;
    cursor: pointer;
    text-decoration: none;
  }

  .btn.ghost {
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-primary);
  }

  :global([data-theme='light']) .btn.ghost {
    background: rgba(0, 0, 0, 0.03);
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  @media (max-width: 768px) {
    .grid2 {
      grid-template-columns: 1fr;
    }
  }
</style>
