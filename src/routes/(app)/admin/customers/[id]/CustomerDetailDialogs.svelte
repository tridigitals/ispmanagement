<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';

  let {
    t,
    showEditPppoe = $bindable(),
    pppoeRouterId = $bindable(),
    pppoeRouters = [],
    loadingPppoeRouters = false,
    pppoePackageId = $bindable(),
    pppoePackageOptions = [],
    onPppoeRouterChange,
    onPppoePackageChange,
    pppoePackageSelectionHasMissingMapping = false,
    pppoeUsername = $bindable(),
    pppoePassword = $bindable(),
    pppoeComment = $bindable(),
    pppoeDisabled = $bindable(),
    savingPppoe = false,
    onSubmitUpdatePppoe,
    showAddSubscription = $bindable(),
    subLocationId = $bindable(),
    subscriptionLocationOptions = [],
    subPackageId = $bindable(),
    subscriptionPackageOptions = [],
    subRouterId = $bindable(),
    subscriptionRouterOptions = [],
    subBillingCycle = $bindable(),
    billingCycleOptions = [],
    subPrice = $bindable(),
    subCurrency = $bindable(),
    subStatus = $bindable(),
    subscriptionStatusOptions = [],
    subStartsAt = $bindable(),
    subEndsAt = $bindable(),
    subNotes = $bindable(),
    savingSubscription = false,
    onSubmitCreateSubscription,
    showEditSubscription = $bindable(),
    onCloseEditSubscription,
    onSubmitUpdateSubscription,
    showAddLocation = $bindable(),
    locLabel = $bindable(),
    locAddress1 = $bindable(),
    locAddress2 = $bindable(),
    locCity = $bindable(),
    locState = $bindable(),
    locPostal = $bindable(),
    locCountry = $bindable(),
    locLatitude = $bindable(),
    locLongitude = $bindable(),
    locNotes = $bindable(),
    creatingLocation = false,
    onAddLocation,
    showEditLocation = $bindable(),
    updatingLocation = false,
    onSubmitUpdateLocation,
    showDeleteCustomer = $bindable(),
    deletingCustomer = false,
    onDeleteCustomer,
    showDeleteLocation = $bindable(),
    deletingLocation = false,
    onDeleteLocation,
  } = $props();
</script>

<Modal
  show={showEditPppoe}
  title={$t('admin.customers.pppoe.edit.title') || 'Edit PPPoE account'}
  onclose={() => (showEditPppoe = false)}
>
  <div class="form">
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.router') || 'Router'}</span>
        <Select2
          bind:value={pppoeRouterId}
          options={pppoeRouters.map((r: any) => ({ label: r.name, value: r.id }))}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={loadingPppoeRouters}
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
          onchange={onPppoeRouterChange}
        />
      </label>
      <div></div>
    </div>

    <label>
      <span>{$t('admin.customers.pppoe.fields.package') || 'Package'}</span>
      <Select2
        bind:value={pppoePackageId}
        options={pppoePackageOptions}
        placeholder={($t('common.select') || 'Select') + '...'}
        width="100%"
        disabled={!pppoeRouterId || pppoePackageOptions.length === 0}
        maxItems={5000}
        searchPlaceholder={$t('common.search') || 'Search'}
        noResultsText={$t('common.no_results') || 'No results'}
        onchange={onPppoePackageChange}
      />
      <div class="field-hint">
        {$t('admin.network.pppoe.form.package_hint') ||
          'Choose a package to control PPP profile and addressing for the selected router.'}
      </div>
    </label>

    {#if pppoePackageSelectionHasMissingMapping}
      <div class="field-hint warning">
        {$t('admin.network.pppoe.form.package_mapping_missing') ||
          'This package does not have a router mapping yet. Existing account values will be kept until a mapping is added.'}
      </div>
    {/if}

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.username') || 'Username'}</span>
        <input class="input" bind:value={pppoeUsername} />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.password') || 'Password'}</span>
        <input
          class="input"
          type="password"
          bind:value={pppoePassword}
          placeholder={$t('admin.customers.pppoe.edit.password_hint') || 'Leave blank to keep'}
        />
      </label>
    </div>

    <label>
      <span>{$t('admin.customers.pppoe.fields.comment') || 'Comment'}</span>
      <input class="input" bind:value={pppoeComment} />
    </label>

    <div class="toggle-row">
      <div class="toggle-text">
        <div class="toggle-title">{$t('admin.customers.pppoe.fields.disabled') || 'Disabled'}</div>
        <div class="toggle-sub">
          {$t('admin.network.pppoe.form.disabled_hint') ||
            'Disable this PPPoE account (will be applied to router when you click Apply).'}
        </div>
      </div>
      <Toggle bind:checked={pppoeDisabled} ariaLabel={$t('admin.customers.pppoe.fields.disabled') || 'Disabled'} />
    </div>

    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showEditPppoe = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn btn-primary"
        onclick={onSubmitUpdatePppoe}
        disabled={savingPppoe || pppoePackageSelectionHasMissingMapping || !pppoeUsername.trim()}
      >
        <Icon name="check-circle" size={16} />
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </div>
</Modal>

<Modal
  show={showAddSubscription}
  title={$t('admin.customers.subscriptions.new.title') || 'Add subscription'}
  onclose={() => (showAddSubscription = false)}
>
  <div class="form">
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.location') || 'Location'}</span>
        <Select2
          bind:value={subLocationId}
          options={subscriptionLocationOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
        />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.package') || 'Package'}</span>
        <Select2
          bind:value={subPackageId}
          options={subscriptionPackageOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
        />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.router') || 'Router (optional)'}</span>
        <Select2
          bind:value={subRouterId}
          options={subscriptionRouterOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
        />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.billing_cycle') || 'Billing cycle'}</span>
        <Select2 bind:value={subBillingCycle} options={billingCycleOptions} width="100%" />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.price') || 'Price'}</span>
        <input class="input" type="number" min="0" step="0.01" bind:value={subPrice} />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.currency') || 'Currency'}</span>
        <input class="input" bind:value={subCurrency} placeholder="IDR" />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.status') || 'Status'}</span>
        <Select2 bind:value={subStatus} options={subscriptionStatusOptions} width="100%" />
      </label>
      <div></div>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.starts_at') || 'Starts at'}</span>
        <input class="input" type="date" bind:value={subStartsAt} />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.ends_at') || 'Ends at'}</span>
        <input class="input" type="date" bind:value={subEndsAt} />
      </label>
    </div>
    <label>
      <span>{$t('admin.customers.subscriptions.fields.notes') || 'Notes'}</span>
      <textarea class="input" rows="3" bind:value={subNotes}></textarea>
    </label>
    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showAddSubscription = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn btn-primary"
        onclick={onSubmitCreateSubscription}
        disabled={savingSubscription || !subLocationId || !subPackageId || !subPrice}
      >
        <Icon name="plus" size={16} />
        {$t('common.create') || 'Create'}
      </button>
    </div>
  </div>
</Modal>

<Modal
  show={showEditSubscription}
  title={$t('admin.customers.subscriptions.edit.title') || 'Edit subscription'}
  onclose={onCloseEditSubscription}
>
  <div class="form">
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.location') || 'Location'}</span>
        <Select2
          bind:value={subLocationId}
          options={subscriptionLocationOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
        />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.package') || 'Package'}</span>
        <Select2
          bind:value={subPackageId}
          options={subscriptionPackageOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
        />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.router') || 'Router (optional)'}</span>
        <Select2
          bind:value={subRouterId}
          options={subscriptionRouterOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
        />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.billing_cycle') || 'Billing cycle'}</span>
        <Select2 bind:value={subBillingCycle} options={billingCycleOptions} width="100%" />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.price') || 'Price'}</span>
        <input class="input" type="number" min="0" step="0.01" bind:value={subPrice} />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.currency') || 'Currency'}</span>
        <input class="input" bind:value={subCurrency} placeholder="IDR" />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.status') || 'Status'}</span>
        <Select2 bind:value={subStatus} options={subscriptionStatusOptions} width="100%" />
      </label>
      <div></div>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.subscriptions.fields.starts_at') || 'Starts at'}</span>
        <input class="input" type="date" bind:value={subStartsAt} />
      </label>
      <label>
        <span>{$t('admin.customers.subscriptions.fields.ends_at') || 'Ends at'}</span>
        <input class="input" type="date" bind:value={subEndsAt} />
      </label>
    </div>
    <label>
      <span>{$t('admin.customers.subscriptions.fields.notes') || 'Notes'}</span>
      <textarea class="input" rows="3" bind:value={subNotes}></textarea>
    </label>
    <div class="actions">
      <button class="btn btn-secondary" onclick={onCloseEditSubscription}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn btn-primary"
        onclick={onSubmitUpdateSubscription}
        disabled={savingSubscription || !subLocationId || !subPackageId || !subPrice}
      >
        <Icon name="check-circle" size={16} />
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </div>
</Modal>

<Modal
  show={showAddLocation}
  title={$t('admin.customers.locations.new.title') || 'Add location'}
  onclose={() => (showAddLocation = false)}
>
  <div class="form">
    <label>
      <span>{$t('admin.customers.locations.fields.label') || 'Label'}</span>
      <input class="input" bind:value={locLabel} placeholder="Site A / Rumah / Kantor" />
    </label>
    <label>
      <span>{$t('admin.customers.locations.fields.address1') || 'Address line 1'}</span>
      <input class="input" bind:value={locAddress1} />
    </label>
    <label>
      <span>{$t('admin.customers.locations.fields.address2') || 'Address line 2'}</span>
      <input class="input" bind:value={locAddress2} />
    </label>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.locations.fields.city') || 'City'}</span>
        <input class="input" bind:value={locCity} />
      </label>
      <label>
        <span>{$t('admin.customers.locations.fields.state') || 'State'}</span>
        <input class="input" bind:value={locState} />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.locations.fields.postal') || 'Postal code'}</span>
        <input class="input" bind:value={locPostal} />
      </label>
      <label>
        <span>{$t('admin.customers.locations.fields.country') || 'Country'}</span>
        <input class="input" bind:value={locCountry} />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.locations.fields.latitude') || 'Latitude'}</span>
        <input class="input mono" bind:value={locLatitude} placeholder="-7.275233" />
      </label>
      <label>
        <span>{$t('admin.customers.locations.fields.longitude') || 'Longitude'}</span>
        <input class="input mono" bind:value={locLongitude} placeholder="110.355211" />
      </label>
    </div>
    <label>
      <span>{$t('admin.customers.locations.fields.notes') || 'Notes'}</span>
      <textarea class="input" rows="3" bind:value={locNotes}></textarea>
    </label>
    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showAddLocation = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn btn-primary" onclick={onAddLocation} disabled={creatingLocation || !locLabel.trim()}>
        <Icon name="plus" size={16} />
        {$t('common.add') || 'Add'}
      </button>
    </div>
  </div>
</Modal>

<Modal
  show={showEditLocation}
  title={$t('admin.customers.locations.edit.title') || 'Edit location'}
  onclose={() => (showEditLocation = false)}
>
  <div class="form">
    <label>
      <span>{$t('admin.customers.locations.fields.label') || 'Label'}</span>
      <input class="input" bind:value={locLabel} placeholder="Site A / Rumah / Kantor" />
    </label>
    <label>
      <span>{$t('admin.customers.locations.fields.address1') || 'Address line 1'}</span>
      <input class="input" bind:value={locAddress1} />
    </label>
    <label>
      <span>{$t('admin.customers.locations.fields.address2') || 'Address line 2'}</span>
      <input class="input" bind:value={locAddress2} />
    </label>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.locations.fields.city') || 'City'}</span>
        <input class="input" bind:value={locCity} />
      </label>
      <label>
        <span>{$t('admin.customers.locations.fields.state') || 'State'}</span>
        <input class="input" bind:value={locState} />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.locations.fields.postal') || 'Postal code'}</span>
        <input class="input" bind:value={locPostal} />
      </label>
      <label>
        <span>{$t('admin.customers.locations.fields.country') || 'Country'}</span>
        <input class="input" bind:value={locCountry} />
      </label>
    </div>
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.locations.fields.latitude') || 'Latitude'}</span>
        <input class="input mono" bind:value={locLatitude} placeholder="-7.275233" />
      </label>
      <label>
        <span>{$t('admin.customers.locations.fields.longitude') || 'Longitude'}</span>
        <input class="input mono" bind:value={locLongitude} placeholder="110.355211" />
      </label>
    </div>
    <label>
      <span>{$t('admin.customers.locations.fields.notes') || 'Notes'}</span>
      <textarea class="input" rows="3" bind:value={locNotes}></textarea>
    </label>
    <div class="actions">
      <button class="btn btn-secondary" onclick={() => (showEditLocation = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn btn-primary"
        onclick={onSubmitUpdateLocation}
        disabled={updatingLocation || !locLabel.trim()}
      >
        <Icon name="check-circle" size={16} />
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </div>
</Modal>

<ConfirmDialog
  show={showDeleteCustomer}
  title={$t('admin.customers.delete.title') || 'Delete customer'}
  message={$t('admin.customers.delete.message') || 'This will remove the customer and all related data.'}
  confirmText={$t('common.delete') || 'Delete'}
  cancelText={$t('common.cancel') || 'Cancel'}
  loading={deletingCustomer}
  onconfirm={onDeleteCustomer}
  oncancel={() => (showDeleteCustomer = false)}
/>

<ConfirmDialog
  show={showDeleteLocation}
  title={$t('admin.customers.locations.delete.title') || 'Delete location'}
  message={$t('admin.customers.locations.delete.message') || 'This location will be removed.'}
  confirmText={$t('common.delete') || 'Delete'}
  cancelText={$t('common.cancel') || 'Cancel'}
  loading={deletingLocation}
  onconfirm={onDeleteLocation}
  oncancel={() => (showDeleteLocation = false)}
/>

<style>
  .form {
    display: grid;
    gap: 0.9rem;
  }

  .grid2 {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
  }

  label > span {
    display: block;
    margin-bottom: 0.35rem;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .input {
    width: 100%;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    padding: 0.65rem 0.75rem;
    outline: none;
  }

  textarea.input {
    resize: vertical;
  }

  .btn {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    padding: 0.55rem 0.9rem;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    font-weight: 650;
    font-size: 0.9rem;
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      transform 0.02s ease;
    user-select: none;
  }

  .btn:hover {
    background: var(--bg-hover);
  }

  .btn:active {
    transform: translateY(1px);
  }

  .btn:disabled {
    opacity: 0.7;
    cursor: wait;
  }

  .btn-primary {
    background: rgba(99, 102, 241, 0.95);
    border-color: rgba(99, 102, 241, 0.55);
    color: white;
  }

  .btn-primary:hover {
    background: rgba(99, 102, 241, 1);
  }

  .btn-secondary {
    background: var(--bg-surface);
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.9rem;
    padding: 0.85rem 0.95rem;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 8%);
  }

  .toggle-text {
    min-width: 0;
    display: grid;
    gap: 0.15rem;
  }

  .toggle-title {
    color: var(--text-primary);
    font-weight: 800;
  }

  .toggle-sub {
    color: var(--text-secondary);
    font-weight: 600;
    font-size: 0.9rem;
    line-height: 1.35;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }

  .field-hint {
    margin-top: 0.35rem;
    color: var(--text-secondary);
    font-size: 0.8rem;
    line-height: 1.35;
  }

  .field-hint.warning {
    color: rgb(251, 191, 36);
  }

  .mono {
    font-variant-numeric: tabular-nums;
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  @media (max-width: 900px) {
    .grid2 {
      grid-template-columns: 1fr;
    }
  }
</style>
